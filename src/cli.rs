use clap::builder::styling::{AnsiColor, Styles};
use clap::Parser;
use std::path::PathBuf;

const HELP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::Yellow.on_default().bold())
    .literal(AnsiColor::Cyan.on_default())
    .placeholder(AnsiColor::Green.on_default());

#[derive(Parser, Debug, Clone)]
#[command(
    name = "recon",
    about = "A versatile network reconnaissance tool",
    version,
    styles = HELP_STYLES,
    disable_help_flag = true,
    disable_version_flag = true,
)]
pub struct Args {
    // ── Positional (renders under Arguments; no help_heading) ────────────────

    /// URL to request (or use --url)
    #[arg(required_unless_present_any = ["url_flag", "cookies", "cookie_delete", "cookie_set", "spf", "dmarc", "dkim", "mta_sts", "bimi", "tls_rpt", "serve", "serve_tls", "serve_sni", "jwt_view", "jwt_sign", "jwt_validate", "netstatus", "editor_cleanup", "sample", "sample_list", "hash", "hash_list", "compress", "decompress", "compress_list", "encode", "encode_list", "encrypt", "decrypt", "encrypt_keygen", "checkdigit", "checkdigit_create", "checkdigit_list", "script", "init", "browser_screenshot", "archive", "extract", "iconv", "list_charsets", "compare", "decode", "decode_all", "md_to_html", "md_to_pdf", "html_to_pdf", "input_file"])]
    pub url: Option<String>,

    // ── HTTP Request ─────────────────────────────────────────────────────────

    /// URL to request — curl-compatible alternative to the positional argument
    #[arg(id = "url_flag", long = "url", value_name = "URL", help_heading = "HTTP Request")]
    pub url_flag: Option<String>,

    /// HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD). When omitted, the method
    /// defaults to GET — or PUT when -T is set, or POST when -d is set.
    #[arg(short = 'X', long = "request", help_heading = "HTTP Request")]
    pub method: Option<String>,

    /// Send request headers (can be repeated: -H "Name: Value")
    #[arg(short = 'H', long = "header", help_heading = "HTTP Request")]
    pub header: Vec<String>,

    /// Request body data. Prefix with @ to read from file (e.g. @body.json)
    #[arg(short = 'd', long = "data", help_heading = "HTTP Request")]
    pub data: Option<String>,

    /// Send data as JSON — auto-sets Content-Type and Accept headers. Supports
    /// @file / @- like -d. Stacks with -d (last body wins; headers merge).
    #[arg(long = "json", value_name = "DATA", help_heading = "HTTP Request")]
    pub json: Option<String>,

    /// Like -d, but @file is not processed — sends the literal string.
    #[arg(long = "data-raw", value_name = "DATA", help_heading = "HTTP Request")]
    pub data_raw: Option<String>,

    /// Like -d with @file, but CR/LF are NOT stripped from file content.
    #[arg(long = "data-binary", value_name = "DATA", help_heading = "HTTP Request")]
    pub data_binary: Option<String>,

    /// URL-encode data. Sub-forms: content | =content | name=content | @file | name@file.
    /// Repeatable; values concatenated with &.
    #[arg(long = "data-urlencode", value_name = "DATA", action = clap::ArgAction::Append, help_heading = "HTTP Request")]
    pub data_urlencode: Vec<String>,

    /// Multipart form field. curl-compatible grammar:
    /// `name=value` (literal), `name=@file` (file contents), `name=@file;type=mime;filename=basename`,
    /// `name=<file` (content from file, keep original name), `name=<-` (content from stdin).
    /// Repeatable; each -F adds a new part. Sets Content-Type: multipart/form-data.
    #[arg(short = 'F', long = "form", value_name = "NAME=VALUE", action = clap::ArgAction::Append, help_heading = "HTTP Request")]
    pub form: Vec<String>,

    /// Like -F but treats VALUE strictly as a literal string (`@` and
    /// `<` are NOT interpreted). Use when the literal content starts
    /// with those characters.
    #[arg(long = "form-string", value_name = "NAME=VALUE", action = clap::ArgAction::Append, help_heading = "HTTP Request")]
    pub form_string: Vec<String>,

    /// Backslash-escape special characters (", \, \r, \n) in form field
    /// names + filenames. Matches curl's --form-escape. Off by default.
    #[arg(long = "form-escape", help_heading = "HTTP Request")]
    pub form_escape: bool,

    /// Upload the given local file as the request body. Defaults method to
    /// PUT unless -X is set explicitly. Pass `-` to read from stdin.
    /// Mutually exclusive with -d/--data.
    #[arg(short = 'T', long = "upload-file", value_name = "PATH", help_heading = "HTTP Request")]
    pub upload_file: Option<std::path::PathBuf>,

    /// Append mode for FTP / SFTP uploads. Maps to FTP's APPE command
    /// and SFTP's O_APPEND. Has no effect on HTTP uploads (that's a
    /// server-side concept).
    #[arg(short = 'a', long = "append", help_heading = "HTTP Request")]
    pub append: bool,

    /// Convert bare LF bytes to CRLF before sending the request body
    /// (or upload). Useful for line-oriented protocols that reject
    /// unix-style newlines.
    #[arg(long = "crlf", help_heading = "HTTP Request")]
    pub crlf: bool,

    /// Retry N times on transient failures (5xx, DNS, connect reset,
    /// timeouts). Default 0 = no retries. Use --retry-all-errors to
    /// also retry on 4xx.
    #[arg(long = "retry", value_name = "N", default_value_t = 0u32, help_heading = "HTTP Request")]
    pub retry: u32,

    /// When --retry is active, also retry on non-transient errors
    /// (4xx, parser errors). Default: transient-only.
    #[arg(long = "retry-all-errors", help_heading = "HTTP Request")]
    pub retry_all_errors: bool,

    /// Retry on ECONNREFUSED specifically (some servers take a while
    /// to come up after startup; useful for liveness-wait loops).
    #[arg(long = "retry-connrefused", help_heading = "HTTP Request")]
    pub retry_connrefused: bool,

    /// Fixed seconds between retries. When unset, recon applies an
    /// exponential backoff: 1s, 2s, 4s, 8s, 16s, 32s (capped).
    #[arg(long = "retry-delay", value_name = "SECS", help_heading = "HTTP Request")]
    pub retry_delay: Option<u64>,

    /// Total time budget across all retries (seconds). Abort once
    /// exceeded, regardless of --retry N.
    #[arg(long = "retry-max-time", value_name = "SECS", help_heading = "HTTP Request")]
    pub retry_max_time: Option<u64>,

    /// Request rate cap. Format: `N/s` / `N/m` / `N/h` (integer N).
    /// Engages when running batches via --input-file: at most N
    /// requests per second/minute/hour.
    #[arg(long = "rate", value_name = "N/s|N/m|N/h", help_heading = "HTTP Request")]
    pub rate: Option<String>,

    /// Curl-compatible protocol allow-list. Syntax: `=proto` means
    /// "only this set", `+proto` allow (default), `-proto` deny.
    /// Example: `=https` (HTTPS only), `+https,-ftp` (add HTTPS,
    /// remove FTP from the defaults).
    #[arg(long = "proto", value_name = "LIST", help_heading = "HTTP Request")]
    pub proto: Option<String>,

    /// Default scheme for URLs that don't carry one. Example:
    /// `--proto-default https` will treat `example.com/foo` as
    /// `https://example.com/foo`.
    #[arg(long = "proto-default", value_name = "SCHEME", help_heading = "HTTP Request")]
    pub proto_default: Option<String>,

    /// Apply --proto's filter to redirect targets too (so an -L
    /// response that redirects to a disallowed scheme is refused).
    #[arg(long = "proto-redir", value_name = "LIST", help_heading = "HTTP Request")]
    pub proto_redir: Option<String>,

    /// Batch-fetch URLs listed in FILE (one per line, `#` comments,
    /// blank lines ignored, `-` reads the list from stdin). Each
    /// URL is processed independently; errors are reported per URL.
    #[arg(long = "input-file", value_name = "FILE", help_heading = "HTTP Request")]
    pub input_file: Option<String>,

    /// Resume an interrupted download. wget-compatible: reads the
    /// current size of the -o target (or basename from the URL) and
    /// sets `Range: bytes=<size>-`. Equivalent to `--continue-at -`.
    #[arg(long = "continue", help_heading = "HTTP Request")]
    pub continue_auto: bool,

    /// Resume from BYTE offset (curl-compatible). Pass `-` to auto-
    /// detect from the local file size (same behaviour as --continue).
    #[arg(short = 'C', long = "continue-at", value_name = "OFFSET", help_heading = "HTTP Request")]
    pub continue_at: Option<String>,

    // ── FTP-specific ─────────────────────────────────────────────────────────

    /// Inhibit EPSV; force PASV on FTP connections. suppaftp defaults
    /// to PASV; this flag makes the intent explicit.
    #[arg(long = "disable-epsv", help_heading = "File Transfer")]
    pub disable_epsv: bool,

    /// Inhibit EPRT / LPRT (the IPv6-capable active-mode commands).
    /// Accepted for curl parity; suppaftp uses passive by default.
    #[arg(long = "disable-eprt", help_heading = "File Transfer")]
    pub disable_eprt: bool,

    /// Force PASV mode (the current default). Accepted for curl parity.
    #[arg(long = "ftp-pasv", help_heading = "File Transfer")]
    pub ftp_pasv: bool,

    /// FTP CWD strategy: `multicwd` (one CWD per path component),
    /// `nocwd` (send full path each time), `singlecwd` (one CWD to
    /// the final dir). Accepted for curl parity; suppaftp uses
    /// `multicwd` semantics today.
    #[arg(long = "ftp-method", value_name = "MODE", help_heading = "File Transfer")]
    pub ftp_method: Option<String>,

    /// Create missing remote directories during FTP upload. Accepted
    /// for curl parity; FTP upload itself is a separate feature
    /// (defer pending suppaftp upload support).
    #[arg(long = "ftp-create-dirs", help_heading = "File Transfer")]
    pub ftp_create_dirs: bool,

    /// Send a raw FTP command before each transfer. Repeatable.
    /// Accepted for curl parity; wired through to suppaftp's
    /// `custom_command` in a follow-up.
    #[arg(short = 'Q', long = "quote", value_name = "CMD", action = clap::ArgAction::Append, help_heading = "File Transfer")]
    pub quote: Vec<String>,

    /// Ignore the address returned by PASV; reuse the control
    /// connection's IP. Defence against NAT-borked responses.
    /// Accepted for curl parity.
    #[arg(long = "ftp-skip-pasv-ip", help_heading = "File Transfer")]
    pub ftp_skip_pasv_ip: bool,

    /// List only (NLST). Returns names without the full directory
    /// listing. Applies to FTP URLs.
    #[arg(short = 'l', long = "list-only", help_heading = "File Transfer")]
    pub list_only: bool,

    /// TFTP: skip option-negotiation extensions. Accepted; tftp
    /// module already defaults to vanilla RFC 1350.
    #[arg(long = "tftp-no-options", help_heading = "File Transfer")]
    pub tftp_no_options: bool,

    // ── SMTP-specific ────────────────────────────────────────────────────────

    /// SMTP AUTH address in `MAIL FROM` (distinct from the sender).
    /// Matches curl's `--mail-auth`.
    #[arg(long = "mail-auth", value_name = "ADDR", help_heading = "SMTP")]
    pub mail_auth: Option<String>,

