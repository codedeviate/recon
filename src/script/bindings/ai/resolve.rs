//! Precedence resolution for backend / model / timeout selection.
//! Highest priority wins. Env reads go through `EnvSource` so tests
//! can drive them deterministically.

use std::time::Duration;

use crate::config::AiConfig;
use super::request::Request;

/// Trait so tests don't have to mutate process env.
pub trait EnvSource {
    fn get(&self, key: &str) -> Option<String>;
}

pub struct ProcessEnv;
impl EnvSource for ProcessEnv {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, PartialEq, Eq)]
pub struct Resolved {
    pub backend: String,
    pub model: Option<String>,
    pub timeout: Duration,
}

pub fn resolve(
    req: &Request,
    config: &AiConfig,
    env: &dyn EnvSource,
) -> Result<Resolved, String> {
    // Backend: per-request → env → config → error
    let backend = req
        .backend
        .clone()
        .or_else(|| env.get("RECON_AI_BACKEND"))
        .or_else(|| config.default_backend.clone())
        .ok_or_else(|| {
            "ai: backend not configured (set RECON_AI_BACKEND, [ai].default_backend, \
             or .backend() in the script)".to_string()
        })?;

    // Model: per-request → env → per-backend config → default config → None
    let model = req
        .model
        .clone()
        .or_else(|| env.get("RECON_AI_MODEL"))
        .or_else(|| {
            config
                .backends
                .get(&backend)
                .and_then(|b| b.model.clone())
        })
        .or_else(|| config.default_model.clone());

    // Timeout: per-request → env → config → DEFAULT_TIMEOUT
    let timeout = req
        .timeout
        .or_else(|| {
            env.get("RECON_AI_TIMEOUT")
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs)
        })
        .or_else(|| config.timeout_secs.map(Duration::from_secs))
        .unwrap_or(DEFAULT_TIMEOUT);

    Ok(Resolved { backend, model, timeout })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MapEnv(HashMap<&'static str, &'static str>);
    impl EnvSource for MapEnv {
        fn get(&self, key: &str) -> Option<String> {
            self.0.get(key).map(|s| s.to_string())
        }
    }

    fn empty_env() -> MapEnv { MapEnv(HashMap::new()) }

    #[test]
    fn unset_everything_errors() {
        let req = Request::new();
        let cfg = AiConfig::default();
        let err = resolve(&req, &cfg, &empty_env()).unwrap_err();
        assert!(err.contains("not configured"), "got: {err}");
    }

    #[test]
    fn per_request_backend_wins_over_env() {
        let mut req = Request::new();
        req.backend = Some("from-req".into());
        let cfg = AiConfig::default();
        let env = MapEnv(HashMap::from([("RECON_AI_BACKEND", "from-env")]));
        let r = resolve(&req, &cfg, &env).unwrap();
        assert_eq!(r.backend, "from-req");
    }

    #[test]
    fn env_backend_wins_over_config() {
        let req = Request::new();
        let cfg = AiConfig { default_backend: Some("from-cfg".into()), ..Default::default() };
        let env = MapEnv(HashMap::from([("RECON_AI_BACKEND", "from-env")]));
        let r = resolve(&req, &cfg, &env).unwrap();
        assert_eq!(r.backend, "from-env");
    }

    #[test]
    fn config_backend_used_when_unset_elsewhere() {
        let req = Request::new();
        let cfg = AiConfig { default_backend: Some("from-cfg".into()), ..Default::default() };
        let r = resolve(&req, &cfg, &empty_env()).unwrap();
        assert_eq!(r.backend, "from-cfg");
    }

    #[test]
    fn per_backend_model_wins_over_default_model() {
        let req = Request::new();
        let mut cfg = AiConfig {
            default_backend: Some("claude".into()),
            default_model: Some("global-default".into()),
            ..Default::default()
        };
        cfg.backends.insert(
            "claude".into(),
            crate::config::AiBackendConfig {
                model: Some("per-backend".into()),
                ..Default::default()
            },
        );
        let r = resolve(&req, &cfg, &empty_env()).unwrap();
        assert_eq!(r.model.as_deref(), Some("per-backend"));
    }

    #[test]
    fn env_model_wins_over_config() {
        let req = Request::new();
        let cfg = AiConfig {
            default_backend: Some("claude".into()),
            default_model: Some("cfg".into()),
            ..Default::default()
        };
        let env = MapEnv(HashMap::from([("RECON_AI_MODEL", "env")]));
        let r = resolve(&req, &cfg, &env).unwrap();
        assert_eq!(r.model.as_deref(), Some("env"));
    }

    #[test]
    fn per_request_model_wins() {
        let mut req = Request::new();
        req.backend = Some("claude".into());
        req.model = Some("req-model".into());
        let cfg = AiConfig::default();
        let env = MapEnv(HashMap::from([("RECON_AI_MODEL", "env-model")]));
        let r = resolve(&req, &cfg, &env).unwrap();
        assert_eq!(r.model.as_deref(), Some("req-model"));
    }

    #[test]
    fn timeout_falls_back_to_60s() {
        let mut req = Request::new();
        req.backend = Some("claude".into());
        let cfg = AiConfig::default();
        let r = resolve(&req, &cfg, &empty_env()).unwrap();
        assert_eq!(r.timeout, DEFAULT_TIMEOUT);
    }

    #[test]
    fn timeout_env_wins_over_config() {
        let mut req = Request::new();
        req.backend = Some("claude".into());
        let cfg = AiConfig { timeout_secs: Some(30), ..Default::default() };
        let env = MapEnv(HashMap::from([("RECON_AI_TIMEOUT", "10")]));
        let r = resolve(&req, &cfg, &env).unwrap();
        assert_eq!(r.timeout, Duration::from_secs(10));
    }

    #[test]
    fn timeout_per_request_wins() {
        let mut req = Request::new();
        req.backend = Some("claude".into());
        req.timeout = Some(Duration::from_secs(5));
        let cfg = AiConfig { timeout_secs: Some(30), ..Default::default() };
        let env = MapEnv(HashMap::from([("RECON_AI_TIMEOUT", "10")]));
        let r = resolve(&req, &cfg, &env).unwrap();
        assert_eq!(r.timeout, Duration::from_secs(5));
    }
}
