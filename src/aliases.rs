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

/// Public entry point. Reads the bundled aliases and the resolved
/// user-config layered TOML, deep-merges the requested `[aliases.<name>]`
/// section, and returns the merged `AliasMap`.
pub fn resolve(name: &str, user_layers: &toml::Value) -> Result<AliasMap> {
    resolve_with(name, bundled(), user_layers)
}

/// Same as `resolve` but with an injectable bundled value, for tests.
fn resolve_with(
    name: &str,
    bundled: &toml::Value,
    user_layers: &toml::Value,
) -> Result<AliasMap> {
    let bundled_section = bundled.get(name);
    let user_section = user_layers
        .get("aliases")
        .and_then(|v| v.get(name));

    if bundled_section.is_none() && user_section.is_none() {
        let mut known: Vec<&str> = bundled
            .as_table()
            .map(|t| t.keys().map(String::as_str).collect())
            .unwrap_or_default();
        if let Some(t) = user_layers.get("aliases").and_then(|v| v.as_table()) {
            for k in t.keys() {
                known.push(k);
            }
        }
        known.sort_unstable();
        known.dedup();
        let known_list = if known.is_empty() {
            "(none)".to_string()
        } else {
            known.join(", ")
        };
        bail!(
            "alias '{name}' is not defined in config.toml or bundled aliases. \
             Known: {known_list}"
        );
    }

    let mut merged = toml::value::Table::new();
    if let Some(t) = bundled_section.and_then(|v| v.as_table()) {
        for (k, v) in t {
            merged.insert(k.clone(), v.clone());
        }
    }
    if let Some(t) = user_section.and_then(|v| v.as_table()) {
        for (k, v) in t {
            merged.insert(k.clone(), v.clone());
        }
    }
    AliasMap::from_toml(&toml::Value::Table(merged))
}

