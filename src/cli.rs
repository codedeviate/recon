use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "recon",
    about = "A versatile network reconnaissance tool",
    version
)]
pub struct Args {
    /// URL to request (or use --url)
    #[arg(required_unless_present_any = ["url_flag", "cookies", "cookie_delete", "cookie_set", "spf", "dmarc", "dkim", "mta_sts", "bimi", "tls_rpt", "serve", "serve_tls", "serve_sni", "jwt_view", "jwt_sign", "jwt_validate", "netstatus"])]
    pub url: Option<String>,

    /// URL to request — curl-compatible alternative to the positional argument
    #[arg(id = "url_flag", long = "url", value_name = "URL")]
    pub url_flag: Option<String>,

    /// HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD)
    #[arg(short = 'X', long = "request", default_value = "GET")]
    pub method: String,

    /// Send request headers (can be repeated: -H "Name: Value")
    #[arg(short = 'H', long = "header")]
    pub header: Vec<String>,

    /// Request body data. Prefix with @ to read from file (e.g. @body.json)
    #[arg(short = 'd', long = "data")]
    pub data: Option<String>,

    /// Follow redirects
    #[arg(short = 'L', long = "location")]
    pub follow_redirects: bool,

    /// Maximum number of redirects to follow
    #[arg(long = "max-redirs", default_value_t = 10)]
    pub max_redirs: usize,

    /// Write output to file instead of stdout
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    /// Silent mode: suppress progress and informational output
    #[arg(short = 's', long = "silent")]
    pub silent: bool,

    /// Verbose: print request/response headers to stderr (-v); add -vv for timing and auth detail
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Include response headers in output
    #[arg(short = 'i', long = "include")]
    pub include_headers: bool,

    /// Custom User-Agent string
    #[arg(short = 'A', long = "user-agent")]
    pub user_agent: Option<String>,

    /// Connection timeout in seconds
    #[arg(long = "connect-timeout", default_value_t = 30)]
    pub timeout: u64,

    /// Fail silently (exit non-zero) on HTTP errors (status >= 400)
    #[arg(short = 'f', long = "fail")]
    pub fail_on_error: bool,

    /// Print only the HTTP status code
    #[arg(short = 'S', long = "status")]
    pub status_only: bool,

    /// Output only the response headers, suppress body
    #[arg(short = 'I', long = "head")]
    pub head_only: bool,

    /// Output status line, all headers, and body
    #[arg(long = "full")]
    pub full: bool,

    /// Print response headers for every hop, following redirects (implies redirect following)
    #[arg(long = "LHEAD")]
    pub lhead: bool,

    /// Prettify response body: auto-detects JSON, XML, HTML, YAML, CSV, TSV
    #[arg(short = 'p', long = "prettify")]
    pub prettify: bool,

    /// Skip TLS/SSH host key verification (insecure — equivalent to curl -k)
    #[arg(short = 'k', long = "insecure")]
    pub insecure: bool,

    /// HTTP Basic auth or SSH username; format: user or user:pass
    #[arg(short = 'u', long = "user", value_name = "USER:PASS")]
    pub user: Option<String>,

    /// Path to SSH private key file for SCP authentication
    #[arg(long = "ssh-key", value_name = "PATH")]
    pub ssh_key: Option<PathBuf>,

    /// Path to SSH public key file (optional; derived from --ssh-key if omitted)
    #[arg(long = "ssh-pubkey", value_name = "PATH")]
    pub ssh_pubkey: Option<PathBuf>,

    /// Passphrase for the SSH private key, or password for SSH password auth
    #[arg(long = "ssh-pass", value_name = "PASS")]
    pub ssh_pass: Option<String>,

    /// Show a progress meter when saving to a file (opt-in, unlike curl)
    #[arg(long = "progress")]
    pub progress: bool,

    /// Send -d data as a URL query string with GET instead of as a request body
    #[arg(short = 'G', long = "get")]
    pub get_data: bool,

    /// Print full error details including internal causes
    #[arg(long = "FULL-ERRORS")]
    pub full_errors: bool,