    /// Allow RCPT TO to fail for some recipients without aborting
    /// the whole send. Accepted for curl parity; lettre's SMTP
    /// transport treats any RCPT failure as fatal today.
    #[arg(long = "mail-rcpt-allowfails", help_heading = "SMTP")]
    pub mail_rcpt_allowfails: bool,

    /// SASL initial response: send the AUTH payload on the same
    /// line as the AUTH verb. Accepted for curl parity; lettre
    /// handles this internally.
    #[arg(long = "sasl-ir", help_heading = "SMTP")]
    pub sasl_ir: bool,

    // ── IMAP / POP3 specific ─────────────────────────────────────────────────

    /// SASL `AUTHZID` (authorization identity; lets one account
    /// auth as another). Accepted; imap/pop3 wiring deferred.
    #[arg(long = "sasl-authzid", value_name = "ID", help_heading = "Mail Retrieval")]
    pub sasl_authzid: Option<String>,

    /// Login options string appended to the AUTH exchange. Matches
    /// curl's `--login-options`. Accepted; wiring deferred.
    #[arg(long = "login-options", value_name = "STR", help_heading = "Mail Retrieval")]
    pub login_options: Option<String>,

    // ── Telnet ───────────────────────────────────────────────────────────────

    /// Set a telnet option (OPT=VAL). Repeatable. Accepted for
    /// curl parity; recon's telnet probe is minimal, so the flag is
    /// declarative today.
    #[arg(long = "telnet-option", value_name = "OPT=VAL", action = clap::ArgAction::Append, help_heading = "Network Tests")]
    pub telnet_option: Vec<String>,

    // ── SSH pinning + compression ────────────────────────────────────────────

    /// Accept only SSH hosts whose public-key SHA-256 matches. Pass
    /// the raw hex digest (64 chars) or base64. Pairs with ssh://,
    /// scp://, sftp:// URLs.
    #[arg(long = "hostpubsha256", value_name = "SHA", help_heading = "Auth & TLS")]
    pub hostpubsha256: Option<String>,

    /// Accept only SSH hosts whose public-key MD5 matches. Legacy
    /// form — prefer --hostpubsha256.
    #[arg(long = "hostpubmd5", value_name = "HEX", help_heading = "Auth & TLS")]
    pub hostpubmd5: Option<String>,

    /// Client SSH public-key file (paired with --privkey). Matches
    /// curl's `--pubkey`.
    #[arg(long = "pubkey", value_name = "PATH", help_heading = "Auth & TLS")]
    pub pubkey: Option<PathBuf>,

    /// Enable SSH transport compression (ssh2 zlib).
    #[arg(long = "compressed-ssh", help_heading = "Auth & TLS")]
    pub compressed_ssh: bool,

    /// Follow redirects
    #[arg(short = 'L', long = "location", help_heading = "HTTP Request")]
    pub follow_redirects: bool,

    /// Request a byte range via the Range header. Accepts curl's
    /// syntax: `0-1023` (first 1 KiB), `2048-` (offset to EOF),
    /// `-512` (last 512 bytes). Multiple ranges comma-separated.
    #[arg(short = 'r', long = "range", value_name = "RANGE", help_heading = "HTTP Request")]
    pub range: Option<String>,

    /// Cap download size (bytes). Aborts when Content-Length exceeds
    /// the limit, or when downloaded bytes cross it. Suffixes K/M/G
    /// accepted (1K=1024, 1M=1024*1024, 1G=1024*1024*1024).
    #[arg(long = "max-filesize", value_name = "BYTES", help_heading = "HTTP Request")]
    pub max_filesize: Option<String>,

    /// Append URL-encoded query parameters to the target URL. Same
    /// sub-forms as --data-urlencode: `name=value`, `content`,
    /// `=content`, `@file`, `name@file`. Repeatable.
    #[arg(long = "url-query", value_name = "DATA", action = clap::ArgAction::Append, help_heading = "HTTP Request")]
    pub url_query: Vec<String>,

    /// Override the request-target in the first request line (the
    /// `PATH HTTP/1.1` part). Rarely needed; useful for OPTIONS `*`
    /// or custom gateways.
    #[arg(long = "request-target", value_name = "PATH", help_heading = "HTTP Request")]
    pub request_target: Option<String>,

    /// Reject URLs with a `user:pass@` prefix. Security hardening —
    /// stops accidental credential leaks in command-line history.
    #[arg(long = "disallow-username-in-url", help_heading = "HTTP Request")]
    pub disallow_username_in_url: bool,

    /// Conditional request — `If-Modified-Since` from a date string
    /// (RFC 2822 / RFC 850 / asctime) OR from a local file's mtime.
    /// Prefix with `-` to invert into `If-Unmodified-Since`.
    #[arg(short = 'z', long = "time-cond", value_name = "TIME|FILE", help_heading = "HTTP Request")]
    pub time_cond: Option<String>,

    /// Read an ETag from FILE and send it as `If-None-Match`.
    #[arg(long = "etag-compare", value_name = "FILE", help_heading = "HTTP Request")]
    pub etag_compare: Option<PathBuf>,

    /// Save the response's `ETag` header to FILE for a future
    /// --etag-compare round-trip.
    #[arg(long = "etag-save", value_name = "FILE", help_heading = "HTTP Request")]
    pub etag_save: Option<PathBuf>,

    /// wget-style shortcut: use the mtime of the -o target as
    /// `If-Modified-Since`. Equivalent to `-z <OUTPUT_FILE>`. Requires
    /// `-o PATH` or `-O`.
    #[arg(long = "timestamping", help_heading = "HTTP Request")]
    pub timestamping: bool,

    /// Send a Referer header. Accepts --referrer as an alias for the common
    /// misspelling. An explicit -H "Referer: …" overrides this.
    #[arg(short = 'e', long = "referer", alias = "referrer", value_name = "URL", help_heading = "HTTP Request")]
    pub referer: Option<String>,

    /// Maximum number of redirects to follow
    #[arg(long = "max-redirs", default_value_t = 10, help_heading = "HTTP Request")]
    pub max_redirs: usize,

    /// Custom User-Agent string
    #[arg(short = 'A', long = "user-agent", help_heading = "HTTP Request")]
    pub user_agent: Option<String>,

    /// Connection timeout in seconds
    #[arg(long = "connect-timeout", default_value_t = 30, help_heading = "HTTP Request")]
    pub timeout: u64,

    /// Total operation timeout in seconds (DNS + TLS + request + body).
    /// Accepts fractional seconds. Exit 28 on timeout.
    #[arg(long = "max-time", value_name = "SECONDS", help_heading = "HTTP Request")]
    pub max_time: Option<f64>,

    /// Send -d data as a URL query string with GET instead of as a request body
    #[arg(short = 'G', long = "get", help_heading = "HTTP Request")]
    pub get_data: bool,

    /// Request compressed response (gzip, deflate, br, zstd) and auto-decompress.
    #[arg(long = "compressed", help_heading = "HTTP Request")]
    pub compressed: bool,

    // ── Auth & TLS ───────────────────────────────────────────────────────────

    /// Skip TLS/SSH host key verification (insecure — equivalent to curl -k)
    #[arg(short = 'k', long = "insecure", help_heading = "Auth & TLS")]
    pub insecure: bool,

    /// Force minimum TLS version 1.2. Handshake fails if the server
    /// can't negotiate at least TLS 1.2.
    #[arg(long = "tlsv1.2", help_heading = "Auth & TLS")]
    pub tlsv12: bool,

    /// Force minimum TLS version 1.3. Handshake fails if the server
    /// can't negotiate at least TLS 1.3.
    #[arg(long = "tlsv1.3", help_heading = "Auth & TLS")]
    pub tlsv13: bool,

    /// Path to a PEM-encoded CA certificate to trust in addition to the
    /// system roots. Use for self-signed corporate roots without -k.
    #[arg(long = "cacert", value_name = "PATH", help_heading = "Auth & TLS")]
    pub cacert: Option<PathBuf>,

    /// Add every `*.pem` / `*.crt` file in DIR as a trusted root.
    #[arg(long = "capath", value_name = "DIR", help_heading = "Auth & TLS")]
    pub capath: Option<PathBuf>,

    /// Use the OS-native trust store (WebPKI / Apple / Windows SChannel
    /// roots) rather than the bundled Mozilla roots. Pairs with
    /// --cacert if extra corporate roots are also needed.
    #[arg(long = "ca-native", help_heading = "Auth & TLS")]
    pub ca_native: bool,

    /// Cap the maximum TLS version. Accepts `1.2` or `1.3`. Use to
    /// probe servers that claim 1.3 support but misbehave under 1.3.
    #[arg(long = "tls-max", value_name = "VERSION", help_heading = "Auth & TLS")]
    pub tls_max: Option<String>,

    /// Force HTTP/1.1 only (disable HTTP/2 upgrade). Useful when a
    /// server's /2 path misbehaves. reqwest's http1_only builder.
    #[arg(long = "http1.1", help_heading = "HTTP Request")]
    pub http11: bool,

    /// Prefer HTTP/2 when negotiated via ALPN. Default reqwest
    /// behaviour for https:// — this flag exists for curl parity
    /// and is effectively a no-op. Does NOT force HTTP/2 over
    /// http:// (use --http2-prior-knowledge for that).
    #[arg(long = "http2", help_heading = "HTTP Request")]
    pub http2: bool,

    /// Issue HTTP/2 without the HTTP/1.1 Upgrade handshake. Useful
    /// against h2c (HTTP/2 over cleartext) endpoints or https://
    /// servers that skip ALPN negotiation.
    #[arg(long = "http2-prior-knowledge", help_heading = "HTTP Request")]
    pub http2_prior_knowledge: bool,

    /// Require the file `~/.netrc` (or $NETRC) for credentials. Fail
    /// if missing. Looks up the URL's host to inject Basic auth when
    /// -u / --user isn't set.
    #[arg(short = 'n', long = "netrc", help_heading = "Auth & TLS")]
    pub netrc: bool,

    /// Read credentials from FILE instead of the default ~/.netrc.
    #[arg(long = "netrc-file", value_name = "FILE", help_heading = "Auth & TLS")]
    pub netrc_file: Option<PathBuf>,

    /// Use ~/.netrc if it exists; silently continue if it doesn't.
    /// Curl's `--netrc-optional`.
    #[arg(long = "netrc-optional", help_heading = "Auth & TLS")]
    pub netrc_optional: bool,

    /// TCP_NODELAY on outgoing sockets (disable Nagle's algorithm).
    /// Useful for interactive protocols but hurts bulk-transfer
    /// throughput in some cases.
    #[arg(long = "tcp-nodelay", help_heading = "Auth & TLS")]
    pub tcp_nodelay: bool,

    /// Disable TCP keepalive probes on the connection.
    #[arg(long = "no-keepalive", help_heading = "Auth & TLS")]
    pub no_keepalive: bool,

    /// Interval (seconds) for TCP keepalive probes. Default reqwest
    /// behaviour when the flag is absent.
    #[arg(long = "keepalive-time", value_name = "SECS", help_heading = "Auth & TLS")]
    pub keepalive_time: Option<u64>,

    /// Route connections to HOST:PORT to TARGET:PORT instead. Bypasses
    /// DNS for the specified host+port pair. Repeatable: `--connect-to
    /// api.example.com:443:127.0.0.1:8443`.
    #[arg(long = "connect-to", value_name = "H1:P1:H2:P2", action = clap::ArgAction::Append, help_heading = "Auth & TLS")]
    pub connect_to: Vec<String>,

    /// Shortcut for `-H "Authorization: Bearer TOKEN"`. OAuth 2.0.
    #[arg(long = "oauth2-bearer", value_name = "TOKEN", help_heading = "Auth & TLS")]
    pub oauth2_bearer: Option<String>,

