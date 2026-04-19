//! Luhn mod-10 algorithm.
//!
//! Verify: starting from rightmost digit, every second digit is doubled.
//! If doubling yields >9, sum its digits (equivalent to subtracting 9).
//! Total sum mod 10 == 0 means valid.
//!
//! Create: apply Luhn with a '0' placeholder at the check position, compute
//! what digit would make the sum divisible by 10.

use super::{sanitize, Verdict, MAX_INPUT_LEN};
use anyhow::{anyhow, Result};

/// Verify a pure-digit Luhn string. `full` includes the check digit.
pub fn luhn_verify(full: &str) -> bool {
    let mut sum = 0u32;
    for (i, c) in full.chars().rev().enumerate() {
        let d = match c.to_digit(10) {
            Some(d) => d,
            None => return false,
        };
        let weighted = if i % 2 == 1 {
            let x = d * 2;
            if x > 9 { x - 9 } else { x }
        } else {
            d
        };
        sum += weighted;
    }
    sum % 10 == 0
}

/// Compute the check digit for an (n-1)-length digit string. Returns 0-9.
pub fn luhn_check_digit(body: &str) -> Result<u32> {
    let mut sum = 0u32;
    // Body becomes positions 2..=n from right after we append a 0 placeholder.
    // Every digit of body starts at rev-index 1 and alternates.
    for (i, c) in body.chars().rev().enumerate() {
        let d = c.to_digit(10).ok_or_else(|| anyhow!("non-digit '{}' in input", c))?;
        let weighted = if i % 2 == 0 {
            let x = d * 2;
            if x > 9 { x - 9 } else { x }
        } else {
            d
        };
        sum += weighted;
    }
    Ok((10 - (sum % 10)) % 10)
}

