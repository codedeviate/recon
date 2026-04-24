//! Country-specific personal IDs that use Luhn: Swedish personnummer,
//! Canadian SIN, South African ID. Includes a date-validity helper
//! shared with Task 7's mod-11 algorithms (CPR, henkilotunnus).

use super::luhn::{luhn_check_digit, luhn_verify};
use super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

/// Validate a DDMMYY date. For `allow_samordning`, accept day+60 (61..=91).
///
/// When `full_year` is `Some(y)`, applies the real Gregorian leap-year rule for
/// February. When `None`, uses the forgiving fallback of accepting Feb 29
/// (century unknown).
pub fn valid_ddmmyy(
    dd: u32,
    mm: u32,
    _yy: u32,
    allow_samordning: bool,
    full_year: Option<u32>,
) -> bool {
    let day = if allow_samordning && dd > 60 { dd - 60 } else { dd };
    if !(1..=12).contains(&mm) || day == 0 {
        return false;
    }
    let max = match mm {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => match full_year {
            Some(y) if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
            Some(_) => 28,
            None => 29, // forgiving: century unknown
        },
        _ => 0,
    };
    day <= max
}

/// Current calendar year in local time (approximate — good enough for
/// age-based warnings; not for millisecond-accurate calendar math).
pub(crate) fn current_year() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let approx = 1970.0 + (secs as f64) / 31_556_952.0;
    approx as u32
}

// ── Swedish personnummer ─────────────────────────────────────────────────

