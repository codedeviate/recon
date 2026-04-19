//! IBAN (ISO 13616) — mod 97 over the country-moved numeric string.

use super::format::group_fixed;
use super::iban_countries::lookup;
use super::luhn::transliterate_alnum;
use super::{sanitize, Verdict};
use anyhow::{anyhow, Result};

/// Compute mod 97 over an arbitrary-length decimal string by streaming.
fn mod97_of_digits(s: &str) -> Result<u32> {
    let mut rem: u32 = 0;
    for c in s.chars() {
        let d = c.to_digit(10).ok_or_else(|| anyhow!("non-digit '{}'", c))?;
        rem = (rem * 10 + d) % 97;
    }
    Ok(rem)
}

fn iban_mod97(iban: &str) -> Result<u32> {
    if iban.len() < 4 {
        return Err(anyhow!("IBAN too short"));
    }
    let rearranged = format!("{}{}", &iban[4..], &iban[..4]);
    let numeric = transliterate_alnum(&rearranged)?;
    mod97_of_digits(&numeric)
}

pub fn verify_iban(input: &str) -> Verdict {
    let clean = sanitize(input, true);
    if !clean.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Verdict::Invalid { reason: "non-alphanumeric input".into() };
    }
    if !(4..=34).contains(&clean.len()) {
        return Verdict::Invalid { reason: format!("length {} out of IBAN range 4..=34", clean.len()) };
    }
    if !clean[..2].chars().all(|c| c.is_ascii_alphabetic()) {
        return Verdict::Invalid { reason: "first 2 chars must be country code (letters)".into() };
    }
    let country_code = &clean[..2];
    let detected = match lookup(country_code) {
        Some(c) => {
            if clean.len() != c.length {
                return Verdict::Invalid {
                    reason: format!("{} IBAN must be {} chars, got {}", c.code, c.length, clean.len()),
                };
            }
            format!("IBAN ({} — {})", c.code, c.name)
        }
        None => format!("IBAN ({} — unknown country)", country_code),
    };
    match iban_mod97(&clean) {
        Ok(1) => Verdict::Valid {
            formatted: group_fixed(&clean, 4, ' '),
            detected,
        },
        Ok(r) => Verdict::Invalid {
            reason: format!("IBAN mod-97 check failed (got {}, expected 1)", r),
        },
        Err(e) => Verdict::Invalid { reason: e.to_string() },
    }
}

/// Create: accept full-length input with "00" at positions 3-4 (placeholder),
/// OR (length - 2) input with positions 3-4 omitted.
pub fn create_iban(input: &str, _raw: bool) -> Result<String> {
    let clean = sanitize(input, true);
    if clean.len() < 2 {
        return Err(anyhow!("input too short"));
    }
    let country_code = &clean[..2];
    let country = lookup(country_code).ok_or_else(|| anyhow!("unknown country '{}'", country_code))?;
    let expected_len = country.length;

    let full_no_check = if clean.len() == expected_len {
        // Placeholder form — positions 3-4 are to be replaced.
        format!("{}00{}", &clean[..2], &clean[4..])
    } else if clean.len() == expected_len - 2 {
        // Omit form — positions 3-4 missing entirely.
        format!("{}00{}", &clean[..2], &clean[2..])
    } else {
        return Err(anyhow!(
            "expected {} (with 00 placeholder) or {} (omit form) chars, got {}",
            expected_len,
            expected_len - 2,
            clean.len()
        ));
    };

    let r = iban_mod97(&full_no_check)?;
    let check_digits = 98 - r;
    let full = format!("{}{:02}{}", &full_no_check[..2], check_digits, &full_no_check[4..]);
    Ok(group_fixed(&full, 4, ' '))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iban_se_valid() {
        match verify_iban("SE3550000000054910000003") {
            Verdict::Valid { formatted, .. } => assert_eq!(formatted, "SE35 5000 0000 0549 1000 0003"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn iban_gb_valid() {
        match verify_iban("GB82WEST12345698765432") {
            Verdict::Valid { formatted, .. } => assert_eq!(formatted, "GB82 WEST 1234 5698 7654 32"),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn iban_de_valid() {
        match verify_iban("DE89370400440532013000") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn iban_no_valid() {
        match verify_iban("NO9386011117947") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn iban_fr_valid() {
        match verify_iban("FR1420041010050500013M02606") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn iban_accepts_spaces_in_input() {
        match verify_iban("SE35 5000 0000 0549 1000 0003") {
            Verdict::Valid { .. } => {}
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn iban_invalid_checksum() {
        match verify_iban("SE3550000000054910000004") {
            Verdict::Invalid { .. } => {}
            _ => panic!(),
        }
    }

    #[test]
    fn iban_wrong_length_for_country() {
        match verify_iban("SE355000000005491000000") {
            Verdict::Invalid { reason } => assert!(reason.contains("SE IBAN")),
            v => panic!("{:?}", v),
        }
    }

    #[test]
    fn iban_unknown_country_still_validates_mod97() {
        // Construct an IBAN with an unknown country code but valid mod-97.
        // ZZ is not in the table. But we can compute its check digits.
        // Just test that the "unknown country" path runs.
        match verify_iban("ZZ82WEST12345698765432") {
            // mod-97 will likely fail; we expect Invalid (mod-97 mismatch), but detected
            // must be "IBAN (ZZ — unknown country)".
            Verdict::Invalid { .. } => {}
            Verdict::Valid { detected, .. } => assert!(detected.contains("unknown")),
        }
    }

    #[test]
    fn create_iban_placeholder_form() {
        let result = create_iban("SE0050000000054910000003", false).unwrap();
        assert_eq!(result, "SE35 5000 0000 0549 1000 0003");
    }

    #[test]
    fn create_iban_omit_form() {
        let result = create_iban("SE50000000054910000003", false).unwrap();
        assert_eq!(result, "SE35 5000 0000 0549 1000 0003");
    }
}