/// `luhn` keyword: bare Luhn on any digit string, no length/format constraints.
pub fn verify_bare(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() > MAX_INPUT_LEN {
        return Verdict::Invalid { reason: "input too long".into() };
    }
    if clean.is_empty() {
        return Verdict::Invalid { reason: "empty input".into() };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    if luhn_verify(&clean) {
        Verdict::Valid { formatted: clean, detected: "Luhn".into() }
    } else {
        Verdict::Invalid { reason: "Luhn check failed".into() }
    }
}

pub fn create_bare(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() > MAX_INPUT_LEN {
        return Err(anyhow!("input too long"));
    }
    if clean.is_empty() {
        return Err(anyhow!("empty input"));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let cd = luhn_check_digit(&clean)?;
    Ok(format!("{}{}", clean, cd))
}

/// Expand A-Z letters to their 2-digit numeric value (A=10..Z=35). Digits pass through.
pub fn transliterate_alnum(s: &str) -> Result<String> {
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if c.is_ascii_digit() {
            out.push(c);
        } else if c.is_ascii_alphabetic() {
            let v = (c.to_ascii_uppercase() as u8 - b'A') as u32 + 10;
            out.push_str(&v.to_string());
        } else {
            return Err(anyhow!("invalid character '{}'", c));
        }
    }
    Ok(out)
}

/// Luhn verify on `prefix + body`.
pub fn luhn_verify_with_prefix(prefix: &str, body: &str) -> bool {
    let combined = format!("{}{}", prefix, body);
    luhn_verify(&combined)
}

/// Luhn check-digit of `prefix + body` (no final check digit yet).
pub fn luhn_check_digit_with_prefix(prefix: &str, body: &str) -> Result<u32> {
    let combined = format!("{}{}", prefix, body);
    luhn_check_digit(&combined)
}

pub fn verify_isin(input: &str) -> Verdict {
    let clean = sanitize(input, true);
    if clean.len() != 12 {
        return Verdict::Invalid { reason: format!("expected 12 chars, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Verdict::Invalid { reason: "non-alphanumeric input".into() };
    }
    let expanded = match transliterate_alnum(&clean) {
        Ok(s) => s,
        Err(e) => return Verdict::Invalid { reason: e.to_string() },
    };
    if luhn_verify(&expanded) {
        Verdict::Valid { formatted: clean, detected: "ISIN".into() }
    } else {
        Verdict::Invalid { reason: "Luhn check failed".into() }
    }
}

pub fn create_isin(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, true);
    if clean.len() != 11 {
        return Err(anyhow!("expected 11 chars (ISIN body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(anyhow!("non-alphanumeric input"));
    }
    let expanded = transliterate_alnum(&clean)?;
    let cd = luhn_check_digit(&expanded)?;
    Ok(format!("{}{}", clean, cd))
}

pub fn verify_npi(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 10 {
        return Verdict::Invalid { reason: format!("expected 10 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    if luhn_verify_with_prefix("80840", &clean) {
        Verdict::Valid { formatted: clean, detected: "NPI".into() }
    } else {
        Verdict::Invalid { reason: "Luhn check failed (with 80840 prefix)".into() }
    }
}

pub fn create_npi(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!("expected 9 digits (NPI body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let cd = luhn_check_digit_with_prefix("80840", &clean)?;
    Ok(format!("{}{}", clean, cd))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luhn_verifies_visa_test_number() {
        assert!(luhn_verify("4111111111111111"));
    }

    #[test]
    fn luhn_rejects_one_flipped_digit() {
        assert!(!luhn_verify("4111111111111112"));
    }

    #[test]
    fn luhn_verifies_amex_test_number() {
        assert!(luhn_verify("378282246310005"));
    }

    #[test]
    fn luhn_verifies_mastercard_test_number() {
        assert!(luhn_verify("5105105105105100"));
    }

    #[test]
    fn luhn_check_digit_matches_known_body() {
        // Visa: body = 411111111111111, expected check digit = 1
        assert_eq!(luhn_check_digit("411111111111111").unwrap(), 1);
    }

    #[test]
    fn luhn_round_trip() {
        let body = "12345678901234";
        let cd = luhn_check_digit(body).unwrap();
        let full = format!("{}{}", body, cd);
        assert!(luhn_verify(&full));
    }

    #[test]
    fn verify_bare_rejects_empty() {
        match verify_bare("") {
            Verdict::Invalid { .. } => {}
            _ => panic!("expected Invalid"),
        }
    }

    #[test]
    fn verify_bare_rejects_letters() {
        match verify_bare("4111ABCD11111111") {
            Verdict::Invalid { .. } => {}
            _ => panic!("expected Invalid"),
        }
    }

    #[test]
    fn transliterate_us_is_3028() {
        assert_eq!(transliterate_alnum("US").unwrap(), "3028");
    }

    #[test]
    fn transliterate_all_digits_passes_through() {
        assert_eq!(transliterate_alnum("0378331005").unwrap(), "0378331005");
    }

    #[test]
    fn transliterate_rejects_non_alnum() {
        assert!(transliterate_alnum("A!B").is_err());
    }

    #[test]
    fn isin_apple_us0378331005_is_valid() {
        match verify_isin("US0378331005") {
            Verdict::Valid { .. } => {}
            v => panic!("expected Valid, got {:?}", v),
        }
    }

    #[test]
    fn isin_rejects_bad_length() {
        match verify_isin("US037833100") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn isin_round_trip() {
        let body = "US037833100";
        let full = create_isin(body, false).unwrap();
        assert_eq!(full, "US0378331005");
        match verify_isin(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn npi_1234567893_valid() {
        match verify_npi("1234567893") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "NPI"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn npi_rejects_wrong_length() {
        match verify_npi("123456789") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }

    #[test]
    fn npi_round_trip() {
        let body = "123456789";
        let full = create_npi(body, false).unwrap();
        assert!(full.starts_with(body));
        assert_eq!(full.len(), 10);
        match verify_npi(&full) {
            Verdict::Valid { .. } => {}
            _ => panic!(),
        }
    }
}
