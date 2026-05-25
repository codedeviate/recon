//! Layered TOML config resolver — reads `/etc/recon/<name>.toml` (system)
//! and `~/.recon/<name>.toml` (user), deep-merges them with user winning,
//! and returns a single `toml::Value`. Used by `src/config.rs` and the gh
//! script binding.
//!
//! See `docs/MANUAL.md` "Configuration files" for the public model;
//! see `~/Development/Starweb/superpowers/recon/specs/2026-05-25-layered-config-design.md`
//! for design rationale.

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::sync::OnceLock;

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

static GLOBAL_OPTS: OnceLock<LayerOpts> = OnceLock::new();

impl LayerOpts {
    /// Build options purely from env vars; the CLI-flag overlay
    /// (`merge_cli_flags`) is applied separately in main.rs.
    pub fn from_env() -> Self {
        Self::from_env_with(|k| std::env::var(k).ok())
    }

    fn from_env_with(read: impl Fn(&str) -> Option<String>) -> Self {
        LayerOpts {
            skip_system:     false,
            skip_user:       false,
            system_override: read("RECON_SYSTEM_CONFIG").map(PathBuf::from),
            user_override:   read("RECON_CONFIG").map(PathBuf::from),
        }
    }

    /// Apply the three CLI flags onto an existing LayerOpts. Flag wins
    /// over env-var override (see spec §"Path resolution").
    pub fn merge_cli_flags(
        mut self,
        no_config: bool,
        no_system_config: bool,
        no_user_config: bool,
    ) -> Self {
        if no_config || no_system_config {
            self.skip_system = true;
        }
        if no_config || no_user_config {
            self.skip_user = true;
        }
        self
    }
}

/// Set the process-wide `LayerOpts` once. Subsequent calls are
/// silently ignored — the first call wins, matching `OnceLock`
/// semantics. Returns the stored opts.
pub fn init_global(opts: LayerOpts) -> &'static LayerOpts {
    let _ = GLOBAL_OPTS.set(opts.clone());
    GLOBAL_OPTS.get().unwrap_or_else(|| {
        // Unreachable in practice (we just set it), but cope if a
        // concurrent caller raced us.
        Box::leak(Box::new(opts))
    })
}

/// Return the process-wide `LayerOpts`, or a default if `init_global`
/// was never called (test paths, REPL).
pub fn global() -> LayerOpts {
    GLOBAL_OPTS.get().cloned().unwrap_or_default()
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

/// Resolve which system+user paths actually exist for the given config
/// name. Returns `None` for layers that are skipped or have no match.
/// `--*-config` flags always beat env-var overrides.
pub fn resolve_paths(name: &str, opts: &LayerOpts) -> Resolved {
    let system_candidates = system_candidates_for(name);
    let user_candidate = user_path_with_home(
        std::env::var("HOME").ok().as_deref(),
        name,
    );
    resolve_paths_with(name, opts, &system_candidates, user_candidate)
}

fn resolve_paths_with(
    name: &str,
    opts: &LayerOpts,
    system_candidates: &[PathBuf],
    user_candidate: Option<PathBuf>,
) -> Resolved {
    let system = if opts.skip_system {
        None
    } else if let Some(p) = &opts.system_override {
        Some(resolve_override(p, name))
    } else {
        system_candidates.iter().find(|p| p.is_file()).cloned()
    };
    let user = if opts.skip_user {
        None
    } else if let Some(p) = &opts.user_override {
        Some(resolve_override(p, name))
    } else {
        user_candidate.filter(|p| p.is_file())
    };
    Resolved { system, user }
}

fn resolve_override(p: &std::path::Path, default_name: &str) -> PathBuf {
    if p.is_dir() {
        p.join(default_name)
    } else {
        p.to_path_buf()
    }
}

/// Load both layers for `name`, deep-merge them (user wins), and return
/// the effective `toml::Value`. Missing files at default paths are
/// silent (returns empty table); missing files at env-var override
/// paths are a hard error.
pub fn load_layered(name: &str, opts: &LayerOpts) -> Result<toml::Value> {
    // If an override is set and points at a nonexistent path, error
    // before consulting resolve_paths (which silently filters non-existent).
    if let Some(p) = opts.system_override.as_ref().filter(|_| !opts.skip_system) {
        let resolved = resolve_override(p, "config.toml");
        if !resolved.exists() {
            return Err(anyhow!(
                "$RECON_SYSTEM_CONFIG points at {} but the file/dir does not exist",
                p.display(),
            ));
        }
    }
    if let Some(p) = opts.user_override.as_ref().filter(|_| !opts.skip_user) {
        let resolved = resolve_override(p, "config.toml");
        if !resolved.exists() {
            return Err(anyhow!(
                "$RECON_CONFIG points at {} but the file/dir does not exist",
                p.display(),
            ));
        }
    }

    let r = resolve_paths(name, opts);
    let mut effective = toml::Value::Table(Default::default());
    if let Some(p) = r.system {
        let v = read_and_parse(&p)?;
        deep_merge(&mut effective, v);
    }
    if let Some(p) = r.user {
        let v = read_and_parse(&p)?;
        deep_merge(&mut effective, v);
    }
    Ok(effective)
}

fn read_and_parse(path: &std::path::Path) -> Result<toml::Value> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("config_resolver: cannot read {}", path.display()))?;
    text.parse::<toml::Value>()
        .map_err(|e| anyhow!("config_resolver: invalid TOML in {}: {e}", path.display()))
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

