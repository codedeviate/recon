//! `gh()` / `gh(repo_spec)` — Rhai-side handle that composes `gh` CLI
//! invocations.
//!
//! Like the `Git` wrapper, methods like `h.pr_list()` / `h.release_view()`
//! pick the right `--json …` fields internally and return parsed Maps.
//! The `.run()` / `.run_text()` / `.run_json()` escape hatches expose
//! anything not promoted.
//!
//! Auto-account-switch: before every `gh` call, the wrapper checks
//! `git config user.email` against an email-to-gh-handle mapping loaded
//! from the `[gh.accounts]` table in the layered `config.toml`. If the
//! config file is missing or has no entry for the current email, no switch
//! happens and the call uses whichever `gh` account is currently active.
//!
//! Config format (in `config.toml`):
//!
//! ```toml
//! [gh.accounts]
//! "you@example.com" = "your-gh-handle"
//! ```
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
            None => return Ok(()), // No mapping; fall back to current gh account.
        };
        // Cache check.
        {
            let cache = self.switched_to.lock().unwrap();
            if cache.as_deref() == Some(expected.as_str()) {
                return Ok(());
            }
        }
        // Switch.
        let switch_result = Command::new("gh")
            .args(["auth", "switch", "--user", &expected])
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
            *self.switched_to.lock().unwrap() = Some(expected);
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

