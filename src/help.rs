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
        FlagHelp { flags: "-v, --verbose", description: "Show connection info and request/response headers on stderr.\nUse -vv for TLS certificate summary, auth detail, and elapsed time." },
        FlagHelp { flags: "-s, --silent", description: "Suppress informational and progress output.\nThe response body is still printed unless -o is used." },
        FlagHelp { flags: "-o, --output <FILE>", description: "Write the response body to a file instead of stdout." },
        FlagHelp { flags: "--progress", description: "Show a progress meter when saving to a file with -o.\nOpt-in only; never shown by default." },
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
                          sha3-256, sha3-512, blake3.",
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
        ExampleHelp { description: "List supported algorithms", command: "recon --hash-list" },
    ],
};

static TOPIC_COMPRESSION: Topic = Topic {
    title: "Compression",
    description: "Compress or decompress any source — a local file, file:// URL, HTTP(S) URL,\n\
                  or stdin. Output goes to stdout or -o <FILE>. Auto-detects gzip, zstd, and\n\
                  bzip2 inputs by magic bytes; deflate and brotli lack a signature so their\n\
                  algorithm must be named explicitly when decompressing.",
    flags: &[
        FlagHelp {
            flags: "--compress <ALGO>",
            description: "Compress with the named algorithm (case-insensitive; alias accepted).\n\
                          Supported: gzip/gz, deflate, zstd/zst, brotli/br, bzip2/bz2.",
        },
        FlagHelp {
            flags: "--decompress [ALGO]",
            description: "Decompress. Omit ALGO to auto-detect (gzip, zstd, bzip2 by magic\n\
                          bytes). Pass the algorithm for deflate or brotli.",
        },
        FlagHelp {
            flags: "--compression-level <LEVEL>",
            description: "Quality for --compress. Number in the algorithm's native range\n\
                          (gzip 0-9, zstd 1-22, brotli 0-11, bzip2 1-9), or a word:\n\
                          fastest, fast, default, good, best. Invalid with --decompress.",
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
    ],
    related: &["-o / --output", "--from-file"],
    examples: &[
        ExampleHelp { description: "QR code to terminal (ASCII)", command: "recon --encode qr \"https://example.com\"" },
        ExampleHelp { description: "QR code to SVG (inferred from extension)", command: "recon --encode qr \"https://example.com\" -o qr.svg" },
        ExampleHelp { description: "QR code to PNG", command: "recon --encode qr \"Contact: +46-70-123\" -o contact.png" },
        ExampleHelp { description: "DataMatrix (Swedish personal number)", command: "recon --encode datamatrix \"199001011234\" -o id.png" },
        ExampleHelp { description: "EAN-13 retail barcode", command: "recon --encode ean13 \"590123412345\" -o retail.png" },
        ExampleHelp { description: "Code 128 alphanumeric", command: "recon --encode code128 \"RECON-TEST-001\"" },
        ExampleHelp { description: "Encode from stdin", command: "echo \"https://example.com\" | recon --encode qr" },
        ExampleHelp { description: "Encode from file", command: "recon --encode qr --from-file long-url.txt -o link.png" },
        ExampleHelp { description: "List supported formats", command: "recon --encode-list" },
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
    ],
    related: &["-w / --write-out", "--cert"],
    examples: &[
        ExampleHelp { description: "Probe a broker", command: "recon mqtt://broker.example.com:1883/" },
        ExampleHelp { description: "Probe over TLS with JSON output", command: "recon mqtts://broker.example.com:8883/ --mqtt-json" },
        ExampleHelp { description: "Publish a retained message at QoS 1", command: "recon mqtt://broker/devices/fan/state -d \"on\" --qos 1 --retain" },
        ExampleHelp { description: "Subscribe to a topic filter, exit after 10 messages", command: "recon mqtt://broker/ --subscribe \"devices/+/state\" --count 10 -v" },
        ExampleHelp { description: "Fall back to MQTT 3.1.1 on a legacy broker", command: "recon mqtt://legacy-broker/ --mqtt-version 3" },
    ],
};

static TOPIC_SCRIPT: Topic = Topic {
    title: "Scripting (--script)",
    description: "Run a Rhai script that drives the recon probe API. Scripts can\n\
                  chain requests, branch on results, loop, and build multi-step\n\
                  health checks. The script's `return N` (integer) becomes the\n\
                  process exit code; uncaught exceptions map to the same exit\n\
                  codes as the CLI (7 connect-refused, 28 timeout, 67 auth).\n\
                  `--script` is mutually exclusive with a positional URL.\n\
                  CLI flags (-H, -k, --connect-timeout, etc.) act as defaults\n\
                  that per-call opts maps can override.\n\
                  \n\
                  Script resolution: when PATH isn't found as given, recon\n\
                  looks in ~/.recon/script/PATH (and auto-appends .rhai when\n\
                  PATH has no extension). Drop reusable scripts in\n\
                  ~/.recon/script/ and call them by bare name:\n\
                    recon --script health",
    flags: &[
        FlagHelp { flags: "--script <PATH>", description: "Load and run a .rhai file. Falls back to\n~/.recon/script/<PATH> when the path doesn't exist as given\n(with auto-.rhai extension when PATH has none).\nExample: recon --script checks.rhai\n         recon --script health     # -> ~/.recon/script/health.rhai" },

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
        FlagHelp { flags: "file_read(path)", description: "Read local file (or file:// URL) as a Rhai Blob (Vec<u8>)." },

        FlagHelp { flags: "print(x)", description: "Rhai built-in. Writes x + newline to stdout." },
        FlagHelp { flags: "sleep_ms(n)", description: "Block the current thread for n milliseconds." },
        FlagHelp { flags: "env(name) / env(name, default)", description: "Read an environment variable. Empty string (or default) when unset." },
        FlagHelp { flags: "now() / now_ms()", description: "Unix seconds or milliseconds as i64." },
        FlagHelp { flags: "assert(cond, msg)", description: "Throw a Rhai exception when cond is false." },
        FlagHelp { flags: "json_parse(s) / json_stringify(x)", description: "Round-trip JSON text ↔ Rhai values (null ↔ (), bool, int, float,\nstring, array, object ↔ map)." },
    ],
    related: &["--script", "-H", "-k", "--connect-timeout", "--max-time", "-L"],
    examples: &[
        ExampleHelp { description: "Hello world", command: "echo 'return 0;' > hi.rhai && recon --script hi.rhai" },
        ExampleHelp { description: "Health-check a URL, exit 1 on non-2xx", command: r#"recon --script check.rhai"# },
        ExampleHelp { description: "Multi-step flow: DNS → HTTP → assert", command: "recon --examples  # see SCRIPTING section" },
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
                  for the full list compiled in.",
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
        "script" | "scripting" | "rhai" => Some(&TOPIC_SCRIPT),
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
