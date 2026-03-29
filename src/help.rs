use colored::Colorize;

struct Topic {
    title: &'static str,
    description: &'static str,
    flags: &'static [FlagHelp],
    related: &'static [&'static str],
    examples: &'static [ExampleHelp],
}

struct FlagHelp {
    flags: &'static str,
    description: &'static str,
}

struct ExampleHelp {
    description: &'static str,
    command: &'static str,
}

// ── Topic definitions ────────────────────────────────────────────────────────

static TOPIC_HTTP: Topic = Topic {
    title: "HTTP/HTTPS Requests",
    description: "Send HTTP and HTTPS requests with full control over method, headers, body,\n\
                  authentication, and redirect behaviour. The URL can be passed as a positional\n\
                  argument or via --url. When -d is provided and the method is GET, recon\n\
                  automatically promotes it to POST (unless -G is set).",
    flags: &[
        FlagHelp { flags: "<URL> / --url <URL>", description: "Target URL. The positional argument and --url are interchangeable.\nA bare hostname (e.g. example.com) is treated as https://example.com." },
        FlagHelp { flags: "-X, --request <METHOD>", description: "HTTP method: GET, POST, PUT, PATCH, DELETE, HEAD.\nDefaults to GET (or POST when -d is present)." },
        FlagHelp { flags: "-H, --header <NAME: VALUE>", description: "Add a request header. Repeatable.\nExample: -H \"Authorization: Bearer tok\" -H \"Accept: application/json\"" },
        FlagHelp { flags: "-d, --data <BODY | @FILE>", description: "Request body. Prefix with @ to read from a file (e.g. -d @payload.json).\nPromotes GET to POST automatically unless -G is also set." },
        FlagHelp { flags: "-u, --user <USER:PASS>", description: "HTTP Basic authentication credentials.\nFormat: username:password." },
        FlagHelp { flags: "-L, --location", description: "Follow HTTP redirects (3xx responses)." },
        FlagHelp { flags: "--max-redirs <N>", description: "Maximum number of redirects to follow (default: 10).\nRequires -L or --LHEAD." },
        FlagHelp { flags: "--LHEAD", description: "Follow redirects and print response headers at every hop.\nImplies redirect following; useful for debugging redirect chains." },
        FlagHelp { flags: "--connect-timeout <SECS>", description: "TCP connection timeout in seconds (default: 30)." },
        FlagHelp { flags: "-G, --get", description: "Send -d data as URL query parameters with GET instead of as a body.\nThe request body remains empty." },
        FlagHelp { flags: "-k, --insecure", description: "Skip TLS certificate verification.\nDisables hostname, expiry, and chain checks." },
        FlagHelp { flags: "-A, --user-agent <STRING>", description: "Custom User-Agent header value." },
        FlagHelp { flags: "-f, --fail", description: "Exit with non-zero status on HTTP 4xx/5xx responses.\nNo output is printed for the error response body." },
    ],
    related: &["--cert", "--cookiejar", "-p / --prettify"],
    examples: &[
        ExampleHelp { description: "Simple GET request", command: "recon https://httpbin.org/get" },
        ExampleHelp { description: "POST a JSON body", command: "recon https://httpbin.org/post -d '{\"name\": \"alice\"}' -H \"Content-Type: application/json\"" },
        ExampleHelp { description: "PUT with explicit method", command: "recon https://httpbin.org/put -X PUT -d '{\"active\": true}'" },
        ExampleHelp { description: "Send body from a file", command: "recon https://api.example.com/upload -d @payload.json -H \"Content-Type: application/json\"" },
        ExampleHelp { description: "Follow redirects and show each hop", command: "recon http://github.com --LHEAD" },
        ExampleHelp { description: "Basic auth on a self-signed server", command: "recon https://staging.internal/api -u alice:s3cr3t -k" },
    ],
};

