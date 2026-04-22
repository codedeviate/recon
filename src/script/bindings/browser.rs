//! `browser()` script binding — a stateful HTTP session handle.
//!
//! Unlike the one-shot `http(url, opts)` binding, a `browser()` handle
//! retains a cookie jar, default headers, user-agent, TLS flags, and
//! redirect policy across multiple requests. Each browser owns its own
//! jar (ephemeral temp file by default, or a named persistent jar via
//! `use_persistent_session("name")`). Multiple browsers in the same
//! script are independent — this is the "parallel browsers" use case.
//!
//! Script-only. No CLI flag maps to this.
//!
//! ```text
//! let b = browser();
//! b.set_user_agent("MyBot/1.0");
//! b.set_header("X-API-Key", "abc");
//! b.get("https://example.com/login");    // collects Set-Cookie
//! b.get("https://example.com/profile");  // sends Cookie: ...
//! b.use_persistent_session("myjar");     // swap to ~/.recon/jars/myjar.db
//! ```
//!
//! Response shape is identical to `http()`: `#{url, final_url, status,
//! body, headers, http_version, duration_ms}`.

use crate::client;
use crate::script::bindings::http;
use crate::script::convert::{anyhow_to_rhai, err, opts_get_str, to_string};
use crate::script::defaults::ScriptDefaults;
use rhai::{Array, Blob, Dynamic, Engine, EvalAltResult, Map};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;
use tempfile::NamedTempFile;

/// Handle returned by `browser()`. Cheap to clone — interior state is
/// shared via `Rc<RefCell<_>>`, matching the `SqliteHandle` pattern.
#[derive(Clone)]
pub struct BrowserHandle {
    state: Rc<RefCell<BrowserState>>,
}

struct BrowserState {
    defaults: ScriptDefaults,

    user_agent: Option<String>,
    extra_headers: Vec<(String, String)>,
    insecure: Option<bool>,
    follow_redirects: Option<bool>,
    max_redirects: Option<usize>,
    max_time: Option<f64>,
    connect_timeout: Option<u64>,
    basic_auth: Option<String>,

    jar: JarLocation,
}

enum JarLocation {
    Temp(NamedTempFile),
    Named(String),
}

impl JarLocation {
    fn path(&self) -> PathBuf {
        match self {
            JarLocation::Temp(f) => f.path().to_path_buf(),
            JarLocation::Named(name) => {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                PathBuf::from(home).join(".recon").join("jars").join(format!("{name}.db"))
            }
        }
    }

    fn name(&self) -> Option<String> {
        match self {
            JarLocation::Named(n) => Some(n.clone()),
            JarLocation::Temp(_) => None,
        }
    }
}

fn new_temp_jar() -> Result<NamedTempFile, Box<EvalAltResult>> {
    tempfile::Builder::new()
        .prefix("recon-browser-")
        .suffix(".db")
        .tempfile()
        .map_err(|e| err(format!("browser: create temp jar: {e}")))
}

impl BrowserHandle {
    fn new(defaults: ScriptDefaults) -> Result<Self, Box<EvalAltResult>> {
        let tmp = new_temp_jar()?;
        Ok(BrowserHandle {
            state: Rc::new(RefCell::new(BrowserState {
                defaults,
                user_agent: None,
                extra_headers: Vec::new(),
                insecure: None,
                follow_redirects: None,
                max_redirects: None,
                max_time: None,
                connect_timeout: None,
                basic_auth: None,
                jar: JarLocation::Temp(tmp),
            })),
        })
    }

