//! Czech Republic check digits.
//!
//! # IČO — Identifikační číslo osoby (legal entity)
//!
//! 8 digits. Weights `[8, 7, 6, 5, 4, 3, 2]` on first 7 digits.
//! `sum = Σ weights[i] * digits[i]`
//! `mod = sum % 11`
//!
//! Check digit rules:
//! - mod == 0  → check = 1
//! - mod == 1  → check = 0
//! - mod == 10 → check = 1
//! - else      → check = 11 - mod
//!
//! Known-valid: `46505334`. Sum = 139, 139 % 11 = 7, check = 11 - 7 = 4. ✓
//!
//! # Rodné číslo (personal code)
//!
//! - 9 digits: historical form issued before 1954. No check digit. Accepted
//!   as a valid format (structural check only — all digits).
//! - 10 digits: modern form. The entire 10-digit number must be divisible by 11.
//!
//! Known-valid 10-digit: `7301011234` — 7301011234 % 11 = 0. ✓
//!
//! # cz-vat — auto-detect
//!
//! Length 8  → IČO (legal entity).
//! Length 9  → rodné číslo pre-1954 (no check digit).
//! Length 10 → rodné číslo modern (divisible by 11).
//! Other     → Invalid.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

// ── IČO (legal entity, 8 digits) ─────────────────────────────────────────────

const ICO_WEIGHTS: [u64; 7] = [8, 7, 6, 5, 4, 3, 2];

fn ico_check(body: &[u32]) -> u32 {
    let sum: u64 = body.iter().zip(ICO_WEIGHTS.iter()).map(|(d, w)| *d as u64 * w).sum();
    let modulo = (sum % 11) as u32;
    match modulo {
        0 => 1,
        1 => 0,
        10 => 1,
        m => 11 - m,
    }
}