static TOPIC_OUTPUT: Topic = Topic {
    title: "Output Control",
    description: "Control what recon prints and where it goes. By default only the response body\n\
                  is written to stdout. Flags let you include headers, show only the status code,\n\
                  prettify structured data, save to a file, or suppress output entirely.",
    flags: &[
        FlagHelp { flags: "-i, --include", description: "Print response headers before the body." },
        FlagHelp { flags: "-I, --head", description: "Print only the response headers; suppress the body entirely." },
        FlagHelp { flags: "--full", description: "Print the status line, all response headers, and the body." },
        FlagHelp { flags: "-S, --status", description: "Print only the HTTP status code (e.g. 200, 404)." },
        FlagHelp { flags: "-p, --prettify", description: "Prettify the response body. Auto-detects JSON, XML, HTML, YAML, CSV, and TSV.\nCombines with -i and --full." },
        FlagHelp { flags: "-v, --verbose", description: "Show connection info and request/response headers on stderr.\nUse -vv for TLS certificate summary, auth detail, and elapsed time." },
        FlagHelp { flags: "-s, --silent", description: "Suppress informational and progress output.\nThe response body is still printed unless -o is used." },
        FlagHelp { flags: "-o, --output <FILE>", description: "Write the response body to a file instead of stdout." },
        FlagHelp { flags: "--progress", description: "Show a progress meter when saving to a file with -o.\nOpt-in only; never shown by default." },
        FlagHelp { flags: "--FULL-ERRORS", description: "Print the full internal error chain instead of a friendly message.\nUseful for debugging unexpected failures." },
    ],
    related: &["-L / --location", "-f / --fail"],
    examples: &[
        ExampleHelp { description: "Print just the status code", command: "recon https://httpbin.org/get -S" },
        ExampleHelp { description: "Include response headers", command: "recon https://httpbin.org/get -i" },
        ExampleHelp { description: "Prettify JSON output", command: "recon https://httpbin.org/get -p" },
        ExampleHelp { description: "Save to file with a progress meter", command: "recon https://example.com/large.zip -o large.zip --progress" },
        ExampleHelp { description: "Verbose mode with full headers", command: "recon https://httpbin.org/get -vv" },
    ],
};

static TOPIC_DNS: Topic = Topic {
    title: "DNS Lookups",
    description: "Query DNS records for a host. By default --dns shows the most common record\n\
                  types (A, AAAA, CNAME, MX, NS, TXT). Use --dns-type to request specific types.\n\
                  Composes with --cert and all email-protection flags.",
    flags: &[
        FlagHelp { flags: "--dns", description: "Enable DNS lookup for the target host.\nShows common record types by default." },
        FlagHelp { flags: "--dns-type <TYPE,...>", description: "\
Comma-separated DNS record types to query.\n\
\n\
  A          IPv4 address\n\
  AAAA       IPv6 address\n\
  CNAME      Canonical name (alias)\n\
  MX         Mail exchange server and priority\n\
  NS         Authoritative name server\n\
  TXT        Text record (SPF, DKIM, verification, etc.)\n\
  SOA        Start of authority (serial, refresh, retry, expire)\n\
  PTR        Reverse DNS (IP to hostname)\n\
  SRV        Service locator (priority, weight, port, target)\n\
  CAA        Certificate authority authorization\n\
  NAPTR      Naming authority pointer (ENUM, SIP routing)\n\
  SSHFP      SSH public key fingerprint\n\
  TLSA       DANE TLS certificate association\n\
  HINFO      Host information (CPU, OS)\n\
  ANAME      Alias for apex/root domain (provider-specific)\n\
\n\
When explicit types are given, empty results and errors are shown\n\
(normally suppressed for default types)." },
    ],
    related: &["--cert", "--spf", "--dmarc"],
    examples: &[
        ExampleHelp { description: "Look up common DNS records", command: "recon example.com --dns" },
        ExampleHelp { description: "Query specific record types", command: "recon example.com --dns --dns-type A,AAAA,MX" },
        ExampleHelp { description: "Query a single type", command: "recon example.com --dns --dns-type TXT" },
        ExampleHelp { description: "Reverse DNS lookup", command: "recon 8.8.8.8 --dns --dns-type PTR" },
        ExampleHelp { description: "Combine DNS with certificate inspection", command: "recon example.com --dns --cert" },
    ],
};