/// Rewrite `argv` letter-by-letter according to `map`. Stops
/// processing at `--`. Long forms and unmapped shorts pass through
/// unchanged.
pub fn apply(argv: Vec<String>, map: &AliasMap) -> Result<Vec<String>> {
    if map.is_empty() {
        return Ok(argv);
    }
    let mut out: Vec<String> = Vec::with_capacity(argv.len());
    let mut iter = argv.into_iter();
    while let Some(tok) = iter.next() {
        if tok == "--" {
            out.push(tok);
            out.extend(iter);
            return Ok(out);
        }
        // Long forms (`--foo`) and non-flag tokens pass through.
        if tok.starts_with("--") || !tok.starts_with('-') || tok.len() < 2 {
            out.push(tok);
            continue;
        }
        // Short-flag cluster: `-x`, `-xy`, `-x VAL`, `-xVAL`.
        // Multi-letter handling is added in later tasks; for now,
        // single-letter only.
        let cluster: Vec<char> = tok[1..].chars().collect();
        // Lookup the leading letter.
        let lead = cluster[0];
        let lead_entry = map.entries.get(&lead).cloned();
        if cluster.len() == 1 {
            match lead_entry {
                Some(entry) => out.push(entry.long),
                None => out.push(tok),
            }
            continue;
        }
        // Multi-character cluster. If the lead letter takes a value,
        // the remainder is the embedded value: `-l3` → `--level 3`.
        if let Some(entry) = &lead_entry {
            if entry.takes_value {
                out.push(entry.long.clone());
                out.push(cluster[1..].iter().collect::<String>());
                continue;
            }
        }
        // Otherwise this is either a combined-bool cluster (handled
        // in Task 7) or a passthrough (no leading-letter mapping at
        // all). For now, error explicitly if the leading letter is
        // a bool alias with junk attached.
        if let Some(entry) = &lead_entry {
            if !entry.takes_value {
                bail!(
                    "alias '{tok}' has trailing value but '{}' takes no value",
                    entry.long
                );
            }
        }
        // No leading-letter mapping → passthrough.
        out.push(tok);
    }
    Ok(out)
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

    fn merge(bundled: &str, user: &str, name: &str) -> AliasMap {
        let bundled_v: toml::Value = toml::from_str(bundled).unwrap();
        let user_v: toml::Value = toml::from_str(user).unwrap();
        resolve_with(name, &bundled_v, &user_v).unwrap()
    }

    #[test]
    fn user_overrides_bundled_per_key() {
        let m = merge(
            r#"[wget]
                "-r" = "--recursive""#,
            r#"[aliases.wget]
                "-r" = "--range""#,
            "wget",
        );
        assert_eq!(m.entries.get(&'r').unwrap().long, "--range");
    }

    #[test]
    fn user_adds_new_letter_to_bundled() {
        let m = merge(
            r#"[wget]
                "-r" = "--recursive""#,
            r#"[aliases.wget]
                "-J" = "--json""#,
            "wget",
        );
        assert_eq!(m.entries.get(&'r').unwrap().long, "--recursive");
        assert_eq!(m.entries.get(&'J').unwrap().long, "--json");
    }

    #[test]
    fn user_only_alias_resolves_without_bundled() {
        let m = merge(
            "",  // no bundled
            r#"[aliases.mine]
                "-x" = "--foo""#,
            "mine",
        );
        assert_eq!(m.entries.get(&'x').unwrap().long, "--foo");
    }

    #[test]
    fn unknown_alias_name_errors() {
        let bundled_v: toml::Value = toml::from_str("").unwrap();
        let user_v: toml::Value = toml::from_str("").unwrap();
        let err = resolve_with("nonesuch", &bundled_v, &user_v).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("nonesuch"), "msg: {msg}");
        assert!(msg.contains("not defined"), "msg: {msg}");
    }

    fn one_entry(ch: char, long: &str, takes_value: bool) -> AliasMap {
        let mut entries = BTreeMap::new();
        entries.insert(ch, AliasEntry { long: long.into(), takes_value });
        AliasMap { entries }
    }

    fn argv(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn empty_map_passes_argv_through() {
        let map = AliasMap::default();
        let out = apply(argv(&["recon", "-r", "url"]), &map).unwrap();
        assert_eq!(out, vec!["recon", "-r", "url"]);
    }

    #[test]
    fn simple_short_to_long() {
        let map = one_entry('r', "--recursive", false);
        let out = apply(argv(&["recon", "-r", "url"]), &map).unwrap();
        assert_eq!(out, vec!["recon", "--recursive", "url"]);
    }

    #[test]
    fn double_dash_terminator_stops_rewrite() {
        let map = one_entry('r', "--recursive", false);
        let out = apply(argv(&["recon", "-r", "--", "-r"]), &map).unwrap();
        assert_eq!(out, vec!["recon", "--recursive", "--", "-r"]);
    }

    #[test]
    fn long_forms_untouched() {
        let map = one_entry('r', "--recursive", false);
        let out = apply(argv(&["recon", "--anything", "url"]), &map).unwrap();
        assert_eq!(out, vec!["recon", "--anything", "url"]);
    }

    #[test]
    fn unknown_short_passes_through() {
        let map = one_entry('r', "--recursive", false);
        let out = apply(argv(&["recon", "-z", "url"]), &map).unwrap();
        assert_eq!(out, vec!["recon", "-z", "url"]);
    }

    #[test]
    fn value_taker_with_space() {
        let map = one_entry('l', "--level", true);
        let out = apply(argv(&["recon", "-l", "3", "url"]), &map).unwrap();
        assert_eq!(out, vec!["recon", "--level", "3", "url"]);
    }

    #[test]
    fn value_taker_with_embedded_value() {
        let map = one_entry('l', "--level", true);
        let out = apply(argv(&["recon", "-l3", "url"]), &map).unwrap();
        assert_eq!(out, vec!["recon", "--level", "3", "url"]);
    }

    #[test]
    fn embedded_value_on_bool_errors() {
        let map = one_entry('r', "--recursive", false);
        let err = apply(argv(&["recon", "-r3"]), &map).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("-r3"), "msg: {msg}");
        assert!(msg.contains("--recursive"), "msg: {msg}");
        assert!(msg.contains("takes no value"), "msg: {msg}");
    }
}
