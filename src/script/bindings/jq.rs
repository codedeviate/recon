//! `jq(value, filter)` and `jq_all(value, filter)` — apply a jq-style
//! filter to any Rhai Map / Array.
//!
//! `jq` returns the *first* result (or `()` if the filter yields none);
//! `jq_all` returns *all* results as an Array. The split avoids the
//! shape ambiguity of a single auto-shaping method.
//!
//! Backed by the `jaq` 3.x crate family. Dynamic ↔ serde_json
//! conversion reuses the helper that already backs `json_parse` and
//! `json_stringify` in `bindings/helpers.rs`.

use rhai::Engine;

pub fn register(_engine: &mut Engine) {
    // bodies land in Tasks 4 and 5
}
