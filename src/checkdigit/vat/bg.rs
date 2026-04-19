//! Bulgarian check digits.
//!
//! # EGN — Единен граждански номер (personal code)
//!
//! 10 digits: `YY MM DD NNN C`.
//!
//! The month field encodes the century:
//! - `01–12` → 1900–1999 (use raw month)
//! - `21–32` → 2000–2099 (real month = MM − 20)
//! - `41–52` → 1800–1899 (real month = MM − 40, rare/historical)
//!
//! The birth-date `(DD, real_MM, YY)` must be a valid calendar date.
//!
//! Check-digit weights: `[2, 4, 8, 5, 10, 9, 7, 3, 6]` on digits 1–9.
//! `check = sum % 11`. If `check == 10`, use 0 instead.
//!
//! Known-valid: `7523169263`.
//!
//! # BULSTAT — legal entity registration number
//!
//! 9 digits. Primary weights `[1, 2, 3, 4, 5, 6, 7, 8]` on digits 1–8.
//! `check = sum % 11`.
//! If `check == 10`, apply secondary weights `[3, 4, 5, 6, 7, 8, 9, 10]`.
//! `check = sum % 11`. If still `10`, the number is invalid.
//!
//! Known-valid: `175074752`.
//!
//! # bg-vat — auto-detect
//!
//! Length 10 → EGN, length 9 → BULSTAT, other → invalid.

use super::super::{sanitize, Verdict};
use super::super::country_id::valid_ddmmyy;
use anyhow::{anyhow, Result};

// ── EGN ─────────────────────────────────────────────────────────────────────

const EGN_WEIGHTS: [u32; 9] = [2, 4, 8, 5, 10, 9, 7, 3, 6];

/// Decode the century-encoded month field.
/// Returns `(real_month, century_prefix)` where century_prefix is 1800/1900/2000.
pub fn decode_bg_century(mm: u32) -> (u32, u32) {
    if mm >= 41 {
        (mm - 40, 1800)
    } else if mm >= 21 {
        (mm - 20, 2000)
    } else {
        (mm, 1900)
    }
}

fn egn_check(body: &[u32]) -> u32 {
    // body must have exactly 9 digits
    let sum: u32 = body.iter().zip(EGN_WEIGHTS.iter()).map(|(d, w)| d * w).sum();
    let c = sum % 11;
    if c == 10 { 0 } else { c }
}

