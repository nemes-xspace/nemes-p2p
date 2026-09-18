//! Shard sahipligi ilanlari (gossip topic `nemes/shard`).
//!
//! Ilan GORELI'dir: madenci "sonraki K id'yi istiyorum" der, komuta
//! ulasma anindaki cursor'a sabirler (baslangic = cursor+1).
//! Gecikmeli gossip'e dayaniklidir; cakisma ilk-yazan-kazanir.
//!
//! Format v0 (imzali):
//!   kanonik = "SHARDILAN|v0|{corpus}|{adet}|{miner_id}|{ts_ms}"

use serde::{Deserialize, Serialize};

pub const SHARD_TOPIC: &str = "nemes/shard";
pub const SHARD_FORMAT: &str = "v0";
/// Ayni miner'dan art arda ilanlarin ts_ms farki (tekrar korumasi icin komuta bakar).
pub const SHARD_MAX_ADET: i64 = 100_000;

/// Gossip ile yayinlanan shard ilani.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardIlan {
    pub v: String,
    pub corpus: String,
    pub adet: i64,
    pub miner_id: String,
    pub ts_ms: i64,
    pub pubkey_b64: String,
    pub sig_b64: String,
}

impl ShardIlan {
    /// Imzalanacak kanonik baytlar.
    pub fn kanonik(corpus: &str, adet: i64, miner_id: &str, ts_ms: i64) -> Vec<u8> {
        format!("SHARDILAN|v0|{}|{}|{}|{}", corpus, adet, miner_id, ts_ms).into_bytes()
    }

    /// Ilani dogrula (format + imza). Kayit baglama (TOFU) komutada.
    pub fn dogrula(&self) -> anyhow::Result<()> {
        use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
        use ed25519_dalek::{Signature, VerifyingKey};
        if self.v != SHARD_FORMAT {
            anyhow::bail!("bilinmeyen shard ilan formati: {}", self.v);
        }
        if self.corpus.is_empty() || self.miner_id.is_empty() {
            anyhow::bail!("corpus/miner_id bos olamaz");
        }
        if self.adet <= 0 || self.adet > SHARD_MAX_ADET {
            anyhow::bail!("adet aralik disi: {}", self.adet);
        }
        let pk_raw = B64.decode(self.pubkey_b64.trim())?;
        let pk_arr: [u8; 32] = pk_raw
            .try_into()
            .map_err(|_| anyhow::anyhow!("pubkey 32 bayt olmali"))?;
        let vk = VerifyingKey::from_bytes(&pk_arr)?;
        let sig_raw = B64.decode(self.sig_b64.trim())?;
        let sig = Signature::try_from(&sig_raw[..])?;
        let msg = Self::kanonik(&self.corpus, self.adet, &self.miner_id, self.ts_ms);
        use ed25519_dalek::Verifier;
        vk.verify(&msg, &sig)?;
        Ok(())
    }

    pub fn json_bayt(&self) -> anyhow::Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    pub fn json_coz(data: &[u8]) -> anyhow::Result<Self> {
        let ilan: Self = serde_json::from_slice(data)?;
        ilan.dogrula()?;
        Ok(ilan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    use ed25519_dalek::Signer;

    #[test]
    fn ilan_imza_dongusu() {
        let sk = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
        let msg = ShardIlan::kanonik("tr", 2000, "miner-test", 1234567890);
        let sig = sk.sign(&msg);
        let ilan = ShardIlan {
            v: SHARD_FORMAT.to_string(),
            corpus: "tr".into(),
            adet: 2000,
            miner_id: "miner-test".into(),
            ts_ms: 1234567890,
            pubkey_b64: B64.encode(sk.verifying_key().to_bytes()),
            sig_b64: B64.encode(sig.to_bytes()),
        };
        assert!(ilan.dogrula().is_ok());
        let geri = ShardIlan::json_coz(&ilan.json_bayt().unwrap()).unwrap();
        assert_eq!(geri.miner_id, "miner-test");
    }
}
