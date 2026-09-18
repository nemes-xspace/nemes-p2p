//! Type definitions for NEMES

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique peer identifier (Ed25519 public key)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PeerId(pub [u8; 32]);

impl PeerId {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn from_public_key(key: &ed25519_dalek::VerifyingKey) -> Self {
        Self(key.to_bytes())
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(hex: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(hex)?;
        if bytes.len() != 32 {
            anyhow::bail!("Invalid peer ID length");
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    pub fn short(&self) -> String {
        format!("{}...{}", &self.to_hex()[..6], &self.to_hex()[26..])
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short())
    }
}

impl From<ed25519_dalek::VerifyingKey> for PeerId {
    fn from(key: ed25519_dalek::VerifyingKey) -> Self {
        Self::from_public_key(&key)
    }
}

impl From<ed25519_dalek::SigningKey> for PeerId {
    fn from(key: ed25519_dalek::SigningKey) -> Self {
        Self::from(key.verifying_key())
    }
}

impl From<[u8; 32]> for PeerId {
    fn from(bytes: [u8; 32]) -> Self {
        Self::new(bytes)
    }
}

impl AsRef<[u8]> for PeerId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Content Identifier (CID) - Blake3 hash of content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentId(pub [u8; 32]);

impl ContentId {
    pub fn new(data: &[u8]) -> Self {
        Self(blake3::hash(data).into())
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(hex: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(hex)?;
        if bytes.len() != 32 {
            anyhow::bail!("Invalid CID length");
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    pub fn short(&self) -> String {
        format!("{}...{}", &self.to_hex()[..8], &self.to_hex()[24..])
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short())
    }
}

impl From<blake3::Hash> for ContentId {
    fn from(hash: blake3::Hash) -> Self {
        Self(hash.into())
    }
}

impl From<[u8; 32]> for ContentId {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for ContentId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Shard identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShardId(pub u64);

impl ShardId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn from_corpus_and_index(corpus: &str, index: u64) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(corpus.as_bytes());
        hasher.update(&index.to_le_bytes());
        let digest = hasher.finalize();
        let bytes = digest.as_bytes();
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&bytes[..8]);
        Self(u64::from_le_bytes(arr))
    }
}

/// Document chunk with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub id: ContentId,
    pub shard_id: ShardId,
    pub content: Vec<u8>,
    pub metadata: ChunkMetadata,
    pub embeddings: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChunkMetadata {
    pub source: String,
    pub language: Option<String>,
    pub timestamp: u64,
    pub tags: Vec<String>,
    pub custom: std::collections::HashMap<String, String>,
}

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Ping/pong for connectivity
    Ping { nonce: u64 },
    Pong { nonce: u64 },

    /// Peer discovery
    FindPeers { query: PeerQuery },
    Peers(Vec<PeerInfo>),

    /// Content discovery
    FindContent { cid: ContentId },
    ContentOffer { cid: ContentId, provider: PeerId },
    ContentRequest { cid: ContentId, offset: u64, length: u32 },
    ContentResponse { cid: ContentId, data: Vec<u8> },

    /// Shard management
    ShardAnnounce { shard_id: u64, provider: PeerId, metadata: ShardMetadata },
    ShardRequest { shard_id: u64, offset: u64, length: u32 },
    ShardData { shard_id: u64, data: Vec<u8> },

    /// Embedding sync
    EmbeddingSync { model: String, vectors: Vec<VectorRecord> },
    EmbeddingRequest { model: String, cids: Vec<ContentId> },
    EmbeddingResponse { records: Vec<VectorRecord> },

    /// Consensus
    BlockProposal { block: Box<ConsensusBlock> },
    Vote { block_hash: ContentId, signature: Vec<u8> },
    FinalityProof { block_hash: ContentId, signatures: Vec<SignatureRecord> },

    /// Wallet/Transaction
    Transaction(Tx),
    Block(Box<ConsensusBlock>),

    /// Custom protocol messages
    Custom { protocol: String, data: Vec<u8> },
}

/// Peer query for discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerQuery {
    pub capabilities: Vec<Capability>,
    pub min_reputation: Option<u32>,
    pub max_distance: Option<u32>,
    pub max_results: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    Embedding,
    Storage,
    Compute,
    Validation,
    Routing,
    Gateway,
}

impl Default for Capability {
    fn default() -> Self {
        Self::Storage
    }
}

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub addresses: Vec<std::net::SocketAddr>,
    pub capabilities: Vec<Capability>,
    pub reputation: u32,
    pub version: String,
    pub last_seen: u64,
    pub protocol_version: String,
}

/// Shard metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMetadata {
    pub shard_id: u64,
    pub corpus: String,
    pub size_bytes: u64,
    pub document_count: u64,
    pub vector_dim: usize,
    pub model: String,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Vector record for embedding storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRecord {
    pub id: ContentId,
    pub vector: Vec<f32>,
    pub metadata: ChunkMetadata,
}

/// Consensus block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusBlock {
    pub height: u64,
    pub parent_hash: ContentId,
    pub timestamp: u64,
    pub transactions: Vec<Tx>,
    pub state_root: ContentId,
    pub proposer: PeerId,
    pub signature: Vec<u8>,
}

/// Transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tx {
    pub from: PeerId,
    pub to: PeerId,
    pub amount: u64,
    pub fee: u64,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub signature: Vec<u8>,
}

/// Signature record for finality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureRecord {
    pub voter: PeerId,
    pub signature: Vec<u8>,
    pub timestamp: u64,
}

/// Wallet address (same as PeerId for now)
pub type WalletAddress = PeerId;

/// Transaction hash
pub type TxHash = ContentId;

/// Block height
pub type BlockHeight = u64;

/// Gas price (in wei-like units)
pub type GasPrice = u64;

/// Gas limit
pub type GasLimit = u64;

/// Nonce
pub type Nonce = u64;
