//! Ukrainian VAT identifiers — auto-detected by length.
//!
//! - 8 digits → EDRPOU (company registration number)
//! - 10 digits → RNOKPP / RNTRC (individual taxpayer number)
//!
//! # EDRPOU (8 digits)
//!
//! Primary weights `[1,2,3,4,5,6,7]` on first 7 digits.
//! If first digit ∈ {3,4,5}: use `[7,1,2,3,4,5,6]` instead.
//! `total = sum(w * d)`. `check = total % 11`.
//! If `check >= 10`: recalculate with `weights += 2` each, `check = (total % 11) % 10`.
//!
//! Sources:
//! - python-stdnum ua.edrpou (exact code confirmed)
//! - Ukrainian Open Data portal / Prozorro; EDRPOU published by Ministry of Justice
//!
//! Hand-verified: `32855961` (first digit 3 → weights [7,1,2,3,4,5,6])
//! products=[21,2,16,15,20,45,36], sum=155, 155%11=1, check=1, digit[7]=1 ✓
//!
//! # RNOKPP (10 digits)
//!
//! Weights `[-1,5,7,9,4,6,10,5,7]` on first 9 digits.
//! `check = ((sum % 11) % 10)`. Must equal 10th digit.
//!
//! Sources:
//! - python-stdnum ua.rntrc
//! - Ukrainian State Tax Service documentation
//!
//! Hand-verified: `1759013776`
//! weights=[-1,5,7,9,4,6,10,5,7], digits=[1,7,5,9,0,1,3,7,7]
//! sum=270, 270%11=6, 6%10=6, digit[9]=6 ✓

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

// ── EDRPOU ────────────────────────────────────────────────────────────────────

const EDRPOU_WEIGHTS_STD: [u64; 7] = [1, 2, 3, 4, 5, 6, 7];
const EDRPOU_WEIGHTS_345: [u64; 7] = [7, 1, 2, 3, 4, 5, 6];

fn edrpou_calc_check(digits: &[u64; 8]) -> Option<u64> {
    let w = if digits[0] >= 3 && digits[0] <= 5 {
        EDRPOU_WEIGHTS_345
    } else {
        EDRPOU_WEIGHTS_STD
    };
    let total: u64 = digits[..7].iter().zip(w.iter()).map(|(d, wt)| d * wt).sum();
    let r = total % 11;
    if r < 10 {
        return Some(r);
    }
    // Secondary: weights + 2
    let w2: [u64; 7] = [w[0]+2, w[1]+2, w[2]+2, w[3]+2, w[4]+2, w[5]+2, w[6]+2];
    let total2: u64 = digits[..7].iter().zip(w2.iter()).map(|(d, wt)| d * wt).sum();
    let r2 = total2 % 11 % 10;
    Some(r2)
}

fn verify_ua_legal_body(clean: &str) -> Verdict {
    debug_assert_eq!(clean.len(), 8);
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: [u64; 8] = {
        let v: Vec<u64> = clean.chars().map(|c| c.to_digit(10).unwrap() as u64).collect();
        [v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]]
    };
    let check = edrpou_calc_check(&digits).unwrap();
    if check == digits[7] {
        Verdict::Valid {
            formatted: format!("UA{}", clean),
            detected: "Ukrainian EDRPOU (legal entity, 8 digits)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("UA EDRPOU check mismatch: expected {}, got {}", check, digits[7]),
        }
    }
}

/// Verify a Ukrainian EDRPOU (legal entity, 8 digits).
pub fn verify_ua_legal(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "UA") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 8 {
        return Verdict::Invalid {
            reason: format!("UA EDRPOU: expected 8 digits, got {}", clean.len()),
        };
    }
    verify_ua_legal_body(&clean)
}

