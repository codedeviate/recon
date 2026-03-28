use std::io;
use std::net::TcpStream;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use ssh2::{CheckResult, KnownHostFileKind, Session};

use crate::cli::Args;

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn download(raw_url: &str, args: &Args) -> Result<()> {
    let target = parse_scp_url(raw_url)?;
    let (user, password) = resolve_credentials(&target, args);

    if !args.silent {
        eprintln!("Connecting to {}@{}:{} …", user, target.host, target.port);
    }

    let tcp = TcpStream::connect(format!("{}:{}", target.host, target.port))
        .with_context(|| format!("Could not connect to {}:{}", target.host, target.port))?;

    let mut sess = Session::new().context("Failed to create SSH session")?;
    sess.set_tcp_stream(tcp);
    sess.handshake()
        .with_context(|| format!("SSH handshake failed with {}", target.host))?;
    sess.set_timeout((args.timeout * 1000) as u32);

    verify_host_key(&sess, &target.host, target.port, args.insecure)?;
    authenticate(&sess, &user, args, password.as_deref())?;
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

/// Returns (ssh_user, optional_password).
/// Priority for user:  URL userinfo > -u flag > $USER / $LOGNAME
/// Priority for pass:  --ssh-pass   > -u :pass suffix
fn resolve_credentials(target: &ScpTarget, args: &Args) -> (String, Option<String>) {
    let user = if !target.user.is_empty() {
        target.user.clone()
    } else if let Some(up) = &args.user {
        up.split(':').next().unwrap_or(up).to_string()
    } else {
        std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .unwrap_or_else(|_| "unknown".to_string())
    };

    let password = args.ssh_pass.clone().or_else(|| {
        args.user
            .as_ref()
            .and_then(|up| up.split_once(':').map(|(_, p)| p.to_string()))
    });

    (user, password)
}

// ── Host key verification ─────────────────────────────────────────────────────

fn verify_host_key(sess: &Session, host: &str, port: u16, insecure: bool) -> Result<()> {
    let (key_bytes, _key_type) = sess
        .host_key()
        .ok_or_else(|| anyhow!("Server did not present a host key"))?;

    if insecure {
        return Ok(());
    }

    let known_hosts_path = home_dir().join(".ssh").join("known_hosts");
    if !known_hosts_path.exists() {
        eprintln!(
            "warning: ~/.ssh/known_hosts not found — host key not verified.\n\
             Run `ssh {}` once to accept the key, or pass --insecure to silence this.",
            host
        );
        return Ok(());
    }

    let mut kh = sess
        .known_hosts()
        .context("Failed to initialise known_hosts")?;

    kh.read_file(&known_hosts_path, KnownHostFileKind::OpenSSH)
        .with_context(|| format!("Failed to read {}", known_hosts_path.display()))?;

    match kh.check_port(host, port, key_bytes) {
        CheckResult::Match => Ok(()),
        CheckResult::Mismatch => Err(anyhow!(
            "SSH host key MISMATCH for {host}:{port} — possible MITM attack.\n  \
             If the server was reinstalled, remove the old entry from ~/.ssh/known_hosts.\n  \
             Use --insecure to skip host key checking."
        )),
        CheckResult::NotFound => Err(anyhow!(
            "SSH host key for {host}:{port} is not in ~/.ssh/known_hosts.\n  \
             Connect once with `ssh {host}` to accept the key, or run:\n  \
               ssh-keyscan -p {port} {host} >> ~/.ssh/known_hosts\n  \
             Use --insecure to skip host key checking."
        )),
        CheckResult::Failure => Err(anyhow!(
            "SSH host key check failed for {host}:{port} (libssh2 internal error)"
        )),
    }
}

// ── Authentication ────────────────────────────────────────────────────────────

fn authenticate(
    sess: &Session,
    user: &str,
    args: &Args,
    password: Option<&str>,
) -> Result<()> {
    // 1. SSH agent
    if try_agent_auth(sess, user) {
        return Ok(());
    }

    // 2. Explicit key from --ssh-key
    if let Some(key_path) = &args.ssh_key {
        let pubkey = args.ssh_pubkey.as_deref();
        let passphrase = args.ssh_pass.as_deref();
        if sess
            .userauth_pubkey_file(user, pubkey, key_path, passphrase)
            .is_ok()
            && sess.authenticated()
        {
            return Ok(());
        }
    }

    // 3. Default key files
    let ssh_dir = home_dir().join(".ssh");
    for key_name in &["id_ed25519", "id_ecdsa", "id_rsa", "id_dsa"] {
        let priv_path = ssh_dir.join(key_name);
        if !priv_path.exists() {
            continue;
        }
        let passphrase = args.ssh_pass.as_deref();
        // pubkey: None — libssh2 derives it from the private key file
        if sess
            .userauth_pubkey_file(user, None, &priv_path, passphrase)
            .is_ok()
            && sess.authenticated()
        {
            return Ok(());
        }
    }

    // 4. Password auth
    if let Some(pass) = password {
        sess.userauth_password(user, pass)
            .context("SSH password authentication failed")?;
        if sess.authenticated() {
            return Ok(());
        }
    }

    Err(anyhow!(
        "All SSH authentication methods exhausted for user '{user}'.\n  \
         Tried: agent, default key files (~/.ssh/id_ed25519 etc.), password.\n  \
         Provide a key with --ssh-key or a password with --ssh-pass."
    ))
}

fn try_agent_auth(sess: &Session, user: &str) -> bool {
    let mut agent = match sess.agent() {
        Ok(a) => a,
        Err(_) => return false,
    };
    if agent.connect().is_err() {
        return false;
    }
    if agent.list_identities().is_err() {
        return false;
    }
    let identities = match agent.identities() {
        Ok(ids) => ids,
        Err(_) => return false,
    };
    for identity in &identities {
        if agent.userauth(user, identity).is_ok() && sess.authenticated() {
            return true;
        }
    }
    false
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

// ── Helpers ───────────────────────────────────────────────────────────────────

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
