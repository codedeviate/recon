//! United Kingdom VAT.
//!
//! 9 or 12 digits. Two algorithms are both still in active use:
//!
//! **Algorithm A (classic mod-97):** weights `[8,7,6,5,4,3,2,10,1]` on all 9 digits.
//! Valid when `sum mod 97 == 0`.
//!
//! **Algorithm B (97-55):** same weights. Valid when `(sum + 55) mod 97 == 0`,
//! equivalently `sum mod 97 == 42`.
//!
//! A number may pass exactly one algorithm. If either passes, the number is valid.
//!
//! For 12-digit form the first 9 digits carry the check and the final 3 are a branch
//! identifier with no additional check digit.
//!
//! Sources: HMRC VAT validation algorithm documentation, python-stdnum gb.vat,
//! Wikipedia "VAT identification number".
//!
//! Hand-verified vectors:
//!   333289454 — algo A: sum=194, 194%97=0 ✓
//!   123456727 — algo B: sum=139, (139+55)%97=0 ✓

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 9] = [8, 7, 6, 5, 4, 3, 2, 10, 1];

fn weighted_sum(digits: &[u32]) -> u32 {
    digits.iter().zip(WEIGHTS.iter()).map(|(d, w)| d * w).sum()
}

pub fn verify_uk_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "UK") {
        Ok(body) => body,
        Err(v) => return v,
    };
    // Accept 9 or 12 digits; for 12-digit, validate only the first 9.
    if clean.len() != 9 && clean.len() != 12 {
        return Verdict::Invalid {
            reason: format!("expected 9 or 12 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid {
            reason: "non-digit input".into(),
        };
    }
    let digits: Vec<u32> = clean[..9].chars().map(|c| c.to_digit(10).unwrap()).collect();
    let sum = weighted_sum(&digits);
    let suffix = if clean.len() == 12 {
        format!(" (branch {})", &clean[9..])
    } else {
        String::new()
    };
    if sum % 97 == 0 {
        return Verdict::Valid {
            formatted: format!("UK{}", clean),
            detected: "UK VAT (classic mod-97)".into(),
            comment: suffix,
        };
    }
    if (sum + 55) % 97 == 0 {
        return Verdict::Valid {
            formatted: format!("UK{}", clean),
            detected: "UK VAT (97-55)".into(),
            comment: suffix,
        };
    }
    Verdict::Invalid {
        reason: format!(
            "UK VAT check failed: sum={}, sum%97={} (algo A wants 0), (sum+55)%97={} (algo B wants 0)",
            sum,
            sum % 97,
            (sum + 55) % 97
        ),
    }
}

/// Compute the 9th check digit using algorithm A (classic mod-97), given an
/// 8-digit body. The check digit is chosen so that the full 9-digit sum mod 97 == 0.
pub fn create_uk_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits (body without check digit), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    // weights for positions 0-7 are [8,7,6,5,4,3,2,10]; weight for check digit (pos 8) is 1.
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let partial: u32 = digits.iter().zip(WEIGHTS[..8].iter()).map(|(d, w)| d * w).sum();
    // Find check_digit in 0-9 such that (partial + check_digit * 1) mod 97 == 0.
    for cd in 0u32..=9 {
        if (partial + cd) % 97 == 0 {
            return Ok(format!("UK{}{}", clean, cd));
        }
    }
    Err(anyhow!("no single check digit 0-9 satisfies algo A for this body (try a different 8-digit body)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Algorithm A (classic mod-97).
    /// Hand-verified: sum=194, 194%97=0 ✓
    #[test]
    fn uk_vat_algo_a_333289454() {
        match verify_uk_vat("333289454") {
            Verdict::Valid { detected, .. } => {
                assert!(detected.contains("classic"), "got detected={:?}", detected);
            }
            v => panic!("{:?}", v),
        }
    }

    /// Algorithm B (97-55).
    /// Hand-verified: sum=139, (139+55)%97=0 ✓
    #[test]
    fn uk_vat_algo_b_123456727() {
        match verify_uk_vat("123456727") {
            Verdict::Valid { detected, .. } => {
                assert!(detected.contains("97-55"), "got detected={:?}", detected);
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn uk_vat_accepts_uk_prefix() {
        match verify_uk_vat("UK333289454") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn uk_vat_rejects_wrong_length() {
        match verify_uk_vat("33328945") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn uk_vat_rejects_bad_check() {
        match verify_uk_vat("333289453") {
            Verdict::Invalid { reason } => assert!(reason.contains("check failed")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn uk_vat_12_digit_branch() {
        // 12-digit form: first 9 valid under algo A, last 3 are branch identifier
        match verify_uk_vat("333289454001") {
            Verdict::Valid { comment, formatted, .. } => {
                assert!(comment.contains("001"), "comment={:?}", comment);
                assert_eq!(formatted, "UK333289454001");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn uk_vat_round_trip() {
        // 33328945 is the 8-digit body of 333289454 (algo A)
        let full = create_uk_vat("33328945", false).unwrap();
        assert_eq!(full, "UK333289454");
        match verify_uk_vat(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
