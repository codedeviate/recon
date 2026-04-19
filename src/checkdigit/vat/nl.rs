//! Dutch VAT (BTW-nummer).
//!
//! 12 characters: 9 digits + literal 'B' + 2-digit sub-entity suffix.
//! The first 9 digits satisfy the elfproef (mod-11) with weights [9,8,7,6,5,4,3,2,-1]:
//!   sum = Σ weights[i] * digits[i], sum % 11 == 0 and sum != 0.
//! The 'B' is a required separator. The trailing 2 digits identify a sub-entity
//! and are not part of the check calculation.
//!
//! Distinct from Dutch BSN (bsn/nl-id) which uses the same elfproef math but
//! has no 'B' separator and no sub-entity suffix.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [i32; 9] = [9, 8, 7, 6, 5, 4, 3, 2, -1];

/// Validate the 9-digit body via elfproef.
fn elfproef_valid(body: &str) -> bool {
    if body.len() != 9 || !body.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let sum: i32 = body
        .chars()
        .enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap() as i32)
        .sum();
    sum != 0 && sum % 11 == 0
}

pub fn verify_nl_vat(input: &str) -> Verdict {
    // sanitize with uppercase=true so any lowercase 'b' becomes 'B'
    let clean = sanitize(input, true);
    if clean.len() != 12 {
        return Verdict::Invalid {
            reason: format!("expected 12 chars (9 digits + 'B' + 2 digits), got {}", clean.len()),
        };
    }
    // Position 9 must be 'B'
    if clean.chars().nth(9) != Some('B') {
        return Verdict::Invalid {
            reason: "expected 'B' at position 10".into(),
        };
    }
    let body = &clean[..9];
    let suffix = &clean[10..];
    if !body.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit in body (positions 1-9)".into() };
    }
    if !suffix.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit in sub-entity suffix (positions 11-12)".into() };
    }
    if elfproef_valid(body) {
        Verdict::Valid {
            formatted: format!("NL{}", clean),
            detected: "Dutch VAT".into(),
        }
    } else {
        Verdict::Invalid { reason: "NL VAT elfproef (mod-11) check failed".into() }
    }
}

pub fn create_nl_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits (body without check digit), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    // Compute the 9th digit: partial_sum + (-1)*d8 ≡ 0 (mod 11), sum != 0.
    let partial_sum: i32 = clean
        .chars()
        .enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap() as i32)
        .sum();
    for d8 in 0i32..=9 {
        let total = partial_sum + WEIGHTS[8] * d8; // WEIGHTS[8] == -1
        if total != 0 && total % 11 == 0 {
            return Ok(format!("NL{}{}B01", clean, d8));
        }
    }
    Err(anyhow!("no valid check digit found for body '{}' (no digit 0-9 satisfies elfproef)", clean))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nl_vat_valid_123456782b01() {
        match verify_nl_vat("123456782B01") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn nl_vat_round_trip() {
        let body = "12345678";
        let full = create_nl_vat(body, false).unwrap();
        // Strip "NL" prefix for verification
        let inner = &full[2..];
        match verify_nl_vat(inner) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn nl_vat_rejects_bad_check() {
        // Change check digit to make elfproef fail
        match verify_nl_vat("123456783B01") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn nl_vat_rejects_missing_b() {
        match verify_nl_vat("12345678201") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
