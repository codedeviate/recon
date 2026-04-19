//! Finnish henkilötunnus — mod 31 with lookup table.

use super::country_id::valid_ddmmyy;
use super::Verdict;
use anyhow::{anyhow, Result};

const CHECK_TABLE: &[u8; 31] = b"0123456789ABCDEFHJKLMNPRSTUVWXY";
const CENTURY_MARKERS: &[char] = &[
    '+', '-', 'Y', 'X', 'W', 'V', 'U', 'A', 'B', 'C', 'D', 'E', 'F',
];

/// Strip only whitespace and unicode spaces; uppercase A-Z. Preserve hyphens and
/// plus signs since they are valid century markers in Finnish henkilötunnus.
fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if c.is_ascii_whitespace()
            || c == '\u{00a0}'
            || c == '\u{2009}'
            || c == '\u{202f}'
            || c == '\u{2007}'
            || c == '\u{2013}'
            || c == '\u{2014}'
        {
            continue;
        }
        if c.is_ascii_lowercase() {
            out.push(c.to_ascii_uppercase());
        } else {
            out.push(c);
        }
    }
    out
}

pub fn verify_henkilotunnus(input: &str) -> Verdict {
    let clean = normalize(input);
    if clean.len() != 11 {
        return Verdict::Invalid { reason: format!("expected 11 chars, got {}", clean.len()) };
    }
    let chars: Vec<char> = clean.chars().collect();
    if !chars[..6].iter().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "date portion must be digits".into() };
    }
    if !CENTURY_MARKERS.contains(&chars[6]) {
        return Verdict::Invalid { reason: format!("invalid century marker '{}' at position 7", chars[6]) };
    }
    if !chars[7..10].iter().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "NNN portion must be digits".into() };
    }
    let dd: u32 = clean[..2].parse().unwrap();
    let mm: u32 = clean[2..4].parse().unwrap();
    let yy: u32 = clean[4..6].parse().unwrap();
    if !valid_ddmmyy(dd, mm, yy, false) {
        return Verdict::Invalid { reason: "invalid date in henkilötunnus".into() };
    }
    let nine = format!("{}{}", &clean[..6], &clean[7..10]);
    let nine_num: u32 = match nine.parse() {
        Ok(n) => n,
        Err(_) => return Verdict::Invalid { reason: "could not parse DDMMYY+NNN as number".into() },
    };
    let idx = (nine_num % 31) as usize;
    let expected = CHECK_TABLE[idx] as char;
    if chars[10].to_ascii_uppercase() != expected {
        return Verdict::Invalid {
            reason: format!("check char mismatch: expected '{}', got '{}'", expected, chars[10]),
        };
    }
    Verdict::Valid { formatted: clean, detected: "Finnish henkilötunnus".into(), comment: String::new() }
}

pub fn create_henkilotunnus(input: &str, _raw: bool) -> Result<String> {
    let clean = normalize(input);
    if clean.len() != 10 {
        return Err(anyhow!("expected 10 chars (DDMMYYCNNN), got {}", clean.len()));
    }
    let chars: Vec<char> = clean.chars().collect();
    if !chars[..6].iter().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("date portion must be digits"));
    }
    if !CENTURY_MARKERS.contains(&chars[6]) {
        return Err(anyhow!("invalid century marker '{}' (must be +, -, Y, X, W, V, U, A, B, C, D, E, or F)", chars[6]));
    }
    if !chars[7..10].iter().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("NNN portion must be digits"));
    }
    let nine = format!("{}{}", &clean[..6], &clean[7..10]);
    let nine_num: u32 = nine.parse()?;
    let idx = (nine_num % 31) as usize;
    let check = CHECK_TABLE[idx] as char;
    Ok(format!("{}{}", clean, check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn henkilotunnus_valid_131052_308t() {
        match verify_henkilotunnus("131052-308T") {
            Verdict::Valid { detected, .. } => assert_eq!(detected, "Finnish henkilötunnus"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn henkilotunnus_invalid_check_char() {
        match verify_henkilotunnus("131052-308A") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }

    #[test]
    fn henkilotunnus_invalid_century_marker() {
        match verify_henkilotunnus("131052Z308T") {
            Verdict::Invalid { reason } => assert!(reason.contains("century")),
            _ => panic!(),
        }
    }

    #[test]
    fn henkilotunnus_invalid_date() {
        // Feb 30
        match verify_henkilotunnus("300252-308T") {
            Verdict::Invalid { reason } => assert!(reason.contains("date")),
            _ => panic!(),
        }
    }

    #[test]
    fn henkilotunnus_2023_century_marker_accepted() {
        // 2000s century 'A' (pre-2023 convention)
        let body = "010100A001";
        let full = create_henkilotunnus(body, false).unwrap();
        assert_eq!(full.len(), 11);
        match verify_henkilotunnus(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn henkilotunnus_round_trip() {
        let body = "131052-308";
        let full = create_henkilotunnus(body, false).unwrap();
        match verify_henkilotunnus(&full) {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn henkilotunnus_create_rejects_bad_length() {
        assert!(create_henkilotunnus("131052-30", false).is_err());
    }
}
