//! `--init`: materialise `~/.recon/` with the standard layout and a
//! commented config skeleton. Idempotent — existing files and directories
//! are left untouched, and each action prints one of `created`, `wrote`,
//! or `skipped (exists)`.

use anyhow::{anyhow, Context, Result};
use std::path::Path;

/// Commented TOML skeleton written to `~/.recon/config.toml` on a first
/// init. Every section is commented so `toml::from_str::<ReconConfig>`
/// parses it as all-defaults.
const SKELETON: &str = r#"# recon — ~/.recon/config.toml
#
# Every section is optional. Uncomment and edit what you need.
# See `recon --help` for per-feature documentation.

# [editor]
# default = "code"
#
# [editor.aliases]
# zed  = "zed --wait"
# sub  = "subl --new-window"

# [netstatus]
# ip_sources         = ["https://ifconfig.me", "https://api.ipify.org"]
# dns_lookup_domains = ["example.com", "google.com"]

# [[netstatus.dns_hijack_checks]]
# domain     = "example.com"
# expected_a = ["93.184.216.34"]

# [sampledata.my_feed]
# mode           = "http"
# default_format = "json"
# count          = 10
# description    = "Custom sample source"
#
# [sampledata.my_feed.urls]
# json = "https://example.com/data.json"
"#;

pub fn run() -> Result<()> {
    let home = std::env::var("HOME")
        .map_err(|_| anyhow!("init: $HOME is not set"))?;
    init_at(Path::new(&home))
}

/// Internal implementation with the home dir injected, so tests can
/// target a tempdir without mutating the process environment.
fn init_at(home: &Path) -> Result<()> {
    let base = home.join(".recon");
    let subdirs = ["script", "jars", "sni"];

    ensure_dir(&base)?;
    for sub in subdirs {
        ensure_dir(&base.join(sub))?;
    }
    ensure_file(&base.join("config.toml"), SKELETON)?;

    println!();
    println!("Done. Edit config.toml or drop .rhai scripts into script/.");
    Ok(())
}

fn ensure_dir(path: &Path) -> Result<()> {
    if path.exists() {
        println!("skipped {} (exists)", path.display());
        return Ok(());
    }
    std::fs::create_dir_all(path)
        .with_context(|| format!("init: create directory {}", path.display()))?;
    println!("created {}", path.display());
    Ok(())
}

fn ensure_file(path: &Path, contents: &str) -> Result<()> {
    if path.exists() {
        println!("skipped {} (exists)", path.display());
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("init: create parent {}", parent.display()))?;
    }
    std::fs::write(path, contents)
        .with_context(|| format!("init: write {}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_full_layout_on_first_run() {
        let home = tempdir().unwrap();
        init_at(home.path()).expect("first init");

        let base = home.path().join(".recon");
        assert!(base.is_dir());
        assert!(base.join("script").is_dir());
        assert!(base.join("jars").is_dir());
        assert!(base.join("sni").is_dir());
        let cfg = base.join("config.toml");
        assert!(cfg.is_file());
        let body = std::fs::read_to_string(&cfg).unwrap();
        assert!(body.starts_with("# recon"));
    }

    #[test]
    fn second_run_is_idempotent() {
        let home = tempdir().unwrap();
        init_at(home.path()).expect("first init");

        // User edits their config.
        let cfg = home.path().join(".recon").join("config.toml");
        std::fs::write(&cfg, "# my edits\n").unwrap();

        // Second run must NOT overwrite.
        init_at(home.path()).expect("second init");
        let body = std::fs::read_to_string(&cfg).unwrap();
        assert_eq!(body, "# my edits\n");
    }

    #[test]
    fn skeleton_parses_as_default_reconconfig() {
        // Everything's commented, so TOML treats it as an empty document.
        let parsed: crate::config::ReconConfig = toml::from_str(SKELETON)
            .expect("skeleton must parse as ReconConfig");
        // All sections None / empty.
        assert!(parsed.editor.is_none());
        assert!(parsed.netstatus.is_none());
        assert!(parsed.sampledata.is_empty());
    }
}
