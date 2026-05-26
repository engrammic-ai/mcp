//! License key validation.

use anyhow::{bail, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

const KEY_PREFIX: &str = "ENGR_";

/// Basic license key format validation.
/// Full cryptographic validation happens server-side.
pub fn validate_license_format(key: &str) -> Result<LicenseBasicInfo> {
    if !key.starts_with(KEY_PREFIX) {
        bail!("License key must start with {}", KEY_PREFIX);
    }

    let token = &key[KEY_PREFIX.len()..];
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        bail!("Invalid license key format");
    }

    // Decode payload (middle part)
    let payload_b64 = parts[1];
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| anyhow::anyhow!("Invalid license key encoding"))?;

    let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
        .map_err(|_| anyhow::anyhow!("Invalid license key payload"))?;

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

pub struct LicenseBasicInfo {
    pub customer: String,
    pub expires_at: i64,
    pub days_remaining: u32,
}