    /// Write the URL (and MIME type) into extended attributes of the
    /// -o output file. macOS + Linux only. Matches curl's `--xattr`.
    #[arg(long = "xattr", help_heading = "Output")]
    pub xattr: bool,

    /// HEAD-only link check (wget-style). Issues a HEAD for the URL
    /// and prints `<STATUS> <URL>`. Exits non-zero if the response is
    /// not 2xx. Skips body transfer entirely.
    #[arg(long = "spider", help_heading = "HTTP Request")]
    pub spider: bool,

    /// Client certificate for mTLS. Path to a PEM file. May include
    /// the private key inline (combined PEM with both CERTIFICATE and
    /// PRIVATE KEY blocks); otherwise pair with --client-key. Use
    /// --cert-type to select non-PEM formats. `-E` is the curl-compatible
    /// short form.
    #[arg(long = "client-cert", short = 'E', value_name = "PATH", help_heading = "Auth & TLS")]
    pub client_cert: Option<PathBuf>,

    /// Private key for the client certificate. PEM-encoded. Only
    /// needed when --client-cert contains only the cert chain. Use
    /// --key-type for non-PEM key formats (DER).
    #[arg(long = "client-key", visible_alias = "key", value_name = "PATH", help_heading = "Auth & TLS")]
    pub client_key: Option<PathBuf>,

    /// Format of --client-cert. `PEM` (default) or `DER`. DER support
    /// is deferred under rustls — pass PEM for now; non-PEM values
    /// error with a clear message.
    #[arg(long = "cert-type", value_name = "PEM|DER", default_value = "PEM", help_heading = "Auth & TLS")]
    pub cert_type: String,

    /// Format of --client-key. `PEM` (default), `DER`, or `ENG`
    /// (OpenSSL engine). Only `PEM` is honored under rustls; `DER`
    /// is deferred and `ENG` has no rustls equivalent.
    #[arg(long = "key-type", value_name = "PEM|DER|ENG", default_value = "PEM", help_heading = "Auth & TLS")]
    pub key_type: String,

    /// Passphrase for an encrypted PKCS#8 private key. Currently
    /// unsupported — if the key is encrypted, decrypt externally
    /// (`openssl pkcs8 -in key.enc -out key.pem`) first.
    #[arg(long = "pass", value_name = "PASS", help_heading = "Auth & TLS")]
    pub cert_pass: Option<String>,

    /// Bind outgoing socket to a specific local address. Accepts an IP
    /// literal (IPv4 or IPv6). Interface names (eth0, en0) are not yet
    /// resolved — pass the address directly for now.
    #[arg(long = "interface", value_name = "IP", help_heading = "Auth & TLS")]
    pub interface: Option<String>,

    /// Throttle downloads to at most RATE bytes per second. Accepts
    /// curl's suffixes: 100K, 2M, 1.5G, or bare bytes.
    #[arg(long = "limit-rate", value_name = "RATE", help_heading = "HTTP Request")]
    pub limit_rate: Option<String>,

    /// Abort if the transfer rate stays below BYTES/sec for
    /// `--speed-time` seconds. Used together; either alone is inert.
    #[arg(long = "speed-limit", value_name = "BYTES", help_heading = "HTTP Request")]
    pub speed_limit: Option<u64>,

    /// Window in seconds for `--speed-limit` (default: 30).
    #[arg(long = "speed-time", value_name = "SECS", default_value_t = 30, help_heading = "HTTP Request")]
    pub speed_time: u64,

    /// Transcode the response body to this charset before prettify
    /// or write (e.g. `--output-charset utf-8`). Detection priority:
    /// explicit `--source-charset` > Content-Type charset > sniff >
    /// windows-1252 fallback. Use `--list-charsets` for supported labels.
    #[arg(long = "output-charset", value_name = "NAME", help_heading = "Text Encoding")]
    pub output_charset: Option<String>,

    /// Override the source charset the server declared (or when none
    /// was declared). Only meaningful together with `--output-charset`.
    #[arg(long = "source-charset", value_name = "NAME", help_heading = "Text Encoding")]
    pub source_charset: Option<String>,

    /// Shorthand for `--output-charset utf-8`. Convenient when talking
    /// to a legacy ISO-8859-1 / Windows-1252 service from a UTF-8 shell.
    #[arg(long = "to-utf8", help_heading = "Text Encoding")]
    pub to_utf8: bool,

    /// Transcode the request body from UTF-8 (the shell's native
    /// encoding) to this charset before sending. Takes priority over any
    /// `charset=` set in an explicit Content-Type header.
    #[arg(long = "request-charset", value_name = "NAME", help_heading = "Text Encoding")]
    pub request_charset: Option<String>,

    /// Skip auto-transcoding the request body even when the request's
    /// `Content-Type` header declares a charset. Use when the body is
    /// already in the target encoding (e.g. read from a pre-encoded file).
    #[arg(long = "request-charset-passthrough", help_heading = "Text Encoding")]
    pub request_charset_passthrough: bool,

    /// Standalone conversion: read input file (or stdin), transcode from
    /// SOURCE to TARGET, write to `-o PATH` (or stdout). Format:
    /// `SOURCE:TARGET` (blank SOURCE means auto-detect). Mutually exclusive
    /// with HTTP invocations. Example: `--iconv iso-8859-1:utf-8 input.txt`.
    #[arg(long = "iconv", value_name = "SOURCE:TARGET", help_heading = "Text Encoding")]
    pub iconv: Option<String>,

    /// List every charset label recon understands and exit.
    #[arg(long = "list-charsets", help_heading = "Text Encoding")]
    pub list_charsets: bool,

    /// Comma-separated list of custom DNS servers to use for name
    /// resolution. Accepts `IP` (port 53 assumed) or `IP:PORT`.
    /// Example: `--dns-servers 1.1.1.1,8.8.8.8:5353`.
    #[arg(long = "dns-servers", value_name = "LIST", help_heading = "DNS")]
    pub dns_servers: Option<String>,

    /// Local IPv4 address to bind outgoing DNS queries to. Use with
    /// `--dns-servers` (an implicit default of 1.1.1.1:53 applies
    /// otherwise).
    #[arg(long = "dns-ipv4-addr", value_name = "IP", help_heading = "DNS")]
    pub dns_ipv4_addr: Option<String>,

    /// Local IPv6 address to bind outgoing DNS queries to.
    #[arg(long = "dns-ipv6-addr", value_name = "IP", help_heading = "DNS")]
    pub dns_ipv6_addr: Option<String>,

    /// Bind DNS queries to a specific named interface (e.g. `eth0`).
    /// Not yet plumbed — recon currently errors out if this is set.
    /// Use `--dns-ipv4-addr` / `--dns-ipv6-addr` with the interface's
    /// literal address as a workaround.
    #[arg(long = "dns-interface", value_name = "IFACE", help_heading = "DNS")]
    pub dns_interface: Option<String>,

    /// HTTP Basic auth or SSH username; format: user or user:pass
    #[arg(short = 'u', long = "user", value_name = "USER:PASS", help_heading = "Auth & TLS")]
    pub user: Option<String>,

    /// Path to SSH private key file for SCP authentication
    #[arg(long = "ssh-key", value_name = "PATH", help_heading = "Auth & TLS")]
    pub ssh_key: Option<PathBuf>,

    /// Path to SSH public key file (optional; derived from --ssh-key if omitted)
    #[arg(long = "ssh-pubkey", value_name = "PATH", help_heading = "Auth & TLS")]
    pub ssh_pubkey: Option<PathBuf>,

    /// Passphrase for the SSH private key, or password for SSH password auth
    #[arg(long = "ssh-pass", value_name = "PASS", help_heading = "Auth & TLS")]
    pub ssh_pass: Option<String>,

    // ── Output ───────────────────────────────────────────────────────────────

    /// Write output to file instead of stdout
    #[arg(short = 'o', long = "output", help_heading = "Output")]
    pub output: Option<PathBuf>,

    /// Save the response body to a file named after the URL's final path
    /// segment (curl -O). Mutually exclusive with -o/--output.
    #[arg(short = 'O', long = "remote-name", help_heading = "Output")]
    pub remote_name: bool,

    /// Create missing parent directories for -o output path.
    #[arg(long = "create-dirs", help_heading = "Output")]
    pub create_dirs: bool,

    /// Prefix for -o / -O output paths (e.g., --output-dir ./dl places files there).
    #[arg(long = "output-dir", value_name = "DIR", help_heading = "Output")]
    pub output_dir: Option<PathBuf>,

    /// Use Content-Disposition filename (RFC 6266) with -O instead of URL basename.
    #[arg(short = 'J', long = "remote-header-name", help_heading = "Output")]
    pub remote_header_name: bool,

    /// Apply response Last-Modified as mtime on saved output file.
    #[arg(long = "remote-time", help_heading = "Output")]
    pub remote_time: bool,

    /// Delete the -o target file on any error (keeps partial output
    /// from lingering on disk after a failed transfer).
    #[arg(long = "remove-on-error", help_heading = "Output")]
    pub remove_on_error: bool,

    /// Refuse to overwrite an existing -o target; exit with a clear
    /// error if the file is already there.
    #[arg(long = "no-clobber", help_heading = "Output")]
    pub no_clobber: bool,

    /// Mode (octal, e.g. 600) applied to files created via -o / -O.
    /// Unix-only; Windows ignores the flag with a warning.
    #[arg(long = "create-file-mode", value_name = "MODE", help_heading = "Output")]
    pub create_file_mode: Option<String>,

    /// Disable stdout buffering (each write flushes). Useful when
    /// piping into another tool that expects bytes as they arrive.
    #[arg(short = 'N', long = "no-buffer", help_heading = "Output")]
    pub no_buffer: bool,

    /// Write the response headers to FILE (one header per line).
    /// Does not affect the body destination.
    #[arg(short = 'D', long = "dump-header", value_name = "FILE", help_heading = "Output")]
    pub dump_header: Option<PathBuf>,

    /// Redirect stderr to FILE. Useful when recon's diagnostic output
    /// would interfere with a script pipeline.
    #[arg(long = "stderr", value_name = "FILE", help_heading = "Output")]
    pub stderr_file: Option<PathBuf>,

    /// Force styled (colored) output on response headers even when
    /// stdout isn't a TTY.
    #[arg(long = "styled-output", help_heading = "Output")]
    pub styled_output: bool,

    /// Disable all styling (overrides --styled-output + auto-detect).
    #[arg(long = "no-styled-output", help_heading = "Output")]
    pub no_styled_output: bool,

    /// Suppress the progress meter. Complements -s/--silent which
    /// suppresses all diagnostic output.
    #[arg(long = "no-progress-meter", help_heading = "Output")]
    pub no_progress_meter: bool,

    /// Show error messages even when -s/--silent is set. Matches
    /// curl's convention of surfacing errors but not diagnostics.
    #[arg(long = "show-error", help_heading = "Output")]
    pub show_error: bool,

    /// Silent mode: suppress progress and informational output
    #[arg(short = 's', long = "silent", help_heading = "Output")]
    pub silent: bool,

