use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

use crate::nemes_core::*;
use crate::nemes_p2p::*;
use crate::nemes_storage::*;
use crate::nemes_embedding::*;
use crate::nemes_consensus::*;
use crate::nemes_wallet::*;

pub struct NodeConfig {
    pub p2p_port: u16,
    pub data_dir: String,
    pub wallet_path: String,
    pub bootstrap_peers: Vec<String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            p2p_port: 4001,
            data_dir: "./data".to_string(),
            wallet_path: "./wallet".to_string(),
            bootstrap_peers: vec![],
        }
    }
}

pub struct Node {
    config: NodeConfig,
    p2p: Arc<crate::nemes_p2p::P2PNode>,
    storage: Arc<StorageEngine>,
    wallet: Wallet,
    running: std::sync::atomic::AtomicBool,
}

impl Node {
    pub async fn new(config: NodeConfig) -> anyhow::Result<Self> {
        let p2p_config = P2PConfig {
            port: config.p2p_port,
            bootstrap_nodes: config.bootstrap_peers.clone(),
            ..Default::default()
        };

        let p2p = Arc::new(P2PNode::new(P2PConfig {
            listen_addresses: vec![format!("0.0.0.0:{}", config.p2p_port).parse()?],
            bootstrap_nodes: config.bootstrap_peers.clone(),
            ..Default::default()
        }).await?);

        let storage = Arc::new(StorageEngine::new(&config.data_dir)?);
        
        // Load or create wallet
        let wallet = Wallet::new(); // TODO: Load from file

        Ok(Self {
            config,
            p2p,
            storage,
            wallet,
            running: std::sync::atomic::AtomicBool::new(false),
        })
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        self.running.store(true, Ordering::Relaxed);
        self.p2p.start().await?;
        Ok(())
    }

    pub async fn shutdown(&self) {
        self.running.store(false, Ordering::Relaxed);
        self.p2p.shutdown().await;
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_startup() {
        let config = NodeConfig::default();
        let node = Node::new(config).await.unwrap();
        node.start().await.unwrap();
        assert!(node.is_running());
        node.shutdown().await;
        assert!(!node.is_running());
    }
}