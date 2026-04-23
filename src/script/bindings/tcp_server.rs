//! TCP server primitives for scripts — `tcp_listen`, `tcp_accept`,
//! `tcp_read`, `tcp_read_line`, `tcp_write`, `tcp_peer_addr`,
//! `tcp_close`, `tcp_close_listener`.
//!
//! Handles wrap `Arc<Mutex<TcpListener>>` / `Arc<Mutex<TcpStream>>`
//! so they're Send+Sync — safe to move into spawned closures from
//! 0.56.0's `thread_spawn`. That pairing is the main use case:
//! accept on the main thread, spawn a handler per connection.

use crate::script::convert::err;
use rhai::{Blob, Engine, EvalAltResult};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct TcpListenerHandle {
    inner: Arc<Mutex<Option<TcpListener>>>,
}

#[derive(Clone)]
pub struct TcpConnHandle {
    inner: Arc<Mutex<Option<TcpStream>>>,
    peer: Arc<String>,
}

pub fn register(engine: &mut Engine) {
    engine.register_type_with_name::<TcpListenerHandle>("TcpListener");
    engine.register_type_with_name::<TcpConnHandle>("TcpConn");

    engine.register_fn(
        "tcp_listen",
        |addr: &str| -> Result<TcpListenerHandle, Box<EvalAltResult>> {
            let listener = TcpListener::bind(addr)
                .map_err(|e| err(format!("tcp_listen '{addr}': {e}")))?;
            Ok(TcpListenerHandle {
                inner: Arc::new(Mutex::new(Some(listener))),
            })
        },
    );

    engine.register_fn(
        "tcp_accept",
        |l: &mut TcpListenerHandle| -> Result<TcpConnHandle, Box<EvalAltResult>> {
            accept_with_timeout(l, None)
        },
    );

    engine.register_fn(
        "tcp_accept",
        |l: &mut TcpListenerHandle, timeout_ms: i64|
         -> Result<TcpConnHandle, Box<EvalAltResult>> {
            accept_with_timeout(l, Some(timeout_ms))
        },
    );

    engine.register_fn(
        "tcp_read",
        |c: &mut TcpConnHandle, n: i64, timeout_ms: i64|
         -> Result<Blob, Box<EvalAltResult>> {
            let mut guard = c
                .inner
                .lock()
                .map_err(|_| err("tcp_read: mutex poisoned"))?;
            let stream = guard
                .as_mut()
                .ok_or_else(|| err("tcp_read: connection closed"))?;
            if timeout_ms > 0 {
                stream
                    .set_read_timeout(Some(Duration::from_millis(timeout_ms as u64)))
                    .ok();
            } else {
                stream.set_read_timeout(None).ok();
            }
            let n = n.max(0) as usize;
            let mut buf = vec![0u8; n];
            let read = stream
                .read(&mut buf)
                .map_err(|e| err(format!("tcp_read: {e}")))?;
            buf.truncate(read);
            Ok(buf)
        },
    );

    engine.register_fn(
        "tcp_read_line",
        |c: &mut TcpConnHandle, timeout_ms: i64|
         -> Result<String, Box<EvalAltResult>> {
            let mut guard = c
                .inner
                .lock()
                .map_err(|_| err("tcp_read_line: mutex poisoned"))?;
            let stream = guard
                .as_mut()
                .ok_or_else(|| err("tcp_read_line: connection closed"))?;
            if timeout_ms > 0 {
                stream
                    .set_read_timeout(Some(Duration::from_millis(timeout_ms as u64)))
                    .ok();
            } else {
                stream.set_read_timeout(None).ok();
            }
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| err(format!("tcp_read_line: {e}")))?;
            while line.ends_with('\n') || line.ends_with('\r') {
                line.pop();
            }
            Ok(line)
        },
    );

    engine.register_fn(
        "tcp_write",
        |c: &mut TcpConnHandle, data: Blob| -> Result<i64, Box<EvalAltResult>> {
            write_bytes(c, &data)
        },
    );

    engine.register_fn(
        "tcp_write",
        |c: &mut TcpConnHandle, data: &str| -> Result<i64, Box<EvalAltResult>> {
            write_bytes(c, data.as_bytes())
        },
    );

    engine.register_fn("tcp_peer_addr", |c: &mut TcpConnHandle| -> String {
        (*c.peer).clone()
    });

    engine.register_fn("tcp_close", |c: &mut TcpConnHandle| {
        if let Ok(mut guard) = c.inner.lock() {
            let _ = guard.take();
        }
    });

    engine.register_fn("tcp_close_listener", |l: &mut TcpListenerHandle| {
        if let Ok(mut guard) = l.inner.lock() {
            let _ = guard.take();
        }
    });
}

