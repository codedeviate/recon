//! Romanian VAT / CIF (Cod de Identificare Fiscală).
//!
//! Variable length: 2–10 digits total.
//! Body = all digits except the last. Check digit = last digit.
//!
//! Algorithm: pad body to 9 digits with leading zeros, apply weights
//! [7, 5, 3, 2, 1, 7, 5, 3, 2] left-to-right, then:
//!   check = (sum * 10) % 11
//!   if check == 10, check = 0.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u64; 9] = [7, 5, 3, 2, 1, 7, 5, 3, 2];

fn compute_check(body: &str) -> u64 {
    // Pad body to 9 digits with leading zeros
    let padded = format!("{:0>9}", body);
    let sum: u64 = padded
        .chars()
        .enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap() as u64)
        .sum();
    let check = (sum * 10) % 11;
    if check == 10 { 0 } else { check }
}

pub fn verify_ro_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "RO") {
        Ok(body) => body,
        Err(v) => return v,
    };
    let len = clean.len();
    if !(2..=10).contains(&len) {
        return Verdict::Invalid {
            reason: format!("expected 2-10 digits, got {}", len),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let body = &clean[..len - 1];
    let check: u64 = clean[len - 1..].parse().expect("validated above");
    let expected = compute_check(body);
    if expected == check {
        Verdict::Valid {
            formatted: format!("RO{}", clean),
            detected: "Romanian VAT (CIF)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("RO VAT check mismatch: expected {}, got {}", expected, check),
        }
    }
}

pub fn create_ro_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    let len = clean.len();
    // Accept 1–9 digit body; full number will be 2–10 digits.
    if !(1..=9).contains(&len) {
        return Err(anyhow!("expected 1-9 digit body (check digit will be appended), got {}", len));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let check = compute_check(&clean);
    Ok(format!("RO{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ro_vat_valid_18547290() {
        match verify_ro_vat("18547290") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ro_vat_valid_13548146() {
        match verify_ro_vat("13548146") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ro_vat_round_trip() {
        let body = "1854729";
        let full = create_ro_vat(body, false).unwrap();
        let digits: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_ro_vat(&digits) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ro_vat_rejects_bad_check() {
        match verify_ro_vat("18547291") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
