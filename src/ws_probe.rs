//! WebSocket probe. Opens a `ws://` or `wss://` connection (HTTP Upgrade
//! handshake), sends a Ping frame with an 8-byte nonce, waits for the
//! matching Pong, closes cleanly. Reports handshake metadata and ping
//! round-trip.
//!
//! Uses the `tungstenite` crate with rustls-webpki-roots for wss://.
//! Exit 0 on successful Ping/Pong; 7 on connect refused; 28 on timeout;
//! 67 on 401/403 handshake rejection.

use anyhow::{anyhow, Context, Result};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use tungstenite::{
    client::{client_with_config, IntoClientRequest},
    client_tls_with_config,
    handshake::HandshakeError,
    protocol::{Message, WebSocketConfig},
    stream::MaybeTlsStream,
};

/// Selected response headers from the server's upgrade response.
pub struct WsHeader {
    pub name: String,
    pub value: String,
}

pub struct WsProbeOk {
    pub host: String,
    pub port: u16,
    pub scheme: &'static str,
    pub connect_ms: f64,
    pub handshake_ms: f64,
    pub http_status: u16,
    pub headers: Vec<WsHeader>,
    pub pong_nonce_matched: bool,
    pub ping_round_trip_ms: f64,
}

pub fn probe(url: &str, timeout_secs: u64) -> Result<WsProbeOk> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let scheme = if url.starts_with("wss://") { "wss" } else { "ws" };
    let request = url
        .into_client_request()
        .with_context(|| format!("ws: invalid URL {url}"))?;

    let uri = request.uri();
    let host = uri
        .host()
        .ok_or_else(|| anyhow!("ws: URL missing host: {url}"))?
        .to_string();
    let port = uri
        .port_u16()
        .unwrap_or(if scheme == "wss" { 443 } else { 80 });

    let timeout = Duration::from_secs(timeout_secs);
    let connect_start = Instant::now();
    let tcp = connect_with_timeout(&host, port, timeout)?;
    let connect_ms = connect_start.elapsed().as_secs_f64() * 1000.0;

    tcp.set_read_timeout(Some(timeout)).ok();
    tcp.set_write_timeout(Some(timeout)).ok();

    let handshake_start = Instant::now();
    let (mut socket, response) = if scheme == "wss" {
        client_tls_with_config(request, tcp, Some(WebSocketConfig::default()), None).map_err(
            |e| match e {
                HandshakeError::Failure(err) => classify_handshake_error(err, &host),
                HandshakeError::Interrupted(_) => {
                    anyhow!("ws: handshake to {host} interrupted")
                }
            },
        )?
    } else {
        client_with_config(
            request,
            MaybeTlsStream::Plain(tcp),
            Some(WebSocketConfig::default()),
        )
        .map_err(|e| match e {
            HandshakeError::Failure(err) => classify_handshake_error(err, &host),
            HandshakeError::Interrupted(_) => {
                anyhow!("ws: handshake to {host} interrupted")
            }
        })?
    };
    let handshake_ms = handshake_start.elapsed().as_secs_f64() * 1000.0;

    let mut collected_headers: Vec<WsHeader> = Vec::new();
    for (k, v) in response.headers() {
        let name = k.as_str().to_ascii_lowercase();
        if matches!(
            name.as_str(),
            "sec-websocket-accept"
                | "sec-websocket-protocol"
                | "sec-websocket-extensions"
                | "server"
        ) {
            if let Ok(s) = v.to_str() {
                collected_headers.push(WsHeader {
                    name: k.as_str().to_string(),
                    value: s.to_string(),
                });
            }
        }
    }
    let http_status = response.status().as_u16();

    let nonce = b"reconprb".to_vec();
    let ping_start = Instant::now();
    socket
        .send(Message::Ping(nonce.clone().into()))
        .context("ws: send Ping")?;

    let (matched, rt_ms) = loop {
        let msg = socket.read().context("ws: read while waiting for Pong")?;
        match msg {
            Message::Pong(payload) => {
                let rt = ping_start.elapsed().as_secs_f64() * 1000.0;
                let matched = payload.as_ref() == nonce.as_slice();
                break (matched, rt);
            }
            Message::Close(_) => {
                return Err(anyhow!("ws: server closed before Pong"));
            }
            _ => continue,
        }
    };

    let _ = socket.close(None);

    Ok(WsProbeOk {
        host,
        port,
        scheme: if scheme == "wss" { "wss" } else { "ws" },
        connect_ms,
        handshake_ms,
        http_status,
        headers: collected_headers,
        pong_nonce_matched: matched,
        ping_round_trip_ms: rt_ms,
    })
}

pub fn run(url: &str, timeout_secs: u64) -> Result<()> {
    let r = probe(url, timeout_secs)?;
    println!(
        "Connected to {}:{} in {:.1}ms (TCP), handshake {:.1}ms",
        r.host, r.port, r.connect_ms, r.handshake_ms
    );
    println!("  HTTP status: {}", r.http_status);
    for h in &r.headers {
        println!("  {}: {}", h.name, h.value);
    }
    println!(
        "Pong: {}  round-trip {:.1}ms",
        if r.pong_nonce_matched {
            "matched nonce"
        } else {
            "unexpected payload"
        },
        r.ping_round_trip_ms
    );
    Ok(())
}

fn connect_with_timeout(host: &str, port: u16, timeout: Duration) -> Result<TcpStream> {
    use std::io::ErrorKind;
    use std::net::ToSocketAddrs;

    let addr = (host, port)
        .to_socket_addrs()
        .with_context(|| format!("ws: could not resolve {host}:{port}"))?
        .next()
        .ok_or_else(|| anyhow!("ws: no address for {host}:{port}"))?;

    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == ErrorKind::TimedOut => {
            Err(anyhow!("ws: connection to {host} timed out"))
                .context(crate::mqtt::ProtocolExitCode::OperationTimedOut)
        }
        Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
            Err(anyhow!("ws: connection refused by {host}"))
                .context(crate::mqtt::ProtocolExitCode::CouldntConnect)
        }
        Err(e) => Err(anyhow!("ws: connect to {host} failed: {e}"))
            .context(crate::mqtt::ProtocolExitCode::CouldntConnect),
    }
}

fn classify_handshake_error(err: tungstenite::Error, host: &str) -> anyhow::Error {
    match err {
        tungstenite::Error::Http(resp) => {
            let code = resp.status();
            let msg = format!("ws: handshake rejected by {host}: HTTP {code}");
            if code.as_u16() == 401 || code.as_u16() == 403 {
                anyhow!(msg).context(crate::mqtt::ProtocolExitCode::LoginDenied)
            } else {
                anyhow!(msg)
            }
        }
        tungstenite::Error::Io(e) if e.kind() == std::io::ErrorKind::TimedOut => {
            anyhow!("ws: handshake to {host} timed out")
                .context(crate::mqtt::ProtocolExitCode::OperationTimedOut)
        }
        other => anyhow!("ws: handshake to {host} failed: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use tungstenite::client::IntoClientRequest;

    #[test]
    fn ws_url_parses() {
        let r = "ws://example.com:9001/foo".into_client_request().unwrap();
        assert_eq!(r.uri().host(), Some("example.com"));
        assert_eq!(r.uri().port_u16(), Some(9001));
        assert_eq!(r.uri().path(), "/foo");
    }

    #[test]
    fn wss_url_parses() {
        let r = "wss://example.com/bar".into_client_request().unwrap();
        assert_eq!(r.uri().host(), Some("example.com"));
        assert_eq!(r.uri().scheme_str(), Some("wss"));
    }

}
