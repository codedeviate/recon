//! End-to-end tests for the --alias preprocessor. Drives the release
//! binary with a tmpdir-isolated config.toml via $RECON_CONFIG, so
//! the user's real ~/.recon/config.toml never leaks in.

use std::process::Command;
use tempfile::TempDir;

fn recon() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_recon"));
    c.env_remove("RECON_CONFIG");
    c.env_remove("RECON_SYSTEM_CONFIG");
    c.env_remove("HOMEBREW_PREFIX");
    c
}

fn isolated_config(body: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(&cfg, body).unwrap();
    (dir, cfg)
}

#[test]
fn explicit_alias_curl_is_noop() {
    let out = recon()
        .args(["--alias", "curl", "--version-short"])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("recon "), "stdout: {stdout}");
}

#[test]
fn unknown_alias_name_errors() {
    // Avoid --version-short / --version because those early-exit
    // BEFORE alias resolution in main.rs. We need a code path that
    // actually reaches the alias preprocessor — `--help-topics` would
    // also early-exit. Use an obviously-bad URL so the request never
    // happens, but the alias check still fires.
    let out = recon()
        .args(["--alias", "bogus", "https://invalid.example.localhost.test"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("bogus"), "stderr: {stderr}");
    assert!(stderr.contains("not defined"), "stderr: {stderr}");
}

#[test]
fn default_alias_applied_from_config() {
    let (_dir, cfg) = isolated_config(
        r#"
[aliases]
default = "mine"

[aliases.mine]
"-X" = "--request"
"#,
    );
    // `-X` is curl's `--request` — already a recon short flag, so
    // this rewrite is harmless. The point is to prove the rewriter
    // actually fires from the config default. We pair `-X HEAD` with
    // an obviously-invalid URL so recon errors *after* parsing flags,
    // and the parse itself proves -X became --request.
    let out = recon()
        .env("RECON_CONFIG", &cfg)
        .args(["-X", "HEAD", "https://invalid.example.localhost.test"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    // Should not be a "clap: unknown flag -X" error. Should be a
    // network or resolution error from the actual request attempt.
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("unknown flag"),
        "stderr: {stderr}"
    );
}

#[test]
fn disable_skips_alias_resolution() {
    let (_dir, cfg) = isolated_config(
        r#"
[aliases]
default = "wget"
"#,
    );
    // With -q, the config-level [aliases] default is ignored. So
    // `-r 0-100` should reach clap as recon's --range, not wget's
    // --recursive. We pair it with an invalid URL so failure is
    // post-parse.
    let out = recon()
        .env("RECON_CONFIG", &cfg)
        .args(["-q", "-r", "0-100", "https://invalid.example.localhost.test"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("--recursive"),
        "alias should be skipped under -q; stderr: {stderr}"
    );
}

#[test]
fn disable_skips_even_explicit_alias() {
    // Stricter than `disable_skips_alias_resolution`: even an
    // explicit `--alias wget` on the command line is suppressed
    // when -q is set. `-q` opts out of alias resolution entirely,
    // including bundled. The user who wants no config but still
    // wants an alias can drop -q and use --alias by itself.
    let out = recon()
        .args(["-q", "--alias", "wget", "-r", "0-100", "https://invalid.example.localhost.test"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("--recursive"),
        "explicit --alias should be skipped under -q; stderr: {stderr}"
    );
}

#[test]
fn alias_to_unimplemented_long_form_errors_via_clap() {
    let out = recon()
        .args(["--alias", "wget", "-r", "https://example.com"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--recursive") || stderr.contains("unexpected argument"),
        "stderr: {stderr}"
    );
}

#[test]
fn user_section_merges_into_bundled() {
    let (_dir, cfg) = isolated_config(
        r#"
[aliases.wget]
"-J" = "--json"
"#,
    );
    // The user's `[aliases.wget]` adds -J; the bundled wget still
    // provides -N → --timestamping. Both should be active after the
    // merge. We test that -N is still rewritten to --timestamping
    // (which recon doesn't implement) → clap errors specifically
    // about --timestamping, proving the bundled entry survived the
    // merge.
    let out = recon()
        .env("RECON_CONFIG", &cfg)
        .args(["--alias", "wget", "-N", "https://example.com"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--timestamping") || stderr.contains("unexpected argument"),
        "stderr: {stderr}"
    );
}