    /// Verbose: print request/response headers to stderr (-v); add -vv for timing and auth detail
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, help_heading = "Output")]
    pub verbose: u8,

    /// Include response headers in output
    #[arg(short = 'i', long = "include", help_heading = "Output")]
    pub include_headers: bool,

    /// Fail silently (exit non-zero) on HTTP errors (status >= 400)
    #[arg(short = 'f', long = "fail", help_heading = "Output")]
    pub fail_on_error: bool,

    /// Like -f, but also write the response body to stdout/file on HTTP errors
    #[arg(long = "fail-with-body", help_heading = "Output")]
    pub fail_with_body: bool,

    /// Print only the HTTP status code
    #[arg(short = 'S', long = "status", help_heading = "Output")]
    pub status_only: bool,

    /// Output only the response headers, suppress body
    #[arg(short = 'I', long = "head", help_heading = "Output")]
    pub head_only: bool,

    /// Output status line, all headers, and body
    #[arg(long = "full", help_heading = "Output")]
    pub full: bool,

    /// Print response headers for every hop, following redirects (implies redirect following)
    #[arg(long = "LHEAD", help_heading = "Output")]
    pub lhead: bool,

    /// Prettify response body: auto-detects JSON, XML, HTML, YAML, CSV, TSV
    #[arg(short = 'p', long = "prettify", help_heading = "Output")]
    pub prettify: bool,

    /// Show a progress meter when saving to a file (opt-in, unlike curl)
    #[arg(long = "progress", help_heading = "Output")]
    pub progress: bool,

    /// Print full error details including internal causes
    #[arg(long = "FULL-ERRORS", help_heading = "Output")]
    pub full_errors: bool,

    /// Format string printed after response. Supports %{var}, %{header{name}},
    /// %{json}, \n \t \r \\ escapes, @file / @- loading, %{stderr} / %{stdout}.
    #[arg(short = 'w', long = "write-out", value_name = "FORMAT", help_heading = "Output")]
    pub write_out: Option<String>,

    // ── Certificate Inspection ───────────────────────────────────────────────

    /// Fetch and display the server's TLS certificate without making an HTTP request (HTTPS only)
    #[arg(long = "cert", help_heading = "Certificate Inspection")]
    pub cert: bool,

    // ── DNS ──────────────────────────────────────────────────────────────────

    /// DNS lookup for the host — shows common record types by default
    #[arg(long = "dns", help_heading = "DNS")]
    pub dns: bool,

    /// DNS record type(s) to query, comma-separated (A,AAAA,MX,NS,TXT,SOA,CNAME,PTR,SRV,CAA,…)
    #[arg(long = "dns-type", value_delimiter = ',', help_heading = "DNS")]
    pub dns_type: Vec<String>,

    // ── WHOIS ────────────────────────────────────────────────────────────────

    /// WHOIS lookup for a domain or IP address
    #[arg(long = "whois", help_heading = "WHOIS")]
    pub whois: bool,

    // ── Network Tests ────────────────────────────────────────────────────────

    /// Traceroute to the host (uses port if specified in the address)
    #[arg(long = "traceroute", alias = "trace", help_heading = "Network Tests")]
    pub traceroute: bool,

    /// Maximum number of hops for traceroute
    #[arg(long = "max-hops", default_value_t = 30, help_heading = "Network Tests")]
    pub max_hops: u8,

    /// Ping the host. TCP ping if a port is given (e.g. host:443), ICMP ping otherwise
    #[arg(long = "ping", help_heading = "Network Tests")]
    pub ping: bool,

    /// Number of pings to send
    #[arg(long = "ping-count", default_value_t = 4, help_heading = "Network Tests")]
    pub ping_count: u32,

    /// Check connectivity using probes defined in ~/.recon/config.toml
    #[arg(long = "netstatus", help_heading = "Network Tests")]
    pub netstatus: bool,

    // ── Email Protection ─────────────────────────────────────────────────────

    /// Validate the SPF record (recursive include/redirect resolution, lookup limits)
    #[arg(long = "spf", help_heading = "Email Protection")]
    pub spf: bool,

    /// Validate the DMARC record and policy
    #[arg(long = "dmarc", help_heading = "Email Protection")]
    pub dmarc: bool,

    /// Validate the DKIM record for the given selector (repeatable: --dkim sel1 --dkim sel2)
    #[arg(long = "dkim", value_name = "SELECTOR", action = clap::ArgAction::Append, help_heading = "Email Protection")]
    pub dkim: Vec<String>,

    /// Validate MTA-STS DNS record and HTTPS policy
    #[arg(long = "mta-sts", help_heading = "Email Protection")]
    pub mta_sts: bool,

    /// Validate the BIMI record (default selector: "default")
    #[arg(long = "bimi", value_name = "SELECTOR", num_args = 0..=1, default_missing_value = "default", help_heading = "Email Protection")]
    pub bimi: Option<String>,

    /// Validate the TLS-RPT reporting record
    #[arg(long = "tls-rpt", help_heading = "Email Protection")]
    pub tls_rpt: bool,

    // ── Cookies ──────────────────────────────────────────────────────────────

    /// Cookie jar to use for this request (name or path to a .db file).
    /// Omit the value to use the default jar.
    #[arg(long = "cookiejar", value_name = "NAME", num_args = 0..=1, default_missing_value = "default", help_heading = "Cookies")]
    pub cookiejar: Option<String>,

    /// List all cookies in the jar (requires --cookiejar)
    #[arg(long = "cookies", help_heading = "Cookies")]
    pub cookies: bool,

    /// Delete the cookie with the given ID (requires --cookiejar)
    #[arg(long = "cookie-delete", value_name = "ID", help_heading = "Cookies")]
    pub cookie_delete: Option<i64>,

    /// Add or update a cookie (requires --cookiejar)
    /// Format: "name=value; Domain=example.com; [Path=/]; [Secure]; [HttpOnly]; [Max-Age=N]"
    #[arg(long = "cookie-set", value_name = "COOKIE", help_heading = "Cookies")]
    pub cookie_set: Option<String>,

    // ── File Server ──────────────────────────────────────────────────────────

    /// Start an HTTP file server on the given port (default: 80)
    #[arg(long = "serve", value_name = "PORT", num_args = 0..=1, default_missing_value = "80", help_heading = "File Server")]
    pub serve: Option<String>,

    /// Start an HTTPS file server on the given port (default: 443)
    #[arg(long = "serve-tls", value_name = "PORT", num_args = 0..=1, default_missing_value = "443", help_heading = "File Server")]
    pub serve_tls: Option<String>,

    /// Force HTTP version for the server: 1.1 or 2 (default: auto-negotiate)
    #[arg(long = "http-version", value_name = "VERSION", help_heading = "File Server")]
    pub http_version: Option<String>,

    /// Path to TLS certificate PEM file (default: ~/.recon/cert.pem)
    #[arg(long = "serve-cert", value_name = "PATH", help_heading = "File Server")]
    pub serve_cert: Option<std::path::PathBuf>,

    /// Path to TLS private key PEM file (default: ~/.recon/key.pem)
    #[arg(long = "serve-key", value_name = "PATH", help_heading = "File Server")]
    pub serve_key: Option<std::path::PathBuf>,

    /// Write access log to this file (in addition to terminal output)
    #[arg(long = "serve-log", value_name = "PATH", help_heading = "File Server")]
    pub serve_log: Option<std::path::PathBuf>,

    /// SNI hostname-to-certificate mapping (repeatable: inline host:cert:key, directory, or config file)
    /// Omit the value to use the default directory: ~/.recon/sni/
    #[arg(long = "serve-sni", value_name = "MAPPING", num_args = 0..=1, default_missing_value = "~/.recon/sni/", action = clap::ArgAction::Append, help_heading = "File Server")]
    pub serve_sni: Vec<String>,

    // ── JWT ──────────────────────────────────────────────────────────────────

    /// Decode and display JWT header and payload without verification
    #[arg(long = "jwt-view", help_heading = "JWT")]
    pub jwt_view: bool,

    /// Sign or complete a JWT token
    #[arg(long = "jwt-sign", help_heading = "JWT")]
    pub jwt_sign: bool,

    /// Verify JWT signature and opt-in claim checks
    #[arg(long = "jwt-validate", help_heading = "JWT")]
    pub jwt_validate: bool,

    /// HMAC secret for signing or validating (required for --jwt-sign and --jwt-validate)
    #[arg(long = "jwt-secret", value_name = "SECRET", help_heading = "JWT")]
    pub jwt_secret: Option<String>,

    /// Algorithm: HS256 (default), HS384, HS512
    #[arg(long = "jwt-alg", alias = "jwt-algorithm", value_name = "ALG", help_heading = "JWT")]
    pub jwt_alg: Option<String>,

    /// JWT issuer claim — set when signing (if absent); assert value when validating with --jwt-validate-iss
    #[arg(long = "jwt-iss", value_name = "VALUE", help_heading = "JWT")]
    pub jwt_iss: Option<String>,

    /// JWT subject claim
    #[arg(long = "jwt-sub", value_name = "VALUE", help_heading = "JWT")]
    pub jwt_sub: Option<String>,

    /// JWT audience claim
    #[arg(long = "jwt-aud", value_name = "VALUE", help_heading = "JWT")]
    pub jwt_aud: Option<String>,

    /// JWT expiry (Unix timestamp). Omit value to use current time.
    #[arg(long = "jwt-exp", value_name = "TIMESTAMP", num_args = 0..=1, default_missing_value = "now", help_heading = "JWT")]
    pub jwt_exp: Option<String>,

    /// JWT not-before (Unix timestamp). Omit value to use current time.
    #[arg(long = "jwt-nbf", value_name = "TIMESTAMP", num_args = 0..=1, default_missing_value = "now", help_heading = "JWT")]
    pub jwt_nbf: Option<String>,

    /// JWT issued-at (Unix timestamp). Omit value to use current time.
    #[arg(long = "jwt-iat", value_name = "TIMESTAMP", num_args = 0..=1, default_missing_value = "now", help_heading = "JWT")]
    pub jwt_iat: Option<String>,

    /// JWT ID claim
    #[arg(long = "jwt-jti", value_name = "VALUE", help_heading = "JWT")]
    pub jwt_jti: Option<String>,

    /// Validate the exp claim (must not be expired)
    #[arg(long = "jwt-validate-exp", help_heading = "JWT")]
    pub jwt_validate_exp: bool,

    /// Validate the nbf claim (must not be used before valid)
    #[arg(long = "jwt-validate-nbf", help_heading = "JWT")]
    pub jwt_validate_nbf: bool,

    /// Validate the iat claim (must not be in the future)
    #[arg(long = "jwt-validate-iat", help_heading = "JWT")]
    pub jwt_validate_iat: bool,

    /// Validate iss matches --jwt-iss
    #[arg(long = "jwt-validate-iss", help_heading = "JWT")]
    pub jwt_validate_iss: bool,

    /// Validate sub matches --jwt-sub
    #[arg(long = "jwt-validate-sub", help_heading = "JWT")]
    pub jwt_validate_sub: bool,

    /// Validate aud matches --jwt-aud
    #[arg(long = "jwt-validate-aud", help_heading = "JWT")]
    pub jwt_validate_aud: bool,

    /// Validate jti matches --jwt-jti
    #[arg(long = "jwt-validate-jti", help_heading = "JWT")]
    pub jwt_validate_jti: bool,

    /// Enable all claim validation checks
    #[arg(long = "jwt-validate-full", help_heading = "JWT")]
    pub jwt_validate_full: bool,

    /// Output JWT results as a single JSON object instead of labeled sections
    #[arg(long = "jwt-json-report", help_heading = "JWT")]
    pub jwt_json_report: bool,

    // ── Hashing ──────────────────────────────────────────────────────────────

    /// Compute a cryptographic hash of the input source. Algorithm name is
    /// case-insensitive; hyphens and underscores are accepted. Supported:
    /// md5, sha1, sha256, sha384, sha512, sha3-256, sha3-512, blake3.
    #[arg(long = "hash", value_name = "ALGO", help_heading = "Hashing")]
    pub hash: Option<String>,

    /// Output format for --hash digest: hex (default), base64, or raw.
    #[arg(long = "hash-format", value_name = "FMT", help_heading = "Hashing")]
    pub hash_format: Option<String>,

    /// List all supported hash algorithms and exit (standalone action).
    #[arg(long = "hash-list", help_heading = "Hashing")]
    pub hash_list: bool,

    // ── Compression ──────────────────────────────────────────────────────────

    /// Compress the input source with the named algorithm. Value is an
    /// algorithm name (case-insensitive; canonical or alias): gzip, gz,
    /// deflate, zstd, zst, brotli, br, bzip2, bz2.
    #[arg(long = "compress", value_name = "ALGO", help_heading = "Compression")]
    pub compress: Option<String>,

    /// Decompress the input source. Omit ALGO to auto-detect from magic
    /// bytes (gzip, zstd, bzip2). Deflate and brotli have no magic bytes
    /// — pass the algorithm explicitly for those.
    #[arg(long = "decompress", value_name = "ALGO", num_args = 0..=1, default_missing_value = "", help_heading = "Compression")]
    pub decompress: Option<String>,

    /// Compression level for --compress. Accepts a number in the algorithm's
    /// native range (e.g. gzip 0-9, zstd 1-22), or one of:
    /// fastest, fast, default, good, best. Invalid with --decompress.
    #[arg(long = "compression-level", value_name = "LEVEL", help_heading = "Compression")]
    pub compression_level: Option<String>,

    /// List supported compression algorithms and exit (standalone action).
    #[arg(long = "compress-list", help_heading = "Compression")]
    pub compress_list: bool,

    // ── Encoding ─────────────────────────────────────────────────────────────

    /// Encode the positional text as a QR / DataMatrix / barcode.
    /// Supported formats: qr, datamatrix, code128, code39, ean13, upca.
    #[arg(long = "encode", value_name = "FORMAT", help_heading = "Encoding")]
    pub encode: Option<String>,

    /// Output format for --encode: ascii, svg, or png. When omitted, inferred
    /// from -o <FILE> extension (.svg / .png); defaults to ASCII otherwise.
    #[arg(long = "encode-format", value_name = "FMT", help_heading = "Encoding")]
    pub encode_format: Option<String>,

    /// Read --encode input from a file. Mutually exclusive with a positional text.
    #[arg(long = "from-file", value_name = "PATH", help_heading = "Encoding")]
    pub from_file: Option<std::path::PathBuf>,

    /// List all supported encode formats and exit (standalone action).
    #[arg(long = "encode-list", help_heading = "Encoding")]
    pub encode_list: bool,

    /// QR error-correction level. `L` = ~7% recoverable (smaller matrix);
    /// `M` = ~15% (default); `Q` = ~25%; `H` = ~30% (largest, best for
    /// codes that will be scratched, laminated, or overlaid). Only
    /// meaningful when --encode qr is active.
    #[arg(long = "qr-level", value_name = "L|M|Q|H", default_value = "M", help_heading = "Encoding")]
    pub qr_level: String,

    /// Show human-readable text (HRT) under 1D barcodes. Default on for
    /// EAN-13 / UPC-A; off for Code128 / Code39 (where the text is often
    /// arbitrary and ugly in the HRT row). Implemented for ASCII and
    /// SVG output — PNG HRT is deferred pending font bundling.
    #[arg(long = "hrt", help_heading = "Encoding")]
    pub hrt: bool,

    /// Explicitly disable HRT. Overrides the default-on behaviour for
    /// EAN / UPC codes.
    #[arg(long = "no-hrt", help_heading = "Encoding")]
    pub no_hrt: bool,

    /// Decode a barcode / QR / DataMatrix / Aztec / PDF417 / MaxiCode
    /// from an image file. Accepts PNG / JPEG / WebP / GIF / BMP. Use
    /// `-` to read the image from stdin. Output: `<FORMAT>\t<TEXT>`
    /// (or JSON when --json is set).
    #[arg(long = "decode", value_name = "IMAGE", help_heading = "Encoding")]
    pub decode: Option<String>,

    /// Comma-separated format restriction for --decode. Speeds up
    /// scanning and disambiguates codes that share prefixes. Values:
    /// qr, datamatrix, aztec, pdf417, maxicode, code128, code39,
    /// code93, codabar, ean13, ean8, itf, upca, upce, rss14.
    #[arg(long = "decode-hints", value_name = "LIST", help_heading = "Encoding")]
    pub decode_hints: Option<String>,

    /// Scan an image for ALL barcodes (not just the first). One line
    /// per detection: `<FORMAT>\t<TEXT>`. Accepts a path or `-` for
    /// stdin. Exits non-zero when no barcodes are detected.
    #[arg(long = "decode-all", value_name = "IMAGE", help_heading = "Encoding")]
    pub decode_all: Option<String>,

    // ── Encryption ───────────────────────────────────────────────────────────

    /// Encrypt the input source (age format). Requires at least one --recipient
    /// or a passphrase source.
    #[arg(long = "encrypt", help_heading = "Encryption")]
    pub encrypt: bool,

    /// Decrypt the input source (age format; armored or binary auto-detected).
    #[arg(long = "decrypt", help_heading = "Encryption")]
    pub decrypt: bool,

    /// Read passphrase from a file (trims one trailing newline). Beats
    /// $RECON_PASSPHRASE; both beat the interactive prompt.
    #[arg(long = "passphrase-file", value_name = "PATH", help_heading = "Encryption")]
    pub passphrase_file: Option<std::path::PathBuf>,

    /// Encrypt to an age X25519 recipient. Value is either a literal public
    /// key (age1...) or a path to a file containing one or more. Repeatable.
    #[arg(long = "recipient", value_name = "KEY_OR_PATH", action = clap::ArgAction::Append, help_heading = "Encryption")]
    pub recipient: Vec<String>,

    /// Decrypt with an age private-key file. File may contain one or more keys.
    /// Repeatable.
    #[arg(long = "identity", value_name = "PATH", action = clap::ArgAction::Append, help_heading = "Encryption")]
    pub identity: Vec<std::path::PathBuf>,

    /// Produce ASCII-armored output (--encrypt only). Decrypt auto-detects.
    #[arg(long = "armor", help_heading = "Encryption")]
    pub armor: bool,

    /// Generate a fresh X25519 key pair (age-compatible) and print it (standalone action).
    #[arg(long = "encrypt-keygen", help_heading = "Encryption")]
    pub encrypt_keygen: bool,

    /// Force the PGP / GPG backend instead of age. Recipients are
    /// passed to `gpg --recipient` (needs a local `gpg` binary).
    #[arg(long = "pgp", help_heading = "Encryption")]
    pub pgp: bool,

    /// Force the age backend, even if recipients look PGP-shaped.
    /// Without --age or --pgp, recon auto-detects per-recipient:
    /// `age1…` is age, anything else is PGP.
    #[arg(long = "age", help_heading = "Encryption")]
    pub age: bool,

    /// Rotate keys: decrypt with --identity and re-encrypt to
    /// --recipient (and/or --passphrase-file). Reads the existing
    /// ciphertext from the positional source (file / stdin / URL) and
    /// writes the rotated ciphertext to -o. Works for age and PGP.
    #[arg(long = "rekey", help_heading = "Encryption")]
    pub rekey: bool,

    // ── Check Digits ─────────────────────────────────────────────────────────

    /// Verify a check digit. Value is the algorithm keyword (luhn, visa, iban, …).
    /// Use --checkdigit-list to see all supported keywords.
    #[arg(long = "checkdigit", value_name = "NAME", help_heading = "Check Digits")]
    pub checkdigit: Option<String>,

    /// Compute and append/insert a check digit. Value is the algorithm keyword.
    #[arg(long = "checkdigit-create", value_name = "NAME", help_heading = "Check Digits")]
    pub checkdigit_create: Option<String>,

    /// List all supported check-digit algorithms and exit (standalone action).
    #[arg(long = "checkdigit-list", help_heading = "Check Digits")]
    pub checkdigit_list: bool,

    /// Print raw output without standard grouping/hyphens (applies to --checkdigit and --checkdigit-create).
    #[arg(long = "raw", help_heading = "Check Digits")]
    pub raw: bool,

    // ── Sample Data ──────────────────────────────────────────────────────────

    /// Fetch sample data by name. Colon shortcut supported: NAME[:FORMAT[:COUNT]].
    /// Examples: customer, customer:csv, customer:csv:25, lorem:txt:3p
    #[arg(long = "sample", value_name = "NAME[:FORMAT[:COUNT]]", help_heading = "Sample Data")]
    pub sample: Option<String>,

    /// Override the format portion of --sample (takes precedence over colon shortcut).
    #[arg(long = "sample-format", value_name = "FMT", help_heading = "Sample Data")]
    pub sample_format: Option<String>,

    /// Override the count portion of --sample (takes precedence over colon shortcut).
    /// Accepts N or N{p|w|c} (unit suffixes only valid for lorem).
    #[arg(long = "sample-count", value_name = "COUNT", help_heading = "Sample Data")]
    pub sample_count: Option<String>,

    /// Write sample output to file(s). Default: sample-{{name}}.{{format}} (bulk)
    /// or sample-{{name}}-{{n}}.{{format}} (per_item). Required for per_item with count > 1.
    #[arg(long = "sample-file", value_name = "PATH", num_args = 0..=1, default_missing_value = "", help_heading = "Sample Data")]
    pub sample_file: Option<String>,

    /// List all available samples (built-in plus user-configured) and exit.
    #[arg(long = "sample-list", help_heading = "Sample Data")]
    pub sample_list: bool,

    /// Seed for lorem ipsum randomization. When omitted, a seed is derived
    /// from the current system time. Only valid with the lorem sample —
    /// using this flag with any other sample is an error.
    #[arg(long = "sample-seed", value_name = "N", help_heading = "Sample Data")]
    pub sample_seed: Option<u64>,

    // ── Editor ───────────────────────────────────────────────────────────────

    /// Open the response output in an editor (e.g. `zed`, `code`, `vim`).
    /// Built-in aliases: zed, code, cursor, subl, vim, nvim, nano, emacs.
    /// Accepts a user alias from [editor.aliases] or a raw shell command.
    /// Omit the value to use `[editor] default` from ~/.recon/config.toml.
    #[arg(long = "editor", value_name = "EDITOR", num_args = 0..=1, default_missing_value = "", help_heading = "Editor")]
    pub editor: Option<String>,

    /// Remove all temp files written by previous --editor invocations (/tmp/recon-*)
    #[arg(long = "editor-cleanup", help_heading = "Editor")]
    pub editor_cleanup: bool,

    // ── Protocol Probes ─────────────────────────────────────────────────────

    /// For udp:// — seconds to wait for a response datagram after sending.
    /// Accepts fractional values. Default: 1.0.
    #[arg(long = "wait-time", value_name = "SECS", default_value_t = 1.0, help_heading = "Protocol Probes")]
    pub wait_time: f64,

    // ── MQTT ─────────────────────────────────────────────────────────────────

    /// Subscribe to an MQTT topic filter. Repeatable.
    #[arg(long = "subscribe", value_name = "FILTER", action = clap::ArgAction::Append, help_heading = "MQTT")]
    pub subscribe: Vec<String>,

    /// MQTT protocol version: 3 (MQTT 3.1.1) or 5 (MQTT 5.0). Default: 5.
    #[arg(long = "mqtt-version", value_name = "N", default_value = "5", help_heading = "MQTT")]
    pub mqtt_version: String,

    /// MQTT client identifier. Default: recon-<random>.
    #[arg(long = "client-id", value_name = "ID", help_heading = "MQTT")]
    pub client_id: Option<String>,

    /// MQTT keepalive interval in seconds (default 60).
    #[arg(long = "keepalive", value_name = "SECS", default_value_t = 60, help_heading = "MQTT")]
    pub keepalive: u16,

    /// MQTT QoS level for publish/subscribe (0, 1, or 2). Default: 0.
    #[arg(long = "qos", value_name = "N", default_value_t = 0, help_heading = "MQTT")]
    pub qos: u8,

    /// Set the MQTT PUBLISH retain flag.
    #[arg(long = "retain", help_heading = "MQTT")]
    pub retain: bool,

    /// Exit after receiving N messages in subscribe mode.
    #[arg(long = "count", value_name = "N", help_heading = "MQTT")]
    pub count: Option<u32>,

    /// Emit structured JSON output for MQTT probe (single object) or
    /// subscribe (NDJSON, one object per line).
    #[arg(long = "mqtt-json", help_heading = "MQTT")]
    pub mqtt_json: bool,

    /// MQTT 5 user-property (repeatable). Format: `KEY=VAL`. Applied to
    /// PUBLISH + SUBSCRIBE packets. Ignored on --mqtt-version 3.
    #[arg(long = "user-property", value_name = "KEY=VAL", action = clap::ArgAction::Append, help_heading = "MQTT")]
    pub user_property: Vec<String>,

    /// MQTT 5 last-will topic (publish this on unexpected disconnect).
    #[arg(long = "will-topic", value_name = "TOPIC", help_heading = "MQTT")]
    pub will_topic: Option<String>,

    /// MQTT 5 last-will payload. Accepts @file / @- like -d.
    #[arg(long = "will-payload", value_name = "PAYLOAD", help_heading = "MQTT")]
    pub will_payload: Option<String>,

    /// MQTT 5 last-will QoS (0, 1, or 2). Default 0.
    #[arg(long = "will-qos", value_name = "Q", default_value_t = 0, help_heading = "MQTT")]
    pub will_qos: u8,

    /// MQTT 5 last-will retain flag.
    #[arg(long = "will-retain", help_heading = "MQTT")]
    pub will_retain: bool,

    /// MQTT 5 session-expiry-interval in seconds.
    #[arg(long = "session-expiry", value_name = "SECS", help_heading = "MQTT")]
    pub session_expiry: Option<u32>,

    /// MQTT 5 clean-start flag. Default true; set `--clean-start=false`
    /// to resume a persistent session.
    #[arg(long = "clean-start", value_name = "BOOL", default_value_t = true, num_args = 0..=1, default_missing_value = "true", help_heading = "MQTT")]
    pub clean_start: bool,

    /// MQTT 5 publish content-type property (e.g. application/json).
    #[arg(long = "content-type", value_name = "MIME", help_heading = "MQTT")]
    pub content_type: Option<String>,

    /// MQTT 5 publish response-topic property (for request/response).
    #[arg(long = "response-topic", value_name = "TOPIC", help_heading = "MQTT")]
    pub response_topic: Option<String>,

    /// MQTT 5 publish correlation-data property. Accepts @file / @- or
    /// raw bytes.
    #[arg(long = "correlation-data", value_name = "DATA", help_heading = "MQTT")]
    pub correlation_data: Option<String>,

    /// MQTT 5 enhanced-auth method name.
    #[arg(long = "auth-method", value_name = "NAME", help_heading = "MQTT")]
    pub auth_method: Option<String>,

    /// MQTT 5 enhanced-auth data blob. Accepts @file / @- or raw bytes.
    #[arg(long = "auth-data", value_name = "DATA", help_heading = "MQTT")]
    pub auth_data: Option<String>,

    // ── File Transfer ────────────────────────────────────────────────────────

    /// FTPS: use implicit TLS (port 990) instead of explicit AUTH TLS.
    /// Has no effect on plain ftp://.
    #[arg(long = "ftps-implicit", help_heading = "File Transfer")]
    pub ftps_implicit: bool,

    /// FTP: use active mode (PORT) instead of the default passive mode
    /// (PASV / EPSV). Rarely needed; servers behind NAT may refuse.
    #[arg(long = "ftp-active", help_heading = "File Transfer")]
    pub ftp_active: bool,

    /// TFTP: request a specific transfer block size via the RFC 2348
    /// `blksize` option. Default is the RFC 1350 fixed size of 512 bytes.
    #[arg(long = "tftp-blksize", value_name = "N", help_heading = "File Transfer")]
    pub tftp_blksize: Option<usize>,

    /// Path to a persistent HSTS cache file. On request, an http:// URL
    /// to a host with a non-expired HSTS entry is upgraded to https://
    /// before sending. On response, Strict-Transport-Security headers
    /// are parsed and the cache is updated (max-age=0 removes entries).
    /// File format matches curl's --hsts.
    #[arg(long = "hsts", value_name = "PATH", help_heading = "HTTP Request")]
    pub hsts: Option<std::path::PathBuf>,

    /// Route the HTTP request through a Unix-domain socket instead of
    /// TCP. Target URL still supplies Host: header and path; transport
    /// goes over the socket. Useful for Docker (/var/run/docker.sock),
    /// systemd-activated services, and kubelet endpoints.
    #[arg(long = "unix-socket", value_name = "PATH", help_heading = "HTTP Request")]
    pub unix_socket: Option<std::path::PathBuf>,

    // ── Proxy ────────────────────────────────────────────────────────────────

    /// Route HTTP(S) requests through a proxy. Scheme selects the type:
    /// http:// = plain HTTP proxy; https:// = TLS-to-proxy; socks5:// =
    /// SOCKS5 (remote DNS); socks5h:// = SOCKS5 with client-side DNS.
    /// Falls back to $HTTPS_PROXY / $HTTP_PROXY / $ALL_PROXY (matching
    /// curl's precedence) when the flag isn't given.
    #[arg(short = 'x', long = "proxy", value_name = "URL", help_heading = "Proxy")]
    pub proxy: Option<String>,

    /// Basic-auth credentials for the proxy. Format: USER:PASS. Takes
    /// priority over any userinfo embedded in the proxy URL.
    #[arg(short = 'U', long = "proxy-user", value_name = "USER:PASS", help_heading = "Proxy")]
    pub proxy_user: Option<String>,

    /// Comma-separated list of hosts that bypass the proxy. Matches
    /// curl's NO_PROXY semantics: exact hostname or leading-dot suffix
    /// match (e.g. `.internal`). `*` means bypass all. Falls back to
    /// $NO_PROXY when absent.
    #[arg(long = "noproxy", value_name = "LIST", help_heading = "Proxy")]
    pub noproxy: Option<String>,

    /// Skip TLS certificate verification on the connection to an
    /// https:// proxy. Doesn't affect origin-cert verification.
    #[arg(long = "proxy-insecure", help_heading = "Proxy")]
    pub proxy_insecure: bool,

    /// Additional PEM-encoded CA certificate trusted for the connection
    /// to an https:// proxy. Adds to the system roots; doesn't replace.
    /// reqwest 0.12 applies CA-bundle overrides globally, so this trust
    /// root also applies to the origin request.
    #[arg(long = "proxy-cacert", value_name = "PATH", help_heading = "Proxy")]
    pub proxy_cacert: Option<std::path::PathBuf>,

    /// Chain a second proxy before the main --proxy. Accepted for
    /// curl parity; reqwest 0.12 has no chained-proxy API, so the
    /// flag is currently declarative.
    #[arg(long = "preproxy", value_name = "URL", help_heading = "Proxy")]
    pub preproxy: Option<String>,

    /// Headers to include on the proxy CONNECT request only.
    /// Repeatable. Accepted for curl parity; reqwest doesn't expose
    /// CONNECT-header hooks in the blocking client.
    #[arg(long = "proxy-header", value_name = "H: V", action = clap::ArgAction::Append, help_heading = "Proxy")]
    pub proxy_header: Vec<String>,

    /// Speak HTTP/2 to the proxy. Accepted; reqwest uses HTTP/2 via
    /// ALPN automatically when the proxy advertises it.
    #[arg(long = "proxy-http2", help_heading = "Proxy")]
    pub proxy_http2: bool,

    /// Force tunneling via CONNECT even for http:// origins. Accepted;
    /// reqwest auto-tunnels for https:// already. `-p` is used by
    /// `--prettify` in recon, so only the long form is accepted.
    #[arg(long = "proxytunnel", help_heading = "Proxy")]
    pub proxytunnel: bool,

    /// CA directory for the proxy connection. Accepted.
    #[arg(long = "proxy-capath", value_name = "DIR", help_heading = "Proxy")]
    pub proxy_capath: Option<std::path::PathBuf>,

    /// Use the OS-native trust store for the proxy connection.
    /// Accepted.
    #[arg(long = "proxy-ca-native", help_heading = "Proxy")]
    pub proxy_ca_native: bool,

    /// TLS CRL for the proxy connection. Accepted.
    #[arg(long = "proxy-crlfile", value_name = "PATH", help_heading = "Proxy")]
    pub proxy_crlfile: Option<std::path::PathBuf>,

    /// Cipher list for the proxy TLS connection. Accepted; rustls
    /// cipher selection is complex and not yet wired.
    #[arg(long = "proxy-ciphers", value_name = "LIST", help_heading = "Proxy")]
    pub proxy_ciphers: Option<String>,

    /// TLS 1.3 cipher suites for the proxy. Accepted.
    #[arg(long = "proxy-tls13-ciphers", value_name = "LIST", help_heading = "Proxy")]
    pub proxy_tls13_ciphers: Option<String>,

    /// Pin the proxy's public-key hash (sha256 base64 / file path).
    /// Accepted.
    #[arg(long = "proxy-pinnedpubkey", value_name = "HASHES", help_heading = "Proxy")]
    pub proxy_pinnedpubkey: Option<String>,

    /// Cipher list for the origin TLS connection. Accepted; rustls
    /// doesn't expose a direct cipher-list knob in 0.23. Revisit
    /// if rustls adds one.
    #[arg(long = "ciphers", value_name = "LIST", help_heading = "Auth & TLS")]
    pub ciphers: Option<String>,

    /// TLS 1.3 cipher suites for the origin. Accepted.
    #[arg(long = "tls13-ciphers", value_name = "LIST", help_heading = "Auth & TLS")]
    pub tls13_ciphers: Option<String>,

    /// Allowed ECDH curves / key-exchange groups. Accepted; rustls
    /// 0.23 exposes `kx_groups` selection but the curve-name mapping
    /// isn't wired yet.
    #[arg(long = "curves", value_name = "LIST", help_heading = "Auth & TLS")]
    pub curves: Option<String>,

    /// PEM-encoded CRL file for the origin connection. Accepted.
    #[arg(long = "crlfile", value_name = "PATH", help_heading = "Auth & TLS")]
    pub crlfile: Option<std::path::PathBuf>,

    /// Pin the server's public-key hash (sha256 base64 or file path).
    /// Accepted; a full custom ServerCertVerifier is a follow-up.
    #[arg(long = "pinnedpubkey", value_name = "HASHES", help_heading = "Auth & TLS")]
    pub pinnedpubkey: Option<String>,

    /// Read command-line flags from FILE (curl's `-K`). One per
    /// line, `#` comments allowed, `@other` includes another config
    /// file. Applied before clap parses the remaining argv.
    #[arg(short = 'K', long = "config", value_name = "FILE", help_heading = "Meta")]
    pub config: Option<PathBuf>,

    /// Don't read `~/.recon/config.toml` or `$RECON_CONFIG` on startup.
    /// Equivalent to curl's `-q` behaviour.
    #[arg(short = 'q', long = "disable", help_heading = "Meta")]
    pub disable_default_config: bool,

    // ── Mail Retrieval ───────────────────────────────────────────────────────

    /// POP3: upgrade to TLS via the STLS command after CAPA. Mirrors
    /// SMTP's STARTTLS. Ignored on pop3s:// (already implicit-TLS).
    #[arg(long = "stls", help_heading = "Mail Retrieval")]
    pub stls: bool,

    /// IMAP: use BODY.PEEK[] when fetching a message so the server
    /// doesn't flip the \Seen flag. Matches curl's IMAP default.
    #[arg(long = "imap-peek", help_heading = "Mail Retrieval")]
    pub imap_peek: bool,

    // ── SMTP ─────────────────────────────────────────────────────────────────

    /// Envelope sender (`MAIL FROM:<…>`). Required for send mode; omit
    /// for probe-only mode (connect + EHLO + capabilities).
    #[arg(long = "mail-from", value_name = "ADDR", help_heading = "SMTP")]
    pub mail_from: Option<String>,

    /// Envelope recipient (`RCPT TO:<…>`). Repeatable for multiple
    /// recipients. Required for send mode.
    #[arg(long = "mail-to", value_name = "ADDR", action = clap::ArgAction::Append, help_heading = "SMTP")]
    pub mail_to: Vec<String>,

    /// Subject header for the test message. Default: "recon SMTP test".
    #[arg(long = "mail-subject", value_name = "STR", help_heading = "SMTP")]
    pub mail_subject: Option<String>,

    /// Message body. Accepts `@file` to read from a file, `@-` to read
    /// from stdin, or the literal text. Default: one-line test note.
    #[arg(long = "mail-body", value_name = "STR", help_heading = "SMTP")]
    pub mail_body: Option<String>,

    /// Extra message header (e.g. `Reply-To: me@example.com`). Repeatable.
    #[arg(long = "mail-header", value_name = "H: V", action = clap::ArgAction::Append, help_heading = "SMTP")]
    pub mail_header: Vec<String>,

    /// SMTP authentication credentials as `user:pass`. Tries AUTH PLAIN
    /// then LOGIN.
    #[arg(long = "smtp-auth", value_name = "USER:PASS", help_heading = "SMTP")]
    pub smtp_auth: Option<String>,

    /// HELO / EHLO hostname to advertise. Default: `recon.local`.
    #[arg(long = "smtp-helo", value_name = "NAME", help_heading = "SMTP")]
    pub smtp_helo: Option<String>,

    /// Don't negotiate STARTTLS even when the server advertises it.
    /// Useful for probing a server's behaviour without TLS.
    #[arg(long = "no-starttls", help_heading = "SMTP")]
    pub no_starttls: bool,

    /// PEM-encoded DKIM signing key (RSA or Ed25519). Enables DKIM
    /// signing on outbound messages. Requires --dkim-selector.
    #[arg(long = "dkim-key", value_name = "PATH", help_heading = "SMTP")]
    pub dkim_key: Option<std::path::PathBuf>,

    /// DKIM selector (the `s=` tag — matches the DNS TXT selector).
    #[arg(long = "dkim-selector", value_name = "SEL", help_heading = "SMTP")]
    pub dkim_selector: Option<String>,

    /// DKIM signing domain (the `d=` tag). Defaults to the domain part
    /// of --mail-from.
    #[arg(long = "dkim-domain", value_name = "DOMAIN", help_heading = "SMTP")]
    pub dkim_domain: Option<String>,

    // ── Docs ─────────────────────────────────────────────────────────────────

    /// Render markdown → HTML. SRC = path / URL / `-` (stdin). Output
    /// via `-o PATH` (or stdout). Honors HTTP flags (-H, -u, -L, -k…)
    /// when SRC is an http(s):// URL.
    #[arg(long = "md-to-html", value_name = "SRC", help_heading = "Docs")]
    pub md_to_html: Option<String>,

    /// Render markdown → PDF via the md-to-html pipeline and
    /// `agent-browser pdf`. `-o PATH` required. Chrome is needed
    /// through agent-browser.
    #[arg(long = "md-to-pdf", value_name = "SRC", help_heading = "Docs")]
    pub md_to_pdf: Option<String>,

    /// Render HTML → PDF via `agent-browser pdf`. SRC = path / URL /
    /// `-` (stdin). `-o PATH` required. Chrome is needed through
    /// agent-browser.
    #[arg(long = "html-to-pdf", value_name = "SRC", help_heading = "Docs")]
    pub html_to_pdf: Option<String>,

    /// Inject a linkable table of contents at the top of the
    /// generated HTML (md-to-html and md-to-pdf only).
    #[arg(long = "toc", help_heading = "Docs")]
    pub toc: bool,

    /// Include headings up to H`N` in the TOC. Default 3.
    #[arg(long = "toc-depth", value_name = "N", default_value_t = 3, help_heading = "Docs")]
    pub toc_depth: u8,

    /// Heading text for the injected TOC. Default "Contents".
    #[arg(long = "toc-title", value_name = "STR", default_value = "Contents", help_heading = "Docs")]
    pub toc_title: String,

    /// Sets <title> (HTML) + PDF metadata title. Default: basename of
    /// SRC with extension stripped.
    #[arg(long = "doc-title", value_name = "STR", help_heading = "Docs")]
    pub doc_title: Option<String>,

    /// Override the bundled print-friendly CSS with a user stylesheet
    /// (inlined into the generated HTML). Pair with --no-default-css
    /// to replace rather than append.
    #[arg(long = "doc-css", value_name = "PATH", help_heading = "Docs")]
    pub doc_css: Option<PathBuf>,

    /// Skip the bundled default CSS. Only useful paired with
    /// --doc-css.
    #[arg(long = "no-default-css", help_heading = "Docs")]
    pub no_default_css: bool,

    /// Enable GitHub-flavored markdown extensions: tables, task
    /// lists, strikethrough, autolinks, footnotes, tagfilter.
    #[arg(long = "gfm", help_heading = "Docs")]
    pub gfm: bool,

    /// Allow raw HTML to pass through the markdown parser verbatim
    /// (comrak's `unsafe_` render option). Needed for cover pages,
    /// styled `<div class="page-break">` markers, and arbitrary
    /// inline HTML. Disabled by default — the markdown input is
    /// assumed safe when this is on.
    #[arg(long = "unsafe-html", help_heading = "Docs")]
    pub unsafe_html: bool,

    /// Start a new PDF page before every top-level `#` heading
    /// (except the first). Inserts `break-before: page` CSS on
    /// every H1 after the opening one. Use with --md-to-pdf /
    /// --html-to-pdf; has no visible effect in HTML output.
    #[arg(long = "page-break-on-h1", help_heading = "Docs")]
    pub page_break_on_h1: bool,

    // ── Compare ──────────────────────────────────────────────────────────────

    /// Diff two sources. Each source is a URL, a local path, or `-` for
    /// stdin. HTTP(S) sources honor all existing request flags (-H, -u,
    /// -L, -k, headers, cookies, …). Exit code: 0 = identical, 1 =
    /// differ, 2+ = source-load error.
    #[arg(long = "compare", value_names = ["A", "B"], num_args = 2, help_heading = "Compare")]
    pub compare: Option<Vec<String>>,

    /// Output format for --compare. `unified` (default) prints a unified
    /// diff. `summary` prints a one-liner (identical / differ / binary).
    /// `sxs` prints a side-by-side view column-wrapped to terminal width.
    #[arg(long = "compare-format", value_name = "FMT", default_value = "unified", help_heading = "Compare")]
    pub compare_format: String,

    /// Context lines around each unified-diff hunk. Default 3.
    #[arg(long = "compare-context", value_name = "N", default_value_t = 3, help_heading = "Compare")]
    pub compare_context: usize,

    // ── Meta ─────────────────────────────────────────────────────────────────

    /// IPFS gateway for ipfs:// and ipns:// URL rewriting. Default:
    /// https://ipfs.io. Also read from $RECON_IPFS_GATEWAY. Set to
    /// http://127.0.0.1:8080 to use a local Kubo / IPFS-Desktop node.
    #[arg(long = "ipfs-gateway", value_name = "URL", help_heading = "HTTP Request")]
    pub ipfs_gateway: Option<String>,

    /// Show detailed usage examples for all flags and commands
    #[arg(long = "examples", help_heading = "Meta")]
    pub examples: bool,

    /// List every flag alphabetically by long name, curl-style.
    /// Short key on the left (or padding), long name, value hint,
    /// short description. Complements --help (topic deep-dives) and
    /// --examples (curated scenarios) — this is the quick lookup.
    #[arg(long = "flags", help_heading = "Meta")]
    pub flags: bool,

    /// Bootstrap `~/.recon/` with script/, jars/, sni/ subdirectories and
    /// a commented config.toml skeleton. Existing files and directories
    /// are not overwritten — safe to re-run.
    #[arg(long = "init", help_heading = "Meta")]
    pub init: bool,

    /// Disable auto-paging of `--help` and `--examples` output. Paging is
    /// on by default when stdout is a TTY (uses `$PAGER` or `less -FRX`).
    /// Also respects `$RECON_NO_PAGER`. Non-TTY stdout (pipes, redirects)
    /// is never paged regardless of this flag.
    #[arg(long = "no-pager", help_heading = "Meta")]
    pub no_pager: bool,

    /// Open URL in a browser (via agent-browser) and save a screenshot.
    /// Use -o PATH to choose the destination; otherwise agent-browser's
    /// default location is used. Requires agent-browser on PATH.
    #[arg(long = "browser-screenshot", value_name = "URL", help_heading = "Browser")]
    pub browser_screenshot: Option<String>,

    /// Create an archive. Format inferred from DEST's extension:
    /// .zip / .tar / .tar.gz (.tgz) / .tar.xz (.txz) / .tar.bz2 (.tbz2).
    /// Remaining positional args after DEST are the sources to include
    /// (files or directories; directories are recursed). Sources are
    /// collected via the same argv pre-split that handles `--script`.
    #[arg(long = "archive", value_name = "DEST", help_heading = "Archive")]
    pub archive: Option<PathBuf>,

    /// Extract an archive. Format inferred from SRC's extension or from
    /// magic bytes. Destination defaults to the current directory; pass
    /// `-o DIR` to change it.
    #[arg(long = "extract", value_name = "SRC", help_heading = "Archive")]
    pub extract: Option<PathBuf>,

    /// Run a Rhai script instead of performing a request. Exposes `http()`,
    /// `tcp()`, `ping()`, `dns()`, `tls()`, `redis()`, `ws()` and more;
    /// script `return N` becomes the process exit code. If PATH isn't found
    /// as given, falls back to `~/.recon/script/PATH` (and `.rhai` is
    /// auto-appended when PATH has no extension). Positional args after the
    /// script path are available to the script as `args[1..]` (args[0] is
    /// the script name). See `--help script`.
    #[arg(long = "script", value_name = "PATH", help_heading = "Meta")]
    pub script: Option<PathBuf>,

    /// Trailing positional args forwarded to `--script`. Populated in
    /// `main.rs` by splitting argv on the `--script` boundary before clap
    /// parses — clap is skipped here to avoid a conflict with the
    /// positional `url` field that would otherwise swallow the first
    /// trailing arg. Exposed to scripts as `args[1..]`.
    #[arg(skip)]
    pub script_args: Vec<String>,

    // ── Options (manual -h / -V; keeps Options at tail of --help) ────────────

    // Display-only declarations. --help is intercepted in main.rs before clap
    // parses, so `help` never receives a value; --version is handled by
    // clap's ArgAction::Version. Present here so they render under Options
    // in --help output.
    /// Print help
    #[arg(short = 'h', long = "help", action = clap::ArgAction::Help, help_heading = "Options")]
    pub help: Option<bool>,

    /// Print version banner (curl-compatible multi-line format)
    #[arg(short = 'V', long = "version", help_heading = "Options")]
    pub version: bool,

    /// Print just the version number (e.g. "recon 0.21.0")
    #[arg(long = "version-short", help_heading = "Options")]
    pub version_short: bool,
}

