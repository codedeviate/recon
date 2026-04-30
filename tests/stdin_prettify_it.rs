//! End-to-end tests for --stdin and --prettify-as. No mock server needed —
//! --stdin runs the post-fetch pipeline over piped bytes, no HTTP request.

use std::io::Write;
use std::process::{Command, Stdio};

fn recon() -> Command {
    Command::new(env!("CARGO_BIN_EXE_recon"))
}

fn run_with_stdin(args: &[&str], stdin_input: &str) -> (i32, String, String) {
    let mut child = recon()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn recon");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin_input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().expect("recon process failed");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (code, stdout, stderr)
}

#[test]
fn stdin_prettify_auto_detects_json() {
    let (code, stdout, stderr) = run_with_stdin(
        &["--stdin", "-p"],
        r#"{"a":1,"b":[2,3]}"#,
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("\"a\": 1"), "stdout was: {stdout}");
    assert!(stdout.contains("\"b\":"), "stdout was: {stdout}");
}

#[test]
fn stdin_prettify_as_json_forces_format() {
    let (code, stdout, stderr) = run_with_stdin(
        &["--stdin", "--prettify-as", "json"],
        r#"{"x":42}"#,
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("\"x\": 42"), "stdout was: {stdout}");
}

#[test]
fn stdin_prettify_as_implies_prettify() {
    // No -p flag, only --prettify-as. Should still pretty-print.
    let (code, stdout, stderr) = run_with_stdin(
        &["--stdin", "--prettify-as", "json"],
        r#"{"y":1}"#,
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("\"y\": 1"), "stdout was: {stdout}");
    assert!(
        stdout.contains('\n'),
        "expected pretty-printed multiline output, got: {stdout}"
    );
}

#[test]
fn stdin_with_invalid_json_and_forced_format_errors() {
    // Strict mode: forced --prettify-as that fails to parse should exit non-zero.
    let (code, _stdout, _stderr) = run_with_stdin(
        &["--stdin", "--prettify-as", "json"],
        "this is not json",
    );
    assert_ne!(code, 0, "expected non-zero exit when forced format fails to parse");
}

#[test]
fn unknown_prettify_as_format_errors_with_code_2() {
    let (code, _stdout, stderr) = run_with_stdin(
        &["--stdin", "--prettify-as", "bogus"],
        "{}",
    );
    assert_eq!(code, 2);
    assert!(stderr.contains("bogus"), "stderr was: {stderr}");
    assert!(stderr.contains("json"), "stderr was: {stderr}");
}

#[test]
fn stdin_with_url_is_mutually_exclusive() {
    let (code, _stdout, stderr) = run_with_stdin(
        &["--stdin", "https://example.com"],
        "{}",
    );
    assert_eq!(code, 2);
    assert!(stderr.contains("--stdin"), "stderr was: {stderr}");
    assert!(stderr.contains("URL"), "stderr was: {stderr}");
}

#[test]
fn empty_stdin_exits_zero_with_empty_output() {
    let (code, stdout, _stderr) = run_with_stdin(&["--stdin", "-p"], "");
    assert_eq!(code, 0);
    assert!(stdout.is_empty() || stdout.trim().is_empty());
}

#[test]
fn stdin_xml_prettify() {
    let (code, stdout, stderr) = run_with_stdin(
        &["--stdin", "--prettify-as", "xml"],
        "<root><a>1</a><b>2</b></root>",
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("<a>"), "stdout was: {stdout}");
    assert!(stdout.contains("<b>"), "stdout was: {stdout}");
}

#[test]
fn stdin_passthrough_without_prettify() {
    // --stdin with no -p, no -o, no charset → raw passthrough.
    let (code, stdout, _stderr) = run_with_stdin(&["--stdin"], "raw text\n");
    assert_eq!(code, 0);
    assert!(stdout.contains("raw text"), "stdout was: {stdout}");
}
