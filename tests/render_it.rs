//! End-to-end tests for HTML → text rendering: the `--html-to-text`
//! transform mode (file + stdin) and the `--render` response hook.

use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_recon");

#[test]
fn html_to_text_from_file() {
    let out = Command::new(BIN)
        .arg("--html-to-text")
        .arg("tests/fixtures/render_sample.html")
        .arg("--width").arg("70")
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Greeting"), "heading missing: {text}");
    assert!(text.contains("the docs"), "anchor text missing: {text}");
    assert!(text.contains("https://example.com/docs"), "footnote url missing: {text}");
    assert!(text.contains("first") && text.contains("second"), "list missing: {text}");
}

#[test]
fn html_to_text_from_stdin() {
    use std::io::Write;
    let mut child = Command::new(BIN)
        .arg("--html-to-text").arg("-")
        .arg("--width").arg("70")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap()
        .write_all(b"<h1>Piped</h1><p>body text</p>").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Piped") && text.contains("body text"), "out: {text}");
}

#[test]
fn html_to_text_writes_to_output_file() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("recon_render_test_{}.txt", std::process::id()));
    let _ = std::fs::remove_file(&path);

    let out = Command::new(BIN)
        .arg("--html-to-text")
        .arg("tests/fixtures/render_sample.html")
        .arg("--width").arg("70")
        .arg("-o").arg(&path)
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let written = std::fs::read_to_string(&path).expect("output file should exist");
    assert!(written.contains("Greeting"), "file content missing heading: {written}");
    assert!(written.contains("https://example.com/docs"), "file content missing footnote url: {written}");

    let _ = std::fs::remove_file(&path);
}
