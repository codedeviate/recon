//! Montenegrin PIB (Poreski Identifikacioni Broj).
//!
//! 8 digits. The first 7 are the serial body; the 8th is a weighted mod-11
//! check digit.
//!
//! # Algorithm
//!
//! Weights `(8, 7, 6, 5, 4, 3, 2)` applied to digits 1–7.
//! `check = ((11 - (sum mod 11)) mod 11) mod 10`
//!
//! ```text
//! weights = (8, 7, 6, 5, 4, 3, 2)
//! total   = Σ wᵢ × dᵢ  for i in 0..7
//! check   = ((11 - total % 11) % 11) % 10
//! ```
//!
//! **Test vectors (from stdnum-js me/pib.spec.ts):**
//! - `02000989` — check = 9 ✓
//! - `02005115` — check = 5 ✓
//! - `02005328` — check = 8 ✓
//! - `03350487` — check = 7 ✓
//! - `03357481` — check = 1 ✓
//! - `03353487` — invalid (computed check = 6, stored = 7) ✓
//!
//! Hand-verified `02000989` (body `0200098`, check `9`):
//! - Products (×8,7,6,5,4,3,2): 0,14,0,0,0,27,16
//! - Sum = 57; 57 mod 11 = 2; (11-2) mod 11 = 9; 9 mod 10 = 9. ✓
//!
//! Sources:
//! - stdnum-js koblas/stdnum-js src/me/pib.ts (algorithm + test vectors)
//! - Verified against all 5 valid test vectors from me/pib.spec.ts

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 7] = [8, 7, 6, 5, 4, 3, 2];

fn compute_check(body: &str) -> u32 {
    // body must be exactly 7 ASCII digits
    let total: u32 = body
        .chars()
        .zip(WEIGHTS.iter())
        .map(|(c, &w)| c.to_digit(10).unwrap() * w)
        .sum();
    ((11 - total % 11) % 11) % 10
}

/// Verify a Montenegrin PIB (8 digits, weighted mod-11 check).
pub fn verify_me_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "ME") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 8 {
        return Verdict::Invalid {
            reason: format!("ME PIB: expected 8 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let body = &clean[..7];
    let check: u32 = clean.chars().nth(7).unwrap().to_digit(10).unwrap();
    let expected = compute_check(body);
    if expected == check {
        Verdict::Valid {
            formatted: format!("ME{}", clean),
            detected: "Montenegrin PIB".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("ME PIB check mismatch: expected {}, got {}", expected, check),
        }
    }
}

/// Create a Montenegrin PIB from a 7-digit body.
pub fn create_me_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 7 {
        return Err(anyhow!(
            "ME PIB: expected 7 digits (body without check digit), got {}",
            clean.len()
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let check = compute_check(&clean);
    Ok(format!("ME{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// stdnum-js me/pib.spec.ts vector. Hand-verified: check=9. ✓
    #[test]
    fn me_vat_valid_02000989() {
        match verify_me_vat("02000989") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "ME02000989");
                assert_eq!(detected, "Montenegrin PIB");
            }
            v => panic!("{:?}", v),
        }
    }

    /// stdnum-js me/pib.spec.ts vector. Hand-verified: check=5. ✓
    #[test]
    fn me_vat_valid_02005115() {
        match verify_me_vat("02005115") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "ME02005115");
                assert_eq!(detected, "Montenegrin PIB");
            }
            v => panic!("{:?}", v),
        }
    }

    /// stdnum-js me/pib.spec.ts vector. Hand-verified: check=8. ✓
    #[test]
    fn me_vat_valid_02005328() {
        match verify_me_vat("02005328") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "ME02005328");
                assert_eq!(detected, "Montenegrin PIB");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn me_vat_rejects_wrong_length() {
        match verify_me_vat("0335348") {
            Verdict::Invalid { reason } => assert!(reason.contains("expected 8 digits")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn me_vat_rejects_bad_check() {
        // 03353487 — computed check=6, stored=7 (from stdnum-js spec)
        match verify_me_vat("03353487") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn me_vat_round_trip() {
        let body = "0200098";
        let full = create_me_vat(body, false).unwrap();
        assert_eq!(full, "ME02000989");
        match verify_me_vat(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn me_vat_accepts_me_prefix() {
        match verify_me_vat("ME02000989") {
            Verdict::Valid { formatted, .. } => assert_eq!(formatted, "ME02000989"),
            v => panic!("{:?}", v),
        }
    }
}
