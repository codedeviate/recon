//! Belarusian UNP (Учетный номер плательщика / Уліковы нумар платніка).
//!
//! 9 characters. The first two can be purely numeric OR from the restricted set
//! `{A,B,C,E,H,K,M,O,P,T}`; positions 3–9 are always digits.
//!
//! # Algorithm
//!
//! Weights `(29, 23, 19, 17, 13, 7, 5, 3)` applied to the first 8 characters.
//! Each character is looked up in the base-36 alphabet
//! `"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"`.
//!
//! For alphanumeric UNPs the second character (from `"ABCEHKMOPT"`) is
//! substituted with its index in that 10-letter set before lookup.
//!
//! ```text
//! alphabet  = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"
//! if alphanumeric: number[1] → str(index_in("ABCEHKMOPT"))
//! check = Σ(weights[i] × alphabet.index(number[i])) mod 11
//! ```
//!
//! `check` must be ≤ 9 (values of 10 indicate a structurally invalid number
//! — such numbers cannot exist with a valid check digit).
//!
//! **Test vectors:**
//! - `200988541` (all-numeric; from python-stdnum by.unp doctest).
//! - `MA1953684` (alphanumeric; from python-stdnum by.unp doctest).
//!
//! Hand-verified `200988541` (body `20098854`, check `1`):
//! - Indices in alphabet: 2,0,0,9,8,8,5,4
//! - Products (×29,23,19,17,13,7,5,3): 58,0,0,153,104,56,25,12
//! - Sum = 408; 408 mod 11 = 1. 9th char = 1. ✓
//!
//! Hand-verified `MA1953684` (alphanumeric, body `MA195368`, check `4`):
//! - M stays as M (alphabet index 22), A → index in "ABCEHKMOPT" = 0 → char '0'
//! - Converted body: `M0195368`
//! - Indices: 22,0,1,9,5,3,6,8
//! - Products: 638,0,19,153,65,21,30,24
//! - Sum = 950; 950 mod 11 = 4. 9th char = 4. ✓
//!
//! Sources:
//! - python-stdnum by.unp (doctest vectors 200988541, MA1953684)
//! - Ministerstvo po nalogam i sboram Respubliki Belarus (MNS BY)

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 8] = [29, 23, 19, 17, 13, 7, 5, 3];
const ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const ALPHA_SECOND: &str = "ABCEHKMOPT";
const VALID_FIRST: &str = "1234567ABCEHKM";

/// Return `Some(index)` for `c` in the base-36 ALPHABET, else `None`.
fn alpha_index(c: char) -> Option<u32> {
    ALPHABET.chars().position(|x| x == c).map(|i| i as u32)
}

/// Compute the check digit for an 8-character (possibly alphanumeric) body.
/// Returns `None` if the computed value is 10 (structurally unrepresentable).
fn compute_check(body: &str) -> Option<u32> {
    debug_assert_eq!(body.len(), 8);
    // For alphanumeric UNPs, substitute the second character.
    let converted: String = if !body.chars().all(|c| c.is_ascii_digit()) {
        let first = body.chars().next().unwrap();
        let second = body.chars().nth(1).unwrap();
        let second_sub = if let Some(idx) = ALPHA_SECOND.chars().position(|x| x == second) {
            std::char::from_digit(idx as u32, 10).unwrap()
        } else {
            second
        };
        let rest: String = body.chars().skip(2).collect();
        format!("{}{}{}", first, second_sub, rest)
    } else {
        body.to_string()
    };

    let sum: u32 = converted
        .chars()
        .zip(WEIGHTS.iter())
        .map(|(c, &w)| w * alpha_index(c).unwrap_or(0))
        .sum();
    let r = sum % 11;
    if r > 9 { None } else { Some(r) }
}

