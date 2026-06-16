//! `ai::*` Rhai bindings — spec at
//! `~/Development/Starweb/superpowers/recon/specs/2026-05-15-ai-script-bindings-design.md`.
//!
//! `#[allow(dead_code)]` suppresses warnings when this tree is compiled
//! via the `lib.rs` `#[path]` re-mount for the test-seam surface. In the
//! binary tree all symbols are reachable, so this allow is a no-op there.
#![allow(dead_code)]

pub mod backend;
pub mod backends;
pub mod flatten;
pub mod request;
pub mod resolve;
pub mod runner;

use std::sync::Arc;
use std::time::Duration;

use rhai::{Engine, EvalAltResult, Map};

use crate::config::AiConfig;
use backend::{dispatch, BackendCtx, Registry, Response};
use request::{Request, Turn};
use resolve::{resolve, ProcessEnv};

/// State shared between every `ai::*` invocation inside one engine.
struct AiState {
    config: AiConfig,
    registry: Registry,
    /// Script-engine verbosity inherited from the CLI `-v` count. Drives
    /// the per-`.send()` telemetry line (0 = silent).
    verbose: u8,
}

fn build_state(verbose: u8) -> AiState {
    let config = crate::config::load()
        .ok()
        .and_then(|c| c.ai)
        .unwrap_or_default();

    let mut registry = Registry::empty();
    registry.register(Box::new(backends::claude::ClaudeBackend));
    registry.register(Box::new(backends::codex::CodexBackend));
    registry.register(Box::new(backends::copilot::CopilotBackend));
    registry.register(Box::new(backends::gemini::GeminiBackend));

    AiState { config, registry, verbose }
}

/// Registers the `ai::*` bindings. `verbose` is the script-engine
/// verbosity (the CLI `-v` count) used for the per-`.send()` telemetry
/// line. Taking the bare `u8` rather than the full `ScriptDefaults`
/// keeps this module's `lib.rs` `#[path]` re-mount free of the
/// `cli::Args` dependency that `ScriptDefaults` carries.
pub fn register(engine: &mut Engine, verbose: u8) {
    let state = Arc::new(build_state(verbose));
    register_with_state(engine, state);
}

fn register_with_state(engine: &mut Engine, state: Arc<AiState>) {
    engine.register_type_with_name::<Request>("AiRequest");

    // Constructors live under the `ai::` namespace so scripts write
    // `ai::request()` and `ai::ask("…")`, matching the encode:: / jwt::
    // pattern used elsewhere in this codebase.
    let mut module = rhai::Module::new();

    module.set_native_fn(
        "request",
        || -> Result<Request, Box<EvalAltResult>> { Ok(Request::new()) },
    );

    {
        let state = state.clone();
        module.set_native_fn(
            "ask",
            move |prompt: &str| -> Result<String, Box<EvalAltResult>> {
                let mut r = Request::new();
                r.set_user(prompt);
                send_string(&mut r, &state)
            },
        );
    }

    engine.register_static_module("ai", module.into());

    // Builder methods. Each takes `&mut Request`, mutates, returns the
    // cloned Request so Rhai can chain.
    engine.register_fn("backend", |req: &mut Request, name: &str| -> Request {
        req.backend = Some(name.into());
        req.clone()
    });
    engine.register_fn("model", |req: &mut Request, name: &str| -> Request {
        req.model = Some(name.into());
        req.clone()
    });
    engine.register_fn("system", |req: &mut Request, s: &str| -> Request {
        req.set_system(s);
        req.clone()
    });
    engine.register_fn("context", |req: &mut Request, s: &str| -> Request {
        req.push_context(s);
        req.clone()
    });
    engine.register_fn("prompt", |req: &mut Request, s: &str| -> Request {
        req.set_user(s);
        req.clone()
    });
    engine.register_fn("user", |req: &mut Request, s: &str| -> Request {
        req.set_user(s);
        req.clone()
    });
    engine.register_fn(
        "assistant",
        |req: &mut Request, s: &str| -> Result<Request, Box<EvalAltResult>> {
            req.push_assistant(s).map_err(rhai_err)?;
            Ok(req.clone())
        },
    );
    engine.register_fn("max_tokens", |req: &mut Request, n: i64| -> Request {
        if n >= 0 {
            req.max_tokens = Some(n as u32);
        }
        req.clone()
    });
    engine.register_fn("temperature", |req: &mut Request, f: f64| -> Request {
        req.temperature = Some(f as f32);
        req.clone()
    });
    engine.register_fn("timeout", |req: &mut Request, secs: i64| -> Request {
        if secs > 0 {
            req.timeout = Some(Duration::from_secs(secs as u64));
        }
        req.clone()
    });

    // Senders ----------------------------------------------------------
    {
        let state = state.clone();
        engine.register_fn(
            "send",
            move |req: &mut Request| -> Result<String, Box<EvalAltResult>> {
                send_string(req, &state)
            },
        );
    }
    {
        let state = state.clone();
        engine.register_fn(
            "send_full",
            move |req: &mut Request| -> Result<Map, Box<EvalAltResult>> {
                send_full(req, &state)
            },
        );
    }
}