    /// Fetch and display the server's TLS certificate without making an HTTP request (HTTPS only)
    #[arg(long = "cert")]
    pub cert: bool,

    /// Traceroute to the host (uses port if specified in the address)
    #[arg(long = "traceroute", alias = "trace")]
    pub traceroute: bool,

    /// Maximum number of hops for traceroute
    #[arg(long = "max-hops", default_value_t = 30)]
    pub max_hops: u8,

    /// Ping the host. TCP ping if a port is given (e.g. host:443), ICMP ping otherwise
    #[arg(long = "ping")]
    pub ping: bool,

    /// Number of pings to send
    #[arg(long = "ping-count", default_value_t = 4)]
    pub ping_count: u32,

    /// DNS lookup for the host — shows common record types by default
    #[arg(long = "dns")]
    pub dns: bool,

    /// DNS record type(s) to query, comma-separated (A,AAAA,MX,NS,TXT,SOA,CNAME,PTR,SRV,CAA,…)
    #[arg(long = "dns-type", value_delimiter = ',')]
    pub dns_type: Vec<String>,

    /// WHOIS lookup for a domain or IP address
    #[arg(long = "whois")]
    pub whois: bool,

    /// Validate the SPF record (recursive include/redirect resolution, lookup limits)
    #[arg(long = "spf")]
    pub spf: bool,

    /// Validate the DMARC record and policy
    #[arg(long = "dmarc")]
    pub dmarc: bool,

    /// Validate the DKIM record for the given selector (repeatable: --dkim sel1 --dkim sel2)
    #[arg(long = "dkim", value_name = "SELECTOR", action = clap::ArgAction::Append)]
    pub dkim: Vec<String>,

    /// Validate MTA-STS DNS record and HTTPS policy
    #[arg(long = "mta-sts")]
    pub mta_sts: bool,

    /// Validate the BIMI record (default selector: "default")
    #[arg(long = "bimi", value_name = "SELECTOR", num_args = 0..=1, default_missing_value = "default")]
    pub bimi: Option<String>,

    /// Validate the TLS-RPT reporting record
    #[arg(long = "tls-rpt")]
    pub tls_rpt: bool,

    /// Start an HTTP file server on the given port (default: 80)
    #[arg(long = "serve", value_name = "PORT", num_args = 0..=1, default_missing_value = "80")]
    pub serve: Option<String>,

    /// Start an HTTPS file server on the given port (default: 443)
    #[arg(long = "serve-tls", value_name = "PORT", num_args = 0..=1, default_missing_value = "443")]
    pub serve_tls: Option<String>,

    /// Force HTTP version for the server: 1.1 or 2 (default: auto-negotiate)
    #[arg(long = "http-version", value_name = "VERSION")]
    pub http_version: Option<String>,

    /// Path to TLS certificate PEM file (default: ~/.recon/cert.pem)
    #[arg(long = "serve-cert", value_name = "PATH")]
    pub serve_cert: Option<std::path::PathBuf>,

    /// Path to TLS private key PEM file (default: ~/.recon/key.pem)
    #[arg(long = "serve-key", value_name = "PATH")]
    pub serve_key: Option<std::path::PathBuf>,

    /// Write access log to this file (in addition to terminal output)
    #[arg(long = "serve-log", value_name = "PATH")]
    pub serve_log: Option<std::path::PathBuf>,

    /// SNI hostname-to-certificate mapping (repeatable: inline host:cert:key, directory, or config file)
    #[arg(long = "serve-sni", value_name = "MAPPING", action = clap::ArgAction::Append)]
    pub serve_sni: Vec<String>,

    /// Cookie jar to use for this request (name or path to a .db file).
    /// Omit the value to use the default jar.
    #[arg(long = "cookiejar", value_name = "NAME", num_args = 0..=1, default_missing_value = "default")]
    pub cookiejar: Option<String>,

    /// List all cookies in the jar (requires --cookiejar)
    #[arg(long = "cookies")]
    pub cookies: bool,

    /// Delete the cookie with the given ID (requires --cookiejar)
    #[arg(long = "cookie-delete", value_name = "ID")]
    pub cookie_delete: Option<i64>,

