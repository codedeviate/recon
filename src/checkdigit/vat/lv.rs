//! Latvian VAT / identification numbers.
//!
//! # Personal code (first digit 0-3): 11 digits
//!
//! Format: `DDMMYY-XNNNC` where X is a century marker
//! (1 = 1800s, 2 = 1900s, see OCMA notes).
//!
//! Algorithm (per official Latvian Register and PEAR Validate_LV):
//! Apply weights `[1, 6, 3, 7, 9, 10, 5, 8, 4, 2, 1]` to ALL 11 digits.
//! The number is valid if `sum % 11 == 1`.
//!
//! Equivalently (for create): `check = (1101 - sum_10) % 11 % 10`
//! where `sum_10` uses weights `[1, 6, 3, 7, 9, 10, 5, 8, 4, 2]` on first 10 digits.
//!
//! Known-valid synthetic: `111111-11111`
//! - sum = 1+6+3+7+9+10+5+8+4+2+1 = 56. 56 % 11 = 1. ✓
//!
//! # Business registration number (first digit 4-9): 11 digits
//!
//! Weights `[9, 1, 4, 8, 3, 10, 2, 5, 7, 6]` on first 10 digits.
//! `check = ((3 - sum % 11) % 11) % 10`
//!
//! Known-valid: `40003032949`
//! - Digits: 4,0,0,0,3,0,3,2,9,4 — check: 9
//! - Sum = 4*9+0*1+0*4+0*8+3*3+0*10+3*2+2*5+9*7+4*6 = 36+0+0+0+9+0+6+10+63+24 = 148
//! - 148 % 11 = 5. (3-5) % 11 = (-2+11) % 11 = 9. 9 % 10 = 9. ✓
//!
//! # lv-vat — auto-detect
//!
//! Length 11:
//!   - First digit 0-3 → personal code.
//!   - First digit 4-9 → business registration number.
//!
//! Other length → Invalid.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

// ── Personal code (first digit 0-3) ──────────────────────────────────────────

/// Weights applied to all 11 digits for personal code verify.
const PERSONAL_VERIFY_WEIGHTS: [u32; 11] = [1, 6, 3, 7, 9, 10, 5, 8, 4, 2, 1];
/// Weights applied to first 10 digits for personal code create.
const PERSONAL_CREATE_WEIGHTS: [u32; 10] = [1, 6, 3, 7, 9, 10, 5, 8, 4, 2];

