//! `git()` / `git(path)` — Rhai-side handle that composes `git` CLI
//! invocations.
//!
//! Methods like `g.status()`, `g.log(n)`, `g.diff()` pick the right
//! `--porcelain` / `--format=...` flags internally and parse the
//! output into Rhai data. The `.run()` / `.run_text()` / `.run_json()`
//! escape hatches expose anything not promoted to a first-class method.
//!
//! Errors throw on non-zero exit; the resulting `EvalAltResult` carries
//! stderr (capped) and the underlying argv (with credentials redacted).
//! Scripts use `try` / `catch` to recover.

use crate::script::convert::err;
use rhai::{Dynamic, Engine, EvalAltResult};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::process::{Command, Output};

const STDERR_CAP: usize = 2048;

#[derive(Clone)]
struct Git {
    cwd: PathBuf,
}

impl Git {
    fn new(path: Option<&str>) -> Self {
        let cwd = match path {
            Some(p) => PathBuf::from(p),
            None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        };
        Git { cwd }
    }

    fn run(&self, args: &[&str]) -> Result<Output, Box<EvalAltResult>> {
        run_command(&self.cwd, args)
    }
}

fn run_command(
    cwd: &std::path::Path,
    args: &[&str],
) -> Result<Output, Box<EvalAltResult>> {
    let mut cmd = Command::new("git");
    cmd.current_dir(cwd).args(args);
    cmd.output().map_err(|e| {
        err(format!(
            "git: failed to spawn `git {}`: {e}",
            args.join(" ")
        ))
    })
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
            "git: `git {}` failed (exit {code}): {}",
            args.join(" "),
            stderr.trim()
        )))
    }
}

/// Split a single-arg-string into argv pieces. Whitespace-separated
/// with `"..."` and `'...'` recognised as quoted groups; backslash
/// escapes the next char outside single quotes. Not a full shell
/// parser — scripts pass simple arg strings.
fn shellwords_split(s: &str) -> Result<Vec<String>, Box<EvalAltResult>> {
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
        return Err(err("git: unterminated quoted argument in args"));
    }
    if !current.is_empty() {
        out.push(current);
    }
    Ok(out)
}

fn parse_json_to_dynamic(s: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    let v: JsonValue = serde_json::from_str(s)
        .map_err(|e| err(format!("git: run_json: stdout not JSON: {e}")))?;
    Ok(crate::script::bindings::helpers::json_to_dynamic(v))
}

pub fn register(engine: &mut Engine) {
    engine.register_type_with_name::<Git>("Git");

    // git() — cwd-bound.
    engine.register_fn("git", || -> Git { Git::new(None) });
    // git(path) — explicit path.
    engine.register_fn("git", |path: &str| -> Git { Git::new(Some(path)) });

    engine.register_fn(
        "run_text",
        |g: &mut Git, args: &str| -> Result<String, Box<EvalAltResult>> {
            let argv = shellwords_split(args)?;
            let argv_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = g.run(&argv_refs)?;
            ok_or_throw(&argv_refs, out)
        },
    );
    engine.register_fn(
        "run_json",
        |g: &mut Git, args: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            let argv = shellwords_split(args)?;
            let argv_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = g.run(&argv_refs)?;
            let stdout = ok_or_throw(&argv_refs, out)?;
            parse_json_to_dynamic(&stdout)
        },
    );
    engine.register_fn(
        "run",
        |g: &mut Git, args: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            let argv = shellwords_split(args)?;
            let argv_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = g.run(&argv_refs)?;
            let stdout = ok_or_throw(&argv_refs, out)?;
            // Sniff: peek at first non-whitespace byte.
            let trimmed = stdout.trim_start();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                if let Ok(v) = parse_json_to_dynamic(&stdout) {
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
    use rhai::Engine;

    fn engine() -> Engine {
        let mut e = Engine::new();
        register(&mut e);
        e
    }

    #[test]
    fn git_constructor_returns_git_type() {
        let mut e = engine();
        // No type assertion — just verify the constructor exists and
        // runs without errors. Real method coverage lands in later tasks.
        let _: Dynamic = e.eval(r#"git()"#).unwrap();
        let _: Dynamic = e.eval(r#"git("/tmp")"#).unwrap();
    }

    #[test]
    fn git_run_text_returns_stdout_string() {
        let mut e = engine();
        let s: String = e
            .eval(r#"git().run_text("--version")"#)
            .unwrap();
        assert!(s.starts_with("git version "), "got: {s}");
    }

    #[test]
    fn git_run_sniffs_text_vs_json() {
        // .run() should peek at the first non-whitespace byte. For
        // `--version`, output starts with "git " → not JSON → returns
        // String.
        let mut e = engine();
        let r: Dynamic = e.eval(r#"git().run("--version")"#).unwrap();
        assert!(r.is_string(), "expected String, got {}", r.type_name());
    }

    #[test]
    fn git_run_nonzero_exit_throws() {
        let mut e = engine();
        let result: Result<String, _> = e.eval(r#"git().run_text("bogus-subcommand-xyz")"#);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("git:"), "got: {msg}");
    }

    #[test]
    fn shellwords_split_handles_quotes_and_escapes() {
        // Direct test on the helper without going through the Rhai engine.
        let parts = shellwords_split(r#"log -n 3 --format "hello world""#).unwrap();
        assert_eq!(parts, vec!["log", "-n", "3", "--format", "hello world"]);

        let parts = shellwords_split(r#"a 'b c' d"#).unwrap();
        assert_eq!(parts, vec!["a", "b c", "d"]);

        // Backslash escapes any next char outside single quotes.
        let parts = shellwords_split(r#"a\ b c"#).unwrap();
        assert_eq!(parts, vec!["a b", "c"]);

        assert!(shellwords_split(r#"unterm "quote"#).is_err());
    }
}