impl Args {
    /// Effective HTTP method after flag precedence is applied.
    /// Priority:
    ///   1. Explicit `-X/--request` if supplied.
    ///   2. PUT when `-T/--upload-file` is set.
    ///   3. POST when any body-bearing flag (`-d`, `--json`, `--data-raw`,
    ///      `--data-binary`, `--data-urlencode`) is present and `-G/--get`
    ///      is not. Matches curl: all five flags imply POST by default.
    ///   4. GET.
    pub fn effective_method(&self) -> String {
        if let Some(m) = &self.method {
            return m.to_uppercase();
        }
        if self.upload_file.is_some() {
            return "PUT".to_string();
        }
        let has_body_flag = self.data.is_some()
            || self.json.is_some()
            || self.data_raw.is_some()
            || self.data_binary.is_some()
            || !self.data_urlencode.is_empty();
        if has_body_flag && !self.get_data {
            return "POST".to_string();
        }
        "GET".to_string()
    }

    /// Returns the effective URL, preferring --url over the positional argument.
    pub fn target_url(&self) -> &str {
        self.url_flag
            .as_deref()
            .or(self.url.as_deref())
            .unwrap_or_default()
    }

    /// Returns true if any email protection check flag is set.
    pub fn has_email_checks(&self) -> bool {
        self.spf || self.dmarc || !self.dkim.is_empty() || self.mta_sts || self.bimi.is_some() || self.tls_rpt
    }

