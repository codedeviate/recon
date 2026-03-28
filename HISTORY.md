# recon — Project History & Design Notes

## Overview

**recon** is a versatile network reconnaissance CLI tool written in Rust. It started as a basic curl clone and evolved into a multi-protocol network investigation tool covering HTTP/HTTPS requests, TLS certificate inspection, DNS lookups, WHOIS queries, ping, and traceroute.

---

## Versioning

recon follows semantic versioning (`MAJOR.MINOR.PATCH`):

- **MINOR** version is incremented when a new feature or flag is added, removed, or significantly changed.
- **PATCH** version is incremented for bug fixes, documentation/help text updates, and other minor changes that don't add or remove features or flags.
- **MAJOR** version is reserved for breaking changes to existing behaviour.

---

## Origins

The project began with a simple goal: build a basic curl clone in Rust that supports HTTP and HTTPS requests, compatible with JetBrains RustRover.

### Initial Requirements
- Standard Cargo project structure (RustRover compatible)
- HTTP and HTTPS support with TLS
- curl-like CLI flags

---

## Architecture Decisions

### HTTP Client: `reqwest` (blocking) + `rustls`

**Decision:** Use `reqwest` in blocking mode with `rustls-tls` instead of async or native-tls.

**Why blocking over async:** A CLI tool has one request lifecycle per invocation. The blocking API is simpler, produces smaller binaries, and avoids needing a `tokio` runtime for the core HTTP path.

**Why rustls over native-tls:** `rustls` is a pure-Rust TLS implementation. It avoids a dependency on the system's OpenSSL, making the binary portable on macOS without Homebrew path issues and easier to cross-compile.

### CLI: `clap` with derive macros

The derive macro approach was chosen over the builder API — the struct doubles as documentation and produces `--help` output automatically with less boilerplate.

### Error handling: `anyhow`

Used throughout for clean, chainable error propagation without custom error types.

---

## Feature Additions (Chronological)

### 1. Basic HTTP/HTTPS (`initial`)

Core HTTP client with:
- GET (default), POST, PUT, DELETE, PATCH, HEAD methods
- Custom headers (`-H`)
- Request body (`-d`, supports `@file` prefix)
- Follow redirects (`-L`, `--max-redirs`)
- Output to file (`-o`) with progress bar
- Silent (`-s`), verbose (`-v`), include headers (`-i`)
- Custom User-Agent (`-A`)
- Connection timeout (`--connect-timeout`)
- Fail on HTTP error (`-f`)

**Modules introduced:** `cli.rs`, `client.rs`, `output.rs`, `main.rs`

---

### 2. Output Filtering (`--BODY`, `--HEAD`)

Added two flags to control what part of the response is printed:

- `--BODY` — print only the response body, suppress status line
- `--HEAD` — print only the response headers, suppress body

**Design note:** `--HEAD` reuses the existing header-printing logic from `-i`/`-v` but routes output to stdout instead of stderr and exits before streaming the body.

---

### 3. Friendly Error Messages (`--FULL-ERRORS`)

**Problem:** `anyhow`'s default error output when used as `fn main() -> anyhow::Result<()>` dumps the full internal error chain including reqwest internals, OS error codes, and rustls details — not user-friendly.

**Solution:** Switched `main()` to return `()` and handle errors manually. A `friendly_message()` function classifies errors into readable messages:

| Root cause pattern | Friendly message |
|---|---|
| `dns error` | `Could not resolve host: <url>` |
| `Connection refused` | `Connection refused: <url>` |
| `timed out` | `Connection timed out` |
| `certificate` / `TLS` | TLS certificate error message |
| File not found | `File not found: <path>` |
| Permission denied | `Permission denied: <path>` |

`--FULL-ERRORS` bypasses all of this and prints the full `anyhow` chain with `{:#}` formatting, useful for debugging.

---

### 4. TLS Certificate Inspection (`--cert`)

**Goal:** Fetch and display a server's TLS certificate without making an HTTP request.

**Approach chosen:** Use `native-tls` (already a transitive dependency via `hyper-tls`) rather than `rustls` directly.