fn is_valid_format(s: &str) -> bool {
    if s.len() != 9 {
        return false;
    }
    let chars: Vec<char> = s.chars().collect();
    // Positions 2..8 must be digits
    if !chars[2..].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }
    // First two: either both digits OR both from ALPHA_SECOND
    let first_two_alpha = chars[0].is_ascii_alphabetic() || chars[1].is_ascii_alphabetic();
    if first_two_alpha {
        if !chars[0].is_ascii_alphabetic() || !chars[1].is_ascii_alphabetic() {
            return false; // mixed digit/letter in first two
        }
        if !ALPHA_SECOND.contains(chars[0]) || !ALPHA_SECOND.contains(chars[1]) {
            return false;
        }
    }
    // First character must be in {1..7, A,B,C,E,H,K,M}
    if !VALID_FIRST.contains(chars[0]) {
        return false;
    }
    true
}

/// Verify a Belarusian UNP (9 characters, weighted mod-11).
pub fn verify_by_vat(input: &str) -> Verdict {
    // Sanitize: strip whitespace/dashes but preserve case (then uppercase)
    let raw = sanitize(input, true);
    // Strip optional "BY" prefix
    let clean = match super::strip_vat_prefix(&raw, "BY") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 9 {
        return Verdict::Invalid {
            reason: format!("BY UNP: expected 9 characters, got {}", clean.len()),
        };
    }
    if !is_valid_format(&clean) {
        return Verdict::Invalid {
            reason: "BY UNP: invalid format (positions 3-9 must be digits; first two must be \
                     numeric or from A,B,C,E,H,K,M,O,P,T)".into(),
        };
    }
    let body = &clean[..8];
    let stored: u32 = clean.chars().nth(8).unwrap().to_digit(10).unwrap();
    match compute_check(body) {
        None => Verdict::Invalid {
            reason: "BY UNP: check digit computation yields 10 (structurally invalid number)".into(),
        },
        Some(expected) if expected == stored => Verdict::Valid {
            formatted: format!("BY{}", clean),
            detected: "Belarusian UNP".into(),
            comment: String::new(),
        },
        Some(expected) => Verdict::Invalid {
            reason: format!("BY UNP check mismatch: expected {}, got {}", expected, stored),
        },
    }
}

/// Create a Belarusian UNP from an 8-character body.
pub fn create_by_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, true);
    if clean.len() != 8 {
        return Err(anyhow!(
            "BY UNP: expected 8 characters (body without check digit), got {}",
            clean.len()
        ));
    }
    match compute_check(&clean) {
        None => Err(anyhow!("BY UNP: check digit computation yields 10 (structurally invalid body)")),
        Some(check) => Ok(format!("BY{}{}", clean, check)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// python-stdnum by.unp doctest vector (all-numeric). Hand-verified: check=1. ✓
    #[test]
    fn by_vat_valid_200988541() {
        match verify_by_vat("200988541") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "BY200988541");
                assert_eq!(detected, "Belarusian UNP");
            }
            v => panic!("{:?}", v),
        }
    }

    /// python-stdnum by.unp doctest vector (alphanumeric). Hand-verified: check=4. ✓
    #[test]
    fn by_vat_valid_ma1953684() {
        match verify_by_vat("MA1953684") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "BYMA1953684");
                assert_eq!(detected, "Belarusian UNP");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn by_vat_rejects_wrong_length() {
        match verify_by_vat("20098854") {
            Verdict::Invalid { reason } => assert!(reason.contains("expected 9 characters")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn by_vat_round_trip_numeric() {
        let body = "20098854";
        let full = create_by_vat(body, false).unwrap();
        assert_eq!(full, "BY200988541");
        match verify_by_vat(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn by_vat_round_trip_alpha() {
        let body = "MA195368";
        let full = create_by_vat(body, false).unwrap();
        assert_eq!(full, "BYMA1953684");
        match verify_by_vat(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn by_vat_rejects_bad_check() {
        // 200988542 — wrong check digit (should be 1)
        match verify_by_vat("200988542") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
