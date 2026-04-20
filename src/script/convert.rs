//! Error + type conversion helpers used by all script bindings.
//!
//! - `anyhow_to_rhai` walks an `anyhow::Error` for a `ProtocolExitCode` tag,
//!   stashes the exit code in a thread-local, and formats the full chain as
//!   the Rhai error message.
//! - `take_protocol_exit_code` / `clear_protocol_exit_code` are read by the
//!   engine's top-level error path so an uncaught probe exception produces the
//!   correct process exit (e.g. 7 for connection-refused, 28 for timeout).
//! - `opts_get_*` pull typed values from Rhai maps with sensible defaults.

#![allow(dead_code)] // helpers consumed by probe bindings landed in later tasks

use crate::mqtt::ProtocolExitCode;
use rhai::{Dynamic, EvalAltResult, Map};
use std::cell::Cell;

thread_local! {
    static LAST_PROTOCOL_EXIT_CODE: Cell<Option<i32>> = const { Cell::new(None) };
}

/// Convert an `anyhow::Error` into a Rhai runtime error. If the error chain
/// contains a `ProtocolExitCode` tag, stash the numeric code so the engine's
/// error handler can use it as the process exit code.
pub fn anyhow_to_rhai(e: anyhow::Error) -> Box<EvalAltResult> {
    let mut code: Option<i32> = None;
    if let Some(c) = e.downcast_ref::<ProtocolExitCode>() {
        code = Some(*c as i32);
    } else {
        for cause in e.chain() {
            if let Some(c) = cause.downcast_ref::<ProtocolExitCode>() {
                code = Some(*c as i32);
                break;
            }
        }
    }
    if let Some(c) = code {
        LAST_PROTOCOL_EXIT_CODE.with(|cell| cell.set(Some(c)));
    }
    format!("{e:#}").into()
}

/// Take (and clear) the last stashed protocol exit code, if any.
pub fn take_protocol_exit_code() -> Option<i32> {
    LAST_PROTOCOL_EXIT_CODE.with(|cell| cell.take())
}

/// Clear any stashed protocol exit code. Called before evaluation so
/// a previous run's state doesn't leak into a new engine invocation.
pub fn clear_protocol_exit_code() {
    LAST_PROTOCOL_EXIT_CODE.with(|cell| cell.set(None));
}

// ── Opts-map field readers ────────────────────────────────────────────────

pub fn opts_get_str(opts: &Map, key: &str) -> Option<String> {
    opts.get(key).and_then(|v| {
        if v.is_string() {
            Some(v.clone().into_string().unwrap_or_default())
        } else {
            None
        }
    })
}

pub fn opts_get_u64(opts: &Map, key: &str) -> Option<u64> {
    opts.get(key).and_then(|v| v.as_int().ok()).and_then(|n| {
        if n < 0 {
            None
        } else {
            Some(n as u64)
        }
    })
}

pub fn opts_get_i64(opts: &Map, key: &str) -> Option<i64> {
    opts.get(key).and_then(|v| v.as_int().ok())
}

pub fn opts_get_bool(opts: &Map, key: &str) -> Option<bool> {
    opts.get(key).and_then(|v| v.as_bool().ok())
}

/// Returns a cloned Map if the key holds one; None otherwise.
pub fn opts_clone_map(opts: &Map, key: &str) -> Option<Map> {
    opts.get(key).and_then(|v| {
        if v.is_map() {
            v.clone().try_cast::<Map>()
        } else {
            None
        }
    })
}

/// Returns a cloned Array if the key holds one; None otherwise.
pub fn opts_clone_array(opts: &Map, key: &str) -> Option<rhai::Array> {
    opts.get(key).and_then(|v| {
        if v.is_array() {
            v.clone().try_cast::<rhai::Array>()
        } else {
            None
        }
    })
}

/// Convenience: produce a Rhai error from a plain string.
pub fn err(msg: impl Into<String>) -> Box<EvalAltResult> {
    msg.into().into()
}

/// Coerce a Dynamic value to a String.
pub fn to_string(v: &Dynamic) -> String {
    if v.is_string() {
        v.clone().into_string().unwrap_or_default()
    } else {
        v.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn anyhow_to_rhai_preserves_message() {
        let e = anyhow!("boom").context("context");
        let rhai_err = anyhow_to_rhai(e);
        assert!(rhai_err.to_string().contains("boom"));
        assert!(rhai_err.to_string().contains("context"));
    }

    #[test]
    fn anyhow_to_rhai_stashes_protocol_exit_code() {
        clear_protocol_exit_code();
        let e = anyhow!("connection refused").context(ProtocolExitCode::CouldntConnect);
        let _ = anyhow_to_rhai(e);
        assert_eq!(take_protocol_exit_code(), Some(7));
    }

    #[test]
    fn anyhow_to_rhai_without_tag_leaves_none() {
        clear_protocol_exit_code();
        let e = anyhow!("generic");
        let _ = anyhow_to_rhai(e);
        assert_eq!(take_protocol_exit_code(), None);
    }

    #[test]
    fn opts_get_str_reads_value() {
        let mut m = Map::new();
        m.insert("method".into(), "POST".into());
        assert_eq!(opts_get_str(&m, "method"), Some("POST".to_string()));
        assert_eq!(opts_get_str(&m, "absent"), None);
    }

    #[test]
    fn opts_get_u64_reads_non_negative_int() {
        let mut m = Map::new();
        m.insert("timeout_ms".into(), (500_i64).into());
        m.insert("neg".into(), (-1_i64).into());
        assert_eq!(opts_get_u64(&m, "timeout_ms"), Some(500));
        assert_eq!(opts_get_u64(&m, "neg"), None);
        assert_eq!(opts_get_u64(&m, "missing"), None);
    }

    #[test]
    fn opts_get_bool_reads_value() {
        let mut m = Map::new();
        m.insert("insecure".into(), true.into());
        assert_eq!(opts_get_bool(&m, "insecure"), Some(true));
        assert_eq!(opts_get_bool(&m, "absent"), None);
    }

    #[test]
    fn opts_clone_map_returns_nested() {
        let mut inner = Map::new();
        inner.insert("X-Foo".into(), "bar".into());
        let mut m = Map::new();
        m.insert("headers".into(), inner.into());
        let cloned = opts_clone_map(&m, "headers").expect("headers is a map");
        assert_eq!(opts_get_str(&cloned, "X-Foo"), Some("bar".to_string()));
    }
}
