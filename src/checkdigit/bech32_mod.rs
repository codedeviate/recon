//! Bech32 / SegWit (BIP-173) — wrapper around the `bech32` crate.

use super::Verdict;
use anyhow::{anyhow, Result};

pub fn verify_bech32(input: &str) -> Verdict {
    let trimmed = input.trim();
    let has_lower = trimmed.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = trimmed.chars().any(|c| c.is_ascii_uppercase());
    if has_lower && has_upper {
        return Verdict::Invalid {
            reason: "bech32 requires consistent case (all-lower or all-upper)".into(),
        };
    }
    let lower = trimmed.to_ascii_lowercase();
    match bech32::decode(&lower) {
        Ok((hrp, _data)) => Verdict::Valid {
            formatted: lower,
            detected: format!("bech32 (hrp '{}')", hrp),
            comment: String::new(),
        },
        Err(e) => Verdict::Invalid { reason: format!("bech32 decode: {}", e) },
    }
}

pub fn create_unsupported(_input: &str, _raw: bool) -> Result<String> {
    Err(anyhow!(
        "bech32 creation requires HRP and witness data; this tool only verifies existing bech32 strings"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bech32_bip173_reference_valid() {
        match verify_bech32("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn bech32_mixed_case_rejected() {
        match verify_bech32("BC1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4") {
            Verdict::Invalid { reason } => assert!(reason.contains("consistent case")),
            _ => panic!(),
        }
    }
}
