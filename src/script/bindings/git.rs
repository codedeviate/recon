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
use rhai::{Dynamic, Engine, EvalAltResult, Map};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::process::{Command, Output};

const STDERR_CAP: usize = 2048;
const LOG_FORMAT: &str = "%H%x09%h%x09%an%x09%ae%x09%aI%x09%s%x09%b%x1e";

fn parse_log(output: &str) -> rhai::Array {
    let mut arr = rhai::Array::new();
    for record in output.split('\x1e') {
        let record = record.trim_start_matches('\n');
        if record.is_empty() {
            continue;
        }
        let fields: Vec<&str> = record.splitn(7, '\t').collect();
        if fields.len() < 7 {
            continue;
        }
        let mut m = rhai::Map::new();
        m.insert("hash".into(), fields[0].to_string().into());
        m.insert("short_hash".into(), fields[1].to_string().into());
        m.insert("author".into(), fields[2].to_string().into());
        m.insert("email".into(), fields[3].to_string().into());
        m.insert("date".into(), fields[4].to_string().into());
        m.insert("subject".into(), fields[5].to_string().into());
        m.insert("body".into(), fields[6].to_string().into());
        arr.push(Dynamic::from(m));
    }
    arr
}

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

fn parse_diff_stat(output: &str) -> rhai::Map {
    let mut per_file = rhai::Array::new();
    let mut files = 0i64;
    let mut insertions = 0i64;
    let mut deletions = 0i64;

    for line in output.lines() {
        let line = line.trim();
        // Per-file line: "<ins>\t<del>\t<path>"
        if let Some((rest, path)) = line.rsplit_once('\t') {
            if let Some((ins, del)) = rest.split_once('\t') {
                let ins_n: i64 = ins.parse().unwrap_or(0);
                let del_n: i64 = del.parse().unwrap_or(0);
                let mut entry = rhai::Map::new();
                entry.insert("path".into(), path.to_string().into());
                entry.insert("insertions".into(), ins_n.into());
                entry.insert("deletions".into(), del_n.into());
                per_file.push(Dynamic::from(entry));
                continue;
            }
        }
        // Summary line: " N files changed, M insertions(+), K deletions(-)"
        if line.contains("file") && (line.contains("changed") || line.contains("change")) {
            let parts = line.split(',');
            for chunk in parts {
                let chunk = chunk.trim();
                if let Some(n_str) = chunk.split_whitespace().next() {
                    if let Ok(n) = n_str.parse::<i64>() {
                        if chunk.contains("file") {
                            files = n;
                        } else if chunk.contains("insertion") {
                            insertions = n;
                        } else if chunk.contains("deletion") {
                            deletions = n;
                        }
                    }
                }
            }
        }
    }

    let mut m = rhai::Map::new();
    m.insert("files".into(), files.into());
    m.insert("insertions".into(), insertions.into());
    m.insert("deletions".into(), deletions.into());
    m.insert("per_file".into(), per_file.into());
    m
}

