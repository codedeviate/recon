//! Serbian VAT / PIB (Poreski Identifikacioni Broj).
//!
//! 9 digits. ISO 7064 MOD 11,10 — identical algorithm to Croatian OIB
//! (`src/checkdigit/vat/hr.rs`) but with 9 digits instead of 11.
//!
//! ```text
//! intermediate = 10
//! for each of the first 8 digits d:
//!     sum = (intermediate + d) mod 10
//!     if sum == 0 { sum = 10 }
//!     intermediate = (sum * 2) mod 11
//! check = (11 - intermediate) mod 10
//! ```
//!
//! **Test vector:** `101134702` (from python-stdnum rs.pib doctest).
//!
//! NOTE: The task brief suggested `100001442` as a possible vector, but
//! hand-computation shows that number has check digit 1, not 2 — it is not a
//! valid PIB. `101134702` is the authoritative test vector from python-stdnum.
//!
//! Hand-verified `101134702` (body `10113470`, check `2`):
//! - intermediate=10; d=1: s=1, int=2
//! - d=0: s=2, int=4
//! - d=1: s=5, int=10
//! - d=1: s=1 (note: (10+1)%10=1), int=2
//! - d=3: s=5, int=10
//! - d=4: s=4 (note: (10+4)%10=4), int=8
//! - d=7: s=5 (note: (8+7)%10=5), int=10
//! - d=0: s=0→10, int=(10*2)%11=9
//! - check = (11-9)%10 = 2. 9th digit = 2. ✓
//!
//! Sources:
//! - python-stdnum rs.pib (doctest vector 101134702)
//! - python-stdnum iso7064.mod_11_10 (same algorithm as OIB/HR)
//! - Serbian Tax Administration (Poreska Uprava) PIB specification

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

fn compute_check(body: &str) -> u32 {
    // body must be exactly 8 ASCII digits
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

/// Verify a Serbian PIB (9 digits, ISO 7064 MOD 11,10).
pub fn verify_rs_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "RS") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 9 {
        return Verdict::Invalid {
            reason: format!("RS PIB: expected 9 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let body = &clean[..8];
    let check: u32 = clean.chars().nth(8).unwrap().to_digit(10).unwrap();
    let expected = compute_check(body);
    if expected == check {
        Verdict::Valid {
            formatted: format!("RS{}", clean),
            detected: "Serbian VAT (PIB)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("RS PIB check mismatch: expected {}, got {}", expected, check),
        }
    }
}

/// Create a Serbian PIB from an 8-digit body.
pub fn create_rs_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("RS PIB: expected 8 digits (body without check digit), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let check = compute_check(&clean);
    Ok(format!("RS{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// python-stdnum rs.pib doctest vector. Hand-verified: check=2. ✓
    #[test]
    fn rs_vat_valid_101134702() {
        match verify_rs_vat("101134702") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "RS101134702");
                assert_eq!(detected, "Serbian VAT (PIB)");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn rs_vat_rejects_wrong_length() {
        match verify_rs_vat("12345678") {
            Verdict::Invalid { reason } => assert!(reason.contains("expected 9 digits")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn rs_vat_round_trip() {
        let body = "10113470";
        let full = create_rs_vat(body, false).unwrap();
        assert_eq!(full, "RS101134702");
        match verify_rs_vat(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn rs_vat_rejects_bad_check() {
        // 101134703 — wrong check digit (should be 2)
        match verify_rs_vat("101134703") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
