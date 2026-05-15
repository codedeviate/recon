//! Bitcoin-family base58check: version byte + payload + 4-byte SHA-256d checksum.

use super::Verdict;
use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};

/// Known version byte → (coin, type label).
pub fn coin_from_version(v: u8) -> Option<(&'static str, &'static str)> {
    match v {
        0x00 => Some(("BTC", "P2PKH")),
        0x05 => Some(("BTC", "P2SH")),
        0x30 => Some(("LTC", "P2PKH (L)")),
        0x32 => Some(("LTC", "P2SH (M)")),
        0x1E => Some(("DOGE", "P2PKH")),
        0x16 => Some(("DOGE", "P2SH")),
        _ => None,
    }
}

fn decode_and_validate(addr: &str) -> Result<(u8, Vec<u8>)> {
    let decoded = bs58::decode(addr)
        .into_vec()
        .map_err(|e| anyhow!("base58 decode: {}", e))?;
    if decoded.len() < 5 {
        return Err(anyhow!("base58check data too short"));
    }
    let (payload, checksum) = decoded.split_at(decoded.len() - 4);
    let h1 = Sha256::digest(payload);
    let h2 = Sha256::digest(h1);
    if &h2[..4] != checksum {
        return Err(anyhow!("base58check checksum mismatch"));
    }
    Ok((payload[0], payload[1..].to_vec()))
}

pub fn verify_with_coin_filter(input: &str, allowed: &[u8]) -> Verdict {
    let clean = input.trim();
    match decode_and_validate(clean) {
        Ok((version, _)) => {
            if !allowed.is_empty() && !allowed.contains(&version) {
                return Verdict::Invalid {
                    reason: format!("version byte 0x{:02X} not permitted for this coin", version),
                };
            }
            let (coin, ty) = coin_from_version(version).unwrap_or(("unknown", "unknown"));
            Verdict::Valid {
                formatted: clean.to_string(),
                detected: format!("base58check ({} — {})", coin, ty),
                comment: String::new(),
            }
        }
        Err(e) => Verdict::Invalid { reason: e.to_string() },
    }
}

pub fn verify_btc(input: &str) -> Verdict { verify_with_coin_filter(input, &[0x00, 0x05]) }
pub fn verify_ltc(input: &str) -> Verdict { verify_with_coin_filter(input, &[0x30, 0x32, 0x05]) }
pub fn verify_doge(input: &str) -> Verdict { verify_with_coin_filter(input, &[0x1E, 0x16]) }

pub fn create_unsupported(_input: &str, _raw: bool) -> Result<String> {
    Err(anyhow!("creation not supported: a valid bitcoin-family address needs a real payload (20-byte RIPEMD-160 of a pubkey hash), not just a check digit"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btc_satoshi_address_valid() {
        match verify_btc("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn btc_corrupted_address_invalid() {
        match verify_btc("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNb") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }
}
