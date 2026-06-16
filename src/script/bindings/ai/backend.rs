//! Backend trait + dispatcher.

use std::time::Duration;
use std::collections::HashMap;

use crate::config::AiConfig;
use super::request::Request;

/// Per-call context passed to each `AiBackend::invoke`. Holds the
/// effective config and a verbosity level for logging.
pub struct BackendCtx<'a> {
    pub config: &'a AiConfig,
    pub effective_model: Option<String>,
    pub effective_timeout: Duration,
    pub verbose: u8,
}

/// Successful backend response.
#[derive(Debug, Clone)]
pub struct Response {
    pub text: String,
    pub backend: String,
    pub model: Option<String>,
    pub duration: Duration,
    pub exit_code: i32,
    /// Total characters sent on the conceptual payload (body + system).
    /// Populated by each backend from its `FlatPayload`; surfaced only in
    /// the `-v` `.send()` telemetry line, not in the `send_full()` map.
    pub chars_in: usize,
}

/// A backend dispatches a `Request` to an underlying CLI / API and
/// returns a `Response`. Implementations live in `backends/<name>.rs`.
pub trait AiBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn invoke(&self, req: &Request, ctx: &BackendCtx<'_>) -> Result<Response, String>;
}

/// Registry of available backends. Built-in backends are registered
/// at engine startup; the `cmd` backend is materialized on demand
/// from `[ai.backends.<name>]` config entries.
pub struct Registry {
    built_ins: HashMap<&'static str, Box<dyn AiBackend>>,
}

impl Registry {
    /// Returns an empty registry. Built-in backends are added by
    /// `with_built_ins` in later tasks.
    pub fn empty() -> Self {
        Self { built_ins: HashMap::new() }
    }

    pub fn register(&mut self, backend: Box<dyn AiBackend>) {
        self.built_ins.insert(backend.name(), backend);
    }

    pub fn get(&self, name: &str) -> Option<&dyn AiBackend> {
        self.built_ins.get(name).map(|b| b.as_ref())
    }

    pub fn has(&self, name: &str) -> bool {
        self.built_ins.contains_key(name)
    }
}

/// Resolves the named backend (built-in or config-defined `cmd`) and
/// invokes it. Returns the `Response` or a tagged error string.
pub fn dispatch(
    backend_name: &str,
    req: &Request,
    config: &AiConfig,
    ctx: &BackendCtx<'_>,
    registry: &Registry,
) -> Result<Response, String> {
    if let Some(b) = registry.get(backend_name) {
        return b.invoke(req, ctx);
    }
    // Fall through to config-defined cmd entries.
    if let Some(bcfg) = config.backends.get(backend_name) {
        if !bcfg.cmd.is_empty() {
            return super::backends::cmd::invoke(backend_name, bcfg, req, ctx);
        }
    }
    Err(format!(
        "ai: backend '{backend_name}' not found (no built-in, and no \
         `[ai.backends.{backend_name}]` with non-empty `cmd` in ~/.recon/config.toml)"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeBackend;
    impl AiBackend for FakeBackend {
        fn name(&self) -> &'static str { "fake" }
        fn invoke(&self, _req: &Request, _ctx: &BackendCtx<'_>) -> Result<Response, String> {
            Ok(Response {
                text: "ok".into(),
                backend: "fake".into(),
                model: None,
                duration: Duration::from_millis(1),
                exit_code: 0,
                chars_in: 0,
            })
        }
    }

    #[test]
    fn registry_round_trip() {
        let mut reg = Registry::empty();
        reg.register(Box::new(FakeBackend));
        assert!(reg.has("fake"));
        assert!(!reg.has("missing"));
        let b = reg.get("fake").expect("present");
        assert_eq!(b.name(), "fake");
    }
}
