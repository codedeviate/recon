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
fn raw_overrides_error_as_not_yet_implemented() {
    // --ja3 / --ja4 / --http2-fingerprint are accepted by clap (so --help and
    // --flags stay stable) but error out at runtime as not-yet-implemented.
    // (Previously this test pinned to a "v0.78" version target; 0.78–0.80
    // shipped without these so the message moved to a version-agnostic form.)
    for flag in ["--ja3", "--ja4", "--http2-fingerprint"] {
        let out = Command::new(recon_bin())
            .args([flag, "dummy-value", "https://example.com/"])
            .output()
            .expect("spawn recon");
        assert!(!out.status.success(), "{flag} unexpectedly succeeded");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("not implemented yet"),
            "{flag}: expected not-yet-implemented message, got: {stderr}"
        );
    }
}

#[test]
fn validate_combination_errors_survive_friendly_message_filter() {
    // Regression: main.rs::friendly_message rewrites any error containing
    // "TLS" or "certificate" to a generic placeholder, which would hide the
    // helpful "this flag combination is not supported" message. Verify the
    // actual message reaches the user for each incompatible pair.
    let cases = [
        (vec!["--impersonate", "chrome_131", "--tlsv1.3"], "fingerprint impersonation"),
        (vec!["--impersonate", "chrome_131", "--tlsv1.2"], "fingerprint impersonation"),
        (vec!["--impersonate", "chrome_131", "--cacert", "/dev/null"], "fingerprint impersonation"),
    ];
    for (flags, expected_substring) in cases {
        let mut argv = flags;
        argv.push("https://example.com/");
        let out = Command::new(recon_bin())
            .args(&argv)
            .output()
            .expect("spawn recon");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(expected_substring),
            "args {argv:?}: expected '{expected_substring}' in stderr, got: {stderr}"
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
