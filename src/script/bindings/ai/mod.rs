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
use request::Request;
use resolve::{resolve, ProcessEnv};

/// State shared between every `ai::*` invocation inside one engine.
struct AiState {
    config: AiConfig,
    registry: Registry,
}

fn build_state() -> AiState {
    let config = crate::config::load()
        .ok()
        .and_then(|c| c.ai)
        .unwrap_or_default();

    let mut registry = Registry::empty();
    registry.register(Box::new(backends::claude::ClaudeBackend));
    registry.register(Box::new(backends::codex::CodexBackend));
    registry.register(Box::new(backends::copilot::CopilotBackend));
    registry.register(Box::new(backends::gemini::GeminiBackend));

    AiState { config, registry }
}

pub fn register(engine: &mut Engine) {
    let state = Arc::new(build_state());
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
        verbose: 0,
    };
    dispatch(&resolved.backend, req, &state.config, &ctx, &state.registry)
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
pub fn register_with_registry(engine: &mut Engine, registry: Registry, config: AiConfig) {
    let state = Arc::new(AiState { config, registry });
    register_with_state(engine, state);
}