fn send_string(req: &mut Request, state: &AiState) -> Result<String, Box<EvalAltResult>> {
    let resp = run_request(req, state).map_err(rhai_err)?;
    Ok(resp.text)
}

fn send_full(req: &mut Request, state: &AiState) -> Result<Map, Box<EvalAltResult>> {
    let resp = run_request(req, state).map_err(rhai_err)?;
    let mut m = Map::new();
    m.insert("text".into(), resp.text.into());
    m.insert("backend".into(), resp.backend.into());
    m.insert(
        "model".into(),
        resp.model
            .map(|s| s.into())
            .unwrap_or_else(|| rhai::Dynamic::UNIT),
    );
    m.insert("duration_ms".into(), (resp.duration.as_millis() as i64).into());
    m.insert("exit_code".into(), (resp.exit_code as i64).into());
    Ok(m)
}

fn run_request(req: &mut Request, state: &AiState) -> Result<Response, String> {
    req.validate_for_send()?;
    let resolved = resolve(req, &state.config, &ProcessEnv)?;
    let ctx = BackendCtx {
        config: &state.config,
        effective_model: resolved.model.clone(),
        effective_timeout: resolved.timeout,
        verbose: state.verbose,
    };
    let resp = dispatch(&resolved.backend, req, &state.config, &ctx, &state.registry)?;

    // Telemetry: one `* ai:` line per successful `.send()` at -v, plus a
    // preamble / stdout-preview line at -vv. Errors emit their own
    // messages, so we only log on the success path.
    if state.verbose >= 1 {
        let chars_out = resp.text.chars().count();
        let final_user_chars = match req.turns.last() {
            Some(Turn::User(s)) => s.chars().count(),
            _ => 0,
        };
        let preamble_len = resp.chars_in.saturating_sub(final_user_chars);
        for line in format_send_log(state.verbose, &resp, chars_out, preamble_len) {
            eprintln!("{line}");
        }
    }
    Ok(resp)
}

/// Render the `-v` / `-vv` telemetry lines for one `.send()`. Pure so it
/// can be unit-tested without capturing stderr. Returns an empty vec when
/// `verbose == 0`.
fn format_send_log(
    verbose: u8,
    resp: &Response,
    chars_out: usize,
    preamble_len: usize,
) -> Vec<String> {
    if verbose == 0 {
        return Vec::new();
    }
    let model = resp.model.as_deref().unwrap_or("default");
    let mut lines = vec![format!(
        "* ai: backend={} model={} duration={:.1}s exit={} chars_in={} chars_out={}",
        resp.backend,
        model,
        resp.duration.as_secs_f64(),
        resp.exit_code,
        resp.chars_in,
        chars_out,
    )];
    if verbose >= 2 {
        let preview: String = resp
            .text
            .chars()
            .take(80)
            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
            .collect();
        lines.push(format!(
            "* ai: preamble={preamble_len} chars; stdout[:80]={preview}"
        ));
    }
    lines
}

fn rhai_err(s: String) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(s.into(), rhai::Position::NONE))
}

/// Test seam — engines built for integration tests inject a registry
/// pre-populated with a `MockBackend`. Production code uses `register`.
///
/// Not gated by `#[cfg(test)]` so that `tests/script_ai_it.rs` (an
/// external test crate in Rust's integration-test model) can import it.
/// Treat this as internal; it is not a public API.
#[doc(hidden)]
pub fn register_with_registry(
    engine: &mut Engine,
    registry: Registry,
    config: AiConfig,
    verbose: u8,
) {
    let state = Arc::new(AiState { config, registry, verbose });
    register_with_state(engine, state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn resp(text: &str, model: Option<&str>, chars_in: usize) -> Response {
        Response {
            text: text.into(),
            backend: "claude".into(),
            model: model.map(|s| s.into()),
            duration: Duration::from_millis(2300),
            exit_code: 0,
            chars_in,
        }
    }

    #[test]
    fn verbose_zero_emits_nothing() {
        let r = resp("hi", Some("sonnet"), 842);
        assert!(format_send_log(0, &r, 2, 100).is_empty());
    }

    #[test]
    fn verbose_one_emits_single_summary_line() {
        let r = resp("hello there", Some("sonnet"), 842);
        let lines = format_send_log(1, &r, 412, 800);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0],
            "* ai: backend=claude model=sonnet duration=2.3s exit=0 chars_in=842 chars_out=412"
        );
    }

    #[test]
    fn missing_model_renders_default() {
        let r = resp("x", None, 10);
        let lines = format_send_log(1, &r, 1, 5);
        assert!(lines[0].contains("model=default"), "got: {}", lines[0]);
    }

    #[test]
    fn verbose_two_adds_preamble_and_preview_with_escaped_newlines() {
        let body = format!("line one\nline two {}", "x".repeat(100));
        let r = resp(&body, Some("m"), 50);
        let lines = format_send_log(2, &r, body.chars().count(), 42);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].starts_with("* ai: preamble=42 chars; stdout[:80]="));
        // First 80 chars only, newline replaced by a space.
        assert!(!lines[1].contains('\n'));
        let preview = lines[1].split("stdout[:80]=").nth(1).unwrap();
        assert_eq!(preview.chars().count(), 80);
        assert!(preview.starts_with("line one line two"));
    }
}
