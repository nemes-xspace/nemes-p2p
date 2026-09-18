//! NEMES Core - Shared types, crypto, and serialization

pub mod crypto;
pub mod types;
pub mod protocol;
pub mod shard;
pub mod storage;
pub mod consensus;
pub mod mesh_audit;

pub use crypto::*;
pub use types::*;
pub use protocol::*;
pub use shard::*;
pub use storage::*;
pub use consensus::*;
pub use mesh_audit::*;

/// NEMES Network identifier
pub const NETWORK_ID: &str = "nemes-mainnet-v1";
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Maximum size for a single message (16MB)
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Default protocol ports
pub const DEFAULT_P2P_PORT: u16 = 4001;
pub const DEFAULT_API_PORT: u16 = 8787;
pub const DEFAULT_RPC_PORT: u16 = 8788;

/// Network magic bytes for protocol identification
pub const NETWORK_MAGIC: [u8; 4] = *b"NEMX";

/// Protocol magic for message framing
pub const PROTOCOL_MAGIC: [u8; 4] = *b"NEMX";

/// Maximum message size for P2P communication
pub const MAX_P2P_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Default gossip fanout
pub const GOSSIP_FANOUT: usize = 6;

/// Default heartbeat interval (seconds)
pub const HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Default peer timeout (seconds)
pub const PEER_TIMEOUT_SECS: u64 = 120;

/// Maximum number of connected peers
pub const MAX_PEERS: usize = 100;

/// Gossipsub degree targets
pub const GOSSIP_DEGREE_LOW: usize = 4;
pub const GOSSIP_DEGREE_HIGH: usize = 12;

/// Gomulu master public key (resmi agin komuta anahtari).
/// Sadece `master-ed25519-pub.pem`'den uretildi, private key hicbir zaman bu repoya girmedi.
/// Resmi surumlerde bu sabit degistirilmeden dagitilir; test aglari `--master-pubkey` ile ezer.
pub const MASTER_PUBKEY_HEX: &str =
    "49d73fc954b4497022a9c957289d221e0823bd964c80f7c66123a51b59aa650b";
pub const MASTER_PUBKEY_B64: &str = "Sdc/yVS0SXAiqclXKJ0iHggjvZZMgPfGYSOlG1mqZQs=";
