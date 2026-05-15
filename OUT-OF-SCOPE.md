# Out of Scope & Wishlist

A living list of items raised during design, implementation, or feature sweeps
that are either explicitly deferred, decided against, or noted as "maybe
later". Also doubles as a wishlist — items under "Waiting" are things worth
building once someone explicitly asks. Kept here so ideas don't disappear
into the black hole of spec files after each release.

Organized into four buckets by reason for non-inclusion. When an item ships,
remove it from this file and note the shipping version in the CHANGELOG
entry rather than leaving a crossed-out line here.

- **Waiting** — can be done; nobody's asked for it.
- **Deferred** — possible to implement; actively put off (scope/complexity
  trade-off or waiting on a concrete use case).
- **Not yet supported** — blocked by upstream / ecosystem maturity; may ship
  when the blocker clears.
- **Out of scope** — fundamentally can't be implemented, architecturally
  mismatched, or intentionally declined by policy.

---

## Waiting — can be done, not asked for

### Check digits

- **ASEAN / African / Middle Eastern tax IDs** — beyond the
  Latin-American + Australian + Mexican set shipped in 0.61.0. Add per
  concrete request.

### Encoding

- **PNG HRT** — 0.61.0 shipped HRT for ASCII + SVG output; PNG HRT is
  deferred pending font bundling. `ab_glyph` + a permissive TTF
  (~50–100 KB compiled) is the path; picking a font + rasterization
  positioning wasn't worth it in the original release.

---

## Deferred — put off, path is known

### Raw fingerprint overrides for `--impersonate` (0.77.0 → v0.78)

- **`--ja3 <STRING>`, `--ja4 <STRING>`, `--http2-fingerprint <STRING>`** —
  reserved in the CLI of 0.77.0 for forward-compatibility but error at
  runtime with a "deferred to v0.78" message. The original 0.77.0 plan
  promised raw fingerprint overrides alongside named profiles, but
  `rquest` 5.1.0 turned out to be a lower-level toolkit (`TlsConfig`
  builders for cipher list / sigalgs / curves / extension order) rather
  than a turnkey "set this JA3 string" library. JA3 strings don't capture
  sigalgs or extension order, so reconstructing a `TlsConfig` from a JA3
  is lossy and partial. JA4's cipher and extension components are
  SHA-256 truncations, fundamentally non-invertible. Each parser would
  be 100–200 lines of brittle TLS plumbing producing partial fingerprints.
  Path forward for v0.78: build a JA3-prefix → known-profile lookup
  table (covers the common "this Chrome version was captured" case),
  ship `--http2-fingerprint` as a real Akamai-format parser into
  `rquest::Http2Config` (the H2 layer is fully introspectable), and
  document `--ja3` / `--ja4` as best-effort with explicit limitations.
  Named `--impersonate <profile>` covers the captcha-testing use case
  in v1; revisit when a real captured-fingerprint case lands that
  named profiles can't reproduce.

### Client cert / custom CA bundle through `--impersonate` (0.77.0)

- **`--client-cert` / `--client-key` with `--impersonate`** — BoringSSL
  has its own mTLS API distinct from rustls, and rquest's `Identity`
  surface differs from reqwest's. v1 of the impersonate path errors
  cleanly when these flags are combined; mTLS with browser
  fingerprint impersonation is a follow-up release. Workaround: use
  the default rustls path for mTLS, accept the rustls-shaped
  fingerprint.
- **`--cacert` / `--capath` with `--impersonate`** — same story.
  rquest 5.1.0's emulation path uses the system root store only;
  user-supplied roots aren't plumbed in v1. Errors at runtime when
  combined with any impersonation flag.

### HTTP/3 browser fingerprint impersonation

- **HTTP/3 / QUIC impersonation** — rquest is HTTP/1.1 + H2 only.
  Chrome's H3 fingerprint would need a separate H3 client (`h3` crate
  + custom QUIC fingerprinting library + ALPN + 0-RTT settings).
  Out of scope for the captcha-server v1 use case, which targets
  HTTPS / H2 services. Revisit if a real H3-fingerprinting target
  appears and an h3-side impersonation crate becomes available.

### `--pinnedpubkey` and `--curves` (require use_preconfigured_tls migration)

