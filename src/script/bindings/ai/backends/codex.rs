//! Codex CLI backend. Invocation: `codex exec [--model M]`.
//! System prompt is inlined into the stdin body (no dedicated flag
//! in the codex CLI as of writing).

use super::super::backend::{AiBackend, BackendCtx, Response};
use super::super::flatten::{flatten_for_subprocess, SystemDelivery};
use super::super::request::Request;
use super::super::runner::run;

pub struct CodexBackend;

impl CodexBackend {
    pub fn build_argv(model: Option<&str>) -> Vec<String> {
        let mut argv = vec!["codex".to_string(), "exec".to_string()];
        if let Some(m) = model {
            argv.push("--model".to_string());
            argv.push(m.to_string());
        }
        argv
    }
}

impl AiBackend for CodexBackend {
    fn name(&self) -> &'static str { "codex" }

    fn invoke(&self, req: &Request, ctx: &BackendCtx<'_>) -> Result<Response, String> {
        let payload = flatten_for_subprocess(req, SystemDelivery::Inline);
        let argv = Self::build_argv(ctx.effective_model.as_deref());
        match run(&argv, &payload.body, ctx.effective_timeout) {
            Ok(r) => Ok(Response {
                text: r.stdout.trim_end_matches('\n').to_string(),
                backend: "codex".into(),
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
        assert_eq!(CodexBackend::build_argv(None), vec!["codex", "exec"]);
    }

    #[test]
    fn argv_with_model() {
        assert_eq!(
            CodexBackend::build_argv(Some("gpt-5")),
            vec!["codex", "exec", "--model", "gpt-5"]
        );
    }
}
