//! Greek VAT (ΑΦΜ — Arithmos Forologikou Mitroou).
//!
//! 9 digits. Weights are powers of 2: [256, 128, 64, 32, 16, 8, 4, 2] on the
//! first 8 digits. Check digit = (sum % 11) % 10.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 8] = [256, 128, 64, 32, 16, 8, 4, 2];

fn compute_check(body: &str) -> u32 {
    let sum: u32 = body.chars().enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap())
        .sum();
    (sum % 11) % 10
}

pub fn verify_el_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "EL") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 9 {
        return Verdict::Invalid { reason: format!("expected 9 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let body = &clean[..8];
    let check: u32 = clean.chars().nth(8).unwrap().to_digit(10).unwrap();
    let expected = compute_check(body);
    if expected == check {
        Verdict::Valid {
            formatted: format!("EL{}", clean),
            detected: "Greek VAT".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("EL VAT check mismatch: expected {}, got {}", expected, check),
        }
    }
}

pub fn create_el_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits (body without check), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit body"));
    }
    let check = compute_check(&clean);
    Ok(format!("EL{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_vat_valid_094259216() {
        match verify_el_vat("094259216") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn el_vat_round_trip() {
        let body = "09425921";
        let full = create_el_vat(body, false).unwrap();
        // full = "EL094259216"
        let digits: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_el_vat(&digits) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn el_vat_rejects_bad_check() {
        // 094259210 — check digit should be 6, not 0
        match verify_el_vat("094259210") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn el_vat_rejects_wrong_length() {
        match verify_el_vat("12345678") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn el_vat_accepts_el_prefix() {
        match verify_el_vat("EL094259216") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn el_vat_accepts_gr_prefix_alias() {
        match verify_el_vat("GR094259216") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