    /// Returns true if any composable domain-inspection flag is set.
    pub fn has_composable(&self) -> bool {
        self.cert || self.dns || self.has_email_checks()
    }

    /// Returns true if any exclusive network-tool flag is set.
    pub fn has_exclusive(&self) -> bool {
        self.ping || self.traceroute || self.whois
    }

    pub fn has_serve(&self) -> bool {
        self.serve.is_some() || self.serve_tls.is_some() || !self.serve_sni.is_empty()
    }

    /// Returns true if any JWT operation flag is set.
    pub fn has_jwt(&self) -> bool {
        self.jwt_view || self.jwt_sign || self.jwt_validate
    }

    /// Returns the count of exclusive flags set (for mutual exclusion check).
    pub fn exclusive_count(&self) -> usize {
        [self.ping, self.traceroute, self.whois].iter().filter(|&&f| f).count()
    }

    /// Parse argv, pre-splitting on `--script PATH` so trailing positional
    /// arguments after the script path populate `script_args` instead of
    /// being consumed by the positional `url` field. Used by `main.rs` and
    /// by tests that need the same semantics.
    pub fn parse_with_script_split<I, S>(argv: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let raw: Vec<String> = argv.into_iter().map(Into::into).collect();
        let (for_clap, trailing) = Self::split_script_trailing(&raw);
        let mut args = Self::try_parse_from(for_clap)?;
        args.script_args = trailing;
        Ok(args)
    }

