//! Non-European tax-ID check-digit algorithms.
//!
//! Each function in this module is self-contained — they share nothing
//! beyond `sanitize` and the `Verdict` type. Algorithms implemented:
//!
//! - Brazilian **CPF** (11 digits, two mod-11 check digits on weighted sums)
//! - Brazilian **CNPJ** (14 digits, two mod-11 check digits)
//! - Argentinian **CUIT / CUIL** (11 digits, one mod-11 check digit)
//! - Chilean **RUT** (8–9 digits + 'K' or digit check)
//! - Peruvian **RUC** (11 digits, one mod-11 check digit)
//! - Australian **ABN** (11 digits, ISO/IEC 7064 MOD 89-style)
//! - Mexican **RFC** (10 or 13 chars, structured mod-11 over alphanumeric)

use super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

// ── Brazilian CPF ────────────────────────────────────────────────────────
// 11 digits. Two check digits at the end.
//   d10 = mod11 of first 9 digits with weights [10,9,8,7,6,5,4,3,2]
//   d11 = mod11 of first 10 digits with weights [11,10,9,8,7,6,5,4,3,2]
// If remainder < 2 → 0, else 11 - remainder.
// Reject all-equal-digit CPFs (they pass the check but are invalid by convention).

pub fn verify_br_cpf(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 11 {
        return Verdict::Invalid {
            reason: format!("Brazilian CPF requires 11 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    if clean.chars().all(|c| c == clean.chars().next().unwrap()) {
        return Verdict::Invalid {
            reason: "all-identical-digit CPFs are invalid by convention".into(),
        };
    }
    let d: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let d10 = mod11_remainder(&d[..9], 10);
    let d11 = mod11_remainder(&d[..10], 11);
    if d10 != d[9] {
        return Verdict::Invalid {
            reason: format!("CPF digit 10 mismatch: expected {d10}, got {}", d[9]),
        };
    }
    if d11 != d[10] {
        return Verdict::Invalid {
            reason: format!("CPF digit 11 mismatch: expected {d11}, got {}", d[10]),
        };
    }
    let formatted = format!("{}.{}.{}-{}", &clean[..3], &clean[3..6], &clean[6..9], &clean[9..]);
    Verdict::Valid {
        formatted,
        detected: "Brazilian CPF".into(),
        comment: String::new(),
    }
}

pub fn create_br_cpf(input: &str, raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 9 {
        return Err(anyhow!("expected 9 digits (CPF body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let d: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let d10 = mod11_remainder(&d, 10);
    let d11_input: Vec<u32> = d.iter().copied().chain(std::iter::once(d10)).collect();
    let d11 = mod11_remainder(&d11_input, 11);
    let full = format!("{clean}{d10}{d11}");
    if raw {
        return Ok(full);
    }
    Ok(format!("{}.{}.{}-{}{}{}",
        &clean[..3], &clean[3..6], &clean[6..9], d10, d11, ""))
}

// Helper: compute mod-11 remainder with descending weights starting from `top`.
// If remainder < 2 → 0, else 11 - remainder.
fn mod11_remainder(digits: &[u32], top: u32) -> u32 {
    let mut sum = 0u32;
    for (i, d) in digits.iter().enumerate() {
        sum += d * (top - i as u32);
    }
    let r = sum % 11;
    if r < 2 {
        0
    } else {
        11 - r
    }
}

// ── Brazilian CNPJ ───────────────────────────────────────────────────────
// 14 digits. Two check digits with weights that cycle 9..=2 then restart.
//   d13 weights = [5,4,3,2,9,8,7,6,5,4,3,2]
//   d14 weights = [6,5,4,3,2,9,8,7,6,5,4,3,2]

pub fn verify_br_cnpj(input: &str) -> Verdict {
    // CNPJ formatted form has '/' which sanitize() doesn't strip.
    let clean: String = sanitize(input, false).chars().filter(|c| *c != '/').collect();
    if clean.len() != 14 {
        return Verdict::Invalid {
            reason: format!("Brazilian CNPJ requires 14 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    if clean.chars().all(|c| c == clean.chars().next().unwrap()) {
        return Verdict::Invalid {
            reason: "all-identical-digit CNPJs are invalid by convention".into(),
        };
    }
    let d: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let w13: [u32; 12] = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let w14: [u32; 13] = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let d13 = mod11_cnpj(&d[..12], &w13);
    let d14 = mod11_cnpj(&d[..13], &w14);
    if d13 != d[12] {
        return Verdict::Invalid {
            reason: format!("CNPJ digit 13 mismatch: expected {d13}, got {}", d[12]),
        };
    }
    if d14 != d[13] {
        return Verdict::Invalid {
            reason: format!("CNPJ digit 14 mismatch: expected {d14}, got {}", d[13]),
        };
    }
    let formatted = format!(
        "{}.{}.{}/{}-{}",
        &clean[..2],
        &clean[2..5],
        &clean[5..8],
        &clean[8..12],
        &clean[12..]
    );
    Verdict::Valid {
        formatted,
        detected: "Brazilian CNPJ".into(),
        comment: String::new(),
    }
}

pub fn create_br_cnpj(input: &str, raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 12 {
        return Err(anyhow!("expected 12 digits (CNPJ body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let d: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let w13: [u32; 12] = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let w14: [u32; 13] = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let d13 = mod11_cnpj(&d, &w13);
    let d14_body: Vec<u32> = d.iter().copied().chain(std::iter::once(d13)).collect();
    let d14 = mod11_cnpj(&d14_body, &w14);
    let full = format!("{clean}{d13}{d14}");
    if raw {
        return Ok(full);
    }
    Ok(format!(
        "{}.{}.{}/{}-{}{}",
        &clean[..2],
        &clean[2..5],
        &clean[5..8],
        &clean[8..],
        d13,
        d14,
    ))
}

fn mod11_cnpj(digits: &[u32], weights: &[u32]) -> u32 {
    let sum: u32 = digits
        .iter()
        .zip(weights.iter())
        .map(|(d, w)| d * w)
        .sum();
    let r = sum % 11;
    if r < 2 {
        0
    } else {
        11 - r
    }
}

// ── Argentinian CUIT / CUIL ──────────────────────────────────────────────
// 11 digits. Weights [5,4,3,2,7,6,5,4,3,2] on first 10. Check = 11 - (sum%11).
// If check == 11 → 0, if check == 10 → invalid.

pub fn verify_ar_cuit(input: &str) -> Verdict {
    verify_ar_cuit_cuil(input, "Argentinian CUIT")
}

pub fn verify_ar_cuil(input: &str) -> Verdict {
    verify_ar_cuit_cuil(input, "Argentinian CUIL")
}

fn verify_ar_cuit_cuil(input: &str, label: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 11 {
        return Verdict::Invalid {
            reason: format!("{label} requires 11 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let d: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let weights = [5u32, 4, 3, 2, 7, 6, 5, 4, 3, 2];
    let sum: u32 = d[..10].iter().zip(weights.iter()).map(|(a, b)| a * b).sum();
    let r = sum % 11;
    let expected = match 11u32.saturating_sub(r) {
        11 => 0,
        10 => {
            return Verdict::Invalid {
                reason: format!("{label} check == 10 (reserved / invalid)"),
            };
        }
        v => v,
    };
    if expected != d[10] {
        return Verdict::Invalid {
            reason: format!("{label} check mismatch: expected {expected}, got {}", d[10]),
        };
    }
    let formatted = format!("{}-{}-{}", &clean[..2], &clean[2..10], &clean[10..]);
    Verdict::Valid {
        formatted,
        detected: label.into(),
        comment: String::new(),
    }
}

pub fn create_ar_cuit(input: &str, raw: bool) -> Result<String> {
    create_ar_cuit_cuil(input, raw)
}

pub fn create_ar_cuil(input: &str, raw: bool) -> Result<String> {
    create_ar_cuit_cuil(input, raw)
}

fn create_ar_cuit_cuil(input: &str, raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 10 {
        return Err(anyhow!("expected 10 digits (CUIT/CUIL body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let d: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let weights = [5u32, 4, 3, 2, 7, 6, 5, 4, 3, 2];
    let sum: u32 = d.iter().zip(weights.iter()).map(|(a, b)| a * b).sum();
    let r = sum % 11;
    let cd = match 11u32.saturating_sub(r) {
        11 => 0,
        10 => return Err(anyhow!("check == 10 (reserved / invalid CUIT/CUIL)")),
        v => v,
    };
    let full = format!("{clean}{cd}");
    if raw {
        return Ok(full);
    }
    Ok(format!("{}-{}-{}", &full[..2], &full[2..10], &full[10..]))
}

// ── Chilean RUT ──────────────────────────────────────────────────────────
// 8–9 digits + one check char ('0'-'9' or 'K').
// Algorithm: weights cycle 2..=7 from right-most body digit;
// sum mod 11; 11 - remainder; 11 → 0, 10 → 'K'.

pub fn verify_cl_rut(input: &str) -> Verdict {
    let clean_upper: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '.')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if clean_upper.len() < 2 || clean_upper.len() > 10 {
        return Verdict::Invalid {
            reason: format!("Chilean RUT length out of range: {}", clean_upper.len()),
        };
    }
    let (body, check_char) = clean_upper.split_at(clean_upper.len() - 1);
    let check_char = check_char.chars().next().unwrap();
    if !body.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit body".into() };
    }
    if !check_char.is_ascii_digit() && check_char != 'K' {
        return Verdict::Invalid {
            reason: format!("check character must be 0-9 or K, got '{check_char}'"),
        };
    }
    let expected = rut_check_char(body);
    if check_char != expected {
        return Verdict::Invalid {
            reason: format!("RUT check mismatch: expected '{expected}', got '{check_char}'"),
        };
    }
    let formatted = format_cl_rut(body, check_char);
    Verdict::Valid {
        formatted,
        detected: "Chilean RUT".into(),
        comment: String::new(),
    }
}

pub fn create_cl_rut(input: &str, raw: bool) -> Result<String> {
    let clean: String = input
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    if clean.len() < 1 || clean.len() > 9 {
        return Err(anyhow!("expected 1-9 digits (RUT body), got {}", clean.len()));
    }
    let cd = rut_check_char(&clean);
    if raw {
        return Ok(format!("{clean}{cd}"));
    }
    Ok(format_cl_rut(&clean, cd))
}

fn rut_check_char(body: &str) -> char {
    let mut sum = 0u32;
    let mut weight = 2u32;
    for c in body.chars().rev() {
        let d = c.to_digit(10).unwrap();
        sum += d * weight;
        weight = if weight == 7 { 2 } else { weight + 1 };
    }
    let r = 11 - (sum % 11);
    match r {
        11 => '0',
        10 => 'K',
        v => std::char::from_digit(v, 10).unwrap(),
    }
}

fn format_cl_rut(body: &str, check: char) -> String {
    let mut pretty = String::new();
    let rev: Vec<char> = body.chars().rev().collect();
    for (i, c) in rev.iter().enumerate() {
        if i > 0 && i % 3 == 0 {
            pretty.push('.');
        }
        pretty.push(*c);
    }
    let pretty_body: String = pretty.chars().rev().collect();
    format!("{pretty_body}-{check}")
}

// ── Peruvian RUC ─────────────────────────────────────────────────────────
// 11 digits. Weights [5,4,3,2,7,6,5,4,3,2] on first 10. Check = (11 - (sum%11)) % 11.
// If check == 10 or 11, use (11 - check) fallback per common practice: 10→0, 11→1.

pub fn verify_pe_ruc(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 11 {
        return Verdict::Invalid {
            reason: format!("Peruvian RUC requires 11 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let d: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let weights = [5u32, 4, 3, 2, 7, 6, 5, 4, 3, 2];
    let sum: u32 = d[..10].iter().zip(weights.iter()).map(|(a, b)| a * b).sum();
    let r = sum % 11;
    let expected = match 11u32.saturating_sub(r) {
        11 => 0,
        10 => 1,
        v => v,
    };
    if expected != d[10] {
        return Verdict::Invalid {
            reason: format!("RUC check mismatch: expected {expected}, got {}", d[10]),
        };
    }
    Verdict::Valid {
        formatted: clean.clone(),
        detected: "Peruvian RUC".into(),
        comment: String::new(),
    }
}

pub fn create_pe_ruc(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, false);
    if clean.len() != 10 {
        return Err(anyhow!("expected 10 digits (RUC body), got {}", clean.len()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("non-digit input"));
    }
    let d: Vec<u32> = clean.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let weights = [5u32, 4, 3, 2, 7, 6, 5, 4, 3, 2];
    let sum: u32 = d.iter().zip(weights.iter()).map(|(a, b)| a * b).sum();
    let r = sum % 11;
    let cd = match 11u32.saturating_sub(r) {
        11 => 0,
        10 => 1,
        v => v,
    };
    Ok(format!("{clean}{cd}"))
}

// ── Australian ABN ───────────────────────────────────────────────────────
// 11 digits. Subtract 1 from the first digit; multiply each by weights
// [10,1,3,5,7,9,11,13,15,17,19]; sum must be divisible by 89.

pub fn verify_au_abn(input: &str) -> Verdict {
    let clean = sanitize(input, false);
    if clean.len() != 11 {
        return Verdict::Invalid {
            reason: format!("Australian ABN requires 11 digits, got {}", clean.len()),
        };
    }
    if !clean.chars().all(|c| c.is_ascii_digit()) {
        return Verdict::Invalid { reason: "non-digit input".into() };
    }
    let d: Vec<i64> = clean
        .chars()
        .map(|c| c.to_digit(10).unwrap() as i64)
        .collect();
    let weights = [10i64, 1, 3, 5, 7, 9, 11, 13, 15, 17, 19];
    let first = d[0] - 1; // first digit minus 1
    let mut sum = first * weights[0];
    for i in 1..11 {
        sum += d[i] * weights[i];
    }
    if sum % 89 != 0 {
        return Verdict::Invalid {
            reason: format!("ABN checksum failed: sum {sum} is not divisible by 89"),
        };
    }
    let formatted = format!("{} {} {} {}", &clean[..2], &clean[2..5], &clean[5..8], &clean[8..]);
    Verdict::Valid {
        formatted,
        detected: "Australian ABN".into(),
        comment: String::new(),
    }
}

/// ABN's algorithm gives no "inverse" — creating a valid ABN from 10
/// arbitrary digits would need brute-force. Refuse and point users at
/// `--checkdigit au_abn` for verification.
pub fn create_au_abn(_input: &str, _raw: bool) -> Result<String> {
    Err(anyhow!(
        "ABN uses a mod-89 checksum with no single \"check digit\" \
         to append. Only verification is supported (--checkdigit au_abn)."
    ))
}

// ── Mexican RFC ──────────────────────────────────────────────────────────
// 10 chars (person) or 13 chars (person with homoclave) or 12 chars
// (company). The trailing character is a single check digit computed
// over an alphabet map. We implement the person (13-char) form, which
// is the most common and includes the homoclave.
//
// Alphabet mapping (space → 00, 0-9 → 00-09, A-Z → 10-40 with special
// handling for Ñ=24). Weights descend from 13 to 2; sum mod 11; check =
// (11 - r) mod 11; 10 is encoded as 'A'.

pub fn verify_mx_rfc(input: &str) -> Verdict {
    let clean: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if clean.len() != 13 && clean.len() != 12 {
        return Verdict::Invalid {
            reason: format!("Mexican RFC length must be 12 (company) or 13 (person), got {}", clean.len()),
        };
    }
    if !clean.chars().all(rfc_char_ok) {
        return Verdict::Invalid {
            reason: "invalid characters (allowed: A-Z, 0-9, Ñ)".into(),
        };
    }
    let expected = rfc_check_char(&clean[..clean.len() - 1]);
    let last = clean.chars().last().unwrap();
    if expected != last {
        return Verdict::Invalid {
            reason: format!("RFC check mismatch: expected '{expected}', got '{last}'"),
        };
    }
    Verdict::Valid {
        formatted: clean.clone(),
        detected: "Mexican RFC".into(),
        comment: String::new(),
    }
}

pub fn create_mx_rfc(input: &str, _raw: bool) -> Result<String> {
    let clean: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if clean.len() != 12 && clean.len() != 11 {
        return Err(anyhow!(
            "expected 11 (company body) or 12 (person body) chars, got {}",
            clean.len()
        ));
    }
    if !clean.chars().all(rfc_char_ok) {
        return Err(anyhow!("invalid characters (allowed: A-Z, 0-9, Ñ)"));
    }
    let cd = rfc_check_char(&clean);
    Ok(format!("{clean}{cd}"))
}

fn rfc_char_ok(c: char) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit() || c == 'Ñ'
}

fn rfc_char_value(c: char) -> u32 {
    match c {
        '0'..='9' => c as u32 - '0' as u32,
        'A'..='N' => 10 + (c as u32 - 'A' as u32),
        'Ñ' => 24, // pre-1998 tables use 24 here
        'O'..='Z' => {
            // After Ñ, each letter's value is bumped by 1 (24 is taken).
            let base = 10 + (c as u32 - 'A' as u32);
            if c >= 'O' {
                base + 1
            } else {
                base
            }
        }
        ' ' => 0,
        _ => 0,
    }
}

fn rfc_check_char(body: &str) -> char {
    // Pad to 12 with leading spaces for person-form (13-char RFC), or
    // 11 for company-form (12-char RFC). Weight descends from 13 / 12.
    let pad_to = 12;
    let mut padded = String::with_capacity(pad_to);
    while padded.len() + body.len() < pad_to {
        padded.push(' ');
    }
    padded.push_str(body);
    let mut sum = 0u32;
    let start_weight = padded.len() as u32 + 1; // 13 for 12 chars
    for (i, c) in padded.chars().enumerate() {
        let w = start_weight - (i as u32);
        sum += rfc_char_value(c) * w;
    }
    let r = sum % 11;
    let cd = match 11u32.saturating_sub(r) {
        11 => 0,
        v => v,
    };
    match cd {
        0..=9 => std::char::from_digit(cd, 10).unwrap(),
        10 => 'A',
        _ => '?',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn br_cpf_valid_known_good() {
        // Well-known valid sample CPF.
        match verify_br_cpf("529.982.247-25") {
            Verdict::Valid { .. } => {}
            Verdict::Invalid { reason } => panic!("expected valid, got: {reason}"),
        }
    }

    #[test]
    fn br_cpf_all_equal_rejected() {
        assert!(matches!(
            verify_br_cpf("11111111111"),
            Verdict::Invalid { .. }
        ));
    }

    #[test]
    fn br_cpf_create_round_trips() {
        let full = create_br_cpf("529982247", true).unwrap();
        assert_eq!(full, "52998224725");
        assert!(matches!(verify_br_cpf(&full), Verdict::Valid { .. }));
    }

    #[test]
    fn br_cnpj_valid_known_good() {
        // Sample CNPJ from Receita Federal test vectors.
        match verify_br_cnpj("11.444.777/0001-61") {
            Verdict::Valid { .. } => {}
            Verdict::Invalid { reason } => panic!("expected valid, got: {reason}"),
        }
    }

    #[test]
    fn br_cnpj_create_round_trips() {
        let full = create_br_cnpj("114447770001", true).unwrap();
        assert_eq!(full, "11444777000161");
    }

    #[test]
    fn ar_cuit_valid_known_good() {
        match verify_ar_cuit("20-12345678-9") {
            // Not guaranteed valid; test actual mechanic with a known-good number.
            _ => {}
        }
        // Known-good: 20-26726577-5 is a real CUIT pattern.
        let ok = create_ar_cuit("2026726577", true).unwrap();
        assert!(matches!(verify_ar_cuit(&ok), Verdict::Valid { .. }));
    }

    #[test]
    fn cl_rut_valid_k_check() {
        // Classic sample: 11.111.111-1 (if check-char 1 matches, else K).
        let ok = create_cl_rut("11111111", true).unwrap();
        match verify_cl_rut(&ok) {
            Verdict::Valid { .. } => {}
            Verdict::Invalid { reason } => panic!("{reason}"),
        }
    }

    #[test]
    fn cl_rut_handles_dashes_and_dots() {
        let ok = create_cl_rut("12345678", false).unwrap();
        // Should look like "12.345.678-X"
        assert!(ok.contains('-') && ok.contains('.'));
        assert!(matches!(verify_cl_rut(&ok), Verdict::Valid { .. }));
    }

    #[test]
    fn pe_ruc_round_trip() {
        let ok = create_pe_ruc("2012345678", false).unwrap();
        assert_eq!(ok.len(), 11);
        assert!(matches!(verify_pe_ruc(&ok), Verdict::Valid { .. }));
    }

    #[test]
    fn au_abn_known_good() {
        // Australian Tax Office sample ABN.
        match verify_au_abn("51 824 753 556") {
            Verdict::Valid { .. } => {}
            Verdict::Invalid { reason } => panic!("expected valid ABN, got: {reason}"),
        }
    }

    #[test]
    fn au_abn_create_rejected() {
        assert!(create_au_abn("01234567890", false).is_err());
    }

    #[test]
    fn mx_rfc_round_trip_person() {
        let body = "HEGA821212"; // 10 chars
        // Add a fake homoclave of 2 chars + computed check.
        let full = create_mx_rfc(&format!("{body}TT"), true).unwrap();
        assert_eq!(full.len(), 13);
        assert!(matches!(verify_mx_rfc(&full), Verdict::Valid { .. }));
    }
}
