//! Croatian VAT / OIB (Osobni Identifikacijski Broj).
//!
//! 11 digits total. ISO 7064 MOD 11,10 chained multiplication algorithm:
//!
//! ```text
//! intermediate = 10
//! for each of the first 10 digits d:
//!     sum = (intermediate + d) % 10
//!     if sum == 0 { sum = 10 }
//!     intermediate = (sum * 2) % 11
//! check = (11 - intermediate) % 10
//! ```
//!
//! The computed check must equal digit 11.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

fn compute_check(body: &str) -> u32 {
    // body must be exactly 10 ASCII digits
    let mut intermediate: u32 = 10;
    for c in body.chars() {
        let d = c.to_digit(10).unwrap();
        let mut sum = (intermediate + d) % 10;
        if sum == 0 {
            sum = 10;
        }
        intermediate = (sum * 2) % 11;
    }
    (11 - intermediate) % 10
}

pub fn verify_hr_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 11 {
        return Verdict::Invalid {
            reason: format!("expected 11 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let body = &clean[..10];
    let check: u32 = clean.chars().nth(10).unwrap().to_digit(10).unwrap();
    let expected = compute_check(body);
    if expected == check {
        Verdict::Valid {
            formatted: format!("HR{}", clean),
            detected: "Croatian VAT (OIB)".into(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("HR VAT (OIB) check mismatch: expected {}, got {}", expected, check),
        }
    }
}

pub fn create_hr_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 10 {
        return Err(anyhow!("expected 10 digits (body without check digit), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let check = compute_check(&clean);
    Ok(format!("HR{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hr_vat_valid_33392005961() {
        // Known valid OIB: 33392005961
        match verify_hr_vat("33392005961") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn hr_vat_round_trip() {
        let body = "3339200596";
        let full = create_hr_vat(body, false).unwrap();
        let raw = &full[2..]; // strip "HR" prefix
        match verify_hr_vat(raw) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn hr_vat_rejects_bad_check() {
        // 33392005962 — wrong check digit
        match verify_hr_vat("33392005962") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn hr_vat_rejects_wrong_length() {
        match verify_hr_vat("1234567890") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