    /// Apply an initial-config map (passed to `browser(#{...})`).
    fn configure(self, init: &Map) -> Result<Self, Box<EvalAltResult>> {
        {
            let mut s = self.state.borrow_mut();
            if let Some(ua) = opts_get_str(init, "user_agent") {
                s.user_agent = Some(ua);
            }
            if let Some(headers) = init.get("headers").cloned() {
                if let Some(map) = headers.try_cast::<Map>() {
                    for (k, v) in map.iter() {
                        s.extra_headers.push((k.to_string(), to_string(v)));
                    }
                }
            }
            if let Some(v) = init.get("insecure").and_then(|v| v.as_bool().ok()) {
                s.insecure = Some(v);
            }
            if let Some(v) = init.get("follow_redirects").and_then(|v| v.as_bool().ok()) {
                s.follow_redirects = Some(v);
            }
            if let Some(n) = init.get("max_redirects").and_then(|v| v.as_int().ok()) {
                if n >= 0 {
                    s.max_redirects = Some(n as usize);
                }
            }
            if let Some(ms) = init.get("timeout_ms").and_then(|v| v.as_int().ok()) {
                if ms >= 0 {
                    s.max_time = Some((ms as f64) / 1000.0);
                }
            }
            if let Some(secs) = init.get("connect_timeout").and_then(|v| v.as_int().ok()) {
                if secs >= 0 {
                    s.connect_timeout = Some(secs as u64);
                }
            }
            if let Some(ba) = opts_get_str(init, "basic_auth") {
                s.basic_auth = Some(ba);
            }
        }
        Ok(self)
    }
}

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    engine.register_type_with_name::<BrowserHandle>("Browser");

    // ── Constructors ──────────────────────────────────────────────────────
    {
        let d = defaults.clone();
        engine.register_fn("browser", move || -> Result<BrowserHandle, Box<EvalAltResult>> {
            BrowserHandle::new(d.clone())
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "browser",
            move |init: Map| -> Result<BrowserHandle, Box<EvalAltResult>> {
                BrowserHandle::new(d.clone())?.configure(&init)
            },
        );
    }

    // ── Configuration setters ─────────────────────────────────────────────
    engine.register_fn("set_user_agent", |h: &mut BrowserHandle, ua: &str| {
        h.state.borrow_mut().user_agent = Some(ua.to_string());
    });
    engine.register_fn("set_header", |h: &mut BrowserHandle, name: &str, value: &str| {
        let mut s = h.state.borrow_mut();
        // Case-insensitive replace; single-value semantics.
        s.extra_headers.retain(|(k, _)| !k.eq_ignore_ascii_case(name));
        s.extra_headers.push((name.to_string(), value.to_string()));
    });
    engine.register_fn("set_headers", |h: &mut BrowserHandle, headers: Map| {
        let mut s = h.state.borrow_mut();
        for (k, v) in headers.iter() {
            let name = k.to_string();
            s.extra_headers.retain(|(kk, _)| !kk.eq_ignore_ascii_case(&name));
            s.extra_headers.push((name, to_string(v)));
        }
    });
    engine.register_fn("remove_header", |h: &mut BrowserHandle, name: &str| {
        h.state
            .borrow_mut()
            .extra_headers
            .retain(|(k, _)| !k.eq_ignore_ascii_case(name));
    });
    engine.register_fn("clear_headers", |h: &mut BrowserHandle| {
        h.state.borrow_mut().extra_headers.clear();
    });
    engine.register_fn("set_timeout_ms", |h: &mut BrowserHandle, ms: i64| {
        if ms >= 0 {
            h.state.borrow_mut().max_time = Some((ms as f64) / 1000.0);
        }
    });
    engine.register_fn("set_connect_timeout", |h: &mut BrowserHandle, secs: i64| {
        if secs >= 0 {
            h.state.borrow_mut().connect_timeout = Some(secs as u64);
        }
    });
    engine.register_fn("set_insecure", |h: &mut BrowserHandle, v: bool| {
        h.state.borrow_mut().insecure = Some(v);
    });
    engine.register_fn("follow_redirects", |h: &mut BrowserHandle, v: bool| {
        h.state.borrow_mut().follow_redirects = Some(v);
    });
    engine.register_fn("set_max_redirects", |h: &mut BrowserHandle, n: i64| {
        if n >= 0 {
            h.state.borrow_mut().max_redirects = Some(n as usize);
        }
    });
    engine.register_fn("set_basic_auth", |h: &mut BrowserHandle, user: &str, pass: &str| {
        h.state.borrow_mut().basic_auth = Some(format!("{user}:{pass}"));
    });

    // ── Session control ───────────────────────────────────────────────────
    engine.register_fn(
        "use_persistent_session",
        |h: &mut BrowserHandle, name: &str| -> Result<(), Box<EvalAltResult>> {
            if name.is_empty() {
                return Err(err("browser: use_persistent_session: name must not be empty"));
            }
            let mut s = h.state.borrow_mut();
            s.jar = JarLocation::Named(name.to_string());
            Ok(())
        },
    );
    engine.register_fn(
        "use_ephemeral_session",
        |h: &mut BrowserHandle| -> Result<(), Box<EvalAltResult>> {
            let tmp = new_temp_jar()?;
            h.state.borrow_mut().jar = JarLocation::Temp(tmp);
            Ok(())
        },
    );
    engine.register_fn(
        "session_name",
        |h: &mut BrowserHandle| -> Dynamic {
            match h.state.borrow().jar.name() {
                Some(n) => n.into(),
                None => Dynamic::UNIT,
            }
        },
    );
    engine.register_fn(
        "clear_cookies",
        |h: &mut BrowserHandle| -> Result<(), Box<EvalAltResult>> {
            let path = h.state.borrow().jar.path();
            if !path.exists() || std::fs::metadata(&path).map(|m| m.len() == 0).unwrap_or(true) {
                return Ok(());
            }
            let conn = rusqlite::Connection::open(&path)
                .map_err(|e| err(format!("browser: open jar '{}': {e}", path.display())))?;
            let _ = conn.execute("DELETE FROM cookies", []);
            Ok(())
        },
    );
    engine.register_fn(
        "cookies",
        |h: &mut BrowserHandle| -> Result<Array, Box<EvalAltResult>> {
            let path = h.state.borrow().jar.path();
            if !path.exists() || std::fs::metadata(&path).map(|m| m.len() == 0).unwrap_or(true) {
                return Ok(Array::new());
            }
            let conn = rusqlite::Connection::open(&path)
                .map_err(|e| err(format!("browser: open jar '{}': {e}", path.display())))?;
            // If the schema hasn't been created yet (no requests made), no cookies.
            let table_exists: bool = conn
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='cookies'",
                    [],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            if !table_exists {
                return Ok(Array::new());
            }
            let mut stmt = conn
                .prepare(
                    "SELECT domain, path, name, value, expires, secure, http_only FROM cookies",
                )
                .map_err(|e| err(format!("browser: prepare cookies: {e}")))?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, Option<i64>>(4)?,
                        r.get::<_, i64>(5)? != 0,
                        r.get::<_, i64>(6)? != 0,
                    ))
                })
                .map_err(|e| err(format!("browser: query cookies: {e}")))?;
            let mut out = Array::new();
            for row in rows {
                let (domain, path, name, value, expires, secure, http_only) =
                    row.map_err(|e| err(format!("browser: read cookie row: {e}")))?;
                let mut m = Map::new();
                m.insert("domain".into(), domain.into());
                m.insert("path".into(), path.into());
                m.insert("name".into(), name.into());
                m.insert("value".into(), value.into());
                m.insert(
                    "expires".into(),
                    match expires {
                        Some(n) => Dynamic::from(n),
                        None => Dynamic::UNIT,
                    },
                );
                m.insert("secure".into(), secure.into());
                m.insert("http_only".into(), http_only.into());
                out.push(m.into());
            }
            Ok(out)
        },
    );

    // ── Request methods ───────────────────────────────────────────────────
    // GET / HEAD / OPTIONS / DELETE: no body argument.
    for method in ["get", "head", "options", "delete"] {
        let m = method.to_string();
        engine.register_fn(
            method,
            move |h: &mut BrowserHandle, url: &str| -> Result<Map, Box<EvalAltResult>> {
                do_request(h, &m, url, None, None)
            },
        );
        let m = method.to_string();
        engine.register_fn(
            method,
            move |h: &mut BrowserHandle, url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                do_request(h, &m, url, None, Some(&opts))
            },
        );
    }

    // POST / PUT / PATCH: body argument (String, Blob, or Map).
    for method in ["post", "put", "patch"] {
        let m = method.to_string();
        engine.register_fn(
            method,
            move |h: &mut BrowserHandle, url: &str, body: Dynamic| -> Result<Map, Box<EvalAltResult>> {
                let b = coerce_body(body)?;
                do_request(h, &m, url, Some(b), None)
            },
        );
        let m = method.to_string();
        engine.register_fn(
            method,
            move |h: &mut BrowserHandle, url: &str, body: Dynamic, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                let b = coerce_body(body)?;
                do_request(h, &m, url, Some(b), Some(&opts))
            },
        );
    }

    // request(opts) — opts must contain `url`; `method` / `body` optional.
    engine.register_fn(
        "request",
        |h: &mut BrowserHandle, opts: Map| -> Result<Map, Box<EvalAltResult>> {
            let url = opts_get_str(&opts, "url")
                .ok_or_else(|| err("browser.request(opts): opts map must contain a 'url' string"))?;
            let method = opts_get_str(&opts, "method").unwrap_or_else(|| "GET".into());
            let body = match opts.get("body").cloned() {
                Some(v) if !v.is_unit() => Some(coerce_body(v)?),
                _ => None,
            };
            do_request(h, &method, &url, body, Some(&opts))
        },
    );
}

