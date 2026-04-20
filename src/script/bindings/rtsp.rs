//! `rtsp(url)` / `rtsps(url)` script binding. Sends `OPTIONS *` and parses
//! the response. Returns `#{ host, port, tls, connect_ms, status_line,
//! status_code, headers: #{...}, methods: [...] }` — `methods` pre-parses
//! the `Public:` header's comma-separated method list.

use crate::rtsp_probe;
use crate::script::convert::{anyhow_to_rhai, opts_get_bool, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    for scheme in ["rtsp", "rtsps"] {
        let d = defaults.clone();
        engine.register_fn(
            scheme,
            move |url: &str| -> Result<Map, Box<EvalAltResult>> {
                do_rtsp(url, d.insecure, d.connect_timeout)
            },
        );
        let d = defaults.clone();
        engine.register_fn(
            scheme,
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                let insecure = opts_get_bool(&opts, "insecure").unwrap_or(d.insecure);
                let timeout = opts_get_u64(&opts, "timeout").unwrap_or(d.connect_timeout);
                do_rtsp(url, insecure, timeout)
            },
        );
    }
}

fn do_rtsp(url: &str, insecure: bool, timeout_secs: u64) -> Result<Map, Box<EvalAltResult>> {
    let r = rtsp_probe::probe(url, insecure, timeout_secs).map_err(anyhow_to_rhai)?;

    let status_code = parse_status_code(&r.status_line);
    let mut headers_map = Map::new();
    let mut methods: Option<Vec<String>> = None;
    for (k, v) in &r.headers {
        if k.eq_ignore_ascii_case("Public") {
            methods = Some(
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
            );
        }
        headers_map.insert(k.as_str().into(), v.clone().into());
    }

    let mut m = Map::new();
    m.insert("host".into(), r.host.into());
    m.insert("port".into(), (r.port as i64).into());
    m.insert("tls".into(), r.tls.into());
    m.insert("connect_ms".into(), r.connect_ms.into());
    m.insert("status_line".into(), r.status_line.trim_end_matches(['\r', '\n']).to_string().into());
    if let Some(code) = status_code {
        m.insert("status_code".into(), (code as i64).into());
    }
    m.insert("headers".into(), headers_map.into());
    if let Some(ms) = methods {
        let arr: Array = ms.into_iter().map(Dynamic::from).collect();
        m.insert("methods".into(), arr.into());
    }
    Ok(m)
}

fn parse_status_code(status_line: &str) -> Option<u16> {
    // "RTSP/1.0 200 OK\r\n"
    let mut parts = status_line.split_whitespace();
    let _version = parts.next()?;
    let code_str = parts.next()?;
    code_str.parse().ok()
}
