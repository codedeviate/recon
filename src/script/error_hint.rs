//! Friendlier formatting for `EvalAltResult::ErrorFunctionNotFound`.
//!
//! Rhai surfaces "function not found" for two distinct situations:
//!   1. The name really is unknown (typo, never registered).
//!   2. The name is fine, but no overload accepts the runtime argument
//!      types — e.g. calling `json_parse(map)` when only `(string)` is
//!      registered.
//!
//! The default message ("Function not found: json_parse (map)") reads
//! like case 1 in both cases, which has historically sent users
//! chasing a missing import when the real fix is `.body` or a `to_string`.
//!
//! This module rewrites the message for case 2: when sibling overloads
//! exist under the same name, append a hint listing them. Requires the
//! `metadata` feature on the `rhai` crate (already enabled).

use rhai::{Engine, EvalAltResult};

/// Format a Rhai eval error. For most variants this returns the
/// default `Display` output. For `ErrorFunctionNotFound`, if there are
/// other functions registered under the same name, append an "Available
/// overloads:" list and a usage hint.
pub fn format(engine: &Engine, err: &EvalAltResult) -> String {
    let default = err.to_string();
    let EvalAltResult::ErrorFunctionNotFound(sig, _pos) = err else {
        return default;
    };

    // Rhai's signature looks like "name (type1, type2, …)" — split off
    // the prefix before the first space-paren. If the format ever
    // changes, the fallback is the default Display impl.
    let Some((name, called_with)) = sig.split_once(' ') else {
        return default;
    };

    let mut overloads = collect_overloads(engine, name);
    if overloads.is_empty() {
        return default;
    }
    overloads.sort();
    overloads.dedup();

    let mut out = format!(
        "{default}\n\
         note: `{name}` is defined, but no overload accepts {called_with}.\n\
         hint: check that you're passing the expected argument types — \
         e.g. an http() response is a map, so pass `r.body` (string) \
         to functions that take a string."
    );
    out.push_str("\nAvailable overloads:");
    for sig in &overloads {
        out.push_str("\n  ");
        out.push_str(sig);
    }
    out
}

fn collect_overloads(engine: &Engine, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    for sig in engine.gen_fn_signatures(false) {
        let Some(sig_name) = sig.split('(').next() else {
            continue;
        };
        if sig_name.trim() == name {
            out.push(simplify_signature(&sig));
        }
    }
    out
}

/// Drop the Rust return-type suffix from a Rhai signature. The full
/// form (`name(_: string) -> core::result::Result<…, alloc::boxed::Box<…>>`)
/// answers "what is registered" but buries the bit users care about —
/// which arg types the overload accepts.
pub(crate) fn simplify_signature(sig: &str) -> String {
    // Find the matching `)` for the args list, then truncate.
    let bytes = sig.as_bytes();
    let mut depth = 0i32;
    let mut close_args = None;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    close_args = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    match close_args {
        Some(i) => sig[..=i].to_string(),
        None => sig.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Engine;

    #[test]
    fn simplify_strips_rust_return_type() {
        assert_eq!(
            simplify_signature(
                "json_parse(_: string) -> core::result::Result<rhai::types::dynamic::Dynamic,alloc::boxed::Box<rhai::types::error::EvalAltResult>>"
            ),
            "json_parse(_: string)"
        );
        assert_eq!(
            simplify_signature("trim(_: string) -> string"),
            "trim(_: string)"
        );
    }

    #[test]
    fn hint_lists_overloads_for_wrong_arg_types() {
        let mut engine = Engine::new();
        engine.register_fn("greet", |s: &str| -> String { format!("hi {s}") });

        // Build the exact error a real script would produce.
        let err = engine.eval::<String>("greet(42)").unwrap_err();
        let msg = format(&engine, &err);

        assert!(msg.contains("`greet` is defined"));
        assert!(msg.contains("(i64)"));
        assert!(msg.contains("greet(_: string)"));
    }

    #[test]
    fn truly_unknown_name_keeps_default_message() {
        let engine = Engine::new();
        let err = engine.eval::<()>("no_such_thing(1)").unwrap_err();
        let msg = format(&engine, &err);
        // No "note:" hint when the name has zero overloads.
        assert!(!msg.contains("note:"));
        assert!(msg.starts_with("Function not found"));
    }
}
