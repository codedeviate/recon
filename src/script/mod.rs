//! Rhai scripting engine for `recon --script PATH`.
//!
//! Loads a `.rhai` file, registers all probe bindings (`http`, `tcp`, `dns`,
//! `tls`, …) plus helpers (`sleep_ms`, `env`, `assert`, …), and executes it.
//! The script's `return N` (integer) becomes the process exit code; uncaught
//! exceptions map to non-zero exits via `ProtocolExitCode` where applicable.
//!
//! Script resolution order when `--script NAME` doesn't exist as given:
//!   1. `NAME` (the literal path, as given)
//!   2. `~/.recon/script/NAME`
//!   3. `~/.recon/script/NAME.rhai` (only if `NAME` has no extension)
//!
//! This lets users drop reusable scripts in `~/.recon/script/` and call
//! them by bare name: `recon --script health` finds
//! `~/.recon/script/health.rhai`.

use crate::cli::Args;
use std::path::{Path, PathBuf};

pub mod bindings;
pub mod convert;
pub mod defaults;
pub mod engine;

/// Entry point from `main.rs`. Returns the process exit code.
pub fn run(args: &Args) -> i32 {
    let requested = match &args.script {
        Some(p) => p.clone(),
        None => {
            eprintln!("error: --script requires a path");
            return 1;
        }
    };
    let resolved = match resolve_script_path(&requested) {
        Some(p) => p,
        None => {
            eprintln!("error: could not find script '{}'", requested.display());
            for tried in tried_paths(&requested) {
                eprintln!("  tried: {}", tried.display());
            }
            return 1;
        }
    };
    engine::run_file(&resolved, args)
}

/// Return the global script directory (`~/.recon/script/`) if `$HOME`
/// is set. `None` when we can't resolve home — in which case the global
/// fallback is skipped.
pub fn script_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".recon").join("script"))
}

/// Resolve a `--script` argument against the filesystem. Tries the path
/// as given first, then falls back to `~/.recon/script/NAME` (and, if
/// `NAME` has no extension, `~/.recon/script/NAME.rhai`).
pub fn resolve_script_path(requested: &Path) -> Option<PathBuf> {
    resolve_in(requested, script_dir().as_deref())
}

fn resolve_in(requested: &Path, dir: Option<&Path>) -> Option<PathBuf> {
    if requested.exists() {
        return Some(requested.to_path_buf());
    }
    let dir = dir?;
    let in_dir = dir.join(requested);
    if in_dir.exists() {
        return Some(in_dir);
    }
    if requested.extension().is_none() {
        let with_ext = in_dir.with_extension("rhai");
        if with_ext.exists() {
            return Some(with_ext);
        }
    }
    None
}

/// Ordered list of paths `resolve_script_path` will have tried. Used for
/// the error message.
fn tried_paths(requested: &Path) -> Vec<PathBuf> {
    let mut out = vec![requested.to_path_buf()];
    if let Some(dir) = script_dir() {
        out.push(dir.join(requested));
        if requested.extension().is_none() {
            out.push(dir.join(requested).with_extension("rhai"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn as_given_path_takes_precedence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.rhai");
        std::fs::write(&path, "return 0;").unwrap();
        let resolved = resolve_in(&path, None).expect("resolves as-given");
        assert_eq!(resolved, path);
    }

    #[test]
    fn global_dir_fallback_finds_named_script() {
        let global = tempdir().unwrap();
        let script = global.path().join("greet.rhai");
        std::fs::write(&script, "return 7;").unwrap();

        let resolved = resolve_in(Path::new("greet.rhai"), Some(global.path()));
        assert_eq!(resolved.as_deref(), Some(script.as_path()));
    }

    #[test]
    fn global_dir_fallback_auto_appends_rhai_extension() {
        let global = tempdir().unwrap();
        let script = global.path().join("health.rhai");
        std::fs::write(&script, "return 0;").unwrap();

        let resolved = resolve_in(Path::new("health"), Some(global.path()));
        assert_eq!(resolved.as_deref(), Some(script.as_path()));
    }

    #[test]
    fn global_dir_fallback_skipped_when_extension_already_present() {
        let global = tempdir().unwrap();
        std::fs::write(global.path().join("foo.txt.rhai"), "return 0;").unwrap();

        // Requested "foo.txt" — has extension, so we don't strip + re-append.
        let resolved = resolve_in(Path::new("foo.txt"), Some(global.path()));
        assert_eq!(resolved, None);
    }

    #[test]
    fn missing_everywhere_returns_none() {
        let global = tempdir().unwrap();
        let resolved = resolve_in(Path::new("no-such-script-xyz"), Some(global.path()));
        assert_eq!(resolved, None);
    }

    #[test]
    fn no_global_dir_falls_through() {
        let resolved = resolve_in(Path::new("/nonexistent/path/xyz.rhai"), None);
        assert_eq!(resolved, None);
    }
}
