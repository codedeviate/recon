//! Portuguese VAT (NIF).

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 8] = [9, 8, 7, 6, 5, 4, 3, 2];

pub fn verify_pt_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Verdict::Invalid { reason: format!("expected 9 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let body = &clean[..8];
    let check: u32 = clean.chars().nth(8).unwrap().to_digit(10).unwrap();
    let sum: u32 = body.chars().enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap())
        .sum();
    let expected = (11 - sum % 11) % 11;
    if expected == 10 {
        return Verdict::Invalid { reason: "PT NIF: computed check digit is 10 — invalid".into() };
    }
    if expected == check {
        Verdict::Valid { formatted: format!("PT{}", clean), detected: "Portuguese VAT (NIF)".into(), comment: String::new() }
    } else {
        Verdict::Invalid { reason: format!("PT NIF check mismatch: expected {}, got {}", expected, check) }
    }
}

pub fn create_pt_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let sum: u32 = clean.chars().enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap())
        .sum();
    let check = (11 - sum % 11) % 11;
    if check == 10 {
        return Err(anyhow!("PT NIF: no valid check digit (computed 10)"));
    }
    Ok(format!("PT{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pt_nif_valid_502757191() {
        match verify_pt_vat("502757191") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn pt_nif_round_trip() {
        let body = "50275719";
        let full = create_pt_vat(body, false).unwrap();
        let clean: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_pt_vat(&clean) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
