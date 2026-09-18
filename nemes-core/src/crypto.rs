//! Crypto primitives for NEMES

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use blake3;
use anyhow::Result;

/// Generate a new Ed25519 keypair
pub fn generate_keypair() -> (ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey) {
    let signing_key = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

/// Hash data using Blake3
pub fn hash(data: &[u8]) -> [u8; 32] {
    blake3::hash(data).into()
}

/// Sign a message with a signing key
pub fn sign(message: &[u8], signing_key: &ed25519_dalek::SigningKey) -> ed25519_dalek::Signature {
    signing_key.sign(message)
}

/// Verify a signature
pub fn verify(message: &[u8], signature: &ed25519_dalek::Signature, verifying_key: &ed25519_dalek::VerifyingKey) -> bool {
    verifying_key.verify(message, signature).is_ok()
}

/// Hash data using Blake3 and return as hex string
pub fn hash_to_hex(data: &[u8]) -> String {
    hex::encode(blake3::hash(data).as_bytes())
}

/// Verify a Blake3 hash
pub fn verify_hash(data: &[u8], expected_hash: &[u8; 32]) -> bool {
    blake3::hash(data).as_bytes() == expected_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash() {
        let data = b"hello world";
        let hash = hash(data);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_sign_verify() {
        let (sk, pk) = generate_keypair();
        let msg = b"test message";
        let sig = sign(msg, &sk);
        assert!(verify(msg, &sig, &pk));
    }
}