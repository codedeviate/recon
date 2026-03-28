use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use rustls::ServerConfig;
use std::convert::Infallible;
use std::fs;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use super::{files, log_request, LogFile};

fn build_tls_config(
    cert_path: &Path,
    key_path: &Path,
    http_version: Option<&str>,
) -> Result<ServerConfig> {
    let cert_bytes = fs::read(cert_path)
        .with_context(|| format!("Failed to read certificate: {}", cert_path.display()))?;
    let key_bytes = fs::read(key_path)
        .with_context(|| format!("Failed to read private key: {}", key_path.display()))?;

    let certs: Vec<rustls::pki_types::CertificateDer<'static>> =
        rustls_pemfile::certs(&mut BufReader::new(cert_bytes.as_slice()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("Failed to parse TLS certificates")?;

    let key = rustls_pemfile::private_key(&mut BufReader::new(key_bytes.as_slice()))
        .context("Failed to parse private key")?
        .context("No private key found in key file")?;

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .context("Failed to configure TLS protocol versions")?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("Failed to build TLS server config")?;

    config.alpn_protocols = match http_version {
        Some("1.1") => vec![b"http/1.1".to_vec()],
        Some("2") => vec![b"h2".to_vec()],
        _ => vec![b"h2".to_vec(), b"http/1.1".to_vec()],
    };

    Ok(config)
}

pub async fn serve(
    port: u16,
    root: Arc<PathBuf>,
    log_file: LogFile,
    cert_path: &Path,
    key_path: &Path,
    http_version: Option<&str>,
) -> Result<()> {
    let tls_config = build_tls_config(cert_path, key_path, http_version)?;
    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind HTTPS listener on {addr}"))?;

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();
        let root = Arc::clone(&root);
        let log_file = log_file.clone();

        tokio::spawn(async move {
            // Perform TLS handshake
            let tls_stream = match tls_acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    let msg = e.to_string();
                    if !msg.contains("unexpected EOF") && !msg.contains("UnexpectedEof") {
                        eprintln!("TLS handshake error from {remote_addr}: {e}");
                    }
                    return;
                }
            };

            // Determine negotiated protocol
            let alpn = tls_stream
                .get_ref()
                .1
                .alpn_protocol()
                .map(|p| p.to_vec());

            let use_h2 = alpn.as_deref() == Some(b"h2");

            let service = service_fn(move |req: Request<Incoming>| {
                let root = Arc::clone(&root);
                let log_file = log_file.clone();
                async move {
                    let start = Instant::now();
                    let method = req.method().to_string();
                    let path = req.uri().path().to_string();

                    let served = files::handle_request(&req, &root);
                    let status = served.response.status().as_u16();
                    let bytes = served.bytes;
                    let elapsed = start.elapsed();

                    log_request(remote_addr, &method, &path, status, bytes, elapsed, &log_file);

                    Ok::<Response<Full<Bytes>>, Infallible>(served.response)
                }
            });

            let io = TokioIo::new(tls_stream);

            let result = if use_h2 {
                hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                    .serve_connection(io, service)
                    .await
            } else {
                hyper::server::conn::http1::Builder::new()
                    .serve_connection(io, service)
                    .await
            };

            if let Err(e) = result {
                let msg = e.to_string();
                if !msg.contains("connection reset") && !msg.contains("Connection reset") {
                    eprintln!("HTTPS connection error: {e}");
                }
            }
        });
    }
}
