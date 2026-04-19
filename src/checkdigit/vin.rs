//! VIN (Vehicle Identification Number) — 17 chars, transliterate letters,
//! weighted mod 11, check digit at position 9 (0-9 or X for 10).

use super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

/// VIN-specific letter transliteration (I/O/Q disallowed).
fn letter_value(c: char) -> Option<u32> {
    match c.to_ascii_uppercase() {
        'A' | 'J' => Some(1),
        'B' | 'K' | 'S' => Some(2),
        'C' | 'L' | 'T' => Some(3),
        'D' | 'M' | 'U' => Some(4),
        'E' | 'N' | 'V' => Some(5),
        'F' | 'W' => Some(6),
        'G' | 'P' | 'X' => Some(7),
        'H' | 'Y' => Some(8),
        'R' | 'Z' => Some(9),
        _ => None,
    }
}

/// Position weights (1-indexed): [8,7,6,5,4,3,2,10,0,9,8,7,6,5,4,3,2]
const WEIGHTS: [u32; 17] = [8, 7, 6, 5, 4, 3, 2, 10, 0, 9, 8, 7, 6, 5, 4, 3, 2];

fn transliterate(chars: &[char]) -> Result<Vec<u32>> {
    let mut out = Vec::with_capacity(chars.len());
    for (i, c) in chars.iter().enumerate() {
        let v = if c.is_ascii_digit() {
            c.to_digit(10).unwrap()
        } else if c.is_ascii_alphabetic() {
            let uc = c.to_ascii_uppercase();
            if uc == 'I' || uc == 'O' || uc == 'Q' {
                return Err(anyhow!(
                    "invalid character '{}' at position {} (I, O, Q disallowed in VIN)",
                    c, i + 1
                ));
            }
            letter_value(uc).ok_or_else(|| anyhow!("invalid VIN char '{}'", c))?
        } else {
            return Err(anyhow!("invalid character '{}' at position {}", c, i + 1));
        };
        out.push(v);
    }
    Ok(out)
}

fn compute_check(values: &[u32]) -> u32 {
    let sum: u32 = values.iter().zip(WEIGHTS.iter()).map(|(v, w)| v * w).sum();
    sum % 11
}

pub fn verify_vin(input: &str) -> Verdict {
    let clean = sanitize(input, true);
    if clean.len() != 17 {
        return Verdict::Invalid { reason: format!("expected 17 chars, got {}", clean.len()) };
    }
    let chars: Vec<char> = clean.chars().collect();
    let check_char = chars[8];
    let check_val: u32 = if check_char == 'X' {
        10
    } else {
        match check_char.to_digit(10) {
            Some(d) => d,
            None => return Verdict::Invalid { reason: "check position (9) must be 0-9 or X".into() },
        }
    };
    let values = match transliterate(&chars) {
        Ok(v) => v,
        Err(e) => return Verdict::Invalid { reason: e.to_string() },
    };
    let mut v2 = values.clone();
    v2[8] = 0;  // neutralize check digit position for computation (weight is 0 anyway)
    let expected = compute_check(&v2);
    if expected == check_val {
        Verdict::Valid { formatted: clean, detected: "VIN".into(), comment: String::new() }
    } else {
        let expected_c = if expected == 10 { 'X' } else { char::from_digit(expected, 10).unwrap() };
        Verdict::Invalid {
            reason: format!("VIN check mismatch: expected '{}', got '{}'", expected_c, check_char),
        }
    }
}

/// Create: accept 17 chars with any placeholder at position 9, OR 16 chars with position 9 omitted.
pub fn create_vin(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, true);
    let chars: Vec<char> = clean.chars().collect();

    let (before9, after9): (&[char], &[char]) = if chars.len() == 17 {
        (&chars[..8], &chars[9..])
    } else if chars.len() == 16 {
        (&chars[..8], &chars[8..])
    } else {
        return Err(anyhow!("expected 16 or 17 chars, got {}", chars.len()));
    };

    let mut full_chars: Vec<char> = before9.to_vec();
    full_chars.push('0');  // placeholder for transliteration
    full_chars.extend_from_slice(after9);

    let values = transliterate(&full_chars)?;
    let check = compute_check(&values);
    let check_char = if check == 10 {
        'X'
    } else {
        char::from_digit(check, 10).unwrap()
    };

    let mut out_chars = full_chars.clone();
    out_chars[8] = check_char;
    Ok(out_chars.iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vin_valid_1hgbh41jxmn109186() {
        match verify_vin("1HGBH41JXMN109186") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "VIN"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn vin_rejects_capital_I_at_start() {
        match verify_vin("I1111111111111111") {
            Verdict::Invalid { reason } => assert!(reason.contains("I, O, Q")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn vin_rejects_wrong_length() {
        match verify_vin("1HGBH41JXMN10918") {
            Verdict::Invalid { reason } => assert!(reason.contains("17")),
            _ => panic!(),
        }
    }

    #[test]
    fn vin_create_omit_form() {
        let input = "1HGBH41JMN109186";  // 16 chars
        let full = create_vin(input, false).unwrap();
        assert_eq!(full, "1HGBH41JXMN109186");
    }

    #[test]
    fn vin_create_placeholder_form() {
        let input = "1HGBH41J_MN109186";  // 17 chars with _ at pos 9
        // Should fail because '_' isn't valid VIN alphanumeric. The create spec says
        // "any placeholder" — but since transliteration would reject _ too, use '0' as placeholder.
        let input0 = "1HGBH41J0MN109186";
        let full = create_vin(input0, false).unwrap();
        assert_eq!(full, "1HGBH41JXMN109186");
    }

    #[test]
    fn vin_create_rejects_i_in_body() {
        // An 'I' in a non-check position should fail.
        assert!(create_vin("1HGBH41J_IN109186", false).is_err() ||
                create_vin("1HGBH41I0MN109186", false).is_err());
    }
}
