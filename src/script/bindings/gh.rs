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

const PR_LIST_FIELDS: &str =
    "number,title,state,author,headRefName,baseRefName,createdAt,url";
const PR_VIEW_FIELDS: &str =
    "number,title,state,author,body,labels,reviewDecision,mergeable,headRefName,baseRefName,createdAt,url";

fn pr_list_opts_to_args(opts: &Map) -> Result<Vec<String>, Box<EvalAltResult>> {
    let mut out = Vec::new();
    for (k, v) in opts {
        match k.as_str() {
            "state" => {
                out.push("--state".into());
                out.push(v.clone().into_string().map_err(|_| err("pr_list opts: state must be a string"))?);
            }
            "author" => {
                out.push("--author".into());
                out.push(v.clone().into_string().map_err(|_| err("pr_list opts: author must be a string"))?);
            }
            "label" => {
                if let Ok(s) = v.clone().into_string() {
                    out.push("--label".into());
                    out.push(s);
                } else if let Some(arr) = v.clone().try_cast::<Array>() {
                    for l in arr {
                        out.push("--label".into());
                        out.push(l.into_string().map_err(|_| err("pr_list opts: label array must contain strings"))?);
                    }
                } else {
                    return Err(err("pr_list opts: label must be string or array of strings"));
                }
            }
            "limit" => {
                let n = v.clone().as_int().map_err(|_| err("pr_list opts: limit must be an integer"))?;
                out.push("--limit".into());
                out.push(n.to_string());
            }
            other => return Err(err(format!("pr_list opts: unknown key '{other}'"))),
        }
    }
    Ok(out)
}

fn pr_create_opts_to_args(opts: &Map) -> Result<Vec<String>, Box<EvalAltResult>> {
    let mut out = Vec::new();
    let mut have_title = false;
    let mut have_body = false;
    let mut have_body_file = false;
    for (k, v) in opts {
        match k.as_str() {
            "title" => {
                have_title = true;
                out.push("--title".into());
                out.push(v.clone().into_string().map_err(|_| err("pr_create opts: title must be a string"))?);
            }
            "body" => {
                have_body = true;
                out.push("--body".into());
                out.push(v.clone().into_string().map_err(|_| err("pr_create opts: body must be a string"))?);
            }
            "body_file" => {
                have_body_file = true;
                out.push("--body-file".into());
                out.push(v.clone().into_string().map_err(|_| err("pr_create opts: body_file must be a string"))?);
            }
            "base" => {
                out.push("--base".into());
                out.push(v.clone().into_string().map_err(|_| err("pr_create opts: base must be a string"))?);
            }
            "head" => {
                out.push("--head".into());
                out.push(v.clone().into_string().map_err(|_| err("pr_create opts: head must be a string"))?);
            }
            "draft" => {
                if v.clone().as_bool().unwrap_or(false) {
                    out.push("--draft".into());
                }
            }
            "reviewer" => {
                if let Ok(s) = v.clone().into_string() {
                    out.push("--reviewer".into());
                    out.push(s);
                } else if let Some(arr) = v.clone().try_cast::<Array>() {
                    for r in arr {
                        out.push("--reviewer".into());
                        out.push(r.into_string().map_err(|_| err("pr_create opts: reviewer array must contain strings"))?);
                    }
                } else {
                    return Err(err("pr_create opts: reviewer must be string or array of strings"));
                }
            }
            "label" => {
                if let Ok(s) = v.clone().into_string() {
                    out.push("--label".into());
                    out.push(s);
                } else if let Some(arr) = v.clone().try_cast::<Array>() {
                    for l in arr {
                        out.push("--label".into());
                        out.push(l.into_string().map_err(|_| err("pr_create opts: label array must contain strings"))?);
                    }
                } else {
                    return Err(err("pr_create opts: label must be string or array of strings"));
                }
            }
            other => return Err(err(format!("pr_create opts: unknown key '{other}'"))),
        }
    }
    if !have_title {
        return Err(err("pr_create opts: title is required"));
    }
    if have_body && have_body_file {
        return Err(err("pr_create opts: body and body_file are mutually exclusive"));
    }
    Ok(out)
}

