//! Integration test for `recon --repl`. Drives a scripted session via
//! stdin and asserts on stdout fragments.

use std::process::{Command, Stdio};
use std::io::Write;

fn recon_binary() -> &'static str {
    // The release binary is built before the test suite runs.
    "./target/release/recon"
}

fn run_session(stdin: &str) -> (String, String, i32) {
    // Each test gets its own history file via a thread/test-name suffix
    // to avoid concurrent-write races when `cargo test` runs tests in
    // parallel. `std::process::id()` is identical across all tests in
    // the same process, so we add a counter.
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let history = format!("/tmp/recon-repl-it-{}-{}.history",
        std::process::id(), n);

    let mut child = Command::new(recon_binary())
        .arg("--repl")
        .arg("--repl-history")
        .arg(&history)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn recon");
    child.stdin.as_mut().unwrap().write_all(stdin.as_bytes()).unwrap();
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = std::fs::remove_file(&history);
    (stdout, stderr, out.status.code().unwrap_or(-1))
}

#[test]
fn basic_expression_and_binding_persist() {
    let (stdout, _stderr, code) = run_session(
        "let x = 6 * 7\nx\n:quit\n"
    );
    assert_eq!(code, 0);
    assert!(stdout.contains("42"), "stdout was: {stdout}");
}

#[test]
fn user_fn_persists_across_lines() {
    let (stdout, _stderr, code) = run_session(
        "fn greet(n) { \"hi \" + n }\ngreet(\"world\")\n:quit\n"
    );
    assert_eq!(code, 0);
    assert!(stdout.contains("\"hi world\""), "stdout was: {stdout}");
}

#[test]
fn vars_lists_user_bindings() {
    let (stdout, stderr, code) = run_session(
        "let answer = 42\n:vars\n:quit\n"
    );
    assert_eq!(code, 0);
    // :vars output goes to stdout
    let combined = format!("{stdout}{stderr}");
    assert!(combined.contains("answer"), "combined was: {combined}");
    assert!(combined.contains("42"), "combined was: {combined}");
}

#[test]
fn threading_stub_errors_with_message() {
    let (_stdout, stderr, code) = run_session(
        "thread_spawn(|| { 1 })\n:quit\n"
    );
    assert_eq!(code, 0);
    assert!(
        stderr.contains("not available in REPL mode"),
        "stderr was: {stderr}"
    );
}

#[test]
fn unknown_meta_command_errors_but_continues() {
    let (stdout, stderr, code) = run_session(
        ":nosuch\n1+1\n:quit\n"
    );
    assert_eq!(code, 0);
    assert!(stderr.contains("unknown command"), "stderr was: {stderr}");
    // REPL must continue after the error and evaluate the next expression
    assert!(stdout.contains("2"), "stdout was: {stdout}");
}

#[test]
fn quit_exits_zero() {
    let (_stdout, _stderr, code) = run_session(":quit\n");
    assert_eq!(code, 0);
}
