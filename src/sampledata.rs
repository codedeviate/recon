//! Fetch canned ecommerce sample data from known APIs, plus a local
//! lorem ipsum generator. Config-overridable.

/// Unit suffix on `--sample-count`, only meaningful for the local `lorem`
/// sample. Non-lorem samples reject a non-`None` unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountUnit {
    /// Paragraphs.
    P,
    /// Words.
    W,
    /// Characters.
    C,
}

/// Parsed `--sample-count` value: a non-negative integer plus an optional
/// single-letter unit suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountSpec {
    pub n: u32,
    pub unit: Option<CountUnit>,
}

/// Parse a `--sample-count` string into a `CountSpec`. Accepts `\d+` or
/// `\d+[pwc]`. Rejects everything else with an error that lists the grammar.
pub fn parse_count(input: &str) -> Result<CountSpec, String> {
    if input.is_empty() {
        return Err(format!(
            "invalid --sample-count '{input}': expected N or N{{p|w|c}}"
        ));
    }

    let bytes = input.as_bytes();
    let last = *bytes.last().unwrap();
    let (digits, unit) = if last.is_ascii_digit() {
        (input, None)
    } else {
        let unit = match last {
            b'p' => CountUnit::P,
            b'w' => CountUnit::W,
            b'c' => CountUnit::C,
            _ => {
                return Err(format!(
                    "invalid --sample-count '{input}': expected N or N{{p|w|c}}"
                ));
            }
        };
        (&input[..input.len() - 1], Some(unit))
    };

    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!(
            "invalid --sample-count '{input}': expected N or N{{p|w|c}}"
        ));
    }

    let n: u32 = digits
        .parse()
        .map_err(|_| format!("invalid --sample-count '{input}': number out of range"))?;

    Ok(CountSpec { n, unit })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_count_plain_number() {
        assert_eq!(parse_count("10"), Ok(CountSpec { n: 10, unit: None }));
        assert_eq!(parse_count("0"), Ok(CountSpec { n: 0, unit: None }));
        assert_eq!(parse_count("1000000"), Ok(CountSpec { n: 1_000_000, unit: None }));
    }

    #[test]
    fn parse_count_with_unit_suffix() {
        assert_eq!(parse_count("2p"), Ok(CountSpec { n: 2, unit: Some(CountUnit::P) }));
        assert_eq!(parse_count("50w"), Ok(CountSpec { n: 50, unit: Some(CountUnit::W) }));
        assert_eq!(parse_count("1000c"), Ok(CountSpec { n: 1000, unit: Some(CountUnit::C) }));
    }

    #[test]
    fn parse_count_rejects_invalid() {
        assert!(parse_count("").is_err());
        assert!(parse_count("abc").is_err());
        assert!(parse_count("10x").is_err());
        assert!(parse_count("p10").is_err());
        assert!(parse_count("50ww").is_err());
        assert!(parse_count("5.0").is_err());
        assert!(parse_count("-3").is_err());
        assert!(parse_count("p").is_err());
    }

    #[test]
    fn parse_count_error_message() {
        let err = parse_count("10x").unwrap_err();
        assert!(err.contains("10x"), "error should echo input, got: {err}");
        assert!(err.contains("N{p|w|c}") || err.contains("p|w|c"),
            "error should describe grammar, got: {err}");
    }
}