**Why native-tls over rustls directly:**
- `rustls` 0.23 has a complex crypto provider API requiring explicit provider installation
- `native-tls` provides `TlsStream::peer_certificate()` which returns the raw certificate cleanly
- `native-tls` wraps the platform TLS (SecureTransport on macOS, OpenSSL on Linux) — more reliable

**Certificate verification is intentionally disabled** during the connection (`danger_accept_invalid_certs(true)`) so the tool can inspect expired, self-signed, or hostname-mismatched certificates — the whole point of a cert inspection tool.

Certificate parsing uses `x509-parser` to extract:
- Subject (CN, O, OU, C, ST, L)
- Issuer
- Validity period with coloured status (green/yellow/red)
- Subject Alternative Names (DNS, IP, email)
- Serial number (hex)
- Signature algorithm (OID mapped to human-readable name)
- Public key type and size (RSA key size computed from modulus byte length)

**URL normalisation:** Bare hostnames (`example.com`, `example.com:8443`) are accepted by prepending `https://` before parsing with the `url` crate.

**Module introduced:** `cert.rs`

---

### 5. Network Diagnostics: DNS, WHOIS, Ping, Traceroute

A large feature set added in one pass. Each feature lives in its own module and shares a common `parse_target()` helper.

#### Shared URL parsing (`util.rs`)

```
parse_target("https://example.com:8080/path") → ("example.com", Some(8080))
parse_target("example.com")                   → ("example.com", None)
parse_target("example.com:443")               → ("example.com", Some(443))
```

Handles protocol stripping, path/query removal, and IPv6 bracket notation.

---

#### DNS Lookup (`--dns`, `--dns-type`)

**Crate:** `hickory-resolver` 0.24 (formerly `trust-dns-resolver`) — pure Rust DNS client supporting all record types.

**Runtime:** Since hickory uses async internally, a single-threaded `tokio` runtime is created inside `dns::run()` with `block_on`. This keeps the rest of the codebase synchronous.

**Default record types queried:** A, AAAA, CNAME, MX, NS, TXT, SOA

**Explicit types** via `--dns-type A,MX,CAA` (comma-separated). When types are explicitly requested, errors and empty results are shown. For default lookups, `NoRecordsFound` errors are silently skipped so the output only shows what exists.

**Record formatting:** Each `RData` variant is matched and formatted to a human-readable string. Unknown variants fall back to `Debug` format.

**Module introduced:** `dns.rs`

---

#### WHOIS Lookup (`--whois`)

**Implementation:** Pure TCP, no external crate. The WHOIS protocol is simple — connect to port 43, send `domain\r\n`, read until EOF.

**Referral chain (up to 3 levels):**
1. Query `whois.iana.org` — returns the authoritative TLD/RIR server via `refer:` line
2. Query that server — returns registry-level WHOIS data, may contain `Registrar WHOIS Server:` referral
3. Query registrar server — returns full registration details

**Result shown:** Only the most specific (deepest) response is printed. If a query fails, falls back to the previous level's response.

**Works for:** Domains (follows TLD → registrar chain) and IP addresses (IANA refers to ARIN/RIPE/APNIC).

**Module introduced:** `whois.rs`

---

#### Ping (`--ping`, `--ping-count`)

Two modes depending on whether a port is in the address:

**ICMP ping (no port):** Implemented in pure Rust using `socket2` with `SOCK_DGRAM` + `IPPROTO_ICMP`.

- On macOS (10.14+), `SOCK_DGRAM` ICMP works without root privileges
- On Linux, requires `net.ipv4.ping_group_range` sysctl or root; fails with a clear, actionable error message suggesting TCP ping as an alternative
- Manually constructs ICMP Echo Request packets (type=8) with Internet checksum
- Handles received packets that may or may not include the IP header (auto-detected by checking if first byte is an IPv4 header marker)
- Shows per-packet RTT and min/avg/max statistics

**TCP ping (port given, e.g. `example.com:443`):** Uses `TcpStream::connect_timeout`. Pure Rust, no privileges needed, works everywhere. Shows connection RTT per attempt and a statistics summary.