/// Look up the gh handle for `email` in `[gh.accounts]` of the layered
/// `config.toml`. Returns `None` when no config file exists, no match
/// is found, or the lookup fails. Never panics, never throws — the gh
/// binding falls back to the active gh account.
pub(crate) fn account_handle_for_email(email: &str) -> Option<String> {
    let opts = crate::config_resolver::global();
    let value = crate::config_resolver::load_layered("config.toml", &opts).ok()?;
    value.get("gh")?
        .get("accounts")?
        .get(email)?
        .as_str()
        .map(str::to_string)
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

const REPO_VIEW_FIELDS: &str =
    "name,owner,description,defaultBranchRef,visibility,isPrivate,homepageUrl,createdAt,url";
const RUN_LIST_FIELDS: &str =
    "databaseId,name,status,conclusion,workflowName,headBranch,event,createdAt,url";
const RUN_VIEW_FIELDS: &str =
    "databaseId,name,status,conclusion,workflowName,headBranch,event,jobs,startedAt,url";

const ISSUE_LIST_FIELDS: &str =
    "number,title,state,author,labels,assignees,createdAt,url";
const ISSUE_VIEW_FIELDS: &str =
    "number,title,state,author,body,labels,assignees,createdAt,closedAt,url";

const RELEASE_LIST_FIELDS: &str = "name,tagName,createdAt,isDraft,isPrerelease,url";
const RELEASE_VIEW_FIELDS: &str =
    "name,tagName,body,createdAt,publishedAt,isDraft,isPrerelease,assets,url,targetCommitish";

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

fn release_create_opts_to_args(opts: &Map) -> Result<Vec<String>, Box<EvalAltResult>> {
    let mut out = Vec::new();
    let mut have_notes = false;
    let mut have_notes_file = false;
    for (k, v) in opts {
        match k.as_str() {
            "title" => {
                out.push("--title".into());
                out.push(v.clone().into_string().map_err(|_| err("release_create opts: title must be a string"))?);
            }
            "notes" => {
                have_notes = true;
                out.push("--notes".into());
                out.push(v.clone().into_string().map_err(|_| err("release_create opts: notes must be a string"))?);
            }
            "notes_file" => {
                have_notes_file = true;
                out.push("--notes-file".into());
                out.push(v.clone().into_string().map_err(|_| err("release_create opts: notes_file must be a string"))?);
            }
            "generate_notes" => {
                if v.clone().as_bool().unwrap_or(false) {
                    out.push("--generate-notes".into());
                }
            }
            "draft" => {
                if v.clone().as_bool().unwrap_or(false) {
                    out.push("--draft".into());
                }
            }
            "prerelease" => {
                if v.clone().as_bool().unwrap_or(false) {
                    out.push("--prerelease".into());
                }
            }
            "target" => {
                out.push("--target".into());
                out.push(v.clone().into_string().map_err(|_| err("release_create opts: target must be a string"))?);
            }
            other => return Err(err(format!("release_create opts: unknown key '{other}'"))),
        }
    }
    if have_notes && have_notes_file {
        return Err(err("release_create opts: notes and notes_file are mutually exclusive"));
    }
    Ok(out)
}

/// Parse the URL line that `gh release create` emits, returning
/// `{ url, tag }` map.
fn parse_release_url(s: &str) -> Result<Map, Box<EvalAltResult>> {
    let re = regex::Regex::new(r"https://[^\s]+/releases/tag/(\S+)").unwrap();
    let cap = re.captures(s).ok_or_else(|| {
        err(format!("gh.release_create: could not parse URL from output: {s}"))
    })?;
    let url = cap.get(0).unwrap().as_str().to_string();
    let tag = cap[1].to_string();
    let mut m = Map::new();
    m.insert("url".into(), url.into());
    m.insert("tag".into(), tag.into());
    Ok(m)
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

fn run_list_opts_to_args(opts: &Map) -> Result<Vec<String>, Box<EvalAltResult>> {
    let mut out = Vec::new();
    for (k, v) in opts {
        match k.as_str() {
            "workflow" => {
                out.push("--workflow".into());
                out.push(v.clone().into_string().map_err(|_| err("run_list opts: workflow must be a string"))?);
            }
            "branch" => {
                out.push("--branch".into());
                out.push(v.clone().into_string().map_err(|_| err("run_list opts: branch must be a string"))?);
            }
            "status" => {
                out.push("--status".into());
                out.push(v.clone().into_string().map_err(|_| err("run_list opts: status must be a string"))?);
            }
            "limit" => {
                let n = v.clone().as_int().map_err(|_| err("run_list opts: limit must be an integer"))?;
                out.push("--limit".into());
                out.push(n.to_string());
            }
            other => return Err(err(format!("run_list opts: unknown key '{other}'"))),
        }
    }
    Ok(out)
}

/// Parse the textual output of `gh auth status` (no --json equivalent).
///
/// Looks for lines containing:
///   "Logged in to <host> account <name> (...)"
///   "Token scopes: '<scope>', '<scope>', ..."
///
/// Returns `{ host, account, scopes: [...] }`. Missing fields stay
/// absent rather than rendering as empty strings.
fn parse_auth_status(s: &str) -> Map {
    let mut m = Map::new();
    let mut scopes = Array::new();
    for line in s.lines() {
        if let Some(idx) = line.find("Logged in to ") {
            let rest = &line[idx + "Logged in to ".len()..];
            if let Some((host, after)) = rest.split_once(" account ") {
                m.insert("host".into(), host.to_string().into());
                let name = after.split_whitespace().next().unwrap_or("").to_string();
                m.insert("account".into(), name.into());
            }
        } else if let Some(idx) = line.find("Token scopes: ") {
            let rest = &line[idx + "Token scopes: ".len()..];
            for tok in rest.split(',') {
                let s = tok.trim().trim_matches('\'').to_string();
                if !s.is_empty() {
                    scopes.push(s.into());
                }
            }
        }
    }
    m.insert("scopes".into(), scopes.into());
    m
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

    engine.register_fn(
        "release_list",
        |h: &mut Gh| -> Result<Array, Box<EvalAltResult>> {
            let argv = &["release", "list", "--json", RELEASE_LIST_FIELDS];
            let out = h.run(argv, true)?;
            let stdout = ok_or_throw(argv, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.release_list: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Array>().ok_or_else(|| err("gh.release_list: expected Array"))
        },
    );

    engine.register_fn(
        "release_view",
        |h: &mut Gh, tag: &str| -> Result<Map, Box<EvalAltResult>> {
            let argv = &["release", "view", tag, "--json", RELEASE_VIEW_FIELDS];
            let out = h.run(argv, true)?;
            let stdout = ok_or_throw(argv, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.release_view: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Map>().ok_or_else(|| err("gh.release_view: expected Map"))
        },
    );

    engine.register_fn(
        "release_create",
        |h: &mut Gh, tag: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
            let mut argv: Vec<String> = vec!["release".into(), "create".into(), tag.to_string()];
            argv.extend(release_create_opts_to_args(&opts)?);
            let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = h.run(&refs, true)?;
            let stdout = ok_or_throw(&refs, out)?;
            parse_release_url(&stdout)
        },
    );

    engine.register_fn(
        "repo_view",
        |h: &mut Gh| -> Result<Map, Box<EvalAltResult>> {
            let argv = &["repo", "view", "--json", REPO_VIEW_FIELDS];
            let out = h.run(argv, true)?;
            let stdout = ok_or_throw(argv, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.repo_view: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Map>().ok_or_else(|| err("gh.repo_view: expected Map"))
        },
    );
    engine.register_fn(
        "repo_view",
        |h: &mut Gh, spec: &str| -> Result<Map, Box<EvalAltResult>> {
            let argv = &["repo", "view", spec, "--json", REPO_VIEW_FIELDS];
            let out = h.run(argv, true)?;
            let stdout = ok_or_throw(argv, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.repo_view: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Map>().ok_or_else(|| err("gh.repo_view: expected Map"))
        },
    );

    engine.register_fn(
        "run_list",
        |h: &mut Gh| -> Result<Array, Box<EvalAltResult>> {
            let argv = &["run", "list", "--json", RUN_LIST_FIELDS];
            let out = h.run(argv, true)?;
            let stdout = ok_or_throw(argv, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.run_list: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Array>().ok_or_else(|| err("gh.run_list: expected Array"))
        },
    );
    engine.register_fn(
        "run_list",
        |h: &mut Gh, opts: Map| -> Result<Array, Box<EvalAltResult>> {
            let mut argv: Vec<String> = vec![
                "run".into(), "list".into(), "--json".into(), RUN_LIST_FIELDS.into(),
            ];
            argv.extend(run_list_opts_to_args(&opts)?);
            let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
            let out = h.run(&refs, true)?;
            let stdout = ok_or_throw(&refs, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.run_list: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Array>().ok_or_else(|| err("gh.run_list: expected Array"))
        },
    );

    engine.register_fn(
        "run_view",
        |h: &mut Gh, id: i64| -> Result<Map, Box<EvalAltResult>> {
            let id_str = id.to_string();
            let argv = &["run", "view", &id_str, "--json", RUN_VIEW_FIELDS];
            let out = h.run(argv, true)?;
            let stdout = ok_or_throw(argv, out)?;
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .map_err(|e| err(format!("gh.run_view: {e}")))?;
            let d = crate::script::bindings::helpers::json_to_dynamic(v);
            d.try_cast::<Map>().ok_or_else(|| err("gh.run_view: expected Map"))
        },
    );

    engine.register_fn(
        "auth_status",
        |h: &mut Gh| -> Result<Map, Box<EvalAltResult>> {
            let argv = &["auth", "status"];
            // auth_status is the rare gh call that we DON'T want
            // auto-switch on — we want to report whatever's currently
            // active, not change it.
            let out = h.run(argv, false)?;
            let stdout = ok_or_throw(argv, out)?;
            Ok(parse_auth_status(&stdout))
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
        let _: Dynamic = e.eval(r#"gh("owner/name")"#).unwrap();
    }

    #[test]
    fn account_handle_for_email_reads_layered_gh_accounts() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join("config.toml");
        std::fs::write(
            &cfg,
            r#"[gh.accounts]
"alice@example.com" = "alice-gh"
"#,
        )
        .unwrap();

        // We can't reliably set the OnceLock-backed global in a test
        // (other tests may have already set it). Instead exercise the
        // data path: call load_layered with an explicit LayerOpts and
        // confirm [gh.accounts] is reachable.
        let opts = crate::config_resolver::LayerOpts {
            skip_system:   true,
            user_override: Some(cfg),
            ..crate::config_resolver::LayerOpts::default()
        };
        let v = crate::config_resolver::load_layered("config.toml", &opts).unwrap();
        let handle = v.get("gh")
            .and_then(|t| t.get("accounts"))
            .and_then(|t| t.get("alice@example.com"))
            .and_then(|s| s.as_str())
            .map(str::to_string);
        assert_eq!(handle, Some("alice-gh".to_string()));
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

    #[test]
    fn release_create_opts_to_args_handles_flags_and_strings() {
        let mut opts = rhai::Map::new();
        opts.insert("title".into(), "v1".to_string().into());
        opts.insert("generate_notes".into(), true.into());
        opts.insert("draft".into(), true.into());
        let args = release_create_opts_to_args(&opts).unwrap();
        assert!(args.contains(&"--title".to_string()) && args.contains(&"v1".to_string()));
        assert!(args.contains(&"--generate-notes".to_string()));
        assert!(args.contains(&"--draft".to_string()));
    }

    #[test]
    fn release_create_rejects_both_notes_and_notes_file() {
        let mut opts = rhai::Map::new();
        opts.insert("notes".into(), "x".to_string().into());
        opts.insert("notes_file".into(), "/path".to_string().into());
        let result = release_create_opts_to_args(&opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("mutually exclusive"));
    }

    #[test]
    fn parse_release_url_extracts_tag_and_url() {
        let m = parse_release_url("https://github.com/owner/repo/releases/tag/v0.89.0\n").unwrap();
        assert_eq!(
            m.get("tag").unwrap().clone().into_string().unwrap(),
            "v0.89.0"
        );
        assert_eq!(
            m.get("url").unwrap().clone().into_string().unwrap(),
            "https://github.com/owner/repo/releases/tag/v0.89.0"
        );
    }

    #[test]
    fn parse_auth_status_extracts_account_and_scopes() {
        let sample = "github.com\n  ✓ Logged in to github.com account sample-user (keyring)\n  - Token scopes: 'admin:public_key', 'gist', 'read:org', 'repo'\n";
        let m = parse_auth_status(sample);
        assert_eq!(m.get("account").unwrap().clone().into_string().unwrap(), "sample-user");
        assert_eq!(m.get("host").unwrap().clone().into_string().unwrap(), "github.com");
        let scopes: rhai::Array = m.get("scopes").unwrap().clone().cast();
        assert_eq!(scopes.len(), 4);
    }

    #[test]
    fn run_list_opts_to_args_translates_keys() {
        let mut opts = rhai::Map::new();
        opts.insert("workflow".into(), "ci.yml".to_string().into());
        opts.insert("status".into(), "completed".to_string().into());
        opts.insert("limit".into(), 10i64.into());
        let args = run_list_opts_to_args(&opts).unwrap();
        assert!(args.contains(&"--workflow".to_string()));
        assert!(args.contains(&"--status".to_string()));
        assert!(args.contains(&"--limit".to_string()));
        assert!(args.contains(&"10".to_string()));
    }

    #[test]
    fn run_list_opts_rejects_unknown_keys() {
        let mut opts = rhai::Map::new();
        opts.insert("notakey".into(), "x".to_string().into());
        let result = run_list_opts_to_args(&opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown key"));
    }
}
