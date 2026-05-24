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
        FlagHelp { flags: "--tlsv1.2 / --tlsv1.3", description: "Force a minimum TLS version for HTTPS. If both are set,\n--tlsv1.3 wins. Curl-compatible spelling." },
        FlagHelp { flags: "--cacert <PATH>", description: "Trust an additional PEM root certificate (on top of the system roots).\nUse for self-signed corporate / internal CAs without -k." },
        FlagHelp { flags: "--capath <DIR>", description: "Directory of .pem/.crt/.cer root certificates. Each file is loaded\nand added as a trust root (mirrors --cacert for a directory)." },
        FlagHelp { flags: "--ca-native", description: "Disable built-in webpki roots and use the OS native trust store only.\nUseful when the system certificate bundle is managed centrally." },
        FlagHelp { flags: "--crlfile <PATH>", description: "PEM file with X.509 CRLs. Server certs in any loaded\nCRL are rejected during TLS handshake. Multi-CRL bundles\nsupported via from_pem_bundle." },
        FlagHelp { flags: "--proxy-capath <DIR>", description: "Directory of .pem/.crt/.cer CA files for proxy TLS\nverification. Mirrors --capath." },
        FlagHelp { flags: "--proxy-ca-native", description: "Disable built-in webpki roots; use OS native roots only.\nMirrors --ca-native (same global toggle, separate flag\nfor curl-parity)." },
        FlagHelp { flags: "--interface <IP>", description: "Bind outgoing sockets to a local IP (IPv4 or IPv6 literal).\nInterface names (eth0, en0) not yet resolved; pass the address directly." },
        FlagHelp { flags: "--limit-rate <RATE>", description: "Throttle download speed. Accepts 100K, 2M, 1.5G, or bare bytes\n(curl-compatible suffixes). B/b = bytes (explicit)." },
        FlagHelp { flags: "--speed-limit <BYTES>", description: "Minimum download bytes-per-second. Combined with --speed-time,\naborts when the rolling rate stays below this floor for the given\nwindow. Useful for failing fast on stalled downloads." },
        FlagHelp { flags: "--speed-time <SECS>", description: "Window in seconds for --speed-limit (default 30)." },
        FlagHelp { flags: "-A, --user-agent <STRING>", description: "Custom User-Agent header value." },
        FlagHelp { flags: "-f, --fail", description: "Exit with non-zero status on HTTP 4xx/5xx responses.\nNo output is printed for the error response body." },
        FlagHelp { flags: "-e, --referer <URL>", description: "Send a Referer header. Also accepts --referrer.\nAn explicit -H \"Referer: …\" takes precedence over this flag." },
        FlagHelp { flags: "-O, --remote-name", description: "Save response body to a file named after the URL's final path segment.\nPercent-decodes the name. Mutually exclusive with -o/--output." },
        FlagHelp { flags: "-T, --upload-file <PATH>", description: "Upload local file as request body.\nDefaults method to PUT unless -X is set explicitly. Mutually exclusive with -d/--data." },
        FlagHelp { flags: "--json <DATA>", description: "Send DATA as a JSON body.\nAuto-sets Content-Type: application/json and Accept: application/json unless overridden by -H. Supports @file and @- (stdin) like -d." },
        FlagHelp { flags: "--data-raw <DATA>", description: "Like -d but @file is NOT processed — sends the literal string (including any leading @)." },
        FlagHelp { flags: "--data-binary <DATA>", description: "Like -d but CR/LF are NOT stripped from @file content." },
        FlagHelp { flags: "--data-urlencode <DATA>", description: "URL-encode DATA for an x-www-form-urlencoded body.\nRepeatable; values joined with &.\nSub-forms: content | =content | name=content | @file | name@file." },
        FlagHelp { flags: "--compressed", description: "Request gzip / deflate / brotli / zstd encoding and auto-decompress the response body." },
        FlagHelp { flags: "--max-time <SECS>", description: "Total operation timeout (DNS + TLS + request + body) in seconds.\nAccepts fractional values (e.g. 0.5). Exit code 28 on timeout." },
    ],
    related: &["--cert", "--cookiejar", "-p / --prettify"],
    examples: &[
        ExampleHelp { description: "Simple GET request", command: "recon https://httpbin.org/get" },
        ExampleHelp { description: "POST a JSON body", command: "recon https://httpbin.org/post -d '{\"name\": \"alice\"}' -H \"Content-Type: application/json\"" },
        ExampleHelp { description: "PUT with explicit method", command: "recon https://httpbin.org/put -X PUT -d '{\"active\": true}'" },
        ExampleHelp { description: "Send body from a file", command: "recon https://api.example.com/upload -d @payload.json -H \"Content-Type: application/json\"" },
        ExampleHelp { description: "Follow redirects and show each hop", command: "recon http://github.com --LHEAD" },
        ExampleHelp { description: "Basic auth on a self-signed server", command: "recon https://staging.internal/api -u alice:s3cr3t -k" },
        ExampleHelp { description: "JSON shorthand (auto-sets Content-Type and Accept)", command: "recon --json '{\"q\":\"rust\"}' https://api.example.com/search" },
        ExampleHelp { description: "URL-encode form fields", command: "recon --data-urlencode \"name=Jane Doe\" --data-urlencode \"city=New York\" https://httpbin.org/post" },
        ExampleHelp { description: "Compressed response", command: "recon --compressed https://httpbin.org/gzip" },
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
        FlagHelp { flags: "--prettify-as <FORMAT>", description: "Force prettify format (json|xml|html|yaml|csv|tsv|auto).\nImplies -p. Use when auto-detection guesses wrong\nor when the input lacks a Content-Type header." },
        FlagHelp { flags: "--stdin", description: "Read body from stdin instead of making an HTTP request.\nRuns the post-fetch pipeline (prettify, --output-charset, -o) over\nthe piped input. Mutually exclusive with a URL.\nExample: pbpaste | recon --stdin --prettify-as json" },
        FlagHelp { flags: "--from-clipboard", description: "Read body from system clipboard (no HTTP request).\nMutex with --stdin and a URL. macOS uses pasteboard;\nLinux uses X11 (with optional Wayland data-control)." },
        FlagHelp { flags: "--to-clipboard", description: "Write output to system clipboard. Mutex with -o and --editor.\nText only — non-UTF-8 output errors out." },
        FlagHelp { flags: "--clipboard <DIR>", description: "Use clipboard for I/O. DIR=in|out|both.\nBare --clipboard auto-resolves: 'out' when an input is given\n(URL/--stdin/etc.), 'in' otherwise." },
        FlagHelp { flags: "-v, --verbose", description: "Show connection info and request/response headers on stderr.\nUse -vv for TLS certificate summary, auth detail, and elapsed time." },
        FlagHelp { flags: "-s, --silent", description: "Suppress informational and progress output.\nThe response body is still printed unless -o is used." },
        FlagHelp { flags: "-o, --output <FILE>", description: "Write the response body to a file instead of stdout." },
        FlagHelp { flags: "--progress", description: "Show a progress meter when saving to a file with -o.\nOpt-in only; never shown by default." },
        FlagHelp { flags: "-O, --remote-name-all", description: "Apply -O (filename from URL) to every URL in --input-file.\nCurl-parity for multi-URL batch downloads." },
        FlagHelp { flags: "-#, --progress-bar", description: "Use # progress bar style (curl -# parity).\nAlso activates the progress meter — no separate --progress needed." },
        FlagHelp { flags: "--FULL-ERRORS", description: "Print the full internal error chain instead of a friendly message.\nUseful for debugging unexpected failures." },
        FlagHelp { flags: "--fail-with-body", description: "Like -f but keeps the response body.\nWrites the body to stdout (or -o / -O destination) then exits non-zero." },
        FlagHelp { flags: "--create-dirs", description: "Create missing parent directories for the -o / -O output path." },
        FlagHelp { flags: "--output-dir <DIR>", description: "Prefix for -o and -O output paths.\nE.g. `--output-dir ./dl -o file.txt` writes to ./dl/file.txt." },
        FlagHelp { flags: "-J, --remote-header-name", description: "With -O, derive the filename from the response Content-Disposition header (RFC 6266).\nFalls back to the URL basename if the header is missing or malformed. Rejects path traversal and Windows-reserved names." },
        FlagHelp { flags: "--remote-time", description: "Apply the response Last-Modified header as the mtime of the saved file." },
        FlagHelp { flags: "-w, --write-out <FORMAT>", description: "Print FORMAT after the response.\nSupports %{var}, %{header{name}}, %{json}, %{stderr}, %{stdout}, and \\n \\t \\r \\\\ escapes.\nLoad from file with @path or stdin with @-.\nSee --help write-out for the full variable list." },
    ],
    related: &["-L / --location", "-f / --fail"],
    examples: &[
        ExampleHelp { description: "Print just the status code", command: "recon https://httpbin.org/get -S" },
        ExampleHelp { description: "Include response headers", command: "recon https://httpbin.org/get -i" },
        ExampleHelp { description: "Prettify JSON output", command: "recon https://httpbin.org/get -p" },
        ExampleHelp { description: "Prettify a JSON payload from clipboard", command: "pbpaste | recon --stdin --prettify-as json" },
        ExampleHelp { description: "Prettify clipboard contents", command: "recon --clipboard --prettify-as json" },
        ExampleHelp { description: "Prettify clipboard in place", command: "recon --clipboard both --prettify-as json" },
        ExampleHelp { description: "Fetch URL, copy result to clipboard", command: "recon https://api.example.com/data --to-clipboard" },
        ExampleHelp { description: "Auto-detect piped stdin (no --stdin needed)", command: "cat raw.json | recon -p" },
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

        FlagHelp { flags: "--dns-servers <LIST>", description: "Comma-separated list of custom DNS servers for HTTP-request\nname resolution. Accepts `IP` (port 53 implied) or `IP:PORT`.\nExamples: 1.1.1.1,8.8.8.8 or 1.1.1.1:5353,9.9.9.9." },
        FlagHelp { flags: "--dns-ipv4-addr <IP>", description: "Local IPv4 address to bind outgoing DNS queries to. Used with\n--dns-servers (defaults to 1.1.1.1:53 if --dns-servers is unset)." },
        FlagHelp { flags: "--dns-ipv6-addr <IP>", description: "Local IPv6 address to bind outgoing DNS queries to." },
        FlagHelp { flags: "--dns-interface <IFACE>", description: "Named-interface DNS binding (`eth0`, `en0`). Not yet plumbed;\nerrors out. Use --dns-ipv4-addr / --dns-ipv6-addr with the\ninterface's literal address as a workaround." },
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
                  because verification is intentionally skipped during inspection.\n\
                  \n\
                  See also: `recon --help impersonate` -- TLS+H2 browser fingerprint\n\
                  impersonation (opt-in --features impersonate build).",
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
        ExampleHelp { description: "Query the jar from a Rhai script (see --help script)", command: r#"recon --script - # let db = sqlite("cookiejar:mysession"); db.query("SELECT ...")"# },
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

static TOPIC_SSH: Topic = Topic {
    title: "SSH Interactive Shell",
    description: "Open an interactive SSH shell on a remote server. Authentication methods\n\
                  tried in order: SSH agent, explicit key (--ssh-key), default key files\n\
                  (~/.ssh/id_ed25519, id_rsa, etc.), and password (--ssh-pass or -u user:pass).\n\
                  The remote terminal is fully interactive: colours, editors (vim, nano), and\n\
                  TUI applications work correctly. Terminal resize is forwarded automatically.",
    flags: &[
        FlagHelp { flags: "ssh://[user@]host[:port]", description: "SSH URL. User and port are optional.\nDefault port: 22.\nExamples: ssh://server, ssh://alice@server:2222" },
        FlagHelp { flags: "--ssh-key <PATH>", description: "Path to the SSH private key file for authentication." },
        FlagHelp { flags: "--ssh-pubkey <PATH>", description: "Path to the SSH public key file. Optional;\nderived from --ssh-key by appending .pub if omitted." },
        FlagHelp { flags: "--pubkey <PATH>", description: "Path to SSH public key file (alias for --ssh-pubkey).\nWhen both are set, --ssh-pubkey wins." },
        FlagHelp { flags: "--ssh-pass <PASS>", description: "Passphrase for the SSH private key, or the login password\nfor SSH password authentication." },
        FlagHelp { flags: "-u, --user <USER:PASS>", description: "SSH username. Optionally include a password with user:pass.\nThe URL userinfo takes priority if both are given." },
        FlagHelp { flags: "-k, --insecure", description: "Skip SSH host-key verification (~/.ssh/known_hosts).\nUse only on hosts you control or trust." },
    ],
    related: &["scp", "-u / --user"],
    examples: &[
        ExampleHelp { description: "Connect with SSH agent auth", command: "recon ssh://myserver.example.com" },
        ExampleHelp { description: "Explicit user in URL", command: "recon ssh://alice@myserver.example.com" },
        ExampleHelp { description: "Non-standard SSH port", command: "recon ssh://alice@myserver.example.com:2222" },
        ExampleHelp { description: "Explicit key file", command: "recon ssh://myserver.example.com --ssh-key ~/.ssh/id_deploy" },
        ExampleHelp { description: "Password auth", command: "recon ssh://myserver.example.com -u alice:s3cr3t" },
        ExampleHelp { description: "Skip host key check (dev/test only)", command: "recon ssh://dev-server.local --insecure" },
    ],
};

static TOPIC_TELNET: Topic = Topic {
    title: "Telnet Client",
    description: "Connect to a Telnet server with full IAC option negotiation. The client\n\
                  accepts server ECHO and SUPPRESS-GO-AHEAD options (standard for interactive\n\
                  sessions) and rejects all others. Subnegotiation blocks are discarded.\n\
                  Authentication is interactive — the server prompts for credentials via the\n\
                  text stream. Press Ctrl+D to close the connection.",
    flags: &[
        FlagHelp { flags: "telnet://host[:port]", description: "Telnet URL. Port is optional.\nDefault port: 23.\nExamples: telnet://bbs.example.com, telnet://host:8023" },
        FlagHelp { flags: "--connect-timeout <SECS>", description: "TCP connection timeout in seconds (default: 30)." },
    ],
    related: &["ssh"],
    examples: &[
        ExampleHelp { description: "Connect to a Telnet server", command: "recon telnet://bbs.example.com" },
        ExampleHelp { description: "Non-standard port", command: "recon telnet://host:8023" },
        ExampleHelp { description: "Short connection timeout", command: "recon telnet://host --connect-timeout 5" },
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

static TOPIC_NETSTATUS: Topic = Topic {
    title: "Network Status",
    description: "Check connectivity using a set of configurable probes defined in\n\
                  ~/.recon/config.toml under [netstatus]. Probes run concurrently and\n\
                  results are shown with pass/fail markers. Exits non-zero if any check\n\
                  fails, making it suitable for scripting with --silent.",
    flags: &[
        FlagHelp {
            flags: "--netstatus",
            description: "Run all configured probes and display a connectivity report.\n\
                          Reads probe list from ~/.recon/config.toml [netstatus] section.\n\
                          Exit code: 0 = all passed (ONLINE), 1 = any failed (DEGRADED/OFFLINE).",
        },
        FlagHelp {
            flags: "-s, --silent",
            description: "Suppress all output. Only the exit code is set.\n\
                          Useful in shell scripts: recon --netstatus --silent && deploy.sh",
        },
    ],
    related: &["--ping", "--dns", "--cert"],
    examples: &[
        ExampleHelp {
            description: "Check connectivity and show full report",
            command: "recon --netstatus",
        },
        ExampleHelp {
            description: "Use in a script (silent, exit code only)",
            command: "recon --netstatus --silent && echo online || echo offline",
        },
        ExampleHelp {
            description: "Check with DNS hijack detection in config",
            command: "recon --netstatus  # requires [[netstatus.dns_hijack_checks]] in config",
        },
    ],
};

static TOPIC_JWT: Topic = Topic {
    title: "JWT Tokens",
    description: "Sign, validate, and inspect JWT tokens. Input can come from -d (inline string\n\
                  or @file prefix for a file), a filename positional argument (no protocol = local\n\
                  file), or stdin.\n\
                  \n\
                  --jwt-sign accepts JSON, a two-part base64 token (header.payload), or a single\n\
                  base64 payload. --jwt-validate requires a full three-part token.",
    flags: &[
        FlagHelp { flags: "--jwt-view", description: "Decode and display the JWT header and payload as pretty-printed JSON.\nNo signature verification is performed." },
        FlagHelp { flags: "--jwt-sign", description: "Sign or complete a JWT.\n  JSON input → treated as payload.\n  Two-part base64 (header.payload) → header preserved, signature added.\n  Single base64 → treated as bare payload.\nAdds iat = now if missing from payload." },
        FlagHelp { flags: "--jwt-validate", description: "Verify the JWT signature. Without extra flags: signature check only.\nAdd --jwt-validate-* flags to check individual claims.\nExits non-zero if any check fails." },
        FlagHelp { flags: "--jwt-secret <SECRET>", description: "HMAC secret. Required for --jwt-sign and --jwt-validate." },
        FlagHelp { flags: "--jwt-alg <ALG>", description: "Signing algorithm: HS256 (default), HS384, HS512.\nOverrides the alg in an existing token header." },
        FlagHelp { flags: "--jwt-iss/sub/aud/jti <VALUE>", description: "Set the claim when signing (only if not already in the payload).\nAssert the claim value when validating (with --jwt-validate-iss/sub/aud/jti)." },
        FlagHelp { flags: "--jwt-exp/nbf/iat [TIMESTAMP]", description: "Set a timestamp claim when signing (only if absent from payload).\nOmit the value to use current time. When validating, used as the reference\ntime for time-based checks (defaults to now if omitted)." },
        FlagHelp { flags: "--jwt-validate-exp/nbf/iat", description: "Enable time-based claim checks:\n  --jwt-validate-exp: exp must not be in the past.\n  --jwt-validate-nbf: nbf must not be in the future.\n  --jwt-validate-iat: iat must exist and not be in the future." },
        FlagHelp { flags: "--jwt-validate-iss/sub/aud/jti", description: "Enable claim equality checks.\nEach requires the corresponding --jwt-iss/sub/aud/jti flag to supply the expected value." },
        FlagHelp { flags: "--jwt-validate-full", description: "Enable all claim checks at once.\nClaim equality checks (iss, sub, aud, jti) are only run if the\ncorresponding value flag is also provided." },
        FlagHelp { flags: "--jwt-json-report", description: "Output results as a JSON object.\n  --jwt-view → {\"header\":{...},\"payload\":{...}}\n  --jwt-validate → {\"valid\":true,\"checks\":[...]}" },
    ],
    related: &["-d / --data", "--prettify"],
    examples: &[
        ExampleHelp { description: "Sign a JSON payload (iat added automatically)", command: "recon --jwt-sign --jwt-secret mysecret -d '{\"sub\":\"alice\",\"iss\":\"acme\"}'" },
        ExampleHelp { description: "Inspect a token without verification", command: "recon --jwt-view -d <token>" },
        ExampleHelp { description: "Validate signature only", command: "recon --jwt-validate --jwt-secret mysecret -d <token>" },
        ExampleHelp { description: "Validate signature and expiry", command: "recon --jwt-validate --jwt-secret mysecret --jwt-validate-exp -d <token>" },
        ExampleHelp { description: "Full validation with issuer check", command: "recon --jwt-validate --jwt-secret mysecret --jwt-validate-full --jwt-iss acme -d <token>" },
        ExampleHelp { description: "JSON output for scripting", command: "recon --jwt-validate --jwt-secret mysecret --jwt-validate-full --jwt-json-report -d <token>" },
    ],
};

static TOPIC_SAMPLE: Topic = Topic {
    title: "Sample Data",
    description: "Fetch canned ecommerce sample data (customers, products, orders, categories,\n\
                  addresses, images) from known free APIs, or generate local lorem ipsum. All\n\
                  built-ins are overridable in ~/.recon/config.toml, and you can add your own\n\
                  named samples — including paid APIs with Bearer tokens — by defining a\n\
                  [sampledata.<name>] section.\n\
                  \n\
                  Note: because --sample takes a value, place the URL-less form such as\n\
                  --sample customer, --sample customer:csv:25, etc. Use --sample-list to see\n\
                  what's available.",
    flags: &[
        FlagHelp {
            flags: "--sample <NAME[:FORMAT[:COUNT]]>",
            description: "Fetch sample data by name. The colon shortcut lets you set format and\n\
                          count inline; empty slots fall back to defaults. Built-in names:\n\
                          customer, product, order, category, address, image, lorem.",
        },
        FlagHelp {
            flags: "--sample-format <FMT>",
            description: "Override the format (takes precedence over the colon shortcut).",
        },
        FlagHelp {
            flags: "--sample-count <N[p|w|c]>",
            description: "Override the count. Unit suffixes p/w/c are only valid for the\n\
                          local 'lorem' sample. Non-lorem samples error on a unit suffix.",
        },
        FlagHelp {
            flags: "--sample-file [PATH]",
            description: "Write output to file(s). Default filename is\n\
                          sample-{{name}}.{{format}} (bulk) or\n\
                          sample-{{name}}-{{n}}.{{format}} (per_item).\n\
                          Required when per_item sample count > 1.",
        },
        FlagHelp {
            flags: "--sample-list",
            description: "Standalone action: list all available samples (built-in plus\n\
                          user-configured). Does not require a URL.",
        },
        FlagHelp {
            flags: "--sample-seed <N>",
            description: "Seed for lorem ipsum randomization. When omitted, a seed is\n\
                          derived from the current system time. Using this flag with\n\
                          any non-lorem sample is an error.",
        },
    ],
    related: &["--editor", "-o / --output", "-p / --prettify", "-i / --include"],
    examples: &[
        ExampleHelp { description: "10 customers to stdout", command: "recon --sample customer" },
        ExampleHelp { description: "25 products, prettified", command: "recon --sample product --sample-count 25 -p" },
        ExampleHelp { description: "Colon shortcut: 25 customers as JSON", command: "recon --sample customer:json:25" },
        ExampleHelp { description: "Open products in Zed", command: "recon --sample product --editor zed" },
        ExampleHelp { description: "3 random images saved to files", command: "recon --sample image --sample-count 3 --sample-file img-{{n}}.jpg" },
        ExampleHelp { description: "50 words of lorem ipsum", command: "recon --sample lorem --sample-count 50w" },
        ExampleHelp { description: "Reproducible lorem with a seed", command: "recon --sample lorem --sample-count 3p --sample-seed 42" },
        ExampleHelp { description: "List all samples", command: "recon --sample-list" },
    ],
};

static TOPIC_HASH: Topic = Topic {
    title: "Hashing",
    description: "Compute a cryptographic hash of any source — a local file, file:// URL, HTTP(S)\n\
                  URL, or stdin. HTTP sources honour the full HTTP flag set (auth, redirects,\n\
                  TLS options, cookies, referer). Output defaults to lowercase hex and can be\n\
                  switched to base64 or raw bytes with --hash-format.",
    flags: &[
        FlagHelp {
            flags: "--hash <ALGO>",
            description: "Algorithm. Case-insensitive; hyphens and underscores accepted.\n\
                          Supported: md5, sha1, sha256, sha384, sha512,\n\
                          sha3-256, sha3-512, blake3, crc32.",
        },
        FlagHelp {
            flags: "--hash-format <FMT>",
            description: "Output format: hex (default, lowercase + newline),\n\
                          base64 (standard + newline), raw (binary, no newline).",
        },
        FlagHelp {
            flags: "--hash-list",
            description: "Standalone action: list all supported algorithms with digest sizes.\n\
                          Does not require a URL.",
        },
    ],
    related: &["-o / --output", "-H / --header", "-u / --user", "-L / --location", "-e / --referer"],
    examples: &[
        ExampleHelp { description: "sha256 of a local file", command: "recon --hash sha256 ./file.bin" },
        ExampleHelp { description: "sha512 of a remote artifact (with auth)", command: "recon --hash sha512 https://api/artifact -H \"Authorization: Bearer $T\" -L" },
        ExampleHelp { description: "blake3 of stdin", command: "cat data | recon --hash blake3" },
        ExampleHelp { description: "sha256 via file:// scheme", command: "recon --hash sha256 file:///tmp/data.bin" },
        ExampleHelp { description: "Base64 output", command: "recon --hash sha256 ./file --hash-format base64" },
        ExampleHelp { description: "Raw digest bytes piped onward", command: "recon --hash sha256 ./file --hash-format raw > digest.bin" },
        ExampleHelp { description: "CRC32 checksum (4-byte digest shown as 8 hex chars)", command: "recon --hash crc32 ./file.bin" },
        ExampleHelp { description: "List supported algorithms", command: "recon --hash-list" },
    ],
};

static TOPIC_COMPRESSION: Topic = Topic {
    title: "Compression",
    description: "Compress or decompress any source — a local file, file:// URL, HTTP(S) URL,\n\
                  or stdin. Output goes to stdout or -o <FILE>. Auto-detects gzip, zstd,\n\
                  bzip2, lz4, xz, snappy, and zlib inputs by magic bytes; deflate and brotli\n\
                  lack a signature so their algorithm must be named explicitly when\n\
                  decompressing.",
    flags: &[
        FlagHelp {
            flags: "--compress <ALGO>",
            description: "Compress with the named algorithm (case-insensitive; alias accepted).\n\
                          Supported: gzip/gz, deflate, zstd/zst, brotli/br, bzip2/bz2,\n\
                          lz4/lz, xz/lzma, snappy/snap/sz, zlib/zl.",
        },
        FlagHelp {
            flags: "--decompress [ALGO]",
            description: "Decompress. Omit ALGO to auto-detect (gzip, zstd, bzip2, lz4, xz,\n\
                          snappy, zlib by magic bytes). Pass the algorithm for deflate or\n\
                          brotli.",
        },
        FlagHelp {
            flags: "--compression-level <LEVEL>",
            description: "Quality for --compress. Number in the algorithm's native range\n\
                          (gzip/deflate/zlib/xz 0-9, zstd 1-22, brotli 0-11, bzip2 1-9),\n\
                          or a word: fastest, fast, default, good, best. lz4 and snappy\n\
                          have no level setting. Invalid with --decompress.",
        },
        FlagHelp {
            flags: "--compress-list",
            description: "Standalone action: list all supported algorithms with their\n\
                          aliases, magic bytes, and level ranges. Does not require a URL.",
        },
    ],
    related: &["-o / --output", "-H / --header", "-u / --user", "-L / --location"],
    examples: &[
        ExampleHelp { description: "gzip a local file to stdout", command: "recon --compress gzip ./big.log" },
        ExampleHelp { description: "zstd with a strong level to disk", command: "recon --compress zstd --compression-level best ./data -o data.zst" },
        ExampleHelp { description: "Decompress from URL (auto-detect)", command: "recon --decompress https://cdn/file.zst" },
        ExampleHelp { description: "Decompress brotli (explicit algorithm)", command: "recon --decompress brotli ./web-asset.br" },
        ExampleHelp { description: "Compress stdin", command: "cat data | recon --compress gzip > data.gz" },
        ExampleHelp { description: "lz4 (fast, no level)", command: "recon --compress lz4 ./data.bin -o data.lz4" },
        ExampleHelp { description: "xz with best compression", command: "recon --compress xz --compression-level best ./data -o data.xz" },
        ExampleHelp { description: "Snappy streaming compression", command: "recon --compress snappy ./stream.log -o stream.sz" },
        ExampleHelp { description: "zlib (RFC 1950, not gzip-wrapped)", command: "recon --compress zlib ./blob.bin -o blob.zlib" },
        ExampleHelp { description: "List supported algorithms", command: "recon --compress-list" },
    ],
};

static TOPIC_ENCODE: Topic = Topic {
    title: "Encoding (QR / DataMatrix / Barcodes)",
    description: "Generate QR codes, DataMatrix codes, and linear barcodes from literal text.\n\
                  Input comes from the positional argument, stdin (via `-` or a pipe), or\n\
                  --from-file <PATH>. Output goes to stdout or a file via -o; the format\n\
                  is inferred from the file extension (.svg / .png) or can be set explicitly\n\
                  with --encode-format.",
    flags: &[
        FlagHelp {
            flags: "--encode <FORMAT>",
            description: "Code format. Supported:\n\
                          qr, datamatrix, code128, code39, ean13, upca.",
        },
        FlagHelp {
            flags: "--encode-format <FMT>",
            description: "Output format: ascii (default for terminal), svg, or png.\n\
                          When omitted, -o <FILE> extension is honored: .svg → svg,\n\
                          .png → png, otherwise ASCII.",
        },
        FlagHelp {
            flags: "--from-file <PATH>",
            description: "Read the encode input from a file. Mutually exclusive with a\n\
                          positional text argument.",
        },
        FlagHelp {
            flags: "--encode-list",
            description: "Standalone action: list all supported formats with their input\n\
                          requirements. Does not require a URL.",
        },
        FlagHelp {
            flags: "--encode-hints <KEY=VAL>",
            description: "Pass an rxing encoder hint. Repeatable. Applies to aztec /\n\
                          pdf417 (the only formats recon currently routes through rxing).\n\
                          Supported keys:\n  \
                          \u{2022} charset=<NAME>     — Character set / ECI (e.g. UTF-8, Shift_JIS).\n  \
                          \u{2022} eclevel=<N>        — Error correction. Aztec: minimum % of EC\n    \
                                                  words. PDF417: 0..8 (higher = more redundancy).\n  \
                          \u{2022} aztec-layers=<N>   — -4..-1 compact, 0 auto, 1..32 full Aztec.\n  \
                          \u{2022} pdf417-compact=<bool>      — Use PDF417 compact mode.\n  \
                          \u{2022} pdf417-compaction=<MODE>   — PDF417 compaction (e.g. TEXT, BYTE).\n  \
                          \u{2022} pdf417-auto-eci=<bool>     — Auto-insert ECIs for non-Latin-1.\n  \
                          \u{2022} margin=<PX>        — Quiet-zone margin in pixels.\n\
                          Unknown keys error. Hints set on non-rxing formats (qr, datamatrix,\n\
                          code128, code39, ean13, upca) also error — those encoders use crates\n\
                          without an equivalent hint API.",
        },
        FlagHelp {
            flags: "--hrt / --no-hrt",
            description: "Show human-readable text under 1D barcodes. Default on for\n\
                          EAN-13 / UPC-A, off for Code128 / Code39. Implemented for\n\
                          ASCII and SVG output; PNG HRT is deferred pending font bundling.",
        },
        FlagHelp {
            flags: "--decode-all <IMAGE>",
            description: "Scan an image for every barcode (not just the first).\n\
                          One line per detection: <FORMAT>\\t<TEXT>.",
        },
    ],
    related: &["-o / --output", "--from-file"],
    examples: &[
        ExampleHelp { description: "QR code to terminal (ASCII)", command: "recon --encode qr \"https://example.com\"" },
        ExampleHelp { description: "QR code to SVG (inferred from extension)", command: "recon --encode qr \"https://example.com\" -o qr.svg" },
        ExampleHelp { description: "QR code to PNG", command: "recon --encode qr \"Contact: +46-70-123\" -o contact.png" },
        ExampleHelp { description: "DataMatrix (Swedish personal number)", command: "recon --encode datamatrix \"199001011234\" -o id.png" },
        ExampleHelp { description: "EAN-13 retail barcode with HRT (default)", command: "recon --encode ean13 \"4006381333931\" -o retail.svg" },
        ExampleHelp { description: "Code 128 alphanumeric with explicit HRT", command: "recon --encode code128 --hrt \"RECON-TEST-001\" -o c128.svg" },
        ExampleHelp { description: "Encode from stdin", command: "echo \"https://example.com\" | recon --encode qr" },
        ExampleHelp { description: "Encode from file", command: "recon --encode qr --from-file long-url.txt -o link.png" },
        ExampleHelp { description: "Compact Aztec (2-layer)", command: "recon --encode aztec --encode-hints aztec-layers=-2 \"compact aztec\"" },
        ExampleHelp { description: "PDF417 with EC level 5", command: "recon --encode pdf417 --encode-hints eclevel=5 -o p.svg \"...\"" },
        ExampleHelp { description: "Aztec with explicit charset (ECI)", command: "recon --encode aztec --encode-hints charset=Shift_JIS \"日本\"" },
        ExampleHelp { description: "List supported formats", command: "recon --encode-list" },
        ExampleHelp { description: "Scan every barcode in an image", command: "recon --decode-all sheet.png" },
    ],
};

static TOPIC_ENCRYPT: Topic = Topic {
    title: "Encryption (age format)",
    description: "Encrypt or decrypt data with the age file format. Supports passphrase-based\n\
                  encryption (scrypt KDF) and X25519 recipient-based encryption. Input comes\n\
                  from any source (file, URL, stdin, file://); output goes to stdout or -o.\n\
                  Binary by default; use --armor for ASCII-armored output. Decrypt auto-detects\n\
                  the format.",
    flags: &[
        FlagHelp {
            flags: "--encrypt",
            description: "Encrypt the input. Needs at least one --recipient or a passphrase source.",
        },
        FlagHelp {
            flags: "--decrypt",
            description: "Decrypt the input. Auto-detects binary vs armored and passphrase vs\n\
                          recipient mode from the header.",
        },
        FlagHelp {
            flags: "--passphrase-file <PATH>",
            description: "Read passphrase from a file (trims one trailing newline). Priority:\n\
                          file > $RECON_PASSPHRASE > interactive prompt.",
        },
        FlagHelp {
            flags: "--recipient <AGE1... | PATH>",
            description: "Encrypt to an X25519 recipient. Literal age1... public key or a\n\
                          path to a file containing one. Repeatable.",
        },
        FlagHelp {
            flags: "--identity <PATH>",
            description: "Decrypt with the age private-key file at PATH. Repeatable.",
        },
        FlagHelp {
            flags: "--armor",
            description: "Produce ASCII-armored output (--encrypt only).",
        },
        FlagHelp {
            flags: "--encrypt-keygen",
            description: "Standalone action: print a fresh X25519 key pair (age-compatible).",
        },
        FlagHelp {
            flags: "--pgp / --age",
            description: "Force the backend. Without either flag, recon auto-detects per\n\
                          recipient: `age1…` = age, anything else (fingerprint / email /\n\
                          key-id) = PGP. --pgp requires a local `gpg` binary on PATH.\n\
                          The two flags are mutually exclusive.",
        },
        FlagHelp {
            flags: "--rekey",
            description: "Rotate keys: decrypt the input with --identity (or --passphrase-file\n\
                          for passphrase-encrypted age files / gpg pinentry for PGP),\n\
                          then re-encrypt to --recipient. Source format is auto-detected\n\
                          (age vs PGP magic bytes). Output written to -o. Can switch\n\
                          backends — age → PGP or PGP → age — by pairing with --pgp /\n\
                          --age on the new side.",
        },
    ],
    related: &["-o / --output", "-H / --header"],
    examples: &[
        ExampleHelp { description: "Generate a key pair", command: "recon --encrypt-keygen -o key.txt" },
        ExampleHelp { description: "Encrypt with passphrase (interactive prompt)", command: "recon --encrypt ./secret.bin -o secret.age" },
        ExampleHelp { description: "Encrypt with passphrase from env", command: "RECON_PASSPHRASE=... recon --encrypt ./secret.bin -o secret.age" },
        ExampleHelp { description: "Encrypt to an X25519 recipient", command: "recon --encrypt ./payload.bin --recipient age1abc... -o payload.age" },
        ExampleHelp { description: "Encrypt armored for paste", command: "recon --encrypt ./note.txt --armor -o note.age.txt" },
        ExampleHelp { description: "Decrypt with a passphrase", command: "recon --decrypt secret.age -o secret.bin" },
        ExampleHelp { description: "Decrypt with a private-key file", command: "recon --decrypt payload.age --identity ~/.config/age/keys.txt -o payload.bin" },
        ExampleHelp { description: "Decrypt a URL-hosted payload", command: "recon --decrypt https://cdn/secret.age --identity ~/.age.key -o secret.bin" },
        ExampleHelp { description: "Encrypt to a PGP recipient (shells out to gpg)", command: "recon --encrypt ./secret.bin --recipient alice@example.com --armor -o secret.pgp" },
        ExampleHelp { description: "Decrypt a PGP message (format auto-detected)", command: "recon --decrypt secret.pgp -o secret.bin" },
        ExampleHelp { description: "Rotate keys: re-encrypt with a new recipient", command: "recon --rekey --identity old-key.txt --recipient age1new... old.age -o new.age" },
        ExampleHelp { description: "Switch backends during rotation (age → PGP)", command: "recon --rekey --identity old-key.txt --pgp --recipient alice@example.com old.age -o new.pgp" },
    ],
};

static TOPIC_EDITOR: Topic = Topic {
    title: "Editor Output",
    description: "Redirect recon's response output into an editor. Saves the body (or whatever the\n\
                  current output flags would print) to /tmp/recon-<timestamp>.<ext> and launches\n\
                  the editor on it — fire-and-forget. Extensions are derived from Content-Type so\n\
                  editors get syntax highlighting automatically.\n\
                  \n\
                  Note: because --editor takes an optional value, do NOT put the URL directly\n\
                  after a bare --editor — clap will consume the URL as the editor value. Either\n\
                  place the URL first (recon <URL> --editor), use --editor=zed <URL>, or use\n\
                  --url to disambiguate (recon --editor --url <URL>).",
    flags: &[
        FlagHelp {
            flags: "--editor [EDITOR]",
            description: "Open the output in the given editor.\n\
                          Built-in aliases: zed, code, cursor, subl, vim, nvim, nano, emacs.\n\
                          Accepts a user alias from [editor.aliases] or a raw shell command.\n\
                          Omit the value to use [editor] default from ~/.recon/config.toml.",
        },
        FlagHelp {
            flags: "--editor-cleanup",
            description: "Delete all /tmp/recon-* temp files written by past --editor runs.\n\
                          Standalone action: does not require a URL.",
        },
        FlagHelp {
            flags: "-vv (with --editor)",
            description: "Also mirror the body to stdout in addition to opening the editor.\n\
                          By default stdout is silent when --editor is active.",
        },
    ],
    related: &["-o / --output", "-p / --prettify", "-i / --include", "--full"],
    examples: &[
        ExampleHelp {
            description: "Open a JSON response in Zed",
            command: "recon --editor zed https://httpbin.org/get",
        },
        ExampleHelp {
            description: "Open prettified HTML in VS Code",
            command: "recon --editor code -p https://example.com",
        },
        ExampleHelp {
            description: "Use a raw command (passes through sh -c)",
            command: "recon --editor \"code --new-window\" https://example.com",
        },
        ExampleHelp {
            description: "Use the default editor from config",
            command: "recon --editor https://example.com",
        },
        ExampleHelp {
            description: "Mirror body to stdout as well",
            command: "recon --editor zed -vv https://httpbin.org/get",
        },
        ExampleHelp {
            description: "Purge leftover temp files",
            command: "recon --editor-cleanup",
        },
    ],
};

static TOPIC_CHECKDIGIT: Topic = Topic {
    title: "Check-Digit Verification and Computation",
    description: "Verify or compute check digits for 40 canonical identifier schemes\n\
                  (55 total lookup strings with aliases). Input comes from the normal\n\
                  source layer: positional argument, '-' or pipe for stdin, URL, or file://.\n\
                  \n\
                  EU VAT (full — 27 countries):\n\
                    se-vat (Sweden)                at-vat (Austria)             be-vat (Belgium)\n\
                    bg-vat (Bulgaria, auto)        cy-vat (Cyprus)              cz-vat (Czech, auto)\n\
                    de-vat (Germany)               dk-vat (Denmark)             ee-vat (Estonia)\n\
                    el-vat (Greece; alias gr-vat)  es-vat (Spain, auto)         fi-vat (Finland)\n\
                    fr-vat (France)                hr-vat (Croatia)             hu-vat (Hungary)\n\
                    ie-vat (Ireland)               it-vat (Italy)               lt-vat (Lithuania)\n\
                    lu-vat (Luxembourg)            lv-vat (Latvia, auto)        mt-vat (Malta)\n\
                    nl-vat (Netherlands)           pl-vat (Poland / NIP)        pt-vat (Portugal / NIF)\n\
                    ro-vat (Romania / CIF)         si-vat (Slovenia)            sk-vat (Slovakia)\n\
                  \n\
                  Multi-variant sub-keywords (explicit selection):\n\
                    es-nif, es-nie, es-cif         Spain: NIF (citizen) / NIE (foreigner) / CIF (entity)\n\
                    bg-egn, bg-bulstat             Bulgaria: EGN (personal) / BULSTAT (legal)\n\
                    cz-person, cz-legal            Czech: rodné číslo / IČO\n\
                    lv-person, lv-business         Latvia: personal / business\n\
                  \n\
                  Non-EU European VAT (13 jurisdictions):\n\
                    no-vat (Norway MVA)                   uk-vat (UK; aliases gb-vat, gbvat)\n\
                    ch-vat (Switzerland UID)              li-vat (Liechtenstein, shares CH alg.)\n\
                    ru-vat (Russia INN, auto-detect)      rs-vat (Serbia PIB)\n\
                    is-vat (Iceland kennitala)            ua-vat (Ukraine, auto-detect)\n\
                    tr-vat (Turkey VKN)                   md-vat (Moldova IDNO)\n\
                    by-vat (Belarus UNP, alphanumeric OK) mk-vat (North Macedonia EDB)\n\
                    me-vat (Montenegro PIB)\n\
                  \n\
                  Multi-variant sub-keywords (non-EU):\n\
                    ru-legal, ru-individual               Russia INN: 10-digit legal / 12-digit individual\n\
                    ua-legal, ua-individual               Ukraine: 8-digit EDRPOU / 10-digit RNOKPP\n\
                  \n\
                  Not implemented in 0.19.0 (no verified algorithm found):\n\
                    Albania NIPT, Bosnia JIB, Kosovo NUI.\n\
                  \n\
                  Output format (verify):\n\
                    <formatted>|<type>|<valid|invalid>|<comment>\n\
                  \n\
                    The comment field is empty unless there's a note to surface. Known\n\
                    comments include:\n\
                    - \"person >= 110 years old — likely data entry error\" (SE personnummer)\n\
                    - \"suffix NN (unusual — typically 01)\" (SE VAT with non-01 suffix)\n\
                    - \"post-2007 CPRs may legitimately fail the mod-11 check\" (DK CPR)\n\
                    - \"valid — no EIP-55 case check applied\" (Ethereum all-lowercase)\n\
                  \n\
                  VAT country-code prefix:\n\
                    All 27 EU VAT keywords accept input with or without the country\n\
                    code prefix. Input with a mismatched prefix is rejected:\n\
                  \n\
                    recon --checkdigit pl-vat 5261040828           # OK, no prefix\n\
                    recon --checkdigit pl-vat PL5261040828         # OK, prefix stripped\n\
                    recon --checkdigit pl-vat DE5261040828         # rejected (DE != PL)\n\
                  \n\
                    Greek VAT accepts both 'EL' and 'GR' prefixes as aliases.",
    flags: &[
        FlagHelp {
            flags: "--checkdigit <NAME> [INPUT]",
            description: "Verify a check digit. NAME is the algorithm keyword (e.g. luhn, visa, iban).\n\
                          Output format: <formatted>|<type>|<valid|invalid>|<comment>\n\
                          On invalid input, prints error to stderr and exits 1.",
        },
        FlagHelp {
            flags: "--checkdigit-create <NAME> [INPUT]",
            description: "Compute and append/insert a check digit.\n\
                          For most algorithms the input is the body digits without the check digit.",
        },
        FlagHelp {
            flags: "--checkdigit-list",
            description: "Standalone action: print all supported algorithms and aliases.\n\
                          Does not require a URL or input.",
        },
        FlagHelp {
            flags: "--raw",
            description: "Strip grouping characters (spaces, hyphens) from output.\n\
                          Applies to --checkdigit and --checkdigit-create.",
        },
    ],
    related: &["--hash", "--encode"],
    examples: &[
        ExampleHelp { description: "Verify a credit card number", command: "recon --checkdigit creditcard 4111111111111111" },
        ExampleHelp { description: "Verify a Visa card", command: "recon --checkdigit visa 4111111111111111" },
        ExampleHelp { description: "Verify an IBAN (spaces accepted)", command: "recon --checkdigit iban 'SE35 5000 0000 0549 1000 0003'" },
        ExampleHelp { description: "Create an Amex check digit", command: "recon --checkdigit-create amex 37828224631000" },
        ExampleHelp { description: "Verify a Swedish personnummer", command: "recon --checkdigit personnummer 811228-9874" },
        ExampleHelp { description: "Verify from stdin", command: "echo '4111111111111111' | recon --checkdigit luhn" },
        ExampleHelp { description: "List all supported algorithms", command: "recon --checkdigit-list" },
        ExampleHelp { description: "Raw output (strip grouping)", command: "recon --checkdigit creditcard 4111111111111111 --raw" },
    ],
};

static TOPIC_WRITE_OUT: Topic = Topic {
    title: "Write-Out Format Variables (-w / --write-out)",
    description: "The -w / --write-out flag emits a format string after the response body.\n\
                  Variable references take the form %{name}. Special forms:\n\
                  \n\
                  %{header{name}}   — value of the named response header (lowercase)\n\
                  %{json}           — all variables as an alphabetical JSON object\n\
                  %{stderr}         — switch subsequent output to stderr\n\
                  %{stdout}         — switch back to stdout\n\
                  \n\
                  Escape sequences: \\n (newline), \\t (tab), \\r (carriage return), \\\\\\ (backslash).\n\
                  Load the format from a file with @path, or from stdin with @-.\n\
                  \n\
                  NOTE: The four connection-phase timing variables (time_namelookup, time_connect,\n\
                  time_appconnect, time_pretransfer) render as 0.000000 in this release. reqwest's\n\
                  blocking client wraps an async hyper client internally; connector instrumentation\n\
                  is deferred per OUT-OF-SCOPE.md. The remaining timing variables are accurate.",
    flags: &[
        FlagHelp { flags: "http_code / response_code", description: "HTTP response status code (both are aliases, e.g. 200, 404)." },
        FlagHelp { flags: "http_version", description: "HTTP protocol version: 1.0, 1.1, 2, or 3." },
        FlagHelp { flags: "url / url_effective", description: "Final URL after any redirects." },
        FlagHelp { flags: "scheme", description: "URL scheme of the effective URL (e.g. https)." },
        FlagHelp { flags: "content_type", description: "Value of the response Content-Type header." },
        FlagHelp { flags: "size_download", description: "Number of bytes in the response body." },
        FlagHelp { flags: "size_upload", description: "Number of bytes sent as the request body." },
        FlagHelp { flags: "size_header", description: "Number of bytes in the response headers." },
        FlagHelp { flags: "speed_download", description: "Download speed: body bytes divided by time_total (bytes/sec)." },
        FlagHelp { flags: "num_redirects", description: "Number of redirect hops followed." },
        FlagHelp { flags: "num_headers", description: "Number of response headers received." },
        FlagHelp { flags: "redirect_url", description: "Value of the Location header when a 3xx is received and -L is not set." },
        FlagHelp { flags: "remote_ip", description: "IP address of the remote server. Requires connector instrumentation; currently empty." },
        FlagHelp { flags: "local_ip", description: "Local IP address used for the connection. Requires connector instrumentation; currently empty." },
        FlagHelp { flags: "time_namelookup", description: "Seconds from start to DNS resolution complete. Currently 0.000000 (deferred)." },
        FlagHelp { flags: "time_connect", description: "Seconds from start to TCP connection established. Currently 0.000000 (deferred)." },
        FlagHelp { flags: "time_appconnect", description: "Seconds from start to TLS/SSL handshake complete. Currently 0.000000 (deferred)." },
        FlagHelp { flags: "time_pretransfer", description: "Seconds from start to first byte ready to transfer. Currently 0.000000 (deferred)." },
        FlagHelp { flags: "time_starttransfer", description: "Seconds from start to first response byte received (TTFB). Accurate." },
        FlagHelp { flags: "time_redirect", description: "Seconds spent on all redirect hops combined. Accurate." },
        FlagHelp { flags: "time_total", description: "Total elapsed seconds for the entire operation. Accurate." },
    ],
    related: &["-o / --output", "-O / --remote-name", "-s / --silent"],
    examples: &[
        ExampleHelp { description: "Print status code and total time", command: "recon -w \"%{http_code} %{time_total}s\\n\" https://example.com/" },
        ExampleHelp { description: "Emit all variables as JSON", command: "recon -w \"%{json}\" -o /dev/null https://example.com/" },
        ExampleHelp { description: "Print Content-Type header", command: "recon -w \"%{header{content-type}}\\n\" -o /dev/null https://example.com/" },
        ExampleHelp { description: "Load format from a file", command: "recon -w @fmt.txt https://example.com/" },
    ],
};

static TOPIC_MQTT: Topic = Topic {
    title: "MQTT Client (probe, publish, subscribe)",
    description: "Connect to an MQTT 3.1.1 or 5.0 broker. Three modes:\n\
                  \n\
                  • Probe (default): connect, dump CONNACK details, disconnect.\n\
                  • Publish: send one message with -d/--data, topic in URL path.\n\
                  • Subscribe: stream messages matching --subscribe filters; exits on\n\
                    Ctrl-C or after --count messages.\n\
                  \n\
                  URL scheme: mqtt:// (port 1883) or mqtts:// (port 8883, TLS).",
    flags: &[
        FlagHelp { flags: "--mqtt-version <N>", description: "Protocol version: 3 (MQTT 3.1.1) or 5 (MQTT 5.0). Default: 5." },
        FlagHelp { flags: "--client-id <ID>", description: "MQTT client identifier.\nDefault: recon-<random-hex-6>." },
        FlagHelp { flags: "--keepalive <SECS>", description: "Keepalive interval in seconds. Default: 60." },
        FlagHelp { flags: "--qos <0|1|2>", description: "QoS level for publish and subscribe. Default: 0 (at-most-once)." },
        FlagHelp { flags: "--retain", description: "Set the PUBLISH retain flag (publish mode only)." },
        FlagHelp { flags: "--subscribe <FILTER>", description: "Topic filter to subscribe to. Repeatable.\nFilters may contain MQTT wildcards: + (one level) and # (multi-level, end only)." },
        FlagHelp { flags: "--count <N>", description: "In subscribe mode, exit after receiving N messages." },
        FlagHelp { flags: "--mqtt-json", description: "Emit structured JSON: probe prints one JSON object, subscribe prints NDJSON\n(one object per message). Non-UTF-8 payloads in subscribe are wrapped as {\"base64\": \"...\"}." },
        FlagHelp { flags: "-d, --data <DATA>", description: "Publish mode: payload (string, @file, or @- for stdin). Triggers publish when the URL has a topic." },
        FlagHelp { flags: "-u, --user <USER:PASS>", description: "Broker username/password in the CONNECT packet. Overrides URL userinfo." },
        FlagHelp { flags: "-k, --insecure", description: "Skip broker TLS certificate verification (mqtts:// only)." },
        FlagHelp { flags: "--connect-timeout <SECS>", description: "Socket connect + CONNACK wait timeout. Default: 30." },

        // MQTT 5 power-user properties (ignored on --mqtt-version 3).
        FlagHelp { flags: "--user-property <KEY=VAL>", description: "MQTT 5 user-property (repeatable). Applied to both PUBLISH\nand SUBSCRIBE packets." },
        FlagHelp { flags: "--will-topic <T> / --will-payload <P>", description: "Last-will message. Broker publishes P to T if this client\ndisconnects unexpectedly. --will-payload accepts @file / @-." },
        FlagHelp { flags: "--will-qos <0|1|2> / --will-retain", description: "QoS + retain flag for the last-will message." },
        FlagHelp { flags: "--session-expiry <SECS>", description: "MQTT 5 session-expiry-interval. Pair with `--clean-start=false`\nto resume a persistent session." },
        FlagHelp { flags: "--clean-start <BOOL>", description: "MQTT 5 clean-start flag. Default true. Set false to resume\na persistent session (requires --session-expiry on create)." },
        FlagHelp { flags: "--content-type <MIME>", description: "Publish content-type property (e.g. application/json)." },
        FlagHelp { flags: "--response-topic <T>", description: "Publish response-topic property for request/response patterns." },
        FlagHelp { flags: "--correlation-data <DATA>", description: "Publish correlation-data property. Accepts @file / @- or raw." },
        FlagHelp { flags: "--auth-method <NAME> / --auth-data <DATA>", description: "MQTT 5 enhanced-authentication on connect." },
    ],
    related: &["-w / --write-out", "--cert"],
    examples: &[
        ExampleHelp { description: "Probe a broker", command: "recon mqtt://broker.example.com:1883/" },
        ExampleHelp { description: "Probe over TLS with JSON output", command: "recon mqtts://broker.example.com:8883/ --mqtt-json" },
        ExampleHelp { description: "Publish a retained message at QoS 1", command: "recon mqtt://broker/devices/fan/state -d \"on\" --qos 1 --retain" },
        ExampleHelp { description: "Subscribe to a topic filter, exit after 10 messages", command: "recon mqtt://broker/ --subscribe \"devices/+/state\" --count 10 -v" },
        ExampleHelp { description: "Fall back to MQTT 3.1.1 on a legacy broker", command: "recon mqtt://legacy-broker/ --mqtt-version 3" },
        ExampleHelp { description: "Publish with MQTT 5 user-property + content-type", command: r#"recon mqtt://broker/events -d '{"ok":true}' --user-property env=prod --content-type application/json"# },
        ExampleHelp { description: "Request/response pattern via response-topic", command: "recon mqtt://broker/req -d 'ping' --response-topic 'rsp/abc' --correlation-data 'corr-1'" },
        ExampleHelp { description: "Set a last-will so the broker announces our disconnect", command: "recon mqtt://broker/status -d 'online' --will-topic status --will-payload offline --will-retain" },
        ExampleHelp { description: "Resume a persistent session", command: "recon mqtt://broker/ --subscribe 'events/#' --session-expiry 3600 --clean-start=false --client-id myclient-1" },
    ],
};

static TOPIC_SCRIPT: Topic = Topic {
    title: "Scripting (--script)",
    description: "Run a Rhai script that drives the recon probe API. Scripts can\n\
                  chain requests, branch on results, loop, query SQLite, hash\n\
                  bodies, and build multi-step health checks. The script's\n\
                  `return N` (integer) becomes the process exit code; uncaught\n\
                  exceptions map to the same exit codes as the CLI (7 connect-\n\
                  refused, 28 timeout, 67 auth).\n\
                  \n\
                  `--script` is mutually exclusive with a positional URL. CLI\n\
                  flags (-H, -k, --connect-timeout, etc.) act as defaults that\n\
                  per-call opts maps can override.\n\
                  \n\
                  First-time setup: `recon --init` creates ~/.recon/ with a\n\
                  script/ subdirectory and a commented config.toml.\n\
                  \n\
                  Script resolution: when PATH isn't found as given, recon\n\
                  looks in ~/.recon/script/PATH (and auto-appends .rhai when\n\
                  PATH has no extension). Drop reusable scripts in\n\
                  ~/.recon/script/ and call them by bare name:\n\
                    recon --script health\n\
                  \n\
                  Positional arguments after the script path are exposed as\n\
                  `args[1..]` (args[0] is the script name as typed). CLI flag\n\
                  values are exposed as the `flags` map.\n\
                  \n\
                  Shebang: add #!/usr/bin/env -S recon --script as the first\n\
                  line, chmod +x, and run the file directly. The shebang is\n\
                  silently treated as a comment by the Rhai engine.",
    flags: &[
        FlagHelp { flags: "--script <PATH>", description: "Load and run a .rhai file. Falls back to\n~/.recon/script/<PATH> when the path doesn't exist as given\n(with auto-.rhai extension when PATH has none).\nExample: recon --script checks.rhai\n         recon --script health     # -> ~/.recon/script/health.rhai" },
        FlagHelp { flags: "shebang (executable scripts)", description: "Add #!/usr/bin/env -S recon --script as the first line,\nthen chmod +x the file. The kernel passes the script path to\nrecon, which strips the shebang before Rhai sees it.\nExample first line: #!/usr/bin/env -S recon --script\nTrailing args: ./check.rhai prod 8080 -> args == [\"check.rhai\",\"prod\",\"8080\"]" },

        FlagHelp { flags: "args (global)", description: "Array of positional arguments. args[0] is the script name\nas typed (e.g. \"health\", not the resolved path). args[1..]\nare trailing positional args: `recon --script foo a b -v` -> \n[\"foo\", \"a\", \"b\", \"-v\"]. Read-only inside the script." },
        FlagHelp { flags: "flags (global)", description: "Map of CLI flags in effect. Keys: headers (array), insecure,\nconnect_timeout, max_time, follow_redirects, max_redirs,\nuser_agent, referer, user, method, data, output, verbose,\nwait_time, ping_count, max_hops. Unset optionals are `()`.\nRead-only inside the script." },

        FlagHelp { flags: "http(url) / http(url, opts)", description: "HTTP(S) request. Returns #{ url, final_url, status, body, headers,\nhttp_version, duration_ms }. opts: #{ method, headers, body,\ntimeout_ms, connect_timeout, insecure, follow_redirects }.\nHTTP non-2xx is a result; network errors throw." },
        FlagHelp { flags: "https(...) / request(opts)", description: "Aliases. request() requires opts.url." },

        FlagHelp { flags: "tcp(url) / tcp(url, opts)", description: "TCP connect probe. Returns #{ ok, host, port, resolved_ip,\nlocal_addr, duration_ms }." },
        FlagHelp { flags: "ping(host) / ping(host, count)", description: "TCP ping (host:port) or ICMP ping (bare host).\nReturns #{ protocol, host, sent, received, loss_pct,\nmin_ms, avg_ms, max_ms, replies: [#{seq, ms}] }." },
        FlagHelp { flags: "dns(host) / dns(host, types)", description: "DNS lookup. Types default to A, AAAA, CNAME, MX, NS, TXT, SOA;\npass an array like [\"A\"] to query specific types.\nReturns #{ host, records: #{...}, errors: #{...}, duration_ms }." },
        FlagHelp { flags: "tls(host) / tls(host, port)", description: "TLS cert inspection (host-verify off). Returns #{ subject, issuer,\nnot_before, not_after, days_remaining, is_expired, san, cert_pem,\nsignature_algorithm, public_key, ... }." },
        FlagHelp { flags: "ntp(url)", description: "SNTPv4 probe. Returns #{ host, port, stratum, precision,\npoll_interval, ref_id, reference_ts, offset_ms, delay_ms }." },

        FlagHelp { flags: "redis(url) / redis(url, cmd)", description: "PING by default, or a shell-split RESP command when `cmd` is\ngiven. Returns #{ host, port, connect_ms, auth_reply,\ncommand, reply, command_ms }." },
        FlagHelp { flags: "ws(url) / wss(url)", description: "WebSocket handshake + Ping/Pong round-trip.\nReturns #{ connect_ms, handshake_ms, http_status, headers,\npong_nonce_matched, ping_ms }." },
        FlagHelp { flags: "dict(url)", description: "RFC 2229 DICT. Bare URL runs the server-info aggregate.\nReturns #{ banner, responses: [#{ command, lines, final_status }] }." },
        FlagHelp { flags: "ldap(url) / ldaps(url)", description: "Anonymous bind + RootDSE. Returns #{ url, connect_ms,\nattrs: #{ namingContexts, supportedLDAPVersion, ... } }." },
        FlagHelp { flags: "whois(host)", description: "Two-hop whois with registrar referral. Returns #{ host, server, body }." },
        FlagHelp { flags: "memcached(url)", description: "Memcached version (+ /stats). Returns #{ host, port, connect_ms,\nversion, version_ms, stats: #{...} }." },
        FlagHelp { flags: "rtsp(url) / rtsps(url)", description: "RTSP OPTIONS. Returns #{ host, port, tls, connect_ms,\nstatus_line, status_code, headers, methods }." },
        FlagHelp { flags: "mqtt_pub(url, payload) / mqtt_sub(url, max_ms)", description: "MQTT publish / subscribe. Runs the full CLI codepath; protocol\noutput flows to stdout. Returns #{ ok, duration_ms }." },
        FlagHelp { flags: "smtp(url) / smtp(url, opts)", description: "SMTP probe / mail delivery. Without opts.mail_from, reports\nEHLO capabilities + AUTH methods + STARTTLS status. With\nmail_from + mail_to, sends a message (optional DKIM signing).\nSee `recon --help smtp` for the full opts reference." },

        FlagHelp { flags: "ftp(url) / ftp(url, opts)", description: "FTP / FTPS probe + retrieve. See `recon --help ftp`." },
        FlagHelp { flags: "sftp(url) / sftp(url, opts)", description: "SSH-backed file transfer. See `recon --help sftp`." },
        FlagHelp { flags: "tftp(url) / tftp(url, opts)", description: "RFC 1350 UDP read. See `recon --help tftp`." },
        FlagHelp { flags: "gopher(url) / gopher(url, opts)", description: "Gopher selector fetch. See `recon --help gopher`." },
        FlagHelp { flags: "pop3(url) / pop3(url, opts)", description: "POP3 probe / retrieve. See `recon --help pop3`." },
        FlagHelp { flags: "imap(url) / imap(url, opts)", description: "IMAP probe / examine / fetch. See `recon --help imap`." },
        FlagHelp { flags: "ipfs_url(url [, #{gateway}])", description: "Rewrite an ipfs:// / ipns:// URL to its HTTP gateway form.\nScripts combine with http() for fetch." },
        FlagHelp { flags: "file_read(path)", description: "Read local file (or file:// URL) as a Rhai Blob (Vec<u8>)." },
        FlagHelp { flags: "compression::compress(algo, blob [, level]) / decompress([algo,] blob)", description: "Stream-compress or decompress a Blob. Algorithms match --compress:\ngzip, deflate, zstd, brotli, bzip2, lz4, xz, snappy, zlib. Optional\nlevel is an integer or word (fastest/fast/default/good/best).\n`decompress(blob)` auto-detects from magic bytes. Errors on lz4/snappy\nwith a level, or on deflate/brotli blobs in auto-detect mode.\ncompression::list() enumerates every algo + aliases + level range." },
        FlagHelp { flags: "archive::create(dest, [sources]) / extract(src, dest_dir) / detect(path)", description: "Same formats as --archive / --extract (.zip, .tar, .tar.gz, .tar.xz,\n.tar.bz2). create returns file count; extract returns extracted\ncount and creates dest_dir if missing. detect(path) returns the\nformat name (\"zip\", \"tar.gz\", ...) or () when unrecognised." },
        FlagHelp { flags: "encode::qr(data) / datamatrix(data) / barcode(format, data)", description: "QR / DataMatrix / 1D barcode generation. Returns a Blob (PNG bytes\nby default). encode::encode(format, data, output_format) for\nASCII or SVG output. encode::list() enumerates supported formats." },
        FlagHelp { flags: "encrypt::encrypt(blob, recipients) / decrypt(blob, identity_paths) / keygen()", description: "age encryption. Recipients are age1... strings or paths. identities\nare filesystem paths to key files. keygen() returns #{public,\nprivate}. encrypt_armored(...) for ASCII output." },
        FlagHelp { flags: "encrypt::rekey(blob, old_ids, new_recipients [, armor])", description: "Key rotation. Decrypt the input with old identities, re-encrypt\nto the new recipient set. Age-only from scripts; use CLI --rekey\nfor cross-backend (age <-> PGP) rotation." },
        FlagHelp { flags: "encrypt::pgp_encrypt(blob, recipients) / pgp_encrypt_armored(...) / pgp_decrypt(blob)", description: "PGP encrypt / decrypt via the system `gpg` binary. Recipients\ncan be fingerprints, key-ids, or emails. pgp_decrypt uses the\nuser's gpg keyring." },
        FlagHelp { flags: "encrypt::detect_backend(recipient)", description: "Returns `\"age\"` or `\"pgp\"` using the same heuristic as the\nCLI auto-detection." },
        FlagHelp { flags: "checkdigit::verify(algo, input) / create(algo, body) / inspect(algo, input) / list()", description: "Check-digit algorithms (VAT per-country, ISBN, EAN-13, Luhn,\ncredit card, VIN, etc.). verify returns bool, inspect returns\na detailed map, create appends the check digit, list enumerates." },
        FlagHelp { flags: "sample::list() / spec(name) / url(name, format)", description: "Browse the built-in sample-data registry. Data fetching is HTTP\n(use http() explicitly); sample:: is informational." },
        FlagHelp { flags: "jwt::sign(claims, secret [, alg]) / validate(token, secret) / view(token)", description: "JWT HS256/HS384/HS512 sign + verify. sign takes a claims map,\nvalidate returns #{valid, checks, header, payload}, view decodes\na token without verifying signature." },
        FlagHelp { flags: "email::spf / dmarc / dkim / mta_sts / bimi / tls_rpt(host) / all(host)", description: "Email-security DNS checks. Each returns a Map #{name, verdict,\nsummary, details}. all(host) runs five of them and returns a\ncomposite map." },
        FlagHelp { flags: "netstatus::check() / probe_http(url) / probe_tcp(host, port)", description: "Network reachability probes. check() runs a default set (HTTP +\nTCP to two DNS servers) and aggregates ONLINE / DEGRADED / OFFLINE." },
        FlagHelp { flags: "sqlite(spec) / sqlite(spec, mode)", description: "Open a SQLite database. spec is a path (contains /, \\, or ends\nwith .db), \":memory:\", or an alias: \"cookiejar\" or\n\"cookiejar:NAME\" for ~/.recon/jars/NAME.db. Mode: \"rw\" (default),\n\"ro\", or \"rwc\" (create). Returns a handle with four methods:\n  db.query(sql [, params]) -> Array<Map>\n  db.query_one(sql [, params]) -> Map or ()\n  db.query_value(sql [, params]) -> scalar or ()\n  db.exec(sql [, params]) -> rows affected\nParams are positional ? placeholders bound from [] in the second\narg: () -> NULL, bool -> INT, i64 -> INT, f64 -> REAL, String ->\nTEXT, Blob -> BLOB." },

        FlagHelp { flags: "md5(x) / sha1(x) / sha256(x) / sha384(x) / sha512(x)", description: "Hash a String or Blob and return a lowercase-hex digest.\nmd5 returns 32 hex chars, sha1 40, sha256 64, sha384 96, sha512 128." },
        FlagHelp { flags: "sha3_256(x) / sha3_512(x) / blake3(x) / crc32(x)", description: "Additional hashes: SHA-3 variants, BLAKE3, and CRC32 (4 bytes\nbig-endian, rendered as 8 hex chars)." },
        FlagHelp { flags: "hash(algo, x) / hash(algo, x, format)", description: "Generic hash. algo is any --hash name (md5, sha256, crc32, …).\nformat is \"hex\" (default) or \"base64\"." },

        FlagHelp { flags: "print(x)", description: "Rhai built-in. Writes x + newline to stdout." },
        FlagHelp { flags: "sleep_ms(n)", description: "Block the current thread for n milliseconds." },
        FlagHelp { flags: "env(name) / env(name, default)", description: "Read an environment variable. Empty string (or default) when unset." },
        FlagHelp { flags: "env_all() -> Map", description: "Snapshot of every process env var as a Rhai map. Aliased as envAll." },
        FlagHelp { flags: "load_dotenv(path) / load_dotenv(path, override)", description: "Parse a .env file and set each KEY=VALUE in the process env. Default\noverrides existing values, so `load_dotenv(\".env\"); load_dotenv(\".env.script\")`\nlayers correctly. Pass `false` to leave pre-existing env in place. Returns the\ncount of vars set. Aliased as loadDotEnv. Call before spawning threads." },
        FlagHelp { flags: "script_path / script_dir / script_name", description: "Read-only String constants pushed into the Scope: resolved absolute\npath, its parent directory, and the file stem (basename minus extension).\nCombine to load sibling files independent of CWD: `load_dotenv(script_dir +\n\"/.env\")` and `load_dotenv(script_dir + \"/.env.\" + script_name)`." },
        FlagHelp { flags: "now() / now_ms()", description: "Unix seconds or milliseconds as i64." },
        FlagHelp { flags: "assert(cond, msg)", description: "Throw a Rhai exception when cond is false." },
        FlagHelp { flags: "json_parse(s) / json_stringify(x)", description: "Round-trip JSON text ↔ Rhai values (null ↔ (), bool, int, float,\nstring, array, object ↔ map)." },
        FlagHelp { flags: "json_stringify(x, true) / json_stringify(x, n)", description: "Pretty-print variants. true = 2-space indent; integer n = n-space\nindent (clamped to 1..=8). n <= 0 falls back to compact output." },

        FlagHelp { flags: "import \"name\" as alias;", description: "Rhai module import. Resolves `name.rhai` next to the running\nscript first; falls back to ~/.recon/script/name.rhai. Lets you\nfactor shared helpers into reusable modules.\nExample:\n  import \"greet\" as g;\n  print(g::hello(\"recon\"));" },

        FlagHelp { flags: "browser() / browser(opts)", description: "Stateful HTTP session handle with sticky cookies + default\nheaders. b.get/post/put/patch/delete/head/options(url [, opts]).\nConfig: set_user_agent, set_header(s), follow_redirects,\nset_timeout_ms, set_insecure, set_basic_auth. Sessions:\nuse_persistent_session(name), use_ephemeral_session,\nclear_cookies, cookies(), session_name(). Body can be\nString / Blob / Map (map auto-serialises to JSON).\nSee `recon --help browser` for the full method reference." },

        FlagHelp { flags: "text::transcode / decode / encode / detect / charset_of / strip_bom / list / normalize_newlines", description: "Charset conversion + text helpers. text::decode(r.body_bytes,\ncharset) is the usual companion to `r.body_bytes` + `r.charset`\non http()/browser() responses. See `recon --help charset`." },

        FlagHelp { flags: "agentBrowser::*", description: "Browser automation via the external agent-browser CLI.\n`agentBrowser::available` (bool) + `agentBrowser::version` (string)\nare always readable. When available, functions include open, click,\nfill, screenshot, snapshot, get, is_visible, eval, and more.\nSee `recon --help agent-browser` for the full list." },
    ],
    related: &["--init", "--script", "-H", "-k", "--connect-timeout", "--max-time", "-L"],
    examples: &[
        ExampleHelp { description: "Bootstrap ~/.recon/ layout and drop your first script", command: "recon --init && $EDITOR ~/.recon/script/health.rhai" },
        ExampleHelp { description: "Run a script by bare name (falls back to ~/.recon/script/NAME.rhai)", command: "recon --script health" },
        ExampleHelp { description: "Pass positional args into a script (args[1..])", command: "recon --script check.rhai example.com 42" },
        ExampleHelp { description: "Inherit CLI flags as script defaults", command: "recon -k -H 'X-Api-Key: abc' --script probe.rhai" },
        ExampleHelp { description: "Query the cookie jar from a script", command: r#"recon --script jar-count  # reads ~/.recon/script/jar-count.rhai"# },
        ExampleHelp { description: "Browse per-module example scripts shipped in the repo", command: "ls script/*.rhai  # one-per-module demos: http, dns, jwt, email, encrypt, …" },
        ExampleHelp { description: "Run a shipped example directly", command: "recon --script script/http.rhai https://example.com" },
        ExampleHelp { description: "Install every shipped example into ~/.recon/script/", command: "cp script/*.rhai ~/.recon/script/" },
        ExampleHelp { description: "Full API surface via the SCRIPTING example block", command: "recon --examples" },
    ],
};

static TOPIC_DOCS: Topic = Topic {
    title: "Document conversions (markdown / HTML / PDF)",
    description: "Three conversions, one source-loader:\n\
                  \n\
                    --md-to-html    markdown → HTML (pure Rust)\n\
                    --html-to-pdf   HTML → PDF (via agent-browser)\n\
                    --md-to-pdf     md → HTML → PDF\n\
                  \n\
                  Source (SRC) is a path, http(s):// URL, or `-` for\n\
                  stdin. URL sources honor every HTTP flag (-H, -u,\n\
                  -L, -k, cookies, proxy, HSTS).\n\
                  \n\
                  HTML backend is `comrak` (CommonMark + GFM). PDF\n\
                  backend is `agent-browser pdf`, which wraps Chrome's\n\
                  printToPDF — preserves anchor links, @page CSS, and\n\
                  produces a clickable TOC in the PDF.\n\
                  \n\
                  TOC generation: comrak emits `id=\"slug\"` on each\n\
                  heading; recon adds a `<nav class=\"toc\">` block\n\
                  with anchor-linked entries up to `--toc-depth`.",
    flags: &[
        FlagHelp { flags: "--md-to-html <SRC>", description: "Render markdown → HTML. Output via -o <PATH> or stdout." },
        FlagHelp { flags: "--md-to-pdf <SRC>", description: "Render markdown → PDF. Requires -o <PATH> and agent-browser." },
        FlagHelp { flags: "--html-to-pdf <SRC>", description: "Render HTML → PDF. Requires -o <PATH> and agent-browser." },
        FlagHelp { flags: "--toc", description: "Inject a linkable table of contents at the top of the\ngenerated HTML." },
        FlagHelp { flags: "--toc-depth <N>", description: "Include headings up to H<N> in the TOC. Default 3." },
        FlagHelp { flags: "--toc-title <STR>", description: "Heading text for the injected TOC. Default \"Contents\"." },
        FlagHelp { flags: "--doc-title <STR>", description: "Sets <title> in the HTML + PDF metadata title." },
        FlagHelp { flags: "--doc-author <STR>", description: "Author field in PDF document properties." },
        FlagHelp { flags: "--doc-subject <STR>", description: "Subject field in PDF document properties." },
        FlagHelp { flags: "--doc-keywords <STR>", description: "Keywords field in PDF document properties\n(comma-separated). Verifiable via `pdfinfo`." },
        FlagHelp { flags: "--doc-css <PATH>", description: "Inline a user stylesheet (appended after the bundled default)." },
        FlagHelp { flags: "--no-default-css", description: "Skip the bundled default CSS. Pair with --doc-css." },
        FlagHelp { flags: "--gfm", description: "Enable GitHub-flavored extensions: tables, task lists,\nstrikethrough, autolinks, footnotes, tagfilter." },
        FlagHelp { flags: "--unsafe-html", description: "Allow raw HTML passthrough (comrak's `unsafe_`). Needed for cover\npages and explicit <div class=\"page-break\"> markers. Assume the\nmarkdown input is trusted when this is on." },
        FlagHelp { flags: "--page-break-on-h1", description: "Start a new PDF page before every top-level `#` heading except\nthe first. Injects `break-before: page` CSS. No visible effect\nin HTML output (printToPDF honours it)." },
        FlagHelp { flags: "md_to_html(src, opts)", description: "Script binding. src = string or Blob. Returns HTML string." },
        FlagHelp { flags: "md_to_pdf(src, dest, opts)", description: "Script binding. src literal, dest path. Needs agent-browser." },
        FlagHelp { flags: "html_to_pdf(src, dest)", description: "Script binding. Needs agent-browser." },
    ],
    related: &["script", "agent-browser", "http"],
    examples: &[
        ExampleHelp { description: "Markdown → HTML with TOC + GFM", command: "recon --md-to-html README.md --toc --gfm -o README.html" },
        ExampleHelp { description: "Fetch live markdown over HTTP, render local", command: "recon --md-to-html https://example.com/doc.md --toc -o doc.html" },
        ExampleHelp { description: "Markdown → PDF with linkable TOC", command: "recon --md-to-pdf CHANGELOG.md --toc --gfm --doc-title 'recon release notes' -o changelog.pdf" },
        ExampleHelp { description: "PDF with full metadata (verifiable via pdfinfo)", command: "recon --md-to-pdf doc.md --doc-title 'My Report' --doc-author 'Alice' --doc-subject 'Q1 results' --doc-keywords 'finance, Q1' -o report.pdf" },
        ExampleHelp { description: "HTML → PDF", command: "recon --html-to-pdf report.html -o report.pdf" },
        ExampleHelp { description: "Inject custom CSS", command: "recon --md-to-pdf notes.md --toc --doc-css print.css -o notes.pdf" },
        ExampleHelp { description: "Replace the bundled CSS entirely", command: "recon --md-to-pdf notes.md --no-default-css --doc-css print.css -o notes.pdf" },
        ExampleHelp { description: "Cover page + chapter breaks", command: "recon --md-to-pdf book.md --toc --gfm --unsafe-html --page-break-on-h1 --doc-title Book -o book.pdf" },
        ExampleHelp { description: "Explicit page break in markdown (with --unsafe-html)", command: r#"printf '# A\n\nFirst.\n\n<div class="page-break"></div>\n\n# B\n\nSecond.\n' > tmp.md && recon --md-to-pdf tmp.md --unsafe-html -o tmp.pdf"# },
    ],
};

static TOPIC_SCRIPT_SERVER: Topic = Topic {
    title: "Script network servers (TCP / UDP)",
    description: "Bind and accept from inside a Rhai script. Designed\n\
                  to pair with 0.56.0's `thread_spawn` — accept on the\n\
                  main thread, spawn a handler per connection.\n\
                  \n\
                  Deliberately NOT exposed as CLI flags — server\n\
                  workflows are always multi-step (accept → per-conn\n\
                  logic) which is what scripts are for. For quick\n\
                  HTTP serving use `recon --serve`.\n\
                  \n\
                  ICMP raw-socket primitives are deferred in 0.57.0.\n\
                  For pinging, use the existing `ping()` script\n\
                  binding.",
    flags: &[
        FlagHelp { flags: "tcp_listen(addr)", description: "Bind a TCP listener. `addr` like \"0.0.0.0:8080\" or \"[::]:8080\"." },
        FlagHelp { flags: "tcp_accept(listener)", description: "Blocking accept. Returns a TcpConn." },
        FlagHelp { flags: "tcp_accept(listener, timeout_ms)", description: "Accept with timeout; raises an error on timeout." },
        FlagHelp { flags: "tcp_read(conn, n, timeout_ms)", description: "Read up to N bytes; returns a Blob." },
        FlagHelp { flags: "tcp_read_line(conn, timeout_ms)", description: "Read one \\n-terminated line; trailing CR/LF stripped." },
        FlagHelp { flags: "tcp_write(conn, blob|str)", description: "Write all bytes / the full string. Returns bytes written." },
        FlagHelp { flags: "tcp_peer_addr(conn)", description: "The remote peer's SocketAddr as a string." },
        FlagHelp { flags: "tcp_close(conn)", description: "Close the connection." },
        FlagHelp { flags: "tcp_close_listener(l)", description: "Close the listener; any in-flight accept() will error." },
        FlagHelp { flags: "udp_bind(addr)", description: "Bind a UDP socket." },
        FlagHelp { flags: "udp_recv_from(sock, max_len, [timeout_ms])", description: "Returns #{ data: Blob, addr: string }." },
        FlagHelp { flags: "udp_send_to(sock, blob|str, addr)", description: "Returns bytes sent." },
        FlagHelp { flags: "udp_close(sock)", description: "Release the socket." },
    ],
    related: &["script", "threads", "serve"],
    examples: &[
        ExampleHelp { description: "Run the shipped tcp echo server", command: "recon --script script/tcp-echo.rhai 127.0.0.1:9000" },
        ExampleHelp { description: "Run the shipped udp listener", command: "recon --script script/udp-listen.rhai 127.0.0.1:9001" },
        ExampleHelp { description: "Test the echo server", command: "printf 'hello\\n' | nc -w1 127.0.0.1 9000" },
    ],
};

static TOPIC_SCRIPT_THREADS: Topic = Topic {
    title: "Script threading (spawn / channels / join)",
    description: "Fan-out concurrency inside a Rhai script. Backed by\n\
                  rhai's `sync` feature (enabled in 0.56.0), which\n\
                  makes the engine Send+Sync at a small per-value\n\
                  locking cost.\n\
                  \n\
                  Each spawned closure runs on a fresh OS thread with\n\
                  its own engine. The spawning engine shares its\n\
                  compiled AST + ScriptDefaults with the worker, so\n\
                  bindings like `http()` see the same CLI-flag\n\
                  inheritance chain.\n\
                  \n\
                  Channels are MPSC (multi-producer, single-consumer).\n\
                  Clone the sender to fan out; share the receiver via\n\
                  the same channel() call site.",
    flags: &[
        FlagHelp { flags: "thread_spawn(fn_ptr)", description: "Spawn a closure. Returns a ThreadHandle. `spawn` alone is reserved\nin rhai; use `thread_spawn`." },
        FlagHelp { flags: "thread_spawn(fn_ptr, arg)", description: "Spawn with one argument forwarded to the closure." },
        FlagHelp { flags: "thread_spawn(fn_ptr, args_array)", description: "Spawn with N arguments forwarded in order." },
        FlagHelp { flags: "join(h)", description: "Block until the handle's thread finishes; returns the closure's\nreturn value, or raises a script error if the worker errored." },
        FlagHelp { flags: "tid()", description: "Current thread ID (stable within a run). Useful for log lines." },
        FlagHelp { flags: "sleep(ms)", description: "Block the current thread. Alias of `sleep_ms` for readability." },
        FlagHelp { flags: "channel()", description: "Returns [sender, receiver]. Unbounded MPSC." },
        FlagHelp { flags: "channel_bounded(n)", description: "Returns [sender, receiver] with capacity N — try_send returns\nfalse when the buffer is full." },
        FlagHelp { flags: "send(tx, val)", description: "Blocking send. Errors only when every receiver has dropped." },
        FlagHelp { flags: "try_send(tx, val)", description: "Non-blocking. Returns true on success, false when bounded and\nfull, errors when the channel is closed." },
        FlagHelp { flags: "recv(rx)", description: "Blocking receive. Errors when every sender has dropped." },
        FlagHelp { flags: "recv(rx, timeout_ms)", description: "Receive with timeout. Errors on timeout or disconnect." },
        FlagHelp { flags: "try_recv(rx)", description: "Non-blocking. Returns () when the channel is empty or closed." },
    ],
    related: &["script", "scripting"],
    examples: &[
        ExampleHelp { description: "Run the shipped demo", command: "recon --script script/thread.rhai" },
        ExampleHelp { description: "Fan out 3 probes + gather", command: r#"recon --script - <<< 'let c = channel(); let tx = c[0]; let rx = c[1]; for i in 0..3 { thread_spawn(|n| { send(tx, http(`https://httpbin.org/anything?i=${n}`).status); }, i); } for j in 0..3 { print(recv(rx, 5000)); }'"# },
    ],
};

static TOPIC_SHELL: Topic = Topic {
    title: "Shell subprocess binding (`shell` / `shell_stream`)",
    description: "Run external commands from a script. Two forms:\n\
                  \n\
                  `shell(cmd, [opts])` — blocking. Returns a Map with\n\
                  `stdout`, `stderr`, `exit_code`, `success`. Use this\n\
                  for the run-one-command-and-parse-output pattern.\n\
                  \n\
                  `shell_stream(cmd, callback, [opts])` — streaming.\n\
                  The callback fires once per stdout / stderr line as\n\
                  the child writes it (the two streams are merged in\n\
                  arrival order). Returns the exit code when the child\n\
                  is done. Built for live progress UIs and the\n\
                  upcoming TUI pane primitive.\n\
                  \n\
                  Command shapes:\n\
                    - String input runs through the platform shell —\n\
                      `sh -c <s>` on Unix, `cmd /C <s>` on Windows.\n\
                      Pipes, globs, redirects, && chains work.\n\
                    - Array input is a literal argv — `shell([\"git\",\n\
                      \"log\"])`. No shell layer, no quoting surprises.\n\
                  \n\
                  Opts map (all keys optional):\n\
                    cwd, env, env_clear, timeout_ms, merge_stderr.",
    flags: &[
        FlagHelp { flags: "shell(cmd_string)", description: "Run through the platform shell. Returns Map with stdout / stderr / exit_code / success." },
        FlagHelp { flags: "shell(argv_array)", description: "Direct argv form. No $VAR expansion, no quoting surprises." },
        FlagHelp { flags: "shell(cmd, opts)", description: "Same forms with an opts map — see the description for keys." },
        FlagHelp { flags: "shell_stream(cmd, callback)", description: "Streaming form. Callback fires per line as the child writes;\nreturns the exit code on child exit." },
        FlagHelp { flags: "shell_stream(cmd, callback, opts)", description: "Streaming with opts map. timeout_ms kills the child and raises\na catchable error on overrun." },
        FlagHelp { flags: "opts.cwd", description: "Working directory (default: inherit from the script process)." },
        FlagHelp { flags: "opts.env", description: "Map of name→value, layered on top of the parent environment." },
        FlagHelp { flags: "opts.env_clear", description: "Bool. Drop the parent env entirely before applying `env`." },
        FlagHelp { flags: "opts.timeout_ms", description: "Kill the child after N ms; raises an error the script can `try`/`catch`." },
        FlagHelp { flags: "opts.merge_stderr", description: "Blocking form only — fold stderr into stdout. Streaming form\nalways merges." },
    ],
    related: &["script", "scripting", "threads"],
    examples: &[
        ExampleHelp { description: "Run the shipped demo", command: "recon --script script/shell.rhai" },
        ExampleHelp { description: "Capture output of a command", command: r#"recon --script - <<< 'let r = shell("git log --oneline -3"); print(r.stdout);'"# },
        ExampleHelp { description: "Stream lines as they arrive", command: r#"recon --script - <<< 'shell_stream("brew upgrade", |line| print(`[brew] ${line}`));'"# },
        ExampleHelp { description: "argv form skips the shell layer", command: r#"recon --script - <<< 'let r = shell(["echo", "$HOME"]); print(r.stdout);'"# },
        ExampleHelp { description: "cwd + env + timeout opts", command: r#"recon --script - <<< 'shell("cargo test", #{ cwd: "/path/to/repo", env: #{ RUST_LOG: "info" }, timeout_ms: 60000 });'"# },
    ],
};

static TOPIC_TUI: Topic = Topic {
    title: "TUI dashboard (`tui::run`)",
    description: "Multi-pane text dashboard for scripts. Built for\n\
                  run-and-watch flows where one or more long-running\n\
                  subprocesses stream their output into distinct text\n\
                  regions while the script logs its own progress.\n\
                  \n\
                  Sits on top of `shell_stream` — see `recon --help\n\
                  shell`. The natural pairing is to spawn each command\n\
                  with `shell_stream` and route lines into a pane via\n\
                  the callback (`|line| main.println(line)`).\n\
                  \n\
                  Single dashboard per process: nested `tui::run` calls\n\
                  raise an error rather than fighting over the\n\
                  terminal. Drop guard restores the terminal on any\n\
                  exit path (normal completion, Rhai error, panic).\n\
                  A best-effort SIGINT handler also restores on\n\
                  Ctrl-C.\n\
                  \n\
                  v1 limitations: no raw mode (resize handled by\n\
                  periodic poll); no PTY (subprocesses still detect\n\
                  \"not a terminal\"); lines truncated to pane width\n\
                  rather than wrapped; wide characters undercounted by\n\
                  one column.",
    flags: &[
        FlagHelp { flags: "tui::run(callback)", description: "Enter alt-screen, build a Dashboard, pass it to the callback.\nRestores terminal on any exit. stdout must be a TTY." },
        FlagHelp { flags: "d.split_vertical([p1, p2, …])", description: "Stack panes top-to-bottom. Each pi is a percentage in (0, 100].\nReturns an Array of PaneHandle. Last pane absorbs rounding." },
        FlagHelp { flags: "d.split_horizontal([p1, p2, …])", description: "Lay panes left-to-right. Same semantics as split_vertical." },
        FlagHelp { flags: "pane.println(line)", description: "Append a line to the pane's scrollback. Auto-scrolls to the\nbottom. Cap: 1000 lines per pane." },
        FlagHelp { flags: "pane.title(s)", description: "Set the pane's top-row title text. Rendered as ` <s> ` followed\nby a horizontal rule to the pane's right edge." },
        FlagHelp { flags: "pane.clear()", description: "Empty the pane's scrollback. Title is preserved." },
    ],
    related: &["script", "shell", "scripting", "threads"],
    examples: &[
        ExampleHelp { description: "Run the shipped demo", command: "recon --script script/tui.rhai" },
        ExampleHelp { description: "Two-pane update script", command: r#"recon --script - <<< 'tui::run(|d| { let p = d.split_vertical([70, 30]); let main = p[0]; let status = p[1]; main.title("output"); status.title("progress"); status.println("brew…"); shell_stream("brew upgrade", |l| main.println(l)); status.println("done"); });'"# },
        ExampleHelp { description: "Three horizontal panes", command: r#"recon --script - <<< 'tui::run(|d| { let p = d.split_horizontal([33, 33, 34]); for i in 0..3 { p[i].title(`pane ${i}`); for j in 0..5 { p[i].println(`line ${j}`); } } sleep_ms(2000); });'"# },
    ],
};

static TOPIC_REPL: Topic = Topic {
    title: "Interactive REPL (--repl)",
    description: "Open an interactive Rhai prompt backed by the script engine.\n\
                  Every binding available in --script mode is available at the prompt:\n\
                  http(), hash_sha256(), encrypt_*, sqlite_*, and so on.\n\
                  \n\
                  State persists across lines: `let` bindings and `fn` definitions\n\
                  remain in scope until you `:reset` or exit.\n\
                  \n\
                  Multi-line input is detected automatically (open `{`, `(`, `\"`).\n\
                  Use `:paste` to force a multi-line capture when the auto-detector\n\
                  mis-classifies pasted content; lines accumulate until `:end`.\n\
                  \n\
                  Autoprint: bare expressions print their result automatically\n\
                  (Python/Node convention). Toggle off with `:set autoprint off`.\n\
                  \n\
                  Threading caveat: thread_spawn is not available in REPL mode\n\
                  because it requires a static AST handle. Calls return an error.\n\
                  Use --script for threaded workflows.",
    flags: &[
        FlagHelp {
            flags: "--repl",
            description: "Open the interactive prompt. Mutually exclusive with --script.\n\
                          Threading is disabled (script-only).",
        },
        FlagHelp {
            flags: "--repl-history <PATH>",
            description: "Override the history file (default ~/.recon/repl_history).\n\
                          Capped at ~1000 lines by rustyline.\n\
                          Loaded on launch; saved on clean exit (:quit, Ctrl-D).",
        },
        FlagHelp {
            flags: ":help",
            description: "Print the REPL cheat sheet (meta-commands, multi-line, autoprint).",
        },
        FlagHelp {
            flags: ":help <topic>",
            description: "Print `recon --help <topic>` content (http, jwt, ...) without leaving the REPL.",
        },
        FlagHelp {
            flags: ":load <path>",
            description: "Eval <path> in the current scope. let/fn defined in the file persist.\n\
                          Path resolves like --script: literal then ~/.recon/script/<path>[.rhai].",
        },
        FlagHelp {
            flags: ":run <path>",
            description: "Eval <path> in a fresh, throwaway scope. Prints the return value.\n\
                          REPL state untouched.",
        },
        FlagHelp {
            flags: ":paste",
            description: "Enter paste mode. Lines accumulate until `:end` alone on a line.\n\
                          Then compile + eval once.",
        },
        FlagHelp {
            flags: ":set <key> <val>",
            description: "Mutate flags. Keys: method, header (append), timeout, user-agent, autoprint (on|off).",
        },
        FlagHelp {
            flags: ":vars / :fns",
            description: ":vars lists bindings (consts + let). :fns lists user-defined functions.",
        },
        FlagHelp {
            flags: ":reset",
            description: "Clear user bindings and user functions. Keeps engine and history.",
        },
        FlagHelp {
            flags: ":save <path>",
            description: "Write this session's input lines to <path> with a timestamp header.",
        },
        FlagHelp {
            flags: ":save-tidy <path>",
            description: "Like :save, but appends missing `;` and drops entries that fail to compile.\n\
                          The result is a runnable script — recon --script <path> should succeed.",
        },
        FlagHelp {
            flags: ":functions [all]",
            description: "List every callable registered with the engine (probes, helpers, builders).\n\
                          Pass `all` to include the Rhai standard library. Alias: :function-list.",
        },
        FlagHelp {
            flags: ":history [N] / :!N",
            description: ":history [N] prints the last N inputs (default 20).\n\
                          :!N re-runs entry N (1-based).",
        },
        FlagHelp {
            flags: ":edit",
            description: "Open $EDITOR (fallback `vi`) with a temp .rhai file.\n\
                          Eval the contents on save+quit.",
        },
        FlagHelp {
            flags: ":time <expr>",
            description: "Evaluate <expr> and print the elapsed wall-clock time.",
        },
        FlagHelp {
            flags: ":quit / :exit",
            description: "Save history file and exit with code 0. Ctrl-D does the same.",
        },
    ],
    related: &["script", "scripting", "threads"],
    examples: &[],
};

static TOPIC_DECODE: Topic = Topic {
    title: "Barcode / QR / DataMatrix decoding (--decode)",
    description: "Scan an image for a barcode and print the embedded\n\
                  text. Supports: QR, DataMatrix, Aztec, PDF417,\n\
                  MaxiCode, Code128, Code39, Code93, Codabar, EAN-13,\n\
                  EAN-8, ITF, UPC-A, UPC-E, RSS-14, RSS-Expanded.\n\
                  \n\
                  Backed by the `rxing` crate (a pure-Rust port of\n\
                  ZXing, the canonical multi-format decoder).\n\
                  \n\
                  Output format: `<FORMAT>\\t<TEXT>` to stdout. Use\n\
                  `--decode-hints` to restrict the scan to specific\n\
                  formats — speeds things up and avoids ambiguity\n\
                  when codes share prefixes.",
    flags: &[
        FlagHelp { flags: "--decode <IMAGE>", description: "Image path, or `-` to read the image bytes from stdin." },
        FlagHelp { flags: "--decode-hints <LIST>", description: "Comma-separated format restriction: qr, datamatrix, aztec, pdf417, maxicode,\ncode128, code39, code93, codabar, ean13, ean8, itf, upca, upce, rss14." },
        FlagHelp { flags: "encode::decode(blob)", description: "Script binding. Takes PNG/JPEG/WebP bytes already in memory and returns\n#{ text, format }." },
    ],
    related: &["encode", "encoding"],
    examples: &[
        ExampleHelp { description: "Decode a QR PNG", command: "recon --decode ticket.png" },
        ExampleHelp { description: "Read image from stdin", command: "cat code.jpg | recon --decode -" },
        ExampleHelp { description: "Restrict to EAN-13 for a product barcode", command: "recon --decode bottle.jpg --decode-hints ean13" },
        ExampleHelp { description: "Restrict to QR + DataMatrix (ambiguous dense codes)", command: "recon --decode mystery.png --decode-hints qr,datamatrix" },
        ExampleHelp { description: "Round-trip encode → decode", command: "recon --encode qr -o /tmp/q.png 'round-trip test' && recon --decode /tmp/q.png" },
    ],
};

static TOPIC_CLIENT_CERT: Topic = Topic {
    title: "Client certificates (mTLS)",
    description: "Present a client certificate during the TLS handshake\n\
                  (mutual TLS / mTLS). Works for any https:// URL; the\n\
                  server must be configured to request a client cert.\n\
                  \n\
                  Two PEM layouts are accepted:\n\
                    * Combined: one file contains both the CERTIFICATE\n\
                      chain and the PRIVATE KEY block. Pass it via\n\
                      --client-cert; leave --client-key unset.\n\
                    * Split: cert in one file, key in another. Pass both.\n\
                  \n\
                  Under the hood, recon builds a single PEM bundle and\n\
                  hands it to rustls via reqwest's `Identity::from_pem`.\n\
                  \n\
                  Non-PEM formats (DER) and encrypted PKCS#8 keys are\n\
                  detected at load time and rejected with a clear\n\
                  message pointing to a `openssl` conversion recipe.\n\
                  Rustls has no crypto-engine concept, so --key-type ENG\n\
                  errors immediately.",
    flags: &[
        FlagHelp { flags: "-E, --client-cert <PATH>", description: "PEM-encoded client certificate. May contain the key inline." },
        FlagHelp { flags: "--client-key <PATH>", description: "PEM-encoded private key. Only needed when --client-cert is cert-only." },
        FlagHelp { flags: "--cert-type <PEM|DER>", description: "Format of --client-cert (default PEM). DER support deferred under rustls." },
        FlagHelp { flags: "--key-type <PEM|DER|ENG>", description: "Format of --client-key. Only PEM is honored; DER and ENG error cleanly." },
        FlagHelp { flags: "--pass <PASS>", description: "Passphrase placeholder. Encrypted PKCS#8 keys are not yet decrypted\ninternally — convert externally via `openssl pkcs8`." },
        FlagHelp { flags: "http(url, #{client_cert, client_key, pass})", description: "Script-binding equivalent." },
    ],
    related: &["-k / --insecure", "--cacert", "--tlsv1.2 / --tlsv1.3"],
    examples: &[
        ExampleHelp { description: "Combined cert + key file", command: "recon --client-cert ~/keys/client-bundle.pem https://mtls.example.com/" },
        ExampleHelp { description: "Split cert and key", command: "recon -E ~/keys/client.crt --client-key ~/keys/client.key https://mtls.example.com/" },
        ExampleHelp { description: "badssl.com mTLS sandbox (client-cert required)", command: "recon -E badssl.com.pem https://client.badssl.com/" },
        ExampleHelp { description: "Script binding", command: r#"recon --script - <<< 'http("https://mtls.example.com/", #{ client_cert: "/path/bundle.pem" });'"# },
    ],
};

static TOPIC_COMPARE: Topic = Topic {
    title: "Source comparison (--compare A B)",
    description: "Diff two sources side-by-side. Each source is a URL, a\n\
                  local path, or `-` for stdin. HTTP(S) sources flow\n\
                  through the same pipeline as a normal request and\n\
                  honor all existing flags (-H, -u, -L, -k, cookies,\n\
                  proxy, …).\n\
                  \n\
                  Exit codes follow the GNU diff convention:\n\
                    0  identical\n\
                    1  differ\n\
                    2+ source load error (e.g. network failure)\n\
                  \n\
                  Binary content is detected by a NUL-byte probe in the\n\
                  first 8 KiB and reported as a byte-count delta instead\n\
                  of a line diff.",
    flags: &[
        FlagHelp { flags: "--compare <A> <B>", description: "Two sources to diff. Each is a URL / path / `-` (stdin)." },
        FlagHelp { flags: "--compare-format <FMT>", description: "Output format: `unified` (default, curl-style +/- diff),\n`summary` (one-liner), `sxs` (column-wrapped side-by-side)." },
        FlagHelp { flags: "--compare-context <N>", description: "Unified-diff context lines around each hunk (default 3)." },
        FlagHelp { flags: "compare(a, b)", description: "Script binding. Takes two Blobs (or strings) already in\nmemory and returns a map with `identical`, `added`,\n`removed`, `binary`, `a_bytes`, `b_bytes`, `diff`." },
    ],
    related: &["http", "output", "script"],
    examples: &[
        ExampleHelp { description: "Compare two local files", command: "recon --compare one.json two.json" },
        ExampleHelp { description: "Compare a URL against a local baseline", command: "recon --compare https://api.example.com/v1/status ./baseline.json" },
        ExampleHelp { description: "Stdin vs a file", command: "curl -s https://a/ | recon --compare - ./b.txt" },
        ExampleHelp { description: "Just tell me if they differ", command: "recon --compare a b --compare-format summary" },
        ExampleHelp { description: "Side-by-side for visual scan", command: "recon --compare a b --compare-format sxs" },
        ExampleHelp { description: "Compare a POST response to a baseline, follow redirects", command: "recon --compare https://a/ ./ref.txt -L -H 'Accept: text/plain'" },
    ],
};

static TOPIC_HSTS: Topic = Topic {
    title: "HSTS (HTTP Strict Transport Security cache)",
    description: "Persistent cache of `Strict-Transport-Security` directives.\n\
                  When `--hsts <file>` is set:\n\
                  \n\
                    * Before sending: if the target is http:// and the\n\
                      hostname has a non-expired HSTS entry, upgrade to\n\
                      https://. A verbose line is printed announcing the\n\
                      upgrade (suppressed by -s).\n\
                    * After receiving: parse the response's STS header,\n\
                      update the cache, save atomically.\n\
                  \n\
                  File format (compatible with curl's --hsts):\n\
                    # comment\n\
                    example.com 1756492800\n\
                    .app        1724956800   (leading '.' = includeSubDomains)\n\
                  \n\
                  Missing files are silently treated as empty — safe\n\
                  first-run UX.",
    flags: &[
        FlagHelp { flags: "--hsts <PATH>", description: "Load + update this HSTS cache file for every request. Set\nrepeatedly to share one cache across scripts / shell wrappers." },
        FlagHelp { flags: "-k, --insecure", description: "HSTS upgrade still happens when -k is set, but cert\nverification is disabled after the upgrade. Useful for\ntesting but a security risk in production." },
        FlagHelp { flags: "http(url, #{hsts: \"/path/to/file\"})", description: "Script-binding equivalent. Same semantics as the CLI flag." },
    ],
    related: &["-k / --insecure", "--cacert", "cookies"],
    examples: &[
        ExampleHelp { description: "Use HSTS to upgrade http:// hits automatically", command: "recon --hsts ~/.recon/hsts.txt http://example.com/" },
        ExampleHelp { description: "Prime the cache from an https:// response", command: "recon --hsts ~/.recon/hsts.txt https://www.cloudflare.com/" },
        ExampleHelp { description: "Inspect the cache", command: "cat ~/.recon/hsts.txt" },
        ExampleHelp { description: "Script with a shared cache", command: r#"recon --script - <<< 'http("http://example.com/", #{ hsts: "/tmp/h.txt" });'"# },
    ],
};

static TOPIC_UNIX_SOCKET: Topic = Topic {
    title: "Unix-domain sockets (--unix-socket)",
    description: "Route the HTTP request over a local Unix-domain socket\n\
                  instead of TCP. The target URL's host + path are\n\
                  preserved; only the transport changes.\n\
                  \n\
                  URL grammar accepted:\n\
                    http://localhost/path   (host becomes the Host: header)\n\
                    https://api/v1/info     (host-only; no actual TLS)\n\
                    /v1.40/version          (path only; Host defaults to\n\
                                             `localhost`)\n\
                  \n\
                  Scope: HTTP/1.1 over UDS, hand-rolled. No HTTP/2 (local\n\
                  peers don't need it), no TLS (nonsensical over a local\n\
                  socket), no redirects (UDS endpoints don't redirect),\n\
                  no chunked transfer decoding.\n\
                  \n\
                  Common sockets: /var/run/docker.sock (Docker API),\n\
                  /run/systemd/private (systemd), /var/run/kubelet.sock\n\
                  (Kubernetes kubelet).",
    flags: &[
        FlagHelp { flags: "--unix-socket <PATH>", description: "Connect to this Unix-domain socket path instead of TCP.\nThe socket file must exist." },
        FlagHelp { flags: "-X, --request <METHOD>", description: "Override the HTTP method (default GET or POST-with-body)." },
        FlagHelp { flags: "-H, --header <H: V>", description: "Custom request headers. Same behaviour as the TCP path." },
        FlagHelp { flags: "-d, --data <DATA>", description: "Request body (String / @file / @- stdin). Defaults method\nto POST." },
        FlagHelp { flags: "--json <DATA>", description: "JSON body + auto Content-Type." },
        FlagHelp { flags: "-T, --upload-file <PATH>", description: "PUT the file as body." },
        FlagHelp { flags: "-o, --output <PATH>", description: "Save response body to a file. Default: stdout." },
        FlagHelp { flags: "-v / -I / --include", description: "Header visibility — same semantics as the TCP path." },
        FlagHelp { flags: "http(url, #{unix_socket: \"/path\"})", description: "Script binding equivalent. Pass the socket path through\nthe opts map; response shape identical to a normal http()." },
    ],
    related: &["-X", "-H", "-d", "--json", "-T", "-o", "protocols"],
    examples: &[
        ExampleHelp { description: "Docker API: ping + version", command: "recon --unix-socket /var/run/docker.sock http://localhost/_ping" },
        ExampleHelp { description: "Docker API: list containers", command: "recon --unix-socket /var/run/docker.sock -p http://localhost/v1.40/containers/json" },
        ExampleHelp { description: "Query by path only (Host defaults to `localhost`)", command: "recon --unix-socket /var/run/docker.sock /v1.40/version" },
        ExampleHelp { description: "POST to a systemd-activated service", command: r#"recon --unix-socket /run/my-service.sock -X POST --json '{"ok":true}' http://svc/submit"# },
        ExampleHelp { description: "Script-side", command: r#"recon --script - <<< 'http("http://localhost/_ping", #{ unix_socket: "/var/run/docker.sock" });'"# },
    ],
};

static TOPIC_PROXY: Topic = Topic {
    title: "Proxy (HTTP / HTTPS / SOCKS5)",
    description: "Route HTTP(S) requests through a proxy. Scheme on the\n\
                  proxy URL selects the type:\n\
                  \n\
                    http://proxy:8080     plain HTTP proxy (CONNECT)\n\
                    https://proxy:8443    TLS-to-proxy (TLS before CONNECT)\n\
                    socks5://proxy:1080   SOCKS5, server-side DNS\n\
                    socks5h://proxy:1080  SOCKS5, client-side DNS\n\
                  \n\
                  Env-var precedence (matches curl):\n\
                    https:// target  -> $HTTPS_PROXY / $https_proxy\n\
                    http://  target  -> $HTTP_PROXY  / $http_proxy\n\
                    either           -> $ALL_PROXY   / $all_proxy\n\
                  \n\
                  `--proxy` always beats any env var. `--noproxy` (or\n\
                  `$NO_PROXY`) provides a bypass list: comma-separated\n\
                  entries, with a leading-dot entry like `.internal`\n\
                  matching every subdomain; `*` bypasses all.",
    flags: &[
        FlagHelp { flags: "-x, --proxy <URL>", description: "Route through this proxy. See title for scheme handling." },
        FlagHelp { flags: "-U, --proxy-user <USER:PASS>", description: "Basic-auth credentials. Takes priority over proxy-URL userinfo." },
        FlagHelp { flags: "--noproxy <LIST>", description: "Comma-separated hosts that bypass the proxy. `.suffix` matches\nsubdomains; `*` bypasses all. Falls back to $NO_PROXY." },
        FlagHelp { flags: "--proxy-insecure", description: "Skip TLS verification on the https:// proxy connection.\nDoesn't affect the origin's TLS." },
        FlagHelp { flags: "--proxy-cacert <PATH>", description: "Additional PEM root for the https:// proxy connection.\nTrust-additive (doesn't replace system roots). Because\nreqwest 0.12 applies CA bundles globally, this root also\naffects the origin request." },
        FlagHelp { flags: "--proxy-pass <PASS>", description: "Passphrase for --proxy-key (HTTPS proxy mTLS).\nAccepted for curl parity; proxy mTLS passphrase support is\nnot exposed by reqwest 0.12. Emits a runtime warning.\nSee OUT-OF-SCOPE.md (Deferred)." },
        FlagHelp { flags: "http(url, #{proxy, proxy_user, noproxy, proxy_insecure, proxy_cacert})", description: "Script-binding equivalents — same semantics as the CLI\nflags, routed through `http(url, opts)`." },
    ],
    related: &["-k / --insecure", "--cacert", "protocols"],
    examples: &[
        ExampleHelp { description: "Route through a corporate HTTP proxy", command: "recon --proxy http://proxy.corp:3128 https://example.com/" },
        ExampleHelp { description: "Authenticated proxy", command: "recon --proxy http://proxy.corp:3128 --proxy-user alice:secret https://example.com/" },
        ExampleHelp { description: "TLS-to-proxy (https:// proxy)", command: "recon --proxy https://secure-proxy.corp:8443 https://example.com/" },
        ExampleHelp { description: "SOCKS5 tunnel with client-side DNS", command: "recon --proxy socks5h://127.0.0.1:9050 https://example.com/" },
        ExampleHelp { description: "Bypass internal hosts", command: "recon --proxy http://corp-proxy --noproxy '.internal,localhost,127.0.0.1' https://example.com/" },
        ExampleHelp { description: "Default-from-env", command: "HTTPS_PROXY=http://proxy.corp:3128 recon https://example.com/" },
        ExampleHelp { description: "Script-side opts", command: r#"recon --script - <<< 'http("https://example.com", #{ proxy: "socks5://127.0.0.1:1080" });'"# },
    ],
};

static TOPIC_IPFS: Topic = Topic {
    title: "IPFS / IPNS (gateway rewrite)",
    description: "`ipfs://CID[/path]` and `ipns://NAME[/path]` URLs are\n\
                  rewritten to `<gateway>/ipfs/CID[/path]` and dispatched\n\
                  through the existing HTTP pipeline. Every HTTP flag\n\
                  (-H, -o, -k, --compressed, …) applies verbatim.\n\
                  \n\
                  Default gateway: https://ipfs.io. Override via\n\
                  --ipfs-gateway <URL> or $RECON_IPFS_GATEWAY. Point it\n\
                  at http://127.0.0.1:8080 to use a local Kubo /\n\
                  IPFS-Desktop node for resolution.\n\
                  \n\
                  No native IPFS-protocol client — the pure-Rust\n\
                  ecosystem (rust-ipfs) is alpha with a large dep tree,\n\
                  and HTTP gateways are how the IPFS ecosystem actually\n\
                  serves content today.",
    flags: &[
        FlagHelp { flags: "ipfs://CID[/path]", description: "Rewritten to <gateway>/ipfs/CID[/path]." },
        FlagHelp { flags: "ipns://NAME[/path]", description: "Rewritten to <gateway>/ipns/NAME[/path]. NAME can be a\npublic key hash, ENS name, or DNSLink domain (whatever the\ngateway resolves)." },
        FlagHelp { flags: "--ipfs-gateway <URL>", description: "Override the default gateway. Also read from\n$RECON_IPFS_GATEWAY. Trailing slash tolerated." },
        FlagHelp { flags: "ipfs_url(url)", description: "Script binding. Returns the rewritten gateway URL without\nfetching, or throws when the input isn't ipfs:// / ipns://." },
        FlagHelp { flags: "ipfs_url(url, #{gateway})", description: "Override the default gateway from within a script." },
    ],
    related: &["-o", "--compressed", "--ipfs-gateway"],
    examples: &[
        ExampleHelp { description: "Fetch a public IPFS CID via the default gateway", command: "recon ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi -o out.bin" },
        ExampleHelp { description: "Resolve an IPNS name (DNSLink)", command: "recon ipns://ipfs.tech/" },
        ExampleHelp { description: "Use a local Kubo node", command: "recon ipfs://bafy... --ipfs-gateway http://127.0.0.1:8080" },
        ExampleHelp { description: "Switch gateway via env var for a whole session", command: "RECON_IPFS_GATEWAY=https://cloudflare-ipfs.com recon ipfs://bafy..." },
        ExampleHelp { description: "Script-side URL rewriting", command: r#"recon --script - <<< 'let u = ipfs_url("ipfs://bafy"); print(http(u).status);'"# },
    ],
};

static TOPIC_POP3: Topic = Topic {
    title: "POP3 / POP3S (mail retrieval)",
    description: "Connect to a POP3 server, read capabilities, optionally\n\
                  retrieve a specific message. URL path is the message\n\
                  number (N) to RETR, or empty for a pure probe.\n\
                  \n\
                  pop3://   default port 110, plain.\n\
                  pop3s://  default port 995, implicit TLS.\n\
                  --stls    upgrade a pop3:// connection via the STLS\n\
                            command after CAPA.",
    flags: &[
        FlagHelp { flags: "pop3://[user[:pass]@]host[:port]/", description: "Probe: CAPA + (with auth) STAT. Disconnect." },
        FlagHelp { flags: "pop3://user:pass@host/N", description: "RETR message N. Requires auth via userinfo or -u." },
        FlagHelp { flags: "pop3s://...", description: "Implicit TLS." },
        FlagHelp { flags: "--stls", description: "Negotiate STLS after CAPA (pop3:// only). Mirrors SMTP STARTTLS." },
        FlagHelp { flags: "-u, --user <USER:PASS>", description: "Auth source when URL has no userinfo." },
        FlagHelp { flags: "-k, --insecure", description: "Skip TLS verification (pop3s:// and --stls)." },
        FlagHelp { flags: "pop3(url) / pop3(url, opts)", description: "Script binding. opts: user, pass, stls, insecure,\ntimeout_ms. Returns #{host, port, tls, banner,\ncapabilities, message_count, total_bytes, message,\nconnect_ms}." },
    ],
    related: &["--stls", "-u", "-k", "protocols"],
    examples: &[
        ExampleHelp { description: "Capability probe (no auth)", command: "recon pop3s://pop.gmail.com/" },
        ExampleHelp { description: "Auth + mailbox stats", command: "recon pop3s://me@gmail.com:apppass@pop.gmail.com/" },
        ExampleHelp { description: "Retrieve message 3", command: "recon pop3s://me:pass@pop.example.com/3" },
        ExampleHelp { description: "STARTTLS upgrade on plain pop3", command: "recon pop3://me:pass@mail.example.com/ --stls" },
    ],
};

static TOPIC_IMAP: Topic = Topic {
    title: "IMAP / IMAPS (mail retrieval)",
    description: "Connect to an IMAP server, report capabilities,\n\
                  optionally select a mailbox or fetch a message.\n\
                  \n\
                  URL path grammar (curl-compatible):\n\
                    imap://host/                -> CAPABILITY + LIST\n\
                    imap://host/INBOX           -> EXAMINE INBOX\n\
                    imap://host/INBOX;UID=N     -> FETCH UID N body\n\
                  \n\
                  imap://   default port 143; STARTTLS negotiated when\n\
                            the server advertises it.\n\
                  imaps://  default port 993, implicit TLS.",
    flags: &[
        FlagHelp { flags: "imap://[user[:pass]@]host[:port]/[MAILBOX[;UID=N]]", description: "Probe / select / fetch based on the path." },
        FlagHelp { flags: "imaps://...", description: "Implicit TLS." },
        FlagHelp { flags: "--imap-peek", description: "Use BODY.PEEK[] when fetching so the server doesn't mark\nthe message \\Seen." },
        FlagHelp { flags: "-u, --user <USER:PASS>", description: "Auth source when URL has no userinfo." },
        FlagHelp { flags: "imap(url) / imap(url, opts)", description: "Script binding. opts: user, pass, insecure, peek.\nReturns #{host, port, tls, capabilities, mailbox?,\nexists?, recent?, mailboxes?, uid?, body?}." },
    ],
    related: &["--imap-peek", "protocols"],
    examples: &[
        ExampleHelp { description: "Capability probe (no auth)", command: "recon imaps://imap.gmail.com/" },
        ExampleHelp { description: "List mailboxes", command: "recon imaps://me%40gmail.com:apppass@imap.gmail.com/" },
        ExampleHelp { description: "EXAMINE INBOX", command: "recon imaps://me:pass@imap.example.com/INBOX" },
        ExampleHelp { description: "Fetch UID 42 without marking \\Seen", command: "recon imaps://me:pass@imap.example.com/INBOX;UID=42 --imap-peek" },
    ],
};

static TOPIC_FTP: Topic = Topic {
    title: "FTP / FTPS (file transfer)",
    description: "Probe + retrieve against an FTP or FTPS server.\n\
                  Path semantics match curl:\n\
                    ftp://host/              -> list root\n\
                    ftp://host/dir/          -> list directory\n\
                    ftp://host/dir/file      -> retrieve file to -o or stdout\n\
                  Auth priority: URL userinfo > -u user:pass > anonymous.\n\
                  FTPS uses explicit AUTH TLS by default.",
    flags: &[
        FlagHelp { flags: "ftp://[user[:pass]@]host[:port]/path", description: "Plain FTP. Default port 21." },
        FlagHelp { flags: "ftps://...", description: "Explicit FTPS (AUTH TLS upgrade on port 21)." },
        FlagHelp { flags: "--ftp-active", description: "Use active mode (PORT) instead of passive (PASV / EPSV)." },
        FlagHelp { flags: "--ftps-implicit", description: "Accept implicit FTPS intent. Currently falls back to explicit\nAUTH TLS with a warning; revisit when a server forces it." },
        FlagHelp { flags: "--list-only", description: "Use NLST instead of LIST (filenames only).\nFaster, less detail." },
        FlagHelp { flags: "-Q, --quote <CMD>", description: "Send CMD as a custom FTP command before listing.\nRepeatable. Pre-transfer commands run in order." },
        FlagHelp { flags: "--ftp-skip-pasv-ip", description: "Use control-connection IP for PASV data channel,\nignoring server-advertised PASV IP. Helps when the\nserver sits behind NAT and reports a private IP." },
        FlagHelp { flags: "--ftp-pasv", description: "Confirm passive-mode FTP (suppaftp 6 default)." },
        FlagHelp { flags: "--disable-epsv", description: "Confirm classic PASV over EPSV (already default)." },
        FlagHelp { flags: "--disable-eprt", description: "Confirm passive mode (recon doesn't use EPRT)." },
        FlagHelp { flags: "-u, --user <USER:PASS>", description: "Auth when the URL has no userinfo. Defaults to anonymous." },
        FlagHelp { flags: "-k, --insecure", description: "Skip TLS certificate verification (ftps:// only)." },
        FlagHelp { flags: "-o, --output <PATH>", description: "Destination for retrieve mode. Defaults to stdout." },
        FlagHelp { flags: "ftp(url) / ftp(url, opts)", description: "Script binding. opts: user, pass, passive, implicit_tls,\ninsecure, timeout_ms, list_only, quote, ftp_skip_pasv_ip,\ndisable_epsv, disable_eprt, ftp_pasv. Returns #{host, port,\ntls, user, connect_ms, welcome, pwd, mode, listing? | bytes?}." },
    ],
    related: &["--connect-timeout", "-k", "-o", "protocols"],
    examples: &[
        ExampleHelp { description: "List a public FTP mirror", command: "recon ftp://ftp.gnu.org/gnu/" },
        ExampleHelp { description: "Retrieve a specific file", command: "recon ftp://ftp.gnu.org/gnu/ls.sig -o ls.sig" },
        ExampleHelp { description: "FTPS with URL-embedded credentials", command: "recon ftps://demo:password@test.rebex.net/" },
        ExampleHelp { description: "FTPS with -u and -k for a self-signed cert", command: "recon ftps://example.com/ -u alice:secret -k" },
        ExampleHelp { description: "Script-side probe", command: r#"recon --script - <<< 'let r = ftp("ftp://ftp.gnu.org/gnu/"); print(r.listing.len());'"# },
    ],
};

static TOPIC_SFTP: Topic = Topic {
    title: "SFTP (SSH file transfer)",
    description: "Probe + retrieve via SSH's SFTP subsystem. Shares the\n\
                  SSH auth scaffolding with scp:// and ssh:// (key\n\
                  identity via --ssh-key, host-key verification via -k /\n\
                  ~/.ssh/known_hosts).\n\
                  \n\
                  Path semantics match curl:\n\
                    sftp://user@host/            -> list home directory\n\
                    sftp://user@host/dir/        -> list that directory\n\
                    sftp://user@host/file        -> retrieve file",
    flags: &[
        FlagHelp { flags: "sftp://[user[:pass]@]host[:port]/path", description: "SSH-backed file transfer. Default port 22." },
        FlagHelp { flags: "--ssh-key <PATH>", description: "SSH private key for authentication. Reused from ssh:// / scp://." },
        FlagHelp { flags: "--ssh-pass <PASS>", description: "Passphrase for an encrypted private key." },
        FlagHelp { flags: "-u, --user <USER:PASS>", description: "Password auth (when the URL has no userinfo). Keys preferred." },
        FlagHelp { flags: "-k, --insecure", description: "Skip SSH host-key verification against ~/.ssh/known_hosts." },
        FlagHelp { flags: "sftp(url) / sftp(url, opts)", description: "Script binding. opts: insecure, timeout_ms, ssh_key. Returns\n#{host, port, user, connect_ms, path, mode,\nlisting? | bytes?}. Listing entries are\n#{name, size, is_dir, mode}." },
    ],
    related: &["--ssh-key", "scp"],
    examples: &[
        ExampleHelp { description: "List the home directory on a demo server", command: "recon sftp://demo:password@test.rebex.net/" },
        ExampleHelp { description: "Retrieve a file with key auth", command: "recon sftp://alice@host/home/alice/report.pdf --ssh-key ~/.ssh/id_ed25519 -o report.pdf" },
    ],
};

static TOPIC_TFTP: Topic = Topic {
    title: "TFTP (UDP file transfer)",
    description: "RFC 1350 Trivial File Transfer Protocol — UDP-based read.\n\
                  Upload (WRQ) is not implemented; this is a read-only probe.\n\
                  \n\
                  URL: tftp://host[:port]/filename (default port 69).\n\
                  Block size negotiation (RFC 2348) via --tftp-blksize.",
    flags: &[
        FlagHelp { flags: "tftp://host[:port]/filename", description: "TFTP read request (RRQ) against the named file. UDP; the\nserver replies from a new ephemeral port, so firewalls that\nlock down UDP by source port will drop the transfer." },
        FlagHelp { flags: "--tftp-blksize <N>", description: "RFC 2348 block-size option (default 512). Larger blocks\nmean fewer round-trips; servers that don't support the\noption fall back to 512." },
        FlagHelp { flags: "--tftp-no-options", description: "Confirm vanilla RFC 1350 mode (no RFC 2347 option\nnegotiation). recon's TFTP probe is already RFC 1350\nonly; this flag emits a verbose-mode note at -v." },
        FlagHelp { flags: "-o, --output <PATH>", description: "Destination file. Defaults to stdout." },
        FlagHelp { flags: "--connect-timeout <SECS>", description: "Per-packet deadline (recv timeout on the UDP socket)." },
        FlagHelp { flags: "tftp(url) / tftp(url, opts)", description: "Script binding. opts: blksize, timeout_ms. Returns\n#{host, port, filename, blksize, bytes, connect_ms}." },
    ],
    related: &["--connect-timeout", "-o"],
    examples: &[
        ExampleHelp { description: "Fetch a file from a local TFTP server", command: "recon tftp://127.0.0.1/boot/image.bin -o image.bin" },
        ExampleHelp { description: "With larger block size for faster transfer", command: "recon tftp://server/firmware.bin --tftp-blksize 1428 -o fw.bin" },
    ],
};

static TOPIC_GOPHER: Topic = Topic {
    title: "Gopher (RFC 1436)",
    description: "Fetch a selector from a Gopher server and stream the\n\
                  response bytes to stdout (or -o). URL grammar:\n\
                    gopher://host[:port]/[TYPE]/selector\n\
                  TYPE is a single RFC 1436 item type character (0 text,\n\
                  1 directory, 7 search, …). When the path begins with a\n\
                  type character, that char is stripped before sending\n\
                  the selector.\n\
                  \n\
                  gophers:// is the same protocol over TLS.",
    flags: &[
        FlagHelp { flags: "gopher://host[:port]/[TYPE]/selector", description: "Plaintext Gopher. Default port 70." },
        FlagHelp { flags: "gophers://...", description: "Gopher over TLS. Default port 70; override via authority." },
        FlagHelp { flags: "-k, --insecure", description: "Skip TLS cert verification (gophers:// only)." },
        FlagHelp { flags: "-o, --output <PATH>", description: "Write response to a file. Defaults to stdout." },
        FlagHelp { flags: "gopher(url) / gopher(url, opts)", description: "Script binding. opts: insecure, timeout_ms. Returns\n#{host, port, tls, selector, item_type, connect_ms,\ncontent (String), bytes (Blob)}." },
    ],
    related: &["-o", "protocols"],
    examples: &[
        ExampleHelp { description: "Fetch the root of a classic Gopher server", command: "recon gopher://gopher.floodgap.com/" },
        ExampleHelp { description: "Fetch a type-0 text document", command: "recon gopher://gopher.floodgap.com/0/gopher/proxy" },
    ],
};

static TOPIC_SMTP: Topic = Topic {
    title: "SMTP / SMTPS probe + mail delivery (with DKIM)",
    description: "Probe an SMTP server or deliver a test message. Two\n\
                  modes based on which flags are set:\n\
                  \n\
                    Probe mode (default): connect, read greeting, send\n\
                    EHLO, report advertised extensions + AUTH methods +\n\
                    STARTTLS availability, disconnect.\n\
                  \n\
                    Send mode (when --mail-from + --mail-to are given):\n\
                    full transaction via lettre — EHLO → STARTTLS (or\n\
                    implicit TLS on smtps://) → AUTH → MAIL → RCPT →\n\
                    DATA → QUIT. Optional DKIM signing with a local key.\n\
                  \n\
                  URL schemes:\n\
                    smtp://HOST[:PORT]/   (plain, default port 25;\n\
                                           upgrades via STARTTLS unless\n\
                                           --no-starttls)\n\
                    smtps://HOST[:PORT]/  (implicit TLS, default port 465)\n\
                  \n\
                  Complements the DNS-based email checks (spf, dmarc,\n\
                  dkim, mta-sts, bimi, tls-rpt) by exercising the wire,\n\
                  not just the DNS records.",
    flags: &[
        FlagHelp { flags: "--mail-from <ADDR>", description: "Envelope sender for MAIL FROM:<…>. Required in send mode." },
        FlagHelp { flags: "--mail-to <ADDR>", description: "Envelope recipient. Repeatable for multi-recipient delivery.\nRequired in send mode." },
        FlagHelp { flags: "--mail-subject <STR>", description: "Subject header. Default: \"recon SMTP test\"." },
        FlagHelp { flags: "--mail-body <STR>", description: "Body. Accepts @file to load from file, @- from stdin, or the\nliteral text. Default: one-line test note." },
        FlagHelp { flags: "--mail-header <H: V>", description: "Additional message header (Reply-To, X-*, etc.). Repeatable." },
        FlagHelp { flags: "--smtp-auth <USER:PASS>", description: "Credentials. Tries AUTH PLAIN then LOGIN. Exit 67 on\nrejection." },
        FlagHelp { flags: "--smtp-helo <NAME>", description: "HELO / EHLO hostname to advertise. Default: `recon.local`." },
        FlagHelp { flags: "--mail-auth <ADDR>", description: "Append AUTH=<ADDR> to MAIL FROM (RFC 4954). Currently\naccepted but emits a warning — lettre 0.11 high-level\nAPI does not expose envelope parameters; deferred." },
        FlagHelp { flags: "--no-starttls", description: "Skip STARTTLS upgrade on smtp://. Useful for probing a\nserver's plaintext behaviour." },
        FlagHelp { flags: "--dkim-key <PATH>", description: "PEM-encoded RSA or Ed25519 private key. Enables DKIM signing\non outbound messages. Requires --dkim-selector." },
        FlagHelp { flags: "--dkim-selector <SEL>", description: "DKIM selector — the `s=` tag. Matches the selector in the\ncorresponding DNS TXT record at\n<selector>._domainkey.<domain>." },
        FlagHelp { flags: "--dkim-domain <DOMAIN>", description: "Signing domain (the `d=` tag). Defaults to the domain part\nof --mail-from." },
        FlagHelp { flags: "-u, --user <USER:PASS>", description: "Alternative credentials source. --smtp-auth takes priority\nwhen both are set." },
        FlagHelp { flags: "-k, --insecure", description: "Skip TLS certificate verification on smtps:// or STARTTLS." },
        FlagHelp { flags: "--connect-timeout <SECS>", description: "Per-operation deadline (connect, EHLO reply, etc.)." },

        FlagHelp { flags: "smtp(url [, opts])", description: "Script binding. opts mirrors the CLI flags with snake_case\nkeys: mail_from, mail_to (Array), mail_subject, mail_body,\nmail_header (Array), smtp_auth, smtp_helo, no_starttls,\ndkim_key, dkim_selector, dkim_domain, insecure, timeout_ms.\nReturns #{host, port, tls, connect_ms, banner,\ncapabilities, auth_methods, starttls_ok, send_result}." },
    ],
    related: &["--spf", "--dmarc", "--dkim", "--mta-sts", "--bimi", "--tls-rpt", "protocols"],
    examples: &[
        ExampleHelp { description: "Probe capabilities of a mail server (no auth, no send)", command: "recon smtp://smtp.gmail.com:587/" },
        ExampleHelp { description: "Probe SMTPS on port 465", command: "recon smtps://mail.example.com/" },
        ExampleHelp { description: "Deliver a test message through a local relay", command: r#"recon smtp://localhost:25/ --mail-from me@example.com --mail-to you@example.com --mail-subject 'hi' --mail-body 'test'"# },
        ExampleHelp { description: "Authenticated send via submission port", command: r#"recon smtp://smtp.gmail.com:587/ --smtp-auth user@gmail.com:apppass --mail-from me@gmail.com --mail-to you@example.com --mail-body 'hi'"# },
        ExampleHelp { description: "DKIM-sign an outgoing test message", command: r#"recon smtp://localhost:25/ --mail-from me@example.com --mail-to you@example.com --mail-body 'signed' --dkim-key dkim.pem --dkim-selector recon1 --dkim-domain example.com"# },
        ExampleHelp { description: "Read the body from a file", command: r#"recon smtp://localhost:25/ --mail-from me@… --mail-to you@… --mail-body @message.txt"# },
        ExampleHelp { description: "Probe without triggering a STARTTLS upgrade", command: "recon smtp://localhost:25/ --no-starttls" },
        ExampleHelp { description: "Script-side probe", command: r#"recon --script - <<< 'let r = smtp("smtp://localhost:25/"); for c in r.capabilities { print(c); }'"# },
    ],
};

static TOPIC_TEXT_ENCODING: Topic = Topic {
    title: "Text Encoding (charsets, iconv)",
    description: "Convert response and request bodies between character sets.\n\
                  Useful when one end of a pipeline speaks UTF-8 and the\n\
                  other speaks ISO-8859-1 / Windows-1252 / Shift-JIS etc.\n\
                  \n\
                  Source-charset resolution priority:\n\
                    1. `--source-charset NAME` (explicit override)\n\
                    2. `Content-Type: ...; charset=NAME` on the response\n\
                    3. BOM sniff (UTF-8 / UTF-16)\n\
                    4. chardetng heuristic\n\
                    5. windows-1252 fallback (browser behaviour)\n\
                  \n\
                  Unmappable characters are substituted with `?` and a\n\
                  warning is printed to stderr (suppressed by `-s`).\n\
                  \n\
                  Scripts get direct access: `r.body_bytes` (raw Blob) and\n\
                  `r.charset` (String or `()`) on every http() / browser()\n\
                  response, plus the `text::*` module for conversion.",
    flags: &[
        FlagHelp { flags: "--output-charset <NAME>", description: "Transcode the response body to NAME before prettify / write.\nPass-through when the source is already NAME. Example:\n--output-charset utf-8 against a Latin-1 server." },
        FlagHelp { flags: "--source-charset <NAME>", description: "Assume the response body is in NAME, overriding any charset=\nthe server declared (or when none was declared). Use when a\nserver lies about its content — e.g. labels windows-1252 as\niso-8859-1." },
        FlagHelp { flags: "--to-utf8", description: "Shorthand for `--output-charset utf-8`. Convenient when the\nprimary goal is \"give me sensible UTF-8 regardless of what the\nserver sent\"." },
        FlagHelp { flags: "--request-charset <NAME>", description: "Transcode the request body from UTF-8 (the shell's native\nencoding) to NAME before sending. Takes priority over any\ncharset= in an explicit Content-Type header." },
        FlagHelp { flags: "--request-charset-passthrough", description: "Skip auto-transcoding the request body even when the explicit\nContent-Type header declares a charset. Use when the body was\nread from a pre-encoded file and must be sent as-is." },
        FlagHelp { flags: "--iconv <SOURCE:TARGET>", description: "Standalone conversion action (no HTTP). Reads the positional\narg as a file path (or stdin when absent), transcodes, writes\nto -o PATH (or stdout). SOURCE blank means auto-detect.\nExamples:\n  recon --iconv iso-8859-1:utf-8 input.txt -o out.txt\n  cat input | recon --iconv :utf-8 > out.txt" },
        FlagHelp { flags: "--list-charsets", description: "Dump a curated list of recognised charset labels and exit. The\nunderlying encoding_rs library accepts many more aliases; these\nare the ones you're likely to reach for." },

        FlagHelp { flags: "text::transcode(blob, from, to)", description: "Script binding. Convert bytes between any two supported\ncharsets. Returns a Blob. Unmappable characters substituted." },
        FlagHelp { flags: "text::decode(blob, charset)", description: "Decode bytes to a UTF-8 String using the given source charset." },
        FlagHelp { flags: "text::encode(str, charset)", description: "Encode a UTF-8 String into bytes in the target charset." },
        FlagHelp { flags: "text::detect(blob)", description: "Sniff the source charset. Returns #{charset, had_bom}.\nUses BOM first, then chardetng heuristic." },
        FlagHelp { flags: "text::charset_of(headers_map)", description: "Pull charset= out of a response headers map. Returns the\ncharset String, or () when absent." },
        FlagHelp { flags: "text::strip_bom(blob) / text::list()", description: "Drop a leading UTF-8/16 BOM if present; enumerate common charsets." },
        FlagHelp { flags: "text::normalize_newlines(str, style)", description: "Rewrite line endings. style: `lf` / `crlf` / `cr`\n(or `unix` / `windows` / `mac`)." },

        FlagHelp { flags: "r.body_bytes / r.charset (in http() + browser() responses)", description: "Script bindings' response Map gains raw bytes and the resolved\ncharset alongside the existing lossy `r.body` String." },
    ],
    related: &["--output-charset", "--source-charset", "--request-charset", "--iconv", "-p"],
    examples: &[
        ExampleHelp { description: "Convert a Latin-1 response to UTF-8", command: "recon --to-utf8 https://legacy.example.com/api" },
        ExampleHelp { description: "Prettify a Shift_JIS page (forces UTF-8 before prettify)", command: "recon -p --output-charset utf-8 https://legacy.jp/index.html" },
        ExampleHelp { description: "POST UTF-8 form data to a Perl service that expects ISO-8859-1", command: r#"recon -X POST -H 'Content-Type: application/x-www-form-urlencoded; charset=iso-8859-1' -d 'name=Jörg' https://perl.example.com/submit"# },
        ExampleHelp { description: "Standalone file conversion", command: "recon --iconv iso-8859-1:utf-8 input.txt -o output.txt" },
        ExampleHelp { description: "Auto-detect source + convert to UTF-8 via stdin", command: "cat legacy.txt | recon --iconv :utf-8 > utf8.txt" },
        ExampleHelp { description: "Script-side re-decode when the CLI can't see the charset", command: r#"recon --script - <<< 'let r = http("https://legacy"); print(text::decode(r.body_bytes, "iso-8859-1"));'"# },
        ExampleHelp { description: "List supported charsets", command: "recon --list-charsets" },
    ],
};

static TOPIC_STRUTIL: Topic = Topic {
    title: "String helpers (trim, regex, sprintf, …)",
    description: "PHP-style free functions for working with strings in\n\
                  scripts and at the REPL. Adds the recognisable names\n\
                  alongside Rhai's existing String methods (which keep\n\
                  working). All functions are top-level callables, not\n\
                  namespaced — `trim(s)` reads the same as in PHP.\n\
                  \n\
                  Regex helpers are backed by the `regex` crate. They\n\
                  accept either a raw pattern or PHP-style delimited\n\
                  form (e.g. `/foo/i`) with the i / m / s / x flags.\n\
                  preg_match returns an Array of capture strings: index\n\
                  0 is the whole match, 1+ are groups; empty if no\n\
                  match.\n\
                  \n\
                  printf / sprintf accept three argument shapes per\n\
                  format: zero args, a single arg, or an Array. Rhai\n\
                  has no variadic concept, so multi-arg formats are\n\
                  passed as `[a, b, c]`. Supported specifiers: d i u o\n\
                  x X b f e E g G s c %% with -, 0, +, space, # flags,\n\
                  plus width and precision.",
    flags: &[
        FlagHelp { flags: "trim(s) / trim(s, mask)", description: "Strip whitespace (or any char in `mask`) from both ends." },
        FlagHelp { flags: "ltrim(s) / ltrim(s, mask)", description: "Strip from the left end only." },
        FlagHelp { flags: "rtrim(s) / rtrim(s, mask)", description: "Strip from the right end only." },
        FlagHelp { flags: "strrev(s)", description: "Reverse a string by Unicode codepoints — accented letters and\nemoji survive intact." },
        FlagHelp { flags: "strip_html(s)", description: "Remove every `<...>` segment. Quoted attribute values are\nrespected so `<a title=\"oh >no<\">` strips cleanly. HTML\nentities pass through untouched (matches PHP strip_tags)." },
        FlagHelp { flags: "nl2br(s)", description: "Insert `<br>` before every `\\n`, `\\r\\n`, or `\\r`. HTML5 form,\nno trailing slash. The original newline is preserved." },
        FlagHelp { flags: "br2nl(s)", description: "Replace `<br>` / `<br/>` / `<br />` (any case, any inner\nwhitespace) with `\\n`. If the tag is immediately followed by\nan EOL, that EOL is kept — so nl2br ↔ br2nl round-trips." },
        FlagHelp { flags: "preg_match(pattern, subject)", description: "Returns Array of captures: index 0 is the whole match, 1+ are\ngroups. Empty array when no match. Errors on invalid regex." },
        FlagHelp { flags: "preg_replace(pattern, replacement, subject)", description: "Replace every match. `$1` / `${name}` in `replacement` expand\nto captures, per the regex crate's default replacement syntax." },
        FlagHelp { flags: "arr.join(sep) / join(arr, sep)", description: "Concatenate an Array's elements with `sep` between them.\nNon-string elements are stringified via Dynamic::to_string." },
        FlagHelp { flags: "sprintf(fmt, args)", description: "Format and return a String. `args` is either a single value or\nan Array for multi-arg formats. Supports flags -, 0, +, space,\n#, plus width and precision." },
        FlagHelp { flags: "printf(fmt, args)", description: "Format and write to stdout. Returns the number of bytes\nwritten (matches C printf)." },
        FlagHelp { flags: "urlencode(s) / urldecode(s)", description: "RFC 3986 percent-encoding for query params and form values.\nurldecode errors on malformed `%xx` sequences." },
        FlagHelp { flags: "base64_encode(s | blob) / base64_decode(s)", description: "Standard base64 with `=` padding. Encode accepts either a\nString (encoded as UTF-8) or a Blob. Decode returns a Blob —\nconvert with text::decode(b, \"utf-8\") for a String." },
        FlagHelp { flags: "html_entity_decode(s)", description: "Decode HTML entities (`&amp;`, `&lt;`, `&#x27;`, numeric refs).\nNatural follow-up call after strip_html, which deliberately\nleaves entities alone." },
        FlagHelp { flags: "str_pad(s, width [, pad [, side]])", description: "Pad to `width` characters with `pad` (default space). `side` is\n\"left\", \"right\" (default), or \"both\". Width <= length leaves the\nstring alone." },
        FlagHelp { flags: "lpad(s, width [, pad]) / rpad(s, width [, pad])", description: "Bare-name aliases for the common left/right pad cases." },
        FlagHelp { flags: "dirname(path) / basename(path [, suffix])", description: "POSIX dirname/basename. Trailing slashes are stripped first.\nbasename's optional `suffix` is trimmed from the result (e.g.\n`basename(\"/var/log/recon.log\", \".log\")` → \"recon\")." },
        FlagHelp { flags: "date_format(unix_ts, fmt [, tz])", description: "Format a Unix timestamp via chrono's strftime spec. Default tz\nis UTC; pass \"local\" for the system timezone." },
    ],
    related: &["script", "scripting", "text"],
    examples: &[
        ExampleHelp { description: "Whitespace + custom mask", command: r#"recon --script - <<< 'print(trim("  hi  ")); print(ltrim("...path", "."));'"# },
        ExampleHelp { description: "Strip HTML and convert linebreaks", command: r#"recon --script - <<< 'print(strip_html("<p>plain <b>text</b></p>"));'"# },
        ExampleHelp { description: "Regex capture", command: r#"recon --script - <<< 'print(preg_match("/^Host:\\s*(.+)$/i", "Host: example.com"));'"# },
        ExampleHelp { description: "printf with multiple args", command: r#"recon --script - <<< 'printf("%-10s %5d\n", ["alpha", 42]);'"# },
        ExampleHelp { description: "URL-encode a query param", command: r#"recon --script - <<< 'print(urlencode("hello world & friends?"));'"# },
        ExampleHelp { description: "Base64 round-trip via a Blob", command: r#"recon --script - <<< 'let b = base64_decode("aGVsbG8="); print(text::decode(b, "utf-8"));'"# },
        ExampleHelp { description: "Decode HTML entities after stripping tags", command: r#"recon --script - <<< 'print(html_entity_decode(strip_html("<p>Tom &amp; Jerry</p>")));'"# },
        ExampleHelp { description: "Pad a number for column alignment", command: r#"recon --script - <<< 'print(str_pad("42", 6, "0", "left"));'"# },
        ExampleHelp { description: "Dirname / basename with suffix trim", command: r#"recon --script - <<< 'print(dirname("/var/log/recon.log")); print(basename("/var/log/recon.log", ".log"));'"# },
        ExampleHelp { description: "Format a unix timestamp (UTC + local)", command: r#"recon --script - <<< 'print(date_format(1700000000, "%Y-%m-%dT%H:%M:%SZ"));'"# },
    ],
};

static TOPIC_JQ: Topic = Topic {
    title: "jq filter (`jq` / `jq_all`)",
    description: "Apply a jq-style filter to any Rhai Map or Array.\n\
                  Backed by the `jaq` crate — full jq grammar including\n\
                  pipes, `select(...)`, `map(...)`, alternative `//`,\n\
                  arithmetic, and the standard-library functions.\n\
                  \n\
                  Two methods, differing only in shape:\n\
                  \n\
                    `obj.jq(filter)` — first result, or `()` if the\n\
                      filter yields nothing.\n\
                    `obj.jq_all(filter)` — every result as an Array.\n\
                  \n\
                  Both also callable as free functions: `jq(obj, f)`\n\
                  and `jq_all(obj, f)`.\n\
                  \n\
                  Strings are NOT auto-parsed — chain\n\
                  `json_parse(s).jq(filter)` when starting from JSON\n\
                  text. Filter parse and runtime errors throw and are\n\
                  catchable with `try` / `catch`.",
    flags: &[
        FlagHelp { flags: "obj.jq(filter) / jq(obj, filter)", description: "Returns the first result, or `()` if the filter yields no results." },
        FlagHelp { flags: "obj.jq_all(filter) / jq_all(obj, filter)", description: "Returns every result as an Array. Empty Array if nothing matches." },
    ],
    related: &["script", "scripting"],
    examples: &[
        ExampleHelp { description: "Run the shipped demo", command: "recon --script script/jq.rhai" },
        ExampleHelp { description: "First match", command: r#"recon --script - <<< 'print([#{n: 1}, #{n: 2}].jq(".[] | select(.n > 1) | .n"));'"# },
        ExampleHelp { description: "All matches", command: r#"recon --script - <<< 'print([1, 2, 3, 4].jq_all(".[] | select(. % 2 == 0)"));'"# },
        ExampleHelp { description: "From raw JSON text", command: r#"recon --script - <<< 'print(json_parse("{\"a\":[1,2,3]}").jq(".a[1]"));'"# },
    ],
};

static TOPIC_GIT: Topic = Topic {
    title: "git wrapper (`git()` / `git(path)`)",
    description: "First-class methods over the `git` CLI. Each method\n\
                  picks the right `--porcelain` / `--format` flags\n\
                  internally and parses the output into Rhai data.\n\
                  \n\
                  Constructors: `git()` binds to the current working\n\
                  directory; `git(path)` binds to a specific repo path.\n\
                  Methods return parsed Maps and Arrays for inspection,\n\
                  `()` for mutating ops, and a `{ hash, short_hash,\n\
                  subject }` Map for `.commit()`.\n\
                  \n\
                  Escape hatches: `.run(args)` sniffs the output (JSON\n\
                  shape returns a Map/Array, otherwise String).\n\
                  `.run_text(args)` and `.run_json(args)` are the\n\
                  explicit forms.\n\
                  \n\
                  Errors throw on non-zero exit with stderr truncated\n\
                  to ~2KB. Scripts use `try` / `catch` to recover.\n\
                  Composes on top of `std::process::Command` directly\n\
                  rather than going through the shell() binding.",
    flags: &[
        FlagHelp { flags: "git() / git(path)", description: "Construct a Git handle bound to cwd (no-arg) or a specific\nrepo path. The handle is Clone+Send+Sync." },
        FlagHelp { flags: "g.status()", description: "Returns Map { branch, upstream, ahead, behind, clean,\nstaged, unstaged, untracked }. Uses --porcelain=v2 --branch." },
        FlagHelp { flags: "g.is_clean()", description: "Convenience: g.status().clean." },
        FlagHelp { flags: "g.log(n) / g.log_range(rev_range)", description: "Returns Array<Map { hash, short_hash, author, email, date,\nsubject, body }>. ISO 8601 dates." },
        FlagHelp { flags: "g.diff() / g.diff(rev)", description: "Returns the patch as a String. Pair with rev for diff against\na specific commit." },
        FlagHelp { flags: "g.diff_stat() / g.diff_stat(rev)", description: "Returns Map { files, insertions, deletions, per_file:\n[{path, insertions, deletions}, ...] }." },
        FlagHelp { flags: "g.branch()", description: "Returns Map { current, upstream, all: Array<String> }.\nupstream is () when no upstream is set." },
        FlagHelp { flags: "g.rev_parse(name)", description: "Resolve a ref to its full 40-char SHA." },
        FlagHelp { flags: "g.remote()", description: "Returns Map of remote-name → URL (parsed from `git remote -v`)." },
        FlagHelp { flags: "g.add(path) / g.add([paths])", description: "Stage one path or an Array of paths." },
        FlagHelp { flags: "g.commit(message)", description: "Commit staged changes. Returns the new commit's\n{ hash, short_hash, subject } Map. Throws on empty index\nor pre-commit hook failure." },
        FlagHelp { flags: "g.push() / g.push(remote) / g.push(remote, branch)", description: "Push to upstream (no args), to remote, or to remote+branch." },
        FlagHelp { flags: "g.pull() / g.pull(remote, branch)", description: "Pull from upstream or remote+branch." },
        FlagHelp { flags: "g.checkout(name)", description: "Switch to a branch or commit." },
        FlagHelp { flags: ".run(args) / .run_text(args) / .run_json(args)", description: "Escape hatches. `.run()` sniffs JSON vs text; `.run_text()`\nand `.run_json()` are explicit. Args are a single string,\nshell-style quoted." },
    ],
    related: &["script", "scripting", "gh", "shell"],
    examples: &[
        ExampleHelp { description: "Run the shipped demo", command: "recon --script script/git.rhai" },
        ExampleHelp { description: "Quick branch check", command: r#"recon --script - <<< 'print(git().branch().current);'"# },
        ExampleHelp { description: "Last 5 commits", command: r#"recon --script - <<< 'for c in git().log(5) { print(`${c.short_hash} ${c.subject}`); }'"# },
        ExampleHelp { description: "Staging area summary", command: r#"recon --script - <<< 'let s = git().status(); print(`staged: ${s.staged.len()}, unstaged: ${s.unstaged.len()}, untracked: ${s.untracked.len()}`);'"# },
        ExampleHelp { description: "Custom git command via escape hatch", command: r#"recon --script - <<< 'print(git().run_text("log --oneline -3"));'"# },
    ],
};

static TOPIC_GH: Topic = Topic {
    title: "GitHub CLI wrapper (`gh()` / `gh(repo)`)",
    description: "First-class methods over the `gh` CLI. Each method\n\
                  picks the right `--json <fields>` flag and parses the\n\
                  output into Rhai Maps and Arrays.\n\
                  \n\
                  Constructors: `gh()` resolves the current repo from\n\
                  cwd; `gh(\"owner/name\")` targets a specific repo via\n\
                  `--repo`.\n\
                  \n\
                  Auto-account-switch: before every `gh` call, the\n\
                  wrapper reads `git config user.email` and runs\n\
                  `gh auth switch --user <handle>` when the active\n\
                  account doesn't match. The email-to-handle mapping\n\
                  is loaded from `$XDG_CONFIG_HOME/recon/gh-accounts.toml`\n\
                  (or set `$RECON_GH_ACCOUNTS_FILE` to override).\n\
                  Without the file, no switch happens. `auth_status()`\n\
                  is the lone exception — it queries whichever account\n\
                  is currently active without triggering a switch.\n\
                  \n\
                  Errors throw on non-zero exit. `gh pr view <id>`\n\
                  exiting 1 for \"not found\" is the canonical case\n\
                  scripts catch with `try` / `catch`.",
    flags: &[
        FlagHelp { flags: "gh() / gh(repo)", description: "Construct a Gh handle. `gh()` uses the cwd's repo;\n`gh(\"owner/name\")` adds --repo to every call." },
        FlagHelp { flags: "h.pr_list() / h.pr_list(opts)", description: "Returns Array<PR Map>. opts: state, author, label\n(string or Array), limit." },
        FlagHelp { flags: "h.pr_view(number)", description: "Returns Map with PR detail. Throws if not found." },
        FlagHelp { flags: "h.pr_create(opts)", description: "Returns { number, url }. opts: title (required), body OR\nbody_file (mutually exclusive), base, head, draft, reviewer,\nlabel." },
        FlagHelp { flags: "h.pr_merge(number) / h.pr_merge(number, opts)", description: "opts: method (merge/squash/rebase), delete_branch, auto." },
        FlagHelp { flags: "h.pr_close(number)", description: "Close a PR without merging." },
        FlagHelp { flags: "h.pr_comment(number, body)", description: "Post a comment on a PR." },
        FlagHelp { flags: "h.issue_list() / h.issue_list(opts)", description: "Returns Array<Issue Map>. opts: state, author, label\n(string or Array), assignee (string or Array), limit." },
        FlagHelp { flags: "h.issue_view(number)", description: "Returns Map with issue detail." },
        FlagHelp { flags: "h.issue_create(opts)", description: "Returns { number, url }. opts: title (required), body OR\nbody_file, label, assignee." },
        FlagHelp { flags: "h.issue_comment(number, body)", description: "Post a comment on an issue." },
        FlagHelp { flags: "h.release_list()", description: "Returns Array of release Maps." },
        FlagHelp { flags: "h.release_view(tag)", description: "Returns Map with release detail (assets included)." },
        FlagHelp { flags: "h.release_create(tag, opts)", description: "Returns { url, tag }. opts: title, notes OR notes_file,\ngenerate_notes, draft, prerelease, target." },
        FlagHelp { flags: "h.repo_view() / h.repo_view(spec)", description: "Returns Map with repo metadata." },
        FlagHelp { flags: "h.run_list() / h.run_list(opts)", description: "Workflow runs. opts: workflow, branch, status, limit." },
        FlagHelp { flags: "h.run_view(id)", description: "Single workflow run, including jobs." },
        FlagHelp { flags: "h.auth_status()", description: "Returns { host, account, scopes }. Does NOT trigger\nauto-switch (the only method that doesn't)." },
        FlagHelp { flags: ".run(args) / .run_text(args) / .run_json(args)", description: "Escape hatches. Same shape as the git wrapper." },
    ],
    related: &["script", "scripting", "git", "shell"],
    examples: &[
        ExampleHelp { description: "Run the shipped demo", command: "recon --script script/gh.rhai" },
        ExampleHelp { description: "Open PRs by an author", command: r#"recon --script - <<< 'for p in gh().pr_list(#{ state: "open", author: "@me" }) { print(`#${p.number} ${p.title}`); }'"# },
        ExampleHelp { description: "Create a release with auto-generated notes", command: r#"recon --script - <<< 'gh().release_create("v0.89.0", #{ generate_notes: true });'"# },
        ExampleHelp { description: "Check the active gh account", command: r#"recon --script - <<< 'print(gh().auth_status().account);'"# },
        ExampleHelp { description: "Raw gh call (escape hatch)", command: r#"recon --script - <<< 'print(gh().run_text("api repos/codedeviate/recon"));'"# },
    ],
};

static TOPIC_FLAGS: Topic = Topic {
    title: "Flag listing (`--flags`)",
    description: "A curl-style alphabetical listing of every flag.\n\
                  Format: `(short, ) --long <VALUE>  short description`,\n\
                  sorted by long name, one flag per line, descriptions\n\
                  capped at ~52 characters.\n\
                  \n\
                  Use `recon --flags` for the quick lookup; follow up\n\
                  with `recon --help <topic>` for the long-form\n\
                  explanation of any feature area.\n\
                  \n\
                  Paging: auto-paged through $PAGER when stdout is a\n\
                  TTY. Disable with --no-pager or $RECON_NO_PAGER.",
    flags: &[
        FlagHelp { flags: "--flags", description: "Print the alphabetical flag list and exit." },
        FlagHelp { flags: "--no-pager", description: "Skip paging; print directly to stdout." },
    ],
    related: &["help", "examples"],
    examples: &[
        ExampleHelp { description: "Browse the full flag list", command: "recon --flags" },
        ExampleHelp { description: "Search for a specific area", command: "recon --flags | grep -i cookie" },
        ExampleHelp { description: "Count flags", command: "recon --flags | wc -l" },
        ExampleHelp { description: "Pipe to a file", command: "recon --flags > flags.txt" },
    ],
};

static TOPIC_ARCHIVE: Topic = Topic {
    title: "Archive Tools (zip, tar, and friends)",
    description: "Create and extract file archives. Two unified flags:\n\
                  `--archive DEST FILE...` bundles the given files and directories\n\
                  (recursively) into DEST; `--extract SRC [-o DIR]` unpacks SRC\n\
                  into DIR (default: current directory).\n\
                  \n\
                  Format is inferred from the filename extension:\n\
                    .zip                 — ZIP\n\
                    .tar                 — uncompressed tar\n\
                    .tar.gz  / .tgz      — tar + gzip\n\
                    .tar.xz  / .txz      — tar + xz\n\
                    .tar.bz2 / .tbz2     — tar + bzip2\n\
                  \n\
                  `--extract` also tries magic-byte detection when the extension\n\
                  isn't recognised, so `.dat` that's actually a ZIP still works.",
    flags: &[
        FlagHelp { flags: "--archive <DEST> <FILE...>", description: "Create an archive. DEST's extension picks the format. Remaining\npositional arguments are the sources (files or directories).\nDirectories are archived recursively, keeping their name as the\narchive's top-level entry." },
        FlagHelp { flags: "--extract <SRC>", description: "Extract an archive. Format from extension or magic bytes.\nDestination defaults to CWD; pass -o DIR to redirect." },
        FlagHelp { flags: "-o / --output <DIR>", description: "Extraction destination directory for --extract. Created if absent." },
    ],
    related: &["--compress", "--decompress", "-o / --output"],
    examples: &[
        ExampleHelp { description: "Zip two files", command: "recon --archive report.zip notes.md summary.md" },
        ExampleHelp { description: "Tar a directory", command: "recon --archive src.tar src/" },
        ExampleHelp { description: "Gzipped tar", command: "recon --archive backup.tar.gz config/ logs/" },
        ExampleHelp { description: "Xz-compressed tar (strongest)", command: "recon --archive release.tar.xz dist/" },
        ExampleHelp { description: "Extract a zip to a specific directory", command: "recon --extract download.zip -o /tmp/unpack/" },
        ExampleHelp { description: "Extract a tar.gz (auto-detect by extension)", command: "recon --extract artifact.tar.gz -o /tmp/artifact/" },
        ExampleHelp { description: "Extract an ambiguously-named archive (magic-byte sniff)", command: "recon --extract blob.dat -o /tmp/out/" },
    ],
};

static TOPIC_PDF_EXPORT: Topic = Topic {
    title: "PDF page → image export",
    description: "`--export-pdf-page <PAGE> <PDF>` renders a single page of a PDF\n\
                  to a raster image (PNG / JPEG / WEBP). Rendering runs through\n\
                  `pdftoppm` from the poppler-utils suite (`brew install poppler`\n\
                  on macOS; `apt install poppler-utils` on Debian/Ubuntu).\n\
                  The viewport flag defines an upper-bound target box; pdftoppm\n\
                  rasterizes the page at the highest DPI that fits, preserving\n\
                  aspect. `--pdf-scale` multiplies for higher pixel density.\n\
                  \n\
                  Output path: -o PATH (extension picks the format), or\n\
                  default `page-<N>.png` in CWD. Use -o - to stream the image\n\
                  bytes to stdout. --pdf-format overrides extension inference.\n\
                  \n\
                  Script equivalent: `pdf_export_page(pdf, page, dest, opts)`\n\
                  or `pdf_export_page(pdf, page, opts)` returning a Blob.",
    flags: &[
        FlagHelp { flags: "--export-pdf-page <PAGE> <PDF>", description: "Export the 1-indexed PAGE of PDF as an image.\nPDF is a local path. Requires `pdftoppm`." },
        FlagHelp { flags: "-o / --output <PATH>", description: "Destination path. Extension picks format\n(.png / .jpg / .jpeg / .webp). `-` writes to stdout.\nDefault: page-<N>.png in CWD." },
        FlagHelp { flags: "--pdf-format <FMT>", description: "Override format inference: png / jpeg / webp.\nUseful with -o - (no extension to sniff)." },
        FlagHelp { flags: "--pdf-viewport <WxH>", description: "Target image box in pixels. Default 1024x1366.\nAspect is preserved; this is an upper-bound box." },
        FlagHelp { flags: "--pdf-scale <N>", description: "Density multiplier (>= 1). Default 2.\nFinal image fits within (W*N × H*N) px." },
        FlagHelp { flags: "--pdf-quality <0-100>", description: "JPEG/WEBP quality. Ignored for PNG. Default 90." },
    ],
    related: &["--md-to-pdf", "--html-to-pdf", "-o / --output"],
    examples: &[
        ExampleHelp { description: "Default: write page-1.png in CWD", command: "recon --export-pdf-page 1 docs/MANUAL.pdf" },
        ExampleHelp { description: "Choose page 3, save as JPEG", command: "recon --export-pdf-page 3 report.pdf -o cover.jpg" },
        ExampleHelp { description: "Large WEBP", command: "recon --export-pdf-page 1 report.pdf -o cover.webp --pdf-viewport 1920x2715 --pdf-scale 2" },
        ExampleHelp { description: "Stream to stdout", command: "recon --export-pdf-page 1 report.pdf --pdf-format png -o -" },
    ],
};

static TOPIC_BROWSER: Topic = Topic {
    title: "Browser Sessions (scripting)",
    description: "`browser()` returns a stateful HTTP session handle for Rhai\n\
                  scripts. Unlike one-shot `http(url, opts)`, a browser keeps\n\
                  a cookie jar, default headers, user-agent, TLS/redirect\n\
                  policy, and basic-auth credentials across multiple\n\
                  requests. Cookies set by one call are sent on the next.\n\
                  \n\
                  Default session is ephemeral — backed by a temp-file\n\
                  jar that's deleted when the script exits.\n\
                  `b.use_persistent_session(\"name\")` swaps in\n\
                  `~/.recon/jars/name.db` (fresh jar — any ephemeral\n\
                  cookies collected so far are discarded).\n\
                  \n\
                  Multiple browsers can be instantiated in the same\n\
                  script. Each has independent state:\n\
                    let b1 = browser();\n\
                    let b2 = browser();\n\
                  Scripts interleave calls as they please; Rhai is\n\
                  single-threaded, so \"parallel\" means independent\n\
                  state, not concurrent I/O.\n\
                  \n\
                  Script-only feature. Response shape matches `http()`.",
    flags: &[
        FlagHelp { flags: "browser()", description: "Build a browser with default config inherited from CLI flags\n(-H, -k, --connect-timeout, --user-agent, etc.)." },
        FlagHelp { flags: "browser(opts)", description: "Build a browser with an initial config map. Keys:\nuser_agent, headers (map), insecure, follow_redirects,\nmax_redirects, timeout_ms, connect_timeout, basic_auth." },

        FlagHelp { flags: "b.set_user_agent(ua)", description: "Set the User-Agent header for all subsequent requests." },
        FlagHelp { flags: "b.set_header(name, value)", description: "Add or replace one default header (case-insensitive)." },
        FlagHelp { flags: "b.set_headers(map)", description: "Merge a map of headers into the defaults. Keys replace\nexisting entries case-insensitively." },
        FlagHelp { flags: "b.remove_header(name) / b.clear_headers()", description: "Delete one or all default headers." },
        FlagHelp { flags: "b.set_timeout_ms(ms)", description: "Overall request deadline in milliseconds (maps to --max-time)." },
        FlagHelp { flags: "b.set_connect_timeout(secs)", description: "TCP/TLS connect deadline in seconds." },
        FlagHelp { flags: "b.set_insecure(bool)", description: "Skip TLS certificate verification." },
        FlagHelp { flags: "b.follow_redirects(bool) / b.set_max_redirects(n)", description: "Redirect policy + cap. Defaults inherited from CLI." },
        FlagHelp { flags: "b.set_basic_auth(user, pass)", description: "Attach an HTTP Basic Authorization header." },

        FlagHelp { flags: "b.use_persistent_session(name)", description: "Swap the cookie jar to ~/.recon/jars/name.db (fresh swap —\nephemeral cookies are discarded). Same file format as\n`--cookiejar` and `sqlite(\"cookiejar:NAME\")`." },
        FlagHelp { flags: "b.use_ephemeral_session()", description: "Swap back to a fresh temp-file jar. Previous session's\ncookies are discarded (temp file deleted)." },
        FlagHelp { flags: "b.session_name()", description: "Returns the current persistent-session name (String) or () for\nephemeral sessions." },
        FlagHelp { flags: "b.clear_cookies()", description: "Wipe every cookie from the current jar." },
        FlagHelp { flags: "b.cookies()", description: "List cookies as an Array of #{domain, path, name, value,\nexpires, secure, http_only}. For richer access, open the\nsame jar with `sqlite(\"cookiejar:NAME\")`." },

        FlagHelp { flags: "b.get(url [, opts]) / b.head / b.options / b.delete", description: "Method helpers without a body argument. `opts` is the same\nper-call override map as `http(url, opts)`." },
        FlagHelp { flags: "b.post(url, body [, opts]) / b.put / b.patch", description: "Method helpers with a body argument. Body can be String,\nBlob, or Map/Array (maps and arrays auto-serialise to JSON\nand set Content-Type: application/json unless the script\nprovides its own)." },
        FlagHelp { flags: "b.request(#{url, method, body, headers, …})", description: "Freeform request with an opts-map. `url` is required;\n`method` defaults to \"GET\". Accepts the same keys as\n`http(url, opts)`." },
    ],
    related: &["--script", "--cookiejar", "sqlite"],
    examples: &[
        ExampleHelp { description: "Ephemeral session: login + follow-up request", command: r#"recon --script - <<< 'let b = browser(); b.get("https://httpbin.org/cookies/set/session/abc"); print(b.get("https://httpbin.org/cookies").body);'"# },
        ExampleHelp { description: "Persistent session survives across script runs", command: r#"recon --script - <<< 'let b = browser(); b.use_persistent_session("myjar"); b.get("https://example.com/login");'"# },
        ExampleHelp { description: "Two browsers in one script, independent cookies", command: r#"recon --script - <<< 'let b1 = browser(); let b2 = browser(); b1.get("https://a.example"); b2.get("https://b.example");'"# },
        ExampleHelp { description: "Full example shipped in the repo", command: "recon --script script/browser.rhai" },
        ExampleHelp { description: "Inspect a persistent jar directly", command: r#"sqlite3 ~/.recon/jars/myjar.db "SELECT domain, name, expires FROM cookies;""# },
    ],
};

static TOPIC_AGENT_BROWSER: Topic = Topic {
    title: "Browser Automation (agent-browser)",
    description: "recon wraps the external `agent-browser` CLI so scripts can drive a\n\
                  real browser (click, fill, screenshot, accessibility snapshot, JS\n\
                  eval, etc.). The wrapper is exposed as the Rhai static module\n\
                  `agentBrowser`; it's always present in scripts even when the\n\
                  `agent-browser` binary isn't installed.\n\
                  \n\
                  Availability is detected once at engine build time:\n\
                    agentBrowser::available : bool    (true when binary is on PATH)\n\
                    agentBrowser::version   : string  (e.g. \"0.26.0\", empty when\n\
                                                       unavailable)\n\
                  \n\
                  When unavailable, every function call raises a Rhai error:\n\
                  'agent-browser: binary not found on PATH'. Guard with\n\
                  `if !agentBrowser::available { ... }`.\n\
                  \n\
                  Install: `brew install agent-browser` (macOS) or\n\
                  `npm install -g agent-browser`.\n\
                  \n\
                  Global options (0.75.0): Set agent-browser launch / security /\n\
                  session options once via agentBrowser::set_default_options(opts)\n\
                  at script start (ignore_https_errors, user_agent, proxy, headers,\n\
                  profile, extension, browser_args, etc.). Per-call overrides are\n\
                  accepted on launch verbs (open, screenshot, snapshot, pdf, eval).\n\
                  See `recon --examples` for patterns.",
    flags: &[
        FlagHelp { flags: "--browser-screenshot <URL>", description: "One-shot: open URL, save a screenshot, close. Honours -o PATH.\nRequires agent-browser installed." },

        FlagHelp { flags: "agentBrowser::available / agentBrowser::version", description: "Boolean + version string. Read-only module constants always\npresent, regardless of whether agent-browser is installed." },
        FlagHelp { flags: "agentBrowser::open(url)", description: "Navigate to URL. Returns stdout as a String." },
        FlagHelp { flags: "agentBrowser::close() / close_all()", description: "Close the current browser / every session." },
        FlagHelp { flags: "agentBrowser::click(sel) / dblclick(sel)", description: "Click or double-click an element. Selector may be CSS, XPath,\nor an agent-browser ref like @e3 from a prior snapshot." },
        FlagHelp { flags: "agentBrowser::type_text(sel, text) / fill(sel, text)", description: "Type into an element. `type_text` keeps existing content; `fill`\nclears first. (Rhai reserves `type`, hence the `type_text` rename.)" },
        FlagHelp { flags: "agentBrowser::press(key)", description: "Press a key on the active element (Enter, Tab, Control+a)." },
        FlagHelp { flags: "agentBrowser::hover / focus / check / uncheck(sel)", description: "Standard element interactions; each takes one selector arg." },
        FlagHelp { flags: "agentBrowser::scroll(dir [, px])", description: "Scroll `up`/`down`/`left`/`right`. Optional pixel count." },
        FlagHelp { flags: "agentBrowser::scrollintoview(sel)", description: "Scroll the matched element into view." },
        FlagHelp { flags: "agentBrowser::wait(arg)", description: "Wait for a selector to appear OR a number of milliseconds\n(string form: `wait(\"2000\")`)." },
        FlagHelp { flags: "agentBrowser::screenshot([path])", description: "Take a screenshot. Without an arg, agent-browser picks a path.\nReturns the path in stdout." },
        FlagHelp { flags: "agentBrowser::pdf(path)", description: "Save the current page as a PDF." },
        FlagHelp { flags: "agentBrowser::snapshot() / snapshot(true)", description: "Accessibility snapshot (optionally interactive-only). Returns\na Rhai Map parsed from JSON." },
        FlagHelp { flags: "agentBrowser::eval(js)", description: "Run JavaScript in the active page. Returns the evaluated\nresult parsed from JSON." },
        FlagHelp { flags: "agentBrowser::get(what [, sel])", description: "Read page info. `what` is text/html/value/attr/title/url/count/\nbox/styles/cdp-url. Returns a Rhai Map with the field named after\n`what` (e.g. `result.title`)." },
        FlagHelp { flags: "agentBrowser::is_visible(sel) / is_enabled(sel) / is_checked(sel)", description: "Element-state predicates. Each returns a bool." },
        FlagHelp { flags: "agentBrowser::find(locator, value, action [, text])", description: "Locate by role/text/label/placeholder/alt/title/testid/first/\nlast/nth and then click/fill/etc. Returns parsed JSON." },
        FlagHelp { flags: "agentBrowser::keyboard_type(text) / keyboard_insert(text)", description: "Type at the focused element without a selector; insert version\nskips key events." },
        FlagHelp { flags: "agentBrowser::back() / forward() / reload()", description: "Navigation." },
        FlagHelp { flags: "agentBrowser::cmd([\"raw\", \"args\", \"here\"])", description: "Escape hatch: run arbitrary agent-browser CLI args. Returns the\nraw stdout as a String." },
        FlagHelp { flags: "agentBrowser::set_default_options(opts)", description: "Set module-level default options applied to every verb. opts is a\nRhai map with snake_case keys (ignore_https_errors, user_agent,\nproxy, headers, profile, session, extension, browser_args, etc.)." },
        FlagHelp { flags: "agentBrowser::clear_default_options()", description: "Reset module-level defaults to empty." },
        FlagHelp { flags: "agentBrowser::default_options() -> Map", description: "Read the current module-level defaults as a Rhai map." },
        FlagHelp { flags: "agentBrowser::open(url, opts) / screenshot(path, opts) / ...", description: "Per-call opts overload on launch verbs. Per-call opts concatenate\nafter defaults so per-call values override defaults (last-wins)." },
    ],
    related: &["--browser-screenshot", "--script"],
    examples: &[
        ExampleHelp { description: "One-shot screenshot via the CLI flag", command: "recon --browser-screenshot https://example.com -o /tmp/shot.png" },
        ExampleHelp { description: "Guard pattern in a script", command: r#"recon --script - <<< 'if !agentBrowser::available { return 2; } agentBrowser::open("https://example.com"); print(agentBrowser::get("title").title); agentBrowser::close();'"# },
        ExampleHelp { description: "Shipped example scripts (in project script/ folder)", command: "recon --script script/agent-browser-title.rhai https://example.com" },
        ExampleHelp { description: "Full reference for agent-browser itself", command: "agent-browser --help" },
    ],
};

static TOPIC_WGET: Topic = Topic {
    title: "wget-compat batch fetching",
    description: "Flags ported from wget for batch URL handling. Originally shipped\n\
                  in 0.64.0 (--input-file, --continue, --spider, --timestamping)\n\
                  with --wait, --tries, --accept, --reject added in 0.67.0. recon\n\
                  uses curl-style short flags throughout, so wget options are\n\
                  long-form only — no -A/-R/-t/-w. The recursive/mirror cluster\n\
                  (-r/-l/-m/-p/-k) is deferred and tracked in OUT-OF-SCOPE.md.",
    flags: &[
        FlagHelp { flags: "--input-file <FILE>", description: "Batch-fetch URLs listed in FILE (one per line, # comments,\nblank lines ignored, `-` reads from stdin). Each URL is\nprocessed independently; per-URL errors don't abort the batch." },
        FlagHelp { flags: "--wait <SECS>", description: "Fixed delay (seconds) between URLs in batch mode. Skipped\nbefore the first URL. Overrides --rate when both are set.\nUse --rate for `N/s` request-rate caps." },
        FlagHelp { flags: "--tries <N>", description: "Total attempts per URL (wget semantics: tries = retries + 1).\nOverrides --retry. `--tries 1` disables retries; `--tries 5`\nallows 4 retries. 0 is rejected at parse time." },
        FlagHelp { flags: "--accept <LIST>", description: "Comma-separated filename-suffix accept list. e.g.\n`--accept jpg,png` keeps only URLs whose final path segment\nends in `.jpg` or `.png`. Case-insensitive. Suffixes match\nwith or without a leading dot (`jpg`, `.jpg`, `JPG` all\ncollapse to `.jpg`). URLs with empty final segment fail." },
        FlagHelp { flags: "--reject <LIST>", description: "Comma-separated filename-suffix reject list. e.g.\n`--reject thumb,bak` drops URLs ending in those suffixes.\nCombines with --accept (URL must pass both). URLs with empty\nfinal segment pass. Case-insensitive." },
        FlagHelp { flags: "--continue", description: "Resume an interrupted download (wget alias).\nReads the current size of the -o target (or basename derived\nfrom the URL) and sets `Range: bytes=<size>-`." },
        FlagHelp { flags: "--continue-at <OFFSET>", description: "Resume from BYTE offset (curl-compatible). Pass `-` to\nauto-detect from the local file size (same as --continue)." },
        FlagHelp { flags: "--spider", description: "HEAD-only check; print `<status> <url>` per URL and exit\nnon-zero on any 4xx/5xx. Pairs with --input-file." },
        FlagHelp { flags: "--timestamping", description: "Skip download when the local file's mtime is ≥ the server's\nLast-Modified. Sets If-Modified-Since on the request." },
        FlagHelp { flags: "--retry <N>", description: "Retry N times on transient failures (5xx, DNS, connect reset,\ntimeouts). Default 0. Overridden by --tries when both are set." },
        FlagHelp { flags: "--rate <N/s|N/m|N/h>", description: "Request rate cap. Format: `N/s` / `N/m` / `N/h`. Engages with\n--input-file: at most N requests per second/minute/hour.\nOverridden by --wait when both are set." },
    ],
    related: &["--retry-all-errors", "--retry-connrefused", "--retry-delay", "--retry-max-time"],
    examples: &[
        ExampleHelp { description: "Batch-fetch with a polite 2s gap", command: "recon --input-file urls.txt --wait 2" },
        ExampleHelp { description: "Filter to image URLs, drop thumbnails", command: "recon --input-file urls.txt --accept jpg,png --reject thumb" },
        ExampleHelp { description: "Wget-style retries (5 total attempts)", command: "recon https://example.com/api --tries 5" },
        ExampleHelp { description: "Spider check filtered by extension", command: "recon --input-file urls.txt --spider --accept html,htm" },
        ExampleHelp { description: "Resume a download (wget habit)", command: "recon https://example.com/big.iso -o big.iso --continue" },
    ],
};

static TOPIC_PROTOCOLS: Topic = Topic {
    title: "Protocol URL Schemes (probes and aliases)",
    description: "In addition to http(s):// and mqtt(s)://, recon dispatches a family\n\
                  of URL schemes for point-probe diagnostics. Some are thin aliases\n\
                  for existing flags (tls://, ping://, traceroute://, whois://,\n\
                  dns:// / dig:// / drill://); the rest are standalone probes\n\
                  (tcp, udp, ntp, dict, redis, memcached, ws, wss, ldap, ldaps,\n\
                  rtsp, rtsps). file:// reads local files. See `recon --version`\n\
                  for the full list compiled in.\n\
                  \n\
                  See also: `recon --help impersonate` -- TLS+H2 browser fingerprint\n\
                  impersonation (opt-in --features impersonate build).",
    flags: &[
        FlagHelp { flags: "file:///path", description: "Read a local file and write its bytes to stdout (or -o <path>).\nAccepts file://localhost/path too. Curl-compatible." },

        FlagHelp { flags: "whois://HOST", description: "Whois lookup. Equivalent to `recon --whois HOST`." },
        FlagHelp { flags: "dns://HOST[/TYPE[,TYPE…]]", description: "DNS lookup. Path is a comma-separated record-type shorthand\n(e.g. dns://example.com/MX,AAAA). `--dns-type` overrides the path\nwhen both are given. Defaults to the standard record-type bundle." },
        FlagHelp { flags: "dig://HOST[/TYPE…]", description: "Alias for dns://. Same semantics." },
        FlagHelp { flags: "drill://HOST[/TYPE…]", description: "Alias for dns://. Same semantics." },

        FlagHelp { flags: "tls://HOST[:PORT]/", description: "TLS handshake + certificate inspection.\nEquivalent to `recon --cert https://HOST[:PORT]/`. Default port 443." },
        FlagHelp { flags: "ping://HOST", description: "ICMP ping. Equivalent to `recon --ping <host>`.\nPort in URL is ignored." },
        FlagHelp { flags: "traceroute://HOST", description: "Traceroute. Equivalent to `recon --traceroute <host>`." },

        FlagHelp { flags: "tcp://HOST:PORT/", description: "TCP connect probe. Reports connect latency and resolved/local\naddress. Exit 0 on connect, 7 refused, 28 timed out.\nPort is required." },
        FlagHelp { flags: "udp://HOST:PORT[/path]", description: "UDP send-and-wait probe. Sends payload from -d (or empty),\nwaits --wait-time seconds for any response. Exit 0 regardless\nof response (UDP silence is ambiguous). Port is required." },
        FlagHelp { flags: "ntp://HOST[:PORT]/", description: "SNTPv4 probe. Reports stratum, reference identifier, offset from\nlocal clock, round-trip delay, precision, poll interval, and the\nserver's reference time. Default port 123." },

        FlagHelp { flags: "dict://HOST[:PORT]/CMD", description: "RFC 2229 DICT client (curl URL grammar). Commands:\n  /d:WORD[:DB[:STRAT]] — DEFINE\n  /m:WORD[:DB[:STRAT]] — MATCH\n  /show:server|databases|strategies|info:DB\nBare dict://HOST/ runs SHOW SERVER + SHOW DATABASES +\nSHOW STRATEGIES as an overview. Default port 2628." },
        FlagHelp { flags: "redis://[:PASS@]HOST[:PORT]", description: "Redis probe (RESP2). No -d → connect + PING. With\n-d 'SET key value' → sends that RESP command (shell-split,\nhonours \"quoted\" tokens). Optional password from URL userinfo\nsends AUTH first. Default port 6379. Exit 7/28/67." },
        FlagHelp { flags: "memcached://HOST[:PORT][/stats]", description: "Memcached text-protocol probe: sends `version`, reports server\nversion + roundtrip. Append /stats to also dump `stats` output.\nDefault port 11211." },

        FlagHelp { flags: "ws://HOST[:PORT][/path]", description: "WebSocket probe: TCP connect → HTTP Upgrade handshake → send\nPing frame with nonce → wait for matching Pong → close. Reports\nlatencies + selected Sec-WebSocket-* headers. Default port 80." },
        FlagHelp { flags: "wss://HOST[:PORT][/path]", description: "Same as ws:// but over TLS. Default port 443. Honours -k." },

        FlagHelp { flags: "ldap://HOST[:PORT]/", description: "Anonymous simple bind → RootDSE query (objectClass=* at scope=\nbase). Reports namingContexts, supportedLDAPVersion,\nvendorName/Version, supportedSASLMechanisms. Default port 389." },
        FlagHelp { flags: "ldaps://HOST[:PORT]/", description: "Same as ldap:// but over TLS. Default port 636." },

        FlagHelp { flags: "rtsp://HOST[:PORT][/path]", description: "RTSP OPTIONS probe (RFC 2326). Prints status line + response\nheaders (Public: supported methods, Server:). Default port 554." },
        FlagHelp { flags: "rtsps://HOST[:PORT][/path]", description: "Same as rtsp:// but over TLS. Default port 322. Honours -k\n(skips certificate verification)." },

        FlagHelp { flags: "smtp://HOST[:PORT]/", description: "SMTP probe and optional mail delivery. Reports EHLO\ncapabilities, AUTH methods, STARTTLS availability. With\n--mail-from + --mail-to sends a test message (optional DKIM\nsigning). Default port 25. See `recon --help smtp`." },
        FlagHelp { flags: "smtps://HOST[:PORT]/", description: "Same as smtp:// but implicit TLS. Default port 465." },

        FlagHelp { flags: "ftp://[user[:pass]@]HOST[:PORT]/path", description: "FTP probe + retrieve. Trailing slash -> list; no trailing\nslash -> retrieve. Default port 21. See `recon --help ftp`." },
        FlagHelp { flags: "ftps://...", description: "Same as ftp:// but with AUTH TLS (explicit FTPS)." },
        FlagHelp { flags: "sftp://[user[:pass]@]HOST[:PORT]/path", description: "SSH-backed file transfer. Default port 22. Shares auth with\nscp:// / ssh://. See `recon --help sftp`." },
        FlagHelp { flags: "tftp://HOST[:PORT]/filename", description: "RFC 1350 UDP read. Default port 69. See `recon --help tftp`." },
        FlagHelp { flags: "gopher://HOST[:PORT]/[TYPE]/selector", description: "Gopher selector fetch (RFC 1436). Default port 70." },
        FlagHelp { flags: "gophers://...", description: "Gopher over TLS." },

        FlagHelp { flags: "pop3://[user[:pass]@]HOST[:PORT]/[N]", description: "POP3 probe / retrieve. Empty path -> CAPA + STAT;\nnumeric path -> RETR. See `recon --help pop3`." },
        FlagHelp { flags: "pop3s://...", description: "Implicit-TLS POP3. Default port 995." },
        FlagHelp { flags: "imap://[user[:pass]@]HOST[:PORT]/[MAILBOX[;UID=N]]", description: "IMAP probe / examine / fetch. See `recon --help imap`." },
        FlagHelp { flags: "imaps://...", description: "Implicit-TLS IMAP. Default port 993." },

        FlagHelp { flags: "ipfs://CID[/path] / ipns://NAME[/path]", description: "Rewritten to <gateway>/ipfs/CID[/path] (default gateway:\nhttps://ipfs.io). Dispatches through the existing HTTP path,\nso every HTTP flag applies. Override gateway with\n--ipfs-gateway or $RECON_IPFS_GATEWAY." },

        FlagHelp { flags: "--wait-time <SECS>", description: "(udp:// only) Seconds to wait for a response datagram after\nsending. Accepts fractional values. Default: 1.0." },
        FlagHelp { flags: "--connect-timeout <SECS>", description: "Socket connect / response deadline for tcp, udp, ntp, tls,\ndict, redis, memcached, ws, wss, rtsp, rtsps probes." },
        FlagHelp { flags: "-k, --insecure", description: "Skip TLS certificate verification for wss://, ldaps://, rtsps://,\nmqtts://, and https:// connections." },
    ],
    related: &["--cert", "--ping", "--traceroute", "--whois", "--dns"],
    examples: &[
        ExampleHelp { description: "Read a local file like curl's file:// scheme", command: "recon file:///etc/hosts" },
        ExampleHelp { description: "Check whether a TCP port accepts connections", command: "recon tcp://github.com:443/" },
        ExampleHelp { description: "Query an NTP server and report clock offset", command: "recon ntp://pool.ntp.org/" },
        ExampleHelp { description: "DICT define (curl URL grammar)", command: "recon dict://dict.dict.org/d:recon" },
        ExampleHelp { description: "DICT server overview (bare URL)", command: "recon dict://dict.dict.org/" },
        ExampleHelp { description: "Redis PING", command: "recon redis://localhost/" },
        ExampleHelp { description: "Redis arbitrary RESP command via -d", command: r#"recon redis://localhost/ -d "SET key \"hello world\"""# },
        ExampleHelp { description: "Memcached version + stats", command: "recon memcached://localhost/stats" },
        ExampleHelp { description: "WebSocket ping/pong round-trip", command: "recon wss://ws.postman-echo.com/raw" },
        ExampleHelp { description: "LDAP RootDSE (anonymous)", command: "recon ldap://ldap.forumsys.com:389/" },
        ExampleHelp { description: "RTSP OPTIONS (supported methods)", command: "recon rtsp://example.com:554/stream" },
        ExampleHelp { description: "DNS with path shorthand for record type", command: "recon dns://example.com/MX,AAAA" },
        ExampleHelp { description: "Ping and traceroute as URL schemes", command: "recon ping://8.8.8.8 && recon traceroute://8.8.8.8" },
    ],
};

static TOPIC_IMPERSONATE: Topic = Topic {
    title: "TLS/H2 Browser Fingerprint Impersonation",
    description: "Make outbound HTTPS requests with a real browser's TLS and HTTP/2\n\
                  fingerprint instead of the default reqwest/rustls signature. When\n\
                  a server uses JA3/JA4 fingerprinting or H2-frame analysis to detect\n\
                  bots, this makes recon indistinguishable from the named browser.\n\
                  \n\
                  The impersonation stack is an opt-in Cargo feature (`--features\n\
                  impersonate`). The default recon binary is built without it to keep\n\
                  the binary small and avoid the BoringSSL build dependency. A\n\
                  prebuilt `recon-impersonate` binary is available for supported\n\
                  platforms from the project releases page.\n\
                  \n\
                  When the feature is absent, any of the four flags below causes an\n\
                  immediate error with a hint to rebuild with `--features impersonate`\n\
                  or to download the `recon-impersonate` release artifact.\n\
                  \n\
                  V1 SCOPE: only --impersonate is implemented. The --ja3, --ja4,\n\
                  and --http2-fingerprint flags are reserved in the CLI for\n\
                  forward-compatibility but error at runtime as not-yet-implemented.\n\
                  Use --impersonate for now. See OUT-OF-SCOPE.md for the rationale.\n\
                  \n\
                  PROFILES\n\
                  Named profiles are forwarded to wreq_util::Emulation. The format\n\
                  uses underscores and dots; hyphens are accepted as a convenience and\n\
                  are normalised to underscores before dispatch.\n\
                  \n\
                  Common families and examples:\n\
                    chrome_131         chrome_127        chrome_124\n\
                    firefox_133        firefox_128\n\
                    safari_18.2        safari_17.5\n\
                    edge_131           edge_127\n\
                    chrome_android_131 chrome_android_127\n\
                    safari_ios_18.1.1  safari_ios_17.4.1\n\
                    okhttp_5           okhttp_4.12\n\
                  \n\
                  Check the wreq_util crate docs for the full list of supported\n\
                  profile identifiers. Profiles are case-sensitive.\n\
                  \n\
                  COMBINATION RULES\n\
                  The impersonation profile owns the entire TLS configuration. The\n\
                  following flags are incompatible with --impersonate in v1 and will\n\
                  be rejected at runtime:\n\
                    --ciphers, --tls13-ciphers   (cipher suite overrides)\n\
                    --tlsv1.2, --tlsv1.3         (TLS version pins)\n\
                    --client-cert, --client-key  (mTLS client certificate)\n\
                    --cacert                     (custom trust root)\n\
                  Combining any of these with --impersonate defeats the fingerprint\n\
                  and is therefore blocked explicitly.",
    flags: &[
        FlagHelp {
            flags: "--impersonate <PROFILE>",
            description: "Impersonate a named browser TLS+H2 fingerprint.\n\
                          Requires the binary to be built with `--features impersonate`.\n\
                          Example profiles: chrome_131, firefox_128, safari_17.5,\n\
                          edge_131, okhttp_5, chrome_android_131, safari_ios_17.4.1.\n\
                          Hyphens are accepted (chrome-131 == chrome_131).\n\
                          Incompatible with --ciphers, --tls13-ciphers, --tlsv1.2,\n\
                          --tlsv1.3, --client-cert, --client-key, --cacert.",
        },
        FlagHelp {
            flags: "--ja3 <STRING>",
            description: "Provide a raw JA3 fingerprint string for TLS impersonation.\n\
                          DEFERRED -- currently errors at runtime with a\n\
                          'not yet implemented' message. Parsed and reserved in the\n\
                          CLI for forward-compatibility only. Use --impersonate instead.",
        },
        FlagHelp {
            flags: "--ja4 <STRING>",
            description: "Provide a raw JA4 fingerprint string for TLS impersonation.\n\
                          DEFERRED -- currently errors at runtime with a\n\
                          'not yet implemented' message. Parsed and reserved in the\n\
                          CLI for forward-compatibility only. Use --impersonate instead.",
        },
        FlagHelp {
            flags: "--http2-fingerprint <STRING>",
            description: "Provide a raw HTTP/2 SETTINGS fingerprint string.\n\
                          DEFERRED -- currently errors at runtime with a\n\
                          'not yet implemented' message. Parsed and reserved in the\n\
                          CLI for forward-compatibility only. Use --impersonate instead.",
        },
    ],
    related: &["--user-agent", "--ciphers", "--tls13-ciphers", "--tlsv1.2", "--tlsv1.3"],
    examples: &[
        ExampleHelp {
            description: "Impersonate Chrome 131 TLS+H2 fingerprint",
            command: "recon https://tls.browserleaks.com/json --impersonate chrome_131",
        },
        ExampleHelp {
            description: "Impersonate Firefox 128",
            command: "recon https://tls.browserleaks.com/json --impersonate firefox_128",
        },
        ExampleHelp {
            description: "Impersonate Safari 17.5",
            command: "recon https://tls.browserleaks.com/json --impersonate safari_17.5",
        },
        ExampleHelp {
            description: "Impersonate an Android Chrome build (accepts hyphens)",
            command: "recon https://api.example.com/data --impersonate chrome-android-131",
        },
        ExampleHelp {
            description: "Impersonate OkHttp 5 (common mobile SDK fingerprint)",
            command: "recon https://api.example.com/data --impersonate okhttp_5",
        },
        ExampleHelp {
            description: "Use in a script via the impersonate opts key",
            command: "recon --script - <<< 'let r = http(\"https://tls.browserleaks.com/json\", #{impersonate: \"chrome_131\"}); print(r.text());'",
        },
    ],
};

static TOPIC_AI: Topic = Topic {
    title: "AI — script-engine bindings to agent CLIs",
    description: "The ai::* namespace lets a Rhai script ask an LLM a question via a\n\
                  subprocess-driven backend (claude, codex, copilot, gemini, or a\n\
                  user-defined command). Build a request with .system / .context /\n\
                  .prompt / optionally .assistant for multi-turn replay, then call\n\
                  .send().",
    flags: &[
        FlagHelp {
            flags: "ai::ask(prompt)",
            description: "One-liner. Equivalent to request().prompt(p).send().",
        },
        FlagHelp {
            flags: "ai::request()",
            description: "Returns a builder. Mutate-in-place or chain — methods return\n\
                          the cloned builder so both styles work.",
        },
        FlagHelp {
            flags: ".backend(name) / .model(name)",
            description: "Backend selection (claude / codex / copilot / gemini / a\n\
                          config-defined cmd entry). Model name is pass-through to\n\
                          the backend's CLI.",
        },
        FlagHelp {
            flags: ".system(s) / .context(s) / .prompt(s) / .user(s)",
            description: "System prompt (singleton), accumulating context blocks,\n\
                          current user turn (singleton). .user is an alias for .prompt.",
        },
        FlagHelp {
            flags: ".assistant(s)",
            description: "Append a prior assistant turn for manual multi-turn replay.\n\
                          Errors if the last turn is already an assistant.",
        },
        FlagHelp {
            flags: ".max_tokens / .temperature / .timeout",
            description: "Hint knobs. timeout is seconds; default 60. max_tokens and\n\
                          temperature are honoured only by backends that expose them.",
        },
        FlagHelp {
            flags: ".send() / .send_full()",
            description: ".send() returns the model's reply as a string. .send_full()\n\
                          returns a map with .text, .backend, .model, .duration_ms,\n\
                          .exit_code.",
        },
    ],
    related: &["script"],
    examples: &[
        ExampleHelp {
            description: "One-shot ask",
            command: "let a = ai::ask(\"Summarize this cert chain\");",
        },
        ExampleHelp {
            description: "Builder with context",
            command: "request().system(\"...\").context(c).prompt(q).send()",
        },
        ExampleHelp {
            description: "Multi-turn replay",
            command: "req.assistant(a1); req.user(\"follow-up\"); req.send();",
        },
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
        "mqtt" => Some(&TOPIC_MQTT),
        "protocols" | "protocol" => Some(&TOPIC_PROTOCOLS),
        "bimi" => Some(&TOPIC_BIMI),
        "tls-rpt" | "tlsrpt" => Some(&TOPIC_TLS_RPT),
        "email" | "email-protection" => Some(&TOPIC_EMAIL),
        "cookies" | "cookiejar" | "cookie" => Some(&TOPIC_COOKIES),
        "encode" | "encoding" | "qr" | "barcode" => Some(&TOPIC_ENCODE),
        "encrypt" | "encryption" | "decrypt" | "age" => Some(&TOPIC_ENCRYPT),
        "scp" => Some(&TOPIC_SCP),
        "ssh" | "ssh-shell" => Some(&TOPIC_SSH),
        "telnet" => Some(&TOPIC_TELNET),
        "jwt" | "jwt-token" | "token" => Some(&TOPIC_JWT),
        "netstatus" => Some(&TOPIC_NETSTATUS),
        "hash" | "hashing" => Some(&TOPIC_HASH),
        "compress" | "compression" | "decompress" => Some(&TOPIC_COMPRESSION),
        "sample" | "sampledata" | "sample-data" => Some(&TOPIC_SAMPLE),
        "editor" | "editor-output" => Some(&TOPIC_EDITOR),
        "serve" | "server" => Some(&TOPIC_SERVE),
        "serve-tls" | "serve-https" | "https-server" => Some(&TOPIC_SERVE_TLS),
        "checkdigit" | "check-digit" | "checksum" => Some(&TOPIC_CHECKDIGIT),
        "write-out" | "writeout" | "write_out" => Some(&TOPIC_WRITE_OUT),
        "script" | "scripting" | "rhai" | "shebang" => Some(&TOPIC_SCRIPT),
        "repl" | "interactive" | "prompt" => Some(&TOPIC_REPL),
        "agent-browser" | "agentbrowser" => Some(&TOPIC_AGENT_BROWSER),
        "browser" | "session" | "browser-session" => Some(&TOPIC_BROWSER),
        "charset" | "encoding-text" | "text-encoding" | "iconv" | "text" => Some(&TOPIC_TEXT_ENCODING),
        "strutil" | "string-helpers" | "trim" | "sprintf" | "printf" | "preg" | "strip-html" | "nl2br" | "strrev" => Some(&TOPIC_STRUTIL),
        "jq" | "filter" | "jaq" => Some(&TOPIC_JQ),
        "git" | "git-wrapper" => Some(&TOPIC_GIT),
        "gh" | "github" | "github-cli" => Some(&TOPIC_GH),
        "smtp" | "smtps" | "mail" | "email-send" => Some(&TOPIC_SMTP),
        "ftp" | "ftps" => Some(&TOPIC_FTP),
        "sftp" => Some(&TOPIC_SFTP),
        "tftp" => Some(&TOPIC_TFTP),
        "gopher" | "gophers" => Some(&TOPIC_GOPHER),
        "pop3" | "pop3s" => Some(&TOPIC_POP3),
        "imap" | "imaps" => Some(&TOPIC_IMAP),
        "impersonate" | "ja3" | "ja4" | "fingerprint" | "tls-fingerprint" | "browser-fingerprint" | "http2-fingerprint" => Some(&TOPIC_IMPERSONATE),
        "ipfs" | "ipns" => Some(&TOPIC_IPFS),
        "proxy" | "proxies" => Some(&TOPIC_PROXY),
        "unix-socket" | "unixsocket" | "uds" => Some(&TOPIC_UNIX_SOCKET),
        "hsts" | "strict-transport-security" => Some(&TOPIC_HSTS),
        "compare" | "diff" => Some(&TOPIC_COMPARE),
        "decode" | "scan" | "barcode-scan" | "decode-image" => Some(&TOPIC_DECODE),
        "threads" | "spawn" | "concurrency" | "thread" => Some(&TOPIC_SCRIPT_THREADS),
        "shell" | "subprocess" | "shell-stream" | "shell_stream" | "exec" => Some(&TOPIC_SHELL),
        "tui" | "dashboard" | "pane" | "panes" | "split" => Some(&TOPIC_TUI),
        "script-server" | "tcp-server" | "udp-server" | "listen" => Some(&TOPIC_SCRIPT_SERVER),
        "docs" | "markdown" | "md-to-html" | "md-to-pdf" | "html-to-pdf" => Some(&TOPIC_DOCS),
        "pdf" | "pdf-export" | "pdf-page" | "pdf-image" | "export-pdf-page" => Some(&TOPIC_PDF_EXPORT),
        "client-cert" | "mtls" | "client-certificate" => Some(&TOPIC_CLIENT_CERT),
        "archive" | "zip" | "tar" | "extract" => Some(&TOPIC_ARCHIVE),
        "flags" | "flag-list" | "list-flags" => Some(&TOPIC_FLAGS),
        "wget" | "wait" | "tries" | "accept" | "reject" | "input-file" | "spider" | "timestamping" | "batch" => Some(&TOPIC_WGET),
        "ai" | "llm" | "chat" => Some(&TOPIC_AI),
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
        "netstatus",
        "traceroute",
        "spf",
        "dmarc",
        "dkim",
        "mta-sts",
        "mqtt",
        "protocols",
        "bimi",
        "tls-rpt",
        "email",
        "cookies",
        "scp",
        "ssh",
        "telnet",
        "jwt",
        "hash",
        "compression",
        "encoding",
        "encryption",
        "checkdigit",
        "sample",
        "editor",
        "serve",
        "serve-tls",
        "write-out",
        "script",
        "repl",
        "browser",
        "agent-browser",
        "archive",
        "charset",
        "strutil",
        "jq",
        "git",
        "gh",
        "smtp",
        "ftp",
        "sftp",
        "tftp",
        "gopher",
        "pop3",
        "imap",
        "ipfs",
        "proxy",
        "unix-socket",
        "hsts",
        "compare",
        "client-cert",
        "decode",
        "threads",
        "shell",
        "tui",
        "script-server",
        "docs",
        "pdf-export",
        "flags",
        "wget",
        "impersonate",
        "ai",
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
