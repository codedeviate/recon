//! Lithuanian VAT.
//!
//! Auto-detects by length:
//!   - 9 digits  → personal / small business
//!   - 12 digits → branch / large entity
//!
//! ## 9-digit algorithm
//! Primary weights `[1,2,3,4,5,6,7,8,9]` on the first 8 digits.
//! `check1 = sum % 11`.
//! If `check1 != 10`, the 9th digit must equal `check1`.
//! If `check1 == 10`, apply secondary weights `[3,4,5,6,7,8,9,1,2]` on the
//! first 8 digits. `check2 = sum % 11`. If `check2 == 10`, the check digit
//! is 0; otherwise the 9th digit must equal `check2`.
//!
//! ## 12-digit algorithm
//! Same fallback logic, but primary weights `[1,2,3,4,5,6,7,8,9,1,2]` on the
//! first 11 digits, and secondary weights `[3,4,5,6,7,8,9,1,2,3,4]`.
//! The 12th digit is the check.

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

const PRIMARY_9: [u32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
const SECONDARY_9: [u32; 8] = [3, 4, 5, 6, 7, 8, 9, 1];

const PRIMARY_12: [u32; 11] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 1, 2];
const SECONDARY_12: [u32; 11] = [3, 4, 5, 6, 7, 8, 9, 1, 2, 3, 4];

fn weighted_sum(digits: &[u32], weights: &[u32]) -> u32 {
    digits.iter().zip(weights.iter()).map(|(d, w)| d * w).sum()
}

fn compute_check_9(body_digits: &[u32]) -> u32 {
    let check1 = weighted_sum(body_digits, &PRIMARY_9) % 11;
    if check1 != 10 {
        return check1;
    }
    let check2 = weighted_sum(body_digits, &SECONDARY_9) % 11;
    if check2 == 10 { 0 } else { check2 }
}

fn compute_check_12(body_digits: &[u32]) -> u32 {
    let check1 = weighted_sum(body_digits, &PRIMARY_12) % 11;
    if check1 != 10 {
        return check1;
    }
    let check2 = weighted_sum(body_digits, &SECONDARY_12) % 11;
    if check2 == 10 { 0 } else { check2 }
}

fn parse_digits(s: &str) -> Vec<u32> {
    s.chars().map(|c| c.to_digit(10).unwrap()).collect()
}

pub fn verify_lt_vat(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    let len = clean.len();
    if len != 9 && len != 12 {
        return Verdict::Invalid {
            reason: format!("expected 9 or 12 digits, got {}", len),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let all: Vec<u32> = parse_digits(&clean);
    let (body, check_pos, expected) = if len == 9 {
        let body = &all[..8];
        let expected = compute_check_9(body);
        (body.to_vec(), all[8], expected)
    } else {
        let body = &all[..11];
        let expected = compute_check_12(body);
        (body.to_vec(), all[11], expected)
    };
    let _ = body; // used above
    if expected == check_pos {
        Verdict::Valid {
            formatted: format!("LT{}", clean),
            detected: "Lithuanian VAT".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("LT VAT check mismatch: expected {}, got {}", expected, check_pos),
        }
    }
}

pub fn create_lt_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    let len = clean.len();
    if len != 8 && len != 11 {
        return Err(anyhow!(
            "expected 8 digits (9-digit body) or 11 digits (12-digit body), got {}",
            len
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let all: Vec<u32> = parse_digits(&clean);
    let check = if len == 8 {
        compute_check_9(&all)
    } else {
        compute_check_12(&all)
    };
    Ok(format!("LT{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lt_vat_valid_9digit_213179412() {
        // Known valid: 213179412
        // primary check: sum=142, 142%11=10 → secondary: sum=189, 189%11=2. Check=2. ✓
        match verify_lt_vat("213179412") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lt_vat_valid_12digit_290061371314() {
        // Known valid: 290061371314
        match verify_lt_vat("290061371314") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lt_vat_round_trip_9() {
        let body = "21317941";
        let full = create_lt_vat(body, false).unwrap();
        let raw = &full[2..]; // strip "LT"
        match verify_lt_vat(raw) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lt_vat_round_trip_12() {
        let body = "29006137131";
        let full = create_lt_vat(body, false).unwrap();
        let raw = &full[2..];
        match verify_lt_vat(raw) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lt_vat_rejects_bad_check() {
        match verify_lt_vat("213179413") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn lt_vat_rejects_wrong_length() {
        match verify_lt_vat("12345678") {
            Verdict::Invalid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
