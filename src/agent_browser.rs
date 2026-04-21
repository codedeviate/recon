//! Shared agent-browser integration. Holds the availability probe, a
//! single `run_cmd` helper that both the Rhai binding and the CLI flag
//! use, and the `--browser-screenshot` CLI flow.
//!
//! We wrap the external CLI rather than linking a browser driver so the
//! dep surface stays the same — scripts without agent-browser installed
//! still load cleanly, they just see `agentBrowser::available == false`.

use anyhow::{anyhow, Context, Result};
use std::process::Command;
use std::sync::OnceLock;

/// Whether the agent-browser binary is reachable + its version string.
#[derive(Clone, Debug)]
pub struct AgentBrowserState {
    pub available: bool,
    /// e.g. "0.26.0" (parsed from `agent-browser --version` stdout). Empty
    /// when `available` is false.
    pub version: String,
}

/// Detected once at first access. Subsequent calls return the cached
/// value — script sessions don't change PATH mid-run.
fn state() -> &'static AgentBrowserState {
    static CELL: OnceLock<AgentBrowserState> = OnceLock::new();
    CELL.get_or_init(detect_state)
}

pub fn state_snapshot() -> AgentBrowserState {
    state().clone()
}

fn detect_state() -> AgentBrowserState {
    match Command::new("agent-browser").arg("--version").output() {
        Ok(out) if out.status.success() => {
            // stdout like "agent-browser 0.26.0\n"
            let text = String::from_utf8_lossy(&out.stdout);
            let version = text
                .split_whitespace()
                .nth(1)
                .unwrap_or("")
                .trim_matches(|c: char| !c.is_ascii_digit() && c != '.')
                .to_string();
            AgentBrowserState {
                available: true,
                version,
            }
        }
        _ => AgentBrowserState {
            available: false,
            version: String::new(),
        },
    }
}

/// Run `agent-browser <args...>`. When `json` is true, sets
/// `AGENT_BROWSER_JSON=1` so structured commands emit parseable output.
/// Returns stdout as UTF-8 lossy. Non-zero exit → Err with stderr text.
/// Missing binary → a specialised error with clear remediation hint.
pub fn run_cmd(args: &[&str], json: bool) -> Result<String> {
    let mut cmd = Command::new("agent-browser");
    cmd.args(args);
    if json {
        cmd.env("AGENT_BROWSER_JSON", "1");
    }
    let out = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow!(
                "agent-browser: binary not found on PATH. \
                 Install via `brew install agent-browser` or \
                 `npm install -g agent-browser`."
            )
        } else {
            anyhow!("agent-browser: spawn failed: {e}")
        }
    })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let code = out
            .status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());
        return Err(anyhow!(
            "agent-browser: exit {code}: {}",
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Early-intercept handler for `recon --browser-screenshot URL [-o PATH]`.
pub fn run_screenshot_cli(url: &str, output: Option<&std::path::Path>) -> Result<()> {
    let s = state();
    if !s.available {
        return Err(anyhow!(
            "agent-browser: binary not found on PATH. \
             Install via `brew install agent-browser` or \
             `npm install -g agent-browser`."
        ));
    }

    // Open in a single session then screenshot; close on the way out so
    // we don't leak a daemon / browser process for each invocation.
    let _ = run_cmd(&["open", url], false).context("agent-browser: open")?;
    let shot_args: Vec<&str> = match output {
        Some(p) => vec!["screenshot", p.to_str().unwrap_or("")],
        None => vec!["screenshot"],
    };
    let out = run_cmd(&shot_args, false).context("agent-browser: screenshot")?;
    let _ = run_cmd(&["close"], false);
    print!("{out}");
    if !out.ends_with('\n') {
        println!();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_is_deterministic() {
        // Whatever the real PATH is, state() must be callable twice and
        // return the same thing. We don't assert a specific available
        // value — depends on the dev environment.
        let a = state_snapshot();
        let b = state_snapshot();
        assert_eq!(a.available, b.available);
        assert_eq!(a.version, b.version);
    }

    #[test]
    fn version_parsing_trims_nondigits() {
        // detect_state parses whatever agent-browser --version emits.
        // Can't easily mock the Command; this test exists as a guard
        // that the function compiles + runs.
        let s = state_snapshot();
        if s.available {
            assert!(
                s.version
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.'),
                "version '{}' should contain only digits and dots",
                s.version
            );
        } else {
            assert!(s.version.is_empty());
        }
    }
}
