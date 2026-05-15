//! Irish VAT.
//!
//! Three historical formats. Auto-detected by length and structure.
//!
//! ## Format 1 — "old" (8 chars: 7 digits + 1 letter)
//!
//! Fully implemented. Weights `[8,7,6,5,4,3,2]` on the 7 digits.
//! `check_idx = sum % 23`. Check letter = `TABLE[check_idx]` where
//! `TABLE = "WABCDEFGHIJKLMNOPQRSTUV"` (W=0, A=1, B=2, ..., V=22).
//!
//! ## Format 2 — "new" (2013+, 9 chars: 7 digits + 1 suffix letter + 1 check letter)
//!
//! The 8th character is a letter A–I (the "suffix", value A=1..I=9).
//! Weighted sum = digits × [8,7,6,5,4,3,2] + suffix_value × 9.
//! `check_idx = sum % 23`. Check letter = TABLE[check_idx] (same table).
//!
//! ## Format 3 — special historical (starts with "0", 8 chars)
//!
//! Not supported. Returns an appropriate error.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const CHECK_TABLE: &[u8] = b"WABCDEFGHIJKLMNOPQRSTUV"; // index 0='W', 1='A', ...

const WEIGHTS_7: [u32; 7] = [8, 7, 6, 5, 4, 3, 2];

/// Compute the Format-1 weighted sum on 7 ASCII digit chars.
fn sum_7digits(body: &str) -> u32 {
    body.chars()
        .enumerate()
        .map(|(i, c)| WEIGHTS_7[i] * c.to_digit(10).unwrap())
        .sum()
}

/// Look up check letter from index 0..23.
fn check_letter(idx: u32) -> char {
    CHECK_TABLE[(idx % 23) as usize] as char
}

/// Value of the suffix letter A=1 .. I=9 (used in Format 2).
/// Returns None if the character is not A–I.
fn suffix_value(c: char) -> Option<u32> {
    let uc = c.to_ascii_uppercase();
    if ('A'..='I').contains(&uc) {
        Some((uc as u32) - ('A' as u32) + 1)
    } else {
        None
    }
}

pub fn verify_ie_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "IE") {
        Ok(body) => body,
        Err(v) => return v,
    };
    let len = clean.len();

    // Format 3: starts with '0' — not supported
    if clean.starts_with('0') && len == 8 {
        return Verdict::Invalid {
            reason: "IE VAT Format 3 (starts with '0', historical) is not supported".into(),
        };
    }

    match len {
        8 => {
            // Format 1: 7 digits + 1 check letter
            let body = &clean[..7];
            let check_char = clean.chars().nth(7).unwrap();
            if !body.chars().all(|c| c.is_ascii_digit()) {
                return Verdict::Invalid {
                    reason: "IE VAT Format 1: first 7 characters must be digits".into(),
                };
            }
            if !check_char.is_ascii_alphabetic() {
                return Verdict::Invalid {
                    reason: "IE VAT Format 1: 8th character must be a letter".into(),
                };
            }
            let sum = sum_7digits(body);
            let expected = check_letter(sum % 23);
            if expected == check_char {
                Verdict::Valid {
                    formatted: format!("IE{}", clean),
                    detected: "Irish VAT (Format 1, old)".into(),
                    comment: String::new(),
                }
            } else {
                Verdict::Invalid {
                    reason: format!(
                        "IE VAT check mismatch: expected '{}', got '{}'",
                        expected, check_char
                    ),
                }
            }
        }
        9 => {
            // Format 2: 7 digits + suffix letter (A–I) + check letter
            let digits_part = &clean[..7];
            let suffix_char = clean.chars().nth(7).unwrap();
            let check_char = clean.chars().nth(8).unwrap();
            if !digits_part.chars().all(|c| c.is_ascii_digit()) {
                return Verdict::Invalid {
                    reason: "IE VAT Format 2: first 7 characters must be digits".into(),
                };
            }
            if !suffix_char.is_ascii_alphabetic() || !check_char.is_ascii_alphabetic() {
                return Verdict::Invalid {
                    reason: "IE VAT Format 2: 8th and 9th characters must be letters".into(),
                };
            }
            let sv = match suffix_value(suffix_char) {
                Some(v) => v,
                None => {
                    return Verdict::Invalid {
                        reason: format!(
                            "IE VAT Format 2: suffix letter must be A–I, got '{}'",
                            suffix_char
                        ),
                    }
                }
            };
            let sum = sum_7digits(digits_part) + sv * 9;
            let expected = check_letter(sum % 23);
            if expected == check_char {
                Verdict::Valid {
                    formatted: format!("IE{}", clean),
                    detected: "Irish VAT (Format 2, 2013+)".into(),
                    comment: String::new(),
                }
            } else {
                Verdict::Invalid {
                    reason: format!(
                        "IE VAT check mismatch: expected '{}', got '{}'",
                        expected, check_char
                    ),
                }
            }
        }
        _ => Verdict::Invalid {
            reason: format!("expected 8 or 9 characters for IE VAT, got {}", len),
        },
    }
}

