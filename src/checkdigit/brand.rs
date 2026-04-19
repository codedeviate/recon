//! Credit card brand detection (IIN-based) + IMEI.
//! All are Luhn-based; what differs is length validation and output format.

use super::format::{group_fixed, group_variable};
use super::luhn::{luhn_check_digit, luhn_verify};
use super::{sanitize, Verdict, MAX_INPUT_LEN};
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Brand {
    Visa,
    Mastercard,
    Amex,
    Discover,
    Jcb,
    Unknown,
}

impl Brand {
    /// Detect brand from IIN (first digits). Digits must already be sanitized/clean.
    pub fn detect(digits: &str) -> Brand {
        if digits.is_empty() {
            return Brand::Unknown;
        }
        // Helper: parse first N chars as u32
        let prefix_n = |n: usize| -> Option<u32> {
            digits.get(..n).and_then(|s| s.parse::<u32>().ok())
        };
        let two = prefix_n(2).unwrap_or(0);
        let three = prefix_n(3).unwrap_or(0);
        let four = prefix_n(4).unwrap_or(0);
        let six = prefix_n(6).unwrap_or(0);

        // Visa: IIN 4
        if digits.starts_with('4') {
            return Brand::Visa;
        }
        // MasterCard: 51-55 or 2221-2720
        if (51..=55).contains(&two) || (2221..=2720).contains(&four) {
            return Brand::Mastercard;
        }
        // Amex: 34 or 37
        if two == 34 || two == 37 {
            return Brand::Amex;
        }
        // Discover: 6011, 65, 644-649, 622126-622925
        if digits.starts_with("6011")
            || two == 65
            || (644..=649).contains(&three)
            || (622126..=622925).contains(&six)
        {
            return Brand::Discover;
        }
        // JCB: 3528-3589
        if (3528..=3589).contains(&four) {
            return Brand::Jcb;
        }
        Brand::Unknown
    }

    pub fn name(&self) -> &'static str {
        match self {
            Brand::Visa => "Visa",
            Brand::Mastercard => "MasterCard",
            Brand::Amex => "American Express",
            Brand::Discover => "Discover",
            Brand::Jcb => "JCB",
            Brand::Unknown => "Unknown brand",
        }
    }

    pub fn valid_lengths(&self) -> &'static [usize] {
        match self {
            Brand::Visa => &[13, 16, 19],
            Brand::Mastercard | Brand::Discover | Brand::Jcb => &[16],
            Brand::Amex => &[15],
            Brand::Unknown => &[],
        }
    }

    pub fn format(&self, digits: &str) -> String {
        match self {
            Brand::Amex => group_variable(digits, &[4, 6, 5], ' '),
            _ => group_fixed(digits, 4, ' '),
        }
    }
}

fn sanitize_digits_or_error(input: &str) -> Result<String, Verdict> {
    let clean = sanitize(input, false);
    if clean.is_empty() {
        return Err(Verdict::Invalid { reason: "empty input".into() });
    }
    if clean.len() > MAX_INPUT_LEN {
        return Err(Verdict::Invalid { reason: "input too long".into() });
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(Verdict::Invalid { reason: "non-digit input".into() });
    }
    Ok(clean)
}

/// Generic creditcard: auto-detect brand then validate.
pub fn verify_creditcard(input: &str) -> Verdict {
    let clean = match sanitize_digits_or_error(input) {
        Ok(c) => c,
        Err(v) => return v,
    };
    let brand = Brand::detect(&clean);
    if brand == Brand::Unknown {
        return Verdict::Invalid {
            reason: "unrecognized card brand (IIN does not match any known issuer)".into(),
        };
    }
    if !brand.valid_lengths().contains(&clean.len()) {
        return Verdict::Invalid {
            reason: format!(
                "{} must be {:?} digits, got {}",
                brand.name(),
                brand.valid_lengths(),
                clean.len()
            ),
        };
    }
    if !luhn_verify(&clean) {
        return Verdict::Invalid { reason: "Luhn check failed".into() };
    }
    Verdict::Valid { formatted: brand.format(&clean), detected: brand.name().into() }
}

