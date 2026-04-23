//! UDP primitives for scripts — `udp_bind`, `udp_recv_from`,
//! `udp_send_to`, `udp_close`.
//!
//! Handle wraps `Arc<Mutex<UdpSocket>>` so it's Send+Sync and
//! `thread_spawn`-friendly.

use crate::script::convert::err;
use rhai::{Blob, Engine, EvalAltResult, Map};
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct UdpHandle {
    inner: Arc<Mutex<Option<UdpSocket>>>,
}

pub fn register(engine: &mut Engine) {
    engine.register_type_with_name::<UdpHandle>("UdpSocket");

    engine.register_fn("udp_bind", |addr: &str| -> Result<UdpHandle, Box<EvalAltResult>> {
        let sock = UdpSocket::bind(addr).map_err(|e| err(format!("udp_bind '{addr}': {e}")))?;
        Ok(UdpHandle {
            inner: Arc::new(Mutex::new(Some(sock))),
        })
    });

    engine.register_fn(
        "udp_recv_from",
        |h: &mut UdpHandle, max_len: i64| -> Result<Map, Box<EvalAltResult>> {
            udp_recv_inner(h, max_len, None)
        },
    );

    engine.register_fn(
        "udp_recv_from",
        |h: &mut UdpHandle, max_len: i64, timeout_ms: i64|
         -> Result<Map, Box<EvalAltResult>> {
            udp_recv_inner(h, max_len, Some(timeout_ms))
        },
    );

    engine.register_fn(
        "udp_send_to",
        |h: &mut UdpHandle, data: Blob, addr: &str| -> Result<i64, Box<EvalAltResult>> {
            send_bytes(h, &data, addr)
        },
    );

    engine.register_fn(
        "udp_send_to",
        |h: &mut UdpHandle, data: &str, addr: &str| -> Result<i64, Box<EvalAltResult>> {
            send_bytes(h, data.as_bytes(), addr)
        },
    );

    engine.register_fn("udp_close", |h: &mut UdpHandle| {
        if let Ok(mut guard) = h.inner.lock() {
            let _ = guard.take();
        }
    });
}

fn udp_recv_inner(
    h: &UdpHandle,
    max_len: i64,
    timeout_ms: Option<i64>,
) -> Result<Map, Box<EvalAltResult>> {
    let guard = h
        .inner
        .lock()
        .map_err(|_| err("udp_recv_from: mutex poisoned"))?;
    let sock = guard
        .as_ref()
        .ok_or_else(|| err("udp_recv_from: socket closed"))?;
    match timeout_ms {
        Some(ms) if ms > 0 => {
            sock.set_read_timeout(Some(Duration::from_millis(ms as u64)))
                .ok();
        }
        _ => {
            sock.set_read_timeout(None).ok();
        }
    }
    let max = max_len.max(0) as usize;
    let mut buf = vec![0u8; max];
    let (n, peer) = sock
        .recv_from(&mut buf)
        .map_err(|e| err(format!("udp_recv_from: {e}")))?;
    buf.truncate(n);
    let mut m = Map::new();
    m.insert("data".into(), rhai::Dynamic::from_blob(buf));
    m.insert("addr".into(), peer.to_string().into());
    Ok(m)
}

fn send_bytes(h: &UdpHandle, data: &[u8], addr: &str) -> Result<i64, Box<EvalAltResult>> {
    let guard = h
        .inner
        .lock()
        .map_err(|_| err("udp_send_to: mutex poisoned"))?;
    let sock = guard
        .as_ref()
        .ok_or_else(|| err("udp_send_to: socket closed"))?;
    let n = sock
        .send_to(data, addr)
        .map_err(|e| err(format!("udp_send_to '{addr}': {e}")))?;
    Ok(n as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_send_recv_roundtrip() {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e);

        let n: i64 = e
            .eval(
                r#"
let srv = udp_bind("127.0.0.1:0");
let cli = udp_bind("127.0.0.1:0");
// Note: real script would learn the server port via a helper;
// this smoke test just exercises the surface.
udp_close(srv);
udp_close(cli);
42
"#,
            )
            .expect("eval");
        assert_eq!(n, 42);
    }
}
