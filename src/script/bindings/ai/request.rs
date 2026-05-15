//! `ai::request()` builder type, exposed to Rhai. Pure data — no I/O.

use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Turn {
    User(String),
    Assistant(String),
}

/// Builder state for one `ai::request()`. Mutated in place by Rhai
/// setter methods; cloned cheaply for chained-return semantics.
#[derive(Debug, Clone, Default)]
pub struct Request {
    pub backend: Option<String>,
    pub model: Option<String>,
    pub system: Option<String>,
    pub contexts: Vec<String>,
    pub turns: Vec<Turn>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub timeout: Option<Duration>,
}

impl Request {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the system prompt; replaces any prior value.
    pub fn set_system(&mut self, s: impl Into<String>) {
        self.system = Some(s.into());
    }

    /// Appends a context block. Multiple calls accumulate in order.
    pub fn push_context(&mut self, s: impl Into<String>) {
        self.contexts.push(s.into());
    }

    /// Sets the current user turn. If the last entry in `turns` is
    /// already a `User`, replaces it. Otherwise appends a new `User`.
    pub fn set_user(&mut self, s: impl Into<String>) {
        let s = s.into();
        if let Some(Turn::User(last)) = self.turns.last_mut() {
            *last = s;
        } else {
            self.turns.push(Turn::User(s));
        }
    }

    /// Appends an `Assistant` turn. Errors if the last entry is
    /// already an Assistant (alternation invariant).
    pub fn push_assistant(&mut self, s: impl Into<String>) -> Result<(), String> {
        if matches!(self.turns.last(), Some(Turn::Assistant(_))) {
            return Err("ai: cannot append assistant turn — last turn is already assistant".into());
        }
        self.turns.push(Turn::Assistant(s.into()));
        Ok(())
    }

    /// Validates the request is ready to send. Used by `.send()` /
    /// `.send_full()`.
    pub fn validate_for_send(&self) -> Result<(), String> {
        match self.turns.last() {
            Some(Turn::User(_)) => Ok(()),
            Some(Turn::Assistant(_)) | None => {
                Err("ai: no user prompt — call .prompt()/.user() before .send()".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_request_is_empty() {
        let r = Request::new();
        assert!(r.backend.is_none());
        assert!(r.system.is_none());
        assert!(r.contexts.is_empty());
        assert!(r.turns.is_empty());
    }

    #[test]
    fn set_system_replaces() {
        let mut r = Request::new();
        r.set_system("a");
        r.set_system("b");
        assert_eq!(r.system.as_deref(), Some("b"));
    }

    #[test]
    fn push_context_accumulates() {
        let mut r = Request::new();
        r.push_context("one");
        r.push_context("two");
        assert_eq!(r.contexts, vec!["one", "two"]);
    }

    #[test]
    fn set_user_appends_if_last_not_user() {
        let mut r = Request::new();
        r.set_user("first");
        assert_eq!(r.turns, vec![Turn::User("first".into())]);
    }

    #[test]
    fn set_user_replaces_trailing_user() {
        let mut r = Request::new();
        r.set_user("first");
        r.set_user("replaced");
        assert_eq!(r.turns, vec![Turn::User("replaced".into())]);
    }

    #[test]
    fn set_user_after_assistant_appends() {
        let mut r = Request::new();
        r.set_user("q1");
        r.push_assistant("a1").unwrap();
        r.set_user("q2");
        assert_eq!(
            r.turns,
            vec![
                Turn::User("q1".into()),
                Turn::Assistant("a1".into()),
                Turn::User("q2".into()),
            ]
        );
    }

    #[test]
    fn push_assistant_after_user_ok() {
        let mut r = Request::new();
        r.set_user("q");
        assert!(r.push_assistant("a").is_ok());
    }

    #[test]
    fn push_assistant_after_assistant_errors() {
        let mut r = Request::new();
        r.set_user("q");
        r.push_assistant("a1").unwrap();
        let err = r.push_assistant("a2").unwrap_err();
        assert!(err.contains("already assistant"), "got: {err}");
    }

    #[test]
    fn validate_for_send_ok_with_user_last() {
        let mut r = Request::new();
        r.set_user("q");
        assert!(r.validate_for_send().is_ok());
    }

    #[test]
    fn validate_for_send_errors_empty() {
        let r = Request::new();
        let err = r.validate_for_send().unwrap_err();
        assert!(err.contains("no user prompt"), "got: {err}");
    }

    #[test]
    fn validate_for_send_errors_when_last_is_assistant() {
        let mut r = Request::new();
        r.set_user("q");
        r.push_assistant("a").unwrap();
        let err = r.validate_for_send().unwrap_err();
        assert!(err.contains("no user prompt"), "got: {err}");
    }
}
