use anyhow::Result;
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use bip39::{Mnemonic, Language};
use rand::rngs::OsRng;
use crate::{PeerId, WalletAddress};

pub struct Wallet {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    address: WalletAddress,
}

impl Wallet {
    pub fn new() -> Self {
        let mut csprng = rand::rngs::OsRng;
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let verifying_key = signing_key.verifying_key();
        let address = WalletAddress::from(signing_key.verifying_key());
        Self {
            signing_key,
            verifying_key,
            address,
        }
    }

    pub fn from_mnemonic(mnemonic: &str) -> anyhow::Result<Self> {
        let mnemonic = Mnemonic::parse_in(Language::English, mnemonic)?;
        let seed = mnemonic.to_seed("");
        let signing_key = SigningKey::from_bytes(&seed[..32].try_into().unwrap());
        let verifying_key = signing_key.verifying_key();
        let address = WalletAddress::from(signing_key.verifying_key());
        Ok(Self {
            signing_key,
            verifying_key,
            address,
        })
    }

    pub fn from_private_key(bytes: &[u8; 32]) -> anyhow::Result<Self> {
        let signing_key = SigningKey::from_bytes(bytes.try_into()?);
        let verifying_key = signing_key.verifying_key();
        let address = WalletAddress::from(signing_key.verifying_key());
        Ok(Self {
            signing_key,
            verifying_key,
            address,
        })
    }

    pub fn address(&self) -> &WalletAddress {
        &self.address
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.verifying_key.verify(message, signature).is_ok()
    }

    pub fn export_private_key(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    pub fn mnemonic(&self) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        let wallet = Wallet::new();
        assert!(!wallet.address().to_hex().is_empty());
    }

    #[test]
    fn test_sign_verify() {
        let wallet = Wallet::new();
        let message = b"test message";
        let sig = wallet.sign(message);
        assert!(wallet.verify(message, &sig));
    }
}