- **`--pinnedpubkey` and `--curves`** — both require migrating recon's
  HTTP TLS plumbing from reqwest's high-level setters (`add_root_certificate`,
  `tls_built_in_root_certs`, `min_tls_version`, `max_tls_version`,
  `danger_accept_invalid_certs`, `identity`) onto a custom
  `rustls::ClientConfig` passed via `use_preconfigured_tls`. The migration
  is tractable (~80–120 LOC for a `build_rustls_client_config(args)`
  helper) but is its own focused effort separate from a single-flag
  plumb-through. When the migration happens, both flags ship together —
  `--pinnedpubkey` via a custom `ServerCertVerifier` that delegates to
  `WebPkiServerVerifier` and then checks SHA-256 of `ParsedCertificate::
  subject_public_key_info()`; `--curves` by overriding `kx_groups` on a
  cloned `CryptoProvider`. Note: P-521 (`secp521r1`) is unavailable under
  the ring backend; the implementation must error gracefully on that
  curve name when `rustls` features include only `ring`.

### curl flags — internal trade-offs, no upstream block

These were originally tracked as "Waiting" but the blocker is recon's
own architecture or scope budget, not upstream — moved here so the
intent is honest.

- **`--suppress-connect-headers`** — hide proxy CONNECT request /
  response from `-v` output. Architectural: recon doesn't render
  the raw proxy CONNECT exchange in verbose mode; no hook point
  exists without a significant low-level rewrite.
- **`--path-as-is`** — preserve `..` / `.` segments in URL paths.
  `reqwest::Url` normalises paths unconditionally; bypassing this
  requires switching to a raw request path, which conflicts with
  the `url::Url` plumbing throughout the codebase. Cost > value.
- **`--proxy-pass`** — shipped as deferred-with-warning in 0.73.0.
  reqwest 0.12's `Identity::from_pem` has no passphrase variant;
  the PKCS#12 path (`from_pkcs12_der`) accepts a password but is a
  different format. Will ship properly when proxy mTLS cert flags
  (`--proxy-cert` / `--proxy-key`) land and the passphrase path is
  clear — an internal sequencing decision.

### SMTP envelope parameters

- **`--mail-auth`** — lettre 0.11's `SmtpTransport::send(message)` builds
  the `MailParameter` vec internally; there is no external API to inject
  `MailParameter::Other { keyword: "AUTH", value: addr }`. Forking
  lettre's send path is out of scope. Revisit if lettre 0.12 exposes an
  envelope-parameter knob, or if recon switches to a lower-level SMTP
  client. Currently accepted at CLI but emits a runtime warning.

### Check digits

- **Partial / prefix verification** — "is this a plausible
  IIN / EDRPOU prefix?" for inputs shorter than the full length. UX
  pattern rather than an algorithm; no clear flag-shape design.
- **Registrant-aware ISBN-13 hyphenation** — needs the ISBN
  registrant-prefix lookup table (large, maintained upstream).
  Current simple 3-1-2-5-1 fallback is fine for most uses.
- **VIES live lookup** — online EU VAT validation against the
  official service. Requires internet request and would be
  architecturally distinct from the offline check-digit math.

### Encoding

- **Logo overlay / colour customisation** on QR codes — fiddly UX
  surface; postpone until concrete demand shapes the flag set.
- **Multi-code image composition** (several codes on one canvas) —
  same reason.

### HTTP / curl compatibility

- **`--cert-status`** — OCSP-staple check during the TLS handshake.
  Requires a custom `rustls::ServerCertVerifier` that inspects the
  staple and falls back to a network OCSP responder. Niche in
  practice (most deployments disable OCSP entirely in favour of
  short-lived certs). Revisit if a concrete need appears.
- **DER client-cert / client-key formats + encrypted PKCS#8** —
  Non-PEM client-cert formats and encrypted-at-rest keys. Currently
  rejected at load time with `openssl` conversion recipes.
  In-process parsing would add the `pkcs8` crate and a DER→rustls
  shim; shipping conversion-via-shell is the right trade-off until
  there's concrete demand.
- **`--anyauth`** — auto-select auth scheme. Security-risky
  (credential probing) and niche.
- **`-w` `%{output{filename}}`** — redirect part of output to a
  specific file. Niche.
- **`-: --next`** — separator between URL-specific flag sets in a
  single invocation. Substantial clap restructure; a future
  release would benefit if `--input-file` users want different
  flag profiles per URL.
- **`-Z, --parallel` cluster** — parallel transfers. Depends on a
  proper work-queue + per-stream progress aggregation. Useful with
  `--input-file` but requires async runtime work.
- **`--variable` / `--expand-*`** — curl's templating language for
  flag values. Substantial parser; interacts with clap's positional
  handling. Low value until multiple flag values would benefit.