#[cfg(test)]
mod resolve_paths_tests {
    use super::*;
    use tempfile::TempDir;

    fn touch(path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, b"").unwrap();
    }

    #[test]
    fn default_opts_with_env_overrides_picks_those() {
        let dir = TempDir::new().unwrap();
        let sys = dir.path().join("sys.toml");
        let usr = dir.path().join("usr.toml");
        touch(&sys);
        touch(&usr);
        let opts = LayerOpts {
            system_override: Some(sys.clone()),
            user_override:   Some(usr.clone()),
            ..LayerOpts::default()
        };
        let r = resolve_paths_with("config.toml", &opts, &[], None);
        assert_eq!(r.system, Some(sys));
        assert_eq!(r.user,   Some(usr));
    }

    #[test]
    fn skip_flags_yield_none() {
        let dir = TempDir::new().unwrap();
        let sys = dir.path().join("sys.toml");
        let usr = dir.path().join("usr.toml");
        touch(&sys);
        touch(&usr);
        let opts = LayerOpts {
            skip_system:     true,
            skip_user:       true,
            system_override: Some(sys),
            user_override:   Some(usr),
        };
        let r = resolve_paths_with("config.toml", &opts, &[], None);
        assert_eq!(r.system, None);
        assert_eq!(r.user,   None);
    }

    #[test]
    fn picks_first_existing_system_candidate() {
        let dir = TempDir::new().unwrap();
        let a = dir.path().join("a.toml");
        let b = dir.path().join("b.toml");
        let c = dir.path().join("c.toml");
        touch(&b);
        touch(&c);
        // Only b and c exist; a doesn't. Candidate order is [a, b, c].
        let opts = LayerOpts::default();
        let r = resolve_paths_with("config.toml", &opts, &[a, b.clone(), c], None);
        assert_eq!(r.system, Some(b));
    }

    #[test]
    fn returns_none_when_no_candidate_exists() {
        let dir = TempDir::new().unwrap();
        let a = dir.path().join("does-not-exist.toml");
        let opts = LayerOpts::default();
        let r = resolve_paths_with("config.toml", &opts, &[a], None);
        assert_eq!(r.system, None);
    }

    #[test]
    fn env_var_pointing_at_directory_appends_name() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join("config.toml");
        touch(&cfg);
        let opts = LayerOpts {
            system_override: Some(dir.path().to_path_buf()),
            ..LayerOpts::default()
        };
        let r = resolve_paths_with("config.toml", &opts, &[], None);
        assert_eq!(r.system, Some(cfg));
    }

    #[test]
    fn env_var_pointing_at_missing_file_returns_error_path() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nope.toml");
        let opts = LayerOpts {
            system_override: Some(missing.clone()),
            ..LayerOpts::default()
        };
        // resolve_paths_with returns the (missing) path; load_layered is
        // the layer that turns this into a hard error.
        let r = resolve_paths_with("config.toml", &opts, &[], None);
        assert_eq!(r.system, Some(missing));
    }

    #[test]
    fn skip_flag_wins_over_env_var_override() {
        let dir = TempDir::new().unwrap();
        let sys = dir.path().join("sys.toml");
        touch(&sys);
        let opts = LayerOpts {
            skip_system:     true,
            system_override: Some(sys),
            ..LayerOpts::default()
        };
        let r = resolve_paths_with("config.toml", &opts, &[], None);
        assert_eq!(r.system, None);
    }
}

#[cfg(test)]
mod load_layered_tests {
    use super::*;
    use tempfile::TempDir;