static TOPIC_CERT: Topic = Topic {
    title: "TLS Certificate Inspection",
    description: "Fetch and display the server's TLS certificate without making a full HTTP\n\
                  request. Shows subject, issuer, validity dates, SANs, key type, and chain\n\
                  details. Works with expired, self-signed, or hostname-mismatched certificates\n\
                  because verification is intentionally skipped during inspection.",
    flags: &[
        FlagHelp { flags: "--cert", description: "Connect to the target over TLS and display the certificate.\nWorks on any HTTPS URL or host:port. Verification is skipped so you\ncan inspect broken or self-signed certs." },
    ],
    related: &["--dns", "-k / --insecure"],
    examples: &[
        ExampleHelp { description: "Inspect a certificate", command: "recon example.com --cert" },
        ExampleHelp { description: "Non-standard TLS port", command: "recon example.com:8443 --cert" },
        ExampleHelp { description: "From a full URL", command: "recon https://example.com --cert" },
        ExampleHelp { description: "Combine with DNS lookup", command: "recon example.com --cert --dns" },
    ],
};

static TOPIC_WHOIS: Topic = Topic {
    title: "WHOIS Lookup",
    description: "Perform a WHOIS lookup for a domain name or IP address. Follows the full\n\
                  referral chain from IANA through the registry to the registrar, showing\n\
                  registrant, dates, nameservers, and status codes.",
    flags: &[
        FlagHelp { flags: "--whois", description: "Run a WHOIS query for the target domain or IP.\nFollows referral chains automatically." },
    ],
    related: &["--dns"],
    examples: &[
        ExampleHelp { description: "WHOIS for a domain", command: "recon example.com --whois" },
        ExampleHelp { description: "WHOIS for an IPv4 address", command: "recon 8.8.8.8 --whois" },
        ExampleHelp { description: "WHOIS for an IPv6 address", command: "recon 2606:4700:: --whois" },
    ],
};

static TOPIC_PING: Topic = Topic {
    title: "Ping",
    description: "Ping a host using ICMP or TCP. When no port is specified, recon sends ICMP\n\
                  echo requests (no root required on macOS). When a port is given (e.g.\n\
                  host:443), it performs a TCP connect/disconnect ping on that port.",
    flags: &[
        FlagHelp { flags: "--ping", description: "Ping the target host.\nICMP if no port, TCP if a port is given." },
        FlagHelp { flags: "--ping-count <N>", description: "Number of ping probes to send (default: 4)." },
    ],
    related: &["--traceroute"],
    examples: &[
        ExampleHelp { description: "ICMP ping", command: "recon example.com --ping" },
        ExampleHelp { description: "TCP ping on port 443", command: "recon example.com:443 --ping" },
        ExampleHelp { description: "Send 10 probes", command: "recon example.com --ping --ping-count 10" },
    ],
};

static TOPIC_TRACEROUTE: Topic = Topic {
    title: "Traceroute",
    description: "Trace the network path to a host. Wraps the system traceroute command.\n\
                  When a port is specified in the target address, it is passed to traceroute\n\
                  via the -p flag. Use --max-hops to limit the number of hops.",
    flags: &[
        FlagHelp { flags: "--traceroute / --trace", description: "Trace the route to the target host.\n--trace is a short alias for --traceroute." },
        FlagHelp { flags: "--max-hops <N>", description: "Maximum number of hops (default: 30)." },
    ],
    related: &["--ping"],
    examples: &[
        ExampleHelp { description: "Basic traceroute", command: "recon example.com --traceroute" },
        ExampleHelp { description: "Using the short alias", command: "recon example.com --trace" },
        ExampleHelp { description: "Trace to a specific port", command: "recon example.com:443 --traceroute" },
        ExampleHelp { description: "Limit hops", command: "recon example.com --traceroute --max-hops 15" },
    ],
};

static TOPIC_SPF: Topic = Topic {
    title: "SPF Validation",
    description: "Validate the Sender Policy Framework (SPF) record for a domain. Recursively\n\
                  resolves include: and redirect= mechanisms, builds a tree of all lookups,\n\
                  counts DNS lookups against the RFC 7208 limit of 10, and warns about common\n\
                  misconfigurations such as multiple SPF records or overly permissive policies.",
    flags: &[
        FlagHelp { flags: "--spf", description: "Validate the SPF record for the target domain.\nRecursively resolves include: and redirect= chains.\nEnforces the 10-lookup limit and reports warnings for\nmultiple records, +all, and other issues." },
    ],
    related: &["--dmarc", "--dkim", "--dns"],
    examples: &[
        ExampleHelp { description: "Validate SPF for a domain", command: "recon example.com --spf" },
        ExampleHelp { description: "SPF with DMARC", command: "recon example.com --spf --dmarc" },
        ExampleHelp { description: "Full email audit", command: "recon example.com --spf --dmarc --dkim default --mta-sts --tls-rpt" },
    ],
};

