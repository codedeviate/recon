//! Mod-11 algorithms: ISBN-10, Dutch BSN, Danish CPR, Norwegian fødselsnummer.

use super::country_id::valid_ddmmyy;
use super::format::group_variable;
use super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

// ── ISBN-10 ──────────────────────────────────────────────────────────────
// Sum(i * d_i) for i in 1..=10, mod 11 == 0. Check digit may be 'X' (value 10).

pub fn verify_isbn10(input: &str) -> Verdict {
    let clean = sanitize(input, true);
    if clean.len() != 10 {
        return Verdict::Invalid { reason: format!("expected 10 chars, got {}", clean.len()) };
    }
    let mut sum = 0u32;
    for (i, c) in clean.chars().enumerate() {
        let v = match c {
            '0'..='9' => c.to_digit(10).unwrap(),
            'X' if i == 9 => 10,
            _ => return Verdict::Invalid { reason: format!("invalid character '{}' at position {}", c, i + 1) },
        };
        sum += ((i as u32) + 1) * v;
    }
    if sum % 11 == 0 {
        let formatted = group_variable(&clean, &[1, 3, 5, 1], '-');
        Verdict::Valid { formatted, detected: "ISBN-10".into() }
    } else {
        Verdict::Invalid { reason: "ISBN-10 mod-11 check failed".into() }
    }
}

pub fn create_isbn10(input: &str, raw: bool) -> Result<String> {
    let clean = sanitize(input, true);
    if clean.len() != 9 {
        return Err(anyhow!("expected 9 chars, got {}", clean.len()));
    }
    let mut sum = 0u32;
    for (i, c) in clean.chars().enumerate() {
        let d = c.to_digit(10).ok_or_else(|| anyhow!("non-digit '{}'", c))?;
        sum += ((i as u32) + 1) * d;
    }
    // Find check digit c in 0..=10 (X=10) such that (sum + 10*c) % 11 == 0.
    let mut check: Option<u32> = None;
    for c in 0..=10u32 {
        if (sum + 10 * c) % 11 == 0 {
            check = Some(c);
            break;
        }
    }
    let check = check.ok_or_else(|| anyhow!("no valid check digit"))?;
    let check_char = if check == 10 { 'X' } else { char::from_digit(check, 10).unwrap() };
    let full = format!("{}{}", clean, check_char);
    if raw { Ok(full) } else { Ok(group_variable(&full, &[1, 3, 5, 1], '-')) }
}

// ── BSN (Netherlands) — "elfproef" ───────────────────────────────────────
// Weights [9,8,7,6,5,4,3,2,-1] applied to 9 digits; sum ≡ 0 mod 11.
// Accept 8-digit input by prepending '0'.

