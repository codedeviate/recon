//! Claude Code CLI backend. Invocation:
//! `claude -p --output-format text [--model M] [--system-prompt S]`
//! Prompt body piped on stdin. The CLI uses the user's existing
//! Claude Code session for auth.

use super::super::backend::{AiBackend, BackendCtx, Response};
use super::super::flatten::{flatten_for_subprocess, SystemDelivery};
use super::super::request::Request;
use super::super::runner::run;

pub struct ClaudeBackend;

impl ClaudeBackend {
    pub fn build_argv(model: Option<&str>, system: Option<&str>) -> Vec<String> {
        let mut argv = vec![
            "claude".to_string(),
            "-p".to_string(),
            "--output-format".to_string(),
            "text".to_string(),
        ];
        if let Some(m) = model {
            argv.push("--model".to_string());
            argv.push(m.to_string());
        }
        if let Some(s) = system {
            argv.push("--system-prompt".to_string());
            argv.push(s.to_string());
        }
        argv
    }
}

impl AiBackend for ClaudeBackend {
    fn name(&self) -> &'static str { "claude" }

    fn invoke(&self, req: &Request, ctx: &BackendCtx<'_>) -> Result<Response, String> {
        let payload = flatten_for_subprocess(req, SystemDelivery::Flag);
        let argv = Self::build_argv(ctx.effective_model.as_deref(), payload.system.as_deref());
        match run(&argv, &payload.body, ctx.effective_timeout) {
            Ok(r) => Ok(Response {
                text: r.stdout.trim_end_matches('\n').to_string(),
                backend: "claude".into(),
                model: ctx.effective_model.clone(),
                duration: r.duration,
                exit_code: r.exit_code,
                chars_in: payload.char_count(),
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
        let argv = ClaudeBackend::build_argv(None, None);
        assert_eq!(argv, vec!["claude", "-p", "--output-format", "text"]);
    }

    #[test]
    fn argv_with_model_only() {
        let argv = ClaudeBackend::build_argv(Some("sonnet"), None);
        assert_eq!(
            argv,
            vec!["claude", "-p", "--output-format", "text", "--model", "sonnet"]
        );
    }

    #[test]
    fn argv_with_system_only() {
        let argv = ClaudeBackend::build_argv(None, Some("be brief"));
        assert_eq!(
            argv,
            vec!["claude", "-p", "--output-format", "text", "--system-prompt", "be brief"]
        );
    }

    #[test]
    fn argv_with_both() {
        let argv = ClaudeBackend::build_argv(Some("opus"), Some("be brief"));
        assert_eq!(
            argv,
            vec![
                "claude", "-p", "--output-format", "text",
                "--model", "opus",
                "--system-prompt", "be brief",
            ]
        );
    }
}
