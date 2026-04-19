//! Spanish VAT — three variants auto-detected by first character.
//!
//! ## NIF (citizen / tax resident)
//!
//! Format: 8 digits + 1 letter (total 9 chars).
//! `letter_idx = digits_as_number mod 23`.
//! Check letter = `NIF_LETTERS[letter_idx]`.
//!
//! ## NIE (foreigner)
//!
//! Format: X|Y|Z + 7 digits + 1 letter (total 9 chars).
//! Substitute prefix: X→0, Y→1, Z→2. Treat resulting 8-char string
//! as a NIF and apply mod-23 letter lookup.
//!
//! ## CIF (legal entity)
//!
//! Format: letter + 7 digits + 1 check char (total 9 chars).
//! Odd-indexed positions (1-indexed: 1,3,5,7 → 0-indexed: 0,2,4,6) doubled
//! with digit-sum if > 9; even-indexed positions (1-indexed: 2,4,6 → 0-indexed:
//! 1,3,5) summed directly. `control = (10 - total % 10) % 10`.
//! Entity type from first letter determines whether check is a letter
//! (from `CIF_LETTERS`), a digit, or either.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

/// Mod-23 letter table for NIF / NIE.
const NIF_LETTERS: &[u8; 23] = b"TRWAGMYFPDXBNJZSQVHLCKE";

/// Control-digit-to-letter table for CIF entities that require a letter check.
/// control 0→'J', 1→'A', …, 9→'I'.
const CIF_LETTERS: &[u8; 10] = b"JABCDEFGHI";

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Compute the NIF check letter for an 8-digit number.
fn nif_check_letter(digits_u32: u32) -> char {
    NIF_LETTERS[(digits_u32 % 23) as usize] as char
}

/// Classify the CIF entity letter. Returns `(requires_letter, requires_digit)`.
/// - digit-only entities: A B E H
/// - letter-only entities: K P Q S N W R
/// - either entities: C D F G J U V
fn cif_check_type(entity: char) -> (bool, bool) {
    match entity {
        'A' | 'B' | 'E' | 'H' => (false, true),
        'K' | 'P' | 'Q' | 'S' | 'N' | 'W' | 'R' => (true, false),
        _ => (true, true), // C D F G J U V — either
    }
}

/// Compute the CIF control value (0–9) for a 7-digit body string.
/// Positions are 0-indexed within the body array.
/// 0-indexed even positions (0,2,4,6) are doubled (digit-sum if > 9).
/// 0-indexed odd positions (1,3,5) are summed directly.
fn cif_control(body: &str) -> u32 {
    let digits: Vec<u32> = body
        .chars()
        .map(|c| c.to_digit(10).unwrap())
        .collect();

    let sum_doubled: u32 = [0usize, 2, 4, 6]
        .iter()
        .map(|&i| {
            let d2 = digits[i] * 2;
            if d2 > 9 { d2 / 10 + d2 % 10 } else { d2 }
        })
        .sum();

    let sum_even: u32 = [1usize, 3, 5].iter().map(|&i| digits[i]).sum();

    let total = sum_doubled + sum_even;
    (10 - total % 10) % 10
}

// ---------------------------------------------------------------------------
// NIF
// ---------------------------------------------------------------------------

pub fn verify_es_nif(input: &str) -> Verdict {
    let clean = sanitize(input, true);
    if clean.len() != 9 {
        return Verdict::Invalid {
            reason: format!("ES NIF: expected 9 characters, got {}", clean.len()),
        };
    }
    let body = &clean[..8];
    let check_char = clean.chars().nth(8).unwrap();

    if !body.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid {
            reason: "ES NIF: first 8 characters must be digits".into(),
        };
    }
    if !check_char.is_ascii_alphabetic() {
        return Verdict::Invalid {
            reason: "ES NIF: 9th character must be a letter".into(),
        };
    }

    let n: u32 = body.parse().unwrap();
    let expected = nif_check_letter(n);
    if expected == check_char {
        Verdict::Valid {
            formatted: format!("ES{}", clean),
            detected: "Spanish VAT (NIF)".into(),
        }
    } else {
        Verdict::Invalid {
            reason: format!(
                "ES NIF check mismatch: expected '{}', got '{}'",
                expected, check_char
            ),
        }
    }
}

pub fn create_es_nif(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, true);
    if clean.len() != 8 {
        return Err(anyhow!(
            "ES NIF create: expected 8 digits (body without check letter), got {}",
            clean.len()
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("ES NIF create: body must be 8 digits"));
    }
    let n: u32 = clean.parse().unwrap();
    let letter = nif_check_letter(n);
    Ok(format!("ES{}{}", clean, letter))
}

// ---------------------------------------------------------------------------
// NIE
// ---------------------------------------------------------------------------

