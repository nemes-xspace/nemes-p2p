use clap::{Parser, Subcommand};
use anyhow::Result;
use nemes_core::protocol::{verify_signed_command, SignedCommand};

#[derive(Parser)]
#[command(name = "nemes-miner", version = "0.1.0", about = "NEMES CLI - Mine knowledge. Not hashes. (VPS-less P2P)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Master keypair uret (private key'i CEVRIMDISI sakla)
    Keygen,
    /// P2P node baslat (mDNS + gossipsub). Komutlari gomulu master-pubkey'e gore dogrular.
    Node {
        #[arg(long, default_value = "4001")]
        port: u16,
        /// Master public key (hex, 32 byte). Verilmezse binary'e gomulu MASTER_PUBKEY_HEX kullanilir.
        #[arg(long)]
        master_pubkey: Option<String>,
    },
    /// Payload'i master private key ile imzala (cevrimdisi calisir).
    /// GUVENLIK: --key-hex shell history'e duser; gercek anahtarla HER ZAMAN --key-file kullan.
    Imzala {
        #[arg(long)]
        key_hex: Option<String>,
        /// PKCS#8 PEM dosyasi (karttaki master-ed25519.pem). Varsa key-hex okunmaz.
        #[arg(long)]
        key_file: Option<String>,
        #[arg(long)]
        epoch: u64,
        #[arg(long)]
        expires: u64,
        #[arg(long)]
        payload: String,
    },
    /// Imzali komutu hedef peer'a gonder (hedef: /ip4/.../tcp/.../p2p/...).
    KomutGonder {
        #[arg(long)]
        hedef: String,
        /// imzala ciktisi JSON dosyasi
        #[arg(long)]
        komut_json: String,
        /// Baglanti ve yayilim icin bekleme suresi (saniye)
        #[arg(long, default_value = "10")]
        wait: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let cli = Cli::parse();
    match cli.command {
        Commands::Keygen => {
            use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
            use ed25519_dalek::SigningKey;
            use rand::rngs::OsRng;
            let sk = SigningKey::generate(&mut OsRng);
            let vk = sk.verifying_key();
            println!("MASTER_PUBKEY_HEX={}", hex::encode(vk.to_bytes()));
            println!("MASTER_PRIV_HEX={}  # BUNU CEVRIMDISI SAKLA, KIMSEYLE PAYLASMA", hex::encode(sk.to_bytes()));
            println!("Ilk kurulumda tum miner binary'lerine MASTER_PUBKEY_HEX gomulur.");
            let _ = B64;
        }
        Commands::Node { port, master_pubkey } => {
            use nemes_p2p::{NetworkEvent, P2PConfig, P2PNode};
            let mut node = P2PNode::new(P2PConfig { port, enable_mdns: true }).await?;
            let rx = node.take_event_receiver().expect("event receiver");
            // node.run() sonsuz dongu; event'leri ayri gorevde dinle
            // Verilmezse gomulu anahtar; verilirse hex veya base64 kabul edilir.
            let master: String = match master_pubkey {
                Some(m) => {
                    let t = m.trim().to_string();
                    if t.len() == 64 && hex::decode(&t).is_ok() {
                        use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
                        B64.encode(hex::decode(&t).unwrap())
                    } else {
                        t
                    }
                }
                None => nemes_core::MASTER_PUBKEY_B64.to_string(),
            };
            tokio::spawn(async move {
                let mut rx = rx;
                while let Some(ev) = rx.recv().await {
                    match ev {
                        NetworkEvent::PeerConnected(p) => println!("PEER BAGLANDI {}", p),
                        NetworkEvent::PeerDisconnected(p) => println!("PEER AYRILDI {}", p),
                        NetworkEvent::MessageReceived { from, topic, data } => {
                            if topic == "nemes/komut" {
                                match serde_json::from_slice::<SignedCommand>(&data) {
                                    Ok(cmd) => {
                                        let ok = verify_signed_command(&cmd, &master);
                                        if ok {
                                            println!("KOMUT KABUL from={} epoch={} payload={}", from, cmd.epoch, cmd.payload_json);
                                        } else {
                                            println!("KOMUT RED from={} (imza yok/hatali)", from);
                                        }
                                    }
                                    Err(e) => println!("KOMUT PARSE HATA from={}: {}", from, e),
                                }
                            } else {
                                println!("MESAJ topic={} from={} {} byte", topic, from, data.len());
                            }
                        }
                        _ => {}
                    }
                }
            });
            println!("P2P node {} portunda. mDNS ile esler bulunuyor. Durdurmak icin Ctrl+C.", port);
            node.run().await?;
        }
        Commands::Imzala { key_hex, key_file, epoch, expires, payload } => {
            use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
            use ed25519_dalek::{Signer, SigningKey};
            let arr: [u8; 32] = if let Some(path) = key_file {
                // PKCS#8 PEM: son 32 byte seed'dir
                let pem = std::fs::read_to_string(&path)?;
                let b64: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
                let der = B64.decode(b64.trim())?;
                if der.len() < 32 {
                    anyhow::bail!("PEM anlasilmadi");
                }
                der[der.len() - 32..].try_into().map_err(|_| anyhow::anyhow!("seed cikartilamadi"))?
            } else if let Some(h) = key_hex {
                eprintln!("UYARI: --key-hex shell history'e duser, gercek anahtarla --key-file kullan");
                let raw = hex::decode(h.trim())?;
                raw.try_into().map_err(|_| anyhow::anyhow!("key 32 byte olmali"))?
            } else {
                anyhow::bail!("--key-file veya --key-hex gerekli");
            };
            let sk = SigningKey::from_bytes(&arr);
            let vk = sk.verifying_key();
            let mut msg = Vec::new();
            msg.extend_from_slice(&epoch.to_le_bytes());
            msg.extend_from_slice(&expires.to_le_bytes());
            msg.extend_from_slice(payload.as_bytes());
            let sig = sk.sign(&msg);
            let cmd = SignedCommand {
                epoch,
                expires,
                payload_json: payload,
                signature_b64: B64.encode(sig.to_bytes()),
                pubkey_b64: B64.encode(vk.to_bytes()),
            };
            println!("{}", serde_json::to_string_pretty(&cmd)?);
        }
Commands::KomutGonder { hedef, komut_json, wait } => {
            use nemes_p2p::{P2PConfig, P2PNode};
            let data = std::fs::read(&komut_json)?;
            let cmd: SignedCommand = serde_json::from_slice(&data)?;
            let mut node = P2PNode::new(P2PConfig { port: 0, enable_mdns: true }).await?;
            let addr: libp2p::Multiaddr = hedef.parse()?;
            node.dial(addr)?;
            // Once baglanti icin swarm calistir
            println!("Hedefe baglaniliyor... max {} sn (swarm calisiyor)", wait);
            node.run_for(wait).await?;
            // Simdi publish yap
            node.publish("nemes/komut", data)?;
            println!("Yayinlandi epoch={}, mesaj gonderilmesi icin 5 sn swarm calisiyor...", cmd.epoch);
            // Mesajin gitmesi icin swarm'i biraz daha calistir
            node.run_for(5).await?;
            println!("TAMAM");
        }
    }
    Ok(())
}
