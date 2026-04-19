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
    if !clean.ends_with("01") {
        return Verdict::Invalid { reason: "SE VAT must end with '01'".into() };
    }
    let orgnr = &clean[..10];
    if luhn_verify(orgnr) {
        Verdict::Valid { formatted: format!("SE{}", clean), detected: "Swedish VAT".into(), comment: String::new() }
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
    fn se_vat_rejects_wrong_suffix() {
        match verify_se_vat("556036079302") {  // ends with 02, not 01
            Verdict::Invalid { reason } => assert!(reason.contains("'01'")),
            v => panic!("{:?}", v),
        }
    }
}
