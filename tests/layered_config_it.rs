//! End-to-end tests for the layered config resolver. Spawn the release
//! binary with `RECON_SYSTEM_CONFIG` / `RECON_CONFIG` set and assert
//! the binary picks them up via --show-config-paths.

use std::process::Command;
use tempfile::TempDir;

fn recon() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_recon"));
    // Strip env vars that the user's shell may have set so tests are
    // deterministic.
    c.env_remove("RECON_CONFIG");
    c.env_remove("RECON_SYSTEM_CONFIG");
    c.env_remove("HOMEBREW_PREFIX");
    c
}

#[test]
fn show_config_paths_with_user_override() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(&cfg, "").unwrap();

    let out = recon()
        .env("RECON_CONFIG", &cfg)
        .arg("--show-config-paths")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(
        stdout.contains(&format!("user: {}", cfg.display())),
        "stdout:\n{stdout}",
    );
}

#[test]
fn show_config_paths_with_no_user_config_flag_skips() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(&cfg, "").unwrap();

    let out = recon()
        .env("RECON_CONFIG", &cfg)
        .args(["--show-config-paths", "--no-user-config"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("user: (skipped)"), "stdout:\n{stdout}");
}

#[test]
fn show_config_paths_with_disable_skips_both() {
    // Note: in this codebase the "skip both layers" flag is --disable / -q,
    // not --no-config (which doesn't exist as a long form).
    let out = recon()
        .args(["--show-config-paths", "--disable"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("system: (skipped)"), "stdout:\n{stdout}");
    assert!(stdout.contains("user: (skipped)"),   "stdout:\n{stdout}");
}

#[test]
fn gh_accounts_in_user_config_is_reachable_via_show_paths() {
    // We can't trivially drive the gh binding from e2e because gh needs
    // to be installed and authenticated. Instead we verify that the
    // resolver loads a user config containing [gh.accounts] without
    // error — that's the data path account_handle_for_email walks
    // (the unit test in src/script/bindings/gh.rs::tests already
    // covers the [gh.accounts]-reachable-via-toml path; this one
    // confirms the binary plumbs the env var to the resolver
    // end-to-end).
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(
        &cfg,
        r#"[gh.accounts]
"e2e@test.example" = "e2e-handle"
"#,
    )
    .unwrap();

    let out = recon()
        .env("RECON_CONFIG", &cfg)
        .arg("--show-config-paths")
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&format!("user: {}", cfg.display())),
        "stdout:\n{stdout}",
    );
}
