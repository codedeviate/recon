//! GitHub Copilot CLI backend. Invocation:
//! `copilot -s --no-color [--model M]`. System prompt is inlined into
//! the stdin body (the standalone `copilot` CLI has no dedicated
//! system-prompt flag). Prompt body piped on stdin.
//!
//! `-s` (silent) suppresses session metadata so stdout is just the
//! model's reply. `--no-color` strips ANSI codes so the response is
//! machine-parseable. Either form works for input — `-p "PROMPT"` or
//! stdin — and recon uses stdin to dodge argv length limits when the
//! prompt accumulates context blocks.
//!
//! Auth: `GH_TOKEN` / `GITHUB_TOKEN` env var with Copilot Requests
//! permission, or interactive `copilot /login` set up beforehand.
//! Recommended model values include `auto`, `gpt-5.3-codex`,
//! `claude-sonnet-4.6`, `claude-haiku-4.5`.

use super::super::backend::{AiBackend, BackendCtx, Response};
use super::super::flatten::{flatten_for_subprocess, SystemDelivery};
use super::super::request::Request;
use super::super::runner::run;

pub struct CopilotBackend;

impl CopilotBackend {
    pub fn build_argv(model: Option<&str>) -> Vec<String> {
        let mut argv = vec![
            "copilot".to_string(),
            "-s".to_string(),
            "--no-color".to_string(),
        ];
        if let Some(m) = model {
            argv.push("--model".to_string());
            argv.push(m.to_string());
        }
        argv
    }
}

impl AiBackend for CopilotBackend {
    fn name(&self) -> &'static str { "copilot" }

    fn invoke(&self, req: &Request, ctx: &BackendCtx<'_>) -> Result<Response, String> {
        let payload = flatten_for_subprocess(req, SystemDelivery::Inline);
        let argv = Self::build_argv(ctx.effective_model.as_deref());
        match run(&argv, &payload.body, ctx.effective_timeout) {
            Ok(r) => Ok(Response {
                text: r.stdout.trim_end_matches('\n').to_string(),
                backend: "copilot".into(),
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
        assert_eq!(
            CopilotBackend::build_argv(None),
            vec!["copilot", "-s", "--no-color"]
        );
    }

    #[test]
    fn argv_with_model() {
        assert_eq!(
            CopilotBackend::build_argv(Some("claude-sonnet-4.6")),
            vec!["copilot", "-s", "--no-color", "--model", "claude-sonnet-4.6"]
        );
    }

    #[test]
    fn argv_with_auto_model() {
        assert_eq!(
            CopilotBackend::build_argv(Some("auto")),
            vec!["copilot", "-s", "--no-color", "--model", "auto"]
        );
    }
}
