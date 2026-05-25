//! Layered TOML config resolver — reads `/etc/recon/<name>.toml` (system)
//! and `~/.recon/<name>.toml` (user), deep-merges them with user winning,
//! and returns a single `toml::Value`. Used by `src/config.rs` and the gh
//! script binding.
//!
//! See `docs/MANUAL.md` "Configuration files" for the public model;
//! see `~/Development/Starweb/superpowers/recon/specs/2026-05-25-layered-config-design.md`
//! for design rationale.

#[allow(unused_imports)]
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct LayerOpts {
    pub skip_system:     bool,
    pub skip_user:       bool,
    pub system_override: Option<PathBuf>,
    pub user_override:   Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct Resolved {
    pub system: Option<PathBuf>,
    pub user:   Option<PathBuf>,
}

/// Deep-merge `overlay` onto `base`. Tables merge recursively; arrays
/// and leaves are replaced by overlay. Type clashes (table vs. leaf)
/// resolve with overlay winning silently — schema enforcement happens
/// in the downstream serde deserialize.
fn deep_merge(base: &mut toml::Value, overlay: toml::Value) {
    use toml::Value;
    match (base, overlay) {
        (Value::Table(b), Value::Table(o)) => {
            for (k, v) in o {
                match b.get_mut(&k) {
                    Some(existing) => deep_merge(existing, v),
                    None => {
                        b.insert(k, v);
                    }
                }
            }
        }
        (slot, overlay) => {
            *slot = overlay;
        }
    }
}

#[cfg(test)]
mod merge_tests {
    use super::*;
    use toml::Value;

    fn v(s: &str) -> Value {
        s.parse().unwrap()
    }

    #[test]
    fn overlay_leaf_replaces_base_leaf() {
        let mut base = v(r#"x = "old""#);
        let overlay = v(r#"x = "new""#);
        deep_merge(&mut base, overlay);
        assert_eq!(base, v(r#"x = "new""#));
    }

    #[test]
    fn overlay_table_merges_sibling_keys_preserved() {
        let mut base = v("[t]\na = 1\nb = 2\n");
        let overlay = v("[t]\nb = 20\nc = 30\n");
        deep_merge(&mut base, overlay);
        assert_eq!(base, v("[t]\na = 1\nb = 20\nc = 30\n"));
    }

    #[test]
    fn overlay_array_replaces_base_array_no_concat() {
        let mut base = v(r#"items = ["a", "b"]"#);
        let overlay = v(r#"items = ["c"]"#);
        deep_merge(&mut base, overlay);
        assert_eq!(base, v(r#"items = ["c"]"#));
    }

    #[test]
    fn overlay_empty_array_replaces_non_empty_base() {
        let mut base = v(r#"items = ["a", "b"]"#);
        let overlay = v("items = []");
        deep_merge(&mut base, overlay);
        assert_eq!(base, v("items = []"));
    }

    #[test]
    fn overlay_table_replaces_base_leaf_of_same_key() {
        let mut base = v(r#"x = "string""#);
        let overlay = v("[x]\na = 1\n");
        deep_merge(&mut base, overlay);
        assert_eq!(base, v("[x]\na = 1\n"));
    }

    #[test]
    fn overlay_leaf_replaces_base_table_of_same_key() {
        let mut base = v("[x]\na = 1\n");
        let overlay = v(r#"x = "string""#);
        deep_merge(&mut base, overlay);
        assert_eq!(base, v(r#"x = "string""#));
    }

    #[test]
    fn empty_overlay_leaves_base_unchanged() {
        let mut base = v("a = 1\nb = 2\n");
        let original = base.clone();
        deep_merge(&mut base, v(""));
        assert_eq!(base, original);
    }

    #[test]
    fn deeply_nested_table_merges_correctly() {
        let mut base = v(r#"
            [a.b.c]
            x = 1
            y = 2
        "#);
        let overlay = v(r#"
            [a.b.c]
            y = 20
            z = 30
            [a.b.d]
            new = "table"
        "#);
        deep_merge(&mut base, overlay);
        assert_eq!(
            base,
            v(r#"
                [a.b.c]
                x = 1
                y = 20
                z = 30
                [a.b.d]
                new = "table"
            "#)
        );
    }
}