// ── Body coercion ─────────────────────────────────────────────────────────

enum RequestBody {
    Raw(String),
    Json(String),
}

fn coerce_body(v: Dynamic) -> Result<RequestBody, Box<EvalAltResult>> {
    if v.is_unit() {
        return Ok(RequestBody::Raw(String::new()));
    }
    if v.is_string() {
        return Ok(RequestBody::Raw(v.into_string().unwrap_or_default()));
    }
    if v.is_blob() {
        let b: Blob = v
            .into_blob()
            .map_err(|_| err("browser: body blob cast failed"))?;
        return Ok(RequestBody::Raw(String::from_utf8_lossy(&b).into_owned()));
    }
    if v.is_map() {
        let jv = crate::script::bindings::helpers::dynamic_to_json(&Dynamic::from(v))?;
        let json = serde_json::to_string(&jv)
            .map_err(|e| err(format!("browser: body map→json: {e}")))?;
        return Ok(RequestBody::Json(json));
    }
    if v.is_array() {
        let jv = crate::script::bindings::helpers::dynamic_to_json(&v)?;
        let json = serde_json::to_string(&jv)
            .map_err(|e| err(format!("browser: body array→json: {e}")))?;
        return Ok(RequestBody::Json(json));
    }
    Err(err("browser: body must be String, Blob, Map, or Array"))
}

