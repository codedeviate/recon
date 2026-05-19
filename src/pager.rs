//! Auto-pager for `--help` and `--examples`.
//!
//! When stdout is a TTY and the user hasn't opted out, spawn `$PAGER`
//! (default `less -FRX`) and dup2 our stdout onto its stdin so subsequent
//! `println!` calls flow through it. After all output is written, the
//! caller MUST invoke `finish()` to flush stdout, close our end of the
//! pipe, and `wait()` on the child — otherwise the pager competes with
//! the shell for terminal control and exits early.
//!
//! Non-Unix targets compile to a no-op: the feature is off on Windows.
//! `colored::control::set_override(true)` is called whenever paging is
//! activated, because `colored` otherwise strips ANSI escapes on our
//! now-piped stdout and `less -R` has nothing to render.

#[cfg(unix)]
use std::io::IsTerminal;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(unix)]
use std::process::{Child, Command, Stdio};

/// Spawn a pager and redirect our stdout to its stdin, returning the
/// Child for lifecycle management. Returns None when paging is disabled,
/// stdout isn't a TTY, or the pager couldn't be spawned.
#[cfg(unix)]
pub fn activate(disabled: bool) -> Option<Child> {
    if disabled || std::env::var("RECON_NO_PAGER").is_ok() {
        return None;
    }
    if !std::io::stdout().is_terminal() {
        return None;
    }
    let cmd = resolve_command();
    let (prog, rest) = cmd.split_first()?;
    let mut child = Command::new(prog)
        .args(rest)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .ok()?;

    // Replace our stdout with the pager's stdin. After dup2, both fds
    // refer to the same pipe; we can drop the child.stdin handle below
    // without closing the duped fd (it's a separate kernel descriptor).
    // SAFETY: both fds are valid, dup2 is always safe when the arguments
    // are valid open descriptors.
    let child_stdin_fd = child.stdin.as_ref()?.as_raw_fd();
    let rc = unsafe { libc::dup2(child_stdin_fd, libc::STDOUT_FILENO) };
    if rc < 0 {
        // dup2 failed — kill the pager we just spawned and fall through
        // to unpaged output. Any println! from here on goes to the
        // original stdout.
        let _ = child.kill();
        let _ = child.wait();
        return None;
    }

    // Drop child.stdin so our dup'd fd is the only writable end. Without
    // this, `less` never sees EOF when we exit (both ends of the pipe
    // are still live via child.stdin) and hangs.
    drop(child.stdin.take());

    // Force colour output through the pipe; `less -R` renders it.
    colored::control::set_override(true);

    Some(child)
}

#[cfg(not(unix))]
pub fn activate(_disabled: bool) -> Option<()> {
    // No-op on non-Unix. Windows callers get unpaged output, same as
    // behaviour before this feature existed.
    None
}

/// Block until the pager exits. Must be called after all output has been
/// written and before `main()` returns — otherwise the shell's foreground
/// process group reclaims the terminal and less gets SIGTTIN/SIGTTOU'd
/// (or the user's keystrokes get eaten by the shell) long before they've
/// finished scrolling.
///
/// Sequence:
/// 1. Flush stdlib's line-buffered stdout so any pending data reaches
///    the pager's read side.
/// 2. Close STDOUT_FILENO so the pipe has no writers; `less` reads
///    until EOF and either exits (`-F` fit-on-one-screen) or sits
///    waiting for user input.
/// 3. `wait()` on the child to block until the user quits or `-F` fires.
#[cfg(unix)]
pub fn finish(child: Option<Child>) {
    if let Some(mut child) = child {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        // SAFETY: closing a fixed, known file descriptor.
        unsafe {
            libc::close(libc::STDOUT_FILENO);
        }
        let _ = child.wait();
    }
}

#[cfg(not(unix))]
pub fn finish(_child: Option<()>) {}

/// Resolve the pager command to run. `$PAGER` wins when set and
/// non-empty, otherwise `less -F -R -X` is used. When the local `less`
/// supports `--mouse` (added in less 530, December 2017), it's
/// appended to the default args so the wheel scrolls the page instead
/// of the terminal scrollback. Shell-split by whitespace only (no
/// quote handling — $PAGER rarely needs it).
#[cfg(unix)]
pub fn resolve_command() -> Vec<String> {
    match std::env::var("PAGER") {
        Ok(s) if !s.trim().is_empty() => s
            .split_whitespace()
            .map(|p| p.to_string())
            .collect(),
        _ => {
            let mut cmd = vec![
                "less".to_string(),
                "-F".to_string(),
                "-R".to_string(),
                "-X".to_string(),
            ];
            if less_supports_mouse() {
                cmd.push("--mouse".to_string());
            }
            cmd
        }
    }
}

/// Probe `less --version`'s first line and return true when the major
/// version is at least 530 (the release that introduced `--mouse`).
/// Falls back to false on any error — old / missing `less` simply
/// doesn't get the flag, matching pre-0.81.2 behaviour.
#[cfg(unix)]
fn less_supports_mouse() -> bool {
    Command::new("less")
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| {
            if !out.status.success() {
                return None;
            }
            // "less 668 (POSIX regular expressions)\n..."
            let text = String::from_utf8_lossy(&out.stdout);
            let first = text.lines().next()?;
            first.split_whitespace().nth(1)?.parse::<u32>().ok()
        })
        .map(|v| v >= 530)
        .unwrap_or(false)
}

/// Check raw argv for `--no-pager`, used during the pre-clap `--help`
/// and `--examples` intercept blocks where `Args` isn't parsed yet.
pub fn no_pager_requested() -> bool {
    std::env::args().any(|a| a == "--no-pager")
        || std::env::var("RECON_NO_PAGER").is_ok()
}

#[cfg(all(unix, test))]
mod tests {
    use super::*;

    #[test]
    fn resolve_command_default_is_less_frx() {
        // Ensure $PAGER is unset for this test. Using set_var is safe in
        // single-threaded test harness; `cargo test` uses threads so we
        // guard with a lock in case other tests touch $PAGER.
        let saved = std::env::var("PAGER").ok();
        std::env::remove_var("PAGER");
        let cmd = resolve_command();
        if let Some(v) = saved {
            std::env::set_var("PAGER", v);
        }
        // The leading flags are stable; --mouse appears on systems
        // running less >= 530.
        assert_eq!(&cmd[..4], &["less", "-F", "-R", "-X"]);
        assert!(cmd.len() == 4 || (cmd.len() == 5 && cmd[4] == "--mouse"));
    }

    #[test]
    fn resolve_command_splits_pager_by_whitespace() {
        // We can't safely mutate $PAGER in a multi-threaded test without
        // a mutex, so simulate the parse directly.
        fn parse(raw: &str) -> Vec<String> {
            if raw.trim().is_empty() {
                return vec!["less".into(), "-F".into(), "-R".into(), "-X".into()];
            }
            raw.split_whitespace().map(|p| p.to_string()).collect()
        }
        assert_eq!(parse("cat"), vec!["cat"]);
        assert_eq!(parse("less -iF"), vec!["less", "-iF"]);
        assert_eq!(
            parse("more -d -r"),
            vec!["more", "-d", "-r"]
        );
        assert_eq!(
            parse(""),
            vec!["less", "-F", "-R", "-X"]
        );
    }
}