pub fn verify_es_nie(input: &str) -> Verdict {
    let clean = sanitize(input, true);
    if clean.len() != 9 {
        return Verdict::Invalid {
            reason: format!("ES NIE: expected 9 characters, got {}", clean.len()),
        };
    }

    let prefix = clean.chars().next().unwrap();
    let sub = match prefix {
        'X' => '0',
        'Y' => '1',
        'Z' => '2',
        _ => {
            return Verdict::Invalid {
                reason: format!(
                    "ES NIE: first character must be X, Y, or Z, got '{}'",
                    prefix
                ),
            }
        }
    };

    let middle = &clean[1..8]; // 7 digits
    let check_char = clean.chars().nth(8).unwrap();

    if !middle.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid {
            reason: "ES NIE: positions 2–8 must be digits".into(),
        };
    }
    if !check_char.is_ascii_alphabetic() {
        return Verdict::Invalid {
            reason: "ES NIE: 9th character must be a letter".into(),
        };
    }

    // Substitute prefix and treat the 8-char result as a NIF number.
    let numeric_str = format!("{}{}", sub, middle);
    let n: u32 = numeric_str.parse().unwrap();
    let expected = nif_check_letter(n);

    if expected == check_char {
        Verdict::Valid {
            formatted: format!("ES{}", clean),
            detected: "Spanish VAT (NIE)".into(),
        }
    } else {
        Verdict::Invalid {
            reason: format!(
                "ES NIE check mismatch: expected '{}', got '{}'",
                expected, check_char
            ),
        }
    }
}

pub fn create_es_nie(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, true);
    if clean.len() != 8 {
        return Err(anyhow!(
            "ES NIE create: expected prefix letter (X/Y/Z) + 7 digits (8 chars), got {}",
            clean.len()
        ));
    }
    let prefix = clean.chars().next().unwrap();
    let sub = match prefix {
        'X' => '0',
        'Y' => '1',
        'Z' => '2',
        _ => {
            return Err(anyhow!(
                "ES NIE create: first character must be X, Y, or Z, got '{}'",
                prefix
            ))
        }
    };
    let middle = &clean[1..];
    if !middle.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("ES NIE create: positions 2–8 must be digits"));
    }
    let numeric_str = format!("{}{}", sub, middle);
    let n: u32 = numeric_str.parse().unwrap();
    let letter = nif_check_letter(n);
    Ok(format!("ES{}{}", clean, letter))
}

// ---------------------------------------------------------------------------
// CIF
// ---------------------------------------------------------------------------

pub fn verify_es_cif(input: &str) -> Verdict {
    let clean = sanitize(input, true);
    if clean.len() != 9 {
        return Verdict::Invalid {
            reason: format!("ES CIF: expected 9 characters, got {}", clean.len()),
        };
    }

    let entity = clean.chars().next().unwrap();
    if !entity.is_ascii_alphabetic() {
        return Verdict::Invalid {
            reason: "ES CIF: first character must be a letter".into(),
        };
    }
    // Reject characters that would be caught by NIF/NIE dispatching
    if matches!(entity, 'X' | 'Y' | 'Z') {
        return Verdict::Invalid {
            reason: "ES CIF: first character X/Y/Z is reserved for NIE".into(),
        };
    }

    let body = &clean[1..8]; // 7 digits
    let check_char = clean.chars().nth(8).unwrap();

    if !body.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid {
            reason: "ES CIF: positions 2–8 must be digits".into(),
        };
    }

    let control = cif_control(body);
    let (needs_letter, needs_digit) = cif_check_type(entity);

    if check_char.is_ascii_alphabetic() {
        if !needs_letter {
            return Verdict::Invalid {
                reason: format!(
                    "ES CIF: entity type '{}' requires a digit check, got letter '{}'",
                    entity, check_char
                ),
            };
        }
        let expected_letter = CIF_LETTERS[control as usize] as char;
        if expected_letter == check_char {
            Verdict::Valid {
                formatted: format!("ES{}", clean),
                detected: "Spanish VAT (CIF)".into(),
            }
        } else {
            Verdict::Invalid {
                reason: format!(
                    "ES CIF check mismatch: expected letter '{}', got '{}'",
                    expected_letter, check_char
                ),
            }
        }
    } else if check_char.is_ascii_digit() {
        if !needs_digit {
            return Verdict::Invalid {
                reason: format!(
                    "ES CIF: entity type '{}' requires a letter check, got digit '{}'",
                    entity, check_char
                ),
            };
        }
        let check_digit = check_char.to_digit(10).unwrap();
        if control == check_digit {
            Verdict::Valid {
                formatted: format!("ES{}", clean),
                detected: "Spanish VAT (CIF)".into(),
            }
        } else {
            Verdict::Invalid {
                reason: format!(
                    "ES CIF check mismatch: expected digit '{}', got '{}'",
                    control, check_char
                ),
            }
        }
    } else {
        Verdict::Invalid {
            reason: format!(
                "ES CIF: last character must be a digit or letter, got '{}'",
                check_char
            ),
        }
    }
}

