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
}
