//! Version banner output.
//!
//! `print_full` matches curl's `--version` format so existing curl tooling
//! (e.g. `grep -q HTTP2 <(curl --version)`) works against recon output.
//! `print_short` prints just `recon <version>` for scripts that only want
//! the number.

const RELEASE_DATE: &str = "2026-04-20";

// Update these when the corresponding Cargo.toml entries change majors/minors.
// Patch-version drift is not reflected — the banner reports the API surface.
const REQWEST_VERSION: &str = "0.12";
const RUSTLS_VERSION: &str = "0.23";

/// Protocols recon can speak. Keep this list in sync with the URL-scheme
/// dispatch in `main.rs` (and the `source::resolve_file_url` branch for
/// `file://`). When adding or removing protocol support, update this list
/// so `recon --version | grep <proto>` stays accurate.
const PROTOCOLS: &[&str] = &[
    "dns", "file", "http", "https", "mqtt", "mqtts", "ntp", "ping", "scp",
    "ssh", "tcp", "telnet", "tls", "traceroute", "udp", "whois",
];

/// Feature tokens. Kept curl-compatible where the concept overlaps (HTTP2,
/// HTTPS, IPv6, SSL, gzip, deflate, brotli, zstd) and recon-specific
/// otherwise (rustls-tls).
const FEATURES: &[&str] = &[
    "HTTP2",
    "HTTPS",
    "IPv6",
    "SSL",
    "gzip",
    "deflate",
    "brotli",
    "zstd",
    "rustls-tls",
];

pub fn print_full() {
    println!(
        "recon {} (reqwest/{} rustls/{})",
        env!("CARGO_PKG_VERSION"),
        REQWEST_VERSION,
        RUSTLS_VERSION,
    );
    println!("Release-Date: {}", RELEASE_DATE);
    println!("Protocols: {}", PROTOCOLS.join(" "));
    println!("Features: {}", FEATURES.join(" "));
}

pub fn print_short() {
    println!("recon {}", env!("CARGO_PKG_VERSION"));
}
