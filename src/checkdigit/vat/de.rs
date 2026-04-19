//! German VAT.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

// 9 digits. Running-product algorithm with seed 10:
//   for each of first 8 digits: sum = (digit + carry) mod 10; if sum == 0, sum = 10;
//     carry = (2 * sum) mod 11
//   Check digit = (11 - carry) mod 10.
pub fn verify_de_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Verdict::Invalid { reason: format!("expected 9 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let mut carry: u32 = 10;
    for c in clean[..8].chars() {
        let d = c.to_digit(10).unwrap();
        let mut sum = (d + carry) % 10;
        if sum == 0 { sum = 10; }
        carry = (2 * sum) % 11;
    }
    let expected = (11 - carry) % 10;
    let check = clean.chars().nth(8).unwrap().to_digit(10).unwrap();
    if expected == check {
        Verdict::Valid {
            formatted: format!("DE{}", clean),
            detected: "German VAT (USt-IdNr)".into(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("DE VAT check mismatch: expected {}, got {}", expected, check),
        }
    }
}

pub fn create_de_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let mut carry: u32 = 10;
    for c in clean.chars() {
        let d = c.to_digit(10).unwrap();
        let mut sum = (d + carry) % 10;
        if sum == 0 { sum = 10; }
        carry = (2 * sum) % 11;
    }
    let check = (11 - carry) % 10;
    Ok(format!("DE{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de_vat_round_trip() {
        let body = "13669597";
        let full = create_de_vat(body, false).unwrap();
        let clean: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_de_vat(&clean) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
