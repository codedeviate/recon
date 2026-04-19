//! Polish VAT (NIP).

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const WEIGHTS: [u32; 9] = [6, 5, 7, 2, 3, 4, 5, 6, 7];

pub fn verify_pl_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 10 {
        return Verdict::Invalid { reason: format!("expected 10 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let body = &clean[..9];
    let check: u32 = clean.chars().nth(9).unwrap().to_digit(10).unwrap();
    let sum: u32 = body.chars().enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap())
        .sum();
    let expected = sum % 11;
    if expected == 10 {
        return Verdict::Invalid { reason: "PL NIP: computed check digit is 10 — invalid".into() };
    }
    if expected == check {
        Verdict::Valid { formatted: format!("PL{}", clean), detected: "Polish VAT (NIP)".into(), comment: String::new() }
    } else {
        Verdict::Invalid { reason: format!("PL NIP check mismatch: expected {}, got {}", expected, check) }
    }
}

pub fn create_pl_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!("expected 9 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let sum: u32 = clean.chars().enumerate()
        .map(|(i, c)| WEIGHTS[i] * c.to_digit(10).unwrap())
        .sum();
    let check = sum % 11;
    if check == 10 {
        return Err(anyhow!("PL NIP: no valid check digit (computed 10)"));
    }
    Ok(format!("PL{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pl_nip_valid_5261040828() {
        match verify_pl_vat("5261040828") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn pl_nip_round_trip() {
        let body = "526104082";
        let full = create_pl_vat(body, false).unwrap();
        let clean: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_pl_vat(&clean) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn pl_nip_rejects_wrong_length() {
        match verify_pl_vat("526104082") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }
}