pub fn create_ua_legal(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 7 {
        return Err(anyhow!("expected 7 digits (body without check digit), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let v: Vec<u64> = clean.chars().map(|c| c.to_digit(10).unwrap() as u64).collect();
    // pad to [u64;8] with placeholder check=0
    let digits: [u64; 8] = [v[0], v[1], v[2], v[3], v[4], v[5], v[6], 0];
    let check = edrpou_calc_check(&digits).unwrap();
    Ok(format!("UA{}{}", clean, check))
}

// ── RNOKPP ────────────────────────────────────────────────────────────────────

// Weight -1 handled via i64 arithmetic.
const RNOKPP_WEIGHTS: [i64; 9] = [-1, 5, 7, 9, 4, 6, 10, 5, 7];

fn verify_ua_individual_body(clean: &str) -> Verdict {
    debug_assert_eq!(clean.len(), 10);
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: Vec<i64> = clean.chars().map(|c| c.to_digit(10).unwrap() as i64).collect();
    let sum: i64 = digits[..9].iter().zip(RNOKPP_WEIGHTS.iter()).map(|(d, w)| d * w).sum();
    // Euclidean modulo to handle potential negative sum
    let check = ((sum % 11) + 11) % 11 % 10;
    if check == digits[9] {
        Verdict::Valid {
            formatted: format!("UA{}", clean),
            detected: "Ukrainian RNOKPP (individual taxpayer, 10 digits)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("UA RNOKPP check mismatch: expected {}, got {}", check, digits[9]),
        }
    }
}

/// Verify a Ukrainian RNOKPP (individual, 10 digits).
pub fn verify_ua_individual(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "UA") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 10 {
        return Verdict::Invalid {
            reason: format!("UA RNOKPP: expected 10 digits, got {}", clean.len()),
        };
    }
    verify_ua_individual_body(&clean)
}

pub fn create_ua_individual(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!("expected 9 digits (body without check digit), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<i64> = clean.chars().map(|c| c.to_digit(10).unwrap() as i64).collect();
    let sum: i64 = digits.iter().zip(RNOKPP_WEIGHTS.iter()).map(|(d, w)| d * w).sum();
    let check = ((sum % 11) + 11) % 11 % 10;
    Ok(format!("UA{}{}", clean, check))
}

// ── Auto-detect dispatch ──────────────────────────────────────────────────────

/// Auto-detect by length: 8 digits → EDRPOU, 10 digits → RNOKPP.
pub fn verify_ua_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "UA") {
        Ok(body) => body,
        Err(v) => return v,
    };
    match clean.len() {
        8 => verify_ua_legal_body(&clean),
        10 => verify_ua_individual_body(&clean),
        n => Verdict::Invalid {
            reason: format!("UA VAT: expected 8 (EDRPOU) or 10 (RNOKPP) digits, got {}", n),
        },
    }
}

pub fn create_ua_vat(input: &str, raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    match clean.len() {
        7 => create_ua_legal(input, raw),
        9 => create_ua_individual(input, raw),
        n => Err(anyhow!("UA VAT: expected 7 (EDRPOU body) or 9 (RNOKPP body) digits, got {}", n)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── EDRPOU ────────────────────────────────────────────────────────────────

    /// Hand-verified: 32855961 (first digit 3 → weights [7,1,2,3,4,5,6])
    /// products=[21,2,16,15,20,45,36], sum=155, 155%11=1, check=1, digit[7]=1 ✓
    #[test]
    fn ua_edrpou_reference_32855961() {
        match verify_ua_vat("32855961") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "UA32855961");
                assert!(detected.contains("EDRPOU"));
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ua_edrpou_rejects_bad_check() {
        match verify_ua_vat("32855968") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ua_edrpou_accepts_ua_prefix() {
        match verify_ua_vat("UA32855961") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ua_edrpou_round_trip() {
        let body = "3285596";
        let full = create_ua_legal(body, false).unwrap();
        assert_eq!(full, "UA32855961");
        match verify_ua_legal(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    // ── RNOKPP ────────────────────────────────────────────────────────────────

    /// Hand-verified: 1759013776
    /// weights=[-1,5,7,9,4,6,10,5,7], digits=[1,7,5,9,0,1,3,7,7]
    /// sum=270, 270%11=6, 6%10=6, digit[9]=6 ✓
    #[test]
    fn ua_rnokpp_reference_1759013776() {
        match verify_ua_vat("1759013776") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "UA1759013776");
                assert!(detected.contains("RNOKPP"));
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ua_rnokpp_reference_2530414071() {
        match verify_ua_vat("2530414071") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ua_rnokpp_rejects_bad_check() {
        match verify_ua_vat("1759013770") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ua_rnokpp_accepts_ua_prefix() {
        match verify_ua_vat("UA1759013776") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ua_rnokpp_round_trip() {
        let body = "175901377";
        let full = create_ua_individual(body, false).unwrap();
        assert_eq!(full, "UA1759013776");
        match verify_ua_individual(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ua_vat_rejects_bad_length() {
        match verify_ua_vat("12345") {
            Verdict::Invalid { reason } => assert!(reason.contains("8")),
            v => panic!("{:?}", v),
        }
    }
}
