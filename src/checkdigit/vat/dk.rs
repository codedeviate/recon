//! Danish VAT.

use super::super::Verdict;
use anyhow::{anyhow, Result};

// 8 digits, weights [2,7,6,5,4,3,2,1], sum mod 11 == 0.
pub fn verify_dk_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "DK") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 8 {
        return Verdict::Invalid { reason: format!("expected 8 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let weights = [2u32, 7, 6, 5, 4, 3, 2, 1];
    let sum: u32 = clean.chars().enumerate()
        .map(|(i, c)| weights[i] * c.to_digit(10).unwrap())
        .sum();
    if sum % 11 == 0 {
        Verdict::Valid {
            formatted: format!("DK{}", clean),
            detected: "Danish VAT (CVR)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid { reason: "DK VAT mod-11 check failed".into() }
    }
}

pub fn create_dk_vat(_input: &str, _raw: bool) -> Result<String> {
    Err(anyhow!("DK VAT has no trailing check digit — the entire 8-digit number is weighted together. Use --checkdigit dk-vat to verify."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dk_vat_reference_13585628() {
        match verify_dk_vat("13585628") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn dk_vat_rejects_bad_length() {
        match verify_dk_vat("1358562") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }
}
