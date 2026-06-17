//! Version banner output.
//!
//! `print_full` matches curl's `--version` format so existing curl tooling
//! (e.g. `grep -q HTTP2 <(curl --version)`) works against recon output.
//! `print_short` prints just `recon <version>` for scripts that only want
//! the number.

const RELEASE_DATE: &str = "2026-06-17";

// Update these when the corresponding Cargo.toml entries change majors/minors.
// Patch-version drift is not reflected — the banner reports the API surface.
const REQWEST_VERSION: &str = "0.12";
const RUSTLS_VERSION: &str = "0.23";

/// Protocols recon can speak. Keep this list in sync with the URL-scheme
/// dispatch in `main.rs` (and the `source::resolve_file_url` branch for
/// `file://`). When adding or removing protocol support, update this list
/// so `recon --version | grep <proto>` stays accurate. Rendered sorted
/// case-insensitively in `print_full`.
// scp / sftp / ssh are gated behind the `ssh` feature (default-on, dropped
// in the impersonate variant — see GitHub issue #1). They are appended
// conditionally in `print_full` rather than listed here.
const PROTOCOLS: &[&str] = &[
    "dict", "dig", "dns", "drill", "file", "ftp", "ftps", "gopher", "gophers",
    "http", "https", "imap", "imaps", "ipfs", "ipns", "ldap", "ldaps", "memcached",
    "mqtt", "mqtts", "ntp", "ping", "pop3", "pop3s", "redis", "rtsp", "rtsps",
    "smtp", "smtps", "tcp", "telnet", "tftp", "tls",
    "traceroute", "udp", "whois", "ws", "wss",
];

/// Feature tokens. Kept curl-compatible where the concept overlaps (HTTP2,
/// HTTPS, IPv6, SSL, gzip, deflate, brotli, zstd) and recon-specific
/// otherwise (rustls-tls, charset, DKIM-signing, JWT, etc.). Rendered
/// sorted case-insensitively in `print_full` so the output scans easily.
const FEATURES: &[&str] = &[
    "age-encrypt",
    "archive",
    "AsynchDNS",
    "browser",
    "brotli",
    "aztec",
    "charset",
    "checkdigit",
    "client-cert",
    "compare",
    "compression",
    "decode",
    "decode-all",
    "flag-listing",
    "interface-name-resolution",
    "latam-tax-ids",
    "mqtt-mtls",
    "range",
    "conditional-get",
    "spider",
    "xattr",
    "multipart",
    "netrc",
    "http-version-pinning",
    "retry",
    "proto-filter",
    "input-file",
    "resume",
    "config-file",
    "DKIM-signing",
    "email-dns",
    "encode",
    "gzip",
    "deflate",
    "hashes",
    "HSTS",
    "HTTP2",
    "HTTPS",
    "HTTPS-proxy",
    "IPv6",
    "JWT",
    "Largefile",
    "libz",
    "markdown",
    "html-render",
    "maxicode-decode",
    "MQTT5",
    "pdf-cover-page",
    "pdf-export",
    "netstatus",
    "pdf417",
    "PGP-shellout",
    "sample",
    "script-concurrency",
    "script-servers",
    "SOCKS5",
    "sqlite",
    "SSL",
    "rustls-tls",
    "threadsafe",
    "UnixSockets",
    "zstd",
];

pub fn print_full() {
    let mut protocols: Vec<&str> = PROTOCOLS.to_vec();
    #[cfg(feature = "ssh")]
    protocols.extend_from_slice(&["scp", "sftp", "ssh"]);
    protocols.sort_by_key(|s| s.to_ascii_lowercase());

    // ssh-pinning / ssh-compress are libssh2 transport capabilities;
    // only present when the `ssh` feature is compiled in.
    let mut features: Vec<&str> = FEATURES.to_vec();
    #[cfg(feature = "ssh")]
    features.extend_from_slice(&["ssh-pinning", "ssh-compress"]);
    #[cfg(feature = "impersonate")]
    features.push("TLS-impersonation");
    features.sort_by_key(|s| s.to_ascii_lowercase());

    println!(
        "recon {} (reqwest/{} rustls/{})",
        env!("CARGO_PKG_VERSION"),
        REQWEST_VERSION,
        RUSTLS_VERSION,
    );
    println!("Release-Date: {}", RELEASE_DATE);
    println!("Protocols: {}", protocols.join(" "));
    println!("Features: {}", features.join(" "));
}

pub fn print_short() {
    println!("recon {}", env!("CARGO_PKG_VERSION"));
}