pub fn create_es_cif(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, true);
    if clean.len() != 8 {
        return Err(anyhow!(
            "ES CIF create: expected entity letter + 7 digits (8 chars), got {}",
            clean.len()
        ));
    }

    let entity = clean.chars().next().unwrap();
    if !entity.is_ascii_alphabetic() {
        return Err(anyhow!(
            "ES CIF create: first character must be a letter, got '{}'",
            entity
        ));
    }
    if matches!(entity, 'X' | 'Y' | 'Z') {
        return Err(anyhow!(
            "ES CIF create: first character X/Y/Z is reserved for NIE"
        ));
    }

    let body = &clean[1..];
    if !body.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("ES CIF create: positions 2–8 must be digits"));
    }

    let control = cif_control(body);
    let (needs_letter, needs_digit) = cif_check_type(entity);

    // For "either" types, prefer digit output; for letter-only, use letter.
    let check: String = if needs_letter && !needs_digit {
        (CIF_LETTERS[control as usize] as char).to_string()
    } else {
        control.to_string()
    };

    Ok(format!("ES{}{}", clean, check))
}

// ---------------------------------------------------------------------------
// Auto-detect dispatcher
// ---------------------------------------------------------------------------

pub fn verify_es_vat(input: &str) -> Verdict {
    let clean = sanitize(input, true);
    if clean.len() != 9 {
        return Verdict::Invalid {
            reason: format!("ES VAT: expected 9 characters, got {}", clean.len()),
        };
    }

    let first = match clean.chars().next() {
        Some(c) => c,
        None => {
            return Verdict::Invalid {
                reason: "ES VAT: empty input".into(),
            }
        }
    };

    if matches!(first, 'X' | 'Y' | 'Z') {
        verify_es_nie(&clean)
    } else if first.is_ascii_alphabetic() {
        verify_es_cif(&clean)
    } else if first.is_ascii_digit() {
        verify_es_nif(&clean)
    } else {
        Verdict::Invalid {
            reason: format!(
                "ES VAT: cannot determine variant from first character '{}'",
                first
            ),
        }
    }
}

pub fn create_es_vat(_input: &str, _raw: bool) -> Result<String> {
    Err(anyhow!(
        "ES VAT creation requires specifying a variant: use es-nif, es-nie, or es-cif"
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- NIF ---

    #[test]
    fn es_nif_valid_12345678z() {
        // 12345678 % 23 = 14 → NIF_LETTERS[14] = 'Z'
        match verify_es_nif("12345678Z") {
            Verdict::Valid { detected, formatted } => {
                assert_eq!(detected, "Spanish VAT (NIF)");
                assert_eq!(formatted, "ES12345678Z");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn es_nif_round_trip() {
        let full = create_es_nif("12345678", false).unwrap();
        assert_eq!(full, "ES12345678Z");
        let raw = &full[2..];
        match verify_es_nif(raw) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn es_nif_rejects_wrong_letter() {
        match verify_es_nif("12345678A") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    // --- NIE ---

    #[test]
    fn es_nie_valid_x1234567l() {
        // X→0: 01234567, 1234567 % 23 = 19 → NIF_LETTERS[19] = 'L'
        match verify_es_nie("X1234567L") {
            Verdict::Valid { detected, formatted } => {
                assert_eq!(detected, "Spanish VAT (NIE)");
                assert_eq!(formatted, "ESX1234567L");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn es_nie_round_trip() {
        let full = create_es_nie("X1234567", false).unwrap();
        assert_eq!(full, "ESX1234567L");
        let raw = &full[2..];
        match verify_es_nie(raw) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    // --- CIF ---

    #[test]
    fn es_cif_valid_a58818501() {
        // body=5881850: sum_doubled=15, sum_even=14, total=29, control=1 → digit '1'
        // Entity 'A' requires digit check.
        match verify_es_cif("A58818501") {
            Verdict::Valid { detected, formatted } => {
                assert_eq!(detected, "Spanish VAT (CIF)");
                assert_eq!(formatted, "ESA58818501");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn es_cif_round_trip() {
        let full = create_es_cif("A5881850", false).unwrap();
        assert_eq!(full, "ESA58818501");
        let raw = &full[2..];
        match verify_es_cif(raw) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn es_cif_letter_type_entity() {
        // Entity 'K' requires letter check. Build one and round-trip.
        let full = create_es_cif("K1234567", false).unwrap();
        // Verify it ends with a letter, not a digit.
        let check_char = full.chars().last().unwrap();
        assert!(check_char.is_ascii_alphabetic(), "expected letter check, got '{}'", check_char);
        let raw = &full[2..];
        match verify_es_cif(raw) {
            Verdict::Valid { detected, .. } => {
                assert_eq!(detected, "Spanish VAT (CIF)");
            }
            v => panic!("{:?}", v),
        }
    }

    // --- Auto-detect ---

    #[test]
    fn es_vat_autodetect_nif() {
        match verify_es_vat("12345678Z") {
            Verdict::Valid { detected, .. } => {
                assert_eq!(detected, "Spanish VAT (NIF)");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn es_vat_autodetect_nie() {
        match verify_es_vat("X1234567L") {
            Verdict::Valid { detected, .. } => {
                assert_eq!(detected, "Spanish VAT (NIE)");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn es_vat_autodetect_cif() {
        match verify_es_vat("A58818501") {
            Verdict::Valid { detected, .. } => {
                assert_eq!(detected, "Spanish VAT (CIF)");
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn es_vat_create_returns_error() {
        assert!(create_es_vat("12345678", false).is_err());
    }
}
