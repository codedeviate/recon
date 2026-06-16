//! User-defined backend driven by `[ai.backends.<name>]` config entries.
//!
//! The config supplies the argv. Optional `model_flag` and `system_flag`
//! cause `--model VAL` / `--system VAL` (or whatever names the user chose)
//! to be appended when those fields are set. Prompt body goes to stdin.

use crate::config::AiBackendConfig;
use super::super::backend::{BackendCtx, Response};
use super::super::flatten::{flatten_for_subprocess, SystemDelivery};
use super::super::request::Request;
use super::super::runner::run;

/// Build argv from a config entry plus the effective model / system.
pub fn build_argv(cfg: &AiBackendConfig, model: Option<&str>, system: Option<&str>) -> Vec<String> {
    let mut argv = cfg.cmd.clone();
    if let (Some(flag), Some(val)) = (cfg.model_flag.as_deref(), model) {
        argv.push(flag.to_string());
        argv.push(val.to_string());
    }
    if let (Some(flag), Some(val)) = (cfg.system_flag.as_deref(), system) {
        argv.push(flag.to_string());
        argv.push(val.to_string());
    }
    argv
}

/// Invoke a `[ai.backends.<name>]` entry. `name` is used only for the
/// returned `Response.backend` field and error messages.
pub fn invoke(
    name: &str,
    cfg: &AiBackendConfig,
    req: &Request,
    ctx: &BackendCtx<'_>,
) -> Result<Response, String> {
    if cfg.cmd.is_empty() {
        return Err(format!(
            "ai: backend '{name}' has no `cmd = [...]` in config — \
             cannot dispatch a user-defined backend without argv"
        ));
    }
    let payload = flatten_for_subprocess(req, SystemDelivery::Flag);
    let chars_in = payload.char_count(); // capture before `payload.body` is moved below
    let argv = build_argv(cfg, ctx.effective_model.as_deref(), payload.system.as_deref());

    // For the cmd backend, the system prompt may need inlining if the
    // user didn't supply a `system_flag`.
    let body = if cfg.system_flag.is_none() {
        if let Some(s) = &payload.system {
            format!("System: {s}\n\n{}", payload.body)
        } else {
            payload.body
        }
    } else {
        payload.body
    };

    match run(&argv, &body, ctx.effective_timeout) {
        Ok(r) => Ok(Response {
            text: r.stdout.trim_end_matches('\n').to_string(),
            backend: name.to_string(),
            model: ctx.effective_model.clone(),
            duration: r.duration,
            exit_code: r.exit_code,
            chars_in,
        }),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(argv: &[&str], model_flag: Option<&str>, system_flag: Option<&str>) -> AiBackendConfig {
        AiBackendConfig {
            cmd: argv.iter().map(|s| s.to_string()).collect(),
            model: None,
            model_flag: model_flag.map(String::from),
            system_flag: system_flag.map(String::from),
        }
    }

    #[test]
    fn argv_minimal_uses_only_cmd() {
        let c = cfg(&["foo", "--print"], None, None);
        assert_eq!(build_argv(&c, None, None), vec!["foo", "--print"]);
    }

    #[test]
    fn argv_appends_model_when_both_present() {
        let c = cfg(&["foo"], Some("-m"), None);
        assert_eq!(
            build_argv(&c, Some("turbo"), None),
            vec!["foo", "-m", "turbo"]
        );
    }

    #[test]
    fn argv_skips_model_when_flag_unset() {
        let c = cfg(&["foo"], None, None);
        assert_eq!(build_argv(&c, Some("turbo"), None), vec!["foo"]);
    }

    #[test]
    fn argv_skips_model_when_value_unset() {
        let c = cfg(&["foo"], Some("-m"), None);
        assert_eq!(build_argv(&c, None, None), vec!["foo"]);
    }

    #[test]
    fn argv_appends_system_when_both_present() {
        let c = cfg(&["foo"], None, Some("--sys"));
        assert_eq!(
            build_argv(&c, None, Some("be brief")),
            vec!["foo", "--sys", "be brief"]
        );
    }

    #[test]
    fn argv_appends_model_then_system_in_order() {
        let c = cfg(&["foo"], Some("-m"), Some("--sys"));
        assert_eq!(
            build_argv(&c, Some("v1"), Some("brief")),
            vec!["foo", "-m", "v1", "--sys", "brief"]
        );
    }
}