/// Swedish personnummer. Accepts 10-digit `YYMMDD-NNNC` / `YYMMDD+NNNC`
/// or 12-digit `YYYYMMDD-NNNC` forms. Luhn is applied to the last 10 digits.
pub fn verify_personnummer(input: &str) -> Verdict {
    // Detect '+' separator (centenarian form) before stripping.
    let raw = input.trim();
    let plus_separator = raw.contains('+') && !raw.contains('-');

    let clean: String = sanitize(input, false).chars().filter(|c| *c != '+').collect();
    let len = clean.len();
    if len != 10 && len != 12 {
        return Verdict::Invalid { reason: format!("expected 10 or 12 digits, got {}", len) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }

    let last10 = &clean[clean.len() - 10..];
    // Swedish format: YYMMDD-NNNC (so last10 has yy at [0..2], mm [2..4], dd [4..6]).
    let yy: u32 = last10[..2].parse().unwrap();
    let mm: u32 = last10[2..4].parse().unwrap();
    let dd: u32 = last10[4..6].parse().unwrap();

    let full_year: Option<u32> = if len == 12 {
        clean[..4].parse::<u32>().ok()
    } else {
        // 10-digit form. Century from separator + current year.
        let current = current_year();
        let current_yy = current % 100;
        let century_base = if yy <= current_yy {
            current - current_yy
        } else {
            current - current_yy - 100
        };
        let mut y = century_base + yy;
        if plus_separator {
            y = y.saturating_sub(100);
        }
        Some(y)
    };

    if !valid_ddmmyy(dd, mm, yy, true, full_year) {
        return Verdict::Invalid { reason: "invalid date in personnummer".into() };
    }
    if !luhn_verify(last10) {
        return Verdict::Invalid { reason: "Luhn check failed".into() };
    }

    let sep = if plus_separator { '+' } else { '-' };
    let formatted = if len == 10 {
        format!("{}{}{}", &last10[..6], sep, &last10[6..])
    } else {
        format!("{}-{}", &clean[..8], &clean[8..])
    };

    let comment = match full_year {
        Some(y) => {
            let age = current_year().saturating_sub(y);
            if age >= 110 {
                format!("person \u{2265} 110 years old \u{2014} likely data entry error (born {})", y)
            } else {
                String::new()
            }
        }
        None => String::new(),
    };

    Verdict::Valid { formatted, detected: "Swedish personnummer".into(), comment }
}

pub fn create_personnummer(input: &str, raw: bool) -> Result<String> {
    let plus_separator = input.trim().contains('+') && !input.trim().contains('-');
    let clean: String = sanitize(input, false).chars().filter(|c| *c != '+').collect();
    let len = clean.len();
    if len != 9 && len != 11 {
        return Err(anyhow!("expected 9 (YYMMDDNNN) or 11 (YYYYMMDDNNN) digits, got {}", len));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    // Validate date on the YYMMDD portion at the start of the relevant window.
    let (date_start, _) = if len == 9 { (0usize, len) } else { (2usize, len) };
    let yy: u32 = clean[date_start..date_start+2].parse().unwrap();
    let mm: u32 = clean[date_start+2..date_start+4].parse().unwrap();
    let dd: u32 = clean[date_start+4..date_start+6].parse().unwrap();

    let full_year: Option<u32> = if len == 11 {
        clean[..4].parse::<u32>().ok()
    } else {
        // 9-digit form (YYMMDDNNN). Century from current year + yy.
        let current = current_year();
        let current_yy = current % 100;
        let century_base = if yy <= current_yy {
            current - current_yy
        } else {
            current - current_yy - 100
        };
        let mut y = century_base + yy;
        if plus_separator {
            y = y.saturating_sub(100);
        }
        Some(y)
    };

    if !valid_ddmmyy(dd, mm, yy, true, full_year) {
        return Err(anyhow!("invalid date in personnummer body"));
    }

    // For Luhn we use the YYMMDDNNN (last 9 chars of clean) regardless of 9/11 input.
    let body10 = &clean[clean.len() - 9..];
    let cd = luhn_check_digit(body10)?;
    let full = format!("{}{}", clean, cd);
    if raw { return Ok(full); }
    let sep = if plus_separator { '+' } else { '-' };
    if len == 9 {
        Ok(format!("{}{}{}{}", &full[..6], sep, &full[6..9], cd))
    } else {
        Ok(format!("{}-{}{}", &full[..8], &full[8..11], cd))
    }
}

// ── Canadian SIN ─────────────────────────────────────────────────────────

pub fn verify_sin(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Verdict::Invalid { reason: format!("expected 9 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    if !luhn_verify(&clean) {
        return Verdict::Invalid { reason: "Luhn check failed".into() };
    }
    let formatted = format!("{} {} {}", &clean[..3], &clean[3..6], &clean[6..]);
    Verdict::Valid { formatted, detected: "Canadian SIN".into(), comment: String::new() }
}

pub fn create_sin(input: &str, raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 8 {
        return Err(anyhow!("expected 8 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let cd = luhn_check_digit(&clean)?;
    let full = format!("{}{}", clean, cd);
    if raw {
        Ok(full)
    } else {
        Ok(format!("{} {} {}", &full[..3], &full[3..6], &full[6..]))
    }
}

// ── South African ID ─────────────────────────────────────────────────────

pub fn verify_sa_id(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 13 {
        return Verdict::Invalid { reason: format!("expected 13 digits, got {}", clean.len()) };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    if luhn_verify(&clean) {
        Verdict::Valid { formatted: clean, detected: "South African ID".into(), comment: String::new() }
    } else {
        Verdict::Invalid { reason: "Luhn check failed".into() }
    }
}

pub fn create_sa_id(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 12 {
        return Err(anyhow!("expected 12 digits, got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let cd = luhn_check_digit(&clean)?;
    Ok(format!("{}{}", clean, cd))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_valid_2026_feb_29() {
        // Forgiving fallback (None): 29 is accepted in February when century unknown.
        assert!(valid_ddmmyy(29, 2, 26, false, None));
    }

    #[test]
    fn date_invalid_feb_30() {
        assert!(!valid_ddmmyy(30, 2, 26, false, None));
    }

    #[test]
    fn date_invalid_month_13() {
        assert!(!valid_ddmmyy(1, 13, 26, false, None));
    }

    #[test]
    fn date_samordning_accepted() {
        // Day 88 = real day 28 with samordningsnummer offset
        assert!(valid_ddmmyy(88, 12, 81, true, None));
    }

    #[test]
    fn personnummer_10digit_valid() {
        match verify_personnummer("811228-9874") {
            Verdict::Valid { formatted, .. } => assert_eq!(formatted, "811228-9874"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn personnummer_12digit_valid() {
        match verify_personnummer("19811228-9874") {
            Verdict::Valid { formatted, .. } => assert_eq!(formatted, "19811228-9874"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn personnummer_plus_separator_preserved() {
        match verify_personnummer("811228+9874") {
            Verdict::Valid { formatted, .. } => assert_eq!(formatted, "811228+9874"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn personnummer_invalid_date_rejected() {
        // Feb 32 is invalid
        match verify_personnummer("813228-9874") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }

    #[test]
    fn personnummer_samordningsnummer_round_trip() {
        let body = "811288987";
        let full = create_personnummer(body, false).unwrap();
        let numeric: String = full.chars().filter(|c| c.is_ascii_digit()).collect();
        match verify_personnummer(&numeric) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn personnummer_create_raw_no_separator() {
        let full = create_personnummer("811228987", true).unwrap();
        assert_eq!(full.len(), 10);
        assert!(!full.contains('-'));
    }

    #[test]
    fn sin_valid_046454286() {
        match verify_sin("046454286") {
            Verdict::Valid { formatted, .. } => assert_eq!(formatted, "046 454 286"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn sin_round_trip() {
        let full = create_sin("04645428", false).unwrap();
        match verify_sin(&full) {
            Verdict::Valid { .. } => {}
            _ => panic!(),
        }
    }

    #[test]
    fn sa_id_valid_8001015009087() {
        match verify_sa_id("8001015009087") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "South African ID"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn sa_id_round_trip() {
        let full = create_sa_id("800101500908", false).unwrap();
        match verify_sa_id(&full) {
            Verdict::Valid { .. } => {}
            _ => panic!(),
        }
    }

    #[test]
    fn date_valid_feb_29_leap_year() {
        assert!(valid_ddmmyy(29, 2, 0, false, Some(2000)));
        assert!(valid_ddmmyy(29, 2, 4, false, Some(2004)));
    }

    #[test]
    fn date_invalid_feb_29_non_leap() {
        assert!(!valid_ddmmyy(29, 2, 1, false, Some(2001)));
        assert!(!valid_ddmmyy(29, 2, 3, false, Some(2003)));
        assert!(!valid_ddmmyy(29, 2, 100, false, Some(2100)));
    }

    #[test]
    fn date_valid_feb_29_year_unknown() {
        assert!(valid_ddmmyy(29, 2, 1, false, None));
    }

    #[test]
    fn personnummer_2001_feb_29_invalid() {
        // 010229 not a real date (2001 was not a leap year).
        use crate::checkdigit::luhn::luhn_check_digit;
        let body = "010229123";
        if let Ok(cd) = luhn_check_digit(body) {
            let full = format!("{}{}", body, cd);
            match verify_personnummer(&full) {
                Verdict::Invalid { reason } => assert!(reason.contains("date"), "reason was: {}", reason),
                v => panic!("expected Invalid (non-leap Feb 29), got {:?}", v),
            }
        }
    }

    #[test]
    fn personnummer_2000_feb_29_valid() {
        // 000229 IS a real date (2000 was a leap year).
        use crate::checkdigit::luhn::luhn_check_digit;
        let body = "000229123";
        let cd = luhn_check_digit(body).unwrap();
        let full = format!("{}{}", body, cd);
        match verify_personnummer(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn personnummer_110plus_comment() {
        // 12-digit form pins the birth year: 1915 → age ≥ 110 by 2026.
        use crate::checkdigit::luhn::luhn_check_digit;
        let body10 = "150101123";  // YYMMDDNNN for Luhn computation
        let cd = luhn_check_digit(body10).unwrap();
        let full12 = format!("19{}{}", body10, cd);  // "19" prefix makes it 1915
        match verify_personnummer(&full12) {
            Verdict::Valid { comment, .. } => {
                assert!(
                    comment.contains("110") || comment.contains("111") || comment.contains("110 years"),
                    "expected 110+ comment, got {:?}",
                    comment
                );
                assert!(comment.contains("1915"), "expected born year in comment, got {:?}", comment);
            }
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn personnummer_young_no_comment() {
        // A young personnummer should have an empty comment.
        use crate::checkdigit::luhn::luhn_check_digit;
        let body = "950101123";
        let cd = luhn_check_digit(body).unwrap();
        let full = format!("{}{}", body, cd);
        match verify_personnummer(&full) {
            Verdict::Valid { comment, .. } => {
                assert!(comment.is_empty(), "expected empty comment, got {:?}", comment);
            }
            v => panic!("{:?}", v),
        }
    }
}
