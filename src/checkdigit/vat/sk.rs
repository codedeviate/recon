//! Slovak VAT.

use super::super::Verdict;
use anyhow::{anyhow, Result};

pub fn verify_sk_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "SK") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 10 {
        return Verdict::Invalid { reason: format!("expected 10 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    if clean.starts_with('0') {
        return Verdict::Invalid { reason: "SK VAT first digit must not be 0".into() };
    }
    let n: u64 = clean.parse().expect("validated above");
    if n % 11 == 0 {
        Verdict::Valid { formatted: format!("SK{}", clean), detected: "Slovak VAT".into(), comment: String::new() }
    } else {
        Verdict::Invalid { reason: format!("SK VAT: {} mod 11 != 0", clean) }
    }
}

pub fn create_sk_vat(_input: &str, _raw: bool) -> Result<String> {
    Err(anyhow!("SK VAT has no trailing check digit — the full 10-digit number must be divisible by 11. Use --checkdigit sk-vat to verify."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sk_vat_valid_2022749619() {
        match verify_sk_vat("2022749619") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn sk_vat_rejects_leading_zero() {
        match verify_sk_vat("0022749619") {
            Verdict::Invalid { reason } => assert!(reason.contains("must not be 0")),
            _ => panic!(),
        }
    }

    #[test]
    fn sk_vat_rejects_non_divisible() {
        match verify_sk_vat("2022749618") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }
}