static TOPIC_DMARC: Topic = Topic {
    title: "DMARC Validation",
    description: "Validate the DMARC (Domain-based Message Authentication, Reporting and\n\
                  Conformance) record for a domain. Checks the policy (none/quarantine/reject),\n\
                  subdomain policy, alignment modes (relaxed/strict) for SPF and DKIM, percentage\n\
                  tag, reporting URIs (rua/ruf), and external report authorization. Cross-validates\n\
                  with SPF and DKIM when those flags are also present.",
    flags: &[
        FlagHelp { flags: "--dmarc", description: "Validate the DMARC record at _dmarc.<domain>.\nChecks policy strength, subdomain policy, SPF/DKIM alignment,\npercentage tag, reporting URIs (rua/ruf), external report\nauthorization, and cross-validates with other email flags." },
    ],
    related: &["--spf", "--dkim", "--bimi"],
    examples: &[
        ExampleHelp { description: "Validate DMARC", command: "recon example.com --dmarc" },
        ExampleHelp { description: "DMARC with SPF and DKIM", command: "recon example.com --dmarc --spf --dkim default" },
        ExampleHelp { description: "Full email protection check", command: "recon example.com --dmarc --spf --dkim default --bimi --mta-sts --tls-rpt" },
    ],
};

static TOPIC_DKIM: Topic = Topic {
    title: "DKIM Validation",
    description: "Validate DomainKeys Identified Mail (DKIM) records for one or more selectors.\n\
                  Each selector is queried at <selector>._domainkey.<domain>. Reports key type\n\
                  (RSA or Ed25519), RSA key size, hash algorithms, service type, and testing/\n\
                  strict flags. The --dkim flag is repeatable to check multiple selectors in\n\
                  one invocation.",
    flags: &[
        FlagHelp { flags: "--dkim <SELECTOR>", description: "Validate the DKIM record for the given selector.\nRepeatable: --dkim google --dkim default.\nReports key type, RSA key size, hash algorithms, service type,\nand flags (testing, strict)." },
    ],
    related: &["--dmarc"],
    examples: &[
        ExampleHelp { description: "Check a single DKIM selector", command: "recon google.com --dkim google" },
        ExampleHelp { description: "Check multiple selectors", command: "recon google.com --dkim google --dkim default" },
        ExampleHelp { description: "DKIM with DMARC cross-validation", command: "recon example.com --dkim selector1 --dmarc" },
    ],
};

static TOPIC_MTA_STS: Topic = Topic {
    title: "MTA-STS Validation",
    description: "Validate the MTA-STS (SMTP MTA Strict Transport Security) configuration for\n\
                  a domain. Checks both the DNS TXT record at _mta-sts.<domain> and the HTTPS\n\
                  policy file at https://mta-sts.<domain>/.well-known/mta-sts.txt. Validates\n\
                  mode (enforce/testing/none), max_age, and MX hostname patterns. Use -k to\n\
                  skip TLS verification when fetching the policy file.",
    flags: &[
        FlagHelp { flags: "--mta-sts", description: "Validate MTA-STS DNS record and HTTPS policy.\nFetches the policy from https://mta-sts.<domain>/.well-known/mta-sts.txt.\nChecks mode (enforce/testing/none), max_age, and MX patterns.\nUse -k / --insecure to skip TLS verification on the policy fetch." },
    ],
    related: &["--tls-rpt", "--dns"],
    examples: &[
        ExampleHelp { description: "Validate MTA-STS", command: "recon example.com --mta-sts" },
        ExampleHelp { description: "MTA-STS with TLS-RPT", command: "recon example.com --mta-sts --tls-rpt" },
        ExampleHelp { description: "Skip TLS verification on policy fetch", command: "recon example.com --mta-sts -k" },
    ],
};

