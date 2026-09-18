//! NEMES signed command protocol (VPS-less authority)

use serde::{Deserialize, Serialize};

/// Command envelope broadcast over gossipsub topic `nemes/komut`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedCommand {
    pub epoch: u64,
    pub expires: u64,
    pub payload_json: String,
    pub signature_b64: String,
    pub pubkey_b64: String,
}

/// Verify a signed command against an expected master public key (base64, 32 bytes).
pub fn verify_signed_command(cmd: &SignedCommand, expected_pubkey_b64: &str) -> bool {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    use ed25519_dalek::{Signature, VerifyingKey};
    if cmd.pubkey_b64 != expected_pubkey_b64 {
        return false;
    }
    let Ok(sig_bytes) = B64.decode(cmd.signature_b64.trim()) else { return false };
    let Ok(pub_bytes) = B64.decode(cmd.pubkey_b64.trim()) else { return false };
    if sig_bytes.len() != 64 || pub_bytes.len() != 32 {
        return false;
    }
    let Ok(sig_arr) = <[u8; 64]>::try_from(sig_bytes) else { return false };
    let Ok(pub_arr) = <[u8; 32]>::try_from(pub_bytes) else { return false };
    let Ok(vk) = VerifyingKey::from_bytes(&pub_arr) else { return false };
    let sig = Signature::from_bytes(&sig_arr);
    let mut msg = Vec::new();
    msg.extend_from_slice(&cmd.epoch.to_le_bytes());
    msg.extend_from_slice(&cmd.expires.to_le_bytes());
    msg.extend_from_slice(cmd.payload_json.as_bytes());
    vk.verify_strict(&msg, &sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn test_signed_command_roundtrip() {
        use rand::rngs::OsRng;
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();
        let pub_b64 = B64.encode(vk.to_bytes());
        let epoch = 7u64;
        let expires = 9999999999u64;
        let payload = "{\"tip\":\"dur\"}".to_string();
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
            pubkey_b64: pub_b64.clone(),
        };
        assert!(verify_signed_command(&cmd, &pub_b64));
        let mut bad = cmd.clone();
        bad.epoch = 8;
        assert!(!verify_signed_command(&bad, &pub_b64));
    }
}
