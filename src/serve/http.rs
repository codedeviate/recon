use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;

use super::{files, log_request, LogFile};

pub async fn serve(port: u16, root: Arc<PathBuf>, log_file: LogFile) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind HTTP listener on {addr}"))?;

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let root = Arc::clone(&root);
        let log_file = log_file.clone();

        tokio::spawn(async move {
            let root = root;
            let log_file = log_file;

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

            let result = hyper::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;

            if let Err(e) = result {
                let msg = e.to_string();
                if !msg.contains("connection reset") && !msg.contains("Connection reset") {
                    eprintln!("HTTP connection error: {e}");
                }
            }
        });
    }
}