// ── Request dispatch ──────────────────────────────────────────────────────

fn do_request(
    h: &BrowserHandle,
    method: &str,
    url: &str,
    body: Option<RequestBody>,
    opts: Option<&Map>,
) -> Result<Map, Box<EvalAltResult>> {
    let (mut args, jar_path) = {
        let state = h.state.borrow();
        let mut args = http::build_args(url, &state.defaults, opts).map_err(anyhow_to_rhai)?;

        // Overlay browser-level config above CLI defaults, below per-call opts
        // (per-call opts were already applied by build_args, so only fields we
        // touch here can be re-clobbered by opts; we handle that explicitly).
        if !has_opts_field(opts, "method") {
            args.method = Some(method.to_string());
        }
        if state.user_agent.is_some() && args.user_agent == state.defaults.user_agent {
            args.user_agent = state.user_agent.clone();
        }
        // Append session headers below any opts-provided headers — build_args
        // already appended opts headers on top of defaults. We insert ours
        // first (if not already present case-insensitively) so opts still wins.
        for (k, v) in &state.extra_headers {
            if !has_header_ci(&args.header, k) {
                args.header.push(format!("{k}: {v}"));
            }
        }
        if state.insecure.is_some() && !opts_has_bool(opts, "insecure") {
            args.insecure = state.insecure.unwrap();
        }
        if state.follow_redirects.is_some() && !opts_has_bool(opts, "follow_redirects") {
            args.follow_redirects = state.follow_redirects.unwrap();
        }
        if let Some(n) = state.max_redirects {
            args.max_redirs = n;
        }
        if state.max_time.is_some() && !has_opts_field(opts, "timeout_ms") {
            args.max_time = state.max_time;
        }
        if let Some(t) = state.connect_timeout {
            if !has_opts_field(opts, "connect_timeout") {
                args.timeout = t;
            }
        }
        if state.basic_auth.is_some() && args.user.is_none() {
            args.user = state.basic_auth.clone();
        }

        let jar_path = state.jar.path();
        args.cookiejar = Some(jar_path.to_string_lossy().into_owned());
        (args, jar_path)
    };

    // Body handling: only apply body if per-call opts didn't set one.
    if !has_opts_field(opts, "body") {
        match body {
            Some(RequestBody::Raw(s)) => {
                if !s.is_empty() {
                    args.data = Some(s);
                }
            }
            Some(RequestBody::Json(s)) => {
                args.data = Some(s);
                if !has_header_ci(&args.header, "content-type") {
                    args.header.push("content-type: application/json".into());
                }
            }
            None => {}
        }
    }

    // Ensure the jar file's parent directory exists for Named sessions.
    if let Some(parent) = jar_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let t0 = Instant::now();
    let (response, metrics) = client::execute(&args).map_err(anyhow_to_rhai)?;
    let status = response.status().as_u16() as i64;
    let final_url = response.url().to_string();
    let headers_map = http::headers_to_rhai_map(response.headers());
    let http_version = metrics.http_version.clone().unwrap_or_else(|| "?".into());
    let body_bytes = response
        .bytes()
        .map_err(|e| err(format!("browser: read body: {e}")))?;
    let duration_ms = t0.elapsed().as_millis() as i64;
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

    let mut result = Map::new();
    result.insert("url".into(), url.to_string().into());
    result.insert("final_url".into(), final_url.into());
    result.insert("status".into(), status.into());
    result.insert("body".into(), body_str.into());
    result.insert("headers".into(), headers_map.into());
    result.insert("http_version".into(), http_version.into());
    result.insert("duration_ms".into(), duration_ms.into());
    Ok(result)
}

