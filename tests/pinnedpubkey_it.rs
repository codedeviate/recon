//! Integration tests for `--pinnedpubkey` / `--curves` (the
//! use_preconfigured_tls custom-rustls path). Network-dependent cases
//! skip quietly when the network is unavailable; the deterministic cases
//! (parse/validation errors) need no network.

use std::process::Command;

fn recon_bin() -> &'static str {
    env!("CARGO_BIN_EXE_recon")
}

/// base64 of 32 zero bytes — a valid-format SHA-256 pin that will never
/// match a real server's public key.
const WRONG_PIN: &str = "sha256//AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

fn is_network_error(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    s.contains("dns") || s.contains("resolve") || s.contains("connect") || s.contains("timed out")
}

#[test]
fn p521_curve_errors_before_network() {
    // P-521 is unavailable under ring; must fail fast with a clear message
    // and never touch the network.
    let out = Command::new(recon_bin())
        .args(["--curves", "P-521", "https://example.com/"])
        .output()
        .expect("spawn recon");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("secp521r1") && stderr.contains("ring"),
        "expected P-521/ring error, got: {stderr}"
    );
}

#[test]
fn unknown_curve_errors() {
    let out = Command::new(recon_bin())
        .args(["--curves", "bananas", "https://example.com/"])
        .output()
        .expect("spawn recon");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown curve"), "got: {stderr}");
}

#[test]
fn malformed_pin_errors_before_network() {
    let out = Command::new(recon_bin())
        .args([
            "--pinnedpubkey",
            "/etc/keys/pub.der",
            "https://example.com/",
        ])
        .output()
        .expect("spawn recon");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("sha256//"),
        "expected sha256// hint, got: {stderr}"
    );
}

#[test]
fn pin_with_client_cert_is_accepted() {
    // The pin + client-cert combination is now supported (the custom TLS
    // path builds client-auth). An empty cert file must therefore fail with
    // a *cert-load* error, NOT the old "cannot be combined" error.
    let out = Command::new(recon_bin())
        .args([
            "--pinnedpubkey",
            WRONG_PIN,
            "--client-cert",
            "/dev/null",
            "https://example.com/",
        ])
        .output()
        .expect("spawn recon");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("cannot be combined"),
        "combine should now be allowed, got: {stderr}"
    );
    assert!(
        stderr.contains("--cert") || stderr.contains("certificate") || stderr.contains("no certificate"),
        "expected a cert-load error, got: {stderr}"
    );
}

#[test]
fn wrong_pin_is_rejected() {
    // Exercises the full custom-rustls path + the use_preconfigured_tls
    // downcast: a non-matching pin must reject the connection. Skips on
    // network failure.
    let out = Command::new(recon_bin())
        .args([
            "--pinnedpubkey",
            WRONG_PIN,
            "--silent",
            "https://example.com/",
        ])
        .output()
        .expect("spawn recon");
    let stderr = String::from_utf8_lossy(&out.stderr);
    if out.status.success() {
        panic!("wrong pin unexpectedly succeeded; stderr={stderr}");
    }
    if is_network_error(&stderr) {
        eprintln!("network unavailable, skipping: {stderr}");
        return;
    }
    assert!(
        stderr.to_ascii_lowercase().contains("pinnedpubkey")
            || stderr.to_ascii_lowercase().contains("public-key"),
        "expected a pin-mismatch error, got: {stderr}"
    );
}

#[test]
fn curves_x25519_connects() {
    // A supported curve must build a working custom config and connect.
    // Skips on network failure; the key assertion is "no parse/build error".
    let out = Command::new(recon_bin())
        .args([
            "--curves",
            "X25519:P-256",
            "--silent",
            "https://example.com/",
        ])
        .output()
        .expect("spawn recon");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("--curves:"), "curves rejected: {stderr}");
    if !out.status.success() && !is_network_error(&stderr) {
        panic!("curves request failed for non-network reason: {stderr}");
    }
}