- **`--libcurl`** — emit a C source file that reproduces the
  invocation via libcurl. Niche; complex emitter.

### curl-parity — deferred (0.50.0 sweep)

Tracked alongside `docs/curl-parity-matrix.md` for day-to-day user
reference.

- **Kerberos / SPNEGO / GSS-API** — all three share the
  `libgssapi-krb5` dependency on Linux/macOS and Windows SSPI on
  Windows. Three FFI integrations is a significant cross-platform
  maintenance tax for a diagnostic tool. Users needing enterprise
  auth tend to have curl installed for exactly these cases. Revisit
  if concrete demand appears.
- **NTLM** — Windows-only via the `sspi` crate's FFI. Niche in
  modern APIs; documented as a curl gap recon doesn't try to paper
  over.
- **alt-svc** — RFC 7838 Alt-Svc header cache. `reqwest` has zero
  primitives; hand-rolling a spec-compliant cache + file persistence
  is ~300 lines. Low practical value for a one-shot CLI (the cache
  would be populated and discarded on every run). Revisit if
  IPv6+HTTP/3-adoption changes the calculus.

### Document conversions

- **typst-based md→PDF alternative** — Chrome-free path for
  markdown → PDF via a hand-rolled md→typst translator + the
  `typst` crate embedded. Would add ~15–25 MB to the release binary
  and require non-trivial translator logic. Revisit if users
  explicitly ask for Chrome-free PDF generation.
- **Custom page sizes / margins / orientations** — agent-browser's
  `pdf` subcommand's flag surface dictates what's feasible. Punt
  until real demand shapes the knobs.

### Script engine

- **ICMP raw-socket send/recv primitives** — `ping()` already covers
  reachability checks; arbitrary ICMP type/code send + recv is
  niche. Requires raw sockets (`CAP_NET_RAW` on Linux, root on
  macOS for non-DGRAM types). Revisit when users ask for specific
  traffic-generation or monitoring use cases.

- **Auto-loading `.env` at script start** — 0.76.0 shipped explicit
  `load_dotenv(path)` only. Auto-loading `<script_dir>/.env` (and
  optionally `<script_dir>/.env.<script_name>`) before script
  execution would save users a two-line preamble but introduces
  surprising behaviour: scripts run from arbitrary directories
  would silently inherit values from any `.env` file sitting next
  to them, and scripts that *don't* want dotenv loading would have
  no opt-out short of moving to a clean directory. Explicit-only is
  the safer default; revisit if multiple users independently ask for
  the convention. Composition is one line:
  `load_dotenv(script_dir + "/.env");`.

### wget recursive / mirror cluster

The whole cluster is one feature area; shipping a subset leaves the
behaviour feeling half-done. ~800–1200 LOC + HTML parser + robots +
canonicalisation. Own spec + plan when someone asks. The 0.67.0
release picked up the standalone-feasible bits as long-form flags
(`--wait` for politeness, `--tries` for retry-count override, and
`--accept` / `--reject` as flat filename-suffix filters); the
recursive-engine pieces below remain deferred.

- `-r, --recursive` + `-l, --level <N>` — recursive fetch.
- `-m, --mirror` — `-r -N -l inf --no-remove-listing` alias.
- `-p, --page-requisites` — single-page offline snapshot.
- `-k, --convert-links` — rewrite absolute links for local viewing.
- `-D` / `-H` / `--exclude-domains` host filters.
- `-np, --no-parent`, `--cut-dirs`, `-Q, --quota`.
- `-b, --background` — shell already provides this.

`--accept` / `--reject` already ship as flat filters (0.67.0); the
recursive variant (filters applied to discovered links during a
crawl) is the part that's still deferred.

---

## Not yet supported — blocked on upstream / ecosystem

### Check digits

- **Albania NIPT** — check letter algorithm is not publicly
  documented. `stdnum-js` explicitly marks it as "not understood".
  Ship if authoritative docs emerge.
- **Bosnia and Herzegovina JIB** — no check digit algorithm found
  in any accessible source; no `python-stdnum` or `stdnum-js`
  module exists.
- **Kosovo NUI** — newer system (~2019); no public algorithm
  documentation; no stdnum module.

### Encoding

- **MaxiCode encoding** — no pure-Rust encoder exists. rxing (ZXing
  port) decodes MaxiCode but ships no encoder. Revisit when someone
  writes one or if shelling out to `dmtx-utils` / `zint` becomes
  acceptable. (Decoding already works via `--decode` and `rxing`.)

