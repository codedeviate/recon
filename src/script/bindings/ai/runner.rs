//! Subprocess runner for `ai::*` backends. No async runtime — uses
//! `std::thread` + an `mpsc` channel with `recv_timeout` to enforce
//! the wall-clock kill switch.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct RunResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration: Duration,
}

#[derive(Debug)]
pub enum RunError {
    Spawn(std::io::Error),
    Timeout(Duration),
    NonZeroExit { code: i32, stderr: String },
    EmptyStdout,
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::Spawn(e) => write!(f, "ai: spawn failed: {e}"),
            RunError::Timeout(d) => write!(f, "ai: timed out after {}s", d.as_secs()),
            RunError::NonZeroExit { code, stderr } => {
                let tail = tail_lines(stderr, 8);
                write!(f, "ai: CLI exited with status {code}:\n{tail}")
            }
            RunError::EmptyStdout => write!(f, "ai: empty response from backend"),
        }
    }
}

impl std::error::Error for RunError {}

fn tail_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

/// Spawn `argv[0]` with `argv[1..]`, pipe `stdin_payload` to its stdin,
/// wait up to `timeout`, capture stdout / stderr, return the result.
///
/// Errors:
/// - Spawn failure (CLI not on PATH, perms) → `Spawn`.
/// - Timeout elapsed before exit → child killed → `Timeout`.
/// - Non-zero exit → `NonZeroExit` with stderr tail.
/// - Exit 0 but empty stdout → `EmptyStdout`.
pub fn run(argv: &[String], stdin_payload: &str, timeout: Duration) -> Result<RunResult, RunError> {
    if argv.is_empty() {
        return Err(RunError::Spawn(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "empty argv",
        )));
    }
    let started = Instant::now();
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(RunError::Spawn)?;

    // Pipe stdin in a thread so a CLI that doesn't drain it fast doesn't
    // deadlock us. Errors here are non-fatal: the child may have exited
    // early, in which case we still want to read stdout / stderr.
    if let Some(mut stdin) = child.stdin.take() {
        let payload = stdin_payload.to_owned();
        thread::spawn(move || {
            let _ = stdin.write_all(payload.as_bytes());
            let _ = stdin.flush();
        });
    }

    // Wait for exit in a background thread, signalling via mpsc so the
    // main thread can apply a timeout.
    let (tx, rx) = mpsc::channel::<std::io::Result<std::process::Output>>();
    let child_id = child.id();
    thread::spawn(move || {
        let out = child.wait_with_output();
        let _ = tx.send(out);
    });

    let output = match rx.recv_timeout(timeout) {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return Err(RunError::Spawn(e)),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // Kill the process group (best effort — use platform-specific kill).
            #[cfg(unix)]
            {
                let _ = std::process::Command::new("kill")
                    .arg("-TERM")
                    .arg(child_id.to_string())
                    .status();
            }
            #[cfg(not(unix))]
            {
                let _ = std::process::Command::new("taskkill")
                    .args(["/PID", &child_id.to_string(), "/F"])
                    .status();
            }
            return Err(RunError::Timeout(timeout));
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            return Err(RunError::Spawn(std::io::Error::other(
                "child wait thread disconnected",
            )));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    let duration = started.elapsed();

    if !output.status.success() {
        return Err(RunError::NonZeroExit {
            code: exit_code,
            stderr,
        });
    }
    if stdout.trim().is_empty() {
        return Err(RunError::EmptyStdout);
    }

    Ok(RunResult { stdout, stderr, exit_code, duration })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(script: &str) -> Vec<String> {
        vec!["sh".to_string(), "-c".to_string(), script.to_string()]
    }

    #[test]
    fn echoes_stdin_via_cat() {
        let argv = vec!["cat".to_string()];
        let r = run(&argv, "hello world", Duration::from_secs(5)).unwrap();
        assert_eq!(r.stdout.trim(), "hello world");
        assert_eq!(r.exit_code, 0);
    }

    #[test]
    fn non_zero_exit_captures_stderr() {
        let argv = sh("echo bad >&2; exit 2");
        let err = run(&argv, "", Duration::from_secs(5)).unwrap_err();
        match err {
            RunError::NonZeroExit { code, stderr } => {
                assert_eq!(code, 2);
                assert!(stderr.contains("bad"));
            }
            other => panic!("expected NonZeroExit, got {other:?}"),
        }
    }

    #[test]
    fn timeout_kills_long_process() {
        let argv = sh("sleep 5; echo ignored");
        let started = Instant::now();
        let err = run(&argv, "", Duration::from_millis(300)).unwrap_err();
        assert!(matches!(err, RunError::Timeout(_)), "got: {err:?}");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "timeout took too long: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn empty_stdout_errors() {
        let argv = sh("exit 0");
        let err = run(&argv, "", Duration::from_secs(5)).unwrap_err();
        assert!(matches!(err, RunError::EmptyStdout), "got: {err:?}");
    }

    #[test]
    fn missing_binary_errors_spawn() {
        let argv = vec!["this-binary-does-not-exist-xyz".to_string()];
        let err = run(&argv, "", Duration::from_secs(5)).unwrap_err();
        assert!(matches!(err, RunError::Spawn(_)), "got: {err:?}");
    }

    #[test]
    fn empty_argv_errors() {
        let argv: Vec<String> = vec![];
        let err = run(&argv, "", Duration::from_secs(5)).unwrap_err();
        assert!(matches!(err, RunError::Spawn(_)), "got: {err:?}");
    }

    #[test]
    fn stderr_tail_trims_to_last_n_lines() {
        let many = (1..=20).map(|i| format!("line{i}")).collect::<Vec<_>>().join("\n");
        let trimmed = tail_lines(&many, 3);
        assert_eq!(trimmed, "line18\nline19\nline20");
    }
}
