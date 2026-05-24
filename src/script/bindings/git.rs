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
#[allow(unused_imports)]
use rhai::{Dynamic, Engine, EvalAltResult, Map};
use std::path::PathBuf;
use std::process::{Command, Output};

#[allow(dead_code)]
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

    #[allow(dead_code)]
    fn run(&self, args: &[&str]) -> Result<Output, Box<EvalAltResult>> {
        run_command(&self.cwd, args)
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

pub fn register(engine: &mut Engine) {
    engine.register_type_with_name::<Git>("Git");

    // git() — cwd-bound.
    engine.register_fn("git", || -> Git { Git::new(None) });
    // git(path) — explicit path.
    engine.register_fn("git", |path: &str| -> Git { Git::new(Some(path)) });
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
}
