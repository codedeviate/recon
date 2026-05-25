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

/// Return the candidate paths for the system layer in priority order
/// (first match wins). Uses real env vars; for tests, see
/// `system_candidates_with_env`.
pub fn system_candidates() -> Vec<PathBuf> {
    system_candidates_for("config.toml")
}

fn system_candidates_for(name: &str) -> Vec<PathBuf> {
    let brew_prefix = std::env::var("HOMEBREW_PREFIX").ok();
    system_candidates_with_env(name, brew_prefix.as_deref())
}

fn system_candidates_with_env(name: &str, brew_prefix: Option<&str>) -> Vec<PathBuf> {
    let mut out = Vec::new();

    #[cfg(target_os = "macos")]
    {
        if let Some(p) = brew_prefix {
            out.push(PathBuf::from(p).join("etc/recon").join(name));
        }
        out.push(PathBuf::from("/opt/homebrew/etc/recon").join(name));
        out.push(PathBuf::from("/usr/local/etc/recon").join(name));
        out.push(PathBuf::from("/etc/recon").join(name));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = brew_prefix; // silence unused on non-mac
        out.push(PathBuf::from("/etc/recon").join(name));
    }

    out
}

/// Return the user-layer path for `config.toml` (no existence check). Returns
/// None when $HOME is unset.
pub fn user_path() -> Option<PathBuf> {
    user_path_with_home(std::env::var("HOME").ok().as_deref(), "config.toml")
}

fn user_path_with_home(home: Option<&str>, name: &str) -> Option<PathBuf> {
    Some(PathBuf::from(home?).join(".recon").join(name))
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

#[cfg(test)]
mod system_candidates_tests {
    use super::*;

    #[test]
    fn includes_etc_recon_on_every_platform() {
        let paths = system_candidates_for("config.toml");
        assert!(
            paths.iter().any(|p| p == &PathBuf::from("/etc/recon/config.toml")),
            "missing /etc/recon/config.toml in {paths:?}",
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_includes_homebrew_paths() {
        let paths = system_candidates_for("config.toml");
        assert!(paths.iter().any(|p| p == &PathBuf::from("/opt/homebrew/etc/recon/config.toml")));
        assert!(paths.iter().any(|p| p == &PathBuf::from("/usr/local/etc/recon/config.toml")));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_homebrew_prefix_env_var_wins_when_set() {
        let paths = system_candidates_with_env("config.toml", Some("/tmp/brewy"));
        assert_eq!(paths.first(), Some(&PathBuf::from("/tmp/brewy/etc/recon/config.toml")));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn linux_only_etc_recon() {
        let paths = system_candidates_for("config.toml");
        assert_eq!(paths, vec![PathBuf::from("/etc/recon/config.toml")]);
    }
}

#[cfg(test)]
mod user_path_tests {
    use super::*;

    #[test]
    fn user_path_with_home_returns_dot_recon() {
        let p = user_path_with_home(Some("/home/test"), "config.toml");
        assert_eq!(p, Some(PathBuf::from("/home/test/.recon/config.toml")));
    }

    #[test]
    fn user_path_without_home_returns_none() {
        let p = user_path_with_home(None, "config.toml");
        assert_eq!(p, None);
    }
}
