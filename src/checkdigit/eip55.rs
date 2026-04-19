//! Ethereum EIP-55 mixed-case checksum.

use super::Verdict;
use anyhow::{anyhow, Result};
use sha3::{Digest, Keccak256};

fn to_eip55(addr_lower: &str) -> String {
    let hash = Keccak256::digest(addr_lower.as_bytes());
    let hex_hash: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
    let mut out = String::with_capacity(40);
    for (i, c) in addr_lower.chars().enumerate() {
        if c.is_ascii_alphabetic() {
            let nib = u8::from_str_radix(&hex_hash[i..i + 1], 16).unwrap();
            if nib >= 8 {
                out.push(c.to_ascii_uppercase());
            } else {
                out.push(c);
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn verify_eip55(input: &str) -> Verdict {
    let trimmed = input.trim();
    let (prefix, body) = if let Some(s) = trimmed.strip_prefix("0x") {
        ("0x", s)
    } else if let Some(s) = trimmed.strip_prefix("0X") {
        ("0x", s)
    } else {
        ("", trimmed)
    };
    if body.len() != 40 {
        return Verdict::Invalid { reason: format!("expected 40 hex chars, got {}", body.len()) };
    }
    if !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return Verdict::Invalid { reason: "non-hex input".into() };
    }
    let lower = body.to_ascii_lowercase();
    let is_mixed = body != lower && body != body.to_ascii_uppercase();
    if !is_mixed {
        return Verdict::Valid {
            formatted: format!("0x{}", body),
            detected: "Ethereum address (no EIP-55 case check applied)".into(),
            comment: String::new(),
        };
    }
    let expected = to_eip55(&lower);
    if body == expected {
        Verdict::Valid {
            formatted: format!("{}{}", prefix, expected),
            detected: "Ethereum EIP-55".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid { reason: "EIP-55 mixed-case checksum mismatch".into() }
    }
}

pub fn create_eip55(input: &str, _raw: bool) -> Result<String> {
    let trimmed = input.trim();
    let body = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")).unwrap_or(trimmed);
    if body.len() != 40 || !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow!("expected 40 hex chars, got {}", body.len()));
    }
    let lower = body.to_ascii_lowercase();
    let mixed = to_eip55(&lower);
    Ok(format!("0x{}", mixed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eip55_lowercase_is_valid_without_check() {
        match verify_eip55("0xde709f2102306220921060314715629080e2fb77") {
            Verdict::Valid { detected, .. } => assert!(detected.contains("no EIP-55")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn eip55_create_produces_mixed_case() {
        // Use an address that actually has alphabetic chars with hash nibbles >= 8
        // so the EIP-55 encoding yields a verifiable mixed-case result.
        let created = create_eip55("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed", false).unwrap();
        match verify_eip55(&created) {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Ethereum EIP-55"),
            v => panic!("{:?}", v),
        }
    }
}