**Module introduced:** `ping.rs`

---

#### Traceroute (`--traceroute` / `--trace`, `--max-hops`)

**Decision:** Spawn the system `traceroute` command rather than implementing raw socket TTL probing.

**Why system command:**
- ICMP traceroute requires raw sockets (`SOCK_RAW`) which need root or setuid on all platforms
- The system `traceroute` binary has the SUID bit set, so it works for regular users without sudo
- Re-implementing TTL probing + ICMP Time Exceeded reception in pure Rust would require root anyway, adding complexity with no benefit

**Port support:** Passes `-p PORT` to `traceroute` when a port is specified in the address. On Unix, `-p` sets the destination port for UDP probes. Windows `tracert` does not support port selection.

**Cross-platform:** Uses `#[cfg(target_os = "windows")]` to switch between `traceroute` (Unix) and `tracert` (Windows).

**Module introduced:** `traceroute.rs`

---

### 6. Redirect Header Tracing (`--LHEAD`)

**Goal:** Inspect the full redirect chain, seeing every response's headers at each hop — not just the final destination.

**Problem with the existing approach:** `reqwest`'s built-in redirect policy (`Policy::limited`) follows redirects internally and discards intermediate responses. Only the final response is returned, making it impossible to inspect 301/302 headers along the way.

**Solution:** When `--LHEAD` is active, redirect following is disabled on the `reqwest` client (`Policy::none()`). A manual loop in `client.rs` handles each hop:
1. Send request to the current URL
2. If the response is a 3xx with a `Location` header and redirects remain, print that response's headers and resolve the next URL
3. Otherwise return the response as the final result

Relative `Location` URLs (e.g. `/new-path`) are resolved against the current URL using the `url` crate's `Url::join()`.

**Output format:** Each intermediate hop prints to stdout:
```
* https://example.com
< HTTP/1.1 301 Moved Permanently
< location: https://www.example.com
<
* Redirecting to https://www.example.com

* https://www.example.com    ← final response label
< HTTP/1.1 200 OK
< content-type: text/html
<
```

**Flag naming:** Follows the existing uppercase long-flag convention (`--HEAD`, `--BODY`, `--FULL-ERRORS`). The name combines `-L` (follow redirects) and `--HEAD` (print headers).

**Implies redirect following** — no need to also pass `-L`.

**`max-redirs` is respected** — the same limit applies to the manual loop.

---

### 7. Response Prettification (`-p` / `--prettify`)

**Goal:** Print response bodies in a human-readable, indented format directly in the terminal without piping to external tools.

**Supported formats and how they are detected:**

| Format | Content-Type match | Body sniff fallback |
|---|---|---|
| JSON | `application/json`, `text/json`, `application/ld+json` | starts with `{` or `[` |
| XML | `application/xml`, `text/xml`, `application/rss+xml`, `application/atom+xml` | starts with `<?xml` |
| HTML | `text/html`, `application/xhtml+xml` | contains `<!doctype html` or `<html` |
| YAML | `application/yaml`, `text/yaml`, `application/x-yaml` | — |
| CSV | `text/csv` | — |
| TSV | `text/tab-separated-values` | — |

If neither the header nor sniffing matches, the body is printed as-is.

**Implementation per format:**

- **JSON** — `serde_json`: parse into `Value`, re-serialize with `to_string_pretty` (2-space indent).
- **XML** — `quick-xml`: event-stream reader with `trim_text`, re-emitted through `Writer::new_with_indent` (2-space indent). Handles attributes, CDATA, namespaces, and processing instructions correctly.
- **YAML** — `serde_yaml`: parse into `Value`, re-serialize. The `---` document marker prepended by serde_yaml is stripped from output.
- **HTML** — Custom byte-scanner: walks the raw bytes tag by tag, tracking indent depth. Closing tags dedent before printing; void elements (`br`, `img`, `input`, etc.) don't affect depth; raw-text elements (`script`, `style`) have their inner content copied verbatim to avoid misinterpreting `<` characters inside JS/CSS.
- **CSV/TSV** — Custom column aligner: parses all rows (quote-aware splitting), computes max width per column, renders a bordered ASCII table with `=` separator after the header row.