    /// Split argv into `(for_clap, trailing_script_args)`. Exposed on
    /// `Args` so `main.rs` and test helpers share one implementation.
    /// Also handles `--archive PATH` — trailing args after DEST go to
    /// the same `script_args` field (it doubles as "trailing positional
    /// sources" for both flags; mutual exclusion is enforced at dispatch).
    pub fn split_script_trailing(raw: &[String]) -> (Vec<String>, Vec<String>) {
        for (i, tok) in raw.iter().enumerate() {
            if tok == "--script" || tok == "--archive" {
                let boundary = (i + 2).min(raw.len());
                return (raw[..boundary].to_vec(), raw[boundary..].to_vec());
            }
            if tok.starts_with("--script=") || tok.starts_with("--archive=") {
                let boundary = (i + 1).min(raw.len());
                return (raw[..boundary].to_vec(), raw[boundary..].to_vec());
            }
        }
        (raw.to_vec(), Vec::new())
    }
}

#[cfg(test)]
impl Args {
    pub fn test_default() -> Self {
        use clap::Parser;
        Args::try_parse_from(["recon", "http://example.com/"]).expect("test default parses")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn effective_method_defaults_to_get() {
        let args = Args::try_parse_from(["recon", "https://example.com/"]).unwrap();
        assert_eq!(args.effective_method(), "GET");
    }

    #[test]
    fn effective_method_promotes_to_post_on_data() {
        let args = Args::try_parse_from(["recon", "https://example.com/", "-d", "x=1"]).unwrap();
        assert_eq!(args.effective_method(), "POST");
    }

    #[test]
    fn effective_method_stays_get_with_dash_g() {
        let args = Args::try_parse_from(["recon", "https://example.com/", "-d", "x=1", "-G"]).unwrap();
        assert_eq!(args.effective_method(), "GET");
    }

    #[test]
    fn effective_method_honors_explicit_request() {
        let args = Args::try_parse_from(["recon", "https://example.com/", "-X", "patch"]).unwrap();
        assert_eq!(args.effective_method(), "PATCH");
    }

    #[test]
    fn effective_method_explicit_overrides_data_post() {
        let args = Args::try_parse_from(["recon", "https://example.com/", "-X", "put", "-d", "x=1"]).unwrap();
        assert_eq!(args.effective_method(), "PUT");
    }

    #[test]
    fn effective_method_promotes_to_put_on_upload_file() {
        let args = Args::try_parse_from(["recon", "https://example.com/", "-T", "Cargo.toml"]).unwrap();
        assert_eq!(args.effective_method(), "PUT");
    }

    #[test]
    fn effective_method_explicit_overrides_upload_put() {
        let args = Args::try_parse_from(["recon", "https://example.com/", "-T", "Cargo.toml", "-X", "POST"]).unwrap();
        assert_eq!(args.effective_method(), "POST");
    }

    #[test]
    fn effective_method_promotes_to_post_on_json() {
        let args = Args::try_parse_from(["recon", "https://example.com/", "--json", "{}"]).unwrap();
        assert_eq!(args.effective_method(), "POST");
    }

    #[test]
    fn effective_method_promotes_to_post_on_data_raw() {
        let args = Args::try_parse_from(["recon", "https://example.com/", "--data-raw", "x"]).unwrap();
        assert_eq!(args.effective_method(), "POST");
    }

    #[test]
    fn effective_method_promotes_to_post_on_data_binary() {
        let args = Args::try_parse_from(["recon", "https://example.com/", "--data-binary", "x"]).unwrap();
        assert_eq!(args.effective_method(), "POST");
    }

    #[test]
    fn effective_method_promotes_to_post_on_data_urlencode() {
        let args = Args::try_parse_from(["recon", "https://example.com/", "--data-urlencode", "a=b"]).unwrap();
        assert_eq!(args.effective_method(), "POST");
    }

    #[test]
    fn effective_method_json_stays_get_with_dash_g() {
        let args = Args::try_parse_from(["recon", "https://example.com/", "--json", "{}", "-G"]).unwrap();
        assert_eq!(args.effective_method(), "GET");
    }
}

#[cfg(test)]
mod body_variant_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_data_raw() {
        let args = Args::try_parse_from(["recon", "--data-raw", "@literal", "http://x/"]).unwrap();
        assert_eq!(args.data_raw.as_deref(), Some("@literal"));
    }