fn pr_merge_opts_to_args(opts: &Map) -> Result<Vec<String>, Box<EvalAltResult>> {
    let mut out = Vec::new();
    for (k, v) in opts {
        match k.as_str() {
            "method" => {
                let s = v.clone().into_string().map_err(|_| err("pr_merge opts: method must be string"))?;
                match s.as_str() {
                    "merge" => out.push("--merge".into()),
                    "squash" => out.push("--squash".into()),
                    "rebase" => out.push("--rebase".into()),
                    other => return Err(err(format!("pr_merge opts: unknown method '{other}' (want merge/squash/rebase)"))),
                }
            }
            "delete_branch" => {
                if v.clone().as_bool().unwrap_or(false) {
                    out.push("--delete-branch".into());
                }
            }
            "auto" => {
                if v.clone().as_bool().unwrap_or(false) {
                    out.push("--auto".into());
                }
            }
            other => return Err(err(format!("pr_merge opts: unknown key '{other}'"))),
        }
    }
    Ok(out)
}

const ISSUE_LIST_FIELDS: &str =
    "number,title,state,author,labels,assignees,createdAt,url";
const ISSUE_VIEW_FIELDS: &str =
    "number,title,state,author,body,labels,assignees,createdAt,closedAt,url";

fn issue_list_opts_to_args(opts: &Map) -> Result<Vec<String>, Box<EvalAltResult>> {
    let mut out = Vec::new();
    for (k, v) in opts {
        match k.as_str() {
            "state" => {
                out.push("--state".into());
                out.push(v.clone().into_string().map_err(|_| err("issue_list opts: state must be a string"))?);
            }
            "author" => {
                out.push("--author".into());
                out.push(v.clone().into_string().map_err(|_| err("issue_list opts: author must be a string"))?);
            }
            "label" => {
                if let Ok(s) = v.clone().into_string() {
                    out.push("--label".into());
                    out.push(s);
                } else if let Some(arr) = v.clone().try_cast::<Array>() {
                    for l in arr {
                        out.push("--label".into());
                        out.push(l.into_string().map_err(|_| err("issue_list opts: label array must contain strings"))?);
                    }
                } else {
                    return Err(err("issue_list opts: label must be string or array of strings"));
                }
            }
            "assignee" => {
                if let Ok(s) = v.clone().into_string() {
                    out.push("--assignee".into());
                    out.push(s);
                } else if let Some(arr) = v.clone().try_cast::<Array>() {
                    for a in arr {
                        out.push("--assignee".into());
                        out.push(a.into_string().map_err(|_| err("issue_list opts: assignee array must contain strings"))?);
                    }
                } else {
                    return Err(err("issue_list opts: assignee must be string or array of strings"));
                }
            }
            "limit" => {
                let n = v.clone().as_int().map_err(|_| err("issue_list opts: limit must be an integer"))?;
                out.push("--limit".into());
                out.push(n.to_string());
            }
            other => return Err(err(format!("issue_list opts: unknown key '{other}'"))),
        }
    }
    Ok(out)
}

fn issue_create_opts_to_args(opts: &Map) -> Result<Vec<String>, Box<EvalAltResult>> {
    let mut out = Vec::new();
    let mut have_title = false;
    let mut have_body = false;
    let mut have_body_file = false;
    for (k, v) in opts {
        match k.as_str() {
            "title" => {
                have_title = true;
                out.push("--title".into());
                out.push(v.clone().into_string().map_err(|_| err("issue_create opts: title must be a string"))?);
            }
            "body" => {
                have_body = true;
                out.push("--body".into());
                out.push(v.clone().into_string().map_err(|_| err("issue_create opts: body must be a string"))?);
            }
            "body_file" => {
                have_body_file = true;
                out.push("--body-file".into());
                out.push(v.clone().into_string().map_err(|_| err("issue_create opts: body_file must be a string"))?);
            }
            "label" => {
                if let Ok(s) = v.clone().into_string() {
                    out.push("--label".into());
                    out.push(s);
                } else if let Some(arr) = v.clone().try_cast::<Array>() {
                    for l in arr {
                        out.push("--label".into());
                        out.push(l.into_string().map_err(|_| err("issue_create opts: label array must contain strings"))?);
                    }
                } else {
                    return Err(err("issue_create opts: label must be string or array of strings"));
                }
            }
            "assignee" => {
                if let Ok(s) = v.clone().into_string() {
                    out.push("--assignee".into());
                    out.push(s);
                } else if let Some(arr) = v.clone().try_cast::<Array>() {
                    for a in arr {
                        out.push("--assignee".into());
                        out.push(a.into_string().map_err(|_| err("issue_create opts: assignee array must contain strings"))?);
                    }
                } else {
                    return Err(err("issue_create opts: assignee must be string or array of strings"));
                }
            }
            other => return Err(err(format!("issue_create opts: unknown key '{other}'"))),
        }
    }
    if !have_title {
        return Err(err("issue_create opts: title is required"));
    }
    if have_body && have_body_file {
        return Err(err("issue_create opts: body and body_file are mutually exclusive"));
    }
    Ok(out)
}