pub fn create_creditcard(input: &str, raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.is_empty() {
        return Err(anyhow!("empty input"));
    }
    if clean.len() > MAX_INPUT_LEN {
        return Err(anyhow!("input too long"));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let brand = Brand::detect(&clean);
    if brand == Brand::Unknown {
        return Err(anyhow!(
            "unrecognized card brand for prefix '{}'",
            clean.get(..4).unwrap_or(&clean)
        ));
    }
    // body length must be (target - 1); use the smallest valid length to resolve
    let target_len = brand
        .valid_lengths()
        .iter()
        .copied()
        .find(|&n| n == clean.len() + 1)
        .ok_or_else(|| {
            anyhow!(
                "body length {} doesn't produce a valid {} length ({:?})",
                clean.len(),
                brand.name(),
                brand.valid_lengths()
            )
        })?;
    let _ = target_len; // just length-validated
    let cd = luhn_check_digit(&clean)?;
    let full = format!("{}{}", clean, cd);
    if raw { Ok(full) } else { Ok(brand.format(&full)) }
}

/// Brand-specific verify: IIN must match the requested brand.
pub fn verify_brand(input: &str, brand: Brand) -> Verdict {
    let clean = match sanitize_digits_or_error(input) {
        Ok(c) => c,
        Err(v) => return v,
    };
    let detected = Brand::detect(&clean);
    if detected != brand {
        let reason = if detected == Brand::Unknown {
            format!("IIN does not match {} (prefix unrecognized)", brand.name())
        } else {
            format!("input IIN matches {}, not requested {}", detected.name(), brand.name())
        };
        return Verdict::Invalid { reason };
    }
    if !brand.valid_lengths().contains(&clean.len()) {
        return Verdict::Invalid {
            reason: format!(
                "{} must be {:?} digits, got {}",
                brand.name(),
                brand.valid_lengths(),
                clean.len()
            ),
        };
    }
    if !luhn_verify(&clean) {
        return Verdict::Invalid { reason: "Luhn check failed".into() };
    }
    Verdict::Valid { formatted: brand.format(&clean), detected: brand.name().into() }
}

pub fn create_brand(input: &str, brand: Brand, raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.is_empty() {
        return Err(anyhow!("empty input"));
    }
    if clean.len() > MAX_INPUT_LEN {
        return Err(anyhow!("input too long"));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    if !brand.valid_lengths().iter().any(|&n| n == clean.len() + 1) {
        return Err(anyhow!(
            "body length {} doesn't produce a valid {} length ({:?})",
            clean.len(),
            brand.name(),
            brand.valid_lengths()
        ));
    }
    let cd = luhn_check_digit(&clean)?;
    let full = format!("{}{}", clean, cd);
    if raw { Ok(full) } else { Ok(brand.format(&full)) }
}

/// IMEI: 15 digits, Luhn, format XX-XXXXXX-XXXXXX-X.
pub fn verify_imei(input: &str) -> Verdict {
    let clean = match sanitize_digits_or_error(input) {
        Ok(c) => c,
        Err(v) => return v,
    };
    if clean.len() != 15 {
        return Verdict::Invalid {
            reason: format!("IMEI must be 15 digits, got {}", clean.len()),
        };
    }
    if !luhn_verify(&clean) {
        return Verdict::Invalid { reason: "IMEI Luhn check failed".into() };
    }
    let formatted = group_variable(&clean, &[2, 6, 6, 1], '-');
    Verdict::Valid { formatted, detected: "IMEI".into() }
}

pub fn create_imei(input: &str, raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.is_empty() {
        return Err(anyhow!("empty input"));
    }
    if clean.len() != 14 {
        return Err(anyhow!("IMEI body must be 14 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let cd = luhn_check_digit(&clean)?;
    let full = format!("{}{}", clean, cd);
    if raw { Ok(full) } else { Ok(group_variable(&full, &[2, 6, 6, 1], '-')) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visa_test_card_is_visa() {
        assert_eq!(Brand::detect("4111111111111111"), Brand::Visa);
    }

    #[test]
    fn mastercard_test_card_detected() {
        assert_eq!(Brand::detect("5105105105105100"), Brand::Mastercard);
    }

    #[test]
    fn amex_test_card_detected() {
        assert_eq!(Brand::detect("378282246310005"), Brand::Amex);
    }

    #[test]
    fn discover_test_card_detected() {
        assert_eq!(Brand::detect("6011111111111117"), Brand::Discover);
    }

    #[test]
    fn jcb_test_card_detected() {
        assert_eq!(Brand::detect("3530111333300000"), Brand::Jcb);
    }

    #[test]
    fn verify_visa_ok() {
        match verify_brand("4111 1111 1111 1111", Brand::Visa) {
            Verdict::Valid { formatted, detected } => {
                assert_eq!(formatted, "4111 1111 1111 1111");
                assert_eq!(detected, "Visa");
            }
            v => panic!("expected Valid, got {:?}", v),
        }
    }

    #[test]
    fn verify_amex_formats_4_6_5() {
        match verify_brand("378282246310005", Brand::Amex) {
            Verdict::Valid { formatted, .. } => assert_eq!(formatted, "3782 822463 10005"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn verify_amex_rejects_visa_input() {
        match verify_brand("4111111111111111", Brand::Amex) {
            Verdict::Invalid { .. } => {}
            v => panic!("expected Invalid, got {:?}", v),
        }
    }

    #[test]
    fn create_visa_from_15_digits() {
        let result = create_brand("411111111111111", Brand::Visa, false).unwrap();
        assert_eq!(result, "4111 1111 1111 1111");
    }

    #[test]
    fn create_visa_raw_strips_spaces() {
        let result = create_brand("411111111111111", Brand::Visa, true).unwrap();
        assert_eq!(result, "4111111111111111");
    }

    #[test]
    fn imei_valid() {
        match verify_imei("490154203237518") {
            Verdict::Valid { formatted, .. } => assert_eq!(formatted, "49-015420-323751-8"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn imei_rejects_14_digits() {
        match verify_imei("49015420323751") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }

    #[test]
    fn imei_round_trip() {
        let body = "49015420323751";
        let full = create_imei(body, true).unwrap();
        match verify_imei(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn creditcard_auto_detect_visa() {
        match verify_creditcard("4111111111111111") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Visa"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn creditcard_auto_detect_amex() {
        match verify_creditcard("378282246310005") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "American Express"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn creditcard_unknown_brand_rejected() {
        match verify_creditcard("9999999999999999") {
            Verdict::Invalid { reason } => assert!(reason.contains("unrecognized card brand")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn verify_brand_rejects_unknown_iin_for_named_brand() {
        // 9999... has no known IIN. Requesting Visa validation should reject at brand step.
        match verify_brand("9999999999999991", Brand::Visa) {
            Verdict::Invalid { reason } => assert!(
                reason.contains("IIN does not match Visa"),
                "unexpected reason: {}", reason
            ),
            v => panic!("expected Invalid, got {:?}", v),
        }
    }
}
