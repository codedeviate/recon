//! `sftp://[user[:pass]@]host[:port]/path` — SSH-backed file transfer.
//!
//! Reuses the SSH auth + host-key-verification scaffolding from
//! `src/ssh_auth.rs` (shared with scp / ssh). Path semantics match curl:
//!
//!   sftp://user@host/            -> list home directory
//!   sftp://user@host/dir/        -> list that directory
//!   sftp://user@host/file        -> retrieve that file

use crate::cli::Args;
use crate::mqtt::ProtocolExitCode;
use anyhow::{anyhow, bail, Context, Result};
use ssh2::Session;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub enum SftpMode {
    List(Vec<SftpEntry>),
    Retrieve(Vec<u8>),
}

pub struct SftpEntry {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub mode: Option<u32>,
}

pub struct SftpProbeOk {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub connect_ms: f64,
    pub path: String,
    pub mode: SftpMode,
}

pub fn probe(url: &str, args: &Args) -> Result<SftpProbeOk> {
    let target = parse_url(url)?;
    let (user, password) = crate::ssh_auth::resolve_credentials(&target.user, args);

    let t0 = Instant::now();
    let tcp = TcpStream::connect(format!("{}:{}", target.host, target.port))
        .map_err(|e| {
            anyhow!("sftp: connect {}:{}: {e}", target.host, target.port)
                .context(ProtocolExitCode::CouldntConnect)
        })?;

    let mut sess = Session::new().context("sftp: session init")?;
    sess.set_tcp_stream(tcp);
    sess.handshake()
        .with_context(|| format!("sftp: handshake with {}", target.host))?;
    sess.set_timeout((args.timeout * 1000) as u32);

    crate::ssh_auth::verify_host_key(&sess, &target.host, target.port, args.insecure)?;
    crate::ssh_auth::authenticate(&sess, &user, args, password.as_deref())
        .map_err(|e| {
            anyhow!("sftp: auth: {e}").context(ProtocolExitCode::LoginDenied)
        })?;
    let connect_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let sftp = sess.sftp().context("sftp: open subsystem")?;

    let mode = if target.path.is_empty() || target.path.ends_with('/') {
        let dir = if target.path.is_empty() { "." } else { &target.path };
        let listing = sftp
            .readdir(Path::new(dir))
            .with_context(|| format!("sftp: readdir '{dir}'"))?;
        let entries = listing
            .into_iter()
            .map(|(p, stat)| SftpEntry {
                name: p.file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| p.display().to_string()),
                size: stat.size.unwrap_or(0),
                is_dir: stat.is_dir(),
                mode: stat.perm,
            })
            .collect();
        SftpMode::List(entries)
    } else {
        let mut f = sftp
            .open(Path::new(&target.path))
            .with_context(|| format!("sftp: open '{}'", target.path))?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).context("sftp: read file")?;
        SftpMode::Retrieve(buf)
    };

    Ok(SftpProbeOk {
        host: target.host,
        port: target.port,
        user,
        connect_ms,
        path: target.path,
        mode,
    })
}

pub fn run(url: &str, args: &Args) -> Result<()> {
    let r = probe(url, args)?;
    eprintln!(
        "Connected to {}@{}:{} in {:.1}ms",
        r.user, r.host, r.port, r.connect_ms
    );
    match r.mode {
        SftpMode::List(entries) => {
            for e in entries {
                let kind = if e.is_dir { "d" } else { "-" };
                let mode = e.mode.map(|m| format!("{:o}", m & 0o777)).unwrap_or_else(|| "---".into());
                println!("{kind} {mode:>4} {:>10} {}", e.size, e.name);
            }
        }
        SftpMode::Retrieve(bytes) => {
            if let Some(path) = &args.output {
                let resolved: PathBuf = if path.is_dir() {
                    let basename = Path::new(&r.path)
                        .file_name()
                        .ok_or_else(|| anyhow!("sftp: cannot derive filename"))?;
                    path.join(basename)
                } else {
                    path.clone()
                };
                std::fs::write(&resolved, &bytes)
                    .with_context(|| format!("sftp: write {}", resolved.display()))?;
                if !args.silent {
                    eprintln!("Saved to {}", resolved.display());
                }
            } else {
                std::io::stdout().write_all(&bytes)?;
            }
        }
    }
    Ok(())
}

struct SftpTarget {
    user: String,
    host: String,
    port: u16,
    path: String,
}

fn parse_url(raw: &str) -> Result<SftpTarget> {
    let parsed = url::Url::parse(raw).with_context(|| format!("sftp: bad URL '{raw}'"))?;
    if parsed.scheme() != "sftp" {
        bail!("sftp: scheme must be sftp (got '{}')", parsed.scheme());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("sftp: URL missing host"))?
        .to_string();
    let port = parsed.port().unwrap_or(22);
    let user = parsed.username().to_string();
    // Keep trailing slash for list/retrieve dispatch. Path starts with '/'.
    let path = parsed.path();
    let path = if path.is_empty() || path == "/" {
        String::new()
    } else {
        path.trim_start_matches('/').to_string()
    };
    Ok(SftpTarget { user, host, port, path })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_url() {
        let t = parse_url("sftp://alice@host/file.txt").unwrap();
        assert_eq!(t.user, "alice");
        assert_eq!(t.host, "host");
        assert_eq!(t.port, 22);
        assert_eq!(t.path, "file.txt");
    }

    #[test]
    fn parse_dir_listing_url() {
        let t = parse_url("sftp://host/some/dir/").unwrap();
        assert!(t.path.ends_with('/'));
    }

    #[test]
    fn parse_custom_port() {
        let t = parse_url("sftp://bob@host:2222/").unwrap();
        assert_eq!(t.port, 2222);
        assert_eq!(t.path, "");
    }

    #[test]
    fn parse_rejects_non_sftp() {
        assert!(parse_url("ssh://host/").is_err());
    }
}
