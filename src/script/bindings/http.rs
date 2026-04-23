//! HTTP script bindings — `http(url)`, `http(url, opts)`, `https(...)`,
//! `request(opts)`. Wraps `client::execute` so scripts get the same
//! request semantics as the CLI (cookies, redirects, body handling, …)
//! with an opts-map overlay on top of inherited CLI defaults.
//!
//! Returned map shape:
//!
//! ```text
//! #{
//!   url: "<requested>",
//!   final_url: "<after redirects>",
//!   status: 200,
//!   body: "<utf-8 lossy>",
//!   headers: #{ "content-type": ["application/json"], ... },
//!   http_version: "1.1" | "2" | "3",
//!   duration_ms: 123,
//! }
//! ```
//!
//! Network errors (unreachable host, TLS failure, timeout) raise Rhai
//! exceptions that carry a `ProtocolExitCode` tag so an uncaught error
//! maps to the curl-compatible exit code (7 for connect-refused, 28 for
//! timeout). HTTP-level non-2xx responses are NOT errors — they return a
//! map with the status field set.

use crate::cli::Args;
use crate::client;
use crate::script::convert::{
    anyhow_to_rhai, err, opts_clone_map, opts_get_bool, opts_get_str, opts_get_u64, to_string,
};
use crate::script::defaults::ScriptDefaults;
use rhai::{Dynamic, Engine, EvalAltResult, Map};
use std::time::Instant;

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("http", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_request(url, &d, None)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "http",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                do_request(url, &d, Some(&opts))
            },
        );
    }
    {
        let d = defaults.clone();
        engine.register_fn("https", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_request(url, &d, None)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "https",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                do_request(url, &d, Some(&opts))
            },
        );
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "request",
            move |opts: Map| -> Result<Map, Box<EvalAltResult>> {
                let url = opts_get_str(&opts, "url")
                    .ok_or_else(|| err("request(opts): opts map must contain a 'url' string"))?;
                do_request(&url, &d, Some(&opts))
            },
        );
    }
}

fn do_request(
    url: &str,
    defaults: &ScriptDefaults,
    opts: Option<&Map>,
) -> Result<Map, Box<EvalAltResult>> {
    let args = build_args(url, defaults, opts).map_err(anyhow_to_rhai)?;

    // Unix-domain socket path — route through the UDS client instead of
    // the HTTP stack. Different response shape, same Map keys.
    if args.unix_socket.is_some() {
        let r = crate::unix_socket::execute(&args).map_err(anyhow_to_rhai)?;
        return Ok(uds_to_rhai_map(url, r));
    }

    let t0 = Instant::now();
    let (response, metrics) = client::execute(&args).map_err(anyhow_to_rhai)?;
    let status = response.status().as_u16() as i64;
    let final_url = response.url().to_string();
    let response_headers = response.headers().clone();
    let headers_map = headers_to_rhai_map(&response_headers);
    let http_version = metrics.http_version.clone().unwrap_or_else(|| "?".into());
    let body_bytes = response
        .bytes()
        .map_err(|e| err(format!("http: read body: {e}")))?;
    let duration_ms = t0.elapsed().as_millis() as i64;
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();
    let charset_dyn = response_charset_dynamic(&response_headers, &body_bytes);

    let mut result = Map::new();
    result.insert("url".into(), url.to_string().into());
    result.insert("final_url".into(), final_url.into());
    result.insert("status".into(), status.into());
    result.insert("body".into(), body_str.into());
    result.insert("body_bytes".into(), Dynamic::from(body_bytes.to_vec()));
    result.insert("charset".into(), charset_dyn);
    result.insert("headers".into(), headers_map.into());
    result.insert("http_version".into(), http_version.into());
    result.insert("duration_ms".into(), duration_ms.into());
    Ok(result)
}

/// Resolve the response charset the way the CLI does: Content-Type
/// `charset=` first, then a chardetng sniff. Returns the encoding label
/// as a Rhai String, or `()` when sniffing yielded nothing useful.
pub(crate) fn response_charset_dynamic(
    headers: &reqwest::header::HeaderMap,
    bytes: &[u8],
) -> Dynamic {
    if let Some(ct) = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(c) = crate::text_encoding::parse_content_type_charset(ct) {
            return c.into();
        }
    }
    let d = crate::text_encoding::detect(bytes);
    if d.had_bom || !bytes.is_empty() {
        d.charset.to_string().into()
    } else {
        Dynamic::UNIT
    }
}

