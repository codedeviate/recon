pub mod files;
pub mod http;
pub mod https;

use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Shared log file handle (None if --serve-log was not given).
pub type LogFile = Option<Arc<Mutex<fs::File>>>;

/// Configuration for the file server, built from CLI flags.
pub struct ServeConfig {
    pub http_port: Option<u16>,
    pub https_port: Option<u16>,
    pub http_version: Option<String>,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub log_file: Option<PathBuf>,
    pub root_dir: PathBuf,
}

/// Run the file server (blocking — creates its own tokio runtime).
pub fn run(config: &ServeConfig) -> Result<()> {
    // Validate root directory
    let root = config
        .root_dir
        .canonicalize()
        .with_context(|| format!("Cannot serve directory: {}", config.root_dir.display()))?;
    if !root.is_dir() {
        bail!("Not a directory: {}", root.display());
    }

    // Validate cert/key if HTTPS is requested
    if config.https_port.is_some() {
        if !config.cert_path.exists() {
            bail!(
                "TLS certificate not found: {}\n\
                 Hint: generate a self-signed cert with:\n  \
                 openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \\\n    \
                 -keyout {} -out {} -days 365 -nodes -subj '/CN=localhost'",
                config.cert_path.display(),
                config.key_path.display(),
                config.cert_path.display()
            );
        }
        if !config.key_path.exists() {
            bail!(
                "TLS private key not found: {}\n\
                 Hint: generate a self-signed cert with:\n  \
                 openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \\\n    \
                 -keyout {} -out {} -days 365 -nodes -subj '/CN=localhost'",
                config.key_path.display(),
                config.key_path.display(),
                config.cert_path.display()
            );
        }
    }

    // Warn if --http-version 2 without --serve-tls
    if config.http_version.as_deref() == Some("2") && config.https_port.is_none() {
        eprintln!(
            "{} HTTP/2 typically requires TLS. Consider adding --serve-tls.",
            "warning:".yellow().bold()
        );
    }

    // Open log file
    let log_file: LogFile = match &config.log_file {
        Some(path) => {
            let f = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .with_context(|| format!("Cannot open log file: {}", path.display()))?;
            Some(Arc::new(Mutex::new(f)))
        }
        None => None,
    };

    print_banner(config, &root);

    // Build multi-threaded tokio runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to create tokio runtime")?;

    let root = Arc::new(root);

    runtime.block_on(async {
        let mut tasks = Vec::new();

        if let Some(port) = config.http_port {
            let r = Arc::clone(&root);
            let lf = log_file.clone();
            tasks.push(tokio::spawn(async move {
                if let Err(e) = http::serve(port, r, lf).await {
                    eprintln!("{} HTTP server error: {e}", "error:".red().bold());
                }
            }));
        }

        if let Some(port) = config.https_port {
            let r = Arc::clone(&root);
            let lf = log_file.clone();
            let cert = config.cert_path.clone();
            let key = config.key_path.clone();
            let hv = config.http_version.clone();
            tasks.push(tokio::spawn(async move {
                if let Err(e) =
                    https::serve(port, r, lf, &cert, &key, hv.as_deref()).await
                {
                    eprintln!("{} HTTPS server error: {e}", "error:".red().bold());
                }
            }));
        }

        // Wait for Ctrl+C
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for Ctrl+C");
        eprintln!("\n{}", "Shutting down...".dimmed());

        // Abort spawned tasks
        for t in &tasks {
            t.abort();
        }
    });

    Ok(())
}

fn print_banner(config: &ServeConfig, root: &PathBuf) {
    eprintln!(
        "\n{}  {}",
        "Serving".green().bold(),
        root.display().to_string().bold()
    );
    eprintln!();
    if let Some(port) = config.http_port {
        eprintln!(
            "  {} http://0.0.0.0:{}",
            "HTTP ".cyan().bold(),
            port
        );
    }
    if let Some(port) = config.https_port {
        eprintln!(
            "  {} https://0.0.0.0:{}",
            "HTTPS".cyan().bold(),
            port
        );
    }
    eprintln!();
    eprintln!("  Press {} to stop", "Ctrl+C".bold());
    eprintln!();
}

/// Log a request to the terminal (coloured) and optionally to a log file.
pub fn log_request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    status: u16,
    bytes: u64,
    elapsed: Duration,
    log_file: &LogFile,
) {
    let status_str = status.to_string();
    let coloured_status = match status {
        200..=299 => status_str.green(),
        300..=399 => status_str.yellow(),
        _ => status_str.red(),
    };

    eprintln!(
        "{} {} {} {} {} {:.1}ms",
        addr.to_string().dimmed(),
        method.bold(),
        path,
        coloured_status,
        files::humanize_size(bytes).dimmed(),
        elapsed.as_secs_f64() * 1000.0
    );

    if let Some(file) = log_file {
        if let Ok(mut f) = file.lock() {
            let _ = writeln!(
                f,
                "{} {} {} {} {} {:.1}ms",
                addr,
                method,
                path,
                status,
                files::humanize_size(bytes),
                elapsed.as_secs_f64() * 1000.0
            );
        }
    }
}