static TOPIC_BIMI: Topic = Topic {
    title: "BIMI Validation",
    description: "Validate the Brand Indicators for Message Identification (BIMI) record for\n\
                  a domain. Queries the TXT record at <selector>._bimi.<domain> (default\n\
                  selector: \"default\"). Checks that the logo URL points to an SVG served over\n\
                  HTTPS and validates the VMC (Verified Mark Certificate) if present. BIMI\n\
                  requires a DMARC policy of quarantine or reject to be effective.",
    flags: &[
        FlagHelp { flags: "--bimi [SELECTOR]", description: "Validate the BIMI record. Optional selector argument\n(default: \"default\").\nChecks logo URL (must be SVG over HTTPS) and VMC certificate.\nNotes DMARC policy dependency." },
    ],
    related: &["--dmarc"],
    examples: &[
        ExampleHelp { description: "Check with default selector", command: "recon example.com --bimi" },
        ExampleHelp { description: "Check with a custom selector", command: "recon example.com --bimi myselector" },
        ExampleHelp { description: "BIMI with DMARC validation", command: "recon example.com --bimi --dmarc" },
    ],
};

static TOPIC_TLS_RPT: Topic = Topic {
    title: "TLS-RPT Validation",
    description: "Validate the SMTP TLS Reporting (TLS-RPT) record for a domain. Queries the\n\
                  TXT record at _smtp._tls.<domain>. Checks the version tag (v=TLSRPTv1),\n\
                  reporting URIs (rua), and validates mailto: and https: URI formats. Notes\n\
                  MTA-STS co-presence — TLS-RPT is most useful when MTA-STS is also deployed.",
    flags: &[
        FlagHelp { flags: "--tls-rpt", description: "Validate the TLS-RPT record at _smtp._tls.<domain>.\nChecks version tag (v=TLSRPTv1), reporting URIs (rua),\nmailto: and https: URI formats, and MTA-STS co-presence." },
    ],
    related: &["--mta-sts"],
    examples: &[
        ExampleHelp { description: "Validate TLS-RPT", command: "recon example.com --tls-rpt" },
        ExampleHelp { description: "TLS-RPT with MTA-STS", command: "recon example.com --tls-rpt --mta-sts" },
    ],
};

static TOPIC_EMAIL: Topic = Topic {
    title: "Email Protection Overview",
    description: "Recon can validate all major email authentication and security standards in a\n\
                  single invocation. Each check can be run independently or composed together.\n\
                  When multiple checks run together, they cross-reference each other (e.g.\n\
                  BIMI notes DMARC policy strength, MTA-STS and TLS-RPT note co-presence).",
    flags: &[
        FlagHelp { flags: "--spf", description: "Validate SPF record. See: recon --help spf" },
        FlagHelp { flags: "--dmarc", description: "Validate DMARC record. See: recon --help dmarc" },
        FlagHelp { flags: "--dkim <SELECTOR>", description: "Validate DKIM record. See: recon --help dkim" },
        FlagHelp { flags: "--mta-sts", description: "Validate MTA-STS. See: recon --help mta-sts" },
        FlagHelp { flags: "--bimi [SELECTOR]", description: "Validate BIMI record. See: recon --help bimi" },
        FlagHelp { flags: "--tls-rpt", description: "Validate TLS-RPT record. See: recon --help tls-rpt" },
    ],
    related: &["--cert", "--dns"],
    examples: &[
        ExampleHelp { description: "Run all email protection checks", command: "recon example.com --spf --dmarc --dkim default --mta-sts --bimi --tls-rpt" },
        ExampleHelp { description: "Quick SPF + DMARC check", command: "recon example.com --spf --dmarc" },
        ExampleHelp { description: "Full domain audit with cert and DNS", command: "recon example.com --cert --dns --spf --dmarc --dkim default --mta-sts --tls-rpt" },
    ],
};

