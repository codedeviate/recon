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

use std::collections::HashMap;

/// Substitute `{{key}}` placeholders in `template` using `vars`. Any
/// placeholder whose key is not in `vars` → error that names the placeholder.
/// Single-brace text (`{` or `}` not part of a `{{…}}` pair) is preserved
/// literally.
pub fn expand_template(
    template: &str,
    vars: &HashMap<&str, String>,
) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let close = after.find("}}").ok_or_else(|| {
            format!("unterminated placeholder in template near '{}'", &rest[open..])
        })?;
        let key = &after[..close];
        match vars.get(key) {
            Some(v) => out.push_str(v),
            None => return Err(format!("unknown placeholder {{{{{key}}}}} in template")),
        }
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Expand `${VAR}` references against the process environment. `$$` is an
/// escape for a literal `$`. A bare `$` not followed by `{...}` or another
/// `$` is preserved literally.
pub fn expand_env(input: &str) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('$') => {
                chars.next();
                out.push('$');
            }
            Some('{') => {
                chars.next(); // consume '{'
                let mut name = String::new();
                let mut closed = false;
                for nc in chars.by_ref() {
                    if nc == '}' {
                        closed = true;
                        break;
                    }
                    name.push(nc);
                }
                if !closed {
                    return Err(format!("unterminated ${{…}} reference in '{input}'"));
                }
                let value = std::env::var(&name).map_err(|_| {
                    format!("config references ${{{name}}} which is not set in the environment")
                })?;
                out.push_str(&value);
            }
            _ => {
                // Bare '$' not followed by '{' or '$' — keep literal.
                out.push('$');
            }
        }
    }
    Ok(out)
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

    use std::collections::HashMap;

    #[test]
    fn expand_template_substitutes_known_placeholders() {
        let mut vars = HashMap::new();
        vars.insert("count", "10".to_string());
        vars.insert("format", "json".to_string());
        let out = expand_template(
            "https://api/x?limit={{count}}&fmt={{format}}",
            &vars,
        ).unwrap();
        assert_eq!(out, "https://api/x?limit=10&fmt=json");
    }

    #[test]
    fn expand_template_errors_on_unknown_placeholder() {
        let vars = HashMap::new();
        let err = expand_template("hello {{name}}", &vars).unwrap_err();
        assert!(err.contains("{{name}}"), "error should name the placeholder: {err}");
    }

    #[test]
    fn expand_template_preserves_literal_braces_when_not_placeholder() {
        let vars = HashMap::new();
        // Single braces should pass through untouched.
        let out = expand_template("plain {x} text", &vars).unwrap();
        assert_eq!(out, "plain {x} text");
    }

    #[test]
    fn expand_env_substitutes() {
        std::env::set_var("RECON_SAMPLE_TEST_A", "hello");
        let out = expand_env("prefix-${RECON_SAMPLE_TEST_A}-suffix").unwrap();
        assert_eq!(out, "prefix-hello-suffix");
        std::env::remove_var("RECON_SAMPLE_TEST_A");
    }

    #[test]
    fn expand_env_errors_on_unset_var() {
        std::env::remove_var("RECON_SAMPLE_DEFINITELY_UNSET");
        let err = expand_env("${RECON_SAMPLE_DEFINITELY_UNSET}").unwrap_err();
        assert!(err.contains("RECON_SAMPLE_DEFINITELY_UNSET"),
            "error should name the variable: {err}");
    }

    #[test]
    fn expand_env_escapes_double_dollar() {
        let out = expand_env("cost is $$5").unwrap();
        assert_eq!(out, "cost is $5");
    }

    #[test]
    fn expand_env_leaves_standalone_dollar_alone() {
        let out = expand_env("no vars here $").unwrap();
        assert_eq!(out, "no vars here $");
    }
}