pub(crate) fn build_args(
    url: &str,
    defaults: &ScriptDefaults,
    opts: Option<&Map>,
) -> anyhow::Result<Args> {
    use clap::Parser;
    let mut args = Args::try_parse_from(["recon", url])?;
    args.header = defaults.headers.clone();
    args.insecure = defaults.insecure;
    args.timeout = defaults.connect_timeout;
    args.max_time = defaults.max_time;
    args.follow_redirects = defaults.follow_redirects;
    args.max_redirs = defaults.max_redirs;
    args.user_agent = defaults.user_agent.clone();
    args.referer = defaults.referer.clone();
    args.user = defaults.user.clone();
    args.method = defaults.method.clone();
    args.tlsv12 = defaults.tlsv12;
    args.tlsv13 = defaults.tlsv13;
    args.cacert = defaults.cacert.clone();
    args.interface = defaults.interface.clone();
    args.limit_rate = defaults.limit_rate.clone();
    args.speed_limit = defaults.speed_limit;
    args.speed_time = defaults.speed_time;
    args.dns_servers = defaults.dns_servers.clone();
    args.dns_ipv4_addr = defaults.dns_ipv4_addr.clone();
    args.dns_ipv6_addr = defaults.dns_ipv6_addr.clone();
    args.dns_interface = defaults.dns_interface.clone();

    if let Some(o) = opts {
        if let Some(m) = opts_get_str(o, "method") {
            args.method = Some(m);
        }
        if let Some(headers_map) = opts_clone_map(o, "headers") {
            let mut new_headers = args.header.clone();
            for (k, v) in headers_map.iter() {
                new_headers.push(format!("{}: {}", k, to_string(v)));
            }
            args.header = new_headers;
        }
        if let Some(body) = opts_get_str(o, "body") {
            args.data = Some(body);
        }
        if let Some(ms) = opts_get_u64(o, "timeout_ms") {
            args.max_time = Some((ms as f64) / 1000.0);
        }
        if let Some(s) = opts_get_u64(o, "connect_timeout") {
            args.timeout = s;
        }
        if let Some(ins) = opts_get_bool(o, "insecure") {
            args.insecure = ins;
        }
        if let Some(fr) = opts_get_bool(o, "follow_redirects") {
            args.follow_redirects = fr;
        }
        if let Some(v) = opts_get_bool(o, "tlsv12") {
            args.tlsv12 = v;
        }
        if let Some(v) = opts_get_bool(o, "tlsv13") {
            args.tlsv13 = v;
        }
        if let Some(p) = opts_get_str(o, "cacert") {
            args.cacert = Some(std::path::PathBuf::from(p));
        }
        if let Some(ip) = opts_get_str(o, "interface") {
            args.interface = Some(ip);
        }
        if let Some(s) = opts_get_str(o, "limit_rate") {
            args.limit_rate = Some(s);
        }
        if let Some(n) = opts_get_u64(o, "speed_limit") {
            args.speed_limit = Some(n);
        }
        if let Some(n) = opts_get_u64(o, "speed_time") {
            args.speed_time = n;
        }
        if let Some(s) = opts_get_str(o, "dns_servers") {
            args.dns_servers = Some(s);
        }
        if let Some(s) = opts_get_str(o, "dns_ipv4_addr") {
            args.dns_ipv4_addr = Some(s);
        }
        if let Some(s) = opts_get_str(o, "dns_ipv6_addr") {
            args.dns_ipv6_addr = Some(s);
        }
        if let Some(s) = opts_get_str(o, "dns_interface") {
            args.dns_interface = Some(s);
        }
        if let Some(s) = opts_get_str(o, "proxy") {
            args.proxy = Some(s);
        }
        if let Some(s) = opts_get_str(o, "proxy_user") {
            args.proxy_user = Some(s);
        }
        if let Some(s) = opts_get_str(o, "noproxy") {
            args.noproxy = Some(s);
        }
        if let Some(b) = opts_get_bool(o, "proxy_insecure") {
            args.proxy_insecure = b;
        }
        if let Some(s) = opts_get_str(o, "proxy_cacert") {
            args.proxy_cacert = Some(std::path::PathBuf::from(s));
        }
        if let Some(s) = opts_get_str(o, "unix_socket") {
            args.unix_socket = Some(std::path::PathBuf::from(s));
        }
    }
    Ok(args)
}

