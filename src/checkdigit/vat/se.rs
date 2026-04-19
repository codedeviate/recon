//! Swedish VAT.

use super::super::luhn::{luhn_check_digit, luhn_verify};
use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

// 12 digits: organisationsnummer (10 digits) + "01". Luhn on the first 10.
pub fn verify_se_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 12 {
        return Verdict::Invalid { reason: format!("expected 12 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let suffix = &clean[10..];
    if suffix == "00" {
        return Verdict::Invalid { reason: "SE VAT suffix '00' is not valid".into() };
    }
    let orgnr = &clean[..10];
    if luhn_verify(orgnr) {
        let comment = if suffix == "01" {
            String::new()
        } else {
            format!("suffix {} (unusual — typically 01, used when one org.nr has multiple VAT-registered entities)", suffix)
        };
        Verdict::Valid { formatted: format!("SE{}", clean), detected: "Swedish VAT".into(), comment }
    } else {
        Verdict::Invalid { reason: "SE VAT Luhn check on org.nr failed".into() }
    }
}

pub fn create_se_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!("expected 9 digits (org.nr body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let cd = luhn_check_digit(&clean)?;
    Ok(format!("SE{}{}01", clean, cd))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_vat_round_trip() {
        let body = "556036079";
        let full = create_se_vat(body, false).unwrap();
        let clean: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_se_vat(&clean) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn se_vat_accepts_02_suffix_with_comment() {
        // Build a valid SE VAT with 02 suffix from a known valid org.nr.
        let body = "556036079";
        let with_01 = create_se_vat(body, false).unwrap();  // "SE556036079301"
        // Substitute the last 2 chars 01 → 02 to get 302 suffix.
        let digits: String = with_01.chars().filter(|c| c.is_ascii_digit()).collect();
        let with_02 = format!("{}02", &digits[..10]);
        match verify_se_vat(&with_02) {
            Verdict::Valid { comment, .. } => {
                assert!(comment.contains("02"), "expected '02' in comment, got {:?}", comment);
                assert!(comment.contains("unusual"), "expected 'unusual' in comment, got {:?}", comment);
            }
            v => panic!("expected Valid with comment, got {:?}", v),
        }
    }

    #[test]
    fn se_vat_rejects_00_suffix() {
        let body = "556036079";
        let with_01 = create_se_vat(body, false).unwrap();
        let digits: String = with_01.chars().filter(|c| c.is_ascii_digit()).collect();
        let with_00 = format!("{}00", &digits[..10]);
        match verify_se_vat(&with_00) {
            Verdict::Invalid { reason } => assert!(reason.contains("00")),
            v => panic!("expected Invalid, got {:?}", v),
        }
    }

    #[test]
    fn se_vat_01_suffix_empty_comment() {
        let body = "556036079";
        let full = create_se_vat(body, false).unwrap();
        let digits: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_se_vat(&digits) {
            Verdict::Valid { comment, .. } => {
                assert!(comment.is_empty(), "expected empty comment, got {:?}", comment);
            }
            v => panic!("{:?}", v),
        }
    }
}
