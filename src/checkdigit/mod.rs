//! Check-digit verification and computation across 40+ identifier schemes.

pub mod brand;
pub mod country_id;
pub mod format;
pub mod iban_countries;
pub mod luhn;
pub mod mod10_ean;
pub mod mod11;
pub mod mod31;
pub mod mod97;
pub mod registry;

use anyhow::Result;

/// Outcome of a verify operation.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Valid { formatted: String, detected: String },
    Invalid { reason: String },
}

/// Input-size cap (bytes after sanitization). Reject anything larger.
pub const MAX_INPUT_LEN: usize = 1024;

/// Static specification for one CLI keyword (canonical or alias).
pub struct Spec {
    pub canonical: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub verify_fn: fn(&str) -> Verdict,
    pub create_fn: fn(&str, raw: bool) -> Result<String>,
}

/// Strip whitespace, hyphens, en-dashes, NBSP, dots. Uppercase A-Z/a-z if `upper`.
pub fn sanitize(input: &str, upper: bool) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if c.is_ascii_whitespace()
            || c == '-'
            || c == '\u{2013}'
            || c == '\u{2014}'
            || c == '\u{00a0}'
            || c == '\u{2009}'
            || c == '\u{202f}'
            || c == '\u{2007}'
            || c == '.'
        {
            continue;
        }
        if upper && c.is_ascii_lowercase() {
            out.push(c.to_ascii_uppercase());
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_whitespace_dashes_dots() {
        assert_eq!(sanitize("4111 1111-1111.1111", false), "4111111111111111");
    }

    #[test]
    fn sanitize_uppercases_when_requested() {
        assert_eq!(sanitize("se35 5000 0000", true), "SE3550000000");
    }

    #[test]
    fn sanitize_preserves_case_when_not_requested() {
        assert_eq!(sanitize("AbC 123", false), "AbC123");
    }

    #[test]
    fn sanitize_strips_unicode_spaces() {
        // Thin space, narrow NBSP, figure space all get removed.
        let input = "SE35\u{2009}5000\u{202f}0000\u{2007}0003";
        assert_eq!(sanitize(input, true), "SE35500000000003");
    }
}