static TOPIC_COOKIES: Topic = Topic {
    title: "Cookie Jar",
    description: "Manage HTTP cookies across requests using named cookie jars. Cookies received\n\
                  from servers are automatically stored and sent back on subsequent requests to\n\
                  matching domains. Jars are stored as SQLite databases in ~/.recon/jars/.\n\
                  You can also list, set, and delete cookies manually.",
    flags: &[
        FlagHelp { flags: "--cookiejar [NAME]", description: "Use a named cookie jar for the request. Cookies are stored in\n~/.recon/jars/<name>.db. Omit the name to use the \"default\" jar.\nYou can also pass an absolute or relative .db path." },
        FlagHelp { flags: "--cookies", description: "List all cookies in the jar. Requires --cookiejar." },
        FlagHelp { flags: "--cookie-set <COOKIE>", description: "Add or update a cookie manually. Requires --cookiejar.\nFormat: \"name=value; Domain=example.com; [Path=/]; [Secure]; [HttpOnly]; [Max-Age=N]\"" },
        FlagHelp { flags: "--cookie-delete <ID>", description: "Delete a cookie by its numeric ID. Requires --cookiejar.\nRun --cookies first to see IDs." },
    ],
    related: &["-u / --user"],
    examples: &[
        ExampleHelp { description: "Login and save cookies", command: "recon https://example.com/login -X POST -d \"user=alice&pass=s3cr3t\" --cookiejar mysession" },
        ExampleHelp { description: "Use saved cookies for a request", command: "recon https://example.com/dashboard --cookiejar mysession" },
        ExampleHelp { description: "List cookies in a jar", command: "recon --cookiejar mysession --cookies" },
        ExampleHelp { description: "Manually set a cookie", command: "recon --cookiejar mysession --cookie-set \"session=abc123; Domain=example.com; Path=/; HttpOnly\"" },
        ExampleHelp { description: "Delete a cookie by ID", command: "recon --cookiejar mysession --cookie-delete 3" },
    ],
};

static TOPIC_SCP: Topic = Topic {
    title: "SCP Download",
    description: "Download files from a remote server over SCP (SSH). Authentication methods\n\
                  tried in order: SSH agent, explicit key (--ssh-key), default key files\n\
                  (~/.ssh/id_ed25519, id_rsa, etc.), and password (-u user:pass or --ssh-pass).\n\
                  The file is saved using the remote basename in the current directory unless\n\
                  -o specifies a different path.",
    flags: &[
        FlagHelp { flags: "scp://<user@>host<:port>/path", description: "SCP URL format. User and port are optional.\nExamples: scp://server/path, scp://user@server:2222/path" },
        FlagHelp { flags: "--ssh-key <PATH>", description: "Path to the SSH private key file for authentication." },
        FlagHelp { flags: "--ssh-pubkey <PATH>", description: "Path to the SSH public key file. Optional;\nderived from --ssh-key by appending .pub if omitted." },
        FlagHelp { flags: "--ssh-pass <PASS>", description: "Passphrase for the SSH private key, or the login password\nfor SSH password authentication." },
        FlagHelp { flags: "-k, --insecure", description: "Skip SSH host-key verification (~/.ssh/known_hosts).\nUse only on hosts you control." },
        FlagHelp { flags: "-o, --output <PATH>", description: "Save the file to a specific path. If a directory,\nthe remote filename is preserved inside it." },
        FlagHelp { flags: "--progress", description: "Show a progress meter during the download." },
    ],
    related: &["-u / --user"],
    examples: &[
        ExampleHelp { description: "Download with SSH agent auth", command: "recon scp://server/home/user/file.tgz" },
        ExampleHelp { description: "Explicit user in URL", command: "recon scp://thomas@server/home/thomas/file.tgz" },
        ExampleHelp { description: "Non-standard SSH port", command: "recon scp://thomas@server:2222/home/thomas/file.tgz" },
        ExampleHelp { description: "Explicit SSH key", command: "recon scp://server/file.tgz --ssh-key ~/.ssh/id_deploy" },
        ExampleHelp { description: "Save to a specific path with progress", command: "recon scp://server/backup.tar.gz -o /backups/ --progress" },
    ],
};