**Flag naming:** `-p` is the natural single-character alias — short, mnemonic, and unambiguous given the existing flag set.

**Body reading:** `--prettify` reads the full body into memory via `response.text()` before formatting, unlike the normal streaming path. The progress bar is therefore skipped. File output (`-o`) is still supported.

**New module:** `prettify.rs`

**New dependencies:**

| Crate | Purpose |
|---|---|
| `serde_json` | JSON parse and pretty-print |
| `serde_yaml` | YAML parse and pretty-print |
| `quick-xml` | XML event streaming and indented re-serialization |

---

### 8. Status Code Output (`-S` / `--status`)

Prints only the numeric HTTP status code to stdout and exits — no headers, no body, no status text.

```
200
```

Implemented as an early return in `write_response()`, before all other output logic, so it is unaffected by `-i`, `-v`, `--HEAD`, `--BODY`, or `--prettify`.

Composes naturally with other flags — for example, `-L` follows redirects first and reports the final status code:

```
recon https://httpbin.org/redirect/3 -S -L
```

---

### 9. Usage Examples (`--examples`)

Prints a comprehensive, colour-formatted reference of every flag and command, grouped into sections, with real-world example invocations.

**Sections:** HTTP Requests · Redirects · Output Control · Error Handling · TLS Certificate · DNS Lookups · WHOIS · Ping · Traceroute · Combining Flags

**Implementation note:** `--examples` is intercepted via a `std::env::args()` scan *before* clap parses `argv`. This allows the flag to work without providing a URL, since clap would otherwise reject the invocation as missing the required positional argument. The flag is still declared in the `Args` struct so it appears in `--help`.

**New module:** `examples.rs`

---

### 10. curl-compatible `--url` flag

**Goal:** Accept the URL as a named flag (`--url https://example.com`) in addition to the existing positional argument, for drop-in compatibility with curl scripts and muscle memory.

**Implementation:** The positional `url` argument was changed from `String` to `Option<String>` with `required_unless_present = "url_flag"`, so clap still rejects invocations where neither form is provided. A second field `url_flag` carries the `--url` value. A `target_url()` method on `Args` resolves the effective URL at call sites, preferring `--url` over the positional when both are given.

All three forms are valid:

```
recon https://example.com
recon --url https://example.com
recon https://example.com --url https://example.com   # --url takes precedence
```

No behaviour changes — the resolved URL is used identically regardless of which form supplied it.

---

### 11. Cookie Jar (`--cookiejar`, `--cookies`, `--cookie-delete`, `--cookie-set`)

**Goal:** Persist cookies across requests so multi-step flows (login → authenticated requests) work without manual header juggling.

**Storage:** SQLite database via `rusqlite`. Each named jar lives at `~/.recon/jars/<name>.db`. Passing an absolute/relative path ending in `.db` uses that file directly instead.

**Schema:**

```sql
CREATE TABLE cookies (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    domain     TEXT    NOT NULL,
    path       TEXT    NOT NULL DEFAULT '/',
    name       TEXT    NOT NULL,
    value      TEXT    NOT NULL,
    expires    INTEGER,            -- Unix timestamp, NULL = session cookie
    secure     INTEGER NOT NULL DEFAULT 0,
    http_only  INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    UNIQUE(domain, path, name)
);
```

`ON CONFLICT(domain, path, name) DO UPDATE SET …` provides upsert semantics so re-visiting a page with `Set-Cookie` updates the stored value rather than inserting a duplicate.

**RFC 6265 matching:**

- **Domain:** A leading `.` on the stored domain enables subdomain matching (added automatically when the `Set-Cookie` header includes a `Domain=` attribute, per RFC 6265 §5.2.3). Without a leading dot, only exact host matches are sent.
- **Path:** The stored `path` must be a prefix of the request path (with `/` matching everything).
- **Secure flag:** Cookies with `Secure` are only sent over HTTPS.
- **Expiry:** `Max-Age` takes precedence over `Expires`. `Max-Age=0` deletes the cookie immediately.