/// Verify a Czech IČO (8-digit legal entity number).
pub fn verify_cz_legal(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Verdict::Invalid {
            reason: format!("Czech IČO requires 8 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let expected = ico_check(&digits[..7]);
    let got = digits[7];
    if expected == got {
        Verdict::Valid {
            formatted: clean,
            detected: "Czech IČO (legal entity)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("IČO check mismatch: expected {}, got {}", expected, got),
        }
    }
}

/// Create a Czech IČO by appending the check digit to a 7-digit body.
pub fn create_cz_legal(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 7 {
        return Err(anyhow!(
            "expected 7 digits (body without check digit), got {}",
            clean.len()
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let check = ico_check(&digits);
    Ok(format!("{}{}", clean, check))
}

// ── Rodné číslo (personal code, 9 or 10 digits) ──────────────────────────────

/// Verify a Czech rodné číslo.
///
/// - 9 digits: pre-1954 historical form with no check digit. Accepted as valid
///   if all characters are digits.
/// - 10 digits: modern form where the entire number must be divisible by 11.
pub fn verify_cz_person(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    match clean.len() {
        9 => {
            if !clean.chars().all(|c| c.is_ascii_digit()) {
                return Verdict::Invalid { reason: "non-digit input".into() };
            }
            Verdict::Valid {
                formatted: clean,
                detected: "Czech rodné číslo (pre-1954, no check digit)".into(),
                comment: String::new(),
            }
        }
        10 => {
            if !clean.chars().all(|c| c.is_ascii_digit()) {
                return Verdict::Invalid { reason: "non-digit input".into() };
            }
            let value: u64 = clean.parse().map_err(|_| ()).unwrap_or(u64::MAX);
            if value % 11 == 0 {
                Verdict::Valid {
                    formatted: clean,
                    detected: "Czech rodné číslo (personal code)".into(),
                    comment: String::new(),
                }
            } else {
                Verdict::Invalid {
                    reason: format!(
                        "rodné číslo check failed: {} is not divisible by 11",
                        clean
                    ),
                }
            }
        }
        n => Verdict::Invalid {
            reason: format!("Czech rodné číslo requires 9 or 10 digits, got {}", n),
        },
    }
}

/// Create a 10-digit Czech rodné číslo by appending a check digit to a 9-digit body.
///
/// Finds the digit 0-9 that, when appended, makes the 10-digit number divisible by 11.
/// Returns an error if no such digit exists (rare but possible for certain bodies).
pub fn create_cz_person(input: &str, _raw: bool) -> Result<String> {
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
    let body: u64 = clean.parse()?;
    // The 10-digit number = body * 10 + check. Find check where (body*10+check) % 11 == 0.
    let base = (body * 10) % 11;
    // We need (base + check) % 11 == 0, i.e., check % 11 == (11 - base) % 11
    let needed = (11 - base) % 11;
    if needed <= 9 {
        Ok(format!("{}{}", clean, needed))
    } else {
        Err(anyhow!(
            "no single check digit (0-9) makes the 10-digit number divisible by 11 for body {}",
            clean
        ))
    }
}

// ── cz-vat auto-detect ────────────────────────────────────────────────────────

/// Verify a Czech VAT number, auto-detecting variant by length.
///
/// - 8 digits  → IČO (legal entity)
/// - 9 digits  → rodné číslo pre-1954 (no check digit)
/// - 10 digits → rodné číslo modern (divisible by 11)
pub fn verify_cz_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "CZ") {
        Ok(body) => body,
        Err(v) => return v,
    };
    match clean.len() {
        8 => verify_cz_legal(&clean),
        9 | 10 => verify_cz_person(&clean),
        n => Verdict::Invalid {
            reason: format!(
                "Czech VAT: expected 8 digits (IČO) or 9-10 digits (rodné číslo), got {}",
                n
            ),
        },
    }
}

/// Creating a Czech VAT number requires specifying the variant (cz-legal or cz-person).
pub fn create_cz_vat(_input: &str, _raw: bool) -> Result<String> {
    Err(anyhow!(
        "cz-vat auto-detect cannot create; use cz-legal (IČO) or cz-person (rodné číslo)"
    ))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // IČO tests

    #[test]
    fn cz_legal_valid_46505334() {
        // sum = 4*8+6*7+5*6+0*5+5*4+3*3+3*2 = 32+42+30+0+20+9+6 = 139
        // 139 % 11 = 7, check = 11 - 7 = 4. Last digit = 4 ✓
        match verify_cz_legal("46505334") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Czech IČO (legal entity)"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cz_legal_rejects_bad_check() {
        // Change last digit from 4 to 5
        match verify_cz_legal("46505335") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cz_legal_rejects_wrong_length() {
        match verify_cz_legal("1234567") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cz_legal_round_trip() {
        let body = "4650533";
        let full = create_cz_legal(body, false).unwrap();
        assert_eq!(full, "46505334");
        match verify_cz_legal(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    // mod == 0 special case: check = 1
    #[test]
    fn cz_legal_mod0_gives_check1() {
        // Find a body where sum % 11 == 0.
        // Try body 0000001: sum = 0+0+0+0+0+0+1*2 = 2. Not 0.
        // body 0000010: sum = 0+0+0+0+0+1*3+0 = 3. Not 0.
        // body 0000055: sum = 0+0+0+0+0+5*3+5*2 = 15+10 = 25. 25%11=3.
        // body 0001100: sum = 0+0+0+1*5+1*4+0+0 = 9.
        // body 1000001: sum = 1*8+0+0+0+0+0+1*2 = 10. mod=10 → check=1.
        // body 1000010: sum = 1*8+0+0+0+0+1*3+0 = 11. mod=0 → check=1.
        let full = create_cz_legal("1000010", false).unwrap();
        assert_eq!(&full[7..], "1");
        match verify_cz_legal(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    // mod == 1 special case: check = 0
    #[test]
    fn cz_legal_mod1_gives_check0() {
        // body 1000011: sum = 1*8+0+0+0+0+1*3+1*2 = 8+3+2 = 13. 13%11=2.
        // body 1000020: sum = 1*8+0+0+0+0+2*3+0 = 8+6=14. 14%11=3.
        // body 0000100: sum = 0+0+0+0+1*4+0+0 = 4.
        // body 0001000: sum = 0+0+0+1*5+0+0+0 = 5.
        // body 0010000: sum = 0+0+1*6+0+0+0+0 = 6.
        // body 0100000: sum = 0+1*7+0+0+0+0+0 = 7.
        // body 1000000: sum = 1*8 = 8.
        // body 1100000: sum = 1*8+1*7 = 15. 15%11=4.
        // body 2000000: sum = 2*8 = 16. 16%11=5.
        // body 1200000: sum = 8+14=22. 22%11=0 → check=1.
        // body 1000100: sum = 8+4=12. 12%11=1 → check=0!
        let full = create_cz_legal("1000100", false).unwrap();
        assert_eq!(&full[7..], "0");
        match verify_cz_legal(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    // Rodné číslo tests

    #[test]
    fn cz_person_valid_10digit_7301011234() {
        // 7301011234 % 11 = 0 ✓
        match verify_cz_person("7301011234") {
            Verdict::Valid { detected, .. } => {
                assert_eq!(detected, "Czech rodné číslo (personal code)")
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cz_person_valid_9digit_pre1954() {
        // Any 9-digit all-digit string is accepted as pre-1954 form
        match verify_cz_person("530101001") {
            Verdict::Valid { detected, .. } => {
                assert_eq!(detected, "Czech rodné číslo (pre-1954, no check digit)")
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cz_person_rejects_not_divisible_by_11() {
        // 7301011235 — last digit bumped by 1
        match verify_cz_person("7301011235") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cz_person_rejects_wrong_length() {
        match verify_cz_person("12345678") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cz_person_round_trip_10digit() {
        let body = "730101123";
        let full = create_cz_person(body, false).unwrap();
        assert_eq!(full, "7301011234");
        match verify_cz_person(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    // Auto-detect tests

    #[test]
    fn cz_vat_autodetect_8digit_is_ico() {
        match verify_cz_vat("46505334") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Czech IČO (legal entity)"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cz_vat_autodetect_10digit_is_person() {
        match verify_cz_vat("7301011234") {
            Verdict::Valid { detected, .. } => {
                assert_eq!(detected, "Czech rodné číslo (personal code)")
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cz_vat_autodetect_9digit_is_pre1954() {
        match verify_cz_vat("530101001") {
            Verdict::Valid { detected, .. } => {
                assert_eq!(detected, "Czech rodné číslo (pre-1954, no check digit)")
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cz_vat_rejects_wrong_length() {
        match verify_cz_vat("12345678901") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cz_vat_create_returns_err() {
        assert!(create_cz_vat("anything", false).is_err());
    }
}
