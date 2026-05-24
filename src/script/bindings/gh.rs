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

use rhai::Engine;

pub fn register(_engine: &mut Engine) {
    // bodies land in later tasks
}
