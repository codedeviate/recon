//! Luxembourg VAT.
//!
//! 8 digits total. First 6 are the body; last 2 are the check.
//! check = first_6 mod 89, zero-padded to 2 digits.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

pub fn verify_lu_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Verdict::Invalid { reason: format!("expected 8 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let body: u64 = clean[..6].parse().expect("validated above");
    let check: u64 = clean[6..].parse().expect("validated above");
    let expected = body % 89;
    if expected == check {
        Verdict::Valid {
            formatted: format!("LU{}", clean),
            detected: "Luxembourg VAT".into(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("LU VAT check mismatch: {} mod 89 = {}, got {}", body, expected, check),
        }
    }
}

pub fn create_lu_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 6 {
        return Err(anyhow!("expected 6 digits (body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let body: u64 = clean.parse().expect("validated above");
    let check = body % 89;
    Ok(format!("LU{}{:02}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lu_vat_valid_10000356() {
        match verify_lu_vat("10000356") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lu_vat_round_trip() {
        let body = "100003";
        let full = create_lu_vat(body, false).unwrap();
        // full = "LU10000356", strip prefix
        let digits: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_lu_vat(&digits) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lu_vat_rejects_bad_check() {
        match verify_lu_vat("10000357") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
