//! Slovenian VAT.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 7] = [8, 7, 6, 5, 4, 3, 2];

pub fn verify_si_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
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
    let mut expected = 11 - sum % 11;
    if expected == 10 { expected = 0; }
    if expected == 11 {
        return Verdict::Invalid { reason: "SI VAT: computed check digit is 11 — invalid".into() };
    }
    if expected == check {
        Verdict::Valid { formatted: format!("SI{}", clean), detected: "Slovenian VAT".into() }
    } else {
        Verdict::Invalid { reason: format!("SI VAT check mismatch: expected {}, got {}", expected, check) }
    }
}

pub fn create_si_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 7 {
        return Err(anyhow!("expected 7 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let sum: u32 = clean.chars().enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap())
        .sum();
    let mut check = 11 - sum % 11;
    if check == 10 { check = 0; }
    if check == 11 {
        return Err(anyhow!("SI VAT: no valid check digit"));
    }
    Ok(format!("SI{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn si_vat_valid_15012557() {
        match verify_si_vat("15012557") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn si_vat_round_trip() {
        let body = "1501255";
        let full = create_si_vat(body, false).unwrap();
        let clean: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_si_vat(&clean) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
