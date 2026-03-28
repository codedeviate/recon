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
    #[arg(required_unless_present_any = ["url_flag", "cookies", "cookie_delete", "cookie_set", "spf", "dmarc", "dkim", "mta_sts", "bimi", "tls_rpt"])]
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

    /// Returns the count of exclusive flags set (for mutual exclusion check).
    pub fn exclusive_count(&self) -> usize {
        [self.ping, self.traceroute, self.whois].iter().filter(|&&f| f).count()
    }
}