**Cookie injection:** Before each request, `cookies_for(domain, path, is_https)` queries the database and builds a `Cookie: name=val; …` header. After each response, all `Set-Cookie` headers are processed and persisted.

**Management commands** (no URL required):

| Flag | Action |
|---|---|
| `--cookiejar <name> --cookies` | List all cookies in the jar as a formatted table |
| `--cookiejar <name> --cookie-set "…"` | Insert/update a cookie from a `Set-Cookie`-style string |
| `--cookiejar <name> --cookie-delete <id>` | Delete the cookie with the given row ID |

After `--cookie-set` or `--cookie-delete` the jar contents are always printed automatically so you can confirm the change without a separate `--cookies` call.

**`--cookie-set` format:** `name=value; Domain=example.com; [Path=/]; [Secure]; [HttpOnly]; [Max-Age=N]` — same syntax as a `Set-Cookie` header; `Domain=` is required.

**New module:** `cookiejar.rs`

**New dependency:** `rusqlite = "0.32"` — SQLite bindings (statically links `libsqlite3`)

---

### 15. SCP Download (`scp://`)

**Goal:** Download files over SSH using the familiar `scp://` URL scheme.

**URL format:** `scp://[user@]host[:port]/path/to/file`

```
recon scp://neh.localhost/home/thomas.bjork/file.tgz
recon scp://thomas@neh.localhost:2222/home/thomas.bjork/file.tgz
```

**Authentication — tried in order:**
1. SSH agent (if `$SSH_AUTH_SOCK` is set and the agent is running)
2. Explicit key via `--ssh-key <path>` (passphrase via `--ssh-pass`)
3. Default key files: `~/.ssh/id_ed25519`, `~/.ssh/id_ecdsa`, `~/.ssh/id_rsa`, `~/.ssh/id_dsa`
4. Password via `-u user:pass` or `--ssh-pass`

**New flags:**

| Flag | Purpose |
|---|---|
| `--ssh-key <path>` | Path to SSH private key file |
| `--ssh-pubkey <path>` | Path to SSH public key (optional; libssh2 derives it if omitted) |
| `--ssh-pass <phrase>` | Key passphrase (when used with `--ssh-key`) or SSH password |

**Credential resolution:**
- Username: URL userinfo (`scp://user@host`) → `-u user` flag → `$USER` / `$LOGNAME`
- Password/passphrase: `--ssh-pass` → `:pass` part of `-u user:pass`

**Host key verification:** Checked against `~/.ssh/known_hosts` by default using libssh2's built-in known-hosts API. `--insecure` skips the check (same flag as for TLS). If `known_hosts` doesn't exist, a warning is printed but the connection proceeds.

**Default output filename:** The basename of the remote path, written to the current directory. Override with `-o`:
- `-o file.tgz` — exact path
- `-o /tmp/` — directory, remote basename preserved inside it

**Progress bar:** Opt-in via `--progress` (consistent with the HTTP download behaviour).

**Crate:** `ssh2 = "0.9"` — synchronous libssh2 bindings. Requires libssh2 to be installed:
- macOS: `brew install libssh2`
- Linux: `apt install libssh2-1-dev` / `dnf install libssh2-devel`

**Channel close sequence:** libssh2 requires explicit `send_eof` → `wait_eof` → `close` → `wait_close` after reading all SCP data. Omitting this causes the remote sshd to hang on large transfers. This is handled correctly in `scp.rs`.

**Module introduced:** `scp.rs`

**Dependency added:** `ssh2 = "0.9"`

---

### 16. Email Protection Validation (`--spf`, `--dmarc`, `--dkim`, `--mta-sts`, `--bimi`, `--tls-rpt`)

**Goal:** Validate email authentication and protection DNS records with deep inspection, recursive resolution, and cross-referencing between checks.

**Architecture:** A new `src/email/` module directory with a shared orchestrator and one sub-module per check. All checks share a single `hickory-resolver` instance (same pattern as `dns.rs`) for DNS caching.

**Dispatch refactor:** The `main.rs` dispatch was changed from a single `if/else if` chain (only one feature at a time) to two groups:

