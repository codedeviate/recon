//! `--proto` / `--proto-default` / `--proto-redir` parser + matcher.
//!
//! Curl's grammar:
//! - `=proto` → reset allow-list to exactly this (repeatable, comma-joined).
//! - `+proto` → add to the default allow-list (`+` optional).
//! - `-proto` → deny.
//! - Multiple operators comma-separated: `=https,+file,-ftp`.
//! - Special value `all` matches every protocol recon knows.
//!
//! Default allow-list: `http, https, ftp, ftps, file, sftp, scp, ssh,
//! telnet, gopher, gophers, pop3, pop3s, imap, imaps, smtp, smtps,
//! ldap, ldaps, ws, wss, tftp, dict, rtsp, rtsps, mqtt, mqtts,
//! redis, memcached, ntp, ipfs, ipns`.

use anyhow::Result;
use std::collections::HashSet;

const DEFAULT_ALLOWED: &[&str] = &[
    "http", "https", "ftp", "ftps", "file", "sftp", "scp", "ssh", "telnet",
    "gopher", "gophers", "pop3", "pop3s", "imap", "imaps", "smtp", "smtps",
    "ldap", "ldaps", "ws", "wss", "tftp", "dict", "rtsp", "rtsps", "mqtt",
    "mqtts", "redis", "memcached", "ntp", "ipfs", "ipns",
];

/// Protocol allow-list resolved from `--proto` / `--proto-redir`.
#[derive(Debug, Clone)]
pub struct ProtoFilter {
    allowed: HashSet<String>,
}

impl ProtoFilter {
    /// Build the default filter (everything recon supports).
    pub fn default_filter() -> Self {
        Self {
            allowed: DEFAULT_ALLOWED.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Parse a curl-compatible `--proto` spec.
    pub fn parse(spec: &str) -> Result<Self> {
        let mut reset_done = false;
        let mut filter = Self::default_filter();
        for raw in spec.split(',') {
            let token = raw.trim();
            if token.is_empty() {
                continue;
            }
            let (op, rest) = split_op(token);
            match op {
                '=' => {
                    if !reset_done {
                        filter.allowed.clear();
                        reset_done = true;
                    }
                    expand_into(rest, |p| {
                        filter.allowed.insert(p.to_string());
                    });
                }
                '+' => {
                    expand_into(rest, |p| {
                        filter.allowed.insert(p.to_string());
                    });
                }
                '-' => {
                    expand_into(rest, |p| {
                        filter.allowed.remove(p);
                    });
                }
                _ => unreachable!(),
            }
        }
        Ok(filter)
    }

    /// Check whether a scheme (case-insensitive) is allowed.
    pub fn allows(&self, scheme: &str) -> bool {
        self.allowed.contains(&scheme.to_ascii_lowercase())
    }

    /// Validate a URL against the filter. `Ok(())` when allowed;
    /// actionable error otherwise.
    pub fn validate_url(&self, url: &str) -> Result<()> {
        let scheme = url
            .split_once("://")
            .map(|(s, _)| s.to_ascii_lowercase())
            .unwrap_or_default();
        if scheme.is_empty() {
            anyhow::bail!(
                "--proto: URL '{url}' has no scheme — use --proto-default to pick one"
            );
        }
        if !self.allows(&scheme) {
            anyhow::bail!(
                "--proto: scheme '{scheme}' is not in the allow-list (allowed: {})",
                self.sorted_list().join(", "),
            );
        }
        Ok(())
    }

    fn sorted_list(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.allowed.iter().map(|s| s.as_str()).collect();
        v.sort();
        v
    }
}

fn split_op(token: &str) -> (char, &str) {
    match token.chars().next() {
        Some('=') => ('=', &token[1..]),
        Some('-') => ('-', &token[1..]),
        Some('+') => ('+', &token[1..]),
        _ => ('+', token),
    }
}

fn expand_into(name: &str, mut f: impl FnMut(&str)) {
    if name.eq_ignore_ascii_case("all") {
        for p in DEFAULT_ALLOWED {
            f(p);
        }
    } else {
        f(&name.to_ascii_lowercase());
    }
}

/// Apply `--proto-default` to a URL that has no scheme. Returns the
/// original URL unchanged when a scheme is already present.
pub fn apply_default_scheme(url: &str, default: Option<&str>) -> String {
    if url.contains("://") {
        return url.to_string();
    }
    if let Some(scheme) = default {
        return format!("{scheme}://{url}");
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_allows_everything_we_know() {
        let f = ProtoFilter::default_filter();
        assert!(f.allows("https"));
        assert!(f.allows("ftp"));
        assert!(f.allows("sftp"));
        assert!(!f.allows("gopher4"));
    }

    #[test]
    fn eq_resets_list() {
        let f = ProtoFilter::parse("=https").unwrap();
        assert!(f.allows("https"));
        assert!(!f.allows("http"));
        assert!(!f.allows("ftp"));
    }

    #[test]
    fn eq_and_plus_combine() {
        let f = ProtoFilter::parse("=https,+http").unwrap();
        assert!(f.allows("https"));
        assert!(f.allows("http"));
        assert!(!f.allows("ftp"));
    }

    #[test]
    fn minus_removes_from_defaults() {
        let f = ProtoFilter::parse("-ftp,-ftps").unwrap();
        assert!(f.allows("https"));
        assert!(!f.allows("ftp"));
        assert!(!f.allows("ftps"));
    }

    #[test]
    fn all_special_expands() {
        let f = ProtoFilter::parse("=all").unwrap();
        assert!(f.allows("https"));
        assert!(f.allows("ftp"));
        assert!(f.allows("mqtt"));
    }

    #[test]
    fn validate_url_rejects_disallowed_scheme() {
        let f = ProtoFilter::parse("=https").unwrap();
        assert!(f.validate_url("https://example.com/").is_ok());
        assert!(f.validate_url("ftp://example.com/").is_err());
    }

    #[test]
    fn validate_url_complains_about_missing_scheme() {
        let f = ProtoFilter::default_filter();
        let err = f.validate_url("example.com/foo").unwrap_err();
        assert!(err.to_string().contains("no scheme"), "got: {err}");
    }

    #[test]
    fn apply_default_scheme_adds_when_missing() {
        assert_eq!(
            apply_default_scheme("example.com/foo", Some("https")),
            "https://example.com/foo"
        );
    }

    #[test]
    fn apply_default_scheme_preserves_existing() {
        assert_eq!(
            apply_default_scheme("http://example.com/", Some("https")),
            "http://example.com/"
        );
    }
}