pub fn verify_bsn(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 8 && clean.len() != 9 {
        return Verdict::Invalid { reason: format!("expected 8 or 9 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let nine = if clean.len() == 8 { format!("0{}", clean) } else { clean.clone() };
    let weights: [i32; 9] = [9, 8, 7, 6, 5, 4, 3, 2, -1];
    let mut sum = 0i32;
    for (i, c) in nine.chars().enumerate() {
        sum += weights[i] * (c.to_digit(10).unwrap() as i32);
    }
    if sum % 11 == 0 && sum != 0 {
        Verdict::Valid { formatted: nine, detected: "Dutch BSN".into() }
    } else {
        Verdict::Invalid { reason: "BSN mod-11 check failed".into() }
    }
}

pub fn create_bsn(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 7 && clean.len() != 8 {
        return Err(anyhow!("expected 7 or 8 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let mut body = clean.clone();
    if body.len() == 7 { body = format!("0{}", body); }
    let weights: [i32; 9] = [9, 8, 7, 6, 5, 4, 3, 2, -1];
    let mut partial = 0i32;
    for (i, c) in body.chars().enumerate().take(8) {
        partial += weights[i] * (c.to_digit(10).unwrap() as i32);
    }
    for c9 in 0..=9 {
        let check_contribution = -1 * c9 as i32;
        if (partial + check_contribution).rem_euclid(11) == 0 && (partial + check_contribution) != 0 {
            return Ok(format!("{}{}", body, c9));
        }
    }
    Err(anyhow!("no valid BSN check digit exists for this body"))
}

// ── CPR (Denmark) — single weighted mod-11; NOTE post-2007 may fail ─────
// Weights [4,3,2,7,6,5,4,3,2,1] on 10 digits; sum mod 11 == 0.

pub fn verify_cpr(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 10 {
        return Verdict::Invalid { reason: format!("expected 10 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let dd: u32 = clean[..2].parse().unwrap();
    let mm: u32 = clean[2..4].parse().unwrap();
    let yy: u32 = clean[4..6].parse().unwrap();
    if !valid_ddmmyy(dd, mm, yy, false) {
        return Verdict::Invalid { reason: "invalid date in CPR".into() };
    }
    let weights = [4u32, 3, 2, 7, 6, 5, 4, 3, 2, 1];
    let mut sum = 0u32;
    for (i, c) in clean.chars().enumerate() {
        sum += weights[i] * c.to_digit(10).unwrap();
    }
    let formatted = format!("{}-{}", &clean[..6], &clean[6..]);
    if sum % 11 == 0 {
        Verdict::Valid { formatted, detected: "Danish CPR".into() }
    } else {
        Verdict::Invalid { reason: "CPR mod-11 check failed (note: post-2007 CPRs may legitimately fail)".into() }
    }
}

pub fn create_cpr(input: &str, raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!("expected 9 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let weights = [4u32, 3, 2, 7, 6, 5, 4, 3, 2, 1];
    let mut partial = 0u32;
    for (i, c) in clean.chars().enumerate() {
        partial += weights[i] * c.to_digit(10).unwrap();
    }
    for c10 in 0..=9u32 {
        if (partial + c10) % 11 == 0 {
            let full = format!("{}{}", clean, c10);
            return if raw { Ok(full) } else { Ok(format!("{}-{}", &full[..6], &full[6..])) };
        }
    }
    Err(anyhow!("no valid CPR check digit exists"))
}

// ── Norwegian fødselsnummer — two check digits K1, K2 ────────────────────

pub fn verify_fodselsnummer(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 11 {
        return Verdict::Invalid { reason: format!("expected 11 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let k1_weights = [3u32, 7, 6, 1, 8, 9, 4, 5, 2];
    let k1_sum: u32 = k1_weights.iter().zip(digits.iter().take(9)).map(|(w, d)| w * d).sum();
    let k1 = (11 - (k1_sum % 11)) % 11;
    if k1 == 10 {
        return Verdict::Invalid { reason: "K1 == 10 — fødselsnummer invalid".into() };
    }
    if k1 != digits[9] {
        return Verdict::Invalid { reason: format!("K1 mismatch: expected {}, got {}", k1, digits[9]) };
    }

    let k2_weights = [5u32, 4, 3, 2, 7, 6, 5, 4, 3, 2];
    let k2_sum: u32 = k2_weights.iter().zip(digits.iter().take(10)).map(|(w, d)| w * d).sum();
    let k2 = (11 - (k2_sum % 11)) % 11;
    if k2 == 10 {
        return Verdict::Invalid { reason: "K2 == 10 — fødselsnummer invalid".into() };
    }
    if k2 != digits[10] {
        return Verdict::Invalid { reason: format!("K2 mismatch: expected {}, got {}", k2, digits[10]) };
    }

    let formatted = format!("{} {}", &clean[..6], &clean[6..]);
    Verdict::Valid { formatted, detected: "Norwegian fødselsnummer".into() }
}

pub fn create_fodselsnummer(input: &str, raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!("expected 9 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let digits: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let k1_weights = [3u32, 7, 6, 1, 8, 9, 4, 5, 2];
    let k1_sum: u32 = k1_weights.iter().zip(digits.iter()).map(|(w, d)| w * d).sum();
    let k1 = (11 - (k1_sum % 11)) % 11;
    if k1 == 10 {
        return Err(anyhow!("K1 == 10 — no valid fødselsnummer"));
    }
    let mut with_k1 = digits.clone();
    with_k1.push(k1);
    let k2_weights = [5u32, 4, 3, 2, 7, 6, 5, 4, 3, 2];
    let k2_sum: u32 = k2_weights.iter().zip(with_k1.iter()).map(|(w, d)| w * d).sum();
    let k2 = (11 - (k2_sum % 11)) % 11;
    if k2 == 10 {
        return Err(anyhow!("K2 == 10 — no valid fødselsnummer"));
    }
    let full = format!("{}{}{}", clean, k1, k2);
    if raw { Ok(full) } else { Ok(format!("{} {}", &full[..6], &full[6..])) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isbn10_valid_0306406152() {
        match verify_isbn10("0306406152") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn isbn10_with_x_check_digit() {
        // ISBN 0-8044-2957-X is a well-known example
        match verify_isbn10("080442957X") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn isbn10_round_trip() {
        let body = "030640615";
        let full = create_isbn10(body, true).unwrap();
        assert_eq!(full.len(), 10);
        match verify_isbn10(&full) {
            Verdict::Valid { .. } => {}
            _ => panic!(),
        }
    }

    #[test]
    fn bsn_valid_111222333() {
        match verify_bsn("111222333") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn bsn_rejects_all_zeros() {
        match verify_bsn("000000000") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }

    #[test]
    fn cpr_date_valid_check_fails_gives_hint() {
        // 0101011234 has valid date; mod-11 will fail.
        match verify_cpr("0101011234") {
            Verdict::Invalid { reason } => assert!(reason.contains("mod-11") || reason.contains("post-2007")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cpr_round_trip_via_create() {
        let full = create_cpr("010101234", false).unwrap();
        let numeric: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_cpr(&numeric) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn cpr_invalid_date_rejected() {
        // Feb 30
        match verify_cpr("3002011234") {
            Verdict::Invalid { reason } => assert!(reason.contains("date")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn fodselsnummer_valid_15076500565() {
        match verify_fodselsnummer("15076500565") {
            Verdict::Valid { formatted, .. } => assert_eq!(formatted, "150765 00565"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn fodselsnummer_round_trip() {
        let body = "150765005";
        let full = create_fodselsnummer(body, true).unwrap();
        assert_eq!(full.len(), 11);
        match verify_fodselsnummer(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }
}
