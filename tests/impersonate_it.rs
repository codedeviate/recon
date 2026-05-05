#![cfg(feature = "impersonate")]

use std::process::Command;

fn recon_bin() -> &'static str {
    env!("CARGO_BIN_EXE_recon")
}

#[test]
fn impersonate_chrome_succeeds() {
    // Hits httpbin.org/headers and asserts the request succeeded with a
    // Chrome-shaped User-Agent. Skips quietly if network is unavailable.
    let out = Command::new(recon_bin())
        .args(["--impersonate", "chrome_131", "--silent", "https://httpbin.org/headers"])
        .output()
        .expect("spawn recon");
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.contains("dns") || stderr.contains("resolve") || stderr.contains("connect") {
            eprintln!("network unavailable, skipping body assertion: {stderr}");
            return;
        }
        panic!("recon failed: stderr={stderr}");
    }
    let body = String::from_utf8_lossy(&out.stdout);
    assert!(body.contains("Chrome"), "expected Chrome in User-Agent, got body: {body}");
}

#[test]
fn impersonate_accepts_hyphenated_profile_name() {
    // chrome-131 (with hyphen) should be accepted by the normalizer just like chrome_131.
    let out = Command::new(recon_bin())
        .args(["--impersonate", "chrome-131", "--silent", "https://httpbin.org/headers"])
        .output()
        .expect("spawn recon");
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        // Acceptable failure: network problem. Unacceptable: "unknown impersonate profile".
        assert!(
            !stderr.contains("unknown impersonate profile"),
            "hyphenated form rejected: {stderr}"
        );
    }
}

#[test]
fn invalid_profile_name_errors_clearly() {
    let out = Command::new(recon_bin())
        .args(["--impersonate", "not-a-real-browser", "https://example.com/"])
        .output()
        .expect("spawn recon");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown impersonate profile"),
        "expected 'unknown impersonate profile' in stderr, got: {stderr}"
    );
}
