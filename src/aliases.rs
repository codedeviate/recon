//! CLI alias resolution. Rewrites short flags into long forms per
//! a named `[aliases.<name>]` section before clap sees the argv.
//!
//! Bundled defaults live in `assets/aliases.toml` and are linked in
//! at build time. User entries in `~/.recon/config.toml` deep-merge
//! on top, per key.

#![allow(dead_code, unused_imports)]   // ← removed in later tasks

use std::collections::BTreeMap;
use std::sync::OnceLock;

#[allow(unused_imports)]
use anyhow::{anyhow, bail, Result};
#[allow(unused_imports)]
use serde::Deserialize;

/// Bundled aliases TOML text. Linked in at compile time.
const BUNDLED_TOML: &str = include_str!("../assets/aliases.toml");

/// Parsed bundled aliases, cached on first access.
fn bundled() -> &'static toml::Value {
    static CACHED: OnceLock<toml::Value> = OnceLock::new();
    CACHED.get_or_init(|| {
        toml::from_str(BUNDLED_TOML).expect("bundled aliases.toml must parse")
    })
}

/// One short→long binding plus arity metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasEntry {
    pub long: String,
    pub takes_value: bool,
}

/// Untagged shape accepted in TOML: either a bare string or a table
/// with explicit `takes_value`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AliasEntryShape {
    Flat(String),
    Detailed {
        long: String,
        #[serde(default)]
        takes_value: bool,
    },
}

impl From<AliasEntryShape> for AliasEntry {
    fn from(s: AliasEntryShape) -> Self {
        match s {
            AliasEntryShape::Flat(long) => AliasEntry { long, takes_value: false },
            AliasEntryShape::Detailed { long, takes_value } => {
                AliasEntry { long, takes_value }
            }
        }
    }
}

/// Resolved alias map, keyed by single short-flag character.
#[derive(Debug, Default, Clone)]
pub struct AliasMap {
    pub entries: BTreeMap<char, AliasEntry>,
}

impl AliasMap {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Parse a `toml::Value` representing one alias section (the
    /// inner table of `[aliases.<name>]`) into an `AliasMap`. Keys
    /// must match `-x` (single dash + single ASCII char).
    pub fn from_toml(value: &toml::Value) -> Result<Self> {
        let table = value
            .as_table()
            .ok_or_else(|| anyhow!("alias section must be a table"))?;
        let mut entries = BTreeMap::new();
        for (key, val) in table {
            let ch = parse_short_key(key)?;
            let shape: AliasEntryShape = val
                .clone()
                .try_into()
                .map_err(|e| anyhow!("alias '{key}': {e}"))?;
            entries.insert(ch, shape.into());
        }
        Ok(AliasMap { entries })
    }
}

fn parse_short_key(key: &str) -> Result<char> {
    let rest = key
        .strip_prefix('-')
        .ok_or_else(|| anyhow!("alias key '{key}': expected key like '-x'"))?;
    let mut chars = rest.chars();
    let ch = chars
        .next()
        .ok_or_else(|| anyhow!("alias key '{key}': expected key like '-x'"))?;
    if chars.next().is_some() {
        bail!("alias key '{key}': expected key like '-x' (single character)");
    }
    Ok(ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_toml_parses() {
        let v = bundled();
        // Two top-level tables.
        assert!(v.get("curl").is_some(), "curl section missing");
        assert!(v.get("wget").is_some(), "wget section missing");
    }

    fn parse_section(toml_text: &str) -> AliasMap {
        let v: toml::Value = toml::from_str(toml_text).unwrap();
        AliasMap::from_toml(&v).unwrap()
    }

    #[test]
    fn untagged_serde_accepts_flat_string() {
        let m = parse_section(r#""-r" = "--recursive""#);
        assert_eq!(
            m.entries.get(&'r'),
            Some(&AliasEntry { long: "--recursive".into(), takes_value: false })
        );
    }

    #[test]
    fn untagged_serde_accepts_table_form() {
        let m = parse_section(r#""-l" = { long = "--level", takes_value = true }"#);
        assert_eq!(
            m.entries.get(&'l'),
            Some(&AliasEntry { long: "--level".into(), takes_value: true })
        );
    }

    #[test]
    fn rejects_short_with_no_dash() {
        let v: toml::Value = toml::from_str(r#""r" = "--recursive""#).unwrap();
        let err = AliasMap::from_toml(&v).unwrap_err();
        assert!(err.to_string().contains("expected key like '-x'"));
    }

    #[test]
    fn rejects_short_with_more_than_one_letter() {
        let v: toml::Value = toml::from_str(r#""-rr" = "--recursive""#).unwrap();
        let err = AliasMap::from_toml(&v).unwrap_err();
        assert!(err.to_string().contains("expected key like '-x'"));
    }
}
