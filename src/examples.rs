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
    example("Print only the HTTP status code (-S / --status)", &[
        "recon https://httpbin.org/get -S",
        "recon https://httpbin.org/status/404 -S",
        "recon https://api.example.com/health -S -L",
    ]);
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
    example("List supported algorithms", &[
        "recon --compress-list",
    ]);
    note("Level aliases (fastest/fast/default/good/best) map to each algorithm's native scale. See --help compression for the word-to-number table.");

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
        "recon --script script/browser-title.rhai https://example.com",
        "cp script/*.rhai ~/.recon/script/       # then: recon --script browser-title URL",
    ]);

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

    note("Available functions: http/https/request, tcp, ping, dns, tls, ntp, redis, ws/wss, dict, ldap/ldaps, whois, memcached, rtsp/rtsps, mqtt_pub/mqtt_sub, file_read. Hashes: md5, sha1, sha256, sha384, sha512, sha3_256, sha3_512, blake3, crc32, plus hash(algo, x [, \"hex\"|\"base64\"]). Helpers: print, sleep_ms, env, now, now_ms, assert, json_parse, json_stringify (compact or pretty via bool / integer indent). See `recon --help script`.");

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