pub fn verify_bg_egn(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 10 {
        return Verdict::Invalid {
            reason: format!("Bulgarian EGN requires 10 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();

    let yy: u32 = digits[0] * 10 + digits[1];
    let mm_raw: u32 = digits[2] * 10 + digits[3];
    let dd: u32 = digits[4] * 10 + digits[5];
    let (real_mm, _century) = decode_bg_century(mm_raw);

    if !valid_ddmmyy(dd, real_mm, yy, false) {
        return Verdict::Invalid {
            reason: format!(
                "invalid date in EGN: DD={}, encoded MM={}, real month={}",
                dd, mm_raw, real_mm
            ),
        };
    }

    let expected = egn_check(&digits[..9]);
    let got = digits[9];
    if expected == got {
        Verdict::Valid {
            formatted: clean.clone(),
            detected: "Bulgarian EGN".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("EGN check mismatch: expected {}, got {}", expected, got),
        }
    }
}

pub fn create_bg_egn(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!(
            "expected 9 digits (body without check digit), got {}",
            clean.len()
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();

    let yy: u32 = digits[0] * 10 + digits[1];
    let mm_raw: u32 = digits[2] * 10 + digits[3];
    let dd: u32 = digits[4] * 10 + digits[5];
    let (real_mm, _century) = decode_bg_century(mm_raw);

    if !valid_ddmmyy(dd, real_mm, yy, false) {
        return Err(anyhow!(
            "invalid date in EGN body: DD={}, encoded MM={}, real month={}",
            dd, mm_raw, real_mm
        ));
    }

    let check = egn_check(&digits);
    Ok(format!("{}{}", clean, check))
}

// ── BULSTAT ──────────────────────────────────────────────────────────────────

const BULSTAT_PRIMARY: [u32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
const BULSTAT_SECONDARY: [u32; 8] = [3, 4, 5, 6, 7, 8, 9, 10];

/// Returns `Some(check)` or `None` if the number is invalid (both algorithms give 10).
fn bulstat_check(body: &[u32]) -> Option<u32> {
    let s1: u32 = body.iter().zip(BULSTAT_PRIMARY.iter()).map(|(d, w)| d * w).sum();
    let c1 = s1 % 11;
    if c1 != 10 {
        return Some(c1);
    }
    let s2: u32 = body.iter().zip(BULSTAT_SECONDARY.iter()).map(|(d, w)| d * w).sum();
    let c2 = s2 % 11;
    if c2 == 10 {
        None // both algorithms yield 10 → invalid combination
    } else {
        Some(c2)
    }
}

pub fn verify_bg_bulstat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Verdict::Invalid {
            reason: format!("Bulgarian BULSTAT requires 9 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let got = digits[8];

    match bulstat_check(&digits[..8]) {
        None => Verdict::Invalid {
            reason: "BULSTAT invalid: both primary and secondary algorithms yield 10".into(),
        },
        Some(expected) if expected == got => Verdict::Valid {
            formatted: clean.clone(),
            detected: "Bulgarian BULSTAT".into(),
            comment: String::new(),
        },
        Some(expected) => Verdict::Invalid {
            reason: format!("BULSTAT check mismatch: expected {}, got {}", expected, got),
        },
    }
}

pub fn create_bg_bulstat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!(
            "expected 8 digits (body without check digit), got {}",
            clean.len()
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    match bulstat_check(&digits) {
        None => Err(anyhow!(
            "BULSTAT body invalid: both primary and secondary algorithms yield 10"
        )),
        Some(check) => Ok(format!("{}{}", clean, check)),
    }
}

// ── bg-vat auto-detect ───────────────────────────────────────────────────────

pub fn verify_bg_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    match clean.len() {
        10 => verify_bg_egn(input),
        9 => verify_bg_bulstat(input),
        other => Verdict::Invalid {
            reason: format!(
                "Bulgarian VAT: expected 9 digits (BULSTAT) or 10 digits (EGN), got {}",
                other
            ),
        },
    }
}

pub fn create_bg_vat(_input: &str, _raw: bool) -> Result<String> {
    Err(anyhow!(
        "bg-vat auto-detect cannot create; use bg-egn or bg-bulstat to specify the variant"
    ))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // EGN tests

    #[test]
    fn bg_egn_valid_7523169263() {
        // Known-valid EGN: 7523169263
        // MM=23 → century=2000, real_mm=3, DD=16, YY=75 → valid date
        // check: sum=234, 234%11=3 ✓
        match verify_bg_egn("7523169263") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Bulgarian EGN"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn bg_egn_rejects_invalid_month() {
        // MM=99 is outside all valid century ranges (max encoded month is 52)
        // even if parsed as real month it would be 99 which is > 12
        match verify_bg_egn("7599169263") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn bg_egn_round_trip() {
        // Body (9 digits): 752316926 — same date as known-valid
        let body = "752316926";
        let full = create_bg_egn(body, false).unwrap();
        assert_eq!(full, "7523169263");
        match verify_bg_egn(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    // BULSTAT tests

    #[test]
    fn bg_bulstat_valid_175074752() {
        // Known-valid BULSTAT: 175074752
        // sum = 1*1+7*2+5*3+0*4+7*5+4*6+7*7+5*8 = 1+14+15+0+35+24+49+40 = 178
        // 178 % 11 = 2. Check digit = 2. Last digit of 175074752 = 2 ✓
        match verify_bg_bulstat("175074752") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Bulgarian BULSTAT"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn bg_bulstat_round_trip() {
        let body = "17507475";
        let full = create_bg_bulstat(body, false).unwrap();
        assert_eq!(full, "175074752");
        match verify_bg_bulstat(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn bg_bulstat_rejects_bad_check() {
        // 175074750 — check should be 2, not 0
        match verify_bg_bulstat("175074750") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    // Auto-detect tests

    #[test]
    fn bg_vat_autodetect_10_digits_uses_egn() {
        match verify_bg_vat("7523169263") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Bulgarian EGN"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn bg_vat_autodetect_9_digits_uses_bulstat() {
        match verify_bg_vat("175074752") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Bulgarian BULSTAT"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn bg_vat_rejects_wrong_length() {
        match verify_bg_vat("12345678") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn bg_vat_create_returns_err() {
        assert!(create_bg_vat("anything", false).is_err());
    }
}