fn accept_with_timeout(
    l: &TcpListenerHandle,
    timeout_ms: Option<i64>,
) -> Result<TcpConnHandle, Box<EvalAltResult>> {
    let guard = l
        .inner
        .lock()
        .map_err(|_| err("tcp_accept: listener mutex poisoned"))?;
    let listener = guard
        .as_ref()
        .ok_or_else(|| err("tcp_accept: listener closed"))?;
    // set_nonblocking + poll is overkill; std's accept is blocking and
    // there's no accept-timeout. For the timeout form we set
    // non-blocking, spin with short sleeps, and bail on timeout.
    let deadline = timeout_ms.and_then(|ms| {
        if ms <= 0 {
            None
        } else {
            Some(std::time::Instant::now() + Duration::from_millis(ms as u64))
        }
    });
    if deadline.is_some() {
        listener
            .set_nonblocking(true)
            .map_err(|e| err(format!("tcp_accept: set_nonblocking: {e}")))?;
    } else {
        let (stream, peer) = listener
            .accept()
            .map_err(|e| err(format!("tcp_accept: {e}")))?;
        return Ok(TcpConnHandle {
            inner: Arc::new(Mutex::new(Some(stream))),
            peer: Arc::new(peer.to_string()),
        });
    }

    loop {
        match listener.accept() {
            Ok((stream, peer)) => {
                // Revert to blocking so subsequent reads behave naturally.
                stream.set_nonblocking(false).ok();
                listener.set_nonblocking(false).ok();
                return Ok(TcpConnHandle {
                    inner: Arc::new(Mutex::new(Some(stream))),
                    peer: Arc::new(peer.to_string()),
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if let Some(deadline) = deadline {
                    if std::time::Instant::now() >= deadline {
                        listener.set_nonblocking(false).ok();
                        return Err(err("tcp_accept: timeout"));
                    }
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => {
                listener.set_nonblocking(false).ok();
                return Err(err(format!("tcp_accept: {e}")));
            }
        }
    }
}

fn write_bytes(c: &TcpConnHandle, data: &[u8]) -> Result<i64, Box<EvalAltResult>> {
    let mut guard = c
        .inner
        .lock()
        .map_err(|_| err("tcp_write: mutex poisoned"))?;
    let stream = guard
        .as_mut()
        .ok_or_else(|| err("tcp_write: connection closed"))?;
    stream
        .write_all(data)
        .map_err(|e| err(format!("tcp_write: {e}")))?;
    Ok(data.len() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let mut e = Engine::new();
        super::super::helpers::register(&mut e);
        register(&mut e);
        e
    }

    #[test]
    fn listen_and_close() {
        let e = engine();
        // Bind to an ephemeral port; immediately close.
        let _: () = e
            .eval(r#"let l = tcp_listen("127.0.0.1:0"); tcp_close_listener(l);"#)
            .expect("eval");
    }

    #[test]
    fn accept_timeout_errors_cleanly() {
        let e = engine();
        // No connecting client; 50ms timeout should fire.
        let res: Result<(), _> = e.eval(
            r#"
let l = tcp_listen("127.0.0.1:0");
tcp_accept(l, 50);
tcp_close_listener(l);
"#,
        );
        assert!(res.is_err());
    }
}