- **Exclusive:** `--ping`, `--traceroute`, `--whois` — mutually exclusive, error if combined with each other or with composable flags
- **Composable:** `--cert`, `--dns`, `--spf`, `--dmarc`, `--dkim`, `--mta-sts`, `--bimi`, `--tls-rpt` — any combination runs sequentially

This allows running a full domain audit in one invocation:

```
recon --cert --dns --dns-type A,AAAA,MX,TXT --dmarc --spf --dkim google example.com
```

**Output format:** Each check prints a coloured verdict badge:
- `✓ PASS` (green) — record exists and validates correctly
- `⚠ WARN` (yellow) — record exists but has issues
- `✗ FAIL` (red) — record missing, malformed, or violates RFC

#### SPF (`--spf`)

Validates `v=spf1` TXT record per RFC 7208: multiple-record PermError detection, recursive `include:`/`redirect=` tree with indented display, DNS lookup counter (max 10), void lookup counter (max 2), warnings for `ptr`, `+all`, missing default.

#### DMARC (`--dmarc`)

Validates `_dmarc.<domain>` TXT per RFC 7489: policy (`p=`) required with strength checks, subdomain policy (`sp=`) comparison, alignment modes (`adkim=`/`aspf=`), percentage (`pct=`), reporting URI validation with external authorization record check.

#### DKIM (`--dkim <selector>`)

Validates `<selector>._domainkey.<domain>` TXT: RSA public key size via ASN.1 DER parsing, Ed25519 support, hash/service/flag validation. Repeatable for multiple selectors.

#### MTA-STS (`--mta-sts`)

Two-phase: DNS `_mta-sts.<domain>` TXT + HTTPS policy fetch from `https://mta-sts.<domain>/.well-known/mta-sts.txt`. Validates mode, max_age, MX pattern matching against real MX records.

#### BIMI (`--bimi [selector]`)

Validates `<selector>._bimi.<domain>` TXT (default: `default`): logo URL must be HTTPS SVG, optional VMC certificate parsed for expiry and BIMI EKU OID.

#### TLS-RPT (`--tls-rpt`)

Validates `_smtp._tls.<domain>` TXT per RFC 8460: version check, reporting URI validation.

#### Cross-validation

When multiple checks run together: DMARC notes SPF/DKIM alignment, BIMI verifies DMARC policy strength, MTA-STS and TLS-RPT note co-presence.

**New modules:** `src/email/mod.rs`, `spf.rs`, `dmarc.rs`, `dkim.rs`, `mta_sts.rs`, `bimi.rs`, `tls_rpt.rs`

**New dependencies:** `base64` (DKIM key decoding), `pem` (VMC certificate parsing)

---

### 17. Per-Topic Help (`--help <topic>`)

**Goal:** Provide detailed, man-page-style help for each feature area without losing the concise overview of `--help`.

**Invocation:** `recon --help <topic>` displays in-depth help for that topic — description, flags with full explanations, related flags, and examples. Plain `--help` is unchanged except for a footer listing available topics.

**Implementation:** Pre-clap argv interception in `main.rs` (same pattern as `--examples`). Scans for `--help`/`-h` before clap parses, checks if the next argument is a topic name. If so, dispatches to `help::print_topic()`. If no topic, calls clap's `print_help()` manually and appends the topic footer.

**Topics (16):** http, output, dns, cert, whois, ping, traceroute, spf, dmarc, dkim, mta-sts, bimi, tls-rpt, email, cookies, scp

**Aliases:** `https` → http, `tls`/`certificate` → cert, `trace` → traceroute, `mtasts` → mta-sts, `tlsrpt` → tls-rpt, `email-protection` → email, `cookiejar`/`cookie` → cookies, `ssh` → scp. Case-insensitive.

**Unknown topic handling:** Prints "Unknown topic: X" and lists all available topics.

**Module introduced:** `help.rs`

---

### 18. HTTP/HTTPS File Server (`--serve`, `--serve-tls`)

**Goal:** Serve the current directory over HTTP and/or HTTPS, like Python's `http.server` but with TLS support, HTTP/2, and access logging.

