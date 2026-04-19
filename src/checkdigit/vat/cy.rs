//! Cypriot VAT.
//!
//! 9 characters: 8 digits followed by 1 uppercase letter.
//!
//! Algorithm (1-indexed positions):
//! - Odd positions (1, 3, 5, 7): remap the digit via lookup table.
//! - Even positions (2, 4, 6, 8): use the digit value as-is.
//! - Sum all values, then check_index = sum % 26.
//! - Check letter = 'A' + check_index.
//!
//! Odd-position remap: 0→1, 1→0, 2→5, 3→7, 4→9, 5→13, 6→15, 7→17, 8→19, 9→21.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const ODD_REMAP: [u32; 10] = [1, 0, 5, 7, 9, 13, 15, 17, 19, 21];

fn compute_check_letter(body: &str) -> char {
    // body must be exactly 8 ASCII digits
    let sum: u32 = body.chars().enumerate().map(|(i, c)| {
        let d = c.to_digit(10).unwrap();
        // 0-indexed i=0 → 1-indexed position 1 (odd), i=1 → position 2 (even), etc.
        if i % 2 == 0 {
            ODD_REMAP[d as usize]  // odd 1-indexed positions (0-indexed 0, 2, 4, 6)
        } else {
            d                       // even 1-indexed positions (0-indexed 1, 3, 5, 7)
        }
    }).sum();
    let idx = sum % 26;
    (b'A' + idx as u8) as char
}

pub fn verify_cy_vat(input: &str) -> Verdict {
    let clean = sanitize(input, true);
    if clean.len() != 9 {
        return Verdict::Invalid { reason: format!("expected 8 digits + 1 letter (9 chars), got {}", clean.len()) };
    }
    let body = &clean[..8];
    let check_char = clean.chars().nth(8).unwrap();
    if !body.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "first 8 characters must be digits".into() };
    }
    if !check_char.is_ascii_uppercase() {
        return Verdict::Invalid { reason: "9th character must be an uppercase letter".into() };
    }
    let expected = compute_check_letter(body);
    if expected == check_char {
        Verdict::Valid {
            formatted: format!("CY{}", clean),
            detected: "Cyprus VAT".into(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("CY VAT check mismatch: expected '{}', got '{}'", expected, check_char),
        }
    }
}

pub fn create_cy_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits (body without check letter), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit body"));
    }
    let letter = compute_check_letter(&clean);
    Ok(format!("CY{}{}", clean, letter))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cy_vat_valid_00532445o() {
        // 00532445O — sum = 1+0+13+3+5+4+9+5 = 40, 40%26=14 → 'O'
        match verify_cy_vat("00532445O") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cy_vat_round_trip() {
        let body = "00532445";
        let full = create_cy_vat(body, false).unwrap();
        // strip the "CY" prefix
        let raw = &full[2..];
        match verify_cy_vat(raw) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cy_vat_rejects_wrong_letter() {
        // 00532445A — 'A' should be wrong (expected 'O')
        match verify_cy_vat("00532445A") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cy_vat_rejects_wrong_length() {
        match verify_cy_vat("0053244O") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
