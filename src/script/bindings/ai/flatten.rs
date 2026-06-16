//! Build the stdin payload for a subprocess-driven backend.
//!
//! Returns a tuple of (system_text, stdin_body).
//! When the backend has a `--system-prompt`-style flag, `system_text` is
//! passed there. Otherwise it's prepended to `stdin_body`. Either way
//! the produced text is consistent across backends.

use super::request::{Request, Turn};

/// Layout choice for the system prompt. `Flag` means the backend will
/// pass `system_text` to a CLI flag separately. `Inline` means the
/// backend will prepend it to the stdin body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemDelivery {
    Flag,
    Inline,
}

/// Output of `flatten_for_subprocess`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatPayload {
    /// The system prompt, if any. Caller decides whether to pass via
    /// `--system-prompt`-style flag or to inline (see `SystemDelivery`).
    pub system: Option<String>,
    /// The text body that should be piped to stdin (after the system
    /// prompt was either flagged or inlined).
    pub body: String,
}

impl FlatPayload {
    /// Total character count of the conceptual payload — body plus the
    /// system prompt when present. Independent of `SystemDelivery`: only
    /// the `"System: "` inline wrapper differs between modes and it is
    /// deliberately not counted here, so `chars_in` is stable across
    /// backends. Used for the `-v` `.send()` telemetry line.
    pub fn char_count(&self) -> usize {
        self.body.chars().count()
            + self.system.as_ref().map_or(0, |s| s.chars().count())
    }
}

pub fn flatten_for_subprocess(req: &Request, delivery: SystemDelivery) -> FlatPayload {
    let body = build_body(req);
    match delivery {
        SystemDelivery::Flag => FlatPayload {
            system: req.system.clone(),
            body,
        },
        SystemDelivery::Inline => {
            let body = match &req.system {
                Some(sys) => format!("System: {sys}\n\n{body}"),
                None => body,
            };
            FlatPayload { system: None, body }
        }
    }
}

fn build_body(req: &Request) -> String {
    let mut out = String::new();

    if !req.contexts.is_empty() {
        for ctx in &req.contexts {
            out.push_str(ctx);
            out.push_str("\n\n");
        }
    }

    // Multi-turn? Render the prior turns as a labelled transcript, then
    // the final user turn as the "current prompt".
    if req.turns.len() > 1 {
        out.push_str("[prior conversation:]\n");
        for (i, t) in req.turns.iter().enumerate() {
            if i == req.turns.len() - 1 {
                break; // last turn handled below as the current prompt
            }
            match t {
                Turn::User(s) => out.push_str(&format!("User: {s}\n")),
                Turn::Assistant(s) => out.push_str(&format!("Assistant: {s}\n")),
            }
        }
        out.push_str("\n[current prompt:]\n");
    }

    if let Some(Turn::User(last)) = req.turns.last() {
        out.push_str(last);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req_with(prompt: &str) -> Request {
        let mut r = Request::new();
        r.set_user(prompt);
        r
    }

    #[test]
    fn single_turn_inline_system_only() {
        let mut r = req_with("hello");
        r.set_system("you are helpful");
        let p = flatten_for_subprocess(&r, SystemDelivery::Inline);
        assert_eq!(p.system, None);
        assert_eq!(p.body, "System: you are helpful\n\nhello");
    }

    #[test]
    fn single_turn_flag_system_only() {
        let mut r = req_with("hello");
        r.set_system("you are helpful");
        let p = flatten_for_subprocess(&r, SystemDelivery::Flag);
        assert_eq!(p.system.as_deref(), Some("you are helpful"));
        assert_eq!(p.body, "hello");
    }

    #[test]
    fn char_count_sums_body_and_system() {
        // Flag mode keeps system separate; char_count includes both.
        let mut r = req_with("hello"); // body "hello" = 5
        r.set_system("hi"); // system "hi" = 2
        let p = flatten_for_subprocess(&r, SystemDelivery::Flag);
        assert_eq!(p.char_count(), 7);

        // No system → just the body.
        let p2 = flatten_for_subprocess(&req_with("héllo"), SystemDelivery::Flag);
        assert_eq!(p2.char_count(), 5); // chars, not bytes
    }

    #[test]
    fn single_turn_no_system_no_context() {
        let r = req_with("just this");
        let p = flatten_for_subprocess(&r, SystemDelivery::Flag);
        assert_eq!(p.system, None);
        assert_eq!(p.body, "just this");
    }

    #[test]
    fn contexts_accumulate_in_order() {
        let mut r = req_with("analyze");
        r.push_context("cert: AAA");
        r.push_context("probe: BBB");
        let p = flatten_for_subprocess(&r, SystemDelivery::Flag);
        assert_eq!(p.body, "cert: AAA\n\nprobe: BBB\n\nanalyze");
    }

    #[test]
    fn multi_turn_renders_transcript() {
        let mut r = Request::new();
        r.set_user("Q1");
        r.push_assistant("A1").unwrap();
        r.set_user("Q2");
        r.push_assistant("A2").unwrap();
        r.set_user("Q3");
        let p = flatten_for_subprocess(&r, SystemDelivery::Flag);
        assert_eq!(
            p.body,
            "[prior conversation:]\nUser: Q1\nAssistant: A1\nUser: Q2\nAssistant: A2\n\n[current prompt:]\nQ3"
        );
    }

    #[test]
    fn multi_turn_with_system_and_context_inline() {
        let mut r = Request::new();
        r.set_system("be concise");
        r.push_context("ctx1");
        r.set_user("Q1");
        r.push_assistant("A1").unwrap();
        r.set_user("Q2");
        let p = flatten_for_subprocess(&r, SystemDelivery::Inline);
        let expected = "System: be concise\n\n\
                        ctx1\n\n\
                        [prior conversation:]\n\
                        User: Q1\n\
                        Assistant: A1\n\
                        \n[current prompt:]\n\
                        Q2";
        assert_eq!(p.body, expected);
    }
}
