//! Batch URL iteration for `--input-file`.
//!
//! File format matches wget + curl convention:
//! - One URL per line.
//! - `#` starts a comment to end-of-line.
//! - Blank lines ignored.
//! - Leading/trailing whitespace trimmed.
//! - Path `-` reads the list from stdin.

use anyhow::{Context, Result};
use std::io::BufRead;

/// Load the URL list from disk or stdin.
pub fn load_urls(path: &str) -> Result<Vec<String>> {
    let reader: Box<dyn BufRead> = if path == "-" {
        Box::new(std::io::BufReader::new(std::io::stdin()))
    } else {
        Box::new(std::io::BufReader::new(
            std::fs::File::open(path)
                .with_context(|| format!("--input-file: open {path}"))?,
        ))
    };
    let mut urls = Vec::new();
    for line in reader.lines() {
        let line = line.context("--input-file: read")?;
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if trimmed.is_empty() {
            continue;
        }
        urls.push(trimmed.to_string());
    }
    Ok(urls)
}

/// Apply a rate-limit before processing URL index `i` in a batch.
/// Rate spec: `N/s` / `N/m` / `N/h`. Returns the per-request sleep
/// duration implied by the rate.
pub fn parse_rate(spec: &str) -> Result<std::time::Duration> {
    let (num, unit) = spec
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("--rate: expected N/s, N/m, or N/h; got '{spec}'"))?;
    let n: u64 = num
        .trim()
        .parse()
        .with_context(|| format!("--rate: '{num}' is not a positive integer"))?;
    if n == 0 {
        anyhow::bail!("--rate: N must be ≥ 1");
    }
    let per = match unit.trim() {
        "s" | "sec" | "secs" | "seconds" => std::time::Duration::from_secs(1),
        "m" | "min" | "mins" | "minutes" => std::time::Duration::from_secs(60),
        "h" | "hr" | "hrs" | "hours" => std::time::Duration::from_secs(3600),
        other => anyhow::bail!("--rate: unknown time unit '{other}' (expected s, m, or h)"),
    };
    Ok(per / n as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_comments_and_blanks() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        writeln!(tmp, "# comment line").unwrap();
        writeln!(tmp, "https://a.example.com/").unwrap();
        writeln!(tmp).unwrap();
        writeln!(tmp, "  https://b.example.com/   # trailing comment").unwrap();
        writeln!(tmp, "https://c.example.com/").unwrap();
        let urls = load_urls(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(urls.len(), 3);
        assert_eq!(urls[0], "https://a.example.com/");
        assert_eq!(urls[1], "https://b.example.com/");
        assert_eq!(urls[2], "https://c.example.com/");
    }

    #[test]
    fn parse_rate_seconds() {
        let d = parse_rate("2/s").unwrap();
        assert_eq!(d, std::time::Duration::from_millis(500));
    }

    #[test]
    fn parse_rate_minutes() {
        let d = parse_rate("30/m").unwrap();
        assert_eq!(d, std::time::Duration::from_secs(2));
    }

    #[test]
    fn parse_rate_hours() {
        let d = parse_rate("60/h").unwrap();
        assert_eq!(d, std::time::Duration::from_secs(60));
    }

    #[test]
    fn parse_rate_rejects_zero_or_junk() {
        assert!(parse_rate("0/s").is_err());
        assert!(parse_rate("bogus").is_err());
        assert!(parse_rate("5/year").is_err());
    }
}
