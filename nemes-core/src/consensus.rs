//! Consensus engine stub (VPS-less, reputation first)

use crate::types::{ConsensusBlock, Tx};

pub struct ConsensusEngine;

impl ConsensusEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn propose_block(&self, _transactions: Vec<Tx>) -> anyhow::Result<ConsensusBlock> {
        anyhow::bail!("consensus propose not implemented in minimal build")
    }

    pub async fn validate_block(&self, _block: &ConsensusBlock) -> anyhow::Result<bool> {
        Ok(true)
    }
}
