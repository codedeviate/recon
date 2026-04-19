//! Icelandic VAT / kennitala.
//!
//! 10 digits: `DDMMYY NNN C R` where:
//! - `DDMMYY` = date of birth or registration
//! - `NNN`    = individual sequence (3 digits)
//! - `C`      = check digit (index 8)
//! - `R`      = century indicator: 0=1900s companies, 8=1800s, 9=1900s individuals
//!
//! Algorithm: weights `[3,2,7,6,5,4,3,2]` on digits 0–7.
//! `check = (-sum) mod 11`. If `check == 10`, the kennitala is structurally invalid.
//!
//! Sources:
//! - python-stdnum is_.kennitala (weights verified against 13-entry doctest)
//! - Wikipedia "Kennitala" (confirms mod-11 check digit at position 9, century byte ignored)
//!
//! Hand-verified: `0208842749`
//! digits = [0,2,0,8,8,4,2,7,4,9]
//! products = [0,4,0,48,40,16,6,14], sum=128
//! check = (-128) mod 11 = 4, digit[8]=4 ✓

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [i64; 8] = [3, 2, 7, 6, 5, 4, 3, 2];

fn verify_is_vat_body(clean: &str) -> Verdict {
    debug_assert_eq!(clean.len(), 10);
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: Vec<i64> = clean.chars().map(|c| c.to_digit(10).unwrap() as i64).collect();
    let sum: i64 = digits[..8].iter().zip(WEIGHTS.iter()).map(|(d, w)| d * w).sum();
    // Use Euclidean modulo so the result is always 0..=10.
    let check = ((-sum) % 11 + 11) % 11;
    if check == 10 {
        return Verdict::Invalid {
            reason: "IS kennitala check == 10: structurally invalid number".into(),
        };
    }
    if check == digits[8] {
        Verdict::Valid {
            formatted: format!("IS{}", clean),
            detected: "Icelandic VAT / kennitala (10 digits, weighted mod-11)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("IS kennitala check mismatch: expected {}, got {}", check, digits[8]),
        }
    }
}

pub fn verify_is_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "IS") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 10 {
        return Verdict::Invalid {
            reason: format!("expected 10 digits, got {}", clean.len()),
        };
    }
    verify_is_vat_body(&clean)
}

/// Create a kennitala from its components.
///
/// Input: 9 digits — the 8-digit base (DDMMYYNNN) followed by 1 century indicator
/// digit (R: 0, 8, or 9). The check digit is computed and inserted at position 8,
/// producing a 10-digit output.
///
/// Example: `"020884279"` (body=`02088427`, century=`9`) → `"IS0208842749"`
pub fn create_is_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!(
            "expected 9 digits (8-digit base DDMMYYNNN + 1 century indicator), got {}",
            clean.len()
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<i64> = clean.chars().map(|c| c.to_digit(10).unwrap() as i64).collect();
    // digits[0..8] = DDMMYYNNN, digits[8] = century indicator
    let sum: i64 = digits[..8].iter().zip(WEIGHTS.iter()).map(|(d, w)| d * w).sum();
    let check = ((-sum) % 11 + 11) % 11;
    if check == 10 {
        return Err(anyhow!("this base produces check == 10, which is structurally invalid in the IS kennitala scheme"));
    }
    let base = &clean[..8];
    let century = &clean[8..9];
    Ok(format!("IS{}{}{}", base, check, century))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-verified: 0208842749
    /// digits=[0,2,0,8,8,4,2,7,4,9], products=[0,4,0,48,40,16,6,14]
    /// sum=128, check=(-128)%11=4, digit[8]=4 ✓
    #[test]
    fn is_vat_reference_0208842749() {
        match verify_is_vat("0208842749") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "IS0208842749");
                assert!(detected.contains("kennitala"));
            }
            v => panic!("{:?}", v),
        }
    }

    /// Second vector from python-stdnum doctest: 2607565169
    #[test]
    fn is_vat_reference_2607565169() {
        match verify_is_vat("2607565169") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    /// Third vector: 4406032540
    #[test]
    fn is_vat_reference_4406032540() {
        match verify_is_vat("4406032540") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn is_vat_accepts_is_prefix() {
        match verify_is_vat("IS0208842749") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn is_vat_accepts_lowercase_prefix() {
        match verify_is_vat("is0208842749") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    /// Change the check digit at index 8 from 4 to 5: should fail
    #[test]
    fn is_vat_rejects_bad_check() {
        match verify_is_vat("0208842750") {
            Verdict::Invalid { reason } => assert!(reason.contains("check mismatch")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn is_vat_rejects_bad_length() {
        match verify_is_vat("020884274") {
            Verdict::Invalid { reason } => assert!(reason.contains("expected 10")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn is_vat_round_trip() {
        // Input: 8-digit base "02088427" + 1 century indicator "9" = "020884279"
        // Expected output: IS + base + check(4) + century(9) = "IS0208842749"
        let body = "020884279";
        let full = create_is_vat(body, false).unwrap();
        assert_eq!(full, "IS0208842749");
        match verify_is_vat(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