    /// Add or update a cookie (requires --cookiejar)
    /// Format: "name=value; Domain=example.com; [Path=/]; [Secure]; [HttpOnly]; [Max-Age=N]"
    #[arg(long = "cookie-set", value_name = "COOKIE")]
    pub cookie_set: Option<String>,

    /// Show detailed usage examples for all flags and commands
    #[arg(long = "examples")]
    pub examples: bool,

    // ── JWT ──────────────────────────────────────────────────────────────────

    /// Decode and display JWT header and payload without verification
    #[arg(long = "jwt-view")]
    pub jwt_view: bool,

    /// Sign or complete a JWT token
    #[arg(long = "jwt-sign")]
    pub jwt_sign: bool,

    /// Verify JWT signature and opt-in claim checks
    #[arg(long = "jwt-validate")]
    pub jwt_validate: bool,

    /// HMAC secret for signing or validating (required for --jwt-sign and --jwt-validate)
    #[arg(long = "jwt-secret", value_name = "SECRET")]
    pub jwt_secret: Option<String>,

    /// Algorithm: HS256 (default), HS384, HS512
    #[arg(long = "jwt-alg", alias = "jwt-algorithm", value_name = "ALG")]
    pub jwt_alg: Option<String>,

    /// JWT issuer claim — set when signing (if absent); assert value when validating with --jwt-validate-iss
    #[arg(long = "jwt-iss", value_name = "VALUE")]
    pub jwt_iss: Option<String>,

    /// JWT subject claim
    #[arg(long = "jwt-sub", value_name = "VALUE")]
    pub jwt_sub: Option<String>,

    /// JWT audience claim
    #[arg(long = "jwt-aud", value_name = "VALUE")]
    pub jwt_aud: Option<String>,

    /// JWT expiry (Unix timestamp). Omit value to use current time.
    #[arg(long = "jwt-exp", value_name = "TIMESTAMP", num_args = 0..=1, default_missing_value = "now")]
    pub jwt_exp: Option<String>,

    /// JWT not-before (Unix timestamp). Omit value to use current time.
    #[arg(long = "jwt-nbf", value_name = "TIMESTAMP", num_args = 0..=1, default_missing_value = "now")]
    pub jwt_nbf: Option<String>,

    /// JWT issued-at (Unix timestamp). Omit value to use current time.
    #[arg(long = "jwt-iat", value_name = "TIMESTAMP", num_args = 0..=1, default_missing_value = "now")]
    pub jwt_iat: Option<String>,

    /// JWT ID claim
    #[arg(long = "jwt-jti", value_name = "VALUE")]
    pub jwt_jti: Option<String>,

    /// Validate the exp claim (must not be expired)
    #[arg(long = "jwt-validate-exp")]
    pub jwt_validate_exp: bool,

    /// Validate the nbf claim (must not be used before valid)
    #[arg(long = "jwt-validate-nbf")]
    pub jwt_validate_nbf: bool,

    /// Validate the iat claim (must not be in the future)
    #[arg(long = "jwt-validate-iat")]
    pub jwt_validate_iat: bool,

    /// Validate iss matches --jwt-iss
    #[arg(long = "jwt-validate-iss")]
    pub jwt_validate_iss: bool,

    /// Validate sub matches --jwt-sub
    #[arg(long = "jwt-validate-sub")]
    pub jwt_validate_sub: bool,

    /// Validate aud matches --jwt-aud
    #[arg(long = "jwt-validate-aud")]
    pub jwt_validate_aud: bool,

    /// Validate jti matches --jwt-jti
    #[arg(long = "jwt-validate-jti")]
    pub jwt_validate_jti: bool,

    /// Enable all claim validation checks
    #[arg(long = "jwt-validate-full")]
    pub jwt_validate_full: bool,

    /// Output JWT results as a single JSON object instead of labeled sections
    #[arg(long = "jwt-json-report")]
    pub jwt_json_report: bool,

    // ── Network status ───────────────────────────────────────────────────────

    /// Check connectivity using probes defined in ~/.recon/config.toml
    #[arg(long = "netstatus")]
    pub netstatus: bool,
}

impl Args {
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
}