**Architecture:** A new `src/serve/` module directory using `hyper` 1.x for the HTTP server and `tokio-rustls` for TLS. Both HTTP and HTTPS servers can run simultaneously as concurrent tokio tasks on a multi-threaded runtime.

**HTTP version negotiation:** Plain HTTP uses HTTP/1.1. HTTPS negotiates HTTP/1.1 and HTTP/2 via ALPN by default. `--http-version 1.1` or `--http-version 2` forces a specific version on HTTPS.

**Directory listing:** Content-negotiated — HTML table for browsers (Accept: text/html), plain text for CLI tools (curl, wget). Sorted directories-first, then alphabetical. Shows filename, size, and modification date.

**Access logging:** Apache-style log printed to stderr (colour-coded by status: green for 2xx, yellow for 3xx, red for 4xx/5xx). Optionally mirrored to a file via `--serve-log` (plain text, no ANSI codes).

**TLS certificates:** Default location `~/.recon/cert.pem` and `~/.recon/key.pem`. Override with `--serve-cert` and `--serve-key`. If files are missing, the error message includes an `openssl` command to generate self-signed certs.

**Dispatch:** `--serve`/`--serve-tls` form their own exclusive group — they can combine with each other but not with any other recon feature.

**New modules:** `src/serve/mod.rs`, `http.rs`, `https.rs`, `files.rs`

**New dependencies:** `hyper`, `hyper-util`, `http-body-util`, `bytes`, `tokio-rustls`, `rustls-pemfile`, `mime_guess`

**Modified:** `tokio` (added `rt-multi-thread`, `macros`, `signal`, `fs`, `io-util` features)

---

### 14. Output Model Overhaul + New Flags

Several output and request flags were added or reworked to align more closely with curl conventions:

**Default output changed to body-only:** Previously the status code was always printed to stderr. Now the default output is the response body only — no status line. Status/headers only appear when explicitly requested via `-I`/`--head`, `--full`, `-i`, or `-v`.

**`--BODY` removed:** Redundant now that body-only is the default.

**`--HEAD` renamed to `-I` / `--head`:** Matches curl's flag names exactly. Behaviour unchanged — prints headers only, no body.

**`--full` added:** Prints status line, all headers, and the body to stdout. Equivalent to the old `-i` in terms of output, but named more intuitively.

**`-v` / `-vv` verbose levels:** The verbose flag is now a counter. `-v` gives the existing request/response header output to stderr. `-vv` additionally prints the effective URL, active auth credentials (username only), and elapsed request time.

**`-u` / `--user user:pass`:** HTTP Basic authentication. Parsed as `user:pass`; if no `:` is present, the whole value is treated as the username with no password. Passed to reqwest's `basic_auth()` which encodes the `Authorization` header correctly.

**`--progress`:** Progress meter when saving to a file is now opt-in. Previously it appeared automatically unless `-s` was set. Now it only shows when `--progress` is explicitly passed. This is a deliberate departure from curl's default-on behaviour.

**`-G` / `--get`:** Forces the method to GET and appends `-d` data to the URL as a query string instead of sending it as the request body. Mirrors curl's `-G` exactly.

---

### 13. Insecure Mode (`-k` / `--insecure`)

**Goal:** Skip TLS certificate verification for HTTPS requests, mirroring curl's `-k`/`--insecure` behaviour.

**What is skipped:** Hostname verification, certificate expiry check, and chain validation against trusted CAs. Any certificate is accepted.

**Use cases:** Self-signed certificates on internal/staging hosts, expired certificates that need to be reached anyway, hosts using a private CA not in the system trust store.

**Implementation:** Passes `.danger_accept_invalid_certs(true)` to the `reqwest` `Client::builder()` when the flag is set. No other behaviour changes — cookies, redirects, prettification, and all other flags compose as normal.

**Note:** This flag is intentionally not applied to `--cert` (TLS certificate inspection), which already disables verification unconditionally, since inspecting a certificate without disabling verification would defeat the purpose.

**Flag naming:** `-k` and `--insecure` match curl exactly for muscle-memory compatibility.

---

### 12. Default Cookie Jar Value

