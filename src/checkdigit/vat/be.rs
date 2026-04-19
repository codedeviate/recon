//! Belgian VAT.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

pub fn verify_be_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "BE") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 10 {
        return Verdict::Invalid { reason: format!("expected 10 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let body: u64 = clean[..8].parse().expect("validated above");
    let check: u64 = clean[8..].parse().expect("validated above");
    if (body + check) % 97 == 0 {
        Verdict::Valid { formatted: format!("BE{}", clean), detected: "Belgian VAT".into(), comment: String::new() }
    } else {
        Verdict::Invalid { reason: format!("BE VAT check failed: ({} + {}) mod 97 != 0", body, check) }
    }
}

pub fn create_be_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits (body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let body: u64 = clean.parse().expect("validated above");
    let check = 97 - body % 97;
    Ok(format!("BE{}{:02}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn be_vat_valid_0776091951() {
        match verify_be_vat("0776091951") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn be_vat_round_trip() {
        let body = "07760919";
        let full = create_be_vat(body, false).unwrap();
        let clean: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_be_vat(&clean) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn be_vat_rejects_bad_check() {
        match verify_be_vat("0776091952") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
