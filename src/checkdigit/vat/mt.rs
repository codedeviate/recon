//! Maltese VAT.
//!
//! 8 digits. Weights [3, 4, 6, 7, 8, 9] apply to the first 6 digits.
//! The last 2 digits are the check: check = 37 - (sum % 37), zero-padded.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 6] = [3, 4, 6, 7, 8, 9];

fn compute_check(body: &str) -> u32 {
    let sum: u32 = body.chars().enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap())
        .sum();
    37 - (sum % 37)
}

pub fn verify_mt_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "MT") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 8 {
        return Verdict::Invalid { reason: format!("expected 8 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let body = &clean[..6];
    let check: u32 = clean[6..].parse().expect("validated above");
    let expected = compute_check(body);
    if expected == check {
        Verdict::Valid {
            formatted: format!("MT{}", clean),
            detected: "Maltese VAT".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("MT VAT check mismatch: expected {:02}, got {:02}", expected, check),
        }
    }
}

pub fn create_mt_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 6 {
        return Err(anyhow!("expected 6 digits (body without check), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit body"));
    }
    let check = compute_check(&clean);
    Ok(format!("MT{}{:02}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mt_vat_valid_15121333() {
        // sum = 1*3+5*4+1*6+2*7+1*8+3*9 = 3+20+6+14+8+27 = 78, 78%37=4, check=33
        match verify_mt_vat("15121333") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn mt_vat_round_trip() {
        let body = "151213";
        let full = create_mt_vat(body, false).unwrap();
        // full = "MT15121333"
        let digits: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_mt_vat(&digits) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn mt_vat_rejects_bad_check() {
        // 15121300 — check should be 33, not 00
        match verify_mt_vat("15121300") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn mt_vat_rejects_wrong_length() {
        match verify_mt_vat("1512133") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
