//! `tcp(url)` / `tcp(url, opts)` script binding.
//!
//! Returns `#{ ok: true, host, port, resolved_ip, local_addr, duration_ms }`
//! on success. Connect failures / timeouts raise Rhai exceptions carrying
//! a `ProtocolExitCode` tag (7 / 28).

use crate::script::convert::{anyhow_to_rhai, opts_get_u64};
use crate::script::defaults::ScriptDefaults;
use crate::tcp_probe;
use rhai::{Engine, EvalAltResult, Map};

pub fn register(engine: &mut Engine, defaults: ScriptDefaults) {
    {
        let d = defaults.clone();
        engine.register_fn("tcp", move |url: &str| -> Result<Map, Box<EvalAltResult>> {
            do_tcp(url, d.connect_timeout)
        });
    }
    {
        let d = defaults.clone();
        engine.register_fn(
            "tcp",
            move |url: &str, opts: Map| -> Result<Map, Box<EvalAltResult>> {
                let timeout = opts_get_u64(&opts, "timeout")
                    .or_else(|| opts_get_u64(&opts, "connect_timeout"))
                    .unwrap_or(d.connect_timeout);
                do_tcp(url, timeout)
            },
        );
    }
}

fn do_tcp(url: &str, timeout_secs: u64) -> Result<Map, Box<EvalAltResult>> {
    let ok = tcp_probe::probe(url, timeout_secs).map_err(anyhow_to_rhai)?;
    let mut m = Map::new();
    m.insert("ok".into(), true.into());
    m.insert("host".into(), ok.host.into());
    m.insert("port".into(), (ok.port as i64).into());
    m.insert("resolved_ip".into(), ok.resolved_ip.to_string().into());
    m.insert("local_addr".into(), ok.local_addr.into());
    m.insert(
        "duration_ms".into(),
        (ok.duration.as_millis() as i64).into(),
    );
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;
    use std::net::TcpListener;
    use std::thread;

    fn engine_with_tcp() -> Engine {
        let args = Args::try_parse_from(["recon", "--script", "/dev/null"]).unwrap();
        let defaults = ScriptDefaults::from_args(&args);
        let mut engine = Engine::new();
        super::super::helpers::register(&mut engine);
        register(&mut engine, defaults);
        engine
    }

    #[test]
    fn tcp_connect_to_local_listener_succeeds() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();
        let accept_thread = thread::spawn(move || {
            // Accept exactly one connection then close.
            let _ = listener.accept();
        });
        let engine = engine_with_tcp();
        let script = format!(r#"tcp("tcp://127.0.0.1:{port}")"#);
        let m: Map = engine.eval(&script).expect("eval");
        assert_eq!(m.get("ok").unwrap().as_bool().unwrap(), true);
        assert_eq!(m.get("port").unwrap().as_int().unwrap(), port as i64);
        accept_thread.join().unwrap();
    }

    #[test]
    fn tcp_to_closed_port_throws() {
        let engine = engine_with_tcp();
        // 127.0.0.1:1 — basically never open.
        let res = engine.eval::<Map>(r#"tcp("tcp://127.0.0.1:1")"#);
        assert!(res.is_err(), "expected throw, got {res:?}");
    }

    #[test]
    fn tcp_accepts_opts_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let accept_thread = thread::spawn(move || {
            let _ = listener.accept();
        });
        let engine = engine_with_tcp();
        let script = format!(r#"tcp("tcp://127.0.0.1:{port}", #{{ timeout: 2 }})"#);
        let m: Map = engine.eval(&script).expect("eval");
        assert_eq!(m.get("ok").unwrap().as_bool().unwrap(), true);
        accept_thread.join().unwrap();
    }
}
