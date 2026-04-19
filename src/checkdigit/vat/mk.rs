//! North Macedonian EDB (Едниствен Даночен Број / Единствен Даночен Број).
//!
//! 13 digits. Optional prefix `"MK"` (ASCII) or `"МК"` (Cyrillic — handled by
//! sanitize before this layer).
//!
//! # Algorithm
//!
//! Weights `(7, 6, 5, 4, 3, 2, 7, 6, 5, 4, 3, 2)` applied to the first 12
//! digits. `check = (-sum mod 11) mod 10`. Must equal the 13th digit.
//!
//! ```text
//! weights = (7, 6, 5, 4, 3, 2, 7, 6, 5, 4, 3, 2)
//! total   = Σ wᵢ × dᵢ  for i in 0..12
//! check   = (-(total mod 11) + 11) mod 11 mod 10
//!         ≡ (-total mod 11) mod 10   (Python semantics)
//! ```
//!
//! **Test vectors (from python-stdnum mk.edb doctests):**
//! - `4030000375897` — check = 7 ✓
//! - `4020990116747` — check = 7 ✓
//! - `4057009501106` — check = 6 ✓
//! - `4030000375890` — invalid (computed check = 7, stored = 0) ✓
//!
//! Hand-verified `4030000375897` (body `403000037589`, check `7`):
//! - Products (×7,6,5,4,3,2,7,6,5,4,3,2): 28,0,15,0,0,0,0,18,35,20,24,18
//! - Sum = 158; (-158 mod 11) mod 10 = ((-158%11)+11)%11%10
//!   158 mod 11 = 4; (-4 mod 11) in Python = 7; 7 mod 10 = 7. ✓
//!
//! Sources:
//! - python-stdnum mk.edb (doctest vectors above)
//! - Uprva za Javni Prihodi (UJP) — North Macedonian Public Revenue Office

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 12] = [7, 6, 5, 4, 3, 2, 7, 6, 5, 4, 3, 2];

fn compute_check(body: &str) -> u32 {
    // body must be exactly 12 ASCII digits
    let total: u32 = body
        .chars()
        .zip(WEIGHTS.iter())
        .map(|(c, &w)| c.to_digit(10).unwrap() * w)
        .sum();
    // Python's -total % 11 equals (11 - (total % 11)) % 11 in unsigned arithmetic.
    let r = total % 11;
    ((11 - r) % 11) % 10
}

/// Verify a North Macedonian EDB (13 digits, weighted mod-11/mod-10).
pub fn verify_mk_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "MK") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 13 {
        return Verdict::Invalid {
            reason: format!("MK EDB: expected 13 digits, got {}", clean.len()),
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
            formatted: format!("MK{}", clean),
            detected: "North Macedonian EDB".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("MK EDB check mismatch: expected {}, got {}", expected, check),
        }
    }
}

/// Create a North Macedonian EDB from a 12-digit body.
pub fn create_mk_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 12 {
        return Err(anyhow!(
            "MK EDB: expected 12 digits (body without check digit), got {}",
            clean.len()
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let check = compute_check(&clean);
    Ok(format!("MK{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// python-stdnum mk.edb doctest vector. Hand-verified: check=7. ✓
    #[test]
    fn mk_vat_valid_4030000375897() {
        match verify_mk_vat("4030000375897") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "MK4030000375897");
                assert_eq!(detected, "North Macedonian EDB");
            }
            v => panic!("{:?}", v),
        }
    }

    /// python-stdnum mk.edb doctest vector. Hand-verified: check=7. ✓
    #[test]
    fn mk_vat_valid_4020990116747() {
        match verify_mk_vat("4020990116747") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "MK4020990116747");
                assert_eq!(detected, "North Macedonian EDB");
            }
            v => panic!("{:?}", v),
        }
    }

    /// python-stdnum mk.edb doctest vector. Hand-verified: check=6. ✓
    #[test]
    fn mk_vat_valid_4057009501106() {
        match verify_mk_vat("4057009501106") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "MK4057009501106");
                assert_eq!(detected, "North Macedonian EDB");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn mk_vat_rejects_wrong_length() {
        match verify_mk_vat("403000037589") {
            Verdict::Invalid { reason } => assert!(reason.contains("expected 13 digits")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn mk_vat_round_trip() {
        let body = "403000037589";
        let full = create_mk_vat(body, false).unwrap();
        assert_eq!(full, "MK4030000375897");
        match verify_mk_vat(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn mk_vat_rejects_bad_check() {
        // 4030000375890 — wrong check (should be 7)
        match verify_mk_vat("4030000375890") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
