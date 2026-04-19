//! Hungarian VAT.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 7] = [9, 7, 3, 1, 9, 7, 3];

pub fn verify_hu_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "HU") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 8 {
        return Verdict::Invalid { reason: format!("expected 8 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let body = &clean[..7];
    let check: u32 = clean.chars().nth(7).unwrap().to_digit(10).unwrap();
    let sum: u32 = body.chars().enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap())
        .sum();
    let expected = (10 - sum % 10) % 10;
    if expected == check {
        Verdict::Valid { formatted: format!("HU{}", clean), detected: "Hungarian VAT".into(), comment: String::new() }
    } else {
        Verdict::Invalid { reason: format!("HU VAT check mismatch: expected {}, got {}", expected, check) }
    }
}

pub fn create_hu_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 7 {
        return Err(anyhow!("expected 7 digits (body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let sum: u32 = clean.chars().enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap())
        .sum();
    let check = (10 - sum % 10) % 10;
    Ok(format!("HU{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hu_vat_valid_12345676() {
        match verify_hu_vat("12345676") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn hu_vat_round_trip() {
        let body = "1234567";
        let full = create_hu_vat(body, false).unwrap();
        let clean: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_hu_vat(&clean) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn hu_vat_rejects_bad_check() {
        match verify_hu_vat("12345679") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
