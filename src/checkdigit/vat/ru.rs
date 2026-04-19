//! Russian VAT / INN (Идентификационный номер налогоплательщика).
//!
//! Auto-detects by length:
//! - 10 digits = legal entity (юридическое лицо)
//! - 12 digits = individual (физическое лицо / ИП)
//!
//! # Legal entity (10 digits)
//!
//! Weights `[2, 4, 10, 3, 5, 9, 4, 6, 8]` on first 9 digits.
//! `check = (sum mod 11) mod 10`. Must equal 10th digit.
//!
//! Hand-verified: `7830002293` (Gazprom's publicly-known INN)
//! - 7×2+8×4+3×10+0×3+0×5+0×9+2×4+2×6+9×8 = 14+32+30+0+0+0+8+12+72 = 168
//! - 168 mod 11 = 3 (168 − 15×11 = 3). 3 mod 10 = 3. 10th digit = 3. ✓
//!
//! # Individual (12 digits)
//!
//! Two check digits:
//! - c11 (11th digit): weights `[7, 2, 4, 10, 3, 5, 9, 4, 6, 8]` on first 10.
//!   `c11 = (sum mod 11) mod 10`.
//! - c12 (12th digit): weights `[3, 7, 2, 4, 10, 3, 5, 9, 4, 6, 8]` on first 11 (includes c11).
//!   `c12 = (sum mod 11) mod 10`.
//!
//! Hand-verified: `500100732259` (FNS published test vector)
//! - c11 weights on digits 1-10 (5,0,0,1,0,0,7,3,2,2):
//!   5×7+0×2+0×4+1×10+0×3+0×5+7×9+3×4+2×6+2×8 = 35+0+0+10+0+0+63+12+12+16 = 148
//!   148 mod 11 = 5, 5 mod 10 = 5. 11th digit = 5. ✓
//! - c12 weights on digits 1-11 (5,0,0,1,0,0,7,3,2,2,5):
//!   5×3+0×7+0×2+1×4+0×10+0×3+7×5+3×9+2×4+2×6+5×8 = 15+0+0+4+0+0+35+27+8+12+40 = 141
//!   141 mod 11 = 9, 9 mod 10 = 9. 12th digit = 9. ✓
//!
//! Sources:
//! - Russian Federal Tax Service (ФНС России) INN algorithm documentation
//! - python-stdnum ru.inn
//! - Wikipedia "Individual Taxpayer Number (Russia)"

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const LEGAL_WEIGHTS: [u32; 9] = [2, 4, 10, 3, 5, 9, 4, 6, 8];
const INDIVIDUAL_C11_WEIGHTS: [u32; 10] = [7, 2, 4, 10, 3, 5, 9, 4, 6, 8];
const INDIVIDUAL_C12_WEIGHTS: [u32; 11] = [3, 7, 2, 4, 10, 3, 5, 9, 4, 6, 8];

fn weighted_check(digits: &[u32], weights: &[u32]) -> u32 {
    let sum: u32 = digits.iter().zip(weights.iter()).map(|(d, w)| d * w).sum();
    (sum % 11) % 10
}

// ── Legal entity (10 digits) ──────────────────────────────────────────────────

fn verify_ru_legal_body(clean: &str) -> Verdict {
    debug_assert_eq!(clean.len(), 10);
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let expected = weighted_check(&digits[..9], &LEGAL_WEIGHTS);
    let got = digits[9];
    if expected == got {
        Verdict::Valid {
            formatted: format!("RU{}", clean),
            detected: "Russian INN (legal)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("RU INN (legal) check mismatch: expected {}, got {}", expected, got),
        }
    }
}

/// Verify a Russian INN for a legal entity (10 digits).
pub fn verify_ru_legal(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "RU") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 10 {
        return Verdict::Invalid {
            reason: format!("RU INN (legal): expected 10 digits, got {}", clean.len()),
        };
    }
    verify_ru_legal_body(&clean)
}

