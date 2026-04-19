//! EU VAT check digits — starter set (SE, DK, FI, DE, FR).

use super::luhn::{luhn_check_digit, luhn_verify};
use super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

// ── Sweden (SE) ──────────────────────────────────────────────────────────
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
        Verdict::Valid { formatted: format!("SE{}", clean), detected: "Swedish VAT".into() }
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

// ── Denmark (DK) ─────────────────────────────────────────────────────────
// 8 digits, weights [2,7,6,5,4,3,2,1], sum mod 11 == 0.
pub fn verify_dk_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
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
        }
    } else {
        Verdict::Invalid { reason: "DK VAT mod-11 check failed".into() }
    }
}

pub fn create_dk_vat(_input: &str, _raw: bool) -> Result<String> {
    Err(anyhow!("DK VAT has no trailing check digit — the entire 8-digit number is weighted together. Use --checkdigit dk-vat to verify."))
}

// ── Finland (FI) ─────────────────────────────────────────────────────────
// 8 digits: 7-digit body + check digit. Weights [7,9,10,5,8,4,2] on first 7.
// check = (11 - sum mod 11) mod 11. If check == 10, invalid.
pub fn verify_fi_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Verdict::Invalid { reason: format!("expected 8 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let weights = [7u32, 9, 10, 5, 8, 4, 2];
    let body = &clean[..7];
    let check = clean.chars().nth(7).unwrap().to_digit(10).unwrap();
    let sum: u32 = body.chars().enumerate()
        .map(|(i, c)| weights[i] * c.to_digit(10).unwrap())
        .sum();
    let expected = (11 - (sum % 11)) % 11;
    if expected == 10 {
        return Verdict::Invalid { reason: "FI VAT: computed check is 10 — number invalid".into() };
    }
    if expected == check {
        Verdict::Valid {
            formatted: format!("FI{}", clean),
            detected: "Finnish VAT (Y-tunnus)".into(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("FI VAT check mismatch: expected {}, got {}", expected, check),
        }
    }
}

pub fn create_fi_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 7 {
        return Err(anyhow!("expected 7 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let weights = [7u32, 9, 10, 5, 8, 4, 2];
    let sum: u32 = clean.chars().enumerate()
        .map(|(i, c)| weights[i] * c.to_digit(10).unwrap())
        .sum();
    let check = (11 - (sum % 11)) % 11;
    if check == 10 {
        return Err(anyhow!("FI VAT: no valid check digit (computed 10)"));
    }
    Ok(format!("FI{}{}", clean, check))
}

// ── Germany (DE) ─────────────────────────────────────────────────────────
// 9 digits. Running-product algorithm with seed 10:
//   for each of first 8 digits: sum = (digit + carry) mod 10; if sum == 0, sum = 10;
//     carry = (2 * sum) mod 11
//   Check digit = (11 - carry) mod 10.
pub fn verify_de_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Verdict::Invalid { reason: format!("expected 9 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let mut carry: u32 = 10;
    for c in clean[..8].chars() {
        let d = c.to_digit(10).unwrap();
        let mut sum = (d + carry) % 10;
        if sum == 0 { sum = 10; }
        carry = (2 * sum) % 11;
    }
    let expected = (11 - carry) % 10;
    let check = clean.chars().nth(8).unwrap().to_digit(10).unwrap();
    if expected == check {
        Verdict::Valid {
            formatted: format!("DE{}", clean),
            detected: "German VAT (USt-IdNr)".into(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("DE VAT check mismatch: expected {}, got {}", expected, check),
        }
    }
}

pub fn create_de_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let mut carry: u32 = 10;
    for c in clean.chars() {
        let d = c.to_digit(10).unwrap();
        let mut sum = (d + carry) % 10;
        if sum == 0 { sum = 10; }
        carry = (2 * sum) % 11;
    }
    let check = (11 - carry) % 10;
    Ok(format!("DE{}{}", clean, check))
}

// ── France (FR) ──────────────────────────────────────────────────────────
// 2-char key + 9-digit SIREN. Key = (12 + 3 × (SIREN mod 97)) mod 97.
pub fn verify_fr_vat(input: &str) -> Verdict {
    let clean = sanitize(input, true);
    if clean.len() != 11 {
        return Verdict::Invalid { reason: format!("expected 11 chars (2-key + 9-SIREN), got {}", clean.len()) };
    }
    let key_part = &clean[..2];
    let siren = &clean[2..];
    if !siren.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "SIREN must be 9 digits".into() };
    }
    let siren_num: u64 = siren.parse().unwrap();
    let expected_key: u32 = ((12 + 3 * (siren_num % 97)) % 97) as u32;
    let key_num: u32 = match key_part.parse() {
        Ok(n) => n,
        Err(_) => {
            return Verdict::Valid {
                formatted: format!("FR{}", clean),
                detected: "French VAT (alphanumeric key — check skipped)".into(),
            };
        }
    };
    if expected_key == key_num {
        Verdict::Valid {
            formatted: format!("FR{}", clean),
            detected: "French VAT".into(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("FR VAT key mismatch: expected {:02}, got {}", expected_key, key_part),
        }
    }
}

pub fn create_fr_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!("expected 9 SIREN digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("SIREN must be digits"));
    }
    let siren_num: u64 = clean.parse().unwrap();
    let key: u32 = ((12 + 3 * (siren_num % 97)) % 97) as u32;
    Ok(format!("FR{:02}{}", key, clean))
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
    fn dk_vat_reference_13585628() {
        match verify_dk_vat("13585628") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn fi_vat_round_trip() {
        let body = "2077474";
        let full = create_fi_vat(body, false).unwrap();
        let clean: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_fi_vat(&clean) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn de_vat_round_trip() {
        let body = "13669597";
        let full = create_de_vat(body, false).unwrap();
        let clean: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_de_vat(&clean) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn fr_vat_round_trip() {
        let siren = "123456789";
        let full = create_fr_vat(siren, false).unwrap();
        let vat_body = full.trim_start_matches("FR").to_string();
        match verify_fr_vat(&vat_body) {
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

    #[test]
    fn dk_vat_rejects_bad_length() {
        match verify_dk_vat("1358562") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }
}