**Goal:** Reduce typing for users who always use one jar — `--cookiejar` alone should just work.

**Implementation:** clap v4's `num_args = 0..=1` combined with `default_missing_value = "default"` makes the `--cookiejar` value optional at the CLI level. When the flag is present but no value follows, clap substitutes `"default"`, resolving to `~/.recon/jars/default.db`.

**All valid forms:**

```
recon https://example.com --cookiejar             # uses ~/.recon/jars/default.db
recon https://example.com --cookiejar mysession   # uses ~/.recon/jars/mysession.db
recon https://example.com --cookiejar ./tmp.db    # uses ./tmp.db directly
recon --cookiejar --cookies                       # lists the default jar, no URL needed
```

The `required_unless_present_any` on the positional URL was extended to include `cookies`, `cookie_delete`, and `cookie_set` so management commands (`--cookies`, `--cookie-delete`, `--cookie-set`) can be used without specifying a URL.

---

## Naming History

The project started as **curlclone** — an accurate but uninspiring name given how much the tool grew beyond simple HTTP requests.

### Candidates considered

| Name | Verdict |
|---|---|
| `probe` | Good fit, but blocked: crates.io name taken (static tracing lib), and `probelabs/probe` (498 stars) uses the same binary name |
| `scout` | Clean, available |
| `tap` | Very short, available |
| `pry` | Short, punchy, available |
| `hop` | Network-y, very short |
| `recon` | Clean on Homebrew, crates.io, and binary namespace |

### Final name: **recon**

Short (5 chars), easy to type, easy to pronounce, and accurately describes the tool's purpose: network reconnaissance. No conflicts found on Homebrew, crates.io, or as a binary name.

---

## Module Structure

```
src/
  main.rs         Entry point — arg parsing and feature dispatch
  cli.rs          clap derive struct with all flags
  client.rs       HTTP request construction and execution (reqwest)
  output.rs       Response streaming, headers, progress bar
  cert.rs         TLS certificate fetch and display (native-tls + x509-parser)
  dns.rs          DNS lookups (hickory-resolver, all record types)
  whois.rs        WHOIS TCP client with referral chain following
  ping.rs         ICMP ping (socket2) and TCP ping (TcpStream)
  traceroute.rs   Traceroute via system command
  util.rs         Shared host/port parsing from any URL format
  cookiejar.rs    SQLite cookie storage, RFC 6265 matching, management helpers
  prettify.rs     Response body prettification (JSON, XML, HTML, YAML, CSV, TSV)
  examples.rs     Colour-formatted usage examples for --examples
  scp.rs          SCP file download via libssh2
  email/
    mod.rs        Email check orchestrator and shared resolver
    spf.rs        SPF record validation (RFC 7208)
    dmarc.rs      DMARC record validation (RFC 7489)
    dkim.rs       DKIM public key record validation
    mta_sts.rs    MTA-STS DNS + HTTPS policy validation
    bimi.rs       BIMI logo/VMC record validation
    tls_rpt.rs    TLS-RPT record validation (RFC 8460)
```

---

## Dependencies

| Crate | Purpose |
|---|---|
| `reqwest` (blocking, rustls-tls) | HTTP/HTTPS client |
| `clap` (derive) | CLI argument parsing |
| `anyhow` | Error handling |
| `indicatif` | Download progress bar |
| `colored` | Terminal colour output |
| `native-tls` | TLS connection for certificate inspection |
| `x509-parser` | X.509 certificate parsing |
| `url` | URL parsing and normalisation |
| `hickory-resolver` (system-config) | DNS client, all record types |
| `tokio` (rt, net) | Async runtime for hickory-resolver |
| `socket2` | Raw ICMP socket for ping |
| `rusqlite` | SQLite cookie jar storage |
| `serde_json` | JSON parse and pretty-print |
| `serde_yaml` | YAML parse and pretty-print |
| `quick-xml` | XML event streaming and indented re-serialization |
| `ssh2` | SCP file download via libssh2 bindings |
| `base64` | DKIM public key decoding |
| `pem` | VMC certificate PEM parsing for BIMI |
