//! Turkish VAT / VKN (Vergi Kimlik Numarası).
//!
//! 10 digits. The first 9 digits encode the taxpayer; the 10th is a check digit.
//!
//! Algorithm (from python-stdnum tr.vkn — confirmed against Gelir İdaresi Başkanlığı):
//! ```text
//! s = 0
//! for i, n in enumerate(reversed(first_9_digits), start=1):
//!     c1 = (n + i) % 10
//!     if c1 != 0:
//!         c2 = (c1 * 2^i) % 9    (or 9 if result is 0)
//!         s += c2
//! check = (10 - s) % 10
//! ```
//! The reversal means position i=1 is the 9th digit (rightmost of the first 9).
//!
//! Sources:
//! - python-stdnum tr.vkn (exact `calc_check_digit` confirmed)
//! - Gelir İdaresi Başkanlığı (Turkish Revenue Administration) VKN specification
//!
//! Hand-verified: `0010213576`
//! reversed first 9 = [7,5,3,1,2,0,1,0,0]
//! i=1: n=7 → c1=8, c2=(8×2)%9=7
//! i=2: n=5 → c1=7, c2=(7×4)%9=1
//! i=3: n=3 → c1=6, c2=(6×8)%9=3
//! i=4: n=1 → c1=5, c2=(5×16)%9=8
//! i=5: n=2 → c1=7, c2=(7×32)%9=8
//! i=6: n=0 → c1=6, c2=(6×64)%9=6
//! i=7: n=1 → c1=8, c2=(8×128)%9=7
//! i=8: n=0 → c1=8, c2=(8×256)%9=5
//! i=9: n=0 → c1=9, c2=(9×512)%9=9 (9×512=4608, 4608%9=0→9)
//! s=54, check=(10−54)%10=6, digit[9]=6 ✓

use super::super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

fn calc_check(digits: &[u32; 9]) -> u32 {
    let mut s: u32 = 0;
    for (i, n) in (1u32..).zip(digits.iter().rev()) {
        let c1 = (n + i) % 10;
        if c1 != 0 {
            // 2^i can grow large; we only care about (c1 * 2^i) % 9
            // Use modular exponentiation: 2^i % 9 cycles with period 6 (2,4,8,7,5,1,…)
            let pow2 = pow2_mod9(i);
            let c2_raw = (c1 * pow2) % 9;
            let c2 = if c2_raw == 0 { 9 } else { c2_raw };
            s += c2;
        }
    }
    (10 - s % 10) % 10
}

/// Compute `(2^exp) % 9`. The sequence cycles with period 6.
#[inline]
fn pow2_mod9(exp: u32) -> u32 {
    // 2^1=2, 2^2=4, 2^3=8, 2^4=7, 2^5=5, 2^6=1, 2^7=2, …
    match exp % 6 {
        1 => 2,
        2 => 4,
        3 => 8,
        4 => 7,
        5 => 5,
        0 => 1, // exp is multiple of 6
        _ => unreachable!(),
    }
}

fn verify_tr_vat_body(clean: &str) -> Verdict {
    debug_assert_eq!(clean.len(), 10);
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let body: [u32; 9] = [
        digits[0], digits[1], digits[2], digits[3], digits[4],
        digits[5], digits[6], digits[7], digits[8],
    ];
    let check = calc_check(&body);
    if check == digits[9] {
        Verdict::Valid {
            formatted: format!("TR{}", clean),
            detected: "Turkish VAT / VKN (10 digits, position-specific algorithm)".into(),
            comment: String::new(),
        }
    } else {
        Verdict::Invalid {
            reason: format!("TR VKN check mismatch: expected {}, got {}", check, digits[9]),
        }
    }
}

pub fn verify_tr_vat(input: &str) -> Verdict {
    let clean = match super::strip_vat_prefix(input, "TR") {
        Ok(body) => body,
        Err(v) => return v,
    };
    if clean.len() != 10 {
        return Verdict::Invalid {
            reason: format!("expected 10 digits, got {}", clean.len()),
        };
    }
    verify_tr_vat_body(&clean)
}

pub fn create_tr_vat(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!("expected 9 digits (body without check digit), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let body: [u32; 9] = [
        digits[0], digits[1], digits[2], digits[3], digits[4],
        digits[5], digits[6], digits[7], digits[8],
    ];
    let check = calc_check(&body);
    Ok(format!("TR{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-verified: 0010213576
    /// reversed first 9 = [7,5,3,1,2,0,1,0,0]
    /// s=54, check=(10-54%10)%10=6, digit[9]=6 ✓
    #[test]
    fn tr_vat_reference_0010213576() {
        match verify_tr_vat("0010213576") {
            Verdict::Valid { formatted, detected, .. } => {
                assert_eq!(formatted, "TR0010213576");
                assert!(detected.contains("VKN"));
            }
            v => panic!("{:?}", v),
        }
    }

    /// Second vector from python-stdnum doctest: 0080463579
    #[test]
    fn tr_vat_reference_0080463579() {
        match verify_tr_vat("0080463579") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    /// Third vector: 9990112519
    #[test]
    fn tr_vat_reference_9990112519() {
        match verify_tr_vat("9990112519") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    /// Mid-range vector: 5200337887
    #[test]
    fn tr_vat_reference_5200337887() {
        match verify_tr_vat("5200337887") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn tr_vat_accepts_tr_prefix() {
        match verify_tr_vat("TR0010213576") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn tr_vat_accepts_lowercase_prefix() {
        match verify_tr_vat("tr0010213576") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn tr_vat_rejects_bad_check() {
        match verify_tr_vat("0010213570") {
            Verdict::Invalid { reason } => assert!(reason.contains("check mismatch")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn tr_vat_rejects_bad_length() {
        match verify_tr_vat("001021357") {
            Verdict::Invalid { reason } => assert!(reason.contains("expected 10")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn tr_vat_round_trip() {
        let body = "001021357";
        let full = create_tr_vat(body, false).unwrap();
        assert_eq!(full, "TR0010213576");
        match verify_tr_vat(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
