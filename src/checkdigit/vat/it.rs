//! Italian VAT (partita IVA).
//!
//! 11 digits. The full 11-digit number must pass a Luhn mod-10 check.

use super::super::{sanitize, Verdict};
use super::super::luhn::{luhn_verify, luhn_check_digit};
use anyhow::{anyhow, Result};

pub fn verify_it_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "IT") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 11 {
        return Verdict::Invalid {
            reason: format!("expected 11 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    if luhn_verify(&clean) {
        Verdict::Valid {
            formatted: format!("IT{}", clean),
            detected: "Italian VAT (partita IVA)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid { reason: "IT VAT Luhn check failed".into() }
    }
}

pub fn create_it_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 10 {
        return Err(anyhow!("expected 10 digits (body without check digit), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let check = luhn_check_digit(&clean)?;
    Ok(format!("IT{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_vat_valid_00743110157() {
        match verify_it_vat("00743110157") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn it_vat_round_trip() {
        let body = "0074311015";
        let full = create_it_vat(body, false).unwrap();
        let digits: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_it_vat(&digits) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn it_vat_rejects_bad_check() {
        match verify_it_vat("00743110158") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn it_vat_accepts_it_prefix() {
        match verify_it_vat("IT00743110157") {
            Verdict::Valid { formatted, .. } => assert_eq!(formatted, "IT00743110157"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn it_vat_accepts_lowercase_prefix() {
        match verify_it_vat("it00743110157") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn it_vat_rejects_fr_prefix() {
        match verify_it_vat("FR00743110157") {
            Verdict::Invalid { reason } => {
                assert!(reason.contains("IT"));
                assert!(reason.contains("FR"));
            }
            v => panic!("{:?}", v),
        }
    }
}