/// Create a Russian INN for a legal entity from a 9-digit body.
pub fn create_ru_legal(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!("RU INN (legal): expected 9 digits (body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let check = weighted_check(&digits, &LEGAL_WEIGHTS);
    Ok(format!("RU{}{}", clean, check))
}

// ── Individual (12 digits) ────────────────────────────────────────────────────

fn verify_ru_individual_body(clean: &str) -> Verdict {
    debug_assert_eq!(clean.len(), 12);
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let expected_c11 = weighted_check(&digits[..10], &INDIVIDUAL_C11_WEIGHTS);
    let got_c11 = digits[10];
    if expected_c11 != got_c11 {
        return Verdict::Invalid {
            reason: format!(
                "RU INN (individual) c11 mismatch: expected {}, got {}",
                expected_c11, got_c11
            ),
        };
    }
    let expected_c12 = weighted_check(&digits[..11], &INDIVIDUAL_C12_WEIGHTS);
    let got_c12 = digits[11];
    if expected_c12 == got_c12 {
        Verdict::Valid {
            formatted: format!("RU{}", clean),
            detected: "Russian INN (individual)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!(
                "RU INN (individual) c12 mismatch: expected {}, got {}",
                expected_c12, got_c12
            ),
        }
    }
}

/// Verify a Russian INN for an individual (12 digits).
pub fn verify_ru_individual(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "RU") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 12 {
        return Verdict::Invalid {
            reason: format!("RU INN (individual): expected 12 digits, got {}", clean.len()),
        };
    }
    verify_ru_individual_body(&clean)
}

/// Create a Russian INN for an individual from an 10-digit body.
pub fn create_ru_individual(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 10 {
        return Err(anyhow!(
            "RU INN (individual): expected 10 digits (body without check digits), got {}",
            clean.len()
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let c11 = weighted_check(&digits, &INDIVIDUAL_C11_WEIGHTS);
    // Build 11-digit slice for c12 computation
    let mut digits11 = digits.clone();
    digits11.push(c11);
    let c12 = weighted_check(&digits11, &INDIVIDUAL_C12_WEIGHTS);
    Ok(format!("RU{}{}{}", clean, c11, c12))
}

// ── Auto-detect dispatcher ────────────────────────────────────────────────────

/// Verify a Russian INN, auto-detecting the variant by length.
///
/// - 10 digits → legal entity
/// - 12 digits → individual
pub fn verify_ru_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "RU") {
        Ok(body) => body,
        Err(v) => return v,
    };
    match clean.len() {
        10 => verify_ru_legal_body(&clean),
        12 => verify_ru_individual_body(&clean),
        n => Verdict::Invalid {
            reason: format!(
                "RU INN: expected 10 (legal) or 12 (individual) digits, got {}",
                n
            ),
        },
    }
}

/// Creating an RU VAT requires specifying the variant (ru-legal or ru-individual).
pub fn create_ru_vat(_input: &str, _raw: bool) -> Result<String> {
    Err(anyhow!(
        "ru-vat auto-detect cannot create; use ru-legal or ru-individual to specify the variant"
    ))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Gazprom's publicly-known INN. Hand-verified:
    /// sum=168, 168 mod 11=3, 3 mod 10=3. 10th digit=3. ✓
    #[test]
    fn ru_legal_valid_7830002293() {
        match verify_ru_legal("7830002293") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "RU7830002293");
                assert_eq!(detected, "Russian INN (legal)");
            }
            v => panic!("{:?}", v),
        }
    }

    /// FNS published test vector. Hand-verified both c11=5 and c12=9. ✓
    #[test]
    fn ru_individual_valid_500100732259() {
        match verify_ru_individual("500100732259") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "RU500100732259");
                assert_eq!(detected, "Russian INN (individual)");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ru_vat_autodetects_legal() {
        match verify_ru_vat("7830002293") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Russian INN (legal)"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ru_vat_autodetects_individual() {
        match verify_ru_vat("500100732259") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Russian INN (individual)"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ru_vat_rejects_wrong_length() {
        match verify_ru_vat("12345678901") {
            Verdict::Invalid { reason } => assert!(reason.contains("expected 10 (legal) or 12")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ru_legal_round_trip() {
        let body = "783000229";
        let full = create_ru_legal(body, false).unwrap();
        assert_eq!(full, "RU7830002293");
        match verify_ru_legal(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ru_individual_round_trip() {
        let body = "5001007322";
        let full = create_ru_individual(body, false).unwrap();
        assert_eq!(full, "RU500100732259");
        match verify_ru_individual(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
