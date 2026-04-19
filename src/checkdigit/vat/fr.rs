//! French VAT.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

// 2-char key + 9-digit SIREN. Key = (12 + 3 × (SIREN mod 97)) mod 97.
pub fn verify_fr_vat(input: &str) -> Verdict {
    let clean = sanitize(input, true);
    if clean.len() != 11 {
        return Verdict::Invalid { reason: format!("expected 11 chars (2-key + 9-SIREN), got {}", clean.len()) };
    }
    let key_part = &clean[..2];
    let siren = &clean[2..];
    if !siren.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "SIREN must be 9 digits".into() };
    }
    let siren_num: u64 = siren.parse().unwrap();
    let expected_key: u32 = ((12 + 3 * (siren_num % 97)) % 97) as u32;
    let key_num: u32 = match key_part.parse() {
        Ok(n) => n,
        Err(_) => {
            return Verdict::Valid {
                formatted: format!("FR{}", clean),
                detected: "French VAT (alphanumeric key — check skipped)".into(),
            };
        }
    };
    if expected_key == key_num {
        Verdict::Valid {
            formatted: format!("FR{}", clean),
            detected: "French VAT".into(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("FR VAT key mismatch: expected {:02}, got {}", expected_key, key_part),
        }
    }
}

pub fn create_fr_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!("expected 9 SIREN digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("SIREN must be digits"));
    }
    let siren_num: u64 = clean.parse().unwrap();
    let key: u32 = ((12 + 3 * (siren_num % 97)) % 97) as u32;
    Ok(format!("FR{:02}{}", key, clean))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr_vat_round_trip() {
        let siren = "123456789";
        let full = create_fr_vat(siren, false).unwrap();
        let vat_body = full.trim_start_matches("FR").to_string();
        match verify_fr_vat(&vat_body) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
