//! Moldovan IDNO (Numărul de Identificare de Stat al Organizaţiei).
//!
//! 13 digits. Used as the Moldovan company/organisation tax registration number.
//! Note: python-stdnum has `md/idno.py` (company), not `idnp.py` (personal);
//! this implements the IDNO — the VAT-relevant identifier.
//!
//! # Algorithm
//!
//! Weights `(7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1)` applied to the first 12
//! digits. `check = sum mod 10`. Must equal the 13th digit.
//!
//! ```text
//! weights = (7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1)
//! check = (Σ wᵢ × dᵢ for i in 0..12) mod 10
//! ```
//!
//! **Test vector:** `1008600038413` (from python-stdnum md.idno doctest).
//!
//! Hand-verified `1008600038413` (body `100860003841`, check `3`):
//! - Positions: 1×7, 0×3, 0×1, 8×7, 6×3, 0×1, 0×7, 0×3, 3×1, 8×7, 4×3, 1×1
//! - Products: 7, 0, 0, 56, 18, 0, 0, 0, 3, 56, 12, 1
//! - Sum = 153; 153 mod 10 = 3. 13th digit = 3. ✓
//!
//! Sources:
//! - python-stdnum md.idno (doctest vector 1008600038413)
//! - Agenția Servicii Publice (ASP) Moldova — company registration database

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 12] = [7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1];

fn compute_check(body: &str) -> u32 {
    // body must be exactly 12 ASCII digits
    let sum: u32 = body
        .chars()
        .zip(WEIGHTS.iter())
        .map(|(c, &w)| c.to_digit(10).unwrap() * w)
        .sum();
    sum % 10
}

/// Verify a Moldovan IDNO (13 digits, weighted mod-10).
pub fn verify_md_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "MD") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 13 {
        return Verdict::Invalid {
            reason: format!("MD IDNO: expected 13 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let body = &clean[..12];
    let check: u32 = clean.chars().nth(12).unwrap().to_digit(10).unwrap();
    let expected = compute_check(body);
    if expected == check {
        Verdict::Valid {
            formatted: format!("MD{}", clean),
            detected: "Moldovan IDNO".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("MD IDNO check mismatch: expected {}, got {}", expected, check),
        }
    }
}

/// Create a Moldovan IDNO from a 12-digit body.
pub fn create_md_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 12 {
        return Err(anyhow!(
            "MD IDNO: expected 12 digits (body without check digit), got {}",
            clean.len()
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let check = compute_check(&clean);
    Ok(format!("MD{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// python-stdnum md.idno doctest vector. Hand-verified: check=3. ✓
    #[test]
    fn md_vat_valid_1008600038413() {
        match verify_md_vat("1008600038413") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "MD1008600038413");
                assert_eq!(detected, "Moldovan IDNO");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn md_vat_rejects_wrong_length() {
        match verify_md_vat("100860003841") {
            Verdict::Invalid { reason } => assert!(reason.contains("expected 13 digits")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn md_vat_round_trip() {
        let body = "100860003841";
        let full = create_md_vat(body, false).unwrap();
        assert_eq!(full, "MD1008600038413");
        match verify_md_vat(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn md_vat_rejects_bad_check() {
        // 1008600038412 — wrong check digit (should be 3)
        match verify_md_vat("1008600038412") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
