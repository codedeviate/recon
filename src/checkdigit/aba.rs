//! US ABA routing number — 9 digits, weighted mod 10 with [3, 7, 1] repeating.

use super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

fn aba_check_sum(digits: &[u32]) -> u32 {
    let weights = [3u32, 7, 1];
    digits.iter()
        .enumerate()
        .map(|(i, d)| d * weights[i % 3])
        .sum::<u32>() % 10
}

pub fn verify_aba(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Verdict::Invalid { reason: format!("expected 9 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    if aba_check_sum(&digits) == 0 {
        Verdict::Valid { formatted: clean, detected: "ABA routing number".into(), comment: String::new() }
    } else {
        Verdict::Invalid { reason: "ABA weighted mod-10 check failed".into() }
    }
}

pub fn create_aba(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    for cd in 0..=9u32 {
        let full: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).chain(std::iter::once(cd)).collect();
        if aba_check_sum(&full) == 0 {
            return Ok(format!("{}{}", clean, cd));
        }
    }
    Err(anyhow!("no valid ABA check digit exists"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aba_valid_122105155() {
        match verify_aba("122105155") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "ABA routing number"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn aba_round_trip() {
        let full = create_aba("12210515", false).unwrap();
        match verify_aba(&full) {
            Verdict::Valid { .. } => {}
            _ => panic!(),
        }
    }

    #[test]
    fn aba_rejects_8_digits() {
        match verify_aba("12210515") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }
}
