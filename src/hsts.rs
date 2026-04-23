//! Persistent HSTS (HTTP Strict Transport Security) cache.
//!
//! File format (compatible with curl's `--hsts` plain-text TSV):
//!
//! ```text
//! # HSTS cache for recon
//! # host [.]hostname expires_unix
//! example.com 1756492800
//! .app        1724956800
//! ```
//!
//! A leading `.` on the host indicates `includeSubDomains`. `max-age`
//! from the server's `Strict-Transport-Security` header sets the
//! `expires_unix` epoch (now + max-age seconds). `max-age=0` removes
//! the entry.
//!
//! Integration (see `src/client.rs::execute`):
//! 1. Before sending: load the store, check if the target hostname has
//!    a non-expired entry. If yes and the URL is `http://`, upgrade to
//!    `https://`.
//! 2. After the response: parse the `Strict-Transport-Security` header,
//!    update the store accordingly, save atomically.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HstsEntry {
    pub expires_unix: i64,
    pub include_subdomains: bool,
}

#[derive(Debug, Default, Clone)]
pub struct HstsStore {
    entries: BTreeMap<String, HstsEntry>,
}

impl HstsStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load from `path`. Missing files are silently treated as empty
    /// (first-run UX). Malformed lines are skipped with no error.
    pub fn load(path: &Path) -> Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(e) => return Err(e).with_context(|| format!("hsts: read {}", path.display())),
        };
        let mut store = Self::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let mut parts = trimmed.split_whitespace();
            let (Some(host_raw), Some(exp_raw)) = (parts.next(), parts.next()) else {
                continue;
            };
            let Ok(expires_unix) = exp_raw.parse::<i64>() else {
                continue;
            };
            let (host, include_subdomains) = if let Some(rest) = host_raw.strip_prefix('.') {
                (rest.to_ascii_lowercase(), true)
            } else {
                (host_raw.to_ascii_lowercase(), false)
            };
            store.entries.insert(
                host,
                HstsEntry {
                    expires_unix,
                    include_subdomains,
                },
            );
        }
        Ok(store)
    }

    /// Write atomically to `path` via a tempfile + rename.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        let tmp = tempfile::NamedTempFile::new_in(
            path.parent().unwrap_or_else(|| Path::new(".")),
        )
        .context("hsts: create tempfile")?;
        {
            let mut w = tmp.as_file();
            writeln!(w, "# HSTS cache (recon {})", env!("CARGO_PKG_VERSION"))?;
            writeln!(w, "# host expires_unix   (leading . = includeSubDomains)")?;
            for (host, entry) in &self.entries {
                let name = if entry.include_subdomains {
                    format!(".{host}")
                } else {
                    host.clone()
                };
                writeln!(w, "{name} {}", entry.expires_unix)?;
            }
            w.flush()?;
        }
        tmp.persist(path)
            .with_context(|| format!("hsts: persist {}", path.display()))?;
        Ok(())
    }

    /// Returns true when this host is protected by a non-expired HSTS
    /// entry — meaning an `http://` request should be upgraded.
    pub fn matches(&self, host: &str) -> bool {
        let host = host.to_ascii_lowercase();
        let now = unix_now();
        // Exact match (entry must not be expired).
        if let Some(e) = self.entries.get(&host) {
            if e.expires_unix > now {
                return true;
            }
        }
        // Subdomain match against any entry with include_subdomains.
        let mut parent = host.as_str();
        while let Some(idx) = parent.find('.') {
            parent = &parent[idx + 1..];
            if parent.is_empty() {
                break;
            }
            if let Some(e) = self.entries.get(parent) {
                if e.include_subdomains && e.expires_unix > now {
                    return true;
                }
            }
        }
        false
    }

    /// Consume a `Strict-Transport-Security` header value and update the
    /// entry for `host`. Returns true when the store changed (a hint to
    /// save).
    pub fn update_from_sts_header(&mut self, host: &str, value: &str) -> bool {
        let Some(parsed) = parse_sts(value) else {
            return false;
        };
        let host = host.to_ascii_lowercase();
        if parsed.max_age == 0 {
            return self.entries.remove(&host).is_some();
        }
        let new_entry = HstsEntry {
            expires_unix: unix_now().saturating_add(parsed.max_age as i64),
            include_subdomains: parsed.include_subdomains,
        };
        match self.entries.get(&host) {
            Some(existing) if *existing == new_entry => false,
            _ => {
                self.entries.insert(host, new_entry);
                true
            }
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Parsed form of an STS header.
pub struct StsDirectives {
    pub max_age: u64,
    pub include_subdomains: bool,
}

pub fn parse_sts(value: &str) -> Option<StsDirectives> {
    let mut max_age: Option<u64> = None;
    let mut include_subdomains = false;
    for part in value.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((k, v)) = part.split_once('=') {
            let k = k.trim().to_ascii_lowercase();
            let v = v.trim().trim_matches('"');
            if k == "max-age" {
                if let Ok(n) = v.parse::<u64>() {
                    max_age = Some(n);
                }
            }
        } else if part.eq_ignore_ascii_case("includeSubDomains") {
            include_subdomains = true;
        }
    }
    max_age.map(|ma| StsDirectives {
        max_age: ma,
        include_subdomains,
    })
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn parse_sts_max_age_only() {
        let d = parse_sts("max-age=3600").unwrap();
        assert_eq!(d.max_age, 3600);
        assert!(!d.include_subdomains);
    }

    #[test]
    fn parse_sts_with_subdomains() {
        let d = parse_sts("max-age=31536000; includeSubDomains").unwrap();
        assert_eq!(d.max_age, 31536000);
        assert!(d.include_subdomains);
    }

    #[test]
    fn parse_sts_quoted_max_age() {
        let d = parse_sts(r#"max-age="7200" ; includeSubDomains"#).unwrap();
        assert_eq!(d.max_age, 7200);
        assert!(d.include_subdomains);
    }

    #[test]
    fn parse_sts_missing_max_age_is_none() {
        assert!(parse_sts("includeSubDomains").is_none());
    }

    #[test]
    fn store_load_tolerates_missing_file() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        tmp.close().unwrap();
        assert!(!path.exists());
        let s = HstsStore::load(&path).unwrap();
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn store_round_trip() {
        let tmp = NamedTempFile::new().unwrap();
        let mut s = HstsStore::new();
        s.entries.insert(
            "example.com".into(),
            HstsEntry { expires_unix: 2_000_000_000, include_subdomains: false },
        );
        s.entries.insert(
            "app".into(),
            HstsEntry { expires_unix: 1_900_000_000, include_subdomains: true },
        );
        s.save(tmp.path()).unwrap();
        let loaded = HstsStore::load(tmp.path()).unwrap();
        assert_eq!(loaded.entries, s.entries);
    }

    #[test]
    fn matches_exact_and_subdomain() {
        let mut s = HstsStore::new();
        s.entries.insert(
            "example.com".into(),
            HstsEntry { expires_unix: i64::MAX, include_subdomains: false },
        );
        s.entries.insert(
            "app".into(),
            HstsEntry { expires_unix: i64::MAX, include_subdomains: true },
        );
        assert!(s.matches("example.com"));
        assert!(!s.matches("foo.example.com"));  // no subdomains flag
        assert!(s.matches("myapp.app"));
        assert!(s.matches("app"));
    }

    #[test]
    fn expired_entry_does_not_match() {
        let mut s = HstsStore::new();
        s.entries.insert(
            "example.com".into(),
            HstsEntry { expires_unix: 1, include_subdomains: true },
        );
        assert!(!s.matches("example.com"));
    }

    #[test]
    fn update_inserts_then_removes_on_zero() {
        let mut s = HstsStore::new();
        assert!(s.update_from_sts_header("example.com", "max-age=3600"));
        assert!(s.matches("example.com"));
        assert!(s.update_from_sts_header("example.com", "max-age=0"));
        assert!(!s.matches("example.com"));
    }

    #[test]
    fn update_subdomains_flag() {
        let mut s = HstsStore::new();
        assert!(s.update_from_sts_header("example.com", "max-age=3600; includeSubDomains"));
        assert!(s.matches("foo.example.com"));
    }

    #[test]
    fn load_ignores_comments_and_malformed() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "# comment\n\nmalformed\nexample.com 1000000\n.app 2000000\ngarbage nonnum\n").unwrap();
        let s = HstsStore::load(tmp.path()).unwrap();
        assert_eq!(s.len(), 2);
    }
}
