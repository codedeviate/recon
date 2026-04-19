//! Check-digit verification and computation across 40+ identifier schemes.

pub mod aba;
pub mod base58check;
pub mod bech32_mod;
pub mod brand;
pub mod country_id;
pub mod eip55;
pub mod format;
pub mod iban_countries;
pub mod luhn;
pub mod mod10_ean;
pub mod mod11;
pub mod mod31;
pub mod mod97;
pub mod mrz;
pub mod registry;
pub mod vat;
pub mod vin;

use anyhow::Result;

/// Outcome of a verify operation.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Valid { formatted: String, detected: String, comment: String },
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

use crate::cli::Args;
use crate::source;

/// Read check-digit input from args.
///
/// If the positional argument looks like a literal identifier value (not a
/// file path, not a URL, and not "-"), use it directly as the input string.
/// Otherwise delegate to the normal source layer (file, file://, http://, stdin).
fn read_checkdigit_input(args: &Args) -> Result<String> {
    let positional = args.target_url();

    if !positional.is_empty() && positional != "-" {
        let lower = positional.to_ascii_lowercase();
        let is_url = lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower.starts_with("file://");
        let is_path = positional.starts_with('/')
            || positional.starts_with("..");
        if !is_url && !is_path && !std::path::Path::new(positional).exists() {
            // Treat it as a literal value.
            return Ok(positional.to_string());
        }
    }

    // Fall back to source layer (stdin pipe, file, URL, file://).
    let bytes = source::read_all(args)?;
    std::str::from_utf8(&bytes)
        .map(|s| s.to_string())
        .map_err(|_| anyhow::anyhow!("input is not valid UTF-8"))
}

pub fn print_list() {
    println!("\nCheck-digit algorithms supported by recon:\n");
    for spec in registry::SPECS {
        let aliases = if spec.aliases.is_empty() {
            String::new()
        } else {
            format!(" ({})", spec.aliases.join(", "))
        };
        println!("  {:18}{:20}  {}", spec.canonical, aliases, spec.description);
    }
    println!();
}

pub fn run_verify(name: &str, args: &Args) -> Result<()> {
    let spec = match registry::resolve_with_suggestion(name) {
        Ok(s) => s,
        Err(Some(new)) => {
            return Err(anyhow::anyhow!(
                "unknown algorithm '{}' — did you mean '{}'?",
                name, new
            ));
        }
        Err(None) => {
            return Err(anyhow::anyhow!(
                "unknown algorithm '{}' (use --checkdigit-list)",
                name
            ));
        }
    };
    let input = read_checkdigit_input(args)?;
    match (spec.verify_fn)(&input) {
        Verdict::Valid { formatted, detected, .. } => {
            let out = if args.raw {
                formatted.chars().filter(|c| !c.is_whitespace() && *c != '-').collect::<String>()
            } else {
                formatted
            };
            println!("{}|{}|valid", out, detected);
            Ok(())
        }
        Verdict::Invalid { reason } => Err(anyhow::anyhow!("{}", reason)),
    }
}

pub fn run_create(name: &str, args: &Args) -> Result<()> {
    let spec = match registry::resolve_with_suggestion(name) {
        Ok(s) => s,
        Err(Some(new)) => {
            return Err(anyhow::anyhow!(
                "unknown algorithm '{}' — did you mean '{}'?",
                name, new
            ));
        }
        Err(None) => {
            return Err(anyhow::anyhow!(
                "unknown algorithm '{}' (use --checkdigit-list)",
                name
            ));
        }
    };
    let input = read_checkdigit_input(args)?;
    let out = (spec.create_fn)(input.trim(), args.raw)?;
    println!("{}", out);
    Ok(())
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
