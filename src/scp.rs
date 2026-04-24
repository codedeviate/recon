use std::io;
use std::net::TcpStream;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use ssh2::Session;

use crate::cli::Args;

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn download(raw_url: &str, args: &Args) -> Result<()> {
    let target = parse_scp_url(raw_url)?;
    let (user, password) = crate::ssh_auth::resolve_credentials(&target.user, args);

    if !args.silent {
        eprintln!("Connecting to {}@{}:{} …", user, target.host, target.port);
    }

    let tcp = TcpStream::connect(format!("{}:{}", target.host, target.port))
        .with_context(|| format!("Could not connect to {}:{}", target.host, target.port))?;

    let mut sess = Session::new().context("Failed to create SSH session")?;
    sess.set_tcp_stream(tcp);
    if args.compressed_ssh {
        sess.set_compress(true);
    }
    sess.handshake()
        .with_context(|| format!("SSH handshake failed with {}", target.host))?;
    sess.set_timeout((args.timeout * 1000) as u32);

    crate::ssh_auth::verify_host_key_with_pins(
        &sess,
        &target.host,
        target.port,
        args.insecure,
        args.hostpubsha256.as_deref(),
        args.hostpubmd5.as_deref(),
    )?;
    crate::ssh_auth::authenticate(&sess, &user, args, password.as_deref())?;
    download_file(&sess, &target.path, args)?;

    Ok(())
}

// ── URL / credential parsing ──────────────────────────────────────────────────

struct ScpTarget {
    user: String,
    host: String,
    port: u16,
    path: PathBuf,
}

fn parse_scp_url(raw: &str) -> Result<ScpTarget> {
    let parsed = url::Url::parse(raw)
        .with_context(|| format!("Invalid SCP URL: {raw}"))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("SCP URL missing host: {raw}"))?
        .to_string();

    let port = parsed.port().unwrap_or(22);

    let path_str = parsed.path();
    if path_str.is_empty() || path_str == "/" {
        return Err(anyhow!("SCP URL missing remote path: {raw}"));
    }

    let user = parsed.username().to_string();

    Ok(ScpTarget {
        user,
        host,
        port,
        path: PathBuf::from(path_str),
    })
}

// ── File download ─────────────────────────────────────────────────────────────

fn download_file(sess: &Session, remote_path: &Path, args: &Args) -> Result<()> {
    let (mut channel, stat) = sess
        .scp_recv(remote_path)
        .with_context(|| format!("SCP failed for {}", remote_path.display()))?;

    let file_size = stat.size();
    let out_path = resolve_output_path(remote_path, args)?;

    if !args.silent {
        eprintln!(
            "Downloading {} ({} bytes) → {}",
            remote_path.display(),
            file_size,
            out_path.display()
        );
    }

    let mut file =
        std::fs::File::create(&out_path)
            .with_context(|| format!("Cannot create output file: {}", out_path.display()))?;

    if args.progress && file_size > 0 {
        let pb = crate::output::make_progress_bar(Some(file_size));
        crate::output::copy_with_progress(&mut channel, &mut file, &pb)?;
        pb.finish_and_clear();
    } else {
        io::copy(&mut channel, &mut file).context("Error reading SCP stream")?;
    }

    // Required EOF/close sequence — libssh2 will stall without this
    channel.send_eof().context("Failed to send SSH channel EOF")?;
    channel.wait_eof().context("Failed to wait for SSH channel EOF")?;
    channel.close().context("Failed to close SSH channel")?;
    channel.wait_close().context("Failed to wait for SSH channel close")?;

    if !args.silent {
        eprintln!("Saved to {}", out_path.display());
    }

    Ok(())
}

fn resolve_output_path(remote_path: &Path, args: &Args) -> Result<PathBuf> {
    let basename = remote_path
        .file_name()
        .ok_or_else(|| anyhow!("Cannot determine filename from remote path: {}", remote_path.display()))?;

    match &args.output {
        Some(p) if p.is_dir() => Ok(p.join(basename)),
        Some(p) => Ok(p.clone()),
        None => Ok(PathBuf::from(basename)),
    }
}

