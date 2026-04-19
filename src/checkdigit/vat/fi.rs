//! Finnish VAT.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

// 8 digits: 7-digit body + check digit. Weights [7,9,10,5,8,4,2] on first 7.
// check = (11 - sum mod 11) mod 11. If check == 10, invalid.
pub fn verify_fi_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Verdict::Invalid { reason: format!("expected 8 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let weights = [7u32, 9, 10, 5, 8, 4, 2];
    let body = &clean[..7];
    let check = clean.chars().nth(7).unwrap().to_digit(10).unwrap();
    let sum: u32 = body.chars().enumerate()
        .map(|(i, c)| weights[i] * c.to_digit(10).unwrap())
        .sum();
    let expected = (11 - (sum % 11)) % 11;
    if expected == 10 {
        return Verdict::Invalid { reason: "FI VAT: computed check is 10 — number invalid".into() };
    }
    if expected == check {
        Verdict::Valid {
            formatted: format!("FI{}", clean),
            detected: "Finnish VAT (Y-tunnus)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("FI VAT check mismatch: expected {}, got {}", expected, check),
        }
    }
}

pub fn create_fi_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 7 {
        return Err(anyhow!("expected 7 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let weights = [7u32, 9, 10, 5, 8, 4, 2];
    let sum: u32 = clean.chars().enumerate()
        .map(|(i, c)| weights[i] * c.to_digit(10).unwrap())
        .sum();
    let check = (11 - (sum % 11)) % 11;
    if check == 10 {
        return Err(anyhow!("FI VAT: no valid check digit (computed 10)"));
    }
    Ok(format!("FI{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fi_vat_round_trip() {
        let body = "2077474";
        let full = create_fi_vat(body, false).unwrap();
        let clean: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_fi_vat(&clean) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