### Encryption

Still deferred after 0.46.0's PGP / rekey landing:

- **Hardware-backed keys** (`age-plugin-*`). Requires either an
  age-crate bump that exposes plugin hooks (0.11 doesn't), or
  re-implementing age's plugin-protocol state machine ourselves.
  GPG smartcards work naturally via the `gpg` subprocess when the
  user's keyring is already configured — no recon work needed there.
- **Mixed recipient-and-passphrase in one invocation**. age 0.11's
  `Encryptor::with_recipients` rejects `scrypt::Recipient`
  alongside X25519 recipients. Producing a mixed-stanza header
  would require bypassing age's Encryptor and writing custom
  stanzas. Revisit if age 0.12+ relaxes the constraint.

### HTTP / curl compatibility — blocked on reqwest / rustls

These were declared accepted-and-CLI-only during the Waiting-arc
because reqwest 0.12 / rustls 0.23 don't expose the necessary
primitive. They'll start taking effect when the upstream lands the
hook (or when recon bypasses reqwest for a direct hyper stack).

- **`--http1.0`** — reqwest 0.12 has `http1_only()` (disable HTTP/2)
  but no primitive to pin 1.0 specifically. Revisit if reqwest gains
  `http1_0_only()`.
- **`--http0.9`** — same story; reqwest doesn't expose 0.9 mode.
- **`--http3`, `--http3-only`** — reqwest's QUIC feature disabled
  in recon's build. Revisit if we opt into the feature + `quinn`.
- **`--tlsv1`, `--tlsv1.0`, `--tlsv1.1`** — rustls dropped TLS < 1.2
  support entirely. Architectural under rustls.
- **`--tcp-fastopen`** — reqwest doesn't expose setsockopt hooks;
  platform-specific.
- **`--local-port <RANGE>`** — reqwest has no source-port selection.
- **`--happy-eyeballs-timeout-ms`** — reqwest 0.12 has built-in HE
  but doesn't expose the timeout knob.
- **`--no-alpn`, `--no-npn`, `--no-sessionid`** — reqwest / rustls
  don't expose these toggles.
- **`--false-start`** — rustls doesn't implement TLS False Start.
- **`--digest`** — reqwest has no HTTP Digest auth; would require a
  custom 401-challenge retry layer or a `reqwest-digest-auth`
  middleware crate that doesn't exist at a stable version yet.
- **`--trace`, `--trace-ascii`, `--trace-config`, `--trace-ids`,
  `--trace-time`** — require reqwest connector hook (same blocker
  as `-w` phase timings).
- **`-w` phase timings** — `time_namelookup`, `time_connect`,
  `time_appconnect`, `time_pretransfer` currently render as
  `0.000000`. reqwest 0.12's blocking client wraps an async hyper
  client internally; cleanly hooking a custom connector requires
  bypassing reqwest or waiting for upstream connector-instrumentation.
- **`--dns-interface`** — bind DNS queries to a named interface.
  Accepted at the CLI but not yet plumbed; hickory 0.24's
  `NameServerConfig::bind_addr` takes a SocketAddr (IP + port), not
  an interface name. Use `--dns-ipv4-addr` / `--dns-ipv6-addr` with
  the literal address as a workaround.
- **DoH** — `--doh-url`, `--doh-insecure`, `--doh-cert-status`.
  hickory-resolver has no DoH yet; would need hickory 0.25 or a
  side-car DoH client.
- **`--tr-encoding`** — request Transfer-Encoding compression.
  reqwest has no opt-out from its own TE header handling; no API
  surface to toggle this flag's behaviour.

### Per-protocol plumb-through (remaining stubs → real)

0.71.0 shipped the bulk of the 0.65.0 FTP/SSH stubs. What remains is
blocked by upstream crate API gaps:

- **FTP**: `--ftp-method` — suppaftp has no CWD-strategy selector;
  no API surface to choose between CWD+RETR vs path-in-RETR.
- **FTP**: `--ftp-account`, `--ftp-alternative-to-user`,
  `-P --ftp-port`, `--ftp-pret`, `--ftp-ssl-ccc`,
  `--ftp-ssl-ccc-mode`, `--ftp-ssl-control` — suppaftp 6 has no
  surface for ACCT, alternative-user, active-mode port, PRET, or
  CCC. Lock-step group; ships when suppaftp grows the APIs.
- **FTP**: `--ftp-create-dirs` — needs an upload path (STOR/APPE)
  that doesn't exist yet; blocked by the same gap as `--append`.
- **FTP / SMTP**: `--ssl`, `--ssl-reqd` — soft / hard TLS-required.
  Only meaningful once the `--ftp-ssl-control` family above lands.
- **SMTP**: `--mail-rcpt-allowfails` — lettre's send loop
  short-circuits on RCPT failure; no partial-success API.
- **SMTP**: `--sasl-ir` — lettre bakes SASL IR per-mechanism
  unconditionally (PLAIN/XOAUTH2 always on, LOGIN always off);
  no toggle to expose.
- **IMAP / POP3**: `--login-options` — imap 3-alpha crate has no
  parameter-passing surface on LOGIN/AUTHENTICATE.
- **IMAP / POP3**: `--sasl-authzid` — neither the imap crate nor
  recon's hand-rolled POP3 probe expose an authzid parameter.
- **Telnet**: `--telnet-option` — `src/telnet.rs` is a TCP banner
  reader with no IAC negotiation; wiring this flag requires building
  telnet IAC infrastructure from scratch (genuine feature work, not
  stub-plumbing).
- **Upload**: `-a / --append` for FTP / SFTP — needs FTP STOR/APPE
  swap + sftp open-flags; no upload path exists yet.

### Per-flag plumb-through (0.66.0 stubs → real)

The proxy + TLS-tuning flags shipped at CLI but most need a custom
rustls `ServerCertVerifier` or cipher-list parser. `--crlfile`,
`--proxy-capath`, and `--proxy-ca-native` shipped in 0.72.0.
Remaining:

- **Proxy**: `--preproxy`, `--proxy-header`, `--proxy-http2`,
  `--proxytunnel` — blocked on reqwest 0.12 not exposing chained-proxy /
  CONNECT-header / proxy HTTP/2-force / force-tunnel API.
- **Proxy TLS**: `--proxy-crlfile`, `--proxy-pinnedpubkey` — require
  the same `use_preconfigured_tls` migration as the non-proxy versions,
  plus per-proxy TLS not yet exposed by reqwest 0.12.
- **Proxy ciphers**: `--proxy-ciphers`, `--proxy-tls13-ciphers` —
  rustls 0.23 has no public cipher-list API; upstream-blocked.
- **TLS tuning**: `--ciphers`, `--tls13-ciphers` — rustls 0.23 has no
  public cipher-list API; upstream-blocked.

(`--pinnedpubkey` and `--curves` also live here in spirit, but the
remaining work is the `use_preconfigured_tls` migration inside recon
rather than an upstream block — tracked under Deferred.)

### Document conversions

- **Other markup → PDF** — reStructuredText, AsciiDoc, Org. Each
  would need its own parser crate. No production-ready pure-Rust
  parsers for any of these formats as of mid-2025. Ships if a viable
  crate appears.
  (PDF metadata beyond title — author, subject, keywords — shipped in 0.74.0.)
- **Pure-Rust HTML+CSS → PDF renderer** — `servo`/`blitz` exist but
  aren't packaged as an embeddable crate yet. `typst` is pure-Rust
  and has `#outline()` for linkable TOC, but does NOT accept HTML
  as input (its HTML support is output-only). Revisit if either
  path matures.

### MQTT

- **Dual rustls majors in the binary** — rumqttc 0.24 pins rustls
  0.22; recon's HTTPS stack uses rustls 0.23. Both coexist
  (~300 KB overhead). Revisit when rumqttc bumps to rustls 0.23.

### Protocol scope

- **SMB / SMBS** — pending a mature pure-Rust SMB client crate. The
  `smb` crate is at 0.5.x and low-volume; `pavao` requires system
  libsmbclient (unacceptable for a cross-platform binary). Revisit
  when the ecosystem matures. (FTP, TFTP, GOPHER, POP3, IMAP, SFTP
  and many others have shipped as protocol probes — this note tracks
  only the still-excluded remainder.)

### Rust edition

- **`std::env::set_var` becomes `unsafe` in edition 2024.** recon's
  Cargo.toml currently pins `edition = "2021"`. The 0.76.0
  `load_dotenv` binding calls `std::env::set_var` to populate the
  process environment from parsed `.env` files. In edition 2024 this
  function gains `unsafe` because concurrent `getenv`/`setenv` calls
  are technically unsound on POSIX. The current code documents the
  constraint (call `load_dotenv` at the top of the script before
  spawning threads via `thread::spawn`) but doesn't enforce it.
  Revisit when migrating to edition 2024: either wrap the call in
  `unsafe { ... }` with the existing single-threaded-startup
  invariant documented as the safety justification, or switch to a
  thread-local env-var store that doesn't need to mutate libc's
  global state.

---

## Out of scope — can't / won't

### Security boundary

- **CVV / CVC validation** — the 3-4 digit card security code is
  cryptographically generated from PAN + expiry + issuer's secret
  CVK. Impossible to verify without access to the card-issuer's key
  material.
- **Mass scanning / credential stuffing / detection evasion
  tooling** — outside the scope of a reconnaissance and verification
  tool, regardless of how plausibly a feature could be implemented.
- **`--random-wait`** (wget) — anti-bot-detection connotations
  conflict with recon's stance.

### Feature mismatch

- **EIN, SSN, postal codes, phone numbers** — these have format
  rules but no algorithmic check digit. A format-validation feature
  is a different tool.

### Architectural mismatch

- **MultiSSL** — curl can ship with multiple TLS backends (OpenSSL +
  Schannel + NSS + …). Rust binaries pick one; recon picks rustls.
  Not a coverage gap; recon deliberately picks one backend.
- **`--engine`** — OpenSSL crypto engine selection. N/A under rustls.
- **CLI server flags** (`recon --listen 0.0.0.0:8080`) — server
  workflows are always multi-step (accept → per-conn handler);
  scripts are the right layer. Quick HTTP serving is already covered
  by the pre-built `recon --serve`.
- **Netscape-format cookie file** (`--cookie <file>` and
  `--cookie-jar <file>` in Netscape format). recon's `.db`
  cookiejar model is intentionally different; there's no path where
  supporting both makes sense.
- **`-w` variables outside the 22-variable subset** —
  `num_connects`, `proxy_ssl_verify_result`, `http_connect`,
  FTP-era fields. Unreachable or meaningless via reqwest; listing
  them would imply support we can't give.
- **`-g, --globoff`** — recon doesn't glob URLs; flag would be a
  no-op. Document the non-feature rather than ship a stub.
- **`--ssl-allow-beast`, `--ssl-auto-client-cert`, `--ssl-no-revoke`,
  `--ssl-revoke-best-effort`** — Windows Schannel-only knobs;
  rustls doesn't expose equivalents.
- **`--proxy-tlsauthtype`, `--proxy-tlspassword`,
  `--proxy-tlsuser`, `--proxy-tlsv1`** — TLS-SRP and forced TLS 1.0
  for proxy. Not supported by rustls.

### Legacy / deprecated curl flags

- **`--metalink`** — deprecated even in curl.
- **`--egd-file`** — EGD randomness source (legacy Unix).
- **`--manual`** — curl's full manual. recon has `--examples` +
  `docs/MANUAL.md` (fuller coverage anyway).
- **`--use-ascii / -B`** — legacy FTP ASCII mode. Modern FTP servers
  default to binary; nobody types this anymore.
- **`--sslv2`, `--sslv3`** — rustls dropped these protocol versions.
- **`--data-ascii`** — legacy alias of `-d` from a time when curl
  had a binary alternative; both are the same thing today.

---

## Notes on process

- When a new idea is parked during a brainstorm, add it here under
  the most honest of the four buckets + a one-line reason.
- When an item here ships, remove it and note "shipped in x.y.z" in
  the CHANGELOG entry rather than leaving a crossed-out line here.
- Items can move between buckets as the world changes. When ecosystem
  maturity unblocks a "Not yet supported" item it graduates to
  "Waiting"; when a "Waiting" item picks up enough scope weight to
  merit punting, it moves to "Deferred".
- **Bucket discipline.** "Waiting" must mean "no upstream block, no
  internal trade-off — just nobody's asked yet." If you find yourself
  writing "blocked on X" inside a Waiting entry, move it. The 0.78.0
  re-bucketing sweep moved ~20 items out of Waiting because they were
  in fact blocked or deferred — the bucket had drifted into a generic
  todo list.
- This file is deliberately not versioned in `CHANGELOG.md` — it's
  a working-notes file, not a release artifact.
- **Audit cadence**: at the end of every multi-release arc, walk
  this file against the shipped flag set (e.g. `recon --flags |
  awk '{print $1}'`) and remove items that landed during the arc.
  The 0.66.2 sweep that produced this revision is the canonical
  example of how stale this gets without one.