    #[test]
    fn parses_data_binary() {
        let args = Args::try_parse_from(["recon", "--data-binary", "@file.bin", "http://x/"]).unwrap();
        assert_eq!(args.data_binary.as_deref(), Some("@file.bin"));
    }

    #[test]
    fn data_urlencode_is_repeatable() {
        let args = Args::try_parse_from([
            "recon",
            "--data-urlencode", "a=hello world",
            "--data-urlencode", "b=x&y",
            "http://x/"
        ]).unwrap();
        assert_eq!(args.data_urlencode.len(), 2);
        assert_eq!(args.data_urlencode[0], "a=hello world");
    }
}

#[cfg(test)]
mod json_flag_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_json_flag() {
        let args = Args::try_parse_from(["recon", "--json", r#"{"a":1}"#, "http://x/"]).unwrap();
        assert_eq!(args.json.as_deref(), Some(r#"{"a":1}"#));
    }

    #[test]
    fn stores_json_at_file_verbatim() {
        let args = Args::try_parse_from(["recon", "--json", "@body.json", "http://x/"]).unwrap();
        assert_eq!(args.json.as_deref(), Some("@body.json"));
    }
}

#[cfg(test)]
mod mqtt_flag_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn mqtt_version_defaults_to_5() {
        let args = Args::try_parse_from(["recon", "mqtt://b/"]).unwrap();
        assert_eq!(args.mqtt_version, "5");
    }

    #[test]
    fn mqtt_version_accepts_3() {
        let args = Args::try_parse_from(["recon", "mqtt://b/", "--mqtt-version", "3"]).unwrap();
        assert_eq!(args.mqtt_version, "3");
    }

    #[test]
    fn mqtt_keepalive_defaults_to_60() {
        let args = Args::try_parse_from(["recon", "mqtt://b/"]).unwrap();
        assert_eq!(args.keepalive, 60);
    }

    #[test]
    fn mqtt_qos_defaults_to_0() {
        let args = Args::try_parse_from(["recon", "mqtt://b/"]).unwrap();
        assert_eq!(args.qos, 0);
    }

    #[test]
    fn mqtt_subscribe_repeatable() {
        let args = Args::try_parse_from([
            "recon", "mqtt://b/", "--subscribe", "a/#", "--subscribe", "b/+/c",
        ]).unwrap();
        assert_eq!(args.subscribe.len(), 2);
    }

    #[test]
    fn mqtt_retain_default_false() {
        let args = Args::try_parse_from(["recon", "mqtt://b/"]).unwrap();
        assert!(!args.retain);
    }

    #[test]
    fn mqtt_count_default_none() {
        let args = Args::try_parse_from(["recon", "mqtt://b/"]).unwrap();
        assert!(args.count.is_none());
    }

    #[test]
    fn mqtt_count_parses() {
        let args = Args::try_parse_from(["recon", "mqtt://b/", "--count", "5"]).unwrap();
        assert_eq!(args.count, Some(5));
    }

    #[test]
    fn mqtt_json_default_false() {
        let args = Args::try_parse_from(["recon", "mqtt://b/"]).unwrap();
        assert!(!args.mqtt_json);
    }

    #[test]
    fn mqtt_json_sets_true() {
        let args = Args::try_parse_from(["recon", "mqtt://b/", "--mqtt-json"]).unwrap();
        assert!(args.mqtt_json);
    }
}

#[cfg(test)]
mod udp_flag_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn wait_time_defaults_to_1() {
        let args = Args::try_parse_from(["recon", "udp://b:1/"]).unwrap();
        assert_eq!(args.wait_time, 1.0);
    }

    #[test]
    fn wait_time_accepts_fractional() {
        let args = Args::try_parse_from(["recon", "udp://b:1/", "--wait-time", "0.5"]).unwrap();
        assert_eq!(args.wait_time, 0.5);
    }
}
