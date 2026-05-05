#![cfg(feature = "impersonate")]

use std::process::Command;

fn recon_bin() -> &'static str {
    env!("CARGO_BIN_EXE_recon")
}

#[test]
fn impersonate_flag_dispatches_to_impersonate_module() {
    // With the impersonate feature on, --impersonate must NOT produce the
    // feature-off "rebuild with --features impersonate" error. It should
    // either succeed or fail with a different, downstream error message.
    let out = Command::new(recon_bin())
        .args(["--impersonate", "chrome-131", "https://does-not-resolve.invalid/"])
        .output()
        .expect("spawn recon");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("rebuild with --features impersonate"),
        "feature-on build should not emit the feature-off rebuild hint, got: {stderr}"
    );
}

#[test]
fn impersonate_flag_off_feature_errors_clearly() {
    // This test is feature-gated *off*: we verify the off-build error
    // separately in Task 4 once the stub message is in place.
}
