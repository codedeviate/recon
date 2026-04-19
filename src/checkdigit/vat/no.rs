//! Norwegian VAT / MVA (organisasjonsnummer).
//!
//! 9 digits. Weights `[3,2,7,6,5,4,3,2]` on the first 8 digits.
//! `check_raw = 11 - (sum mod 11)`. If `check_raw == 11`, check digit = 0.
//! If `check_raw == 10`, the number is structurally invalid.
//!
//! Source: Brønnøysundregistrene (Norwegian Register Centre), python-stdnum no.mva.
//! Hand-verified: 974760673 → sum=173, 173%11=8, check_raw=3, last digit=3 ✓

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 8] = [3, 2, 7, 6, 5, 4, 3, 2];

pub fn verify_no_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "NO") {
        Ok(body) => body,
        Err(v) => return v,
    };
    // Strip optional "MVA" suffix (users may type "974760673MVA")
    let clean = if clean.to_ascii_uppercase().ends_with("MVA") {
        clean[..clean.len() - 3].to_string()
    } else {
        clean
    };
    if clean.len() != 9 {
        return Verdict::Invalid {
            reason: format!("expected 9 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid {
            reason: "non-digit input".into(),
        };
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let sum: u32 = digits[..8].iter().zip(WEIGHTS.iter()).map(|(d, w)| d * w).sum();
    let check_raw = 11 - (sum % 11);
    let check = if check_raw == 11 {
        0
    } else if check_raw == 10 {
        return Verdict::Invalid {
            reason: "NO VAT check_raw == 10: structurally invalid number".into(),
        };
    } else {
        check_raw
    };
    if check == digits[8] {
        Verdict::Valid {
            formatted: format!("NO{}", clean),
            detected: "Norwegian VAT / MVA (organisasjonsnummer)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("NO VAT check mismatch: expected {}, got {}", check, digits[8]),
        }
    }
}

pub fn create_no_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits (body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let sum: u32 = digits.iter().zip(WEIGHTS.iter()).map(|(d, w)| d * w).sum();
    let check_raw = 11 - (sum % 11);
    if check_raw == 10 {
        return Err(anyhow!("this 8-digit body produces check_raw=10, which is structurally invalid in the NO MVA scheme"));
    }
    let check = if check_raw == 11 { 0 } else { check_raw };
    Ok(format!("NO{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brønnøysundregistrene's own orgnr — verified by hand.
    /// sum=173, 173%11=8, check_raw=3, last digit 3 ✓
    #[test]
    fn no_vat_reference_974760673() {
        match verify_no_vat("974760673") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "NO974760673");
                assert!(detected.contains("Norwegian"));
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn no_vat_accepts_no_prefix() {
        match verify_no_vat("NO974760673") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn no_vat_accepts_mva_suffix() {
        match verify_no_vat("974760673MVA") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn no_vat_rejects_bad_length() {
        match verify_no_vat("97476067") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn no_vat_rejects_wrong_check() {
        match verify_no_vat("974760674") {
            Verdict::Invalid { reason } => assert!(reason.contains("check mismatch")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn no_vat_round_trip() {
        // Use first 8 digits of 974760673
        let body = "97476067";
        let full = create_no_vat(body, false).unwrap();
        assert_eq!(full, "NO974760673");
        match verify_no_vat(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