fn has_header_ci(headers: &[String], name: &str) -> bool {
    headers.iter().any(|h| {
        h.split_once(':')
            .map(|(k, _)| k.trim().eq_ignore_ascii_case(name))
            .unwrap_or(false)
    })
}

fn has_opts_field(opts: Option<&Map>, key: &str) -> bool {
    opts.map(|o| o.contains_key(key)).unwrap_or(false)
}

fn opts_has_bool(opts: Option<&Map>, key: &str) -> bool {
    opts.and_then(|o| o.get(key)).is_some_and(|v| v.is::<bool>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;
    use wiremock::matchers::{header, header_exists, method, path as wm_path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn defaults() -> ScriptDefaults {
        let args = Args::try_parse_from(["recon", "--script", "/dev/null"]).unwrap();
        ScriptDefaults::from_args(&args)
    }

    fn engine_with_browser() -> Engine {
        let mut e = Engine::new();
        crate::script::bindings::helpers::register(&mut e);
        register(&mut e, defaults());
        e
    }

    async fn eval_i64(script: String) -> Result<i64, String> {
        tokio::task::spawn_blocking(move || {
            let engine = engine_with_browser();
            engine.eval::<i64>(&script).map_err(|e| e.to_string())
        })
        .await
        .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sticky_cookies_across_calls() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(wm_path("/set"))
            .respond_with(
                ResponseTemplate::new(200).insert_header("set-cookie", "session=abc; Path=/"),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(wm_path("/check"))
            .and(header("cookie", "session=abc"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;

        let url = server.uri();
        let script = format!(
            r#"
let b = browser();
b.get("{url}/set");
let r = b.get("{url}/check");
r.status
"#
        );
        let status = eval_i64(script).await.expect("eval");
        assert_eq!(status, 200);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn per_browser_isolation() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(wm_path("/set"))
            .respond_with(
                ResponseTemplate::new(200).insert_header("set-cookie", "tok=x; Path=/"),
            )
            .mount(&server)
            .await;
        // Route A: requires cookie. Route B: must NOT have cookie.
        Mock::given(method("GET"))
            .and(wm_path("/has"))
            .and(header_exists("cookie"))
            .respond_with(ResponseTemplate::new(200).set_body_string("has"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(wm_path("/nocookie"))
            .respond_with(ResponseTemplate::new(418).set_body_string("no"))
            .mount(&server)
            .await;

        let url = server.uri();
        // b1 gets a cookie; b2 should not have one.
        let script = format!(
            r#"
let b1 = browser();
let b2 = browser();
b1.get("{url}/set");
let r1 = b1.get("{url}/has");
let r2 = b2.get("{url}/nocookie");
r1.status * 1000 + r2.status
"#
        );
        let combined = eval_i64(script).await.expect("eval");
        assert_eq!(combined, 200 * 1000 + 418);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn header_config_persists() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(wm_path("/a"))
            .and(header("x-api-key", "abc"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(wm_path("/b"))
            .and(header("x-api-key", "abc"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let url = server.uri();
        let script = format!(
            r#"
let b = browser();
b.set_header("X-API-Key", "abc");
let a = b.get("{url}/a");
let bb = b.get("{url}/b");
a.status + bb.status
"#
        );
        let sum = eval_i64(script).await.expect("eval");
        assert_eq!(sum, 400);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn map_body_autoserialises_to_json() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(wm_path("/j"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(201).set_body_string("ok"))
            .mount(&server)
            .await;

        let url = server.uri();
        let script = format!(
            r#"
let b = browser();
let r = b.post("{url}/j", #{{ a: 1, b: [2, 3] }});
r.status
"#
        );
        let status = eval_i64(script).await.expect("eval");
        assert_eq!(status, 201);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn string_body_no_auto_content_type() {
        let server = MockServer::start().await;
        // Accept the request only if no content-type is application/json.
        Mock::given(method("POST"))
            .and(wm_path("/raw"))
            .respond_with(|req: &wiremock::Request| {
                let ct = req
                    .headers
                    .get("content-type")
                    .map(|v| v.to_str().unwrap_or(""))
                    .unwrap_or("");
                if ct.contains("application/json") {
                    ResponseTemplate::new(415)
                } else {
                    ResponseTemplate::new(200).set_body_string("ok")
                }
            })
            .mount(&server)
            .await;

        let url = server.uri();
        let script = format!(
            r#"
let b = browser();
let r = b.post("{url}/raw", "hello");
r.status
"#
        );
        let status = eval_i64(script).await.expect("eval");
        assert_eq!(status, 200);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn opts_user_agent_overrides_browser() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(wm_path("/ua"))
            .and(header("user-agent", "Bar/2.0"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let url = server.uri();
        let script = format!(
            r#"
let b = browser();
b.set_user_agent("Foo/1.0");
let r = b.get("{url}/ua", #{{ headers: #{{ "user-agent": "Bar/2.0" }} }});
r.status
"#
        );
        let status = eval_i64(script).await.expect("eval");
        assert_eq!(status, 200);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cookies_listing_after_set() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(wm_path("/set"))
            .respond_with(
                ResponseTemplate::new(200).insert_header("set-cookie", "it=works; Path=/"),
            )
            .mount(&server)
            .await;

        let url = server.uri();
        let script = format!(
            r#"
let b = browser();
b.get("{url}/set");
let c = b.cookies();
if c.len() >= 1 && c[0].name == "it" && c[0].value == "works" {{ 1 }} else {{ 0 }}
"#
        );
        let ok = eval_i64(script).await.expect("eval");
        assert_eq!(ok, 1, "cookies() should list the set cookie");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn clear_cookies_wipes_jar() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(wm_path("/set"))
            .respond_with(
                ResponseTemplate::new(200).insert_header("set-cookie", "s=1; Path=/"),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(wm_path("/nocookie"))
            .respond_with(|req: &wiremock::Request| {
                if req.headers.get("cookie").is_some() {
                    ResponseTemplate::new(418)
                } else {
                    ResponseTemplate::new(200)
                }
            })
            .mount(&server)
            .await;

        let url = server.uri();
        let script = format!(
            r#"
let b = browser();
b.get("{url}/set");
b.clear_cookies();
let r = b.get("{url}/nocookie");
r.status
"#
        );
        let status = eval_i64(script).await.expect("eval");
        assert_eq!(status, 200);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn persistent_session_fresh_swap_discards_ephemeral_cookies() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(wm_path("/set"))
            .respond_with(
                ResponseTemplate::new(200).insert_header("set-cookie", "eph=1; Path=/"),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(wm_path("/nocookie"))
            .respond_with(|req: &wiremock::Request| {
                if req.headers.get("cookie").is_some() {
                    ResponseTemplate::new(418)
                } else {
                    ResponseTemplate::new(200)
                }
            })
            .mount(&server)
            .await;

        // Use a unique jar name under /tmp via HOME override so the test
        // doesn't pollute the user's ~/.recon.
        let tmp_home = tempfile::tempdir().unwrap();
        let prev_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", tmp_home.path());

        let url = server.uri();
        let script = format!(
            r#"
let b = browser();
b.get("{url}/set");
b.use_persistent_session("swap-test-jar");
let r = b.get("{url}/nocookie");
r.status
"#
        );
        let status = eval_i64(script).await.expect("eval");

        // Restore HOME before asserting so failures don't leak a bad env.
        match prev_home {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }

        assert_eq!(status, 200);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn session_name_reports_ephemeral_and_named() {
        let script = r#"
let b = browser();
let ephem = b.session_name();
b.use_persistent_session("mine");
let named = b.session_name();
#{ is_unit: ephem == (), named: named }
"#;
        let engine = engine_with_browser();
        let m: Map = engine.eval(script).expect("eval");
        assert_eq!(m.get("is_unit").and_then(|v| v.as_bool().ok()), Some(true));
        assert_eq!(
            m.get("named").map(|v| v.clone().into_string().unwrap_or_default()),
            Some("mine".to_string())
        );
    }
}
