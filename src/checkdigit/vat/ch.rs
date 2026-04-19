//! Swiss UID / IDE (Unternehmens-Identifikationsnummer).
//!
//! Format: `CHE-XXX.XXX.XXX` optionally suffixed by ` MWST`, ` TVA`, or ` IVA`.
//! After stripping prefix, dots, whitespace, and optional suffix → 9 digits.
//!
//! Algorithm: weights `[5,4,3,2,7,6,5,4]` on the first 8 digits.
//! `check_raw = 11 - (sum mod 11)`. If `check_raw == 11`, check digit = 0.
//! If `check_raw == 10`, the number is structurally invalid.
//!
//! The prefix is `CHE` (3 chars), so the shared 2-char `strip_vat_prefix` helper
//! is NOT used — prefix handling is local to this file.
//!
//! Sources: Swiss Federal Statistical Office (BFS) UID documentation (uid.admin.ch),
//! python-stdnum ch.uid, Wikipedia "Unique Enterprise Identification Number".
//!
//! Hand-verified vectors:
//!   CHE-100.155.212 → sum=86, 86%11=9, check_raw=2, last digit=2 ✓
//!   CHE-116.281.710 → sum=132, 132%11=0, check_raw=11→0, last digit=0 ✓ (check_raw=11 case)
//!   CHE-107.787.577 → sum=191, 191%11=4, check_raw=7, last digit=7 ✓

use super::super::Verdict;
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 8] = [5, 4, 3, 2, 7, 6, 5, 4];

/// Strip CHE prefix, dots, whitespace, dashes and optional MWST/TVA/IVA suffix.
fn parse_ch_uid(input: &str) -> String {
    let clean_upper = input
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '.' && *c != '-')
        .collect::<String>()
        .to_ascii_uppercase();
    let after_prefix = clean_upper.strip_prefix("CHE").unwrap_or(&clean_upper);
    let without_suffix = after_prefix
        .strip_suffix("MWST")
        .or_else(|| after_prefix.strip_suffix("IVA"))
        .or_else(|| after_prefix.strip_suffix("TVA"))
        .unwrap_or(after_prefix);
    without_suffix.to_string()
}

fn compute_check(digits8: &[u32]) -> Option<u32> {
    let sum: u32 = digits8.iter().zip(WEIGHTS.iter()).map(|(d, w)| d * w).sum();
    let check_raw = 11 - (sum % 11);
    if check_raw == 10 {
        None // structurally invalid
    } else if check_raw == 11 {
        Some(0)
    } else {
        Some(check_raw)
    }
}

pub fn verify_ch_vat(input: &str) -> Verdict {
    let clean = parse_ch_uid(input);
    if clean.len() != 9 {
        return Verdict::Invalid {
            reason: format!("expected 9 digits after stripping CHE prefix and suffix, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid {
            reason: "non-digit input".into(),
        };
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    match compute_check(&digits[..8]) {
        None => Verdict::Invalid {
            reason: "CH UID check_raw == 10: structurally invalid number".into(),
        },
        Some(check) => {
            if check == digits[8] {
                Verdict::Valid {
                    formatted: format!("CHE-{}.{}.{}", &clean[..3], &clean[3..6], &clean[6..]),
                    detected: "Swiss UID / IDE".into(),
                    comment: String::new(),
                }
            } else {
                Verdict::Invalid {
                    reason: format!("CH UID check mismatch: expected {}, got {}", check, digits[8]),
                }
            }
        }
    }
}

pub fn create_ch_vat(input: &str, _raw: bool) -> Result<String> {
    // Accept 8 raw digits (body without check digit).
    // Also accept with CHE prefix and dots for convenience.
    let clean = parse_ch_uid(input);
    // After parsing, if 9 digits were given, treat last as check digit for round-trip;
    // but create expects the 8-digit body.
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits (UID body without check digit), got {} (parsed: {:?})", clean.len(), clean));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    match compute_check(&digits) {
        None => Err(anyhow!("this 8-digit body produces check_raw=10, which is structurally invalid")),
        Some(check) => Ok(format!("CHE-{}.{}.{}", &clean[..3], &clean[3..6], format!("{}{}", &clean[6..], check))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CHE-100.155.212 — sum=86, 86%11=9, check_raw=2, last=2 ✓
    #[test]
    fn ch_vat_reference_100155212() {
        match verify_ch_vat("100155212") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "CHE-100.155.212");
                assert!(detected.contains("Swiss"));
            }
            v => panic!("{:?}", v),
        }
    }

    /// CHE-116.281.710 — sum=132, check_raw=11→0, last=0 ✓ (tests the check_raw==11 branch)
    #[test]
    fn ch_vat_reference_116281710_check_raw_11() {
        match verify_ch_vat("116281710") {
            Verdict::Valid { formatted, .. } => {
                assert_eq!(formatted, "CHE-116.281.710");
            }
            v => panic!("{:?}", v),
        }
    }

    /// CHE-107.787.577 — sum=191, check_raw=7, last=7 ✓
    #[test]
    fn ch_vat_reference_107787577() {
        match verify_ch_vat("107787577") {
            Verdict::Valid { formatted, .. } => {
                assert_eq!(formatted, "CHE-107.787.577");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ch_vat_accepts_che_prefix_and_dots() {
        match verify_ch_vat("CHE-100.155.212") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ch_vat_accepts_mwst_suffix() {
        match verify_ch_vat("CHE-100.155.212 MWST") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ch_vat_accepts_tva_suffix() {
        match verify_ch_vat("CHE-100.155.212 TVA") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ch_vat_accepts_iva_suffix() {
        match verify_ch_vat("CHE-100.155.212 IVA") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ch_vat_rejects_bad_check() {
        match verify_ch_vat("100155213") {
            Verdict::Invalid { reason } => assert!(reason.contains("check mismatch")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ch_vat_rejects_bad_length() {
        match verify_ch_vat("10015521") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ch_vat_round_trip() {
        // 8-digit body of CHE-100.155.212
        let full = create_ch_vat("10015521", false).unwrap();
        assert_eq!(full, "CHE-100.155.212");
        match verify_ch_vat(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
