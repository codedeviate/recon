//! Integration tests for --from-clipboard / --to-clipboard / --clipboard.
//!
//! Tests that actually read/write the clipboard are gated on the
//! `RECON_CLIPBOARD_TESTS` env var because CI Linux runners typically
//! lack a display server. Validation tests run unconditionally.

use std::io::Write;
use std::process::{Command, Stdio};

fn recon() -> Command {
    Command::new(env!("CARGO_BIN_EXE_recon"))
}

fn clipboard_tests_enabled() -> bool {
    std::env::var_os("RECON_CLIPBOARD_TESTS").is_some()
}

fn read_clipboard() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

fn write_clipboard(text: &str) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(text.to_string());
    }
}

fn run_with_stdin(args: &[&str], stdin_input: &str) -> (i32, String, String) {
    let mut child = recon()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn recon");
    child.stdin.as_mut().unwrap().write_all(stdin_input.as_bytes()).unwrap();
    let out = child.wait_with_output().expect("recon process failed");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn from_clipboard_to_clipboard_round_trip() {
    if !clipboard_tests_enabled() {
        return;
    }
    let original = read_clipboard();
    let payload = r#"{"x":42}"#;
    write_clipboard(payload);

    let (code, _stdout, stderr) = run_with_stdin(
        &["--from-clipboard", "--to-clipboard", "--prettify-as", "json"],
        "",
    );
    assert_eq!(code, 0, "stderr: {stderr}");

    let result = read_clipboard().expect("read result");
    assert!(result.contains("\"x\": 42"), "result was: {result}");

    if let Some(orig) = original {
        write_clipboard(&orig);
    }
}

#[test]
fn clipboard_both_shortcut() {
    if !clipboard_tests_enabled() {
        return;
    }
    let original = read_clipboard();
    write_clipboard(r#"{"y":1}"#);

    let (code, _stdout, stderr) = run_with_stdin(
        &["--clipboard", "both", "--prettify-as", "json"],
        "",
    );
    assert_eq!(code, 0, "stderr: {stderr}");

    let result = read_clipboard().expect("read result");
    assert!(result.contains("\"y\": 1"), "result was: {result}");

    if let Some(orig) = original {
        write_clipboard(&orig);
    }
}

#[test]
fn clipboard_bare_with_input_resolves_to_out() {
    // --stdin provides input → --clipboard alone should resolve to "out".
    // Validation passes either way; clipboard write may fail in headless env (exit 1, NOT 2).
    let (code, _stdout, stderr) = run_with_stdin(
        &["--stdin", "--clipboard", "-p"],
        r#"{"a":1}"#,
    );
    assert_ne!(code, 2, "validation should pass; got exit 2 with stderr: {stderr}");
}

#[test]
fn stdin_and_from_clipboard_are_mutually_exclusive() {
    let (code, _stdout, stderr) = run_with_stdin(
        &["--stdin", "--from-clipboard", "-p"],
        "{}",
    );
    assert_eq!(code, 2);
    assert!(stderr.contains("--stdin") && stderr.contains("--from-clipboard"),
            "stderr was: {stderr}");
}

#[test]
fn from_clipboard_with_url_is_mutually_exclusive() {
    let (code, _stdout, stderr) = run_with_stdin(
        &["--from-clipboard", "https://example.com"],
        "",
    );
    assert_eq!(code, 2);
    assert!(stderr.contains("URL") || stderr.contains("from-clipboard"),
            "stderr was: {stderr}");
}

#[test]
fn to_clipboard_and_output_are_mutually_exclusive() {
    let (code, _stdout, stderr) = run_with_stdin(
        &["--stdin", "--to-clipboard", "-o", "/tmp/recon-x.json", "-p"],
        "{}",
    );
    assert_eq!(code, 2);
    assert!(stderr.contains("--to-clipboard") && stderr.contains("--output"),
            "stderr was: {stderr}");
}

#[test]
fn unknown_clipboard_dir_errors() {
    let (code, _stdout, stderr) = run_with_stdin(
        &["--clipboard", "bogus", "-p"],
        "{}",
    );
    assert_eq!(code, 2);
    assert!(stderr.contains("bogus") && stderr.contains("in|out|both"),
            "stderr was: {stderr}");
}