static TOPIC_SERVE: Topic = Topic {
    title: "HTTP Server",
    description: "Start a static file server serving the current directory over HTTP. Directory\n\
                  listings are returned as HTML for browsers and plain text for curl. Files are\n\
                  served with MIME type detection. Access is logged to the terminal and optionally\n\
                  to a file with --serve-log.",
    flags: &[
        FlagHelp { flags: "--serve [PORT]", description: "Start an HTTP server on the given port (default: 80).\nServes the current directory. Directory listings are auto-generated." },
        FlagHelp { flags: "--serve-log <PATH>", description: "Write access log entries to the given file path in addition to stdout." },
    ],
    related: &["--serve-tls"],
    examples: &[
        ExampleHelp { description: "Serve on port 8080", command: "recon --serve 8080" },
        ExampleHelp { description: "Serve on default port 80", command: "recon --serve" },
        ExampleHelp { description: "Serve with a log file", command: "recon --serve 8080 --serve-log access.log" },
        ExampleHelp { description: "Serve HTTP and HTTPS together", command: "recon --serve 8080 --serve-tls 8443" },
    ],
};

static TOPIC_SERVE_TLS: Topic = Topic {
    title: "HTTPS Server",
    description: "Start a static file server over HTTPS with TLS. Supports HTTP/1.1 and HTTP/2\n\
                  via ALPN negotiation. Requires a certificate and private key in PEM format.\n\
                  \n\
                  recon looks for TLS certificates in ~/.recon/ by default:\n\
                    ~/.recon/cert.pem    Certificate file\n\
                    ~/.recon/key.pem     Private key file\n\
                  \n\
                  To generate certificates for local development:\n\
                  \n\
                  Option 1 — mkcert (recommended, browsers trust it automatically):\n\
                    mkcert -install\n\
                    mkcert -key-file ~/.recon/key.pem -cert-file ~/.recon/cert.pem localhost 127.0.0.1 ::1\n\
                  \n\
                  Option 2 — openssl (self-signed, browsers will show a warning):\n\
                    openssl req -x509 -newkey rsa:2048 -keyout ~/.recon/key.pem \\\n\
                      -out ~/.recon/cert.pem -days 365 -nodes -subj \"/CN=localhost\"",
    flags: &[
        FlagHelp { flags: "--serve-tls [PORT]", description: "Start an HTTPS server on the given port (default: 443).\nServes the current directory with TLS." },
        FlagHelp { flags: "--http-version <VERSION>", description: "HTTP protocol version to advertise via ALPN: 1.1 or 2.\nDefaults to auto (negotiates the best version with the client)." },
        FlagHelp { flags: "--serve-cert <PATH>", description: "Path to the TLS certificate PEM file (default: ~/.recon/cert.pem)." },
        FlagHelp { flags: "--serve-key <PATH>", description: "Path to the TLS private key PEM file (default: ~/.recon/key.pem)." },
        FlagHelp { flags: "--serve-log <PATH>", description: "Write access log entries to the given file path in addition to stdout." },
        FlagHelp { flags: "--serve-sni <MAPPING>", description: "\
SNI hostname-to-certificate mapping (repeatable).\n\
Three formats are auto-detected:\n\
\n\
  Inline:     --serve-sni \"myapp.local:cert.pem:key.pem\"\n\
  Directory:  --serve-sni ~/.recon/sni/\n\
              (files named <hostname>-cert.pem and <hostname>-key.pem)\n\
  Config:     --serve-sni sni.conf\n\
              (lines: hostname cert.pem key.pem)\n\
\n\
Implies --serve-tls (port 443) if not explicitly given.\n\
Multiple values can be mixed. Unmatched hostnames use the\n\
default cert or reject the connection." },
    ],
    related: &["--serve"],
    examples: &[
        ExampleHelp { description: "Serve HTTPS on port 8443", command: "recon --serve-tls 8443" },
        ExampleHelp { description: "Force HTTP/2", command: "recon --serve-tls 8443 --http-version 2" },
        ExampleHelp { description: "Use custom certificates", command: "recon --serve-tls 8443 --serve-cert ./cert.pem --serve-key ./key.pem" },
        ExampleHelp { description: "Serve HTTP and HTTPS together", command: "recon --serve 8080 --serve-tls 8443" },
        ExampleHelp { description: "Generate trusted cert with mkcert (recommended)", command: "mkcert -install && mkcert -key-file ~/.recon/key.pem -cert-file ~/.recon/cert.pem localhost 127.0.0.1 ::1" },
        ExampleHelp { description: "Generate self-signed cert with openssl", command: "openssl req -x509 -newkey rsa:2048 -keyout ~/.recon/key.pem -out ~/.recon/cert.pem -days 365 -nodes -subj \"/CN=localhost\"" },
        ExampleHelp { description: "SNI: different certs per hostname", command: "recon --serve-sni \"myapp.local:certs/myapp.pem:certs/myapp-key.pem\" --serve-sni \"api.local:certs/api.pem:certs/api-key.pem\"" },
        ExampleHelp { description: "SNI: from a certificate directory", command: "recon --serve-sni ~/.recon/sni/" },
    ],
};

