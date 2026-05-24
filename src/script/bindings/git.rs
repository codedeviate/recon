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

use rhai::Engine;

pub fn register(_engine: &mut Engine) {
    // bodies land in later tasks
}