fn parse_porcelain_v2(output: &str) -> Map {
    use rhai::Array;
    let mut branch = String::new();
    let mut upstream: Option<String> = None;
    let mut ahead = 0i64;
    let mut behind = 0i64;
    let mut staged = Array::new();
    let mut unstaged = Array::new();
    let mut untracked = Array::new();

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            branch = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("# branch.upstream ") {
            upstream = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            // Format: "+<a> -<b>"
            let mut parts = rest.split_whitespace();
            if let Some(a) = parts.next() {
                ahead = a.trim_start_matches('+').parse().unwrap_or(0);
            }
            if let Some(b) = parts.next() {
                behind = b.trim_start_matches('-').parse().unwrap_or(0);
            }
        } else if let Some(rest) = line.strip_prefix("1 ") {
            // `1 <xy> <sub> <m1> <m2> <m3> <h1> <h2> <path>`
            let parts: Vec<&str> = rest.splitn(8, ' ').collect();
            if parts.len() == 8 {
                let xy = parts[0];
                let path = parts[7];
                let mut entry = Map::new();
                entry.insert("path".into(), path.to_string().into());
                entry.insert("x_y".into(), xy.to_string().into());
                if xy.chars().nth(0) != Some('.') {
                    staged.push(Dynamic::from(entry.clone()));
                }
                if xy.chars().nth(1) != Some('.') {
                    unstaged.push(Dynamic::from(entry));
                }
            }
        } else if let Some(rest) = line.strip_prefix("2 ") {
            // Rename: `2 <xy> <sub> <m1> <m2> <m3> <h1> <h2> <X<score>> <path>\t<orig>`
            let parts: Vec<&str> = rest.splitn(10, ' ').collect();
            if parts.len() == 10 {
                let xy = parts[0];
                let path_pair = parts[9];
                let (new_path, orig_path) = path_pair
                    .split_once('\t')
                    .unwrap_or((path_pair, ""));
                let mut entry = Map::new();
                entry.insert("path".into(), new_path.to_string().into());
                entry.insert("x_y".into(), xy.to_string().into());
                entry.insert("original".into(), orig_path.to_string().into());
                if xy.chars().nth(0) != Some('.') {
                    staged.push(Dynamic::from(entry.clone()));
                }
                if xy.chars().nth(1) != Some('.') {
                    unstaged.push(Dynamic::from(entry));
                }
            }
        } else if let Some(rest) = line.strip_prefix("? ") {
            untracked.push(rest.to_string().into());
        }
        // Ignore `u` (unmerged) and `!` (ignored) lines for v1.
    }

    let clean = staged.is_empty() && unstaged.is_empty() && untracked.is_empty();

    let mut m = Map::new();
    m.insert("branch".into(), branch.into());
    m.insert("upstream".into(), match upstream {
        Some(u) => u.into(),
        None => Dynamic::UNIT,
    });
    m.insert("ahead".into(), ahead.into());
    m.insert("behind".into(), behind.into());
    m.insert("clean".into(), clean.into());
    m.insert("staged".into(), staged.into());
    m.insert("unstaged".into(), unstaged.into());
    m.insert("untracked".into(), untracked.into());
    m
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

    engine.register_fn(
        "status",
        |g: &mut Git| -> Result<Map, Box<EvalAltResult>> {
            let argv = &["status", "--porcelain=v2", "--branch"];
            let out = g.run(argv)?;
            let stdout = ok_or_throw(argv, out)?;
            Ok(parse_porcelain_v2(&stdout))
        },
    );

    engine.register_fn("is_clean", |g: &mut Git| -> Result<bool, Box<EvalAltResult>> {
        let argv = &["status", "--porcelain=v2", "--branch"];
        let out = g.run(argv)?;
        let stdout = ok_or_throw(argv, out)?;
        let m = parse_porcelain_v2(&stdout);
        Ok(m.get("clean").and_then(|v| v.as_bool().ok()).unwrap_or(false))
    });

    engine.register_fn(
        "log",
        |g: &mut Git, n: i64| -> Result<rhai::Array, Box<EvalAltResult>> {
            let n_str = n.to_string();
            let fmt = format!("--format={LOG_FORMAT}");
            let argv = &["log", "-n", &n_str, &fmt];
            let out = g.run(argv)?;
            let stdout = ok_or_throw(argv, out)?;
            Ok(parse_log(&stdout))
        },
    );

    engine.register_fn(
        "log_range",
        |g: &mut Git, range: &str| -> Result<rhai::Array, Box<EvalAltResult>> {
            let fmt = format!("--format={LOG_FORMAT}");
            let argv = &["log", &fmt, range];
            let out = g.run(argv)?;
            let stdout = ok_or_throw(argv, out)?;
            Ok(parse_log(&stdout))
        },
    );

    engine.register_fn("diff", |g: &mut Git| -> Result<String, Box<EvalAltResult>> {
        let argv = &["diff"];
        let out = g.run(argv)?;
        ok_or_throw(argv, out)
    });
    engine.register_fn(
        "diff",
        |g: &mut Git, rev: &str| -> Result<String, Box<EvalAltResult>> {
            let argv = &["diff", rev];
            let out = g.run(argv)?;
            ok_or_throw(argv, out)
        },
    );
    engine.register_fn(
        "diff_stat",
        |g: &mut Git| -> Result<rhai::Map, Box<EvalAltResult>> {
            let argv = &["diff", "--numstat", "--shortstat"];
            let out = g.run(argv)?;
            let stdout = ok_or_throw(argv, out)?;
            Ok(parse_diff_stat(&stdout))
        },
    );
    engine.register_fn(
        "diff_stat",
        |g: &mut Git, rev: &str| -> Result<rhai::Map, Box<EvalAltResult>> {
            let argv = &["diff", "--numstat", "--shortstat", rev];
            let out = g.run(argv)?;
            let stdout = ok_or_throw(argv, out)?;
            Ok(parse_diff_stat(&stdout))
        },
    );

    engine.register_fn(
        "branch",
        |g: &mut Git| -> Result<rhai::Map, Box<EvalAltResult>> {
            // Current branch.
            let cur_argv = &["branch", "--show-current"];
            let cur_out = ok_or_throw(cur_argv, g.run(cur_argv)?)?;
            let current = cur_out.trim().to_string();

            // All branches.
            let all_argv = &["branch", "-a", "--format=%(refname:short)"];
            let all_out = ok_or_throw(all_argv, g.run(all_argv)?)?;
            let all: rhai::Array = all_out
                .lines()
                .map(|l| Dynamic::from(l.trim().to_string()))
                .filter(|d| !d.clone().into_string().unwrap_or_default().is_empty())
                .collect();

            // Upstream (optional, may fail when no upstream is set).
            let up_argv = &["rev-parse", "--abbrev-ref", "@{upstream}"];
            let upstream = match g.run(up_argv) {
                Ok(out) if out.status.success() => {
                    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if s.is_empty() {
                        Dynamic::UNIT
                    } else {
                        Dynamic::from(s)
                    }
                }
                _ => Dynamic::UNIT,
            };

            let mut m = rhai::Map::new();
            m.insert("current".into(), current.into());
            m.insert("all".into(), all.into());
            m.insert("upstream".into(), upstream);
            Ok(m)
        },
    );

    engine.register_fn(
        "rev_parse",
        |g: &mut Git, name: &str| -> Result<String, Box<EvalAltResult>> {
            let argv = &["rev-parse", name];
            let out = g.run(argv)?;
            let s = ok_or_throw(argv, out)?;
            Ok(s.trim().to_string())
        },
    );

    engine.register_fn(
        "add",
        |g: &mut Git, path: &str| -> Result<(), Box<EvalAltResult>> {
            let argv = &["add", path];
            let out = g.run(argv)?;
            ok_or_throw(argv, out)?;
            Ok(())
        },
    );
    engine.register_fn(
        "add",
        |g: &mut Git, paths: rhai::Array| -> Result<(), Box<EvalAltResult>> {
            let mut args: Vec<String> = vec!["add".to_string()];
            for p in paths {
                args.push(p.into_string().map_err(|_| err("git.add: array must contain strings"))?);
            }
            let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let out = g.run(&refs)?;
            ok_or_throw(&refs, out)?;
            Ok(())
        },
    );

    engine.register_fn(
        "commit",
        |g: &mut Git, message: &str| -> Result<rhai::Map, Box<EvalAltResult>> {
            let commit_argv = &["commit", "-m", message];
            let out = g.run(commit_argv)?;
            ok_or_throw(commit_argv, out)?;

            // Re-fetch the new commit's info.
            let log_argv = &["log", "-n", "1", "--format=%H%x09%h%x09%s"];
            let log_out = ok_or_throw(log_argv, g.run(log_argv)?)?;
            let parts: Vec<&str> = log_out.trim().splitn(3, '\t').collect();
            let mut m = rhai::Map::new();
            if parts.len() == 3 {
                m.insert("hash".into(), parts[0].to_string().into());
                m.insert("short_hash".into(), parts[1].to_string().into());
                m.insert("subject".into(), parts[2].to_string().into());
            }
            Ok(m)
        },
    );

    engine.register_fn(
        "push",
        |g: &mut Git| -> Result<(), Box<EvalAltResult>> {
            let argv = &["push"];
            let out = g.run(argv)?;
            ok_or_throw(argv, out)?;
            Ok(())
        },
    );
    engine.register_fn(
        "push",
        |g: &mut Git, remote: &str| -> Result<(), Box<EvalAltResult>> {
            let argv = &["push", remote];
            let out = g.run(argv)?;
            ok_or_throw(argv, out)?;
            Ok(())
        },
    );
    engine.register_fn(
        "push",
        |g: &mut Git, remote: &str, branch: &str| -> Result<(), Box<EvalAltResult>> {
            let argv = &["push", remote, branch];
            let out = g.run(argv)?;
            ok_or_throw(argv, out)?;
            Ok(())
        },
    );

    engine.register_fn(
        "pull",
        |g: &mut Git| -> Result<(), Box<EvalAltResult>> {
            let argv = &["pull"];
            let out = g.run(argv)?;
            ok_or_throw(argv, out)?;
            Ok(())
        },
    );
    engine.register_fn(
        "pull",
        |g: &mut Git, remote: &str, branch: &str| -> Result<(), Box<EvalAltResult>> {
            let argv = &["pull", remote, branch];
            let out = g.run(argv)?;
            ok_or_throw(argv, out)?;
            Ok(())
        },
    );

    engine.register_fn(
        "checkout",
        |g: &mut Git, name: &str| -> Result<(), Box<EvalAltResult>> {
            let argv = &["checkout", name];
            let out = g.run(argv)?;
            ok_or_throw(argv, out)?;
            Ok(())
        },
    );

    engine.register_fn(
        "remote",
        |g: &mut Git| -> Result<rhai::Map, Box<EvalAltResult>> {
            let argv = &["remote", "-v"];
            let out = g.run(argv)?;
            let stdout = ok_or_throw(argv, out)?;
            let mut m = rhai::Map::new();
            for line in stdout.lines() {
                // Each line: "<name>\t<url> (fetch|push)"
                let mut parts = line.split('\t');
                let name = parts.next().unwrap_or("").trim();
                let rest = parts.next().unwrap_or("");
                let url = rest.split_whitespace().next().unwrap_or("");
                if !name.is_empty() && !m.contains_key(name) {
                    m.insert(name.into(), url.to_string().into());
                }
            }
            Ok(m)
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

    use std::process::Command as StdCommand;
    use tempfile::TempDir;

    fn make_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let cwd = dir.path();
        for args in &[
            vec!["init", "-q", "-b", "master"],
            vec!["config", "user.email", "test@example.com"],
            vec!["config", "user.name", "Test"],
            vec!["commit", "--allow-empty", "-q", "-m", "initial"],
        ] {
            StdCommand::new("git")
                .current_dir(cwd)
                .args(args)
                .output()
                .expect("git command failed during test setup");
        }
        dir
    }

    #[test]
    fn git_status_clean_repo() {
        let dir = make_repo();
        let mut e = engine();
        let path = dir.path().to_string_lossy().to_string();
        let script = format!(r#"git("{}").status()"#, path);
        let m: rhai::Map = e.eval(&script).unwrap();
        assert_eq!(m.get("branch").unwrap().clone().into_string().unwrap(), "master");
        assert_eq!(m.get("clean").unwrap().as_bool().unwrap(), true);
        let staged: rhai::Array = m.get("staged").unwrap().clone().cast();
        let unstaged: rhai::Array = m.get("unstaged").unwrap().clone().cast();
        let untracked: rhai::Array = m.get("untracked").unwrap().clone().cast();
        assert!(staged.is_empty());
        assert!(unstaged.is_empty());
        assert!(untracked.is_empty());
    }

    #[test]
    fn git_status_dirty_repo() {
        let dir = make_repo();
        std::fs::write(dir.path().join("foo.txt"), "hi").unwrap();
        let mut e = engine();
        let path = dir.path().to_string_lossy().to_string();
        let script = format!(r#"git("{}").status()"#, path);
        let m: rhai::Map = e.eval(&script).unwrap();
        assert_eq!(m.get("clean").unwrap().as_bool().unwrap(), false);
        let untracked: rhai::Array = m.get("untracked").unwrap().clone().cast();
        assert_eq!(untracked.len(), 1);
        assert_eq!(untracked[0].clone().into_string().unwrap(), "foo.txt");
    }

    #[test]
    fn git_log_returns_commits_in_order() {
        let dir = make_repo();
        // Add two more commits for variety.
        for msg in &["second", "third"] {
            StdCommand::new("git")
                .current_dir(dir.path())
                .args(&["commit", "--allow-empty", "-q", "-m", msg])
                .output()
                .unwrap();
        }
        let mut e = engine();
        let path = dir.path().to_string_lossy().to_string();
        let script = format!(r#"git("{}").log(3)"#, path);
        let arr: rhai::Array = e.eval(&script).unwrap();
        assert_eq!(arr.len(), 3);
        let first: rhai::Map = arr[0].clone().cast();
        assert_eq!(first.get("subject").unwrap().clone().into_string().unwrap(), "third");
        let third: rhai::Map = arr[2].clone().cast();
        assert_eq!(third.get("subject").unwrap().clone().into_string().unwrap(), "initial");
    }

    #[test]
    fn git_log_commit_has_expected_fields() {
        let dir = make_repo();
        let mut e = engine();
        let path = dir.path().to_string_lossy().to_string();
        let script = format!(r#"git("{}").log(1)"#, path);
        let arr: rhai::Array = e.eval(&script).unwrap();
        let c: rhai::Map = arr[0].clone().cast();
        for k in &["hash", "short_hash", "author", "email", "date", "subject", "body"] {
            assert!(c.contains_key(*k), "missing key: {k}");
        }
    }

    #[test]
    fn git_diff_returns_patch_text() {
        let dir = make_repo();
        std::fs::write(dir.path().join("foo.txt"), "hello\n").unwrap();
        StdCommand::new("git")
            .current_dir(dir.path())
            .args(&["add", "foo.txt"])
            .output()
            .unwrap();
        StdCommand::new("git")
            .current_dir(dir.path())
            .args(&["commit", "-q", "-m", "add foo"])
            .output()
            .unwrap();
        std::fs::write(dir.path().join("foo.txt"), "hello\nworld\n").unwrap();
        let mut e = engine();
        let path = dir.path().to_string_lossy().to_string();
        let s: String = e
            .eval(&format!(r#"git("{}").diff()"#, path))
            .unwrap();
        assert!(s.contains("+world"), "got: {s}");
    }

    #[test]
    fn git_diff_stat_returns_counts() {
        let dir = make_repo();
        std::fs::write(dir.path().join("foo.txt"), "a\nb\nc\n").unwrap();
        StdCommand::new("git")
            .current_dir(dir.path())
            .args(&["add", "foo.txt"])
            .output()
            .unwrap();
        StdCommand::new("git")
            .current_dir(dir.path())
            .args(&["commit", "-q", "-m", "add foo"])
            .output()
            .unwrap();
        std::fs::write(dir.path().join("foo.txt"), "a\nb\nc\nd\n").unwrap();
        let mut e = engine();
        let path = dir.path().to_string_lossy().to_string();
        let m: rhai::Map = e
            .eval(&format!(r#"git("{}").diff_stat()"#, path))
            .unwrap();
        assert_eq!(m.get("files").unwrap().as_int().unwrap(), 1);
        assert_eq!(m.get("insertions").unwrap().as_int().unwrap(), 1);
        let per_file: rhai::Array = m.get("per_file").unwrap().clone().cast();
        assert_eq!(per_file.len(), 1);
        let f0: rhai::Map = per_file[0].clone().cast();
        assert_eq!(f0.get("path").unwrap().clone().into_string().unwrap(), "foo.txt");
    }

    #[test]
    fn git_branch_returns_current_and_all() {
        let dir = make_repo();
        let mut e = engine();
        let path = dir.path().to_string_lossy().to_string();
        let m: rhai::Map = e
            .eval(&format!(r#"git("{}").branch()"#, path))
            .unwrap();
        assert_eq!(m.get("current").unwrap().clone().into_string().unwrap(), "master");
        let all: rhai::Array = m.get("all").unwrap().clone().cast();
        assert!(all.iter().any(|d| d.clone().into_string().map(|s| s == "master").unwrap_or(false)));
    }

    #[test]
    fn git_rev_parse_returns_full_sha() {
        let dir = make_repo();
        let mut e = engine();
        let path = dir.path().to_string_lossy().to_string();
        let s: String = e
            .eval(&format!(r#"git("{}").rev_parse("HEAD")"#, path))
            .unwrap();
        assert_eq!(s.trim().len(), 40, "expected 40-char SHA, got: {s}");
    }

    #[test]
    fn git_remote_returns_map_of_name_to_url() {
        let dir = make_repo();
        // Add a fake remote.
        StdCommand::new("git")
            .current_dir(dir.path())
            .args(&["remote", "add", "origin", "https://example.com/repo.git"])
            .output()
            .unwrap();
        let mut e = engine();
        let path = dir.path().to_string_lossy().to_string();
        let m: rhai::Map = e
            .eval(&format!(r#"git("{}").remote()"#, path))
            .unwrap();
        let origin = m.get("origin").unwrap().clone().into_string().unwrap();
        assert_eq!(origin, "https://example.com/repo.git");
    }

    #[test]
    fn git_add_then_commit_returns_new_commit_map() {
        let dir = make_repo();
        std::fs::write(dir.path().join("foo.txt"), "hi").unwrap();
        let mut e = engine();
        let path = dir.path().to_string_lossy().to_string();
        e.eval::<Dynamic>(&format!(r#"git("{}").add("foo.txt")"#, path)).unwrap();
        let m: rhai::Map = e
            .eval(&format!(r#"git("{}").commit("add foo")"#, path))
            .unwrap();
        assert_eq!(m.get("subject").unwrap().clone().into_string().unwrap(), "add foo");
        assert_eq!(m.get("hash").unwrap().clone().into_string().unwrap().len(), 40);
    }

    #[test]
    fn git_add_accepts_array_of_paths() {
        let dir = make_repo();
        std::fs::write(dir.path().join("a.txt"), "1").unwrap();
        std::fs::write(dir.path().join("b.txt"), "2").unwrap();
        let mut e = engine();
        let path = dir.path().to_string_lossy().to_string();
        e.eval::<Dynamic>(&format!(r#"git("{}").add(["a.txt", "b.txt"])"#, path)).unwrap();
        let m: rhai::Map = e
            .eval(&format!(r#"git("{}").status()"#, path))
            .unwrap();
        let staged: rhai::Array = m.get("staged").unwrap().clone().cast();
        assert_eq!(staged.len(), 2);
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
