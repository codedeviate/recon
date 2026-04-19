//! Mod-10 EAN family: EAN-13, EAN-8, UPC-A, UPC-E, ISBN-13, GTIN, SSCC.
//!
//! Algorithm: working right-to-left (excluding the check digit), multiply
//! every second digit by 3 and the others by 1. Sum them. Check digit is
//! `(10 - sum % 10) % 10`.

use super::format::{group_fixed, group_variable};
use super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

/// Compute the check digit for an `n-1`-length digit string.
pub fn mod10_ean_check(body: &str) -> Result<u32> {
    let mut sum: u32 = 0;
    for (i, c) in body.chars().rev().enumerate() {
        let d = c.to_digit(10).ok_or_else(|| anyhow!("non-digit '{}'", c))?;
        sum += if i % 2 == 0 { d * 3 } else { d };
    }
    Ok((10 - sum % 10) % 10)
}

/// Verify a full `n`-length digit string including its trailing check digit.
pub fn mod10_ean_verify(full: &str) -> bool {
    if full.is_empty() || !full.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let body = &full[..full.len() - 1];
    let check: u32 = full.chars().last().and_then(|c| c.to_digit(10)).unwrap_or(99);
    match mod10_ean_check(body) {
        Ok(expected) => expected == check,
        Err(_) => false,
    }
}

fn verify_fixed_len(input: &str, n: usize, name: &str, formatter: fn(&str) -> String) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != n {
        return Verdict::Invalid { reason: format!("expected {} digits, got {}", n, clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    if mod10_ean_verify(&clean) {
        Verdict::Valid { formatted: formatter(&clean), detected: name.into(), comment: String::new() }
    } else {
        Verdict::Invalid { reason: format!("{} check digit mismatch", name) }
    }
}

fn create_fixed_len(input: &str, n: usize, raw: bool, formatter: fn(&str) -> String) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != n - 1 {
        return Err(anyhow!("expected {} digits, got {}", n - 1, clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let cd = mod10_ean_check(&clean)?;
    let full = format!("{}{}", clean, cd);
    if raw { Ok(full) } else { Ok(formatter(&full)) }
}

// ── Formatters ───────────────────────────────────────────────────────────

fn fmt_nop(s: &str) -> String { s.to_string() }
fn fmt_ean13(s: &str) -> String { group_variable(s, &[1, 6, 6], ' ') }
fn fmt_ean8(s: &str) -> String { group_fixed(s, 4, ' ') }
fn fmt_upca(s: &str) -> String { group_variable(s, &[1, 5, 5, 1], ' ') }
fn fmt_isbn13(s: &str) -> String { group_variable(s, &[3, 1, 2, 6, 1], '-') }
fn fmt_gtin14(s: &str) -> String { group_variable(s, &[1, 2, 5, 5, 1], ' ') }
fn fmt_sscc(s: &str) -> String { group_variable(s, &[1, 7, 9, 1], ' ') }

// ── Per-type verify/create ───────────────────────────────────────────────

pub fn verify_ean13(input: &str) -> Verdict { verify_fixed_len(input, 13, "EAN-13", fmt_ean13) }
pub fn create_ean13(input: &str, raw: bool) -> Result<String> { create_fixed_len(input, 13, raw, fmt_ean13) }

pub fn verify_ean8(input: &str) -> Verdict { verify_fixed_len(input, 8, "EAN-8", fmt_ean8) }
pub fn create_ean8(input: &str, raw: bool) -> Result<String> { create_fixed_len(input, 8, raw, fmt_ean8) }

pub fn verify_upca(input: &str) -> Verdict { verify_fixed_len(input, 12, "UPC-A", fmt_upca) }
pub fn create_upca(input: &str, raw: bool) -> Result<String> { create_fixed_len(input, 12, raw, fmt_upca) }

pub fn verify_upce(input: &str) -> Verdict { verify_fixed_len(input, 8, "UPC-E", fmt_nop) }
pub fn create_upce(input: &str, raw: bool) -> Result<String> { create_fixed_len(input, 8, raw, fmt_nop) }

pub fn verify_isbn13(input: &str) -> Verdict { verify_fixed_len(input, 13, "ISBN-13", fmt_isbn13) }
pub fn create_isbn13(input: &str, raw: bool) -> Result<String> { create_fixed_len(input, 13, raw, fmt_isbn13) }

pub fn verify_gtin8(input: &str) -> Verdict { verify_fixed_len(input, 8, "GTIN-8", fmt_ean8) }
pub fn create_gtin8(input: &str, raw: bool) -> Result<String> { create_fixed_len(input, 8, raw, fmt_ean8) }

pub fn verify_gtin12(input: &str) -> Verdict { verify_fixed_len(input, 12, "GTIN-12", fmt_upca) }
pub fn create_gtin12(input: &str, raw: bool) -> Result<String> { create_fixed_len(input, 12, raw, fmt_upca) }

pub fn verify_gtin13(input: &str) -> Verdict { verify_fixed_len(input, 13, "GTIN-13", fmt_ean13) }
pub fn create_gtin13(input: &str, raw: bool) -> Result<String> { create_fixed_len(input, 13, raw, fmt_ean13) }

pub fn verify_gtin14(input: &str) -> Verdict { verify_fixed_len(input, 14, "GTIN-14", fmt_gtin14) }
pub fn create_gtin14(input: &str, raw: bool) -> Result<String> { create_fixed_len(input, 14, raw, fmt_gtin14) }

pub fn verify_sscc(input: &str) -> Verdict { verify_fixed_len(input, 18, "SSCC", fmt_sscc) }
pub fn create_sscc(input: &str, raw: bool) -> Result<String> { create_fixed_len(input, 18, raw, fmt_sscc) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ean13_valid_5901234123457() {
        match verify_ean13("5901234123457") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "EAN-13"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ean13_rejects_wrong_length() {
        match verify_ean13("590123412345") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }

    #[test]
    fn upca_valid_036000291452() {
        match verify_upca("036000291452") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn isbn13_valid_9780306406157() {
        match verify_isbn13("978-0-306-40615-7") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn ean13_round_trip() {
        let body = "590123412345";
        let full = create_ean13(body, true).unwrap();
        assert!(mod10_ean_verify(&full));
    }

    #[test]
    fn gtin14_round_trip() {
        let body = "5012345678900";
        let full = create_gtin14(body, true).unwrap();
        assert_eq!(full.len(), 14);
        assert!(mod10_ean_verify(&full));
    }

    #[test]
    fn sscc_round_trip() {
        let body = "00012345555555555";
        let full = create_sscc(body, true).unwrap();
        assert_eq!(full.len(), 18);
        assert!(mod10_ean_verify(&full));
    }

    #[test]
    fn ean8_round_trip() {
        let body = "1234567";
        let full = create_ean8(body, true).unwrap();
        assert!(mod10_ean_verify(&full));
    }

    #[test]
    fn upca_format_1_5_5_1() {
        match verify_upca("036000291452") {
            Verdict::Valid { formatted, .. } => assert_eq!(formatted, "0 36000 29145 2"),
            v => panic!("{:?}", v),
        }
    }
}
