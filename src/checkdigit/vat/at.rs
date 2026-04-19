//! Austrian VAT (UID).

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 7] = [1, 2, 1, 2, 1, 2, 1];

/// Reduce a weighted product: if weight is 2 and product > 9, sum its digits.
fn reduce(d: u32, w: u32) -> u32 {
    let p = d * w;
    if w == 2 && p > 9 { p / 10 + p % 10 } else { p }
}

/// Strip optional "ATU" or "U" prefix from an already-uppercased string.
fn strip_prefix(s: &str) -> &str {
    if s.starts_with("ATU") {
        &s[3..]
    } else if s.starts_with('U') {
        &s[1..]
    } else {
        s
    }
}

pub fn verify_at_vat(input: &str) -> Verdict {
    let upped = sanitize(input, true);
    let digits = strip_prefix(&upped);
    if digits.len() != 8 {
        return Verdict::Invalid { reason: format!("expected 8 digits after optional ATU/U prefix, got {}", digits.len()) };
    }
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit body".into() };
    }
    let body = &digits[..7];
    let check: u32 = digits.chars().nth(7).unwrap().to_digit(10).unwrap();
    let s: u32 = body.chars().enumerate()
        .map(|(i, c)| reduce(c.to_digit(10).unwrap(), WEIGHTS[i]))
        .sum();
    let expected = (10 - (s + 4) % 10) % 10;
    let clean_digits = digits.to_string();
    if expected == check {
        Verdict::Valid { formatted: format!("AT{}", clean_digits), detected: "Austrian VAT (UID)".into() }
    } else {
        Verdict::Invalid { reason: format!("AT VAT check mismatch: expected {}, got {}", expected, check) }
    }
}

pub fn create_at_vat(input: &str, _raw: bool) -> Result<String> {
    let upped = sanitize(input, true);
    let stripped = strip_prefix(&upped);
    // Accept either 7 (body only) or 8 (with check) — but spec says 7-digit body
    if stripped.len() != 7 {
        return Err(anyhow!("expected 7 digits (body without check), got {}", stripped.len()));
    }
    if !stripped.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit body"));
    }
    let s: u32 = stripped.chars().enumerate()
        .map(|(i, c)| reduce(c.to_digit(10).unwrap(), WEIGHTS[i]))
        .sum();
    let check = (10 - (s + 4) % 10) % 10;
    Ok(format!("AT{}{}", stripped, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_vat_valid_12345675() {
        match verify_at_vat("12345675") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn at_vat_valid_with_atu_prefix() {
        match verify_at_vat("ATU12345675") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn at_vat_round_trip() {
        let body = "1234567";
        let full = create_at_vat(body, false).unwrap();
        let clean: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_at_vat(&clean) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn at_vat_rejects_bad_check() {
        match verify_at_vat("12345670") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
