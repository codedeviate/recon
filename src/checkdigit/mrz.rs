//! Passport MRZ check digits (ICAO Doc 9303).
//!
//! Supports TD1 (3 × 30, ID card), TD2 (2 × 36, ID card), TD3 (2 × 44, passport).
//! Weighted mod 10 with weights [7, 3, 1] repeating. Letters A-Z = 10..35,
//! filler '<' = 0.

use super::Verdict;
use anyhow::{anyhow, Result};

fn mrz_value(c: char) -> Option<u32> {
    if c == '<' {
        Some(0)
    } else if c.is_ascii_digit() {
        c.to_digit(10)
    } else if c.is_ascii_alphabetic() {
        Some((c.to_ascii_uppercase() as u8 - b'A') as u32 + 10)
    } else {
        None
    }
}

fn mrz_check(field: &str) -> Option<u32> {
    let weights = [7u32, 3, 1];
    let mut sum = 0u32;
    for (i, c) in field.chars().enumerate() {
        let v = mrz_value(c)?;
        sum += v * weights[i % 3];
    }
    Some(sum % 10)
}

pub fn verify_mrz(input: &str) -> Verdict {
    let lines: Vec<&str> = input.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let (rows, width) = match (lines.len(), lines.first().map(|l| l.len()).unwrap_or(0)) {
        (3, 30) => (3, 30),
        (2, 36) => (2, 36),
        (2, 44) => (2, 44),
        _ => return Verdict::Invalid {
            reason: format!(
                "unknown MRZ format ({} lines × {} chars; expected 3×30, 2×36, or 2×44)",
                lines.len(), lines.first().map(|l| l.len()).unwrap_or(0)
            ),
        },
    };
    if lines.iter().any(|l| l.len() != width) {
        return Verdict::Invalid { reason: "MRZ line widths inconsistent".into() };
    }

    // Extract checks per format. We use slicing on bytes — MRZ is pure ASCII so byte-indexing is safe.
    let (detected, checks): (&str, Vec<(&str, &str, char)>) = match (rows, width) {
        (2, 44) => {
            // TD3 passport
            let line2 = lines[1];
            (
                "MRZ (TD3 passport)",
                vec![
                    ("document number", &line2[0..9], line2.as_bytes()[9] as char),
                    ("date of birth", &line2[13..19], line2.as_bytes()[19] as char),
                    ("date of expiry", &line2[21..27], line2.as_bytes()[27] as char),
                ],
            )
        }
        (2, 36) => {
            // TD2 ID card
            let line2 = lines[1];
            (
                "MRZ (TD2 ID card)",
                vec![
                    ("document number", &line2[0..9], line2.as_bytes()[9] as char),
                    ("date of birth", &line2[13..19], line2.as_bytes()[19] as char),
                    ("date of expiry", &line2[21..27], line2.as_bytes()[27] as char),
                ],
            )
        }
        (3, 30) => {
            // TD1 ID card
            let line1 = lines[0];
            let line2 = lines[1];
            (
                "MRZ (TD1 ID card)",
                vec![
                    ("document number", &line1[5..14], line1.as_bytes()[14] as char),
                    ("date of birth", &line2[0..6], line2.as_bytes()[6] as char),
                    ("date of expiry", &line2[8..14], line2.as_bytes()[14] as char),
                ],
            )
        }
        _ => unreachable!(),
    };

    for (name, field, expected_char) in checks {
        let expected = match expected_char.to_digit(10) {
            Some(d) => d,
            None => return Verdict::Invalid { reason: format!("{} check position not a digit", name) },
        };
        let computed = match mrz_check(field) {
            Some(c) => c,
            None => return Verdict::Invalid { reason: format!("{} contains invalid character", name) },
        };
        if computed != expected {
            return Verdict::Invalid {
                reason: format!("{} check digit mismatch (expected {}, got {})", name, computed, expected),
            };
        }
    }

    Verdict::Valid { formatted: input.trim().to_string(), detected: detected.into(), comment: String::new() }
}

pub fn create_mrz(_input: &str, _raw: bool) -> Result<String> {
    Err(anyhow!("MRZ creation is not supported — requires the whole document structure including composite fields; use --checkdigit mrz to verify an existing MRZ"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mrz_td3_passport_valid() {
        // Wikipedia ICAO reference passport MRZ.
        let mrz = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\nL898902C36UTO7408122F1204159ZE184226B<<<<<10";
        match verify_mrz(mrz) {
            Verdict::Valid { detected, .. } => assert!(detected.contains("TD3")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn mrz_unknown_format_rejected() {
        let mrz = "ABC\nDEF";  // 2x3, not a known size
        match verify_mrz(mrz) {
            Verdict::Invalid { reason } => assert!(reason.contains("unknown MRZ format")),
            _ => panic!(),
        }
    }
}