/// Verify a Latvian personal code (first digit 0-3, 11 digits).
///
/// Algorithm verified against PEAR Validate_LV and the Wikipedia formula:
/// weights `[1,6,3,7,9,10,5,8,4,2,1]` on all 11 digits; valid if sum % 11 == 1.
pub fn verify_lv_personal(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 11 {
        return Verdict::Invalid {
            reason: format!("Latvian personal code requires 11 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let first = digits[0];
    if first > 3 {
        return Verdict::Invalid {
            reason: format!(
                "Latvian personal code first digit must be 0-3, got {}",
                first
            ),
        };
    }
    let sum: u32 = digits.iter().zip(PERSONAL_VERIFY_WEIGHTS.iter()).map(|(d, w)| d * w).sum();
    if sum % 11 == 1 {
        Verdict::Valid {
            formatted: clean,
            detected: "Latvian personal code".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!(
                "Latvian personal code check failed: weighted sum {} % 11 = {}, expected 1",
                sum,
                sum % 11
            ),
        }
    }
}

/// Create a Latvian personal code by appending the check digit to a 10-digit body.
pub fn create_lv_personal(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 10 {
        return Err(anyhow!(
            "expected 10 digits (body without check digit), got {}",
            clean.len()
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let first = digits[0];
    if first > 3 {
        return Err(anyhow!(
            "Latvian personal code first digit must be 0-3, got {}",
            first
        ));
    }
    let sum_10: u32 = digits.iter().zip(PERSONAL_CREATE_WEIGHTS.iter()).map(|(d, w)| d * w).sum();
    // We need (sum_10 + check) % 11 == 1, i.e., check = (1 - sum_10 % 11 + 11) % 11
    // If that gives 10, use 0 (the last weight is 1 so check*1=check; sum%11==1 still holds
    // only if check==10 maps to 0, but 0*1=0 and sum_10%11 would need to be 1 already — handled).
    let sum_mod = sum_10 % 11;
    let check_digit = ((11 + 1 - sum_mod) % 11) % 10;
    Ok(format!("{}{}", clean, check_digit))
}

// ── Business registration number (first digit 4-9) ───────────────────────────

const BUSINESS_WEIGHTS: [u32; 10] = [9, 1, 4, 8, 3, 10, 2, 5, 7, 6];

fn business_check(body: &[u32]) -> u32 {
    let sum: u32 = body.iter().zip(BUSINESS_WEIGHTS.iter()).map(|(d, w)| d * w).sum();
    let m = sum % 11;
    // (3 - m) % 11 in a way that handles underflow
    let v = (3u32 + 11 - m) % 11;
    v % 10
}

/// Verify a Latvian business registration number (first digit 4-9, 11 digits).
pub fn verify_lv_business(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 11 {
        return Verdict::Invalid {
            reason: format!(
                "Latvian business number requires 11 digits, got {}",
                clean.len()
            ),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let first = digits[0];
    if first < 4 {
        return Verdict::Invalid {
            reason: format!(
                "Latvian business number first digit must be 4-9, got {}",
                first
            ),
        };
    }
    let expected = business_check(&digits[..10]);
    let got = digits[10];
    if expected == got {
        Verdict::Valid {
            formatted: clean,
            detected: "Latvian business registration number".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!(
                "Latvian business check mismatch: expected {}, got {}",
                expected, got
            ),
        }
    }
}

/// Create a Latvian business registration number by appending the check digit.
pub fn create_lv_business(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 10 {
        return Err(anyhow!(
            "expected 10 digits (body without check digit), got {}",
            clean.len()
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let first = digits[0];
    if first < 4 {
        return Err(anyhow!(
            "Latvian business number first digit must be 4-9, got {}",
            first
        ));
    }
    let check = business_check(&digits);
    Ok(format!("{}{}", clean, check))
}

// ── lv-vat auto-detect ────────────────────────────────────────────────────────

/// Verify a Latvian VAT / identification number, auto-detecting by first digit.
///
/// - First digit 0-3 → personal code
/// - First digit 4-9 → business registration number
pub fn verify_lv_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "LV") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 11 {
        return Verdict::Invalid {
            reason: format!("Latvian VAT requires 11 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let first = clean.chars().next().unwrap().to_digit(10).unwrap();
    if first <= 3 {
        verify_lv_personal(&clean)
    } else {
        verify_lv_business(&clean)
    }
}

/// Creating an LV VAT number requires specifying the variant (lv-personal or lv-business).
pub fn create_lv_vat(_input: &str, _raw: bool) -> Result<String> {
    Err(anyhow!(
        "lv-vat auto-detect cannot create; use lv-personal or lv-business to specify the variant"
    ))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Business tests

    #[test]
    fn lv_business_valid_40003032949() {
        // Sum = 36+0+0+0+9+0+6+10+63+24 = 148; 148%11=5; (3-5+11)%11=9; 9%10=9. ✓
        match verify_lv_business("40003032949") {
            Verdict::Valid { detected, .. } => {
                assert_eq!(detected, "Latvian business registration number")
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lv_business_rejects_bad_check() {
        // Change last digit from 9 to 0
        match verify_lv_business("40003032940") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lv_business_rejects_wrong_length() {
        match verify_lv_business("4000303294") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lv_business_round_trip() {
        let body = "4000303294";
        let full = create_lv_business(body, false).unwrap();
        assert_eq!(full, "40003032949");
        match verify_lv_business(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lv_business_rejects_personal_prefix() {
        // First digit 3 → should be rejected as not a business number
        match verify_lv_business("30003032949") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    // Personal tests

    #[test]
    fn lv_personal_valid_11111111111() {
        // sum = 1*1+1*6+1*3+1*7+1*9+1*10+1*5+1*8+1*4+1*2+1*1 = 56
        // 56 % 11 = 1 ✓
        match verify_lv_personal("11111111111") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Latvian personal code"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lv_personal_round_trip() {
        let body = "1111111111";
        let full = create_lv_personal(body, false).unwrap();
        assert_eq!(&full, "11111111111");
        match verify_lv_personal(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lv_personal_rejects_bad_check() {
        // Change last digit from 1 to 2
        match verify_lv_personal("11111111112") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lv_personal_rejects_business_prefix() {
        // First digit 4 → should be rejected as not a personal code
        match verify_lv_personal("40003032949") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lv_personal_rejects_wrong_length() {
        match verify_lv_personal("1111111111") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    // Auto-detect tests

    #[test]
    fn lv_vat_autodetect_business() {
        match verify_lv_vat("40003032949") {
            Verdict::Valid { detected, .. } => {
                assert_eq!(detected, "Latvian business registration number")
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lv_vat_autodetect_personal() {
        match verify_lv_vat("11111111111") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Latvian personal code"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lv_vat_rejects_wrong_length() {
        match verify_lv_vat("1234567890") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lv_vat_create_returns_err() {
        assert!(create_lv_vat("anything", false).is_err());
    }
}
