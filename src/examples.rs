use colored::Colorize;

pub fn print() {
    let title = "recon — usage examples";
    println!("\n{}\n", title.bold());

    section("GETTING STARTED");

    example("Bootstrap the ~/.recon/ layout (idempotent, never overwrites)", &[
        "recon --init",
    ]);
    note("Creates ~/.recon/{script,jars,sni}/ and a commented config.toml. Safe to re-run — existing files are skipped.");

    section("HTTP REQUESTS");

    example("GET request (default method)", &[
        "recon https://httpbin.org/get",
        "recon --url https://httpbin.org/get",
    ]);
    example("--url as a curl-compatible alternative to the positional argument (--url)", &[
        "recon --url https://httpbin.org/get",
        r#"recon --url https://httpbin.org/post -d '{"key": "value"}' -H "Content-Type: application/json""#,
        "recon --url https://example.com --cert",
        "recon --url example.com --dns",
    ]);
    example("POST with a JSON body (-d infers POST when method is GET)", &[
        r#"recon https://httpbin.org/post -d '{"name": "alice", "role": "admin"}'"#,
        r#"recon https://httpbin.org/post -d '{"name": "alice"}' -H "Content-Type: application/json""#,
    ]);
    example("Explicit HTTP method (-X / --request)", &[
        r#"recon https://httpbin.org/put  -X PUT   -d '{"active": true}'"#,
        r#"recon https://httpbin.org/patch -X PATCH -d '{"email": "new@example.com"}'"#,
        "recon https://httpbin.org/delete -X DELETE",
    ]);
    example("Send body from a file (prefix path with @)", &[
        r#"recon https://api.example.com/upload -d @payload.json -H "Content-Type: application/json""#,
        "recon https://api.example.com/upload -d @./data/request.xml -H \"Content-Type: application/xml\"",
    ]);
    example("Custom User-Agent (-A / --user-agent)", &[
        r#"recon https://httpbin.org/user-agent -A "MyBot/1.0""#,
        r#"recon https://httpbin.org/user-agent -A "Mozilla/5.0 (compatible)""#,
    ]);
    example("Custom headers (-H, repeatable)", &[
        r#"recon https://api.example.com/data -H "Authorization: Bearer eyJhbGci..." -H "X-Request-ID: abc123""#,
        r#"recon https://api.example.com/data -H "Accept: application/json" -H "X-Api-Version: 2""#,
    ]);
    example("Connection timeout (--connect-timeout, default 30s)", &[
        "recon https://slow.example.com --connect-timeout 5",
        "recon https://api.example.com  --connect-timeout 60",
    ]);
    example("HTTP Basic authentication (-u / --user)", &[
        r#"recon https://httpbin.org/basic-auth/alice/s3cr3t -u alice:s3cr3t"#,
        r#"recon https://api.example.com/private -u admin:password -p"#,
        r#"recon https://intranet.corp/api -u alice:s3cr3t -H "Accept: application/json""#,
    ]);
    example("Send a Referer header (-e / --referer)", &[
        "recon https://api.example.com/ -e https://dashboard.example.com/",
        "recon https://api.example.com/ --referrer https://dashboard.example.com/",
    ]);
    example("Save to a file named after the URL (-O / --remote-name)", &[
        "recon https://example.com/files/report.pdf -O",
        "recon https://example.com/files/report.pdf -O -L",
    ]);
    example("Upload a local file (-T / --upload-file, defaults to PUT)", &[
        "recon https://api.example.com/files/img.jpg -T ./img.jpg",
        "recon https://api.example.com/upload -T payload.json -X POST",
    ]);
    example("Send -d data as URL query parameters with GET (-G / --get)", &[
        "recon https://httpbin.org/get -G -d 'q=rust&lang=en'",
        "recon https://api.example.com/search -G -d 'query=hello&page=1&limit=20'",
        "recon https://api.example.com/items -G -d 'filter=active'",
    ]);
    note("-G appends the -d value as a query string; the request body is empty.");

    section("CURL COMPATIBILITY — QUICK WINS (0.20.0)");

    example("JSON body shorthand (--json)", &[
        r#"recon --json '{"a":1}' https://httpbin.org/post"#,
        r#"recon --json @payload.json https://api.example.com/"#,
    ]);
    note("Auto-sets Content-Type: application/json and Accept: application/json unless -H overrides.");

    example("Data variants (--data-raw / --data-binary / --data-urlencode)", &[
        r#"recon --data-raw '@literal' https://httpbin.org/post"#,
        r#"recon --data-binary @image.bin https://api.example.com/upload"#,
        r#"recon --data-urlencode "name=Jane Doe" --data-urlencode "city=New York" https://httpbin.org/post"#,
    ]);

    example("Compressed response (--compressed)", &[
        "recon --compressed https://httpbin.org/brotli -o /tmp/body.bin",
        "recon --compressed https://httpbin.org/gzip",
    ]);
    note("Requests gzip / deflate / brotli / zstd; decompresses transparently.");

    example("Total-time cap (--max-time)", &[
        "recon --max-time 5 https://example.com/slow",
        "recon --max-time 0.5 https://httpbin.org/delay/5",
    ]);
    note("Aborts after the given seconds (fractional allowed); exit code 28.");

    example("Fail with body (--fail-with-body)", &[
        "recon --fail-with-body -o err.html https://httpbin.org/status/404",
    ]);
    note("Exits non-zero on 4xx/5xx but keeps the response body for inspection.");

    example("Save with Content-Disposition filename (-J / --remote-header-name)", &[
        "recon -O -J https://example.com/downloads/report",
        "recon -O -J --output-dir ./downloads https://example.com/file",
    ]);

    example("Create directories / prefix output (--create-dirs, --output-dir)", &[
        "recon --create-dirs -o /tmp/nested/sub/file.txt https://example.com/data",
        "recon --output-dir ./out -O https://example.com/file.txt",
    ]);

    example("Preserve server mtime (--remote-time)", &[
        "recon --remote-time -o page.html https://example.com/",
    ]);

    example("Scriptable metrics (-w / --write-out)", &[
        r#"recon -w "%{http_code} %{time_total}s\n" https://example.com/"#,
        r#"recon -w "%{json}" -o /dev/null https://example.com/"#,
        r#"recon -w "%{header{content-type}}\n" -o /dev/null https://example.com/"#,
    ]);
    note("Runs AFTER the response. See `--help write-out` for the full variable list.");

    section("REDIRECTS");

    example("Follow redirects (-L / --location)", &[
        "recon https://httpbin.org/redirect/3 -L",
    ]);
    example("Limit the number of redirects followed (--max-redirs)", &[
        "recon https://httpbin.org/redirect/10 -L --max-redirs 3",
    ]);
    example("Show headers at every hop in the redirect chain (--LHEAD)", &[
        "recon https://httpbin.org/redirect/3 --LHEAD",
        "recon http://github.com --LHEAD",
    ]);

    section("OUTPUT CONTROL");

    example("Print response headers along with the body (-i / --include)", &[
        "recon https://httpbin.org/get -i",
    ]);
    example("Verbose mode — shows connection, TLS status, and request/response headers (-v)", &[
        "recon https://httpbin.org/get -v",
        r#"recon https://httpbin.org/get -v -H "X-Debug: true""#,
    ]);
    example("Extra verbose — also shows TLS certificate summary, auth info, and elapsed time (-vv)", &[
        "recon https://httpbin.org/get -vv",
        "recon https://expired.badssl.com -vv --insecure",
        r#"recon https://httpbin.org/basic-auth/alice/s3cr3t -u alice:s3cr3t -vv"#,
    ]);
    note("-vv makes a second TLS connection to retrieve the certificate summary (debug mode).");
    example("Print only the HTTP status code (--status)", &[
        "recon https://httpbin.org/get --status",
        "recon https://httpbin.org/status/404 --status",
        "recon https://api.example.com/health --status -L",
    ]);
    example("Show errors even when silenced (-S / --show-error, curl-compat)", &[
        "recon https://httpbin.org/status/500 -sS",
        "recon https://api.example.com/data -s --show-error",
    ]);
    note("-S / --show-error matches curl: re-enables error diagnostics that -s suppressed.");
    example("Print only the response headers, no body (-I / --head)", &[
        "recon https://httpbin.org/get --head",
        "recon https://httpbin.org/get -I",
        "recon https://api.example.com/users -I",
    ]);
    example("Print status line, all headers, and body (--full)", &[
        "recon https://httpbin.org/get --full",
        "recon https://api.example.com/data --full -p",
    ]);
    example("Prettify response body — auto-detects JSON, XML, HTML, YAML, CSV, TSV (-p / --prettify)", &[
        "recon https://httpbin.org/get -p",
        "recon https://httpbin.org/xml -p",
        "recon https://api.example.com/report.csv -p",
        "recon https://httpbin.org/get -i -p",
    ]);
    example("Prettify a payload from stdin / clipboard (--stdin)", &[
        "pbpaste | recon --stdin -p",
        "pbpaste | recon --stdin --prettify-as json",
        "recon --stdin -p --prettify-as json -o pretty.json < raw.json",
    ]);
    note("--stdin reads the body from stdin and runs the same post-fetch pipeline (prettify, --output-charset, -o) without making an HTTP request. Pairs with --prettify-as to force the format when the input has no Content-Type to hint from.");

    example("Force prettify format with --prettify-as", &[
        "recon https://example.com/api -p --prettify-as json",
        "recon https://example.com/feed -p --prettify-as xml",
    ]);
    note("--prettify-as overrides auto-detection. Useful when the server returns the wrong Content-Type (e.g. text/plain for JSON) or when -p's body sniff guesses wrong. Implies -p so you can drop the explicit -p when using it.");

    example("Clipboard input/output (--clipboard, --from-clipboard, --to-clipboard)", &[
        "recon --clipboard --prettify-as json",
        "recon --clipboard both --prettify-as json",
        "recon --from-clipboard -p",
        "recon https://api.example.com/data --to-clipboard",
        "cat data.json | recon --to-clipboard",
    ]);
    note("--clipboard alone auto-resolves direction: 'out' when there's already an input source (URL, --stdin, --from-clipboard), 'in' otherwise. The 'both' value is the explicit in-place form. Native cross-platform via the arboard crate — no need to pipe through pbpaste/pbcopy/xclip.");

    example("Auto-detected stdin (drop --stdin when piped)", &[
        "echo '{\"a\":1}' | recon -p",
        "cat raw.json | recon --prettify-as json",
        "curl -s https://api.example.com/data | recon -p --to-clipboard",
    ]);
    note("When stdin is piped (not a TTY) and no URL or input flag is given, recon implicitly enters --stdin mode. Interactive invocation (TTY stdin) without a URL still produces a usage error — the auto-detect only fires for actual pipes.");

    example("Save response body to a file (-o / --output)", &[
        "recon https://example.com/image.png -o image.png",
        "recon https://api.example.com/export.csv -o export.csv -s",
        "recon https://api.example.com/data -p -o pretty.json",
    ]);
    example("Show a progress meter when saving to a file (--progress)", &[
        "recon https://example.com/large-file.zip -o large-file.zip --progress",
        "recon https://releases.example.com/app.tar.gz -o app.tar.gz --progress",
    ]);
    note("Progress is opt-in — unlike curl, it is never shown by default.");
    example("Hash-style progress bar (-# / --progress-bar, curl parity)", &[
        "recon https://example.com/large-file.zip -O -#",
        "recon https://speed.hetzner.de/100MB.bin -o /tmp/test.bin -#",
        "recon --input-file urls.txt --remote-name-all -#",
    ]);
    note("-# activates the progress meter and switches to a # character bar style. Equivalent to curl -# or curl --progress-bar.");
    example("Silent mode — suppress informational output (-s / --silent)", &[
        "recon https://httpbin.org/get -s",
        "recon https://api.example.com/data -s -o result.json",
    ]);

    section("ERROR HANDLING");

    example("Exit non-zero on HTTP 4xx/5xx responses (-f / --fail)", &[
        "recon https://httpbin.org/status/404 -f",
        "recon https://api.example.com/data -f && echo OK || echo FAILED",
    ]);
    example("Show full internal error chain for debugging (--FULL-ERRORS)", &[
        "recon https://expired.badssl.com --FULL-ERRORS",
        "recon https://httpbin.org/get --FULL-ERRORS",
    ]);

    section("TLS / INSECURE");

    example("Skip TLS certificate verification (-k / --insecure)", &[
        "recon https://self-signed.example.com -k",
        "recon https://expired.badssl.com -k",
        "recon https://internal.corp:8443 -k -p",
        "recon https://staging.example.com -k -i",
    ]);
    note("Disables hostname, expiry, and chain verification. Use only on hosts you control or trust.");

    example("Inspect a server's TLS certificate (--cert)", &[
        "recon https://example.com --cert",
        "recon example.com --cert",
        "recon example.com:8443 --cert",
    ]);
    note("Works with expired, self-signed, or hostname-mismatched certs — verification is intentionally skipped.");

    example("Pin a minimum TLS version (curl-compat)", &[
        "recon --tlsv1.2 https://example.com -I",
        "recon --tlsv1.3 https://example.com -I",
    ]);
    note("Rejects handshakes below the stated version. --tlsv1.3 wins when both are set.");

    example("Trust an extra PEM CA without disabling verification", &[
        "recon --cacert /etc/ssl/internal-ca.pem https://internal.corp",
    ]);

    example("Reject revoked certs via CRL (--crlfile)", &[
        "recon https://example.com --crlfile /etc/pki/tls/crls/all.pem",
        "recon https://api.example.com --crlfile /tmp/issuer.crl.pem -v",
    ]);
    note("--crlfile loads PEM-encoded CRLs (multi-CRL bundles supported). Server certs found in any loaded CRL are rejected by the TLS handshake. Convert DER CRLs to PEM via 'openssl crl -inform DER -in <path> -outform PEM'.");

    example("Proxy CA configuration (--proxy-capath, --proxy-ca-native)", &[
        "recon https://example.com --proxy http://corp-proxy:3128 --proxy-capath /etc/pki/proxy/",
        "recon https://example.com --proxy https://proxy:8443 --proxy-ca-native",
    ]);
    note("reqwest 0.12 doesn't expose per-proxy TLS roots — the global ClientBuilder TLS config covers both server and proxy connections. These flags exist for curl-parity and augment the global config.");

    example("Bind outgoing socket to a specific local IP", &[
        "recon --interface 10.0.0.5 https://example.com",
        "recon --interface ::1 https://[::1]:8080/",
    ]);
    note("Interface *names* (eth0, en0) are not yet resolved; pass the address directly.");

    example("Custom DNS resolver for HTTP requests", &[
        "recon --dns-servers 1.1.1.1,8.8.8.8 https://example.com",
        "recon --dns-servers 1.1.1.1:5353 --dns-ipv4-addr 10.0.0.5 https://example.com",
    ]);
    note("--dns-servers accepts comma-separated `IP` or `IP:PORT`. --dns-ipv4-addr / --dns-ipv6-addr bind DNS queries to a specific local address. --dns-interface (named interface) is not yet plumbed; use the address form for now.");

    section("BROWSER FINGERPRINT IMPERSONATION (0.77.0, opt-in)");

    example("Impersonate Chrome 131 against an HTTPS endpoint", &[
        "recon --impersonate chrome_131 https://example.com/",
        "recon --impersonate chrome-131 https://httpbin.org/headers",
    ]);
    note("Requires a build with --features impersonate (BoringSSL via wreq). The default recon binary errors on these flags with a rebuild hint pointing at the recon-impersonate release artifact. Hyphens in the profile name are accepted as a convenience (chrome-131 ≡ chrome_131).");

    example("Impersonate Firefox / Safari / Edge / mobile / OkHttp", &[
        "recon --impersonate firefox_128 https://example.com/",
        "recon --impersonate safari_17.5 https://example.com/",
        "recon --impersonate edge_131 https://example.com/",
        "recon --impersonate chrome_android_131 https://example.com/",
        "recon --impersonate safari_ios_17.4.1 https://example.com/",
        "recon --impersonate okhttp_5 https://example.com/",
    ]);
    note("Profile names follow wreq_util's serde rename convention (underscores + dots). See `recon --help impersonate` for the full list of supported profiles.");

    example("Verify the live fingerprint against tls.peet.ws", &[
        "recon --impersonate chrome_131 https://tls.peet.ws/api/all",
        "recon --impersonate firefox_128 https://tls.peet.ws/api/all",
    ]);
    note("tls.peet.ws echoes back the JA3/JA4/H2 fingerprint observed from the server side — useful for confirming the impersonation actually changed what hits the wire.");

    example("Use from a script via the http() opts map", &[
        "recon --script script/impersonate.rhai chrome_131 https://example.com/",
    ]);
    note("The `impersonate` opts key on http() takes a profile name string, just like the CLI flag. Demo at script/impersonate.rhai.");

    example("Raw fingerprint overrides (deferred — currently error)", &[
        "recon --ja3 \"771,4865-...,0-23-...,29-23-24,0\" https://example.com/   # not yet implemented",
        "recon --ja4 t13d1516h2_8daaf6152771_b1ff8ab2d16f https://example.com/   # not yet implemented",
        "recon --http2-fingerprint \"1:65536,4:6291456|...|0|m,a,s,p\" https://example.com/   # not yet implemented",
    ]);
    note("These flags are reserved in the CLI for forward-compatibility but error at runtime — implementation is deferred (no concrete captured-fingerprint use case has driven the work yet). Use --impersonate <profile> for now; named profiles cover the common captcha-testing cases. See OUT-OF-SCOPE.md for the rationale.");

    example("Rate control and slow-transfer abort", &[
        "recon --limit-rate 500K https://example.com/big.bin -o big.bin",
        "recon --limit-rate 2M https://cdn/data -o data.bin",
        "recon --speed-limit 1024 --speed-time 10 https://slow.example.com -o x",
    ]);
    note("--limit-rate suffixes: K/M/G/T (1024-based), B for bytes. --speed-limit together with --speed-time aborts when the rolling rate stays below BYTES/sec for SECS seconds.");

    section("DNS LOOKUPS");

    example("Look up common DNS records for a host (--dns)", &[
        "recon example.com --dns",
        "recon https://example.com --dns",
    ]);
    example("Query specific record type(s) (--dns-type, comma-separated)", &[
        "recon example.com --dns --dns-type A",
        "recon example.com --dns --dns-type MX",
        "recon example.com --dns --dns-type A,AAAA,MX,TXT",
        "recon example.com --dns --dns-type NS,SOA",
        "recon 8.8.8.8   --dns --dns-type PTR",
        "recon _dmarc.example.com --dns --dns-type TXT",
    ]);
    note("Supported types: A  AAAA  CNAME  MX  NS  TXT  SOA  PTR  SRV  CAA  and more.");

    section("WHOIS");

    example("WHOIS lookup for a domain or IP address (--whois)", &[
        "recon example.com --whois",
        "recon 8.8.8.8    --whois",
        "recon 2606:4700:: --whois",
    ]);
    note("Follows the full referral chain: IANA → registry → registrar.");

    section("PING");

    example("ICMP ping (no port — requires no root on macOS) (--ping)", &[
        "recon example.com --ping",
        "recon 8.8.8.8 --ping",
    ]);
    example("TCP ping — connect/disconnect on the given port (--ping)", &[
        "recon example.com:443 --ping",
        "recon example.com:22  --ping",
        "recon api.example.com:8080 --ping",
    ]);
    example("Control the number of probes sent (--ping-count)", &[
        "recon example.com --ping --ping-count 10",
        "recon example.com:443 --ping --ping-count 1",
    ]);

    section("TRACEROUTE");

    example("Trace the route to a host (--traceroute / --trace)", &[
        "recon example.com --traceroute",
        "recon example.com --trace",
    ]);
    example("Trace to a specific port (passed to traceroute -p)", &[
        "recon example.com:443  --traceroute",
        "recon example.com:8080 --trace",
    ]);
    example("Limit the number of hops (--max-hops, default 30)", &[
        "recon example.com --traceroute --max-hops 15",
    ]);

    section("COOKIE JAR");

    example("Make a request using a named cookie jar (--cookiejar)", &[
        "recon https://example.com/login --cookiejar mysession",
        r#"recon https://api.example.com/data --cookiejar work -H "Content-Type: application/json""#,
    ]);
    note("Cookies are stored in ~/.recon/jars/<name>.db — or pass an absolute/relative .db path.");
    example("Use the default cookie jar (omit the value after --cookiejar)", &[
        "recon https://example.com/login --cookiejar",
        "recon https://example.com/dashboard --cookiejar",
        "recon --cookiejar --cookies",
    ]);
    note("Omitting the value uses the jar named 'default' (~/.recon/jars/default.db).");
    example("List all cookies in a jar (--cookies)", &[
        "recon --cookiejar mysession --cookies",
        "recon --cookiejar --cookies",
    ]);
    example("Add or update a cookie manually (--cookie-set)", &[
        r#"recon --cookiejar mysession --cookie-set "sessionid=abc123; Domain=example.com; Path=/; HttpOnly""#,
        r#"recon --cookiejar mysession --cookie-set "token=xyz; Domain=api.example.com; Max-Age=3600; Secure""#,
    ]);
    note("Format: name=value; Domain=…; [Path=/]; [Secure]; [HttpOnly]; [Max-Age=N]");
    example("Delete a cookie by its ID (--cookie-delete)", &[
        "recon --cookiejar mysession --cookie-delete 3",
    ]);
    note("Run --cookies first to see IDs.");
    example("Full login flow — save cookies then use them", &[
        r#"recon https://example.com/login -X POST -d "user=alice&pass=s3cr3t" --cookiejar mysession"#,
        "recon https://example.com/dashboard --cookiejar mysession",
        "recon https://example.com/dashboard --cookiejar mysession -p",
    ]);

    section("SCP DOWNLOAD");

    example("Download a file via SCP (uses SSH agent or default key files automatically)", &[
        "recon scp://neh.localhost/home/thomas.bjork/file.tgz",
        "recon scp://builds.internal/var/releases/app-1.0.tar.gz",
    ]);
    note("The file is saved using the remote basename in the current directory.");
    example("Specify the SSH user in the URL", &[
        "recon scp://thomas@neh.localhost/home/thomas.bjork/file.tgz",
        "recon scp://deploy@10.0.0.5/srv/releases/latest.tar.gz",
    ]);
    example("Non-standard SSH port", &[
        "recon scp://thomas@neh.localhost:2222/home/thomas.bjork/file.tgz",
    ]);
    example("Specify an SSH private key (--ssh-key)", &[
        "recon scp://neh.localhost/home/thomas.bjork/file.tgz --ssh-key ~/.ssh/id_deploy",
        "recon scp://deploy@server/srv/app.tar.gz --ssh-key ~/.ssh/deploy_ed25519",
    ]);
    example("Encrypted private key — provide the passphrase with --ssh-pass", &[
        "recon scp://neh.localhost/file.tgz --ssh-key ~/.ssh/id_rsa --ssh-pass 'myPassphrase'",
    ]);
    example("SSH password authentication via -u user:pass", &[
        "recon scp://neh.localhost/file.tgz -u thomas:s3cr3t",
        "recon scp://server/file.tgz -u deploy:deploy123 --ssh-pass deploy123",
    ]);
    note("--ssh-pass serves as the key passphrase when --ssh-key is given, or the login password otherwise.");
    example("Save to a specific path (-o)", &[
        "recon scp://neh.localhost/home/thomas.bjork/file.tgz -o /tmp/file.tgz",
        "recon scp://builds.internal/var/releases/app.tar.gz -o ./releases/",
    ]);
    note("If -o is a directory the remote filename is preserved inside it.");
    example("Download with a progress bar (--progress)", &[
        "recon scp://neh.localhost/large-dataset.tar.gz --progress",
        "recon scp://thomas@server/backup.tar.gz -o /backups/ --progress",
    ]);
    example("Skip SSH host key verification (--insecure)", &[
        "recon scp://staging.internal/deploy.tar.gz --insecure",
        "recon scp://thomas@new-server/file.tgz --insecure --ssh-key ~/.ssh/id_ed25519",
    ]);
    note("--insecure skips ~/.ssh/known_hosts checking. Use only on hosts you control.");
    example("Specify both public and private key explicitly (--ssh-pubkey)", &[
        "recon scp://server/file.tgz --ssh-key ~/.ssh/custom_rsa --ssh-pubkey ~/.ssh/custom_rsa.pub",
    ]);

    section("SSH INTERACTIVE SHELL");

    example("Open an interactive SSH shell (ssh://)", &[
        "recon ssh://myserver.example.com",
        "recon ssh://alice@myserver.example.com",
        "recon ssh://alice@myserver.example.com:2222",
    ]);
    example("Explicit SSH key file (--ssh-key)", &[
        "recon ssh://myserver.example.com --ssh-key ~/.ssh/id_deploy",
        "recon ssh://alice@myserver.example.com --ssh-key ~/.ssh/id_rsa --ssh-pass 'passphrase'",
    ]);
    example("Password authentication (-u / --user or --ssh-pass)", &[
        "recon ssh://myserver.example.com -u alice:s3cr3t",
        "recon ssh://alice@myserver.example.com --ssh-pass s3cr3t",
    ]);
    example("Skip host key verification (dev/test only, -k / --insecure)", &[
        "recon ssh://dev.local --insecure",
    ]);
    note("SSH agent keys are tried first. Add your key with: ssh-add ~/.ssh/id_ed25519");

    section("TELNET CLIENT");

    example("Connect to a Telnet server (telnet://)", &[
        "recon telnet://bbs.example.com",
        "recon telnet://host:8023",
    ]);
    example("Short connection timeout", &[
        "recon telnet://host --connect-timeout 5",
    ]);
    note("Authentication is interactive — type your credentials when prompted. Press Ctrl+D to disconnect.");

    section("EMAIL PROTECTION");

    example("Validate the SPF record for a domain (--spf)", &[
        "recon example.com --spf",
        "recon google.com --spf",
    ]);
    note("Recursively resolves include: and redirect= chains, enforces the 10-lookup limit.");
    example("Validate the DMARC record and policy (--dmarc)", &[
        "recon example.com --dmarc",
        "recon google.com --dmarc",
    ]);
    note("Checks policy strength, alignment modes, reporting URIs, and external authorization.");
    example("Validate DKIM records for specific selectors (--dkim, repeatable)", &[
        "recon google.com --dkim google",
        "recon google.com --dkim google --dkim default",
        "recon example.com --dkim selector1 --dkim selector2",
    ]);
    note("Each selector is checked independently. Reports key type, size, hash, and flags.");
    example("Validate MTA-STS DNS record and HTTPS policy (--mta-sts)", &[
        "recon google.com --mta-sts",
        "recon example.com --mta-sts",
    ]);
    note("Fetches the policy from https://mta-sts.<domain>/.well-known/mta-sts.txt and cross-checks MX patterns.");
    example("Validate the BIMI record (--bimi, optional selector, default: \"default\")", &[
        "recon google.com --bimi",
        "recon cnn.com --bimi",
        "recon example.com --bimi myselector",
    ]);
    note("Checks logo URL (must be SVG over HTTPS) and VMC certificate if present.");
    example("Validate the TLS-RPT reporting record (--tls-rpt)", &[
        "recon google.com --tls-rpt",
        "recon example.com --tls-rpt",
    ]);
    note("Validates reporting URIs (mailto: and https:). Best used alongside --mta-sts.");
    example("Run multiple email checks together", &[
        "recon google.com --dmarc --spf",
        "recon google.com --dmarc --spf --dkim google --mta-sts --tls-rpt",
        "recon example.com --dmarc --spf --dkim default --bimi",
    ]);
    note("Checks cross-reference each other: DMARC notes SPF/DKIM alignment, BIMI checks DMARC policy strength, MTA-STS and TLS-RPT note co-presence.");
    example("Combine email checks with cert and DNS inspection", &[
        "recon google.com --cert --dns --dns-type A,AAAA,MX,TXT --dmarc --spf --dkim google",
        "recon example.com --cert --dmarc --spf --mta-sts --tls-rpt",
    ]);
    note("All composable flags run sequentially in one invocation.");

    section("WEB SERVER");

    example("Serve the current directory over HTTP (--serve)", &[
        "recon --serve 8080",
        "recon --serve",
    ]);
    note("Default port is 80. Serves files and directory listings from the current directory.");
    example("Serve over HTTPS (--serve-tls)", &[
        "recon --serve-tls 8443",
        "recon --serve-tls",
    ]);
    note("Default port is 443. Requires ~/.recon/cert.pem and ~/.recon/key.pem (or --serve-cert/--serve-key).");
    example("Run both HTTP and HTTPS simultaneously", &[
        "recon --serve 8080 --serve-tls 8443",
    ]);
    example("Force HTTP/2 only on HTTPS (--http-version)", &[
        "recon --serve-tls 8443 --http-version 2",
    ]);
    example("Write access log to a file (--serve-log)", &[
        "recon --serve 8080 --serve-log access.log",
        "recon --serve 8080 --serve-tls 8443 --serve-log server.log",
    ]);
    note("Log is always printed to the terminal. --serve-log adds a plain-text copy to a file.");
    example("Use custom TLS certificate files (--serve-cert, --serve-key)", &[
        "recon --serve-tls 8443 --serve-cert ./my-cert.pem --serve-key ./my-key.pem",
    ]);
    example("Generate a self-signed certificate for local development", &[
        "openssl req -x509 -newkey rsa:2048 -keyout ~/.recon/key.pem -out ~/.recon/cert.pem -days 365 -nodes -subj \"/CN=localhost\"",
    ]);
    note("Run this once. recon will use these files by default for --serve-tls.");

    example("SNI: different certificates per hostname (--serve-sni)", &[
        "recon --serve-sni \"myapp.local:certs/myapp.pem:certs/myapp-key.pem\" --serve-sni \"api.local:certs/api.pem:certs/api-key.pem\"",
        "recon --serve-tls 8443 --serve-sni ~/.recon/sni/",
        "recon --serve-sni sni.conf",
    ]);
    note("Three formats: inline host:cert:key, directory with <host>-cert.pem/<host>-key.pem files, or a config file.");

    section("COMBINING FLAGS");

    example("POST JSON, follow redirects, prettify response", &[
        r#"recon https://api.example.com/users -d '{"name":"alice"}' -H "Content-Type: application/json" -L -p"#,
    ]);
    example("Show full headers and prettify", &[
        "recon https://httpbin.org/get -i -p",
    ]);
    example("Save prettified JSON to file silently", &[
        "recon https://api.example.com/data -p -s -o result.json",
    ]);
    example("Check if an endpoint is up (exit code only)", &[
        "recon https://api.example.com/health -f -s",
    ]);
    example("Inspect the redirect chain and then see final body prettified", &[
        "recon http://github.com --LHEAD -p",
    ]);
    example("Auth + insecure + prettify (self-signed staging server)", &[
        r#"recon https://staging.internal/api/data -u alice:s3cr3t -k -p"#,
    ]);
    example("Search with query params using GET, prettify the response", &[
        "recon https://api.example.com/search -G -d 'q=rust' -p",
    ]);
    example("Download a file with progress, silencing other output", &[
        "recon https://example.com/release.tar.gz -o release.tar.gz --progress -s",
    ]);

    section("JWT TOKENS");

    example("Sign a JSON payload (HS256, iat added automatically)", &[
        r#"recon --jwt-sign --jwt-secret mysecret -d '{"sub":"alice","iss":"acme"}'"#,
    ]);
    example("Sign with claim flags (added only if not already in payload)", &[
        r#"recon --jwt-sign --jwt-secret mysecret --jwt-sub alice --jwt-iss acme -d '{"role":"admin"}'"#,
        r#"recon --jwt-sign --jwt-secret mysecret --jwt-exp now --jwt-iss acme -d '{"sub":"alice"}'"#,
    ]);
    example("Sign with HS512 algorithm", &[
        r#"recon --jwt-sign --jwt-secret mysecret --jwt-alg HS512 -d '{"sub":"alice"}'"#,
    ]);
    example("Complete a partial token (header.payload, missing signature)", &[
        r#"recon --jwt-sign --jwt-secret mysecret -d 'eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJhbGljZSJ9'"#,
    ]);
    example("Read token from a file", &[
        r#"recon --jwt-view token.jwt"#,
        r#"recon --jwt-validate --jwt-secret mysecret token.jwt"#,
    ]);
    example("View token contents (no verification)", &[
        r#"echo $TOKEN | recon --jwt-view"#,
        r#"recon --jwt-view --jwt-json-report -d <token>"#,
    ]);
    example("Validate signature only", &[
        r#"echo $TOKEN | recon --jwt-validate --jwt-secret mysecret"#,
    ]);
    example("Validate with time-based checks", &[
        r#"recon --jwt-validate --jwt-secret mysecret --jwt-validate-exp -d <token>"#,
        r#"recon --jwt-validate --jwt-secret mysecret --jwt-validate-exp --jwt-validate-nbf -d <token>"#,
    ]);
    example("Full validation with issuer and audience checks", &[
        r#"recon --jwt-validate --jwt-secret mysecret --jwt-validate-full --jwt-iss acme --jwt-aud api -d <token>"#,
    ]);
    example("Validate at a specific point in time (--jwt-exp as reference time)", &[
        r#"recon --jwt-validate --jwt-secret mysecret --jwt-validate-exp --jwt-exp 1700000000 -d <token>"#,
    ]);
    example("JSON report for scripting", &[
        r#"recon --jwt-validate --jwt-secret mysecret --jwt-validate-full --jwt-json-report -d <token> | jq .valid"#,
    ]);

    section("NETWORK STATUS");

    example("Check connectivity (requires ~/.recon/config.toml)", &[
        "recon --netstatus",
    ]);
    example("Use in scripts — silent mode, exit code only", &[
        "recon --netstatus --silent && deploy.sh",
    ]);
    note("The [netstatus] section in ~/.recon/config.toml defines the probes to run.");
    example("Example ~/.recon/config.toml [netstatus] section", &[
        "# [netstatus]",
        "# ip_sources = [\"https://api.ipify.org\", \"https://ifconfig.me/ip\"]",
        "# dns_lookup_domains = [\"example.com\"]",
        "# probes = [",
        "#   \"https://www.google.com\",",
        "#   \"ping://8.8.8.8\",",
        "#   \"dns://8.8.8.8\",",
        "#   \"tcp://8.8.8.8:53\",",
        "#   \"tls://www.google.com:443\",",
        "#   \"ntp://pool.ntp.org\",",
        "# ]",
    ]);
    example("DNS hijack detection (repeat block for multiple servers)", &[
        "# [[netstatus.dns_hijack_checks]]",
        "# server = \"8.8.8.8\"",
        "# domain = \"example.com\"",
        "# expected = \"93.184.216.34\"",
    ]);

    section("HASHING");

    example("Hash a local file", &[
        "recon --hash sha256 ./file.bin",
        "recon --hash md5 Cargo.toml",
    ]);
    example("Hash a remote URL with the full HTTP flag set", &[
        "recon --hash sha512 https://api.example.com/artifact -H \"Authorization: Bearer $T\" -L",
    ]);
    example("Hash from stdin (explicit or implicit)", &[
        "cat data | recon --hash blake3",
        "recon --hash sha256 -",
    ]);
    example("Read through the file:// scheme", &[
        "recon --hash sha256 file:///tmp/data.bin",
    ]);
    example("Alternative output formats (--hash-format)", &[
        "recon --hash sha256 ./file --hash-format base64",
        "recon --hash sha256 ./file --hash-format raw > digest.bin",
    ]);
    example("Write digest to a file (-o)", &[
        "recon --hash sha256 ./file -o digest.hex",
    ]);
    example("List supported algorithms", &[
        "recon --hash-list",
    ]);
    note("Accepted algorithm aliases: sha-256, sha_256, sha3_256, etc. Case-insensitive. See --help hash for the full list.");

    section("COMPRESSION");

    example("Compress a local file with gzip", &[
        "recon --compress gzip ./big.log -o big.log.gz",
        "recon --compress gz Cargo.toml > cargo.gz",
    ]);
    example("Compress to stdout with a quality knob", &[
        "recon --compress zstd --compression-level best ./data -o data.zst",
        "recon --compress brotli --compression-level 9 ./web > web.br",
    ]);
    example("Decompress a file (auto-detect from magic bytes)", &[
        "recon --decompress ./foo.gz",
        "recon --decompress https://cdn/file.zst",
    ]);
    example("Decompress brotli or deflate (explicit algorithm required)", &[
        "recon --decompress brotli ./asset.br",
        "recon --decompress deflate ./raw.zz",
    ]);
    example("Piping streams without touching disk", &[
        "cat data | recon --compress gzip | recon --decompress > data-roundtrip",
    ]);
    example("lz4 / snappy (no level setting)", &[
        "recon --compress lz4 ./data.bin -o data.lz4",
        "recon --compress snappy ./stream.log -o stream.sz",
    ]);
    note("Lz4 and Snappy are frame-format streaming. Both ignore --compression-level; passing one gives a clear error.");

    example("xz and zlib streams", &[
        "recon --compress xz --compression-level best ./data -o data.xz",
        "recon --compress zlib ./blob.bin -o blob.zlib",
    ]);
    note("zlib produces a raw RFC 1950 stream (not gzip-wrapped). xz supports the full 0-9 level range like gzip.");

    example("List supported algorithms", &[
        "recon --compress-list",
    ]);
    note("Level aliases (fastest/fast/default/good/best) map to each algorithm's native scale. See --help compression for the word-to-number table.");

    section("ARCHIVES (zip / tar / tar.gz / tar.xz / tar.bz2)");

    example("Create a zip from multiple files", &[
        "recon --archive report.zip notes.md summary.md",
    ]);
    example("Tar a directory (no compression)", &[
        "recon --archive src.tar src/",
    ]);
    example("Gzipped / xz-compressed / bzipped tar", &[
        "recon --archive backup.tar.gz config/ logs/",
        "recon --archive release.tar.xz dist/",
        "recon --archive snap.tar.bz2 data/",
    ]);
    example("Extract into a chosen directory", &[
        "recon --extract download.zip -o /tmp/unpack/",
        "recon --extract artifact.tar.gz -o /tmp/artifact/",
    ]);
    note("Archive format is inferred from the destination extension for --archive, and from the source extension (plus magic-byte sniff as fallback) for --extract. Supported: .zip, .tar, .tar.gz / .tgz, .tar.xz / .txz, .tar.bz2 / .tbz2.");

    section("ENCODING");

    example("QR code to terminal (ASCII)", &[
        "recon --encode qr \"https://example.com\"",
    ]);
    example("QR code saved to disk (format inferred from extension)", &[
        "recon --encode qr \"https://example.com\" -o qr.svg",
        "recon --encode qr \"Contact: +46-70-123\" -o contact.png",
    ]);
    example("DataMatrix / linear barcodes", &[
        "recon --encode datamatrix \"199001011234\" -o id.png",
        "recon --encode ean13 \"590123412345\" -o retail.png",
        "recon --encode code128 \"RECON-TEST-001\" -o shelf.svg",
        "recon --encode code39 \"SKU-42\" -o label.svg",
    ]);
    example("Input from stdin or a file", &[
        "echo \"https://example.com\" | recon --encode qr",
        "recon --encode qr --from-file long-url.txt -o link.png",
    ]);
    example("Explicit output format (override the extension guess)", &[
        "recon --encode qr \"text\" --encode-format svg",
        "recon --encode qr \"text\" --encode-format png > code.png",
    ]);
    example("List supported formats", &[
        "recon --encode-list",
    ]);
    note("Only encoding is supported in this release. Decoding (image → text) may land in a later version.");

    section("ENCRYPTION");

    example("Generate a fresh age key pair", &[
        "recon --encrypt-keygen -o ~/.config/age/keys.txt",
    ]);
    example("Passphrase-based encrypt / decrypt (interactive prompt)", &[
        "recon --encrypt ./secrets.bin -o secrets.age",
        "recon --decrypt secrets.age -o secrets.bin",
    ]);
    example("Scripted with a passphrase file", &[
        "recon --encrypt ./secrets.bin --passphrase-file ~/.recon/pass -o secrets.age",
        "recon --decrypt secrets.age --passphrase-file ~/.recon/pass -o secrets.bin",
    ]);
    example("Scripted with the env var", &[
        "RECON_PASSPHRASE=... recon --encrypt ./secrets.bin -o secrets.age",
    ]);
    example("Encrypt to an X25519 recipient (or several)", &[
        "recon --encrypt ./payload.bin --recipient age1xyz... -o payload.age",
        "recon --encrypt ./payload.bin --recipient ./alice.pub --recipient ./bob.pub -o payload.age",
    ]);
    example("ASCII armor for email or chat paste", &[
        "recon --encrypt ./note.txt --armor -o note.age.txt",
    ]);
    example("Decrypt a URL-hosted payload", &[
        "recon --decrypt https://cdn/secret.age --identity ~/.config/age/keys.txt -o secret.bin",
    ]);
    note("The --passphrase <TEXT> flag is intentionally not offered (secrets on the command line leak to process lists and shell history). Use --passphrase-file, $RECON_PASSPHRASE, or the interactive prompt.");

    example("Encrypt / decrypt with PGP (shells out to `gpg`) — 0.46.0", &[
        "recon --encrypt ./secret.bin --recipient alice@example.com --armor -o secret.pgp",
        "recon --encrypt ./secret.bin --pgp --recipient 0xDEADBEEF -o secret.pgp",
        "recon --decrypt secret.pgp -o secret.bin        # format auto-detected",
    ]);
    note("Auto-detection: recipients starting with `age1` or matching an existing file path → age backend; anything else (hex fingerprint, key-id, email, uid) → PGP. --pgp / --age override the heuristic. Requires `gpg` on PATH for PGP operations (install gnupg).");

    example("Rotate keys (--rekey) — 0.46.0", &[
        r#"recon --rekey \
      --identity old-key.txt \
      --recipient age1new... \
      old.age -o new.age"#,
        r#"# Cross-backend rotation (age -> PGP):
recon --rekey \
      --identity old-key.txt \
      --pgp --recipient alice@example.com --armor \
      old.age -o new.pgp"#,
    ]);
    note("--rekey decrypts the existing ciphertext with --identity (and/or --passphrase-file for passphrase-encrypted files), then re-encrypts to the new --recipient set. Source format is auto-detected from magic bytes; target backend follows the same rules as plain --encrypt.");

    section("CHECK DIGITS");

    example("Verify a credit card number", &[
        "recon --checkdigit creditcard 4111111111111111",
        "recon --checkdigit visa 4111111111111111",
        "recon --checkdigit amex 378282246310005",
    ]);

    example("Create a credit card number from 15 body digits", &[
        "recon --checkdigit-create visa 411111111111111",
        "recon --checkdigit-create amex 37828224631000",
    ]);

    example("IBAN verification (accepts spaces in input)", &[
        "recon --checkdigit iban SE3550000000054910000003",
        "recon --checkdigit iban 'SE35 5000 0000 0549 1000 0003'",
        "recon --checkdigit iban GB82WEST12345698765432",
    ]);

    example("Create an IBAN — accepts both placeholder and omit form", &[
        "recon --checkdigit-create iban SE0050000000054910000003",
        "recon --checkdigit-create iban SE500000000054910000003",
    ]);

    example("Swedish personnummer (10 or 12 digits, + separator for >=100 yrs)", &[
        "recon --checkdigit personnummer 811228-9874",
        "recon --checkdigit personnummer 19811228-9874",
        "recon --checkdigit-create personnummer 811228987",
    ]);

    example("Other national IDs", &[
        "recon --checkdigit fodselsnummer 15076500565    # Norway",
        "recon --checkdigit henkilotunnus 131052-308T    # Finland",
        "recon --checkdigit sin 046454286                # Canada",
        "recon --checkdigit sa-id 8001015009087          # South Africa",
    ]);

    example("Vehicle Identification Number (VIN)", &[
        "recon --checkdigit vin 1HGBH41JXMN109186",
        "recon --checkdigit-create vin 1HGBH41JMN109186",
    ]);

    example("Passport / ID MRZ (TD1/TD2/TD3)", &[
        "echo 'P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\\nL898902C36UTO7408122F1204159ZE184226B<<<<<10' | recon --checkdigit mrz",
    ]);

    example("Cryptocurrency addresses", &[
        "recon --checkdigit btc 1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
        "recon --checkdigit eth 0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed",
        "recon --checkdigit bech32 bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",
    ]);

    example("EU VAT — single-algorithm countries", &[
        "recon --checkdigit pl-vat 5261040828          # Poland (NIP)",
        "recon --checkdigit it-vat 00743110157         # Italy (Luhn)",
        "recon --checkdigit-create fr-vat 123456789    # France: key computed from SIREN",
        "recon --checkdigit at-vat ATU12345675         # Austria: ATU prefix accepted",
    ]);

    example("EU VAT — multi-variant auto-detect", &[
        "recon --checkdigit es-vat 12345678Z           # Spain: auto-detects NIF",
        "recon --checkdigit es-vat X1234567L           # Spain: auto-detects NIE",
        "recon --checkdigit es-vat A58818501           # Spain: auto-detects CIF",
        "recon --checkdigit bg-vat 7523169263          # Bulgaria: auto-detects EGN",
        "recon --checkdigit bg-vat 175074752           # Bulgaria: auto-detects BULSTAT",
        "recon --checkdigit cz-vat 46505334            # Czech: IČO (8 digits)",
        "recon --checkdigit cz-vat 7301011234          # Czech: rodné číslo (10)",
    ]);

    example("EU VAT — explicit sub-keyword", &[
        "recon --checkdigit es-nif 12345678Z",
        "recon --checkdigit bg-egn 7523169263",
        "recon --checkdigit cz-legal 46505334",
        "recon --checkdigit lv-business 40003032949",
    ]);

    example("EU VAT — with or without country prefix", &[
        "recon --checkdigit pl-vat 5261040828         # bare body",
        "recon --checkdigit pl-vat PL5261040828       # with prefix (stripped)",
        "recon --checkdigit pl-vat DE5261040828       # error: prefix mismatch",
    ]);

    example("EU VAT — comment field surfaces warnings", &[
        "recon --checkdigit se-vat 556036079302        # comment: 'suffix 02 (unusual)'",
    ]);

    example("Non-EU European VAT — single-algorithm", &[
        "recon --checkdigit no-vat 974760673             # Norway MVA",
        "recon --checkdigit uk-vat GB333289454           # UK VAT (GB prefix accepted)",
        "recon --checkdigit ch-vat CHE-100.155.212       # Swiss UID",
        "recon --checkdigit rs-vat 101134702             # Serbia PIB",
        "recon --checkdigit tr-vat 0010213576            # Turkey VKN",
    ]);

    example("Non-EU European VAT — multi-variant auto-detect", &[
        "recon --checkdigit ru-vat 7830002293            # Russia: auto-detects legal (10)",
        "recon --checkdigit ru-vat 500100732259          # Russia: auto-detects individual (12)",
        "recon --checkdigit ua-vat 32855961              # Ukraine: auto-detects EDRPOU (8)",
        "recon --checkdigit ua-vat 1759013776            # Ukraine: auto-detects RNOKPP (10)",
    ]);

    example("IMEI, ABA routing, ISIN, NPI", &[
        "recon --checkdigit imei 490154203237518",
        "recon --checkdigit aba 122105155",
        "recon --checkdigit isin US0378331005",
        "recon --checkdigit npi 1234567893",
    ]);

    example("List all supported algorithms", &[
        "recon --checkdigit-list",
    ]);

    example("Raw output (strip grouping/hyphens)", &[
        "recon --checkdigit creditcard 4111111111111111 --raw",
        "recon --checkdigit-create iban SE500000000054910000003 --raw",
    ]);

    note("Verify output format: <formatted>|<type>|<valid|invalid>|<comment>. Exit 0 valid, 1 invalid, 2 misuse.");

    section("SAMPLE DATA");

    example("10 customers in JSON (default)", &[
        "recon --sample customer",
    ]);
    example("Colon shortcut: NAME[:FORMAT[:COUNT]]", &[
        "recon --sample customer:json:25",
        "recon --sample product::5",
    ]);
    example("Override count or format with explicit flags", &[
        "recon --sample customer --sample-count 50",
        "recon --sample product --sample-format json",
    ]);
    example("Save to a file (bulk mode)", &[
        "recon --sample product --sample-file products.json",
    ]);
    example("Save per-item results (images)", &[
        "recon --sample image --sample-count 5 --sample-file img-{{n}}.jpg",
    ]);
    example("Local lorem ipsum with unit suffix", &[
        "recon --sample lorem --sample-count 3p     # 3 paragraphs",
        "recon --sample lorem --sample-count 50w    # 50 words",
        "recon --sample lorem --sample-count 200c   # 200 characters",
    ]);
    example("Reproducible lorem with --sample-seed", &[
        "recon --sample lorem --sample-count 3p --sample-seed 42",
        "recon --sample lorem --sample-count 50w --sample-seed 1000",
    ]);
    example("Open sample data in an editor", &[
        "recon --sample product --editor zed",
        "recon --sample lorem --sample-count 5p --editor code",
    ]);
    example("List all available samples", &[
        "recon --sample-list",
    ]);
    note("Custom samples (including paid APIs with Bearer tokens) can be added in ~/.recon/config.toml under [sampledata.<name>]. See --help sample for details.");

    section("SMTP / SMTPS (0.44.0)");

    example("Probe a mail server (no auth, no send)", &[
        "recon smtp://smtp.gmail.com:587/",
        "recon smtps://mail.example.com/",
        "recon smtp://localhost:1025/        # MailHog / local test relay",
    ]);
    note("Probe mode connects, reads the greeting, sends EHLO, reports every advertised extension (SIZE, 8BITMIME, PIPELINING, CHUNKING…), AUTH mechanisms, and STARTTLS availability. No message is sent unless --mail-from + --mail-to are given.");

    example("Deliver a test message through a relay", &[
        r#"recon smtp://localhost:25/ \
      --mail-from me@example.com \
      --mail-to you@example.com \
      --mail-subject 'recon test' \
      --mail-body 'hello'"#,
    ]);

    example("Authenticated submission via port 587 (STARTTLS)", &[
        r#"recon smtp://smtp.gmail.com:587/ \
      --smtp-auth user@gmail.com:apppassword \
      --mail-from me@gmail.com \
      --mail-to you@example.com \
      --mail-body 'hi from recon'"#,
    ]);

    example("DKIM-sign an outgoing message", &[
        r#"recon smtp://localhost:25/ \
      --mail-from me@example.com \
      --mail-to you@example.com \
      --mail-body 'signed payload' \
      --dkim-key dkim.pem \
      --dkim-selector recon1 \
      --dkim-domain example.com"#,
    ]);
    note("The DKIM-Signature header is applied to the message before delivery. Signing algorithm is inferred from the key (RSA or Ed25519). Selector must match a TXT record at <selector>._domainkey.<domain> for the receiver to verify.");

    example("Read the body from a file / stdin", &[
        "recon smtp://localhost:25/ --mail-from … --mail-to … --mail-body @message.txt",
        "echo hello | recon smtp://localhost:25/ --mail-from … --mail-to … --mail-body @-",
    ]);

    example("Script-side probe", &[
        r#"recon --script - <<< 'let r = smtp("smtp://localhost:1025/"); print(r.capabilities);'"#,
    ]);

    section("FILE TRANSFER (0.47.0)");

    example("FTP probe + list + retrieve", &[
        "recon ftp://ftp.gnu.org/gnu/                    # list directory",
        "recon ftp://ftp.gnu.org/gnu/ls.sig -o ls.sig    # retrieve file",
        "recon ftp://user:pass@host/dir/ -v",
    ]);
    note("Auth priority: URL userinfo > -u user:pass > anonymous. Default mode is passive (PASV / EPSV); use --ftp-active for servers that require it.");

    example("FTPS (explicit AUTH TLS)", &[
        "recon ftps://test.rebex.net/ -u demo:password",
        "recon ftps://self-signed.example/ -k",
    ]);

    example("SFTP via SSH", &[
        "recon sftp://demo:password@test.rebex.net/",
        "recon sftp://alice@host/reports/q4.pdf --ssh-key ~/.ssh/id_ed25519 -o q4.pdf",
    ]);

    example("TFTP (UDP read)", &[
        "recon tftp://router.local/config.cfg -o config.cfg",
        "recon tftp://server/firmware.bin --tftp-blksize 1428 -o fw.bin",
    ]);
    note("TFTP is UDP-based; servers reply from a new ephemeral port. Firewalls that restrict UDP by source port can drop the transfer after the first reply.");

    example("Gopher selector fetch", &[
        "recon gopher://gopher.floodgap.com/",
        "recon gopher://gopher.floodgap.com/0/gopher/proxy",
        "recon gophers://secure-gopher.example/",
    ]);

    section("WGET-STYLE BATCH FETCHING (0.67.0)");

    example("Polite delay between URLs in a batch", &[
        "recon --input-file urls.txt --wait 2",
        "recon --input-file urls.txt --wait 5 --spider",
    ]);

    example("Filename-suffix accept/reject filters", &[
        "recon --input-file urls.txt --accept jpg,png",
        "recon --input-file urls.txt --reject thumb,bak",
        "recon --input-file urls.txt --accept html,htm --reject draft",
    ]);

    example("Wget-style retry count (--tries overrides --retry)", &[
        "recon https://api.example.com/ --tries 5",
        "recon --input-file urls.txt --tries 3 --retry-delay 2",
        "recon --input-file urls.txt --retry 1 --tries 10  # --tries wins",
    ]);

    note("--wait is a fixed-seconds delay between URLs and overrides --rate when both are set. --tries means total attempts (wget semantics: tries = retries + 1) and overrides --retry. --accept/--reject match the URL's final path segment (case-insensitive); URLs with empty final segments fail --accept and pass --reject — same behaviour as wget. Short forms (-A/-R/-t/-w) are not provided because recon reserves single-letter flags for curl compatibility; the wget recursive cluster (-r/-l/-m/-p/-k) remains deferred (see OUT-OF-SCOPE.md).");

    section("PROXY EXTRAS + TLS TUNING + --config (0.66.0)");

    example("Flags from a config file (-K / --config)", &[
        "recon -K ~/.recon-ci.cfg https://example.com/",
        "echo '--insecure' > prod.cfg && recon -K prod.cfg -I https://corp.internal/",
        "cat api.cfg  # --user alice:secret\\n--retry 3\\nurl = https://api.example.com/",
    ]);

    example("Include another config file", &[
        "# base.cfg:  --user-agent 'recon-ci/1.0'",
        "# main.cfg:  @base.cfg",
        "#            --insecure",
        "recon -K main.cfg https://target/",
    ]);

    example("Proxy + TLS tuning (accepted; some plumb-through deferred)", &[
        "recon --ciphers 'TLS_AES_256_GCM_SHA384' https://example.com/   # accepted, not yet wired",
        "recon --pinnedpubkey 'sha256//BASE64HASH' https://example.com/  # accepted",
        "recon --preproxy http://proxy1 --proxy http://proxy2 https://example.com/",
    ]);

    note("0.66.0 ships -K/--config fully wired (config-file expansion before clap parses, with @include support, # comments, key=value or --flag value forms, cycle detection). The proxy + TLS-tuning flags are accepted at the CLI but most need rustls/reqwest primitives that aren't stable yet — they'll start taking effect when the plumbing lands, transparently to users already calling them.");

    section("PER-PROTOCOL KNOBS (0.65.0)");

    example("SSH host-key pinning + compression", &[
        "recon ssh://me@example.com --hostpubsha256 '3a:47:…'",
        "recon sftp://me@example.com/file --hostpubsha256 '3a47…' -o file",
        "recon scp://me@example.com/motd --compressed-ssh -o motd",
    ]);

    example("FTP filenames-only listing (--list-only)", &[
        "recon ftp://test.rebex.net/ --list-only",
        "recon ftp://example.com/dir/ --list-only -i",
    ]);

    example("FTP custom commands before listing (-Q / --quote)", &[
        "recon ftp://test.rebex.net/ -Q PWD -Q NOOP -v",
        "recon ftp://test.rebex.net/ --quote 'SITE HELP' --list-only",
    ]);
    note("--quote runs each command via suppaftp's custom_command before the listing step. FEAT can fail (multi-line 211 response not parsed by suppaftp); use single-line FTP verbs.");

    example("FTP passive-mode override for NAT'd servers (--ftp-skip-pasv-ip)", &[
        "recon ftp://nat-server.example.com/ --ftp-skip-pasv-ip",
    ]);

    example("SSH pubkey-only auth (--pubkey alias)", &[
        "recon ssh://example.com --pubkey ~/.ssh/id_ed25519.pub",
    ]);

    example("SMTP AUTH + IMAP login options", &[
        "recon smtp://mail.example.com --mail-from a@example.com --mail-to b@example.com \\\n      --mail-auth admin@example.com --smtp-auth alice:secret",
        "recon imaps://mail.example.com --login-options 'AUTH=PLAIN' --sasl-authzid admin",
    ]);

    note("SSH pinning + --compressed-ssh are fully wired to ssh2. FTP --list-only, --quote, and --ftp-skip-pasv-ip are wired in 0.71.0 (suppaftp). SMTP --mail-auth is accepted but emits a warning — lettre 0.11's high-level send API does not expose envelope parameters. IMAP / POP3 / Telnet plumb-through remains deferred.");

    section("RETRY + PROTO FILTER + BATCH (0.64.0)");

    example("Retry transient failures", &[
        "recon --retry 3 https://flaky.example.com/api/",
        "recon --retry 5 --retry-delay 2 https://api.example.com/",
        "recon --retry 3 --retry-all-errors https://api.example.com/",
        "recon --retry 10 --retry-connrefused --retry-max-time 60 https://starting.example.com/",
    ]);

    example("Rate-limit a batch", &[
        "recon --input-file urls.txt --rate 2/s -O",
        "recon --input-file urls.txt --rate 60/m --spider",
    ]);

    example("Protocol restriction", &[
        "recon --proto '=https' https://example.com/     # HTTPS only",
        "recon --proto '-ftp,-ftps' https://example.com/  # block FTP variants",
        "recon --proto-default https example.com          # scheme injection",
        "recon --proto-redir '=https' -L http://example.com/  # forbid downgrades",
    ]);

    example("Batch fetch with --input-file", &[
        "recon --input-file urls.txt --spider        # bulk link check",
        "recon --input-file urls.txt -O --rate 5/s   # polite download",
        "cat urls.txt | recon --input-file -",
    ]);

    example("Save every URL from a list to its own file (--remote-name-all)", &[
        "recon --input-file urls.txt --remote-name-all          # -O for every URL",
        "recon --input-file urls.txt --remote-name-all --rate 2/s  # rate-limited",
        "recon --input-file urls.txt --remote-name-all --output-dir ./downloads/",
    ]);
    note("--remote-name-all implies -O for every URL in the loop; --output-dir prefixes each filename.");

    example("Resume a download", &[
        "recon --continue -O https://example.com/big.iso       # wget-style auto-resume",
        "recon -C - -O https://example.com/big.iso              # curl-style auto",
        "recon -C 5242880 -O https://example.com/big.iso        # explicit 5 MiB offset",
    ]);

    section("FORMS + NETRC + HTTP VERSION (0.63.0)");

    example("Multipart form uploads (-F / --form)", &[
        "recon -F 'name=alice' -F 'bio=@bio.txt' https://api.example.com/profile -X POST",
        "recon -F 'avatar=@me.png;type=image/png;filename=user.png' https://api.example.com/upload -X POST",
        "recon -F 'data=<payload.json' https://api.example.com/submit -X POST     # file content, no filename",
        "cat secret | recon -F 'body=<-' https://api.example.com/post -X POST",
        "recon --form-string 'literal=@not-a-path' https://api.example.com/ -X POST",
    ]);

    example(".netrc-backed credentials", &[
        "recon -n https://private.example.com/api              # ~/.netrc machine entry",
        "recon --netrc-file ~/creds.netrc https://a.example.com",
        "recon --netrc-optional https://a.example.com          # silent fallback",
    ]);

    example("HTTP version pinning", &[
        "recon --http1.1 https://broken-h2.example.com/         # disable HTTP/2 upgrade",
        "recon --http2-prior-knowledge http://h2c.local:8080/   # h2c (cleartext HTTP/2)",
    ]);

    example("Upload variants", &[
        "cat body.json | recon -T - https://upload.example.com/    # stdin upload",
        "recon --crlf -T linefile.txt https://upload.example.com/   # LF→CRLF before send",
    ]);

    section("CURL EASY WINS (0.62.0)");

    example("Byte-range requests + size cap", &[
        "recon -r 0-1023 https://example.com/big.bin -o first-1kb.bin",
        "recon -r -512 https://example.com/big.bin -o last-512.bin",
        "recon --max-filesize 10M https://example.com/download.iso -o small.iso",
    ]);

    example("Conditional requests (If-Modified-Since, ETag)", &[
        "recon -z 'Wed, 21 Oct 2024 07:28:00 GMT' https://example.com/doc",
        "recon -z baseline.html https://example.com/page -o page.html",
        "recon --timestamping https://example.com/doc.txt -o doc.txt",
        "recon --etag-compare etag.txt --etag-save etag.txt https://api.example.com/v1/",
    ]);

    example("URL surface + security hardening", &[
        "recon --url-query 'q=rust' --url-query 'page=2' https://api.example.com/search",
        "recon --disallow-username-in-url https://user:pass@example.com/   # rejected",
    ]);

    example("Output extras", &[
        "recon https://example.com/ -o out.html --no-clobber",
        "recon https://example.com/big.bin -o big.bin --remove-on-error",
        "recon https://example.com/secret -o secret --create-file-mode 600",
        "recon -I -D headers.txt https://example.com/",
        "recon https://example.com/ --stderr /tmp/recon.log",
        "recon https://example.com/big.bin -o big.bin --no-progress-meter",
    ]);

    example("Conditional connection + TLS tuning", &[
        "recon --tcp-nodelay https://chat.example.com/long-poll",
        "recon --no-keepalive https://api.example.com/",
        "recon --tls-max 1.2 https://picky.example.com/                  # cap at TLS 1.2",
        "recon --ca-native --cacert corp-root.pem https://internal.corp/",
        "recon --capath /etc/corp/cas https://internal.corp/",
        "recon --connect-to api.example.com:443:127.0.0.1:8443 https://api.example.com/",
    ]);

    example("OAuth2, xattr, spider", &[
        "recon --oauth2-bearer $TOKEN https://api.example.com/me",
        "recon --xattr https://example.com/doc.pdf -o doc.pdf  # writes URL + MIME into xattrs",
        "recon --spider https://example.com/           # HEAD-based liveness check",
        "recon --spider -f https://api.example.com/health  # fail-fast CI check",
    ]);

    note("--spider issues a HEAD and prints `<STATUS> <URL>`; non-2xx → non-zero exit. --max-filesize checks Content-Length (streaming cap during-download requires a future release). --request-target is accepted at parse time but errors at execute — reqwest 0.12 has no hook for the request-line target.");

    section("RECON-OWN WAITING ITEMS (0.61.0)");

    example("Latin-American + Australian + Mexican tax IDs", &[
        "recon --checkdigit br_cpf '529.982.247-25'           # Brazilian CPF",
        "recon --checkdigit br_cnpj '11.444.777/0001-61'      # Brazilian CNPJ",
        "recon --checkdigit ar_cuit '20-12345678-9'           # Argentinian CUIT",
        "recon --checkdigit cl_rut '12.345.678-5'             # Chilean RUT",
        "recon --checkdigit au_abn '51 824 753 556'           # Australian ABN",
        "recon --checkdigit-create br_cpf '529982247'         # → 52998224725",
    ]);

    example("110+ year warning on Nordic + Bulgarian IDs", &[
        "recon --checkdigit cpr '0101891234'           # Danish CPR — warns if ≥110",
        "recon --checkdigit fnr '01018912345'          # Norwegian FNR",
        "recon --checkdigit henkilotunnus '010189-1234'  # Finnish HETU",
        "recon --checkdigit bg-egn '8901011234'        # Bulgarian EGN",
    ]);

    example("Human-readable text under 1D barcodes (--hrt)", &[
        "recon --encode ean13 --encode-format svg '4006381333931' -o product.svg   # HRT default-on for EAN/UPC",
        "recon --encode code128 --encode-format svg --hrt 'SHIP-4711' -o ship.svg  # explicit opt-in",
        "recon --encode ean13 --no-hrt '4006381333931' -o bare.svg                  # suppress HRT",
    ]);
    note("HRT is rendered for ASCII and SVG output in 0.61.0. PNG HRT is deferred (needs bundled font); PNG output ignores --hrt with no warning.");

    example("Scan every barcode in an image (--decode-all)", &[
        "recon --decode-all sheet.png          # one line per detected code",
        "cat photo.jpg | recon --decode-all -",
    ]);

    example("MQTT over TLS with mutual auth", &[
        "recon mqtts://broker.example.com -E client.pem --client-key client.key --mqtt-topic 'sensors/#'",
    ]);

    example("Bind outgoing socket to an interface by name", &[
        "recon --interface eth0 https://example.com/",
        "recon --interface en0 https://example.com/          # macOS",
        "recon --interface 10.0.0.5 https://example.com/     # IP literal still works",
    ]);

    section("FLAG LISTING (0.60.0)");

    example("Browse the full flag list", &[
        "recon --flags",
        "recon --flags | less",
        "recon --flags > flags.txt",
    ]);

    example("Search for a specific area", &[
        "recon --flags | grep -i cookie",
        "recon --flags | grep -E '^\\s*-[a-zA-Z],'       # just the flags with short keys",
    ]);

    note("--flags is curl's `--help all` layout: short key (or 4 spaces) + long name + <VALUE> + a short description capped at ~52 chars. Sorted alphabetically by long name. Use --flags as the quick-lookup index; follow up with `recon --help <topic>` for any feature area's long-form deep dive.");

    section("DOCUMENT CONVERSIONS (0.58.0)");

    example("Markdown → HTML (pure-Rust)", &[
        "recon --md-to-html README.md -o README.html",
        "recon --md-to-html README.md --toc --gfm -o README.html",
        "curl -s https://example.com/doc.md | recon --md-to-html - > doc.html",
    ]);

    example("Markdown → PDF (via agent-browser)", &[
        "recon --md-to-pdf CHANGELOG.md --toc --gfm --doc-title 'recon notes' -o changelog.pdf",
        "recon --md-to-pdf docs.md --toc --toc-depth 4 -o docs.pdf",
    ]);

    example("PDF document metadata (author / subject / keywords)", &[
        "recon --md-to-pdf report.md --doc-title 'Q1 Results' --doc-author 'Alice Smith' --doc-subject 'Quarterly report' --doc-keywords 'finance, Q1, 2026' -o report.pdf",
        "# Verify: pdfinfo report.pdf | grep -E '(Title|Author|Subject|Keywords)'",
    ]);

    example("HTML → PDF", &[
        "recon --html-to-pdf report.html -o report.pdf",
        "recon --html-to-pdf https://example.com/page.html -o page.pdf",
    ]);

    example("Custom CSS", &[
        "recon --md-to-pdf notes.md --toc --doc-css print.css -o notes.pdf",
        "recon --md-to-pdf notes.md --no-default-css --doc-css print.css -o notes.pdf",
    ]);

    example("Cover page + chapter breaks (book-style layout)", &[
        "recon --md-to-pdf book.md --toc --gfm --unsafe-html --page-break-on-h1 --doc-title Book -o book.pdf",
    ]);

    example("Raw-HTML passthrough — explicit page breaks", &[
        r#"printf '# A\n\nFirst.\n\n<div class="page-break"></div>\n\n# B\n\nSecond.\n' > tmp.md"#,
        "recon --md-to-pdf tmp.md --unsafe-html -o tmp.pdf",
    ]);

    note("HTML backend is `comrak` (CommonMark + GFM, pure Rust). PDF backend is `agent-browser pdf` (wraps Chrome's printToPDF, preserving anchor links so the TOC stays clickable). --md-to-html is pure-Rust / no external deps. --md-to-pdf and --html-to-pdf require agent-browser on PATH (`brew install agent-browser`). URL sources flow through the normal request pipeline and honor every HTTP flag.");

    section("PDF PAGE EXPORT");

    example("Render PDF page 1 to PNG (default)", &[
        "recon --export-pdf-page 1 docs/MANUAL.pdf",
        "# Writes ./page-1.png at 1024x1366 @ 2x device scale.",
    ]);

    example("Choose output path + format by extension", &[
        "recon --export-pdf-page 3 report.pdf -o cover.jpg",
        "recon --export-pdf-page 3 report.pdf -o cover.webp",
    ]);

    example("Larger image via viewport + device scale", &[
        "recon --export-pdf-page 1 docs/MANUAL.pdf --pdf-viewport 1920x2715 --pdf-scale 2 -o cover.png",
        "# Resulting image ≈ 3840 x 5430 px.",
    ]);

    example("JPEG quality tuning", &[
        "recon --export-pdf-page 1 docs/MANUAL.pdf -o cover.jpg --pdf-quality 75",
    ]);

    example("Pipe the image to another tool", &[
        "recon --export-pdf-page 1 docs/MANUAL.pdf --pdf-format png -o - | open -f -a Preview",
    ]);

    note("Renders via `pdftoppm` (poppler-utils). The viewport flag defines an upper-bound box; pdftoppm rasterizes the page preserving aspect at the highest DPI that still fits, then --pdf-scale multiplies for higher pixel density. PNG / JPEG come from pdftoppm directly; WEBP output is encoded in-process via the `webp` crate. Install via `brew install poppler` (macOS) or `apt install poppler-utils` (Debian/Ubuntu).");

    section("SCRIPT TCP / UDP SERVERS (0.57.0)");

    example("Run the shipped tcp echo server", &[
        "recon --script script/tcp-echo.rhai 127.0.0.1:9000",
        "printf 'hello\\n' | nc -w1 127.0.0.1 9000   # in another shell",
    ]);

    example("Run the shipped udp listener", &[
        "recon --script script/udp-listen.rhai 127.0.0.1:9001",
        "echo 'beacon' | nc -u -w1 127.0.0.1 9001   # in another shell",
    ]);

    note("Server bindings are script-only (no CLI flag surface). Pair with thread_spawn (0.56.0): accept on the main thread, hand each connection off to a spawned closure. tcp_accept + udp_recv_from both have timeout variants so the main loop can poll for shutdown signals. ICMP raw-socket primitives are deferred — use the existing ping() binding for basic reachability checks.");

    section("SCRIPT THREADING (0.56.0)");

    example("Spawn, channel, join", &[
        "recon --script script/thread.rhai",
    ]);

    example("Fan-out probes", &[
        r#"recon --script - <<< '
  let c = channel();
  let tx = c[0]; let rx = c[1];
  for u in ["https://a.com/", "https://b.com/", "https://c.com/"] {
      thread_spawn(|url| { send(tx, `${url} → ${http(url).status}`); }, u);
  }
  for i in 0..3 { print(recv(rx, 10000)); }
'"#,
    ]);

    note("`spawn` is reserved in Rhai; use `thread_spawn` (takes a FnPtr, optional arg or args array). `channel()` returns [sender, receiver]; clone the sender to fan out. `channel_bounded(n)` adds back-pressure. Worker threads build a fresh engine and inherit the parent's ScriptDefaults (so http/tcp/etc. probes see the same CLI-flag defaults). `sync`-feature cost: ~10-15% locking overhead on hot paths, irrelevant for diagnostic workloads.");

    section("SHELL SUBPROCESS (0.87.0)");

    example("Capture a command's output (blocking)", &[
        r#"recon --script - <<< 'let r = shell("git log --oneline -3"); print(r.stdout); print(`exit: ${r.exit_code}`);'"#,
    ]);
    note("Returns Map with `stdout`, `stderr`, `exit_code`, `success`. String input goes through `sh -c` (or `cmd /C` on Windows), so pipes / globs / && chains work. Pass an Array for direct argv form — `shell([\"echo\", \"$HOME\"])` prints the literal `$HOME` with no shell expansion.");

    example("Stream stdout+stderr line by line", &[
        r#"recon --script - <<< 'shell_stream("brew upgrade", |line| print(`[brew] ${line}`));'"#,
    ]);
    note("The callback fires once per merged stdout/stderr line as the child writes it. Returns the exit code. Built for live progress UIs and the upcoming TUI panes; ordering matches the OS-level interleave of the two pipes.");

    example("Opts map — cwd / env / timeout / merge_stderr", &[
        r#"recon --script - <<< 'let r = shell("cargo test", #{ cwd: "/path/to/repo", env: #{ RUST_LOG: "info" }, timeout_ms: 60000 });'"#,
    ]);
    note("All opts keys are optional: cwd, env (layered on parent), env_clear (drop parent env first), timeout_ms (kills the child + raises a catchable error), merge_stderr (blocking form only — streaming always merges).");

    example("Multi-step local update with `try`/`catch`", &[
        r#"recon --script - <<< 'for cmd in ["brew upgrade", "npm -g update"] { try { shell_stream(cmd, |line| print(line)); } catch (e) { print(`step failed: ${e}`); } }'"#,
    ]);
    note("Each shell call is independent — a non-zero exit code does NOT raise an error (the Map's `success` is just `false`). A timeout DOES raise an error, hence the try/catch. This is the run-and-watch pattern the TUI pane primitive (next section) sits on top of.");

    section("TUI DASHBOARD (0.87.0)");

    example("Two-pane update dashboard", &[
        "recon --script script/tui.rhai",
    ]);
    note("The shipped demo splits 70/30 vertically, streams two `for i; do echo; sleep; done` loops into the top pane, and logs progress in the bottom pane. Demonstrates the pattern: `shell_stream` callback writes to a pane handle.");

    example("Three horizontal panes (inline)", &[
        r#"recon --script - <<< '
  tui::run(|d| {
      let p = d.split_horizontal([33, 33, 34]);
      for i in 0..3 {
          p[i].title(`pane ${i}`);
          for j in 0..5 { p[i].println(`line ${j}`); }
      }
      sleep_ms(2000);
  });'"#,
    ]);
    note("`split_horizontal([percents])` lays panes left-to-right; `split_vertical([percents])` stacks them top-to-bottom. Last pane absorbs rounding so the screen is always fully covered. Pane methods: `println(line)`, `title(s)`, `clear()`.");

    section("DECODING / Aztec / PDF417 / MaxiCode (0.55.0)");

    example("Decode a QR / barcode from an image", &[
        "recon --decode ticket.png",
        "recon --decode product.jpg --decode-hints ean13",
    ]);

    example("Pipe an image from stdin", &[
        "cat code.png | recon --decode -",
        "curl -s https://example.com/qr.png | recon --decode -",
    ]);

    example("Round-trip encode → decode", &[
        "recon --encode qr -o /tmp/q.png 'hello' && recon --decode /tmp/q.png",
        "recon --encode aztec -o /tmp/a.png 'transit ticket' && recon --decode /tmp/a.png",
        "recon --encode pdf417 -o /tmp/p.png 'license data' && recon --decode /tmp/p.png",
    ]);

    example("Encode the new formats", &[
        "recon --encode aztec -o aztec.png 'compact 2D code'    # transit / shipping",
        "recon --encode pdf417 -o pdf.png  'stacked linear code' # IDs, shipping labels",
    ]);

    note("--decode reads PNG / JPEG / WebP / GIF / BMP. Output line: `<FORMAT>\\t<TEXT>`. Use --decode-hints to restrict scanning — speeds up and prevents prefix-match ambiguity. rxing (pure-Rust ZXing port) powers both decode and the new Aztec/PDF417/MaxiCode encoders.");

    section("CLIENT CERTIFICATES / mTLS (0.54.0)");

    example("Combined PEM (cert + key in one file)", &[
        "recon --client-cert ~/keys/bundle.pem https://mtls.example.com/",
        "recon -E ~/keys/bundle.pem https://mtls.example.com/  # curl-compatible -E",
    ]);

    example("Split cert and key", &[
        "recon -E client.crt --client-key client.key https://mtls.example.com/",
    ]);

    example("Encrypted PKCS#8 key — decrypt externally first", &[
        "openssl pkcs8 -in encrypted.key -out client.key",
        "recon -E client.crt --client-key client.key https://mtls.example.com/",
    ]);

    example("Script binding", &[
        r#"recon --script - <<< 'http("https://mtls.example.com/", #{ client_cert: "/path/bundle.pem" });'"#,
    ]);

    note("DER cert/key format is accepted at parse time but errors with a conversion recipe (`openssl x509 -inform DER -outform PEM …`) — rustls wants PEM. `--key-type ENG` has no rustls equivalent and errors immediately. Encrypted keys are detected and refused; decrypt externally then re-feed.");

    section("COMPARE (0.53.0)");

    example("Diff two local files", &[
        "recon --compare one.json two.json",
        "recon --compare --compare-context 5 one.json two.json",
    ]);

    example("Diff a live URL against a baseline", &[
        "recon --compare https://api.example.com/v1/status ./baseline.json",
        "recon --compare ./ref.html https://example.com/ -L",
    ]);

    example("Summary / side-by-side formats", &[
        "recon --compare a.txt b.txt --compare-format summary",
        "recon --compare a.txt b.txt --compare-format sxs",
    ]);

    example("Pipe stdin as one of the two sources", &[
        "curl -s https://a/ | recon --compare - ./baseline.json",
    ]);

    note("Exit code follows GNU diff: 0 = identical, 1 = differ, 2+ = source-load error. Binary content (NUL in first 8 KiB) skips the line diff and reports a byte-count delta instead. All HTTP flags (-H, -u, -L, -k, cookies, proxy) apply to URL sources.");

    section("SCRIPTING FILE I/O (0.53.0)");

    example("Streaming handle — read, seek, write, close", &[
        r#"recon --script - <<< 'let h = file_open("/tmp/log.bin", "rwc"); file_write(h, "abcdef"); file_seek(h, 0, "start"); let first3 = file_read(h, 3); file_close(h); print(first3.len());'"#,
    ]);

    example("Whole-file helpers", &[
        r#"recon --script - <<< 'file_write_all("/tmp/x", "hello"); file_append_all("/tmp/x", " world"); print(file_read("/tmp/x"));'"#,
        r#"recon --script - <<< 'if file_exists("/etc/hosts") { print(file_size("/etc/hosts")); }'"#,
    ]);

    note("file_open modes: `r` read, `w` write/truncate/create, `rw` read+write, `rwc` / `w+` read+write+create+truncate, `a` append, `ra` read+append. `file_seek(h, pos, whence)` takes whence = start|cur|end. Handles wrap Arc<Mutex<File>> so they survive thread boundaries.");

    example("Raw print without trailing newline", &[
        r#"recon --script - <<< 'print_raw("loading"); sleep_ms(200); print_raw(".\n");'"#,
        r#"recon --script - <<< 'eprint("warn: something"); eprint_raw("status: ")'"#,
    ]);

    note("`print_raw(s|blob)` writes to stdout without appending a newline and flushes. `eprint(s)` writes to stderr with newline; `eprint_raw` is the no-newline variant. Useful for progress bars, line protocols, or pre-framed byte output.");

    section("QR ERROR CORRECTION (0.53.0)");

    example("Tune QR redundancy", &[
        "recon --encode qr --qr-level L -o low.png 'small text'",
        "recon --encode qr --qr-level H -o durable.png 'long-lived sticker'",
    ]);

    note("Levels: L (~7%), M (~15%, default), Q (~25%), H (~30%). Higher levels recover more scratched / partly obscured codes but produce larger matrices. Only meaningful for --encode qr; ignored by other formats.");

    section("ENCODE HINTS — Aztec / PDF417 tuning (0.78.0)");

    example("Compact vs full Aztec via aztec-layers", &[
        "recon --encode aztec --encode-hints aztec-layers=-2 'compact aztec'",
        "recon --encode aztec --encode-hints aztec-layers=4  'full aztec'",
    ]);

    example("PDF417 error-correction level (0..8)", &[
        "recon --encode pdf417 --encode-hints eclevel=2 -o p2.svg 'low EC'",
        "recon --encode pdf417 --encode-hints eclevel=8 -o p8.svg 'max EC'",
    ]);

    example("ECI / charset override (rxing CharacterSet hint)", &[
        "recon --encode aztec  --encode-hints charset=Shift_JIS '日本'",
        "recon --encode pdf417 --encode-hints charset=UTF-8     'unicode payload'",
    ]);

    note("--encode-hints is repeatable. Applies only to aztec / pdf417 — recon's other encoders (qr, datamatrix, code128, code39, ean13, upca) use crates without a hint API, so passing hints with them errors. Unknown keys also error so typos fail loud. See `recon --help encoding` for the full key list.");

    section("HSTS (0.52.0)");

    example("Populate cache from an https:// response", &[
        "recon --hsts ~/.recon/hsts.txt https://www.cloudflare.com/",
        "cat ~/.recon/hsts.txt  # curl-compatible TSV",
    ]);

    example("Auto-upgrade http:// using the cache", &[
        "recon --hsts ~/.recon/hsts.txt http://www.cloudflare.com/",
    ]);
    note("On request, an http:// URL to a host with a non-expired cache entry is upgraded to https:// before sending. On response, any Strict-Transport-Security header updates the cache (max-age=0 removes). File format matches curl's --hsts. Missing files are silently treated as empty.");

    example("Shared cache across scripts + CLI", &[
        r#"recon --script - <<< 'http("http://example.com/", #{ hsts: "/tmp/h.txt" });'"#,
    ]);

    section("UNIX SOCKETS (0.51.0)");

    example("Docker API over /var/run/docker.sock", &[
        "recon --unix-socket /var/run/docker.sock http://localhost/_ping",
        "recon --unix-socket /var/run/docker.sock -p http://localhost/v1.40/version",
        "recon --unix-socket /var/run/docker.sock http://localhost/v1.40/containers/json",
    ]);

    example("Path-only target (Host defaults to localhost)", &[
        "recon --unix-socket /var/run/docker.sock /v1.40/info",
    ]);

    example("POST to a systemd-activated service", &[
        r#"recon --unix-socket /run/my-service.sock \
      -X POST --json '{"hello":"world"}' \
      http://svc/submit"#,
    ]);
    note("HTTP/1.1 over UDS is hand-rolled — no HTTP/2, no TLS, no redirects, no chunked decoding. Sufficient for Docker / systemd / kubelet diagnostics. Scripts pass the socket as `#{ unix_socket: \"/path\" }` on the http() opts map.");

    section("PROXY (0.50.0)");

    example("Route through an HTTP proxy", &[
        "recon --proxy http://proxy.corp:3128 https://example.com/",
        "recon -x proxy.corp:3128 https://example.com/  # scheme defaults to http",
    ]);

    example("Authenticated proxy", &[
        "recon --proxy http://proxy.corp:3128 --proxy-user alice:secret https://example.com/",
    ]);

    example("TLS-to-proxy (https:// proxy URL)", &[
        "recon --proxy https://secure-proxy.corp:8443 https://example.com/",
        "recon --proxy https://self-signed-proxy.corp --proxy-insecure https://example.com/",
        "recon --proxy https://corp-proxy --proxy-cacert corp-ca.pem https://example.com/",
    ]);

    example("SOCKS5 tunneling (e.g. Tor)", &[
        "recon --proxy socks5h://127.0.0.1:9050 https://example.com/",
        "recon --proxy socks5://bastion:1080 https://internal.example/",
    ]);

    example("Bypass lists and env-var precedence", &[
        "recon --proxy http://corp --noproxy '.internal,localhost,127.0.0.1' https://example.com/",
        "HTTPS_PROXY=http://proxy.corp:3128 recon https://example.com/",
        "NO_PROXY='.internal' HTTPS_PROXY=http://proxy.corp recon https://foo.internal/",
    ]);
    note("Precedence: --proxy flag > $HTTPS_PROXY (for https://) / $HTTP_PROXY (for http://) > $ALL_PROXY. --noproxy beats $NO_PROXY. `*` in the bypass list means bypass all. SOCKS5 requires the `socks` feature baked in (on by default).");

    example("Proxy mTLS passphrase (--proxy-pass, deferred)", &[
        "recon --proxy https://corp-proxy --proxy-pass s3cr3t https://example.com/",
    ]);
    note("--proxy-pass is accepted for curl parity but has no effect in 0.73.0 — reqwest 0.12 does not expose a passphrase API for proxy mTLS. A runtime warning is emitted. See OUT-OF-SCOPE.md.");

    section("IPFS / IPNS (0.49.0)");

    example("Fetch content by CID via the default gateway (ipfs.io)", &[
        "recon ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        "recon ipfs://bafy... -o out.bin",
    ]);

    example("Resolve an IPNS DNSLink", &[
        "recon ipns://ipfs.tech/",
    ]);

    example("Route through a local Kubo / IPFS-Desktop node", &[
        "recon ipfs://bafy... --ipfs-gateway http://127.0.0.1:8080",
        "RECON_IPFS_GATEWAY=https://cloudflare-ipfs.com recon ipfs://bafy...",
    ]);
    note("ipfs:// and ipns:// URLs are rewritten to <gateway>/ipfs/CID or <gateway>/ipns/NAME and dispatched through the existing HTTP path, so every HTTP flag (-H, -o, -k, --compressed, --output-charset, etc.) applies verbatim. No native IPFS-protocol client in the binary.");

    section("MAIL RETRIEVAL (0.48.0)");

    example("POP3 capability probe (no auth)", &[
        "recon pop3s://pop.gmail.com/",
        "recon pop3s://pop.example.com/ -v",
    ]);

    example("POP3 auth + mailbox stats", &[
        "recon pop3s://me%40gmail.com:apppass@pop.gmail.com/",
    ]);

    example("POP3 retrieve message 3", &[
        "recon pop3s://me:pass@mail.example.com/3",
    ]);

    example("POP3 with STARTTLS (STLS command)", &[
        "recon pop3://me:pass@mail.example.com/ --stls",
    ]);

    example("IMAP capability probe", &[
        "recon imaps://imap.gmail.com/",
    ]);

    example("IMAP EXAMINE INBOX + fetch UID 42 (without \\Seen)", &[
        "recon imaps://me:pass@imap.example.com/INBOX",
        "recon imaps://me:pass@imap.example.com/INBOX;UID=42 --imap-peek",
    ]);
    note("Path grammar mirrors curl: empty path -> probe; mailbox -> EXAMINE; `;UID=N` suffix -> FETCH. --imap-peek uses BODY.PEEK[] so the server doesn't set \\Seen.");

    section("MQTT (0.22.0)");

    example("Probe a broker (default mode)", &[
        "recon mqtt://broker.example.com:1883/",
        "recon mqtts://broker.example.com:8883/",
    ]);
    note("Connects, dumps CONNACK details, disconnects. Default MQTT version is 5.0 — use --mqtt-version 3 for 3.1.1.");

    example("Probe with JSON output for scripting", &[
        "recon mqtt://broker/ --mqtt-json | jq .connect_reason",
    ]);

    example("Publish a message", &[
        r#"recon mqtt://broker/devices/fan/state -d "on""#,
        r#"recon mqtt://broker/devices/fan/state -d "on" --qos 1 --retain"#,
        "recon mqtt://broker/logs -d @event.json",
    ]);

    example("Subscribe to topic filters", &[
        r#"recon mqtt://broker/ --subscribe "devices/+/state""#,
        r#"recon mqtt://broker/ --subscribe "devices/#" --count 10"#,
        r#"recon mqtt://broker/ --subscribe "a/#" --subscribe "b/#" -v"#,
    ]);
    note("Default output is payload-only. Use -v to prefix the topic; --mqtt-json for NDJSON.");

    example("Auth and TLS", &[
        "recon mqtts://user:pass@broker:8883/",
        "recon mqtts://broker:8883/ -u alice:s3cr3t",
        "recon mqtts://self-signed.broker/ -k",
    ]);

    example("MQTT 5 user-properties + content-type (0.45.0)", &[
        r#"recon mqtt://broker/events -d '{"ok":true}' \
      --user-property env=prod \
      --user-property caller=recon \
      --content-type application/json"#,
    ]);

    example("Request/response pattern (0.45.0)", &[
        r#"recon mqtt://broker/service/req -d '{"action":"ping"}' \
      --response-topic service/rsp/alice \
      --correlation-data 'corr-abc-123' \
      --content-type application/json"#,
    ]);

    example("Last-will message on unexpected disconnect (0.45.0)", &[
        r#"recon mqtt://broker/status -d 'online' \
      --will-topic status/myclient \
      --will-payload 'offline' \
      --will-retain --will-qos 1"#,
    ]);

    example("Resume a persistent session (0.45.0)", &[
        r#"recon mqtt://broker/ --subscribe 'events/#' \
      --session-expiry 3600 \
      --clean-start=false \
      --client-id myclient-1 --count 10"#,
    ]);
    note("0.45.0 added MQTT 5 power-user properties: --user-property, --will-*, --session-expiry, --clean-start, --content-type, --response-topic, --correlation-data, --auth-method, --auth-data. All silently ignored on --mqtt-version 3.");

    section("PROTOCOL URL SCHEMES");

    example("Read a local file (file://)", &[
        "recon file:///etc/hosts",
        "recon file:///tmp/data.bin -o /tmp/copy.bin",
    ]);
    note("Cat-like semantics; empty or 'localhost' host only. Other hosts error out.");

    example("Aliases for existing flags", &[
        "recon whois://example.com",
        "recon dns://example.com/MX",
        "recon dig://example.com/A,AAAA",
        "recon drill://example.com",
        "recon tls://github.com:443/",
        "recon ping://8.8.8.8",
        "recon traceroute://google.com",
    ]);
    note("whois://, dns://dig://drill://, tls://, ping://, traceroute:// wrap the corresponding flags. dns://HOST/TYPE path is a record-type shorthand; --dns-type overrides when both are given.");

    example("TCP connect probe", &[
        "recon tcp://github.com:443/",
        "recon tcp://localhost:22/",
    ]);
    note("Reports connect latency, resolved address, local address. Exit 0 on connect, 7 refused, 28 timed out.");

    example("UDP send-and-wait probe", &[
        "recon udp://8.8.8.8:53/",
        "recon udp://8.8.8.8:53/ --wait-time 2",
        r#"recon udp://example.com:1234/ -d "hello""#,
    ]);
    note("UDP silence is ambiguous — exit 0 regardless of response unless send fails.");

    example("NTP (SNTPv4) server probe", &[
        "recon ntp://pool.ntp.org/",
        "recon ntp://time.google.com/",
    ]);
    note("Reports stratum, reference identifier, offset from local clock, round-trip delay.");

    example("DICT (RFC 2229, curl URL grammar)", &[
        "recon dict://dict.dict.org/",
        "recon dict://dict.dict.org/d:recon",
        "recon dict://dict.dict.org/d:hello:wn",
        "recon dict://dict.dict.org/m:recon",
        "recon dict://dict.dict.org/show:databases",
    ]);
    note("Bare URL runs SHOW SERVER + SHOW DATABASES + SHOW STRATEGIES. Otherwise: /d:WORD[:DB[:STRAT]] defines, /m:WORD[…] matches, /show:… introspects.");

    example("Redis (RESP2)", &[
        "recon redis://localhost/",
        "recon redis://:password@localhost/",
        r#"recon redis://localhost/ -d "SET foo bar""#,
        r#"recon redis://localhost/ -d "GET foo""#,
        r#"recon redis://localhost/ -d "CLIENT LIST""#,
        r#"recon redis://localhost/ -d "SET key \"hello world\"""#,
    ]);
    note("Without -d: connect + PING. With -d: shell-split the string (whitespace, \"…\", '…', \\-escapes) and send as RESP array. Password in userinfo sends AUTH first.");

    example("Memcached (text protocol)", &[
        "recon memcached://localhost/",
        "recon memcached://localhost/stats",
    ]);
    note("Default: issues `version` and reports the reply + roundtrip. Path `/stats` also dumps `stats` output.");

    example("WebSocket handshake + ping/pong", &[
        "recon ws://127.0.0.1:9876/",
        "recon wss://ws.postman-echo.com/raw",
    ]);
    note("TCP connect → HTTP Upgrade → Ping frame with 8-byte nonce → wait for matching Pong → close. Reports handshake info and Ping RTT. wss:// honours -k.");

    example("LDAP anonymous RootDSE", &[
        "recon ldap://ldap.forumsys.com:389/",
        "recon ldaps://ldap.example.com/",
    ]);
    note("Anonymous simple bind then searches RootDSE (scope=base, objectClass=*). Reports namingContexts, supportedLDAPVersion, vendorName/Version, supportedSASLMechanisms.");

    example("RTSP OPTIONS (RFC 2326)", &[
        "recon rtsp://example.com:554/stream",
        "recon rtsps://example.com/stream",
        "recon rtsps://self-signed.example.com/stream -k",
    ]);
    note("Sends OPTIONS, prints status line + response headers (Public: listed methods, Server:). rtsps:// uses TLS on port 322 and honours -k.");

    section("BROWSER AUTOMATION (agent-browser)");

    example("One-shot screenshot via the CLI flag", &[
        "recon --browser-screenshot https://example.com -o /tmp/shot.png",
    ]);
    note("Requires `agent-browser` installed on PATH (`brew install agent-browser` / `npm install -g agent-browser`). The flag opens the URL, captures a screenshot, closes the browser.");

    example("Scripted browser flow with availability guard", &[
        r#"cat > /tmp/title.rhai <<'EOF'
if !agentBrowser::available {
    print("install: brew install agent-browser");
    return 2;
}
agentBrowser::open("https://example.com");
let r = agentBrowser::get("title");
agentBrowser::close();
print(r.title);
return 0;
EOF
recon --script /tmp/title.rhai"#,
    ]);
    note("The `agentBrowser` static module is always present in scripts. Check `agentBrowser::available` before calling any method; functions throw a clear Rhai error when the binary isn't installed.");

    example("Shipped example scripts (copy or run in place)", &[
        "ls script/                              # bundled with the repo",
        "recon --script script/agent-browser-title.rhai https://example.com",
        "cp script/*.rhai ~/.recon/script/       # then: recon --script agent-browser-title URL",
    ]);

    example("agent-browser global options (ignore-https-errors, user-agent, headers)", &[
        r#"agentBrowser::set_default_options(#{ ignore_https_errors: true, user_agent: "MyBot/1.0" })"#,
        r#"agentBrowser::open("https://self-signed.example")"#,
        r#"agentBrowser::open("https://api.example", #{ headers: #{ Authorization: "Bearer x" } })"#,
    ]);
    note("Defaults persist for the script's lifetime. Use agentBrowser::clear_default_options() to reset. Per-call opts override defaults for that call only. See script/agent-browser-options.rhai for the full demo.");

    section("SCRIPTING (--script)");

    example("Run a Rhai script by path or by bare name", &[
        "recon --script workflow.rhai                 # literal path",
        "recon --script health                        # -> ~/.recon/script/health.rhai",
        "recon --init                                 # bootstrap ~/.recon/ first time",
    ]);
    note("Script's `return N` (integer) becomes the process exit code. Uncaught exceptions exit 1 (or 7/28/67 if a network error carries a ProtocolExitCode). --script is mutually exclusive with a positional URL. Bare names resolve against ~/.recon/script/ and auto-append .rhai.");

    example("Minimal health check (bruno-style)", &[
        r#"cat > /tmp/health.rhai <<'EOF'
let r = https("https://example.com");
if r.status != 200 {
    print(`got ${r.status}, expected 200`);
    return 1;
}
return 0;
EOF
recon --script /tmp/health.rhai"#,
    ]);

    example("Chain DNS → TCP → HTTP", &[
        r#"cat > /tmp/chain.rhai <<'EOF'
let d = dns("example.com", ["A"]);
assert(d.records.A.len() > 0, "no A records");
let t = tcp("tcp://example.com:443");
assert(t.ok, "tcp failed");
let r = https("https://example.com");
print(`${d.records.A.len()} IPs, tcp ok, http ${r.status}`);
return 0;
EOF
recon --script /tmp/chain.rhai"#,
    ]);

    example("Poll a status endpoint until ready", &[
        r#"cat > /tmp/poll.rhai <<'EOF'
for i in 0..10 {
    let r = http("http://localhost:8080/health");
    if r.status == 200 { return 0; }
    sleep_ms(1000);
}
return 1;
EOF
recon --script /tmp/poll.rhai"#,
    ]);

    example("Inspect a cert and branch on days_remaining", &[
        r#"cat > /tmp/cert.rhai <<'EOF'
let c = tls("example.com", 443);
if c.days_remaining < 30 {
    print(`CERT EXPIRES IN ${c.days_remaining} DAYS`);
    return 2;
}
return 0;
EOF
recon --script /tmp/cert.rhai"#,
    ]);

    example("Parameterise via args[1..] and flags", &[
        r#"cat > /tmp/check.rhai <<'EOF'
if args.len() < 2 {
    print(`usage: recon --script ${args[0]} HOST`);
    return 2;
}
let host = args[1];
let r = https(`https://${host}`);
if flags.verbose > 0 {
    print(`${r.duration_ms}ms status=${r.status}`);
}
return if r.status == 200 { 0 } else { 1 };
EOF
recon --script /tmp/check.rhai example.com
recon -v --script /tmp/check.rhai example.com    # flags.verbose = 1"#,
    ]);
    note("args[0] is the script name as typed (bare name with global-dir fallback, or literal path). args[1..] are positional args after the script path. `flags` mirrors the ScriptDefaults set (insecure, connect_timeout, headers, user_agent, ...) plus data + output; unset optionals are `()`.");

    example("Executable shebang script (0.68.0)", &[
        r#"cat > ~/bin/health <<'EOF'
#!/usr/bin/env -S recon --script
let host = if args.len() > 1 { args[1] } else { "example.com" };
let r = https(`https://${host}`);
print(`${r.status} ${host} (${r.duration_ms}ms)`);
return if r.status == 200 { 0 } else { 1 };
EOF
chmod +x ~/bin/health
health example.com          # run directly — no `recon --script` needed
health api.example.com -k   # extra args land in args[2], flags.insecure etc."#,
    ]);
    note("Shebang line: #!/usr/bin/env -S recon --script  (-S splits arguments; required on macOS and modern Linux). The #! is silently converted to a // Rhai comment before compilation, preserving line numbers in error messages. Trailing arguments after the script name land in args[1..] exactly as with --script.");

    example("SQLite: query the cookie jar", &[
        r#"cat > /tmp/jar.rhai <<'EOF'
let db = sqlite("cookiejar");          // ~/.recon/jars/default.db
let soon = now() + 86400;              // cookies expiring in next 24h
let rows = db.query(
    "SELECT domain, name, expires FROM cookies
     WHERE expires IS NOT NULL AND expires < ?
     ORDER BY expires",
    [soon]
);
for r in rows { print(`${r.domain} ${r.name} expires ${r.expires}`); }
return 0;
EOF
recon --script /tmp/jar.rhai"#,
    ]);
    note("sqlite(\"cookiejar:NAME\") targets ~/.recon/jars/NAME.db. The default mode is \"rw\" — scripts can INSERT/UPDATE the jar. Use \"ro\" for a read-only handle.");

    example("SQLite: arbitrary file with create-on-missing", &[
        r#"cat > /tmp/scratch.rhai <<'EOF'
let db = sqlite("/tmp/scratch.db", "rwc");
db.exec("CREATE TABLE IF NOT EXISTS seen (url TEXT PRIMARY KEY, ts INTEGER)");
let url = if args.len() > 1 { args[1] } else { "https://example.com" };
db.exec("INSERT OR REPLACE INTO seen VALUES (?, ?)", [url, now()]);
print(db.query_value("SELECT COUNT(*) FROM seen", []));
return 0;
EOF
recon --script /tmp/scratch.rhai https://example.com"#,
    ]);

    example("SQLite: in-memory scratch database", &[
        r#"cat > /tmp/mem.rhai <<'EOF'
let db = sqlite(":memory:");
db.exec("CREATE TABLE t (host TEXT, ms INTEGER)");
for host in ["example.com", "example.org", "example.net"] {
    let r = tcp(`tcp://${host}:443`);
    db.exec("INSERT INTO t VALUES (?, ?)", [host, r.duration_ms]);
}
let fastest = db.query_one("SELECT host, ms FROM t ORDER BY ms LIMIT 1", []);
print(`fastest: ${fastest.host} at ${fastest.ms}ms`);
return 0;
EOF
recon --script /tmp/mem.rhai"#,
    ]);
    note("`:memory:` creates an ephemeral database that disappears when the script ends. Useful for aggregating probe results across multiple requests.");

    example("Share helpers via `import` (falls back to ~/.recon/script/)", &[
        r#"cat > ~/.recon/script/assertions.rhai <<'EOF'
fn expect_200(r) { assert(r.status == 200, `expected 200, got ${r.status}`); r }
fn expect_json(r) { assert(r.headers["content-type"][0].contains("json"), "expected JSON"); r }
EOF
cat > /tmp/consumer.rhai <<'EOF'
import "assertions" as a;
let r = a::expect_json(a::expect_200(https("https://httpbin.org/json")));
print(`${r.body.len()} bytes`);
return 0;
EOF
recon --script /tmp/consumer.rhai"#,
    ]);
    note("`import \"name\"` first looks for `name.rhai` next to the running script; if not found, falls back to ~/.recon/script/name.rhai. Scripts in the global dir import sibling modules via the same first resolver.");

    example("Compress and decompress from a script", &[
        r#"cat > /tmp/compress.rhai <<'EOF'
// Download + gzip the body + stash to disk.
let r = https("https://example.com");
let gz = compression::compress("gzip", r.body.to_blob());
// (file_write isn't shipped; print sizes instead.)
print(`${r.body.len()}B -> ${gz.len()}B`);
// Round-trip sanity.
let back = compression::decompress(gz);   // auto-detect
assert(back.len() == r.body.len(), "mismatch");
return 0;
EOF
recon --script /tmp/compress.rhai"#,
    ]);

    example("Bundle a directory into a tarball from a script", &[
        r#"cat > /tmp/archive.rhai <<'EOF'
if args.len() < 2 {
    print(`usage: ${args[0]} DEST SOURCE [SOURCE...]`);
    return 1;
}
let dest = args[1];
let sources = [];
for i in 2..args.len() { sources.push(args[i]); }
let n = archive::create(dest, sources);
print(`${n} files archived to ${dest} (${archive::detect(dest)})`);
return 0;
EOF
recon --script /tmp/archive.rhai /tmp/bundle.tar.gz /tmp/src /tmp/data"#,
    ]);

    example("Hash a response body + pretty-print a signed payload", &[
        r#"cat > /tmp/sign.rhai <<'EOF'
let r = https("https://example.com");
let body_sha = sha256(r.body);
let payload = #{ url: r.final_url, sha256: body_sha, status: r.status };
print(json_stringify(payload, true));
return 0;
EOF
recon --script /tmp/sign.rhai"#,
    ]);

    example("Stateful browser() with sticky cookies + headers", &[
        r#"cat > /tmp/browser.rhai <<'EOF'
let b = browser();
b.set_user_agent("recon-demo/1.0");
b.set_header("X-API-Key", env("API_KEY", ""));
b.get("https://httpbin.org/cookies/set/session/abc");   // collects Set-Cookie
let r = b.get("https://httpbin.org/cookies");           // replays it
print(r.body);
return 0;
EOF
recon --script /tmp/browser.rhai"#,
    ]);
    note("browser() keeps cookies, headers, user-agent, redirect policy, and timeouts across calls. Default jar is an ephemeral temp file; `b.use_persistent_session(\"name\")` swaps in ~/.recon/jars/name.db. POST/PUT/PATCH accept String, Blob, or Map — maps auto-serialise to JSON. See `recon --help browser`.");

    example("Multiple independent browsers in one script", &[
        r#"cat > /tmp/multi.rhai <<'EOF'
let a = browser();  a.set_user_agent("scraper-a");
let b = browser();  b.set_user_agent("scraper-b");
let ra = a.get("https://httpbin.org/user-agent");
let rb = b.get("https://httpbin.org/user-agent");
print(`a: ${ra.body}`);
print(`b: ${rb.body}`);
return 0;
EOF
recon --script /tmp/multi.rhai"#,
    ]);

    example("Layered .env loading (common + per-script) (0.76.0)", &[
        r#"cat > /tmp/dotenv.rhai <<'EOF'
// One common .env shared by every script in a directory, plus a
// per-script override. The second load wins, so script-specific
// values overlay the common defaults.
file_write_all("/tmp/recon.env", "API_HOST=api.example.com\nLOG_LEVEL=info\n");
file_write_all("/tmp/recon.env.greet", "LOG_LEVEL=debug\nGREETING=hello\n");
load_dotenv("/tmp/recon.env");
load_dotenv("/tmp/recon.env.greet");
print(`API_HOST=${env("API_HOST")}`);
print(`LOG_LEVEL=${env("LOG_LEVEL")}`);   // debug — specific wins
print(`GREETING=${env("GREETING")}`);
// Pass `false` to leave any pre-existing env (e.g. shell exports)
// in place: load_dotenv("/tmp/recon.env", false);
return 0;
EOF
recon --script /tmp/dotenv.rhai"#,
    ]);
    note("load_dotenv(path) overrides existing values by default — that's what makes `common.env, then .env.<scriptname>` layer correctly. Pass `false` to leave pre-existing vars (shell-env wins). env_all() returns the whole environment as a Map. Aliases: loadDotEnv, envAll. std::env::set_var is technically unsound under concurrent reads, so call load_dotenv at the top of the script, before any thread_spawn.");

    example("Sibling .env via script_dir / script_name (resolved-path constants) (0.76.1)", &[
        r#"cat > /tmp/myscript.rhai <<'EOF'
// `script_dir` is the resolved absolute parent of the running script,
// `script_name` is its file stem (e.g. "myscript"). Combine with .env
// names to load files siblings to the script, independent of CWD.
load_dotenv(script_dir + "/.env");                       // shared
load_dotenv(script_dir + "/.env." + script_name);        // per-script overlay
print(`API_HOST=${env("API_HOST", "(unset)")}`);
return 0;
EOF
echo 'API_HOST=staging.example.com' > /tmp/.env
echo 'API_HOST=myscript-prod.example.com' > /tmp/.env.myscript
recon --script /tmp/myscript.rhai      # API_HOST = myscript-prod.example.com"#,
    ]);
    note("`script_path`, `script_dir`, and `script_name` are read-only String constants pushed into the script's Scope alongside `args` and `flags`. Use them when scripts need to find sibling files (.env, fixtures, helper modules) without depending on CWD.");

    example("Browse per-module example scripts (one .rhai per binding)", &[
        "ls script/                                   # 27 shipped examples",
        "recon --script script/http.rhai https://example.com",
        "recon --script script/jwt.rhai",
        "recon --script script/encrypt.rhai           # age round-trip",
        "recon --script script/email.rhai example.com # SPF + DMARC snapshot",
        "cp script/*.rhai ~/.recon/script/            # install into global dir",
    ]);
    note("The repo's script/ directory ships one example per binding module (http, dns, tls, redis, ws, ldap, encode, encrypt, checkdigit, sample, jwt, email, netstatus, sqlite, archive, compression, hash, agent-browser, …). Each script is ~15 lines, documents its args at the top, and exits 0 on success (non-zero when an upstream precondition is missing).");

    note("Available functions: http/https/request, browser(), tcp, ping, dns, tls, ntp, redis, ws/wss, dict, ldap/ldaps, whois, memcached, rtsp/rtsps, mqtt_pub/mqtt_sub, file_read. Module bindings: compression::, archive::, sqlite(), encode::, encrypt::, checkdigit::, sample::, jwt::, email::, netstatus::, text::, agentBrowser::. Hashes: md5, sha1, sha256, sha384, sha512, sha3_256, sha3_512, blake3, crc32, plus hash(algo, x [, \"hex\"|\"base64\"]). Helpers: print, sleep_ms, env, env_all, load_dotenv, now, now_ms, assert, json_parse, json_stringify. See `recon --help script`.");

    section("TEXT ENCODING (charsets)");

    example("Convert a Latin-1 / Windows-1252 response to UTF-8", &[
        "recon --to-utf8 https://legacy.example.com/api",
        "recon --output-charset utf-8 https://legacy.example.com/api",
    ]);
    note("Source charset detection: `--source-charset NAME` → Content-Type `charset=` → BOM sniff → chardetng heuristic → windows-1252 fallback. Unmappable characters are substituted with `?` and a warning is written to stderr (suppress with `-s`).");

    example("Prettify a Shift_JIS page (forces UTF-8 before prettify)", &[
        "recon -p --output-charset utf-8 https://legacy.jp/index.html",
    ]);

    example("POST UTF-8 input to a Perl / legacy service that expects ISO-8859-1", &[
        r#"recon -X POST \
      -H 'Content-Type: application/x-www-form-urlencoded; charset=iso-8859-1' \
      -d 'name=Jörg' \
      https://perl.example.com/submit"#,
        r#"recon --request-charset iso-8859-1 -d 'name=Jörg' https://perl.example.com/submit"#,
    ]);
    note("The request body is read as UTF-8 from the shell and transcoded before sending whenever an explicit Content-Type charset is set (or `--request-charset` is given). `--request-charset-passthrough` skips this — handy when sending a pre-encoded file.");

    example("Standalone file / stdin conversion (iconv-compatible)", &[
        "recon --iconv iso-8859-1:utf-8 input.txt -o output.txt",
        "cat legacy.txt | recon --iconv :utf-8 > utf8.txt        # auto-detect source",
        "recon --list-charsets                                     # see supported labels",
    ]);

    example("Script-side charset work (text::*)", &[
        r#"cat > /tmp/decode.rhai <<'EOF'
let r = http("https://legacy.example.com/");
// r.charset is the declared/sniffed charset; r.body_bytes is the raw Blob.
let utf8 = text::decode(r.body_bytes, r.charset ?? "windows-1252");
print(utf8);
return 0;
EOF
recon --script /tmp/decode.rhai"#,
    ]);
    note("Every http() / browser() response map now includes `body_bytes` (raw Blob) and `charset` (String or `()` when undecidable). Scripts combine these with `text::decode()` / `text::transcode()` for precise control. See `recon --help charset`.");

    section("STRING HELPERS (script + REPL)");

    example("Trim whitespace or a custom mask", &[
        r#"recon --script - <<< 'print(trim("  hi  ")); print(ltrim("...path", ".")); print(rtrim("file.log", ".log"));'"#,
    ]);
    note("trim / ltrim / rtrim default to whitespace; pass a second argument to strip any character in that mask (PHP semantics). The existing Rhai `.trim()` String method keeps working alongside these free functions.");

    example("Reverse, strip HTML, switch newlines for <br>", &[
        r#"recon --script - <<< 'print(strrev("café")); print(strip_html("<p>plain <b>text</b></p>")); print(nl2br("a\nb"));'"#,
    ]);
    note("strrev reverses by Unicode codepoints so accented letters and emoji stay intact. strip_html respects quoted attributes; nl2br ↔ br2nl round-trips cleanly because br2nl preserves the trailing EOL on the original tag.");

    example("Join an Array (method or free-function form)", &[
        r#"recon --script - <<< 'print(["a", "b", "c"].join(", ")); print(join([1, "two", 3.5], "-"));'"#,
    ]);
    note("Rhai 1.24's BasicArrayPackage doesn't ship Array.join — recon registers it so `arr.join(sep)` and `join(arr, sep)` both work. Non-string elements stringify via Dynamic::to_string.");

    example("Regex match + replace (PHP-style delimiters optional)", &[
        r#"recon --script - <<< 'print(preg_match("/^Host:\\s*(.+)$/i", "Host: example.com")); print(preg_replace("\\s+", "-", "a  b   c"));'"#,
    ]);
    note("preg_match returns an Array: index 0 is the whole match, 1+ are captures. Empty array if no match. `/pat/i` form supports the i / m / s / x flags.");

    example("printf / sprintf — pass an Array for multi-arg formats", &[
        r#"recon --script - <<< 'printf("%-10s %5d\n", ["alpha", 42]); print(sprintf("hex=%#x", 255));'"#,
    ]);
    note("Specifiers: d i u o x X b f e E g G s c %%. Flags: - (left-align), 0 (zero-pad), + (force sign), space (space-sign), # (alt form). Rhai has no variadic concept, so multi-arg formats pass `[a, b, c]`.");

    example("URL encode / decode (RFC 3986)", &[
        r#"recon --script - <<< 'print(urlencode("hello world & friends?")); print(urldecode("name%3DJ%C3%B6rg"));'"#,
    ]);
    note("urlencode for query params and form values. urldecode errors on malformed `%xx` sequences.");

    example("Base64 encode / decode (encode accepts string or Blob)", &[
        r#"recon --script - <<< 'print(base64_encode("hello")); let b = base64_decode("aGVsbG8="); print(text::decode(b, "utf-8"));'"#,
    ]);
    note("base64_decode returns a Blob — convert with `text::decode(b, \"utf-8\")` when you want a String. Encoding uses the standard alphabet with `=` padding.");

    example("Decode HTML entities (companion to strip_html)", &[
        r#"recon --script - <<< 'print(html_entity_decode(strip_html("<p>Tom &amp; Jerry</p>")));'"#,
    ]);
    note("strip_html leaves entities alone (matches PHP strip_tags); html_entity_decode is the natural follow-up call when scraping text out of HTML.");

    example("Pad strings for column alignment", &[
        r#"recon --script - <<< 'print(str_pad("42", 6, "0", "left")); print(rpad("hi", 5, ".")); print(str_pad("hi", 6, "-", "both"));'"#,
    ]);
    note("str_pad takes (s, width [, pad [, side]]) with side ∈ left/right/both. lpad / rpad are the bare-name aliases. Multi-char pad strings cycle.");

    example("POSIX dirname / basename (optional suffix trim)", &[
        r#"recon --script - <<< 'print(dirname("/var/log/recon.log")); print(basename("/var/log/recon.log", ".log"));'"#,
    ]);
    note("Trailing slashes are stripped first. basename's optional suffix is trimmed from the result, and is only honoured when it doesn't equal the whole name.");

    example("Format a Unix timestamp", &[
        r#"recon --script - <<< 'print(date_format(1700000000, "%Y-%m-%dT%H:%M:%SZ"));'"#,
        r#"recon --script - <<< 'print(date_format(now_ms() / 1000, "%a %d %b %Y", "local"));'"#,
    ]);
    note("Format spec is chrono's strftime. Third arg switches between UTC (default) and \"local\" (system timezone).");

    section("JQ FILTER (0.89.0)");

    example("First match vs. all matches", &[
        r#"recon --script - <<< 'let a = [1, 2, 3, 4]; print(a.jq(".[] | select(. > 2)")); print(a.jq_all(".[] | select(. > 2)"));'"#,
    ]);
    note("`jq(filter)` returns the first result (or `()` if no match); `jq_all(filter)` returns every result as an Array. Both forms also callable as free functions: `jq(obj, f)` / `jq_all(obj, f)`. Backed by `jaq` — full jq grammar including pipes, `select`, `map`, `//`.");

    example("Pipe + select + project", &[
        r#"recon --script - <<< 'let prs = [#{n: 1, on: true}, #{n: 2, on: false}, #{n: 3, on: true}]; print(prs.jq_all(".[] | select(.on) | .n"));'"#,
    ]);
    note("Strings are not auto-parsed — chain `json_parse(s).jq(filter)` to start from JSON text. Filter parse and runtime errors throw and are catchable with try/catch.");

    section("CONFIGURATION FILES");

    example("Show which config files the layered resolver picked", &[
        "recon --show-config-paths",
    ]);

    example("Override the user-config path", &[
        "RECON_CONFIG=/path/to/my-config.toml recon --netstatus",
    ]);

    example("Skip the system layer for this invocation", &[
        "recon --no-system-config --netstatus",
    ]);

    example("Worked-example minimal layered setup", &[
        "# /etc/recon/config.toml  (admin-shipped)",
        "# [editor]",
        "# default = \"vim\"",
        "# [gh.accounts]",
        "# \"shared@example.com\" = \"shared-gh\"",
        "",
        "# ~/.recon/config.toml    (user override)",
        "# [editor]",
        "# default = \"zed\"",
        "# [gh.accounts]",
        "# \"me@home\" = \"my-personal-gh\"",
    ]);

    note("System layer search order on macOS: $HOMEBREW_PREFIX/etc/recon, /opt/homebrew/etc/recon, /usr/local/etc/recon, /etc/recon (first existing match wins, no merging across system candidates). Linux: /etc/recon only.");
    note("--disable / -q skips both layers. --no-system-config / --no-user-config skip just one. Skip flags always win over $RECON_SYSTEM_CONFIG / $RECON_CONFIG env vars.");

    section("GIT WRAPPER (0.89.0)");

    example("Run the shipped demo (creates a temp repo)", &[
        "recon --script script/git.rhai",
    ]);
    note("`git()` binds to the current directory; `git(path)` binds to a specific repo. Methods return parsed Maps and Arrays — `.status()` gives porcelain v2 data, `.log(n)` gives commit objects, `.diff()` returns the patch as a String. `.commit()` returns `{ hash, short_hash, subject }` after committing staged changes.");

    example("Quick branch + cleanliness check", &[
        r#"recon --script - <<< 'let g = git(); print(`${g.branch().current} ${if g.is_clean() {"clean"} else {"dirty"}}`);'"#,
    ]);

    example("List recent commits with subject", &[
        r#"recon --script - <<< 'for c in git().log(5) { print(`${c.short_hash} ${c.subject}`); }'"#,
    ]);
    note("Escape hatches: `.run(args)` sniffs JSON vs text, `.run_text(args)` and `.run_json(args)` are explicit. Errors throw on non-zero exit — wrap in `try`/`catch` to recover. Composes on top of `std::process::Command` directly rather than going through the shell() binding.");

    section("GH WRAPPER (0.89.0)");

    example("Run the shipped demo (skips when gh not authenticated)", &[
        "recon --script script/gh.rhai",
    ]);
    note("`gh()` resolves the current repo from cwd; `gh(\"owner/name\")` adds --repo to every call. Auto-account-switch: before every gh call, the wrapper reads `git config user.email` and runs `gh auth switch --user <handle>` when needed. Mapping comes from CLAUDE.md.");

    example("List your open PRs", &[
        r#"recon --script - <<< 'for p in gh().pr_list(#{ state: "open", author: "@me", limit: 10 }) { print(`#${p.number} ${p.title}`); }'"#,
    ]);

    example("Create a release with auto-generated notes", &[
        r#"recon --script - <<< 'let r = gh().release_create("v0.89.0", #{ generate_notes: true }); print(r.url);'"#,
    ]);

    example("Configure gh auto-account-switch via ~/.recon/config.toml", &[
        "cat > ~/.recon/config.toml <<'EOF'",
        "[gh.accounts]",
        "\"you@example.com\" = \"your-gh-handle\"",
        "EOF",
        "recon --script - <<< 'gh().pr_list();'   # auto-switches based on git config user.email",
    ]);

    note("`auth_status()` is the lone method that does NOT trigger auto-switch — useful when querying which account is currently active. All other methods throw on non-zero exit; `pr_view(<id>)` exiting 1 for \"not found\" is the canonical case scripts catch with try/catch.");

    section("EDITOR OUTPUT");

    example("Open the response in an editor (--editor [EDITOR])", &[
        "recon --editor zed https://httpbin.org/get",
        "recon --editor code https://example.com",
        "recon --editor vim -p https://api.github.com",
    ]);
    example("Use a raw command passed through sh -c", &[
        r#"recon --editor "code --new-window" https://example.com"#,
        r#"recon --editor "subl -n" https://example.com"#,
    ]);
    example("Use the default editor from ~/.recon/config.toml [editor] default", &[
        "recon --editor https://example.com",
    ]);
    example("Mirror the body to stdout as well (-vv)", &[
        "recon --editor zed -vv https://httpbin.org/get",
    ]);
    example("Purge all /tmp/recon-* temp files (standalone action)", &[
        "recon --editor-cleanup",
    ]);
    note("Built-in aliases: zed, code, cursor, subl, vim, nvim, nano, emacs. User aliases can be added under [editor.aliases] in ~/.recon/config.toml.");

    section("AI SCRIPT BINDINGS (0.79.0)");

    example("One-shot question to the configured backend", &[
        "# In a .rhai script:",
        "let a = ai::ask(\"Summarize the response headers\");",
        "# Selecting backend per script:",
        "ai::request().backend(\"claude\").prompt(q).send()",
    ]);

    example("Builder with system + accumulating context", &[
        "let req = ai::request();",
        "req.system(\"You are concise.\");",
        "req.context(\"Cert: \" + cert_pem);",
        "req.context(\"Probe: \" + banner);",
        "req.prompt(\"Anything unusual?\");",
        "let answer = req.send();",
    ]);

    example("Multi-turn replay (manual)", &[
        "let req = ai::request().prompt(\"Q1\");",
        "let a1 = req.send();",
        "req.assistant(a1);",
        "req.user(\"Q2\");",
        "let a2 = req.send();",
    ]);

    note("ai::* requires a backend (claude / codex / copilot / gemini, or a user-defined `cmd` entry in ~/.recon/config.toml under [ai.backends.<name>]). Select with .backend(), $RECON_AI_BACKEND, or [ai].default_backend. send() throws on failure — wrap in `try { ... } catch (e) { ... }` to recover.");

    section("INTERACTIVE REPL");

    example("Launch the REPL", &[
        "recon --repl",
    ]);
    note("Opens an interactive Rhai prompt with all script bindings available. Type :help for the cheat sheet, :quit to exit.");

    example("Preconfigure default flags at launch", &[
        "recon --repl -H 'X-Token: abc' -X POST",
    ]);
    note("The `flags` constant inside the REPL reflects the launch-time CLI flags. :set can adjust them at runtime.");

    example("Load a helper script into the session", &[
        "recon --repl",
    ]);
    note(":load ~/.recon/script/helpers.rhai  — Functions and let bindings defined in the file become available at the prompt.");

    example("Run a script in isolation (without touching REPL state)", &[
        "recon --repl",
    ]);
    note(":run benchmarks.rhai  — Builds a throwaway engine, evaluates the file, prints the return value, drops everything. REPL bindings unaffected.");

    example("Save a session as a reusable script", &[
        "recon --repl",
    ]);
    note(":save session.rhai  — Writes each successful input line to <path> with a timestamp header.");

    example("Save a session as a directly-runnable script", &[
        "recon --repl",
    ]);
    note(":save-tidy session.rhai  — Like :save, but appends missing `;` and drops entries that fail to parse; the result runs with `recon --script session.rhai` without manual fixup.");

    example("List every callable registered with the engine", &[
        "recon --repl",
    ]);
    note(":functions  — Probes, helpers, and builders that the engine knows about, plus user-defined functions. Pass `all` to also include the Rhai standard library.");

    println!();
}

fn section(title: &str) {
    println!("  {}", title.yellow().bold());
    println!();
}

fn example(desc: &str, commands: &[&str]) {
    println!("    {}", desc.bold());
    for cmd in commands {
        println!("      {}", cmd.cyan());
    }
    println!();
}

fn note(text: &str) {
    println!("    {} {}", "note:".dimmed().bold(), text.dimmed());
    println!();
}
