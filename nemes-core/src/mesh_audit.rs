//! Mesh denetim atamasi (B15): kimin hangi kaniti denetleyecegi herkesce
//! deterministik hesaplanir.
//!
//! Model: komuta kapanan batch'teki bayrakli kanitlar icin `duyuru` yayinlar
//! (gossip `nemes/denetim`); her madenci duyurudaki atamalari BU modulle
//! yeniden hesaplayip dogrular, kendine dusenleri denetler. Komuta gorev
//! kurmaz, sadece hakemlik yapar (imza/token + kosinus + escrow/slash aynen).
//!
//! Secim: `blake3(gorev_id || b'\x00' || salt)` ilk 8 bayti -> u64; sirali sirali
//! aday listesinden prover HARIC tutularak sirayla `n` denetci secilir.
//! Adaylar siralanir (farkli dugumler ayni siralamayi gorur).

use serde::{Deserialize, Serialize};

/// Denetim duyuru gossip konusu.
pub const DENETIM_TOPIC: &str = "nemes/denetim";
/// Kanit basina denetci sayisi.
pub const DENETIM_SAYISI: usize = 2;

/// Tek kanit atamasi (duyuru ici).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KanitAtama {
    /// Kor ID (denetci gercek madde_id bilmez).
    pub kor: i64,
    /// Asil embed gorev ID'si (sonuc bildiriminde kullanilir).
    pub gorev: String,
    /// Secilmis denetci miner_id'leri (sirali).
    pub denetciler: Vec<String>,
}

/// Batch denetim duyurusu (gossip yuku, hafif).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenetimDuyuru {
    pub gorev_id: String,
    /// Gunluk salt (spot_salt gunu; tekrar-hesap girdisi).
    pub salt_gun: i64,
    pub atamalar: Vec<KanitAtama>,
}

/// Atama girdisi ozeti (duyuru butunlugu icin; tasiyan kanal imzalar).
pub fn duyuru_ozet(d: &DenetimDuyuru) -> String {
    let mut h = blake3::Hasher::new();
    h.update(d.gorev_id.as_bytes());
    h.update(&d.salt_gun.to_le_bytes());
    for a in &d.atamalar {
        h.update(&a.kor.to_le_bytes());
        for x in &a.denetciler {
            h.update(x.as_bytes());
            h.update(b"\x00");
        }
    }
    h.finalize().to_hex().to_string()
}

/// Denetcileri sec: deterministik, prover haric, adaylar sirali.
/// `adaylar` sirali degilse iceride siralanir (girdi sirasi sonucu etkilemez).
pub fn denetciler(
    gorev_id: &str,
    salt_gun: i64,
    adaylar: &[String],
    prover: &str,
    n: usize,
) -> Vec<String> {
    let mut havuz: Vec<&str> = adaylar
        .iter()
        .map(|s| s.as_str())
        .filter(|s| *s != prover)
        .collect();
    havuz.sort_unstable();
    havuz.dedup();
    if havuz.is_empty() || n == 0 {
        return Vec::new();
    }
    let mut h = blake3::Hasher::new();
    h.update(gorev_id.as_bytes());
    h.update(b"\x00");
    h.update(&salt_gun.to_le_bytes());
    let basla = u64::from_le_bytes(h.finalize().as_bytes()[..8].try_into().unwrap_or([0; 8])) as usize;
    let m = havuz.len();
    let al = n.min(m);
    (0..al)
        .map(|i| havuz[(basla + i) % m].to_string())
        .collect()
}

/// Duyuruyu dogrula: atamalar bu modulle yeniden hesaplaninca aynisi cikmali.
/// `kayitli` = komutanin sundugu aday listesi (kalp atisi taze madenciler).
pub fn duyuru_dogrula(duyuru: &DenetimDuyuru, kayitli: &[String], proverlar: &std::collections::HashMap<i64, String>) -> bool {
    for a in &duyuru.atamalar {
        let prover = match proverlar.get(&a.kor) {
            Some(p) => p,
            None => return false,
        };
        // a.gorev, prover'in kanitina ait olmali (atama o gorev uzerinden turetilir).
        let beklenen = denetciler(&a.gorev, duyuru.salt_gun, kayitli, prover, a.denetciler.len());
        if beklenen != a.denetciler {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn adaylar() -> Vec<String> {
        vec!["m1", "m2", "m3", "m4", "m5"]
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn test_deterministik() {
        let a = adaylar();
        let x = denetciler("g:1", 42, &a, "m1", 2);
        let y = denetciler("g:1", 42, &a, "m1", 2);
        assert_eq!(x, y, "ayni girdi ayni atama");
        assert_eq!(x.len(), 2);
        assert!(!x.contains(&"m1".to_string()), "prover haric");
    }

    #[test]
    fn test_sira_bagimsiz() {
        let mut ters = adaylar();
        ters.reverse();
        let x = denetciler("g:9", 7, &adaylar(), "m2", 3);
        let y = denetciler("g:9", 7, &ters, "m2", 3);
        assert_eq!(x, y, "aday sirasi sonucu degistirmez");
    }

    #[test]
    fn test_dagilim_kabaca_adil() {
        // 500 gorev × 2 denetci: her aday ~200 kez secilmeli (±%40 tolerans).
        let a = adaylar();
        let mut say: HashMap<String, usize> = HashMap::new();
        for i in 0..500 {
            for d in denetciler(&format!("g:{}", i), 1, &a, "m1", 2) {
                *say.entry(d).or_default() += 1;
            }
        }
        // m1 haric 4 aday, toplam 1000 secim -> ~250/aday.
        for m in ["m2", "m3", "m4", "m5"] {
            let s = *say.get(m).unwrap_or(&0);
            assert!((150..=350).contains(&s), "{} dengesiz: {}", m, s);
        }
        assert!(!say.contains_key("m1"), "prover hic secilmemeli");
    }

    #[test]
    fn test_koseler() {
        let a = adaylar();
        assert!(denetciler("g", 1, &[], "m1", 2).is_empty());
        assert!(denetciler("g", 1, &a, "m1", 0).is_empty());
        // Tek aday kalirsa o doner.
        let x = denetciler("g", 1, &["m1".to_string(), "m9".to_string()], "m1", 5);
        assert_eq!(x, vec!["m9".to_string()]);
    }

    #[test]
    fn test_duyuru_turu() {
        let a = adaylar();
        let d = DenetimDuyuru {
            gorev_id: "g:1".to_string(),
            salt_gun: 42,
            atamalar: vec![KanitAtama {
                kor: 777,
                gorev: "g:1".to_string(),
                denetciler: denetciler("g:1", 42, &a, "m1", 2),
            }],
        };
        let mut proverlar = HashMap::new();
        proverlar.insert(777i64, "m1".to_string());
        assert!(duyuru_dogrula(&d, &a, &proverlar));
        // Bozsa yakalanir.
        let mut kotu = d.clone();
        kotu.atamalar[0].denetciler = vec!["m1".to_string()];
        assert!(!duyuru_dogrula(&kotu, &a, &proverlar));
        // Ozet sabit.
        assert_eq!(duyuru_ozet(&d).len(), 64);
    }
}
