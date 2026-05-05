# recon ↔ curl feature parity

Quick reference for how recon covers the features curl advertises in its
`--version` output. Updated alongside `OUT-OF-SCOPE.md`; if something
lands or gets explicitly declined, this file changes.

Last sweep: 0.77.0 (2026-05-05).

## Legend

- ✅ **Shipped** — feature is available via recon (CLI flag and/or
  always-on capability).
- ✅ **Always-on** — Rust / recon gets this behaviour for free by virtue
  of the toolchain or architectural choices. No user-visible flag
  needed, but the capability is present.
- ❌ **Architectural N/A** — feature doesn't map onto a single-backend
  Rust binary. Not a coverage gap.
- ⏸ **Deferred** — recon has deliberately punted. Rationale in
  [`OUT-OF-SCOPE.md`](../OUT-OF-SCOPE.md).
- 🆕 **Beyond curl** — feature recon has that curl doesn't advertise.

## Feature matrix

### Curl `--version` features that recon also has

| curl feature | Status | Notes |
|---|---|---|
| AsynchDNS | ✅ Always-on | hickory-resolver's `TokioAsyncResolver` powers DNS resolution; see `src/dns_resolver.rs`. All DNS lookups are non-blocking at the primitive level. |
| Largefile | ✅ Always-on | Rust's native 64-bit types handle files > 2 GB on every target. |
| libz | ✅ Shipped (different impl) | Pure-Rust `flate2` crate. Identical wire behaviour; no system libz dep. Response gzip/deflate handled automatically by `--compressed` / reqwest's feature flags. |
| threadsafe | ✅ Always-on | Rust's type system enforces thread safety at compile time. No runtime flag needed. |
| HTTP2 | ✅ Always-on | reqwest 0.12 negotiates HTTP/2 via ALPN by default. |
| HTTPS | ✅ Always-on | rustls-tls with webpki roots; or BoringSSL with `--features impersonate`. |
| IPv6 | ✅ Always-on | reqwest + hickory do v6 transparently; `--dns-ipv6-addr` for local-address binding; `--ipv4` / `--ipv6` for resolver hints. |
| SSL | ✅ Always-on | Reported via `recon --version` (matches curl's label; actually means TLS). |
| gzip / deflate / brotli / zstd | ✅ Shipped | Pure-Rust implementations. Transparent on responses; explicit on `--compress` / `--decompress` / `--archive`. |
| Proxy (HTTP / SOCKS5) | ✅ 0.50.0 | `-x / --proxy URL` with scheme-based routing. See `recon --help proxy`. |
| HTTPS-proxy | ✅ 0.50.0 | `-x https://proxy:8443/` is recognised; `--proxy-cacert`, `--proxy-capath`, `--proxy-ca-native`, `--proxy-insecure` for TLS-to-proxy configuration (0.72.0). `--proxy-pass` accepted at parse time but warns at runtime — reqwest 0.12 doesn't expose a passphrase API for proxy mTLS. |
| UnixSockets | ✅ 0.51.0 | `--unix-socket /path/to/sock`; Docker / systemd / kubelet diagnostics. |
| HSTS | ✅ 0.52.0 | `--hsts <file>` persistent cache; curl-compatible file format. |
| HTTP3 | ⏸ Deferred | reqwest 0.12 has experimental HTTP/3 behind a feature flag, but rustls / quinn integration in reqwest is not yet stable for general HTTP work. Revisit when reqwest's H3 surface matures. Note: `--features impersonate` is also H1+H2 only because rquest 5.1.0 doesn't yet support QUIC fingerprinting. |
| alt-svc | ⏸ Deferred | RFC 7838 Alt-Svc header cache. reqwest exposes no primitives; hand-rolling ~300 lines for a niche one-shot use case didn't pass the bar. See OUT-OF-SCOPE.md. |
| Kerberos / SPNEGO / GSS-API | ⏸ Deferred | All three share the libgssapi-krb5 FFI dep on Linux/macOS and Windows SSPI on Windows. Cross-platform maintenance tax significant; users needing enterprise auth tend to have curl installed. |
| NTLM | ⏸ Deferred | Windows-only via the `sspi` crate's FFI. Niche for modern APIs. |
| TLS-SRP | ⏸ Deferred | RFC 5054 TLS-SRP. Neither rustls nor BoringSSL exposes SRP cipher suites in stable APIs. Vanishingly rare on the public web; users with SRP-only servers tend to use openssl s_client. |
| ECH | ⏸ Deferred | Encrypted Client Hello (RFC draft). rustls 0.23 has experimental ECH only behind unstable features; not yet wired in reqwest. Revisit when rustls + reqwest both stabilise it. |
| IDN | ⏸ Deferred | International Domain Names. The `url` crate (used by reqwest) handles `https://例え.jp/` parsing, so URLs with non-ASCII hosts work, but recon doesn't actively normalize / Punycode-convert user input via `idna` outside of URL parsing. Adding explicit `--idn` flags is on the wishlist if a real workflow surfaces. |
| PSL | ⏸ Deferred | Public Suffix List — used by curl for cookie-domain scoping. recon's cookie jar (sqlite-backed) does not yet consult PSL when deciding whether a cookie domain is too broad; cookies with a public-suffix domain attribute would be accepted. Not a security risk for typical scripted use. Revisit if a cookie scoping issue lands. |
| MultiSSL | ❌ Architectural N/A | Rust binaries pick one TLS backend; recon picks rustls (and BoringSSL with `--features impersonate`). Adding fallback (OpenSSL + rustls) would mean feature-flagged build matrices. Not a capability gap. |

### Curl flags shipped in recon

These are flags recon imports from curl's command-line vocabulary. Most
work identically; a few have subtly different semantics noted below.

| curl flag(s) | recon support | Notes |
|---|---|---|
| `-E / --client-cert`, `--client-key`, `--cert-type`, `--key-type`, `--pass` | ✅ 0.54.0 | mTLS via `reqwest::Identity::from_pem`. PEM-encoded client certs only; DER errors with a conversion recipe; encrypted PKCS#8 refused with an `openssl pkcs8` recipe (rustls has no engine concept). |
| `--cacert`, `--capath`, `--ca-native` | ✅ Shipped | Adds extra trust roots, or replaces with OS roots only. |
| `--crlfile` | ✅ 0.72.0 | PEM-encoded CRLs; multi-CRL bundles supported. Server certs in any loaded CRL are rejected at handshake. |
| `--tlsv1.2`, `--tlsv1.3` | ✅ Shipped | Pin a minimum TLS version. `--tlsv1.3` wins when both are set. |
| `-z / --time-cond` | ✅ Shipped | If-Modified-Since / If-Unmodified-Since from a timestamp string OR a file's mtime. Pairs with `--etag-compare` / `--etag-save` for ETag round-trips. |
| `-C / --continue-at` | ✅ Shipped | Resume a partial download (Range header + offset-aware writer). `-C -` auto-detects from the existing output file's size. |
| `--range` | ✅ Shipped | Byte-range request. Pairs with `--max-filesize` to abort oversized downloads. |
| `--retry`, `--retry-all-errors`, `--retry-connrefused`, `--retry-delay`, `--retry-max-time` | ✅ 0.64.0 | Exponential backoff with optional fixed delay. `--tries` (wget-style) added in 0.67.0 as an alternative spelling. |
| `-n / --netrc`, `--netrc-file`, `--netrc-optional` | ✅ 0.63.0 | Reads `~/.netrc` (or `$NETRC`) for `machine` / `login` / `password` records. |
| `--xattr` | ✅ Shipped | Writes URL + MIME type as extended attributes on the saved file (Unix). |
| `-K / --config` | ✅ Shipped | Curl-format config file (`#` / `;` comments, quoted strings, `--` flags one per line). Pre-clap argv expansion. |
| `--input-file / -i` | ✅ Shipped | Read a list of URLs from a file (one per line). |
| `--http1.0`, `--http1.1`, `--http2`, `--http2-prior-knowledge` | ✅ Shipped | Pin a HTTP version. H3 is deferred. |
| `-F / --form`, `--form-string`, `--form-escape` | ✅ Shipped | Multipart upload with file refs (`@path`), explicit `Content-Type`, `filename=` overrides. |
| `--oauth2-bearer` | ✅ Shipped | Bearer token shorthand for `-H 'Authorization: Bearer …'`. |
| `--limit-rate`, `--speed-limit`, `--speed-time` | ✅ Shipped | Throttle and slow-transfer abort. |
| `--interface` | ✅ Shipped | Bind outgoing socket to a local IP. Interface names (eth0, en0) supported on Linux/macOS via getifaddrs; Windows accepts IP literals only. |
| `--write-out` | ✅ Shipped | curl-style format string with `%{var}` substitution (status, time_*, url_effective, …). |

### Beyond curl — recon-only features

| Feature | Where | Notes |
|---|---|---|
| 🆕 TLS+H2 browser fingerprint impersonation | `--features impersonate` | Mimic Chrome / Firefox / Safari / Edge / mobile / OkHttp at the JA3 / JA4 / H2-SETTINGS level via BoringSSL + rquest. Curl has no equivalent (curl-impersonate is a separate fork project). See `recon --help impersonate`. |
| 🆕 Email-protection sweep | `--spf`, `--dmarc`, `--dkim`, `--mta-sts`, `--tls-rpt`, `--bimi`, plus `--cross-validate` | One-command audit of an org's email-auth posture, with cross-validation flags that surface SPF↔DMARC alignment issues. |
| 🆕 Multi-protocol probes | `--imap`, `--pop3`, `--smtp`, `--mqtt`, `--redis-probe`, `--memcached-probe`, `--ldap-probe`, `--rtsp-probe`, `--ntp-probe`, … | recon ships dedicated mode probes for ~20 protocols beyond HTTP, with capability discovery and per-protocol metrics. |
| 🆕 Document conversion | `--md-to-html`, `--md-to-pdf`, `--html-to-pdf` | Markdown → HTML / PDF and HTML → PDF with TOC, cover pages, page breaks, PDF Info-dict metadata (author / subject / keywords). Uses agent-browser as the headless-Chrome backend. |
| 🆕 Codec / crypto tools | `--hash`, `--encode`, `--decode`, `--encrypt`, `--decrypt`, `--compress`, `--decompress`, `--archive`, `--extract`, `--checkdigit*` | 10 hash algorithms, 20+ encodings, age + PGP encryption, 9 compression streams, multi-format archive support, 30+ check-digit algorithms (Luhn, ISBN-13, IBAN, BIC, Personnummer, EDRPOU, …). |
| 🆕 Barcode encode / decode | `--encode`, `--decode` over QR / DataMatrix / Aztec / PDF417 / MaxiCode / 1D | Round-trip 2D and 1D codes from PNG / JPEG / WebP / SVG. |
| 🆕 JWT round-trips | `--jwt-view`, `--jwt-sign`, `--jwt-validate` | Sign, inspect, and validate JWTs with HS / RS / ES / PS / EdDSA algorithms. |
| 🆕 Rhai script engine | `--script <file.rhai>` | Every CLI feature is also a script binding (`http()`, `dns()`, `ping()`, `tcp_connect()`, `mqtt_pub()`, …). Sticky-session `browser()` for stateful flows; threading; SQLite; clipboard. See [`script/README.md`](../script/README.md). |
| 🆕 Sample-data generators | `--sample <kind>`, `--sample-list` | Built-in generators for Lorem Ipsum, addresses, IBAN, UUID, dates, and more — handy for filling forms or seeding databases. |
| 🆕 Built-in document server | `--serve <dir>`, `--serve-tls`, `--serve-port` | One-shot static-file server with optional TLS for local testing. |
| 🆕 Topic-organised help | `recon --help <topic>` | Long-form topic pages (`tls`, `proxy`, `mqtt`, `jwt`, `impersonate`, `protocols`, …) instead of one massive `--help` dump. |
| 🆕 Curated examples | `recon --examples` | ~60 sections of runnable scenarios grouped by feature area. |
| 🆕 Flag listing | `recon --flags` | Curl-style alphabetical flag index with one-line summaries. |

## Glossary

Curl features not in the matrix are either trivially present via Rust
stdlib (stdin / stdout, file I/O, signal handling) or so curl-specific
(debug-mode, libcurl-only concepts, `--libcurl` codegen) that mapping
them to recon isn't meaningful.
