//! `gh()` / `gh(repo_spec)` — Rhai-side handle that composes `gh` CLI
//! invocations.
//!
//! Like the `Git` wrapper, methods like `h.pr_list()` / `h.release_view()`
//! pick the right `--json …` fields internally and return parsed Maps.
//! The `.run()` / `.run_text()` / `.run_json()` escape hatches expose
//! anything not promoted.
//!
//! Auto-account-switch: before every `gh` call, the wrapper checks
//! `git config user.email` against the user's CLAUDE.md mapping
//! (codedv8@gmail.com → codedeviate, thomas.bjork@starweb.se → starweb-thomas)
//! and runs `gh auth switch --user <handle>` if the active account
//! doesn't match. Opt out per call via `#{ auto_switch_account: false }`.
//!
//! Errors throw on non-zero exit (matching `git`). Scripts use
//! `try` / `catch` to recover — especially relevant for `gh pr view`
//! which exits 1 for "not found".

#[allow(unused_imports)]
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use crate::script::convert::err;

const STDERR_CAP: usize = 2048;

#[derive(Clone)]
struct Gh {
    repo: Option<String>,
    // Cached auth-switch state: Some(handle) when we last switched to
    // that handle. Behind an Arc<Mutex<>> so Gh stays Clone + Send + Sync
    // and the cache survives across clones (Rhai may clone the Gh
    // value when scripts assign or pass it).
    switched_to: Arc<Mutex<Option<String>>>,
}

impl Gh {
    fn new(repo: Option<String>) -> Self {
        Gh {
            repo,
            switched_to: Arc::new(Mutex::new(None)),
        }
    }

    fn run(&self, args: &[&str], auto_switch: bool) -> Result<Output, Box<EvalAltResult>> {
        if auto_switch {
            self.ensure_account()?;
        }
        // Compose argv: args + --repo <spec> if set.
        let mut owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        if let Some(repo) = &self.repo {
            owned.push("--repo".into());
            owned.push(repo.clone());
        }
        let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
        let mut cmd = Command::new("gh");
        cmd.args(&refs);
        cmd.output().map_err(|e| {
            err(format!(
                "gh: failed to spawn `gh {}`: {e}",
                refs.join(" ")
            ))
        })
    }

    fn ensure_account(&self) -> Result<(), Box<EvalAltResult>> {
        let email = match read_git_email() {
            Some(e) => e,
            None => return Ok(()), // No email configured; let gh use its default.
        };
        let expected = match account_handle_for_email(&email) {
            Some(h) => h,
            None => return Ok(()), // Unknown email; fall back to current gh account.
        };
        // Cache check.
        {
            let cache = self.switched_to.lock().unwrap();
            if cache.as_deref() == Some(expected) {
                return Ok(());
            }
        }
        // Switch.
        let switch_result = Command::new("gh")
            .args(["auth", "switch", "--user", expected])
            .output()
            .map_err(|e| err(format!("gh: failed to invoke `gh auth switch`: {e}")))?;
        if !switch_result.status.success() {
            // Don't hard-fail — some scripts may not need a switch
            // (e.g. only-public-repo work). Log on stderr and continue.
            eprintln!(
                "gh: warning: auto-switch to '{expected}' failed: {}",
                String::from_utf8_lossy(&switch_result.stderr).trim()
            );
        } else {
            *self.switched_to.lock().unwrap() = Some(expected.to_string());
        }
        Ok(())
    }
}

fn read_git_email() -> Option<String> {
    let out = Command::new("git")
        .args(["config", "user.email"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

pub(crate) fn account_handle_for_email(email: &str) -> Option<&'static str> {
    match email {
        "codedv8@gmail.com" => Some("codedeviate"),
        "thomas.bjork@starweb.se" => Some("starweb-thomas"),
        _ => None,
    }
}

fn ok_or_throw(args: &[&str], output: Output) -> Result<String, Box<EvalAltResult>> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let mut stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if stderr.len() > STDERR_CAP {
            stderr.truncate(STDERR_CAP);
            stderr.push_str("\n…[stderr truncated]");
        }
        let code = output.status.code().unwrap_or(-1);
        Err(err(format!(
            "gh: `gh {}` failed (exit {code}): {}",
            args.join(" "),
            stderr.trim()
        )))
    }
}

