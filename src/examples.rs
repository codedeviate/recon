use colored::Colorize;

pub fn print() {
    let title = "recon — usage examples";
    println!("\n{}\n", title.bold());

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
