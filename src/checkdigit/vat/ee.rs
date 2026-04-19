//! Estonian VAT.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 9] = [3, 7, 1, 3, 7, 1, 3, 7, 1];

pub fn verify_ee_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Verdict::Invalid { reason: format!("expected 9 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let sum: u32 = clean.chars().enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap())
        .sum();
    if sum % 10 == 0 {
        Verdict::Valid { formatted: format!("EE{}", clean), detected: "Estonian VAT".into(), comment: String::new() }
    } else {
        Verdict::Invalid { reason: format!("EE VAT weighted mod-10 check failed (sum {})", sum) }
    }
}

pub fn create_ee_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits (body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let sum: u32 = clean.chars().enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap())
        .sum();
    // weights[8] == 1, so check * 1 must make total sum % 10 == 0
    let check = (10 - sum % 10) % 10;
    Ok(format!("EE{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ee_vat_valid_100931558() {
        match verify_ee_vat("100931558") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ee_vat_round_trip() {
        let body = "10093155";
        let full = create_ee_vat(body, false).unwrap();
        let clean: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_ee_vat(&clean) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ee_vat_rejects_bad_check() {
        match verify_ee_vat("100931559") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