    fn write(path: &std::path::Path, body: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    fn opts_for(sys: Option<&std::path::Path>, usr: Option<&std::path::Path>) -> LayerOpts {
        LayerOpts {
            system_override: sys.map(|p| p.to_path_buf()),
            user_override:   usr.map(|p| p.to_path_buf()),
            ..LayerOpts::default()
        }
    }

    #[test]
    fn both_layers_missing_yields_empty_table() {
        let opts = LayerOpts {
            skip_system: true,
            skip_user:   true,
            ..LayerOpts::default()
        };
        let v = load_layered("config.toml", &opts).unwrap();
        assert_eq!(v, toml::Value::Table(Default::default()));
    }

    #[test]
    fn system_only_loads_cleanly() {
        let dir = TempDir::new().unwrap();
        let sys = dir.path().join("sys.toml");
        write(&sys, r#"[a]
x = 1
"#);
        let opts = opts_for(Some(&sys), None);
        let opts = LayerOpts { skip_user: true, ..opts };
        let v = load_layered("config.toml", &opts).unwrap();
        assert_eq!(v.get("a").and_then(|t| t.get("x")).and_then(|x| x.as_integer()), Some(1));
    }

    #[test]
    fn user_only_loads_cleanly() {
        let dir = TempDir::new().unwrap();
        let usr = dir.path().join("usr.toml");
        write(&usr, r#"[a]
y = 2
"#);
        let opts = opts_for(None, Some(&usr));
        let opts = LayerOpts { skip_system: true, ..opts };
        let v = load_layered("config.toml", &opts).unwrap();
        assert_eq!(v.get("a").and_then(|t| t.get("y")).and_then(|y| y.as_integer()), Some(2));
    }

    #[test]
    fn both_layers_merge_with_user_winning() {
        let dir = TempDir::new().unwrap();
        let sys = dir.path().join("sys.toml");
        let usr = dir.path().join("usr.toml");
        write(&sys, r#"[editor]
default = "vim"
[ai.backends.work]
cmd = "/opt/claude"
"#);
        write(&usr, r#"[editor]
default = "zed"
[ai.backends.scratch]
cmd = "claude"
"#);
        let opts = opts_for(Some(&sys), Some(&usr));
        let v = load_layered("config.toml", &opts).unwrap();
        assert_eq!(
            v.get("editor").and_then(|t| t.get("default")).and_then(|d| d.as_str()),
            Some("zed"),
        );
        assert_eq!(
            v.get("ai").and_then(|t| t.get("backends"))
                .and_then(|t| t.get("work")).and_then(|t| t.get("cmd"))
                .and_then(|c| c.as_str()),
            Some("/opt/claude"),
        );
        assert_eq!(
            v.get("ai").and_then(|t| t.get("backends"))
                .and_then(|t| t.get("scratch")).and_then(|t| t.get("cmd"))
                .and_then(|c| c.as_str()),
            Some("claude"),
        );
    }

    #[test]
    fn malformed_toml_errors_with_path() {
        let dir = TempDir::new().unwrap();
        let usr = dir.path().join("usr.toml");
        write(&usr, "this is = not valid = toml\n");
        let opts = opts_for(None, Some(&usr));
        let opts = LayerOpts { skip_system: true, ..opts };
        let err = load_layered("config.toml", &opts).unwrap_err().to_string();
        assert!(err.contains("invalid TOML"), "got: {err}");
        assert!(err.contains(usr.display().to_string().as_str()), "got: {err}");
    }

    #[test]
    fn env_override_missing_file_errors_loudly() {
        let opts = LayerOpts {
            system_override: Some(PathBuf::from("/nonexistent/path/here.toml")),
            ..LayerOpts::default()
        };
        let opts = LayerOpts { skip_user: true, ..opts };
        let err = load_layered("config.toml", &opts).unwrap_err().to_string();
        assert!(err.contains("does not exist") || err.contains("cannot read"), "got: {err}");
    }
}

#[cfg(test)]
mod layer_opts_tests {
    use super::*;

    #[test]
    fn from_env_with_no_vars_set_yields_empty_overrides() {
        let opts = LayerOpts::from_env_with(|_| None);
        assert!(opts.system_override.is_none());
        assert!(opts.user_override.is_none());
    }

    #[test]
    fn from_env_picks_up_recon_system_config() {
        let opts = LayerOpts::from_env_with(|k| match k {
            "RECON_SYSTEM_CONFIG" => Some("/tmp/sys.toml".into()),
            _ => None,
        });
        assert_eq!(opts.system_override, Some(PathBuf::from("/tmp/sys.toml")));
        assert!(opts.user_override.is_none());
    }

    #[test]
    fn from_env_picks_up_recon_config() {
        let opts = LayerOpts::from_env_with(|k| match k {
            "RECON_CONFIG" => Some("/tmp/usr.toml".into()),
            _ => None,
        });
        assert_eq!(opts.user_override, Some(PathBuf::from("/tmp/usr.toml")));
        assert!(opts.system_override.is_none());
    }
}