/// Parse the URL line that `gh issue create` emits, returning
/// `{ number, url }` map.
fn parse_issue_url(s: &str) -> Result<Map, Box<EvalAltResult>> {
    let re = regex::Regex::new(r"https://[^\s]+/issues/(\d+)").unwrap();
    let cap = re.captures(s).ok_or_else(|| {
        err(format!("gh.issue_create: could not parse URL from output: {s}"))
    })?;
    let url = cap.get(0).unwrap().as_str().to_string();
    let number: i64 = cap[1].parse().unwrap_or(0);
    let mut m = Map::new();
    m.insert("url".into(), url.into());
    m.insert("number".into(), number.into());
    Ok(m)
}

/// Parse the URL line that `gh pr create` emits, returning
/// `{ number, url }` map.
fn parse_pr_url(s: &str) -> Result<Map, Box<EvalAltResult>> {
    let re = regex::Regex::new(r"https://[^\s]+/pull/(\d+)").unwrap();
    let cap = re.captures(s).ok_or_else(|| {
        err(format!("gh.pr_create: could not parse URL from output: {s}"))
    })?;
    let url = cap.get(0).unwrap().as_str().to_string();
    let number: i64 = cap[1].parse().unwrap_or(0);
    let mut m = Map::new();
    m.insert("url".into(), url.into());
    m.insert("number".into(), number.into());
    Ok(m)
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

    engine.register_fn(
        "pr_list",
        |h: &mut Gh| -> Result<Array, Box<EvalAltResult>> {
            let argv = &["pr", "list", "--json", PR_LIST_FIELDS];
            let out = h.run(argv, true)?;
            let stdout = ok_or_throw(argv, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.pr_list: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Array>().ok_or_else(|| err("gh.pr_list: expected Array"))
        },
    );
    engine.register_fn(
        "pr_list",
        |h: &mut Gh, opts: Map| -> Result<Array, Box<EvalAltResult>> {
            let mut argv: Vec<String> = vec!["pr".into(), "list".into(), "--json".into(), PR_LIST_FIELDS.into()];
            argv.extend(pr_list_opts_to_args(&opts)?);
            let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = h.run(&refs, true)?;
            let stdout = ok_or_throw(&refs, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.pr_list: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Array>().ok_or_else(|| err("gh.pr_list: expected Array"))
        },
    );

    engine.register_fn(
        "pr_view",
        |h: &mut Gh, number: i64| -> Result<Map, Box<EvalAltResult>> {
            let n_str = number.to_string();
            let argv = &["pr", "view", &n_str, "--json", PR_VIEW_FIELDS];
            let out = h.run(argv, true)?;
            let stdout = ok_or_throw(argv, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.pr_view: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Map>().ok_or_else(|| err("gh.pr_view: expected Map"))
        },
    );

    engine.register_fn(
        "pr_create",
        |h: &mut Gh, opts: Map| -> Result<Map, Box<EvalAltResult>> {
            let mut argv: Vec<String> = vec!["pr".into(), "create".into()];
            argv.extend(pr_create_opts_to_args(&opts)?);
            let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = h.run(&refs, true)?;
            let stdout = ok_or_throw(&refs, out)?;
            parse_pr_url(&stdout)
        },
    );

    engine.register_fn(
        "pr_merge",
        |h: &mut Gh, number: i64| -> Result<(), Box<EvalAltResult>> {
            let n_str = number.to_string();
            let argv = &["pr", "merge", &n_str];
            let out = h.run(argv, true)?;
            ok_or_throw(argv, out)?;
            Ok(())
        },
    );
    engine.register_fn(
        "pr_merge",
        |h: &mut Gh, number: i64, opts: Map| -> Result<(), Box<EvalAltResult>> {
            let n_str = number.to_string();
            let mut argv: Vec<String> = vec!["pr".into(), "merge".into(), n_str];
            argv.extend(pr_merge_opts_to_args(&opts)?);
            let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = h.run(&refs, true)?;
            ok_or_throw(&refs, out)?;
            Ok(())
        },
    );

    engine.register_fn(
        "pr_close",
        |h: &mut Gh, number: i64| -> Result<(), Box<EvalAltResult>> {
            let n_str = number.to_string();
            let argv = &["pr", "close", &n_str];
            let out = h.run(argv, true)?;
            ok_or_throw(argv, out)?;
            Ok(())
        },
    );

    engine.register_fn(
        "pr_comment",
        |h: &mut Gh, number: i64, body: &str| -> Result<(), Box<EvalAltResult>> {
            let n_str = number.to_string();
            let argv = &["pr", "comment", &n_str, "--body", body];
            let out = h.run(argv, true)?;
            ok_or_throw(argv, out)?;
            Ok(())
        },
    );

    engine.register_fn(
        "issue_list",
        |h: &mut Gh| -> Result<Array, Box<EvalAltResult>> {
            let argv = &["issue", "list", "--json", ISSUE_LIST_FIELDS];
            let out = h.run(argv, true)?;
            let stdout = ok_or_throw(argv, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.issue_list: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Array>().ok_or_else(|| err("gh.issue_list: expected Array"))
        },
    );
    engine.register_fn(
        "issue_list",
        |h: &mut Gh, opts: Map| -> Result<Array, Box<EvalAltResult>> {
            let mut argv: Vec<String> = vec![
                "issue".into(), "list".into(), "--json".into(), ISSUE_LIST_FIELDS.into(),
            ];
            argv.extend(issue_list_opts_to_args(&opts)?);
            let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = h.run(&refs, true)?;
            let stdout = ok_or_throw(&refs, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.issue_list: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Array>().ok_or_else(|| err("gh.issue_list: expected Array"))
        },
    );

    engine.register_fn(
        "issue_view",
        |h: &mut Gh, number: i64| -> Result<Map, Box<EvalAltResult>> {
            let n_str = number.to_string();
            let argv = &["issue", "view", &n_str, "--json", ISSUE_VIEW_FIELDS];
            let out = h.run(argv, true)?;
            let stdout = ok_or_throw(argv, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.issue_view: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Map>().ok_or_else(|| err("gh.issue_view: expected Map"))
        },
    );

    engine.register_fn(
        "issue_create",
        |h: &mut Gh, opts: Map| -> Result<Map, Box<EvalAltResult>> {
            let mut argv: Vec<String> = vec!["issue".into(), "create".into()];
            argv.extend(issue_create_opts_to_args(&opts)?);
            let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = h.run(&refs, true)?;
            let stdout = ok_or_throw(&refs, out)?;
            parse_issue_url(&stdout)
        },
    );

    engine.register_fn(
        "issue_comment",
        |h: &mut Gh, number: i64, body: &str| -> Result<(), Box<EvalAltResult>> {
            let n_str = number.to_string();
            let argv = &["issue", "comment", &n_str, "--body", body];
            let out = h.run(argv, true)?;
            ok_or_throw(argv, out)?;
            Ok(())
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::{Dynamic, Engine, Array};

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

    #[test]
    fn pr_list_opts_to_args_translates_all_keys() {
        let mut opts = rhai::Map::new();
        opts.insert("state".into(), "open".to_string().into());
        opts.insert("author".into(), "@me".to_string().into());
        opts.insert("label".into(), "bug".to_string().into());
        opts.insert("limit".into(), 50i64.into());
        let args = pr_list_opts_to_args(&opts).unwrap();
        assert!(args.contains(&"--state".to_string()));
        assert!(args.contains(&"open".to_string()));
        assert!(args.contains(&"--author".to_string()));
        assert!(args.contains(&"--label".to_string()));
        assert!(args.contains(&"50".to_string()));
    }

    #[test]
    fn pr_list_opts_to_args_accepts_label_array() {
        let mut opts = rhai::Map::new();
        let mut labels = rhai::Array::new();
        labels.push("bug".to_string().into());
        labels.push("urgent".to_string().into());
        opts.insert("label".into(), labels.into());
        let args = pr_list_opts_to_args(&opts).unwrap();
        // Two --label args, one per element.
        let label_count = args.iter().filter(|s| s.as_str() == "--label").count();
        assert_eq!(label_count, 2);
    }

    #[test]
    fn pr_list_opts_rejects_unknown_keys() {
        let mut opts = rhai::Map::new();
        opts.insert("unknownkey".into(), "x".to_string().into());
        let result = pr_list_opts_to_args(&opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown key"));
    }

    #[test]
    fn pr_create_opts_to_args_handles_body_vs_body_file() {
        let mut opts = rhai::Map::new();
        opts.insert("title".into(), "T".to_string().into());
        opts.insert("body_file".into(), "/path/body.md".to_string().into());
        let args = pr_create_opts_to_args(&opts).unwrap();
        assert!(args.contains(&"--body-file".to_string()));
        assert!(args.contains(&"/path/body.md".to_string()));

        let mut opts = rhai::Map::new();
        opts.insert("title".into(), "T".to_string().into());
        opts.insert("body".into(), "Hello".to_string().into());
        let args = pr_create_opts_to_args(&opts).unwrap();
        assert!(args.contains(&"--body".to_string()));
        assert!(args.contains(&"Hello".to_string()));
    }

    #[test]
    fn pr_create_opts_rejects_missing_title() {
        let mut opts = rhai::Map::new();
        opts.insert("body".into(), "hello".to_string().into());
        let result = pr_create_opts_to_args(&opts);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("title is required"),
        );
    }

    #[test]
    fn pr_create_opts_rejects_body_and_body_file_together() {
        let mut opts = rhai::Map::new();
        opts.insert("title".into(), "T".to_string().into());
        opts.insert("body".into(), "x".to_string().into());
        opts.insert("body_file".into(), "/p".to_string().into());
        let result = pr_create_opts_to_args(&opts);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("mutually exclusive"),
        );
    }

    #[test]
    fn parse_pr_url_extracts_number_and_url() {
        let m = parse_pr_url("https://github.com/owner/repo/pull/123\n").unwrap();
        assert_eq!(m.get("number").unwrap().as_int().unwrap(), 123);
        assert_eq!(
            m.get("url").unwrap().clone().into_string().unwrap(),
            "https://github.com/owner/repo/pull/123"
        );
    }

    #[test]
    fn issue_list_opts_to_args_translates_keys() {
        let mut opts = rhai::Map::new();
        opts.insert("state".into(), "closed".to_string().into());
        opts.insert("label".into(), "bug".to_string().into());
        opts.insert("limit".into(), 25i64.into());
        let args = issue_list_opts_to_args(&opts).unwrap();
        assert!(args.contains(&"--state".to_string()) && args.contains(&"closed".to_string()));
        assert!(args.contains(&"--label".to_string()) && args.contains(&"bug".to_string()));
        assert!(args.contains(&"--limit".to_string()) && args.contains(&"25".to_string()));
    }

    #[test]
    fn issue_create_opts_rejects_missing_title() {
        let opts = rhai::Map::new();
        let result = issue_create_opts_to_args(&opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("title is required"));
    }

    #[test]
    fn parse_issue_url_extracts_number_and_url() {
        let m = parse_issue_url("https://github.com/owner/repo/issues/42\n").unwrap();
        assert_eq!(m.get("number").unwrap().as_int().unwrap(), 42);
        assert_eq!(
            m.get("url").unwrap().clone().into_string().unwrap(),
            "https://github.com/owner/repo/issues/42"
        );
    }
}