// ── Topic resolution ─────────────────────────────────────────────────────────

fn resolve_topic(key: &str) -> Option<&'static Topic> {
    match key.to_lowercase().as_str() {
        "http" | "https" => Some(&TOPIC_HTTP),
        "output" => Some(&TOPIC_OUTPUT),
        "dns" => Some(&TOPIC_DNS),
        "cert" | "tls" | "certificate" => Some(&TOPIC_CERT),
        "whois" => Some(&TOPIC_WHOIS),
        "ping" => Some(&TOPIC_PING),
        "traceroute" | "trace" => Some(&TOPIC_TRACEROUTE),
        "spf" => Some(&TOPIC_SPF),
        "dmarc" => Some(&TOPIC_DMARC),
        "dkim" => Some(&TOPIC_DKIM),
        "mta-sts" | "mtasts" => Some(&TOPIC_MTA_STS),
        "bimi" => Some(&TOPIC_BIMI),
        "tls-rpt" | "tlsrpt" => Some(&TOPIC_TLS_RPT),
        "email" | "email-protection" => Some(&TOPIC_EMAIL),
        "cookies" | "cookiejar" | "cookie" => Some(&TOPIC_COOKIES),
        "scp" | "ssh" => Some(&TOPIC_SCP),
        "serve" | "server" => Some(&TOPIC_SERVE),
        "serve-tls" | "serve-https" | "https-server" => Some(&TOPIC_SERVE_TLS),
        _ => None,
    }
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Resolve and render a help topic. Returns true if the topic was found.
pub fn print_topic(topic: &str) -> bool {
    match resolve_topic(topic) {
        Some(t) => {
            render_topic(t);
            true
        }
        None => false,
    }
}

/// Print an error message for an unknown topic, followed by the topic footer.
pub fn print_unknown_topic(topic: &str) {
    eprintln!("{}", format!("Unknown topic: {topic}").red().bold());
    eprintln!();
    print_topic_footer();
}

/// Print the footer listing available topics and the --help <topic> hint.
pub fn print_topic_footer() {
    println!(
        "{}",
        "For detailed help on a specific topic: recon --help <topic>".dimmed()
    );
    let topics = topic_keys().join(", ");
    println!("{}", format!("Topics: {topics}").dimmed());
}

/// Returns the primary topic keys in display order.
pub fn topic_keys() -> Vec<&'static str> {
    vec![
        "http",
        "output",
        "dns",
        "cert",
        "whois",
        "ping",
        "traceroute",
        "spf",
        "dmarc",
        "dkim",
        "mta-sts",
        "bimi",
        "tls-rpt",
        "email",
        "cookies",
        "scp",
        "serve",
        "serve-tls",
    ]
}

// ── Rendering ────────────────────────────────────────────────────────────────

fn render_topic(topic: &Topic) {
    println!();
    println!("{}", format!("recon — {}", topic.title).bold());
    println!();

    for line in topic.description.lines() {
        println!("  {line}");
    }
    println!();

    // FLAGS
    println!("  {}", "FLAGS".yellow().bold());
    println!();
    for flag in topic.flags {
        println!("    {}", flag.flags.bold());
        for line in flag.description.lines() {
            println!("      {line}");
        }
        println!();
    }

    // RELATED FLAGS
    if !topic.related.is_empty() {
        println!("  {}", "RELATED FLAGS".yellow().bold());
        println!();
        for item in topic.related {
            println!("    {}", item.dimmed());
        }
        println!();
    }

    // EXAMPLES
    if !topic.examples.is_empty() {
        println!("  {}", "EXAMPLES".yellow().bold());
        println!();
        for ex in topic.examples {
            println!("    {}", ex.description.bold());
            println!("      {}", ex.command.cyan());
            println!();
        }
    }
}