/// Local copy of shellwords_split from git.rs — see notes there. The
/// `_local` suffix avoids name collision with git.rs's helper without
/// the friction of extracting a shared module.
fn shellwords_split_local(s: &str) -> Result<Vec<String>, Box<EvalAltResult>> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escape = false;
    for c in s.chars() {
        if escape {
            current.push(c);
            escape = false;
            continue;
        }
        match c {
            '\\' if !in_single => escape = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if in_single || in_double {
        return Err(err("gh: unterminated quoted argument in args"));
    }
    if !current.is_empty() {
        out.push(current);
    }
    Ok(out)
}

fn parse_json_to_dynamic_local(s: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    let v: serde_json::Value = serde_json::from_str(s)
        .map_err(|e| err(format!("gh: run_json: stdout not JSON: {e}")))?;
    Ok(crate::script::bindings::helpers::json_to_dynamic(v))
}

pub fn register(engine: &mut Engine) {
    engine.register_type_with_name::<Gh>("Gh");

    engine.register_fn("gh", || -> Gh { Gh::new(None) });
    engine.register_fn("gh", |repo: &str| -> Gh { Gh::new(Some(repo.to_string())) });

    engine.register_fn(
        "run_text",
        |h: &mut Gh, args: &str| -> Result<String, Box<EvalAltResult>> {
            let argv = shellwords_split_local(args)?;
            let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = h.run(&refs, true)?;
            ok_or_throw(&refs, out)
        },
    );
    engine.register_fn(
        "run_json",
        |h: &mut Gh, args: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            let argv = shellwords_split_local(args)?;
            let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = h.run(&refs, true)?;
            let stdout = ok_or_throw(&refs, out)?;
            parse_json_to_dynamic_local(&stdout)
        },
    );
    engine.register_fn(
        "run",
        |h: &mut Gh, args: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            let argv = shellwords_split_local(args)?;
            let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = h.run(&refs, true)?;
            let stdout = ok_or_throw(&refs, out)?;
            let trimmed = stdout.trim_start();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                if let Ok(v) = parse_json_to_dynamic_local(&stdout) {
                    return Ok(v);
                }
            }
            Ok(Dynamic::from(stdout))
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::{Dynamic, Engine};

    fn engine() -> Engine {
        let mut e = Engine::new();
        register(&mut e);
        e
    }

    #[test]
    fn gh_constructor_returns_gh_type() {
        let mut e = engine();
        let _: Dynamic = e.eval(r#"gh()"#).unwrap();
        let _: Dynamic = e.eval(r#"gh("codedeviate/recon")"#).unwrap();
    }

    #[test]
    fn account_handle_for_email_maps_known_emails() {
        assert_eq!(account_handle_for_email("codedv8@gmail.com"), Some("codedeviate"));
        assert_eq!(
            account_handle_for_email("thomas.bjork@starweb.se"),
            Some("starweb-thomas"),
        );
        assert_eq!(account_handle_for_email("unknown@example.com"), None);
    }

    #[test]
    fn gh_run_text_function_registered() {
        // We can't actually call `gh` in a unit test (auth requirements).
        // Confirm registration by checking the engine has the function
        // and that `gh --version` (works without auth) returns sensibly.
        // Skip if gh isn't on PATH (CI may not have it).
        if std::process::Command::new("gh").arg("--version").output().is_err() {
            return; // gh not available; skip.
        }
        let mut e = engine();
        let s: String = e
            .eval(r#"gh().run_text("--version")"#)
            .unwrap();
        assert!(s.to_lowercase().contains("gh"), "got: {s}");
    }
}