pub fn create_ie_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, true);
    let len = clean.len();
    match len {
        7 => {
            // Format 1: 7-digit body → append check letter
            if !clean.chars().all(|c| c.is_ascii_digit()) {
                return Err(anyhow!("IE VAT Format 1 create: expected 7 digits"));
            }
            let sum = sum_7digits(&clean);
            let letter = check_letter(sum % 23);
            Ok(format!("IE{}{}", clean, letter))
        }
        8 => {
            // Format 2: 7 digits + suffix letter (A–I) → append check letter
            let digits_part = &clean[..7];
            let suffix_char = clean.chars().nth(7).unwrap();
            if !digits_part.chars().all(|c| c.is_ascii_digit()) {
                return Err(anyhow!("IE VAT Format 2 create: first 7 characters must be digits"));
            }
            let sv = suffix_value(suffix_char).ok_or_else(|| {
                anyhow!(
                    "IE VAT Format 2 create: suffix letter must be A–I, got '{}'",
                    suffix_char
                )
            })?;
            let sum = sum_7digits(digits_part) + sv * 9;
            let letter = check_letter(sum % 23);
            // clean = 7 digits + suffix letter; append the computed check letter
            Ok(format!("IE{}{}", clean, letter))
        }
        _ => Err(anyhow!(
            "expected 7 digits (Format 1) or 7 digits + suffix letter (Format 2), got {} chars",
            len
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ie_vat_format1_valid_1234567t() {
        // sum = 1*8+2*7+3*6+4*5+5*4+6*3+7*2 = 8+14+18+20+20+18+14 = 112
        // 112 % 23 = 20 → TABLE[20] = 'T'
        match verify_ie_vat("1234567T") {
            Verdict::Valid { detected, .. } => {
                assert!(detected.contains("Format 1"), "detected: {}", detected);
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ie_vat_format1_round_trip() {
        let body = "1234567";
        let full = create_ie_vat(body, false).unwrap();
        let raw = &full[2..]; // strip "IE"
        match verify_ie_vat(raw) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ie_vat_format1_rejects_bad_check() {
        match verify_ie_vat("1234567A") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ie_vat_format2_round_trip() {
        // Build a format-2 number: 7 digits + suffix 'A' (value 1)
        // sum_7 for 1234567 = 112, + 1*9 = 121, 121%23 = 121 - 5*23 = 121-115 = 6 → TABLE[6]='F'
        // So "1234567AF" should be valid Format 2
        let created = create_ie_vat("1234567A", false).unwrap();
        let raw = &created[2..];
        match verify_ie_vat(raw) {
            Verdict::Valid { detected, .. } => {
                assert!(detected.contains("Format 2"), "detected: {}", detected);
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ie_vat_rejects_wrong_length() {
        match verify_ie_vat("123456") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
