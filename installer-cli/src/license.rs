//! License key validation with Ed25519 signature verification.

use anyhow::{bail, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

const KEY_PREFIX: &str = "ENGR_";
const ISSUER: &str = "engrammic";

// Ed25519 public key for license verification (32 bytes, URL-safe base64)
// Must match the keypair in context-service/license/keys.py
const LICENSE_PUBLIC_KEY: &str = "hBOyf9EQFrXkTJh-YuLFZKcp4Bd9HPNXRfe9BBunhwM";

/// Validate license key format and cryptographic signature.
pub fn validate_license_format(key: &str) -> Result<LicenseBasicInfo> {
    if !key.starts_with(KEY_PREFIX) {
        bail!("License key must start with {}", KEY_PREFIX);
    }

    let token = &key[KEY_PREFIX.len()..];
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        bail!("Invalid license key format");
    }

    let header_b64 = parts[0];
    let payload_b64 = parts[1];
    let signature_b64 = parts[2];

    // Decode and parse payload
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| anyhow::anyhow!("Invalid license key encoding"))?;

    let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
        .map_err(|_| anyhow::anyhow!("Invalid license key payload"))?;

    // Verify signature over "header.payload"
    verify_signature(header_b64, payload_b64, signature_b64)?;

    // Extract and verify claims
    let issuer = payload["iss"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing issuer in license"))?;
    if issuer != ISSUER {
        bail!("Invalid license issuer");
    }

    let customer = payload["sub"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing customer in license"))?;
    let exp = payload["exp"]
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("Missing expiry in license"))?;

    // Check not expired
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    if exp < now {
        bail!("License key has expired");
    }

    let days_remaining = (exp - now) / (24 * 60 * 60);

    Ok(LicenseBasicInfo {
        customer: customer.to_string(),
        expires_at: exp,
        days_remaining: days_remaining as u32,
    })
}

fn verify_signature(header_b64: &str, payload_b64: &str, signature_b64: &str) -> Result<()> {
    // Decode public key
    let pubkey_bytes = URL_SAFE_NO_PAD
        .decode(LICENSE_PUBLIC_KEY)
        .map_err(|_| anyhow::anyhow!("Invalid embedded public key"))?;

    let pubkey_array: [u8; 32] = pubkey_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid public key length"))?;

    let verifying_key = VerifyingKey::from_bytes(&pubkey_array)
        .map_err(|_| anyhow::anyhow!("Invalid public key"))?;

    // Decode signature
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|_| anyhow::anyhow!("Invalid signature encoding"))?;

    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid signature length"))?;

    let signature = Signature::from_bytes(&sig_array);

    // Verify signature over "header.payload"
    let message = format!("{}.{}", header_b64, payload_b64);

    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|_| anyhow::anyhow!("Invalid license signature"))?;

    Ok(())
}

#[derive(Debug)]
pub struct LicenseBasicInfo {
    pub customer: String,
    pub expires_at: i64,
    pub days_remaining: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_prefix() {
        let result = validate_license_format("invalid_key");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ENGR_"));
    }

    #[test]
    fn rejects_malformed_structure() {
        let result = validate_license_format("ENGR_only.two");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("format"));
    }
}
