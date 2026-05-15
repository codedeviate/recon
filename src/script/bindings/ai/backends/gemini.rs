//! Gemini CLI backend. Invocation:
//! `gemini --prompt [--model M] [--system S]`. Prompt on stdin.

use crate::script::bindings::ai::backend::{AiBackend, BackendCtx, Response};
use crate::script::bindings::ai::flatten::{flatten_for_subprocess, SystemDelivery};
use crate::script::bindings::ai::request::Request;
use crate::script::bindings::ai::runner::run;

pub struct GeminiBackend;

impl GeminiBackend {
    pub fn build_argv(model: Option<&str>, system: Option<&str>) -> Vec<String> {
        let mut argv = vec!["gemini".to_string(), "--prompt".to_string()];
        if let Some(m) = model {
            argv.push("--model".to_string());
            argv.push(m.to_string());
        }
        if let Some(s) = system {
            argv.push("--system".to_string());
            argv.push(s.to_string());
        }
        argv
    }
}

impl AiBackend for GeminiBackend {
    fn name(&self) -> &'static str { "gemini" }

    fn invoke(&self, req: &Request, ctx: &BackendCtx<'_>) -> Result<Response, String> {
        let payload = flatten_for_subprocess(req, SystemDelivery::Flag);
        let argv = Self::build_argv(ctx.effective_model.as_deref(), payload.system.as_deref());
        match run(&argv, &payload.body, ctx.effective_timeout) {
            Ok(r) => Ok(Response {
                text: r.stdout.trim_end_matches('\n').to_string(),
                backend: "gemini".into(),
                model: ctx.effective_model.clone(),
                duration: r.duration,
                exit_code: r.exit_code,
            }),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argv_minimal() {
        assert_eq!(GeminiBackend::build_argv(None, None), vec!["gemini", "--prompt"]);
    }

    #[test]
    fn argv_with_model() {
        assert_eq!(
            GeminiBackend::build_argv(Some("gemini-2.0-flash"), None),
            vec!["gemini", "--prompt", "--model", "gemini-2.0-flash"]
        );
    }

    #[test]
    fn argv_with_system() {
        assert_eq!(
            GeminiBackend::build_argv(None, Some("be brief")),
            vec!["gemini", "--prompt", "--system", "be brief"]
        );
    }
}