fn uds_to_rhai_map(url: &str, r: crate::unix_socket::UdsResponse) -> Map {
    let mut headers = Map::new();
    for (k, v) in &r.headers {
        let entry = headers
            .entry(k.as_str().into())
            .or_insert_with(|| rhai::Array::new().into());
        if let Some(arr) = entry.write_lock::<rhai::Array>() {
            let mut a = arr;
            a.push(v.clone().into());
        }
    }
    let body_str = String::from_utf8_lossy(&r.body).into_owned();
    let mut out = Map::new();
    out.insert("url".into(), url.to_string().into());
    out.insert("final_url".into(), r.final_url.into());
    out.insert("status".into(), (r.status as i64).into());
    out.insert("body".into(), body_str.into());
    out.insert("body_bytes".into(), Dynamic::from(r.body));
    out.insert("charset".into(), Dynamic::UNIT);
    out.insert("headers".into(), headers.into());
    out.insert("http_version".into(), r.http_version.into());
    out.insert("duration_ms".into(), (r.duration_ms as i64).into());
    out
}

pub(crate) fn headers_to_rhai_map(headers: &reqwest::header::HeaderMap) -> Map {
    let mut m = Map::new();
    for name in headers.keys() {
        let vals: Vec<Dynamic> = headers
            .get_all(name)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string().into()))
            .collect();
        m.insert(name.as_str().into(), vals.into());
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn defaults_from_args() -> ScriptDefaults {
        let args = Args::try_parse_from(["recon", "--script", "/dev/null"]).unwrap();
        ScriptDefaults::from_args(&args)
    }

    fn engine_with_http(defaults: ScriptDefaults) -> Engine {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e, defaults);
        e
    }

    /// Evaluate a script that returns a map and extract `(status, body)`
    /// inside the blocking thread — returning the Rhai `Map` across the
    /// spawn_blocking boundary would require `Send`, but Rhai uses `Rc`.
    async fn eval_status_body(script: String) -> Result<(i64, String), String> {
        tokio::task::spawn_blocking(move || {
            let defaults = defaults_from_args();
            let engine = engine_with_http(defaults);
            match engine.eval::<Map>(&script) {
                Ok(m) => {
                    let status = m.get("status").and_then(|v| v.as_int().ok()).unwrap_or(-1);
                    let body = m
                        .get("body")
                        .map(|v| v.clone().into_string().unwrap_or_default())
                        .unwrap_or_default();
                    Ok((status, body))
                }
                Err(e) => Err(e.to_string()),
            }
        })
        .await
        .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_get_returns_status_and_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/hello"))
            .respond_with(ResponseTemplate::new(200).set_body_string("hi there"))
            .mount(&server)
            .await;

        let url = format!("{}/hello", server.uri());
        let (status, body) = eval_status_body(format!(r#"http("{url}")"#))
            .await
            .expect("eval");
        assert_eq!(status, 200);
        assert_eq!(body, "hi there");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_post_with_body_and_custom_header() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/echo"))
            .and(header("X-Custom", "yes"))
            .respond_with(ResponseTemplate::new(201).set_body_string("created"))
            .mount(&server)
            .await;

        let url = format!("{}/echo", server.uri());
        let (status, _) = eval_status_body(format!(
            r#"http("{url}", #{{ method: "POST", headers: #{{ "X-Custom": "yes" }}, body: "hello" }})"#
        ))
        .await
        .expect("eval");
        assert_eq!(status, 201);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn http_5xx_is_result_not_exception() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/err"))
            .respond_with(ResponseTemplate::new(503).set_body_string("down"))
            .mount(&server)
            .await;

        let url = format!("{}/err", server.uri());
        let (status, _) = eval_status_body(format!(r#"http("{url}")"#))
            .await
            .expect("eval");
        assert_eq!(status, 503);
    }

    #[test]
    fn unreachable_host_throws() {
        let defaults = defaults_from_args();
        let engine = engine_with_http(defaults);
        let script = r#"http("http://127.0.0.1:1/")"#;
        let res = engine.eval::<Map>(script);
        assert!(res.is_err(), "expected throw, got {res:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn request_opts_map_with_url_field() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/x"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let url = format!("{}/x", server.uri());
        let (status, _) =
            eval_status_body(format!(r#"request(#{{ url: "{url}", method: "PUT" }})"#))
                .await
                .expect("eval");
        assert_eq!(status, 204);
    }
}
