//! End-to-end test of the `ai::*` Rhai bindings. Uses a `MockBackend`
//! so no real CLI invocation happens.

use std::sync::Mutex;
use std::time::Duration;

use recon_cli::config::AiConfig;
use recon_cli::script::bindings::ai::backend::{AiBackend, BackendCtx, Registry, Response};
use recon_cli::script::bindings::ai::register_with_registry;
use recon_cli::script::bindings::ai::request::Request;

/// Backend that records the request it sees and returns canned text.
struct MockBackend {
    canned: String,
    seen: Mutex<Vec<Request>>,
}
impl MockBackend {
    fn new(canned: &str) -> Self {
        Self { canned: canned.into(), seen: Mutex::new(Vec::new()) }
    }
}
impl AiBackend for MockBackend {
    fn name(&self) -> &'static str { "mock" }
    fn invoke(&self, req: &Request, _ctx: &BackendCtx<'_>) -> Result<Response, String> {
        self.seen.lock().unwrap().push(req.clone());
        Ok(Response {
            text: self.canned.clone(),
            backend: "mock".into(),
            model: None,
            duration: Duration::from_millis(1),
            exit_code: 0,
        })
    }
}

fn engine_with_mock(canned: &str) -> rhai::Engine {
    let mut registry = Registry::empty();
    registry.register(Box::new(MockBackend::new(canned)));
    let mut engine = rhai::Engine::new();
    let mut cfg = AiConfig::default();
    cfg.default_backend = Some("mock".into());
    register_with_registry(&mut engine, registry, cfg);
    engine
}

#[test]
fn ask_one_liner_returns_canned() {
    let engine = engine_with_mock("hello");
    let out: String = engine.eval(r#"ai::ask("hi")"#).unwrap();
    assert_eq!(out, "hello");
}

#[test]
fn builder_chain_returns_canned() {
    let engine = engine_with_mock("done");
    let script = r#"
        let req = ai::request();
        req.backend("mock");
        req.system("be terse");
        req.context("ctx1");
        req.context("ctx2");
        req.prompt("Q");
        req.send()
    "#;
    let out: String = engine.eval(script).unwrap();
    assert_eq!(out, "done");
}

#[test]
fn send_full_returns_object_with_fields() {
    let engine = engine_with_mock("body");
    let script = r#"
        let req = ai::request();
        req.backend("mock");
        req.prompt("Q");
        let r = req.send_full();
        r["text"] + "|" + r["backend"]
    "#;
    let out: String = engine.eval(script).unwrap();
    assert_eq!(out, "body|mock");
}

#[test]
fn multi_turn_replay_works() {
    let engine = engine_with_mock("a2");
    let script = r#"
        let req = ai::request().backend("mock").prompt("Q1");
        req.send();
        req.assistant("a1");
        req.user("Q2");
        req.send()
    "#;
    let out: String = engine.eval(script).unwrap();
    assert_eq!(out, "a2");
}

#[test]
fn send_without_prompt_errors() {
    let engine = engine_with_mock("ignored");
    let script = r#"ai::request().backend("mock").send()"#;
    let err = engine.eval::<String>(script).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("no user prompt"), "got: {msg}");
}

#[test]
fn unknown_backend_errors() {
    let engine = engine_with_mock("ignored");
    let script = r#"ai::request().backend("nope").prompt("Q").send()"#;
    let err = engine.eval::<String>(script).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("not found"), "got: {msg}");
}

#[test]
fn appending_assistant_when_last_is_assistant_errors() {
    let engine = engine_with_mock("ignored");
    let script = r#"
        let req = ai::request().backend("mock").prompt("Q1");
        req.assistant("a1");
        req.assistant("a2");
        req.send()
    "#;
    let err = engine.eval::<String>(script).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("already assistant"), "got: {msg}");
}
