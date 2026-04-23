# Changelog

All notable changes to recon are recorded here. Format based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

For pre-0.4.1 design context and architectural notes, see [HISTORY.md](HISTORY.md).

## [Unreleased]

## [0.54.0] - 2026-04-24

### Added

Client-certificate package — closes the `--key-type` trap documented in
`OUT-OF-SCOPE.md` by shipping it as a full mTLS bundle rather than a
single flag that means nothing without its partners.

- **`-E, --client-cert <PATH>`** — PEM-encoded client certificate.
  Accepts combined PEMs (cert + key in one file) or cert-only PEMs
  paired with `--client-key`.
- **`--client-key <PATH>`** (alias `--key`) — separate PEM key file.
- **`--cert-type <PEM|DER>`** — cert format. PEM is honored; DER is
  accepted at parse time and errors with a `openssl` conversion recipe
  (rustls has no DER client-cert path today).
- **`--key-type <PEM|DER|ENG>`** — key format. `ENG` (OpenSSL engines)
  errors immediately; `DER` errors with a conversion recipe; `PEM`
  flows through.
- **`--pass <PASS>`** — passphrase placeholder. Encrypted PKCS#8 keys
  are detected and refused with a clear message pointing at
  `openssl pkcs8`; real in-process decryption is a follow-up.
- **Script opts-map keys**: `client_cert`, `client_key`, `cert_type`,
  `key_type`, `pass`.
- **`recon --version` Features token**: `client-cert`.

### Technical

New `src/client_cert.rs` owns the loader. `reqwest::Identity::from_pem`
accepts the same combined form regardless of whether the caller started
with one file or two, so both paths funnel through a single byte-level
concat before handing off to rustls. No new crate dependencies.

### Out of scope

Removed from `OUT-OF-SCOPE.md`: `--key-type` (the package-deal trap).
Remaining entries around TLS: `--cert-status` (OCSP staple), `--engine`
(rustls), DER cert/key formats (deferred with clear error), encrypted
PKCS#8 in-process decryption (deferred with clear error).

## [0.53.0] - 2026-04-24

### Added

Quick-wins bundle kicking off the post-curl-parity expansion roadmap.

- **`--compare <A> <B>`** — diff two sources (URL, local path, or `-`
  for stdin). HTTP(S) sides flow through the normal request pipeline
  and honor every existing flag (`-H`, `-u`, `-L`, `-k`, cookies,
  proxy, HSTS). Exit code follows GNU `diff` convention: `0` identical,
  `1` differ, `2+` source-load error. Formats: `unified` (default),
  `summary` (one-liner), `sxs` (side-by-side). Context-line count
  tunable via `--compare-context`. Binary content is detected by a
  NUL-byte probe in the first 8 KiB and reported as a byte-count delta
  rather than attempting a line diff.
- **Script `compare(a, b)`** — in-memory counterpart returning a map
  with `identical`, `added`, `removed`, `binary`, `a_bytes`, `b_bytes`,
  `diff`. Accepts `Blob` or `&str` on both sides.
- **Script raw-print helpers** — `print_raw(s|blob)` writes to stdout
  without a trailing newline and flushes; `eprint(s)` writes to stderr
  with newline; `eprint_raw(s|blob)` is the stderr no-newline variant;
  `flush()` forces an explicit stdout flush. Useful for progress
  displays, line protocols, and byte-precise output.
- **Streaming file I/O in scripts** — `file_open(path, mode)` returns a
  `FileHandle` (an `Arc<Mutex<File>>` newtype), plus `file_read(h, n)`,
  `file_read_all(h)`, `file_write(h, data)`, `file_seek(h, pos, whence)`,
  `file_tell(h)`, `file_flush(h)`, `file_close(h)`. Whole-file
  convenience helpers land alongside: `file_write_all`, `file_append_all`,
  `file_exists`, `file_size`, `file_delete`. Handle type is deliberately
  `Send + Sync` so it survives the rhai `sync`-feature flip planned for
  0.56.0.
- **`--qr-level <L|M|Q|H>`** — QR error-correction level (default `M`).
  Script callers pass `#{ qr_level: "H" }` via the four-arg
  `encode::encode(fmt, data, output, opts)` overload.
- **New `--version` Features tokens**: `compare`.

### Changed

- `cli::Args` now derives `Clone` so subcommand code paths (starting
  with `compare::run`) can mutate a per-call copy without touching the
  main Args.

## [0.52.2] - 2026-04-23

### Fixed

- `recon --version` now reports the actual ship date of the current
  build (`Release-Date: 2026-04-23`). Previously stuck at 2026-04-20
  across the 0.50.0 / 0.51.0 / 0.52.0 / 0.52.1 releases. CLAUDE.md now
  requires `RELEASE_DATE` in `src/version.rs` to be bumped alongside
  every `Cargo.toml` version change.

## [0.52.1] - 2026-04-23

### Fixed

- `recon --version` Features list now includes the always-on and shipped
  curl-parity tokens that were missing from the banner: `AsynchDNS`,
  `HSTS`, `Largefile`, `libz`, `threadsafe`, `UnixSockets`. Banner-only
  change — capabilities were already present, just not advertised.

## [0.52.0] - 2026-04-23

### Added

Third and final curl-parity phase-6 release. Closes the `--hsts` gap.

- **`--hsts <PATH>`** — persistent HSTS (HTTP Strict Transport Security) cache. Two-way integration:
  - **Request side**: load the cache; if the target URL is `http://` and the hostname has a non-expired entry (exact match or subdomain match under `includeSubDomains`), upgrade the URL to `https://` before sending. A verbose line announces the upgrade (suppressed by `-s`).
  - **Response side**: parse `Strict-Transport-Security` headers from `https://` responses, update the cache (`max-age` sets expiry, `max-age=0` removes, `includeSubDomains` tracked per entry), save atomically via `tempfile::NamedTempFile::persist`.
- **File format** matches curl's plain-text TSV (one entry per line: `host expires_unix`, leading `.` = includeSubDomains). Cross-compatible; you can share the file with curl.
- **Missing cache file** is silently treated as empty — first-run UX.
- **Script binding**: `http(url, opts)` opts gain `hsts: "/path/to/cache"`. Same semantics as the CLI flag.
- **`recon --help hsts`** (alias `strict-transport-security`) with file-format reference + examples.
- **`recon --examples`** new `HSTS (0.52.0)` section with 3 blocks.
- **`script/hsts.rhai`** under the existing "Routing" category.
- **`docs/curl-parity-matrix.md`** updated — HSTS moves from "planned" to "shipped".

### Rationale

`reqwest` has zero HSTS primitives; hand-rolled a ~300-line store (parse / match / update / save) in `src/hsts.rs`. File format chosen for curl compatibility: exact-match or leading-dot-subdomain entries with a Unix-epoch expiry column. `update_from_sts_header` returns true when the store changed, keeping the save-on-update path idempotent.

### Notes

- HSTS preload list is **not** bundled. Only entries added via an actual server's STS header are honored. Users who want preload-list behaviour should browse with a conventional browser.
- `--insecure` still works alongside `--hsts`: HSTS upgrades `http://` to `https://`, but `-k` still disables cert verification after the upgrade. Useful for testing; risky in production.
- HSTS updates only happen on `https://` responses (STS directives from `http://` are non-authoritative per RFC 6797).

## [0.51.0] - 2026-04-23

### Added

Second of three curl-parity phase-6 releases. Unix-domain-socket support for Docker API / systemd-activated services / Kubernetes kubelet diagnostics.

- **`--unix-socket <PATH>`** — route the HTTP request over a Unix-domain socket instead of TCP. Target URL's host + path are preserved; transport changes. URL grammar tolerant: `http://localhost/path`, `https://api/v1/info` (host-only; no TLS), or `/v1.40/version` (path-only; Host defaults to `localhost`).
- **Script binding**: `http(url, opts)` opts gain `unix_socket: "/path"`. When set, routes through `src/unix_socket.rs` and returns a Map identical to a normal `http()` response (with `charset: ()` since UDS payloads aren't decoded).
- **`recon --help unix-socket`** (aliases `unixsocket`, `uds`) with URL grammar, supported flags, common sockets.
- **`recon --examples`** new `UNIX SOCKETS (0.51.0)` section with 3 blocks.
- **`script/unix-socket.rhai`** under the existing "Routing" category.
- **`docs/curl-parity-matrix.md`** updated — UnixSockets moves from "planned" to "shipped".

### Rationale

reqwest's blocking client has no UDS support, and the full async-inside-sync stack (hyper + tokio + custom connector + duplicating reqwest's feature matrix) was overkill for one-shot diagnostic requests. Hand-rolled a minimal HTTP/1.1 client over `std::os::unix::net::UnixStream`: ~350 lines including tests. Scope is deliberately narrow — no HTTP/2, no TLS, no redirects, no chunked-decoding — matching what users actually reach for UDS for: Docker API, systemd sockets, kubelet endpoints.

### Notes

- Abstract-socket namespace (`@`-prefixed or NUL-prefixed names on Linux) deferred; path-based sockets cover the real-world use case.
- `--data-urlencode` not plumbed for UDS; use `-d` / `--json` instead (UDS APIs almost always want JSON anyway).

## [0.50.0] - 2026-04-23

### Added

First of three planned curl-parity phase-6 releases. Proxy support was a genuine capability gap — recon shipped zero proxy flags prior to 0.50.0.

- **`-x, --proxy <URL>`** — route HTTP(S) requests through a proxy. Scheme selects the type: `http://` (plain HTTP proxy), `https://` (TLS-to-proxy), `socks5://` (SOCKS5 with server-side DNS), `socks5h://` (SOCKS5 with client-side DNS). Env-var precedence matches curl: `$HTTPS_PROXY` (or `$https_proxy`) for https:// targets, `$HTTP_PROXY` (or `$http_proxy`) for http://, `$ALL_PROXY` (or `$all_proxy`) as fallback. CLI flag always wins over env.
- **`-U, --proxy-user <USER:PASS>`** — Basic-auth credentials against the proxy. Overrides URL userinfo.
- **`--noproxy <LIST>`** — comma-separated bypass list. Matches curl's semantics: exact hostname match, leading-dot for subdomain match (`.internal`), `*` to bypass everything. Falls back to `$NO_PROXY` / `$no_proxy`.
- **`--proxy-insecure`** — skip TLS verification on the connection to an `https://` proxy.
- **`--proxy-cacert <PATH>`** — additional PEM root for the https:// proxy connection. Trust-additive; because reqwest 0.12 applies CA bundles globally, this root also applies to the origin request.
- **Script binding**: `http(url, opts)` opts gain `proxy`, `proxy_user`, `noproxy`, `proxy_insecure`, `proxy_cacert` — same semantics as the CLI flags.
- **`recon --help proxy`** (alias `proxies`) — flag reference + env-var precedence table + noproxy grammar.
- **`recon --examples`** — new `PROXY (0.50.0)` section with 5 blocks.
- **`script/proxy.rhai`** — example under a new "Routing" category in `script/README.md`.
- **`docs/curl-parity-matrix.md`** (new) — quick-reference table of recon's coverage of curl's `--version` features: what's shipped, what's always-on via Rust/rustls, what's architecturally N/A, what's deferred.

### Changed

- **`recon --version`** now sorts `Protocols:` and `Features:` lists alphanumerically (case-insensitive) so the output reads like curl's. Also backfills the protocol list with everything shipped since 0.44.0 (ftp/ftps, sftp, tftp, gopher/gophers, pop3/pop3s, imap/imaps, ipfs/ipns, smtp/smtps) and adds build-feature tokens for the newer capabilities (charset, MQTT5, DKIM-signing, PGP-shellout, SOCKS5, etc.).
- `Cargo.toml` `reqwest` gains the `socks` feature (enables `socks5://` / `socks5h://` proxy URLs). Version 0.49.0 → 0.50.0.
- `src/client.rs::execute` plumbs `proxy::build_proxy_from_args` + `apply_proxy_tls` onto the `ClientBuilder`.

### OUT-OF-SCOPE.md additions

Four items from the 13-item curl-parity wishlist deferred with rationale:

- **Kerberos / SPNEGO / GSS-API** — cross-platform libgssapi FFI tax.
- **NTLM** — Windows-only sspi FFI.
- **alt-svc** — low practical value for a one-shot CLI.
- **MultiSSL** — architectural mismatch (Rust picks one backend).

The remaining 5 wishlist items (AsynchDNS, Largefile, libz, threadsafe, HTTPS-proxy primitives) were already present — documented in `docs/curl-parity-matrix.md`.

## [0.49.0] - 2026-04-23

### Added

Third and final protocol-coverage release in the 0.47.0–0.49.0 arc.

- **`ipfs://CID[/path]` and `ipns://NAME[/path]` URL schemes** — rewritten to `<gateway>/ipfs/CID[/path]` or `<gateway>/ipns/NAME[/path]` and dispatched through the existing HTTP pipeline. Every HTTP flag (`-H`, `-o`, `-k`, `--compressed`, `--output-charset`, etc.) applies to the rewritten request unchanged. Default gateway `https://ipfs.io`.
- **`--ipfs-gateway <URL>`** (help_heading = "HTTP Request") — override the default gateway. Also read from `$RECON_IPFS_GATEWAY`. Trailing slashes tolerated. Point at `http://127.0.0.1:8080` to use a local Kubo / IPFS-Desktop node for resolution.
- **Script binding `ipfs_url(url [, #{gateway}])`** — returns the gateway URL for a given `ipfs://` / `ipns://` address without fetching. Scripts compose with `http()` for retrieval. Throws on non-IPFS URLs.
- **Help**: `recon --help ipfs` (alias `ipns`) with URL grammar + gateway configuration. Added to `topic_keys()` and `TOPIC_PROTOCOLS`.
- **`recon --examples`**: new `IPFS / IPNS (0.49.0)` section.
- **Example script**: `script/ipfs.rhai` under a new "Content addressing" category in `script/README.md`.

### Changed

- `src/main.rs` rewrites `args.url` / `args.url_flag` in place before URL dispatch when either starts with `ipfs://` or `ipns://`. Clean separation — no new dispatch branch.

### Rationale

No new pure-Rust IPFS crate dep. The `rust-ipfs` alpha has a large dep tree and requires a local node or libp2p peer discovery; HTTP gateways are how the IPFS ecosystem actually serves content today. Revisit only if a mature native-protocol client emerges and a user asks.

## [0.48.0] - 2026-04-23

### Added

Two new URL-scheme probes covering the mail-retrieval protocols deferred at 0.47.0:

- **`pop3://` / `pop3s://`** — hand-rolled over TCP + rustls (mirroring the SMTP probe pattern). URL path grammar matches curl: empty path runs a capability-only probe (CAPA + STAT when authed); numeric path N retrieves message N (RETR). `--stls` upgrades a `pop3://` connection to TLS via the STLS command after CAPA.
- **`imap://` / `imaps://`** — uses the `imap = "3.0.0-alpha.15"` crate. Path grammar matches curl: empty path → CAPABILITY + LIST; `/MAILBOX` → EXAMINE that mailbox + report EXISTS/RECENT; `/MAILBOX;UID=N` → FETCH UID N body. `--imap-peek` uses `BODY.PEEK[]` so the server doesn't flip the `\Seen` flag.
- **Script bindings**: `pop3(url [, opts])` and `imap(url [, opts])`. Each returns a Map with the same shape as the CLI output.
- **New CLI flags** (help_heading = "Mail Retrieval"): `--stls`, `--imap-peek`.
- **Help**: `recon --help pop3`, `--help imap`. Added to `topic_keys()` and TOPIC_PROTOCOLS.
- **`recon --examples`**: new `MAIL RETRIEVAL (0.48.0)` section with 6 blocks.
- **Example scripts**: `script/pop3.rhai`, `script/imap.rhai` under a new "Mail retrieval" category in `script/README.md`.

### Changed

- `Cargo.toml` adds `imap = "3.0.0-alpha.15"` with `rustls-tls` feature. Version 0.47.0 → 0.48.0.
- `src/main.rs` URL dispatch gains 4 new branches (`pop3://`, `pop3s://`, `imap://`, `imaps://`).

### Known limitations

- IMAP `--insecure` (skip TLS verification) is not wired through the imap 3 alpha's ClientBuilder in this release; tracked as a follow-up when a custom-verifier hook lands upstream.

## [0.47.0] - 2026-04-23

### Added

Four new URL-scheme protocol probes covering the curl file-transfer family (minus SMB):

- **`ftp://` / `ftps://`** — probe + directory listing + file retrieval via `suppaftp`. Anonymous by default; URL userinfo / `-u user:pass` for authenticated sessions. Explicit AUTH TLS on `ftps://`; passive mode by default (`--ftp-active` for PORT / active). `--ftps-implicit` accepted but currently warns and falls back to explicit AUTH TLS. Path semantics match curl: trailing `/` → list, no trailing slash → retrieve.
- **`sftp://`** — SSH-backed file transfer via `ssh2::Sftp` (already linked). Same auth scaffolding as `scp://` and `ssh://` (`--ssh-key`, host-key verification). Directory listings include name / size / is_dir / mode.
- **`tftp://`** — RFC 1350 UDP read. Hand-rolled over `UdpSocket`. Optional RFC 2348 `blksize` negotiation via `--tftp-blksize`. Upload (WRQ) not in scope.
- **`gopher://` / `gophers://`** — RFC 1436 selector fetch. Hand-rolled. TLS variant uses rustls with the shared webpki root store.
- **Script bindings**: `ftp(url [, opts])`, `sftp(url [, opts])`, `tftp(url [, opts])`, `gopher(url [, opts])`. Each returns a Map mirroring the CLI output.
- **New CLI flags** (help_heading = "File Transfer"): `--ftp-active`, `--ftps-implicit`, `--tftp-blksize`.
- **Help**: four new topics — `recon --help ftp`, `--help sftp`, `--help tftp`, `--help gopher`. Each listed in `topic_keys()` and with URL-scheme entries in `TOPIC_PROTOCOLS`.
- **`recon --examples`**: new `FILE TRANSFER (0.47.0)` section with 5 blocks.
- **Example scripts**: `script/ftp.rhai`, `script/sftp.rhai`, `script/tftp.rhai`, `script/gopher.rhai`, all under a new "File transfer" category in `script/README.md`.

### Changed

- `Cargo.toml` adds `suppaftp = "6"` with `rustls` feature (~500 KB). Version 0.46.1 → 0.47.0.
- `src/main.rs` URL dispatch gains 6 new branches (`ftp://`, `ftps://`, `sftp://`, `tftp://`, `gopher://`, `gophers://`).

### Removed from OUT-OF-SCOPE.md

- FTP, TFTP, GOPHER, SFTP from the "permanently out of scope" list.
- The "non-HTTP protocols" blanket line narrowed to list only SMB / SMBS as remaining deferred.

## [0.46.1] - 2026-04-23

### Added

Catch-up patch closing the script-binding gap from 0.46.0 and bringing 0.45.0 / 0.46.0 script examples in line with the shipped features.

- **`encrypt::rekey(blob, old_identity_paths, new_recipients [, armor])`** script binding. Decrypts age-format input with the old identities, re-encrypts to the new recipient set. Armor is optional (default binary).
- **`encrypt::pgp_encrypt(blob, recipients)`** + **`encrypt::pgp_encrypt_armored(...)`** + **`encrypt::pgp_decrypt(blob)`** script bindings. Shell out to the system `gpg` binary, same as the CLI path.
- **`encrypt::detect_backend(recipient)`** — returns `"age"` or `"pgp"` using the same heuristic as the CLI auto-detection. Handy for scripts that accept recipients as user input and need to branch.
- **`script/encrypt.rhai`** rewritten to exercise keygen, in-memory encrypt / armored encrypt, rekey round-trip, and backend dispatch.
- **`script/mqtt.rhai`** extended to publish with MQTT-5 `user_properties` + `content_type` via the opts map (demonstrates the 0.45.0 work from the script side).

### Changed

- **`CLAUDE.md`** gains an "Exposure policy — every feature must reach every surface" section. Every new flag / function / protocol probe / script binding must land in all four surfaces in the same release: `recon --help <topic>`, `recon --examples`, the Rhai script engine, and the documentation trio (CHANGELOG / HISTORY / OUT-OF-SCOPE + `script/*.rhai` examples where applicable).
- `TOPIC_SCRIPT` gains FlagHelp entries for the four new `encrypt::*` functions.

## [0.46.0] - 2026-04-23

### Added

- **PGP / GPG interop via shell-out to `gpg`**. Recon detects OpenPGP recipients (anything not matching `age1…` / an existing file path) and delegates encrypt / decrypt to the system `gpg` binary. Plaintext is piped over stdin; ciphertext is captured from stdout. Requires `gpg` on PATH (install `gnupg`); clear error when absent.
- **`--pgp` / `--age`** force-backend flags. Mutually exclusive. Without either, recon auto-detects per-recipient: `age1…` → age; anything else (hex fingerprint, key-id, email, uid) → PGP. Decrypt path auto-detects from magic bytes (armored `-----BEGIN PGP MESSAGE-----` or binary OpenPGP packet-tag high bit).
- **`--rekey`** — key rotation. Decrypts the input with `--identity` (and/or `--passphrase-file` for passphrase-encrypted age), then re-encrypts to the new `--recipient` set. Source format auto-detected (age vs PGP). Can switch backends during rotation — `age → PGP` or `PGP → age` — by pairing with `--pgp` / `--age` on the target side.
- **`src/encrypt.rs`** gains `pub fn detect_backend`, `pub fn gpg_encrypt_bytes`, `pub fn gpg_decrypt_bytes`, `pub fn decrypt_bytes_age`, `pub fn run_rekey` (available for script bindings in a future release).
- **`recon --help encrypt`** extended with three new flag entries + four new examples (PGP encrypt/decrypt + basic / cross-backend rekey).
- **`recon --examples` ENCRYPTION section** gains two new blocks demonstrating PGP auto-detection and `--rekey`.

### Changed

- `src/main.rs` encrypt dispatch now also runs on `--rekey` (mutually exclusive with `--encrypt` / `--decrypt`).
- `run_decrypt` routes to the PGP backend when the input's magic bytes indicate OpenPGP or when `--pgp` is explicit.

### Removed from OUT-OF-SCOPE.md

- PGP / GPG interop (shipped).
- Key rotation / management (shipped).

### Still deferred

- **Hardware-backed keys (`age-plugin-*`)** — age 0.11 doesn't expose plugin hooks. GPG smartcards work naturally through the `gpg` subprocess when the user's keyring is configured.
- **Mixed recipient-and-passphrase in one age header** — age 0.11's `Encryptor::with_recipients` rejects `scrypt::Recipient` alongside X25519 recipients. Would require bypassing age's Encryptor and writing custom stanzas. OUT-OF-SCOPE.md updated with rationale.

## [0.45.0] - 2026-04-23

### Added

MQTT 5 power-user properties deferred from 0.22.0 — the rumqttc 0.24 machinery was already linked, now exposed through CLI + script surface:

- **`--user-property <KEY=VAL>`** (repeatable) — MQTT 5 user-property on PUBLISH + SUBSCRIBE packets. Silently ignored on `--mqtt-version 3`.
- **`--will-topic <T>` + `--will-payload <P>` + `--will-qos <0|1|2>` + `--will-retain`** — last-will message published by the broker on unexpected disconnect. Payload accepts `@file` / `@-` for file / stdin input.
- **`--session-expiry <SECS>`** — MQTT 5 session-expiry-interval connect property.
- **`--clean-start <BOOL>`** — default `true`; set `--clean-start=false` to resume a persistent session (paired with `--session-expiry` and a fixed `--client-id`).
- **`--content-type <MIME>`** — publish content-type property (e.g. `application/json`).
- **`--response-topic <T>`** — publish response-topic property for request/response patterns.
- **`--correlation-data <DATA>`** — publish correlation-data property. Accepts `@file` / `@-` / raw.
- **`--auth-method <NAME>` + `--auth-data <DATA>`** — MQTT 5 enhanced-authentication for SASL-style flows.
- **Script binding**: `mqtt_pub` / `mqtt_sub` opts maps gain `user_properties` (Array of `#{key, value}` or `"k=v"` strings), `will` (Map with topic/payload/qos/retain), `session_expiry`, `clean_start`, `content_type`, `response_topic`, `correlation_data`, `auth_method`, `auth_data`.
- **`recon --help mqtt`** extended with 10 new `FlagHelp` entries + 4 new examples (user-property publish, request/response pattern, last-will, persistent session resume).
- **`recon --examples` MQTT section** gains four new blocks demonstrating each property group.

### Changed

- `src/mqtt.rs::setup_options_v5` wires `ConnectProperties` (session-expiry, user-properties, auth-method/data) and `LastWill` through the existing `rumqttc::v5::MqttOptions` setters.
- `publish_v5` now calls `publish_with_properties` when any publish property is set; falls back to the no-property path otherwise to avoid wire-cost on simple publishes.
- `subscribe_v5` mirrors the same pattern for user-properties on SUBSCRIBE.

### Removed from OUT-OF-SCOPE.md

- The six MQTT 5 power-user items (shipped this release). Client-cert mTLS + dual rustls majors still deferred; their rationale kept verbatim.

## [0.44.0] - 2026-04-23

### Added

- **`smtp://` / `smtps://` URL-scheme probe** — diagnose an SMTP server end-to-end. Probe mode (default): TCP connect, read greeting, send EHLO, report every advertised extension + AUTH mechanisms + STARTTLS availability, disconnect. Send mode (`--mail-from` + `--mail-to`): full transaction with AUTH / MAIL / RCPT / DATA / QUIT via the `lettre` crate, with optional DKIM signing.
- **`--mail-from`, `--mail-to` (repeatable), `--mail-subject`, `--mail-body`, `--mail-header`** (repeatable) — compose the test message. `--mail-body` accepts `@file` / `@-` for file / stdin input.
- **`--smtp-auth <user:pass>`** — AUTH PLAIN → LOGIN fallback. Exit 67 on rejection.
- **`--smtp-helo <NAME>`** — HELO / EHLO identifier (default: `recon.local`).
- **`--no-starttls`** — skip the STARTTLS upgrade on `smtp://`.
- **`--dkim-key <PATH>` + `--dkim-selector <SEL>` + `--dkim-domain <DOMAIN>`** — sign outbound messages with RSA or Ed25519. Algorithm auto-detected from the PEM. Signing domain defaults to the domain part of `--mail-from`.
- **Script binding `smtp(url)` / `smtp(url, opts)`** — returns a Map with host, port, tls, connect_ms, banner, capabilities (Array), auth_methods (Array), starttls_ok (bool or `()`), send_result (Map or `()`). `opts` mirrors the CLI flags with snake_case keys.
- **`recon --help smtp`** (aliases `smtps`, `mail`, `email-send`) with flag reference + examples.
- **`recon --examples` SMTP section** with 7 scenarios (probe, authed send, DKIM sign, @file body, scripted probe).
- **`script/smtp.rhai`** example script with TCP reachability guard.
- **Dependency**: `lettre = "0.11"` with `smtp-transport`, `rustls-tls`, `ring`, `builder`, `dkim` features.

### Removed from OUT-OF-SCOPE.md

- SMTP / SMTPS protocol probe (shipped this release).
- "recon is HTTP(S)-only" blanket statement — superseded by actual shipped protocol probes.

## [0.43.0] - 2026-04-22

### Added

First-class character-set (charset) support across the CLI and script surface. Motivating use case: a PHP service talking UTF-8 and a Perl service talking ISO-8859-1 that exchange data via `recon`.

- **`--output-charset <NAME>`** transcodes the response body to NAME before prettify or write. Source detection priority: `--source-charset` > response `Content-Type: ...; charset=NAME` > BOM sniff (UTF-8/UTF-16) > chardetng heuristic > windows-1252 fallback. Pass-through when source == target. Unmappable characters are substituted with `?` (iconv `-c` behaviour); a warning goes to stderr unless `-s`.
- **`--source-charset <NAME>`** overrides the server's declared/sniffed source charset (for servers that lie or omit `charset=`).
- **`--to-utf8`** — shorthand for `--output-charset utf-8`, the most common case.
- **`--request-charset <NAME>`** transcodes the request body (from UTF-8, the shell's native encoding) to NAME before sending. Overrides any `charset=` on an explicit Content-Type.
- **Auto-transcoding on request** when an explicit `Content-Type: ...; charset=X` is set. The UTF-8 `-d` body is converted to X before `request.body(...)`. `--request-charset-passthrough` skips this (for pre-encoded input).
- **`--iconv <SOURCE:TARGET>`** — standalone file/stdin conversion mode, no HTTP. Reads from the positional arg (or stdin), writes to `-o PATH` (or stdout). Blank `SOURCE` means auto-detect via BOM + chardetng. Exit 0 on success, 1 on unmappable substitution, 2 on errors.
- **`--list-charsets`** dumps the curated set of recognised charset labels.
- **`text::*` script module** — `transcode(blob, from, to)`, `decode(blob, charset)`, `encode(str, charset)`, `detect(blob) -> #{charset, had_bom}`, `charset_of(headers) -> String | ()`, `strip_bom(blob)`, `list()`, `normalize_newlines(str, style)` (`lf`/`crlf`/`cr`).
- **`r.body_bytes` + `r.charset`** on every `http()` and `browser()` response map — raw Blob and the resolved charset (or `()` when undecidable). Scripts combine these with `text::decode()` for explicit re-decoding when the CLI's automatic detection is unreliable.
- **`recon --help charset`** (aliases `text`, `iconv`, `text-encoding`) renders the full feature reference.
- **Two new example scripts:** `script/text.rhai` (charset inspect/decode/encode round-trip + BOM detection + newline normalisation) and `script/browser-iso8859.rhai` (browser() talking to a Latin-1 service via Content-Type-driven auto-transcoding).
- **Integration test suite** `tests/charset_it.rs` with 7 end-to-end wiremock scenarios: output transcoding, no-op passthrough, source override, request transcoding via Content-Type, request passthrough, `--iconv` file round-trip, `--iconv :utf-8` auto-detect from stdin.

### Changed

- `Cargo.toml` adds `encoding_rs = "0.8"` (the WHATWG-spec-compliant charset library browsers use) and `chardetng = "0.1"` (charset auto-detection).
- Response body pipeline in `src/output.rs::write_response_to` gains a transcode step when `--output-charset`/`--to-utf8` is set. Zero-copy streaming is preserved when no charset flag is present (no change for existing workflows).
- `src/client.rs`: every `request.body(...)` call site (six of them — `-T`, `--json`, `--data-raw`, `--data-binary`, `--data-urlencode`, `-d`) routes through a new `apply_request_body()` helper that handles the optional transcode.
- `recon --examples` gains a `TEXT ENCODING (charsets)` section with 5 examples.

## [0.42.1] - 2026-04-22

### Added

- Three additional `browser()` example scripts in `script/`:
  - `browser-login.rhai` — JSON login with a Map body → sticky session cookie on a follow-up protected request.
  - `browser-persist.rhai` — demonstrates `use_persistent_session(name)` and inspects the `~/.recon/jars/NAME.db` jar contents; jar survives across script runs.
  - `browser-multi.rhai` — three browsers with distinct user-agents and cookie jars in one script, asserting isolation.
- `browser` added to the `--help` Topics footer so `recon --help` advertises the new topic.

### Changed

- Renamed the five agent-browser example scripts from `browser-*.rhai` → `agent-browser-*.rhai` to disambiguate from the new scripting-`browser()` examples. Old names removed.
  - `browser-screenshot.rhai` → `agent-browser-screenshot.rhai`
  - `browser-title.rhai` → `agent-browser-title.rhai`
  - `browser-snapshot.rhai` → `agent-browser-snapshot.rhai`
  - `browser-form-login.rhai` → `agent-browser-form-login.rhai`
  - `browser-guard.rhai` → `agent-browser-guard.rhai`
- Updated `script/README.md` to separate "Sessions (scriptable browser())" from "Browser automation (external agent-browser CLI)".
- Help text and example references in `src/help.rs` and `src/examples.rs` point at the renamed files.

## [0.42.0] - 2026-04-22

### Added

- **`browser()` script binding** — a stateful HTTP session handle. Unlike the one-shot `http(url, opts)` binding, a browser keeps cookies, default headers, user-agent, redirect policy, timeouts, and basic-auth credentials across multiple requests. Script-only feature (no CLI flag).
  - Constructors: `browser()` / `browser(#{user_agent, headers, insecure, follow_redirects, max_redirects, timeout_ms, connect_timeout, basic_auth})`.
  - Configuration: `set_user_agent`, `set_header`, `set_headers`, `remove_header`, `clear_headers`, `set_timeout_ms`, `set_connect_timeout`, `set_insecure`, `follow_redirects`, `set_max_redirects`, `set_basic_auth`.
  - Sessions: `use_persistent_session(name)` swaps the jar to `~/.recon/jars/NAME.db` (fresh swap — ephemeral cookies are discarded); `use_ephemeral_session()` reverts to a new temp-file jar; `clear_cookies()`; `cookies()` returns `[#{domain, path, name, value, expires, secure, http_only}, …]`; `session_name()` returns the active name or `()` for ephemeral.
  - Requests: `b.get / head / options / delete(url [, opts])` and `b.post / put / patch(url, body [, opts])`. Body accepts String, Blob, Map, or Array — maps and arrays auto-serialise to JSON with `Content-Type: application/json`. `b.request(#{url, method, body, headers, …})` for the freeform form.
  - Multiple browsers can coexist in the same script with independent state ("parallel browsers" use case — Rhai is single-threaded, so this means sequential interleaved calls against isolated jars).
- **`recon --help browser`** — new help topic with the full method reference. `recon --help session` and `recon --help browser-session` are aliases. Real browser automation moved to `recon --help agent-browser` only (`--help browser` no longer resolves there).
- **`script/browser.rhai`** example showing the ephemeral-session idiom.

### Changed

- `src/script/bindings/http.rs`: `build_args` and `headers_to_rhai_map` promoted to `pub(crate)` so the browser binding can reuse the opts-map overlay and response-header conversion without duplication.
- `src/script/bindings/helpers.rs`: `dynamic_to_json` promoted to `pub(crate)` for the browser binding's Map/Array→JSON body coercion.
- `tempfile` moved from `[dev-dependencies]` to `[dependencies]` (the browser binding uses `NamedTempFile` to back ephemeral cookie jars).

## [0.41.0] - 2026-04-22

### Added

- **Per-module example scripts in `script/`.** One `.rhai` file per script binding, covering every module: protocol probes (`http`, `tcp`, `ping`, `dns`, `tls`, `ntp`, `redis`, `ws`, `dict`, `ldap`, `whois`, `memcached`, `rtsp`, `mqtt`), data primitives (`file`, `hash`, `compression`, `archive`, `sqlite`), domain tools (`encode`, `encrypt`, `checkdigit`, `sample`, `jwt`, `email`, `netstatus`), and a minimal `agent-browser` example beyond the existing five browser recipes. 21 new files; the five `browser-*.rhai` recipes stay.
- **`tests/script_examples_it.rs`** integration test walks `script/` and `engine.compile_file`s every `.rhai`, flagging parse errors mechanically. A second test verifies `script/README.md` indexes every file.

### Changed

- **`script/README.md`** rewritten as a categorised index (Protocol probes / Data primitives / Domain tools / Browser automation) with one-line descriptions, usage patterns, and guard-pattern guidance for scripts that need external services.
- **`TOPIC_SCRIPT` help examples** now reference the shipped `script/` directory (ls, run, copy-to-global idioms).
- **`recon --examples` SCRIPTING** section adds a "Browse per-module example scripts" block pointing users at `script/*.rhai`.

## [0.40.0] - 2026-04-22

### Added

Seven new Rhai static modules exposing CLI features that previously had no script surface. Brings script parity back in line with the CLI after several releases' worth of new flags.

- **`encode::*`** — QR / DataMatrix / 1D barcode generation. `encode::qr(data)` / `datamatrix(data)` / `barcode(format, data)` return PNG Blobs; `encode::encode(format, data, "ascii" | "svg" | "png")` switches output form; `encode::list()` enumerates formats.
- **`encrypt::*`** — age encryption. `encrypt(plaintext, [recipients])` / `encrypt_armored(...)` produce binary / ASCII-armored Blobs; `decrypt(ciphertext, [identity_paths])` reverses; `keygen()` returns `#{ public: "age1...", private: "AGE-SECRET-KEY-1..." }`. Passphrase mode is CLI-only (scripts shouldn't prompt interactively).
- **`checkdigit::*`** — 80+ check-digit algorithms (VAT per-country, ISBN, EAN-13, Luhn, VIN, credit card, etc.). `verify(algo, input)` → bool, `inspect(algo, input)` → detailed map, `create(algo, body)` appends the check digit, `list()` enumerates all algorithms.
- **`sample::*`** — informational for the built-in sample-data registry. `list()` / `spec(name)` / `url(name, format)`. Actual fetching uses `http()` explicitly.
- **`jwt::*`** — HS256 / HS384 / HS512 sign + verify. `sign(claims_map, secret [, alg])` returns a token string, `validate(token, secret)` returns `#{valid, checks, header, payload}`, `view(token_or_json)` decodes without verification.
- **`email::*`** — DNS-based email-security checks. `spf(host)` / `dmarc(host)` / `dkim(host, selector)` / `mta_sts(host)` / `bimi(host [, selector])` / `tls_rpt(host)` each return `#{name, verdict, summary, details}`. `email::all(host)` runs five of them and returns a composite map.
- **`netstatus::*`** — network-reachability probes. `check()` runs a default HTTP + TCP probe set and returns `#{status, probes, passed, total}` with status being `"ONLINE"` / `"DEGRADED"` / `"OFFLINE"`. `probe_http(url)` / `probe_tcp(host, port)` expose individual probes.

### Changed

- New public helper `encrypt::encrypt_bytes_recipients` / `decrypt_bytes_identities` in `src/encrypt.rs` — in-memory age encryption used by the script binding (and available for future CLI refactors).
- `netstatus::probe_http` / `probe_tcp` promoted to `pub(crate)` so the script binding can call them directly.
- TOPIC_SCRIPT help topic gets seven new FLAGS entries covering the new modules.

## [0.39.0] - 2026-04-22

### Added

- **`--dns-servers <LIST>`** overrides DNS resolution for HTTP requests with a comma-separated list of custom nameservers. Accepts `IP` (port 53 implied) or `IP:PORT`. Uses a hickory-backed resolver wired into reqwest's `Resolve` trait; both UDP and TCP fallback paths are registered per server.
- **`--dns-ipv4-addr <IP>` / `--dns-ipv6-addr <IP>`** bind outgoing DNS queries to a specific local address (per-protocol). When `--dns-ipv4-addr` or `--dns-ipv6-addr` is set without `--dns-servers`, a default of `1.1.1.1:53` is used (system resolvers would ignore the bind address, so falling back to them would silently void the setting).
- **`--dns-interface <IFACE>`** accepted at the CLI but not yet plumbed — recon errors out with a clear message directing users to `--dns-ipv4-addr` / `--dns-ipv6-addr`. OUT-OF-SCOPE updated with rationale.
- **Script parity**: all four DNS-override flags available on `http(url, opts)` as `dns_servers`, `dns_ipv4_addr`, `dns_ipv6_addr`, `dns_interface`.

### Changed

- New `src/dns_resolver.rs` module: parses flag values, builds `ResolverConfig` + `NameServerConfig` entries for hickory, and implements `reqwest::dns::Resolve` on a `CustomResolver` type whose `resolve` future delegates to `TokioAsyncResolver::lookup_ip`.
- `src/client.rs::execute` calls `dns_resolver::build_from_args` and plugs the returned resolver into `ClientBuilder::dns_resolver` when any DNS flag is set; otherwise the default getaddrinfo path remains.

### Removed from OUT-OF-SCOPE.md

- `--dns-servers`, `--dns-ipv4-addr`, `--dns-ipv6-addr` (shipped this release).

## [0.38.0] - 2026-04-22

### Added

- **`--limit-rate <RATE>`** throttles HTTP downloads. Accepts curl's grammar: `100K` = 102,400 B/s, `2M` = 2,097,152 B/s, `1.5G`, `1G`, bare bytes, optional trailing `B`. Implementation: `RateLimitedWriter` that wraps the output writer and sleeps between writes so cumulative throughput tracks the pinned rate.
- **`--speed-limit <BYTES>` + `--speed-time <SECS>`** aborts slow transfers. When the rolling download rate stays below `speed_limit` B/s for `speed_time` seconds (default 30), the write path returns `ErrorKind::TimedOut`. Useful for failing fast on stalled CDNs / dial-up-class interfaces.
- **Script parity**: all three flags available on `http(url, opts)` — `limit_rate: "1M"`, `speed_limit: 1024`, `speed_time: 15`.

### Changed

- New `src/ratelimit.rs` module with `parse_rate`, `RateLimitedWriter`, `SpeedWatchWriter`. All three compose on top of `Box<dyn Write + 'a>` so they nest cleanly.
- `src/output.rs::write_response_to` threads output through `wrap_with_rate_control` when either flag is set. Unaffected when neither is set.
- TOPIC_HTTP help topic + `recon --examples` TLS section gain rate-control entries.

### Removed from OUT-OF-SCOPE.md

- `--limit-rate`, `--speed-limit`, `--speed-time` (all shipped this release).

## [0.37.0] - 2026-04-22

### Added

- **`--tlsv1.2` / `--tlsv1.3`** force a minimum TLS version for HTTPS. Handshake fails if the server can't negotiate at least the pinned version. Both flags present → `--tlsv1.3` wins (higher minimum). Curl-compatible spelling.
- **`--cacert <PATH>`** trusts an additional PEM root certificate on top of the system trust store. Use for self-signed corporate / internal CAs without reaching for `-k`.
- **`--interface <IP>`** binds outgoing HTTP sockets to a specific local IP (IPv4 or IPv6 literal). Interface-name resolution (e.g. `eth0`) is not yet supported — pass the address directly.
- **Script parity**: all four flags are available as `http(url, opts)` keys: `tlsv12`, `tlsv13`, `cacert`, `interface`. Reflected in the `flags` global map that scripts inherit from the CLI invocation.

### Changed

- `src/client.rs::execute` extended to wire `min_tls_version`, `add_root_certificate`, and `local_address` onto the reqwest `ClientBuilder` when the respective flags are set.
- `src/script/defaults.rs::ScriptDefaults` gains the four fields; `src/script/bindings/http.rs::build_args` overlays them from per-call opts; `src/script/bindings/cli.rs` exposes them in the `flags` map.

### Removed from OUT-OF-SCOPE.md

- `--tlsv1.2`, `--cacert`, `--interface` (all shipped this release).

## [0.36.0] - 2026-04-22

### Added

- **`compression::*` Rhai static module** exposing all nine stream algorithms (gzip, deflate, zstd, brotli, bzip2, lz4, xz, snappy, zlib) to scripts. Functions: `compress(algo, blob [, level])` with integer or word levels (fastest/fast/default/good/best), `decompress(blob)` with magic-byte auto-detect, `decompress(algo, blob)` for explicit algo, `list()` returning algo metadata as an array of maps, `detect(blob)` returning the algorithm name or `()`. Level-less algos (lz4, snappy) reject a level argument with a clear error; deflate/brotli have no signature so auto-detect on them throws "pass the algo explicitly".
- **`archive::*` Rhai static module** exposing the 0.35.0 archive tools to scripts. Functions: `create(dest, [sources])` returns files-archived count, `extract(src, dest_dir)` creates the destination dir and returns extracted count, `detect(path)` returns the format label (`"zip"`, `"tar.gz"`, …) or `()`. Matches the CLI's format list (.zip / .tar / .tar.gz / .tar.xz / .tar.bz2) and extension-based detection. Extract also falls back to magic-byte sniffing when the extension isn't recognised.
- Closes the script-parity gap retroactively for 0.34.0 (compression streams) and 0.35.0 (archive flags). Going forward, every new CLI feature ships a script binding alongside.

### Changed

- TOPIC_SCRIPT help topic picks up the two new module entries; `recon --examples` SCRIPTING section adds a compress-and-decompress example and a script-driven archive example.

## [0.35.0] - 2026-04-22

### Added

- **`--archive DEST FILE...`** creates an archive from one or more files or directories. Format is inferred from DEST's extension: `.zip`, `.tar`, `.tar.gz` / `.tgz`, `.tar.xz` / `.txz`, `.tar.bz2` / `.tbz2`. Directory sources are archived recursively. Trailing positional args after DEST are captured via the same argv pre-split that handles `--script` trailing args.
- **`--extract SRC [-o DIR]`** extracts an archive into `DIR` (default: current directory). Format inferred from SRC's extension first, then from magic bytes (so `.dat` that's actually a ZIP still works). Magic sniff covers ZIP (`PK\x03\x04`), gzip (`1f 8b`), xz (`fd 37 7a 58 5a 00`), bzip2 (`BZh`), and tar (`ustar` at offset 257).
- New `recon --help archive` topic (aliases: `zip`, `tar`, `extract`) documenting the format table, output conventions, and magic-byte fallback for `--extract`. `recon --examples` gains an ARCHIVES section.

### Changed

- New direct deps: `zip = "2"` (features `deflate`, `bzip2`) and `tar = "0.4"`. Reuses `flate2` / `xz2` / `bzip2` (all already in the tree after 0.34.0) for tar+compression combinations.
- `Args::split_script_trailing` extended to also split on `--archive <DEST>`; trailing positional args go into `script_args` for both flags (mutual exclusion enforced at dispatch — `--archive` and `--script` would otherwise fight for the same vec).
- `src/archive.rs` is the new module owning both CLI flags + the per-format create/extract implementations. Includes a ~30-line in-module `walkdir` helper to avoid pulling a dep for directory recursion.

## [0.34.0] - 2026-04-22

### Added

- **Four new stream-compression algorithms on `--compress` / `--decompress`:**
  - **`lz4`** (alias `lz`) — LZ4 frame format via `lz4_flex`. Fast, no level setting.
  - **`xz`** (alias `lzma`) — XZ/LZMA via `xz2`. Levels 0-9, default 6.
  - **`snappy`** (aliases `snap`, `sz`) — Google Snappy frame format via `snap`. No level setting.
  - **`zlib`** (alias `zl`) — raw RFC 1950 (not gzip-wrapped) via existing `flate2`. Levels 0-9, default 6.
- Magic-byte auto-detect extended: lz4 (`04 22 4d 18`), xz (`fd 37 7a 58 5a 00`), snappy (`ff 06 00 00 73 4e 61 50 70 59`), and zlib (CMF byte `0x78` + FLG byte where `(CMF*256 + FLG) % 31 == 0`, per RFC 1950).
- `--compression-level` against lz4 or snappy now errors with a clear "algorithm has no level setting" message rather than silently ignoring the value.
- `recon --compress-list` picks up the four new entries.

### Changed

- New direct deps: `lz4_flex = "0.11"`, `xz2 = "0.1"`, `snap = "1"`. `flate2` (already in tree) also used for the new zlib support.
- Docs: TOPIC_COMPRESSION help topic lists every supported algorithm + aliases; `recon --examples` COMPRESSION section adds four new example rows.

## [0.33.0] - 2026-04-21

### Added

- **`agentBrowser` Rhai static module** wrapping the external `agent-browser` CLI (browser automation). Exposes ~30 functions — `open`, `close`, `click`, `dblclick`, `type_text`, `fill`, `press`, `hover`, `focus`, `check`, `uncheck`, `scroll`, `scrollintoview`, `wait`, `screenshot`, `pdf`, `snapshot`, `eval`, `get`, `is_visible`/`is_enabled`/`is_checked`, `find`, `keyboard_type`/`keyboard_insert`, `back`/`forward`/`reload`, plus `cmd([…])` as a raw-CLI escape hatch. Always registered; `agentBrowser::available` (bool) and `agentBrowser::version` (string) are readable whether or not `agent-browser` is installed. When unavailable, any function call throws a clear Rhai error asking the user to install the binary. JSON envelopes from agent-browser (`{success, data, error}`) are automatically unwrapped so scripts see the `data` payload directly.
- **`recon --browser-screenshot URL [-o PATH]`** convenience CLI flag. Opens the URL in a browser via agent-browser, writes a screenshot, closes. Requires the binary on PATH.
- **Project-level `script/` folder** with `README.md` and five reference scripts: `browser-screenshot.rhai`, `browser-title.rhai`, `browser-snapshot.rhai`, `browser-form-login.rhai`, `browser-guard.rhai`. Not installed automatically — run directly with `recon --script script/NAME.rhai` or copy into `~/.recon/script/` for bare-name invocation.

### Changed

- New `src/agent_browser.rs` module owns availability detection (cached via `OnceLock`) and the shared `run_cmd(args, json)` helper. Both the Rhai binding and the CLI flag delegate to it.
- `json_to_dynamic` in `src/script/bindings/helpers.rs` promoted to `pub(crate)` so `agentBrowser`'s JSON-parsing commands (`snapshot`, `get`, `eval`, `find`) can reuse it.

## [0.32.0] - 2026-04-21

### Added

- **Rhai `import "name" as alias;` support.** Scripts can now factor shared helpers into reusable modules. Resolution order: (1) sibling `.rhai` file next to the importing script (so `/tmp/foo.rhai` importing `"helpers"` finds `/tmp/helpers.rhai`), (2) fallback to `~/.recon/script/<name>.rhai`. Scripts that already live in the global dir get sibling imports via the first resolver naturally, no special case. Absolute paths and relative `../` imports pass through the default resolver. `.rhai` extension is auto-appended.
- Wired via a `ModuleResolversCollection` with two `FileModuleResolver`s chained in `script::engine::build_engine`. `engine::run_file` now compiles the source with `engine.compile_with_scope` + `ast.set_source(path)` (rather than `engine.eval_with_scope`) so the default resolver can locate the importing script's directory — without `set_source`, Rhai's resolver had no source path and imports failed even for sibling files.

## [0.31.2] - 2026-04-21

### Fixed

- **Paged `recon --help` now keeps ANSI colours.** Clap renders main `--help` via its own styling and uses auto-detection — once our pager dup2's stdout to a pipe, clap sees "not a TTY" and strips colour. Main help now sets `ColorChoice::Always` on the clap Command when a pager is active so `less -R` receives and renders the escape codes. Topic help (`--help script`, etc.) was already correct because it uses the `colored` crate, which honours our `set_override(true)` at activate time. Non-paged `--help` (redirected, piped) continues to emit mono output via clap's default Auto.

## [0.31.1] - 2026-04-21

### Fixed

- **Pager no longer exits after the first page.** Previously `recon --examples` and `recon --help` would show one screen and return — the child `less` process was competing with the shell for terminal control because `recon` exited before it. `pager::finish()` now flushes stdout, closes STDOUT_FILENO so `less` sees EOF on its pipe, and blocks on `child.wait()` until the user quits the pager. Non-TTY paths (redirects, pipes, `--no-pager`, `RECON_NO_PAGER`) continue to deliver full content since `activate()` returns `None` and `finish()` is a no-op.

## [0.31.0] - 2026-04-21

### Added

- **Auto-paging for `--help` and `--examples`.** When stdout is a TTY, recon now pipes help output through `$PAGER` (default `less -FRX`), matching `git log` / `git help`. Short topics still appear instantly because `less -F` exits when content fits on one screen; long ones (`recon --examples`) open scrollable. ANSI colours are preserved through the pipe via an explicit `colored::control::set_override(true)` when paging activates — without it `colored` would strip escapes on the pipe.
- **`--no-pager` flag** to disable per-invocation (mirrors git). Also honours `$RECON_NO_PAGER` env var for shell profiles and CI. Non-TTY stdout (redirects, pipes) is never paged regardless. Missing pager binaries (`PAGER=nonexistent`) fall through silently to unpaged output.

### Changed

- New `src/pager.rs` module handling decision logic, spawn, and `dup2`. Unix-only; Windows targets compile it as a no-op (unpaged, same as before).

## [0.30.1] - 2026-04-21

### Changed

- Documentation sweep bringing `recon --help` topics and `recon --examples` back in sync with the 0.25.0 → 0.30.0 feature batch (scripting, script hashes, `args`/`flags` globals, SQLite bindings, `--init`, `~/.recon/script/` fallback).
- `recon --help script`: rewrote the description to mention SQLite, hashes, `--init`-as-first-step, and `args`/`flags` globals up-front. Refreshed the EXAMPLES block with six representative recipes (bootstrap, bare-name scripts, positional args, flag inheritance, cookie-jar query, cross-reference to `--examples`).
- `recon --help hash`: added `crc32` to the algorithm list and a crc32 example row (the 0.27.0 addition wasn't reflected).
- `recon --help cookies`: added a pointer to the `sqlite("cookiejar:NAME")` script binding so cookie-jar introspection is discoverable from both topic pages.
- `recon --examples`: refined the SCRIPTING section's opening example (path vs bare-name resolution, `--init` reference). Added an in-memory SQLite example showing how to aggregate probe results into `:memory:`.

## [0.30.0] - 2026-04-21

### Added

- **`recon --init`** bootstraps `~/.recon/` with the standard layout: creates `~/.recon/`, `~/.recon/script/`, `~/.recon/jars/`, `~/.recon/sni/`, and writes a commented `config.toml` skeleton covering every `ReconConfig` section (editor / netstatus / sampledata). Idempotent — existing files and directories are not overwritten; each action prints `created`, `wrote`, or `skipped (exists)`.

## [0.29.0] - 2026-04-21

### Added

- **`sqlite(spec [, mode])` script binding.** Opens a SQLite database by path (`/tmp/data.db`), in-memory alias (`:memory:`), or internal-database alias (`cookiejar`, `cookiejar:NAME` → `~/.recon/jars/NAME.db`). Mode defaults to `"rw"`; `"ro"` and `"rwc"` (create on missing) also supported. Returns a handle with four methods: `query(sql [, params])` → Array<Map>, `query_one(sql [, params])` → Map or `()`, `query_value(sql [, params])` → scalar Dynamic or `()`, `exec(sql [, params])` → i64 rows affected. Positional `?` params bind from a Rhai array; types map to SQLite affinity (`()` → NULL, bool/i64 → INTEGER, f64 → REAL, String → TEXT, Blob → BLOB). SQL errors throw Rhai exceptions surfacing rusqlite's error message. Spec resolution order: `:memory:` first, then literal path (detected by `/`, `\`, or `.db` suffix), then alias lookup.

## [0.28.0] - 2026-04-21

### Added

- **`args` and `flags` constants exposed to Rhai scripts.** `args` is an array where `args[0]` is the script name as typed (e.g. `"health"` when the user types `recon --script health`, not the resolved `~/.recon/script/health.rhai` absolute path) and `args[1..]` are trailing positional arguments: `recon --script foo a b -v` yields `args = ["foo", "a", "b", "-v"]`. `flags` is a map mirroring the CLI flag set that `ScriptDefaults` uses — `headers`, `insecure`, `connect_timeout`, `max_time`, `follow_redirects`, `max_redirs`, `user_agent`, `referer`, `user`, `method`, `data`, `output`, `verbose`, `wait_time`, `ping_count`, `max_hops`. Unset optional scalars are `()` (Rhai unit) rather than missing keys. Both are pushed as constants (read-only from inside the script) via `rhai::Scope::push_constant`.
- **Trailing positional args after `--script PATH`** are now captured into the new `Args::script_args` field via a pre-parse argv split in `Args::parse_with_script_split`. Clap's positional `url` field no longer swallows the first trailing arg. Trailing args without `--script` error with "requires --script".

### Changed

- `main.rs` now calls `Args::parse_with_script_split(std::env::args())` instead of `Args::parse()` so the argv split runs before clap does.

## [0.27.0] - 2026-04-21

### Added

- **Hash functions in scripts.** Per-algo: `md5(x)`, `sha1(x)`, `sha256(x)`, `sha384(x)`, `sha512(x)`, `sha3_256(x)`, `sha3_512(x)`, `blake3(x)`, `crc32(x)` — each returns a lowercase-hex digest. Generic: `hash(algo, x)` and `hash(algo, x, format)` where format is `"hex"` (default) or `"base64"`. Input accepts String (UTF-8 bytes) or Rhai Blob, so `md5(file_read("path"))` works without conversion.
- **CRC32 added to `--hash`.** `recon --hash crc32 FILE` prints the 8-hex CRC-32 digest. Also listed in `--hash-list`. New `crc32fast` dependency.
- **`json_stringify` prettify overloads.** `json_stringify(v, true)` pretty-prints with a 2-space indent; `json_stringify(v, n)` uses an n-space indent (clamped to 1..=8); `n <= 0` or `json_stringify(v, false)` fall back to compact output. Bare `json_stringify(v)` is unchanged.

### Changed

- **Shared `hash::digest_string(algo, bytes, format)` helper** in the hash module. Both the script bindings and future CLI helpers can call it instead of reimplementing the "hash bytes + format" pipeline.

## [0.26.0] - 2026-04-21

### Added

- **Global script directory at `~/.recon/script/`.** When `--script PATH` isn't found as given, recon now falls back to `~/.recon/script/PATH` and (when `PATH` has no extension) `~/.recon/script/PATH.rhai`. Drop reusable scripts in `~/.recon/script/` and call them by bare name: `recon --script health` resolves to `~/.recon/script/health.rhai`. Script-not-found errors list every path that was tried. Directory sits next to `~/.recon/config.toml` so one location holds both CLI config and reusable automation.

## [0.25.19] - 2026-04-20

### Changed

- Added `recon --help script` topic documenting the full Rhai API (http / tcp / ping / dns / tls / ntp / redis / ws / dict / ldap / whois / memcached / rtsp / mqtt / file_read + helpers) with per-function signatures and return shapes.
- Added a `SCRIPTING (--script)` section to `recon --examples` with a bruno-style health check, a DNS → TCP → HTTP chain, a polling loop, and a cert-days-remaining branch.
- Added `HISTORY.md` entry #28 with the design narrative: crate pick (Rhai), probe-extraction pattern established in Task 5, exit-code plumbing via the thread-local + `anyhow_to_rhai`, CLI-flag inheritance decision, test-shape notes (spawn_blocking + !Send caveats), and the deliberate-out-of-scope list (no async, no file_write, no remote scripts, no mqtt_sub structured return).

## [0.25.18] - 2026-04-20

### Added

- **`json_parse(s)` / `json_stringify(value)` script helpers.** Round-trip between JSON text and Rhai values. Null ↔ `()` (unit), bool ↔ bool, integer/float ↔ i64/f64, string ↔ string, array ↔ array, object ↔ map. Malformed input or non-finite floats raise Rhai exceptions.

## [0.25.17] - 2026-04-20

### Added

- **`file_read(path)` script binding.** Reads a local filesystem path OR a `file://` URL and returns a Rhai Blob (`Vec<u8>`). Scripts that want text can `.to_string()` the blob; binary payloads round-trip cleanly via Blob. No `file_write` counterpart — scripts can only read, matching the principle-of-least-surprise scope decision in the plan.

## [0.25.16] - 2026-04-20

### Added

- **`mqtt_pub(url, payload)` and `mqtt_sub(url, max_ms)` script bindings.** Wrap the existing MQTT publish/subscribe codepath by synthesising an `Args` struct from `ScriptDefaults` + optional opts map. Opts: `qos`, `retain`, `version` (`"3"` or `"5"`), `client_id`, `keepalive`, `timeout`, `insecure`, `username` / `password`; `mqtt_sub` also accepts `count` for "stop after N messages". Return map is `#{ ok: true, duration_ms }`. The MQTT module's stdout output (connect banner, received messages) flows to stdout — scripts that need structured per-message data can capture stdout themselves; carving a pure-collection subscribe codepath out of `mqtt.rs` is deliberately deferred to avoid churning the 44KB module for this release.

## [0.25.15] - 2026-04-20

### Added

- **`rtsp(url)` / `rtsps(url)` script bindings.** Sends `OPTIONS *` and parses the response. Returns `#{ host, port, tls, connect_ms, status_line, status_code, headers, methods }`. `methods` pre-parses the comma-separated `Public:` header for easy branching. `opts.insecure` / `opts.timeout` overlay inherited defaults.

### Changed

- `rtsp_probe.rs` refactored to `probe()`/`run()` split. `probe()` returns `RtspProbeOk { host, port, tls, connect_ms, status_line, headers }`; `run()` prints from the struct.

## [0.25.14] - 2026-04-20

### Added

- **`memcached(url)` / `memcached(url, opts)` script binding.** Returns `#{ host, port, connect_ms, version, version_ms, stats: #{...} }`. When URL path is `/stats`, the full parsed `STAT key value` map is included.

### Changed

- `memcached_probe.rs` refactored to `probe()`/`run()` split with a `MemcachedProbeOk` struct; stats lines are parsed into a `BTreeMap<String, String>` rather than streamed.

## [0.25.13] - 2026-04-20

### Added

- **`whois(host)` script binding.** Auto-discovers the authoritative WHOIS server via IANA, follows a registrar-WHOIS referral if present (same behaviour as `--whois`). Returns `#{ host, server, body }`.

### Changed

- `whois.rs` refactored to `probe()`/`run()` split. `probe()` returns `WhoisProbeOk { host, server, body }`.

## [0.25.12] - 2026-04-20

### Added

- **`ldap(url)` / `ldaps(url)` script binding.** Anonymous simple bind + RootDSE query (`objectClass=*` at scope=base). Returns `#{ url, connect_ms, attrs: #{ "namingContexts": [...], "supportedLDAPVersion": [...], "vendorName": [...], "vendorVersion": [...], "supportedSASLMechanisms": [...] } }`. Optional `opts.timeout` overrides the inherited default.

### Changed

- `ldap_probe.rs` refactored to `probe()`/`run()` split. `probe()` returns `LdapProbeOk { display_url, connect_ms, attrs: BTreeMap<String, Vec<String>> }`; `run()` prints from it.

## [0.25.11] - 2026-04-20

### Added

- **`dict(url)` / `dict(url, opts)` script binding.** Returns `#{ host, port, banner, responses: [#{ command, lines: [String], final_status: i64 }] }`. Bare `dict://host/` runs the server-info aggregate (SHOW SERVER + SHOW DATABASES + SHOW STRATEGIES). Command path grammar matches curl (`d:WORD`, `m:WORD`, `show:…`). Transient streaming of response lines is captured into the `lines` array per command.

### Changed

- `dict_probe.rs` refactored to `probe()`/`run()` split: `probe()` returns `DictProbeOk { host, port, banner, responses }`; `run()` prints from it. Each `DictResponse` captures the command label, all lines received, and the final status code (250 ok or 5xx error).

## [0.25.10] - 2026-04-20

### Added

- **`ws(url)` / `wss(url)` script binding.** Both accept an optional `opts` map with `timeout`. Returns `#{ host, port, scheme, connect_ms, handshake_ms, http_status, headers, pong_nonce_matched, ping_ms }`. Handshake rejection (401/403) carries exit 67; connect refused = 7; timeout = 28.

### Changed

- `ws_probe.rs` refactored to `probe()`/`run()` split: `probe()` returns `WsProbeOk` with full connect/handshake/ping timings and selected response headers; `run()` prints from it with the same formatting.

## [0.25.9] - 2026-04-20

### Added

- **`redis(url)` / `redis(url, command)` / `redis(url, command, opts)` script binding.** With no command, sends PING. With a command string, splits it shell-style (whitespace + `"…"` + `'…'` + `\`-escapes — same splitter the CLI `-d` path uses) and sends the tokens as a RESP2 array. Returns `#{ host, port, connect_ms, auth_reply, command, reply, command_ms }`. Connect refused = 7, timeout = 28, AUTH rejected = 67.

### Changed

- `redis_probe.rs` refactored to `probe()`/`run()` split. `probe()` returns a `RedisProbeOk` struct with connect timing, AUTH reply, command label/reply, and command timing; `run()` prints from it. `shell_split` is now `pub(crate)` so the script binding can reuse the same tokenizer.

## [0.25.8] - 2026-04-20

### Added

- **`ntp(url)` / `ntp(url, opts)` script binding.** Accepts `ntp://host` or bare `host`. Returns `#{ host, port, stratum, precision, poll_interval, ref_id, reference_ts, offset_ms, delay_ms }`. `opts.timeout` overrides the inherited `--connect-timeout`. Exit code on timeout is 28, on unreachable is 7.

### Changed

- `ntp_probe.rs` refactored to the `probe()`/`run()` split pattern: `probe()` returns an `NtpProbeOk` struct; `run()` prints the existing formatted fields from it.

## [0.25.7] - 2026-04-20

### Added

- **`tls(host)` / `tls(host, port)` script binding.** Default port 443; alternate via second arg or `host:port` syntax. Returns `#{ host, port, subject, issuer, not_before, not_after, not_before_ts, not_after_ts, days_remaining, is_expired, san, serial_hex, signature_algorithm, public_key, cert_pem }`. Subject/issuer are maps with `common_name`, `organization`, `organizational_unit`, `country`, `state`, `locality`. Handshake runs with hostname-verification off so self-signed / expired certs can still be inspected.

### Changed

- `cert.rs` split into `parse_target` + `fetch_der` + `fetch_and_print`. The DER-fetching TCP + TLS logic is now reusable by the script binding.

## [0.25.6] - 2026-04-20

### Added

- **`dns(host)` / `dns(host, types)` script binding.** Returns `#{ host, records: #{ "A": [...], "AAAA": [...], … }, errors: #{ "TYPE": "msg" }, duration_ms }`. Default types are the standard set (A, AAAA, CNAME, MX, NS, TXT, SOA) — pass a Rhai array like `["A", "MX"]` to query specific types. No-records and lookup-errors are surfaced distinctly (empty array vs. entry in `errors`).

### Changed

- `dns.rs` refactored to the `probe()`/`run()` split: `probe()` returns a `DnsResults` struct; `run()` formats the printed output as before. The explicit-types error path is preserved (unknown/error records still print under explicit mode).

## [0.25.5] - 2026-04-20

### Added

- **`ping(host)` / `ping(host, count)` script binding.** Host may be `host:port` (TCP ping) or bare `host` (ICMP — unprivileged on macOS; requires `net.ipv4.ping_group_range` or root on Linux). Returns `#{ protocol, host, resolved_ip, port, sent, received, loss_pct, replies: [#{seq, ms}], min_ms, avg_ms, max_ms }`. Default count inherits `--ping-count`.

### Changed

- `ping.rs` refactored to the `probe()`/`run()` split pattern established by Task 5: `probe()` returns a `PingResult` struct with per-reply `PingReply { seq, ms }`; `run()` wraps it with the existing stdout formatting. Public API for the CLI is unchanged.

## [0.25.4] - 2026-04-20

### Added

- **`tcp(url)` / `tcp(url, opts)` script binding.** Returns `#{ ok, host, port, resolved_ip, local_addr, duration_ms }`. Connect failures / timeouts raise Rhai exceptions with the matching `ProtocolExitCode` (7 / 28). Extracted a structured `tcp_probe::probe()` core out of `tcp_probe::run()` — `run()` is now a thin wrapper that prints the probe result. This split establishes the pattern that subsequent probe bindings (dns, ntp, ldap, …) follow.

## [0.25.3] - 2026-04-20

### Added

- **`http(url)` / `http(url, opts)` / `https(...)` / `request(opts)` script bindings.** Wraps `client::execute` so scripts get the same request semantics as the CLI (cookies, redirects, body handling) with an opts-map overlay — `method`, `headers`, `body`, `timeout_ms`, `connect_timeout`, `insecure`, `follow_redirects`. Returns `#{ url, final_url, status, body, headers, http_version, duration_ms }`. HTTP-level non-2xx is a result (not an exception) with the `status` field set; network failures (connect refused, DNS failure, timeout, TLS error) raise Rhai exceptions whose exit codes match the CLI (`7` for connect-refused, `28` for timeout) via `ProtocolExitCode` stashing + `reqwest::Error::is_connect/is_timeout` detection.

## [0.25.2] - 2026-04-20

### Added

- **`ScriptDefaults` + `convert` module** — internal scaffolding ahead of per-probe bindings. `ScriptDefaults::from_args` snapshots the relevant CLI flags (`-H`, `-k`, `--connect-timeout`, `--max-time`, `-L`, `-A`, `-e`, `-u`, `--wait-time`, `--ping-count`, `--max-hops`, verbosity) so script bindings inherit them as per-call defaults. `convert::anyhow_to_rhai` walks the anyhow chain for `ProtocolExitCode`, stashes the exit code in a thread-local, and formats the error for Rhai; `take_protocol_exit_code` in the engine's error path produces the right process exit (7 for connection-refused, 28 for timeout) from uncaught probe exceptions.

## [0.25.1] - 2026-04-20

### Added

- **Script helpers: `sleep_ms`, `env`, `now`, `now_ms`, `assert`** registered on the Rhai engine. `env(name)` returns an empty string when the variable is unset; `env(name, default)` returns the fallback. `assert(cond, msg)` throws a Rhai exception (exits 1) when `cond` is false. `print` is already provided by Rhai's default engine and writes to stdout.

## [0.25.0] - 2026-04-20

### Added

- **`--script PATH.rhai` scaffolding** — mutually exclusive with the positional URL. Loads and executes a Rhai script; `return N` becomes the process exit code. The engine is currently empty (no bindings) — subsequent 0.25.x patch releases register `http()`, `tcp()`, `dns()`, `tls()`, `redis()`, `ws()`, `ldap()`, and the rest of the probe surface as callable functions with structured return maps. This task ships the flag, module layout, and exit-code wiring so later tasks can layer bindings in without churning the CLI surface.

## [0.24.15] - 2026-04-20

### Changed

- Rewrote `recon --help protocols` to cover every URL scheme shipped in 0.24.0–0.24.14 (file, whois, dns/dig/drill, dict, redis, memcached, ws/wss, ldap/ldaps, rtsp/rtsps) — previously the topic still listed only the six schemes from 0.23.0.
- Rewrote the `PROTOCOL URL SCHEMES` section of `recon --examples` with examples for each new scheme.
- Added `HISTORY.md` entry #27 documenting the 0.24.x batch, the design decisions (ProtocolExitCode reuse across modules, `-d` reuse for redis arbitrary command, bare-URL → server-info UX for dict://, rustls crypto-provider installation pattern, explicit rtsps handshake completion), and the crate choices (tungstenite 0.29, ldap3 0.12, hand-rolled for the rest).

## [0.24.14] - 2026-04-20

### Changed

- **`dict://host/` without a command path now runs a server-info probe** — emits SHOW SERVER, SHOW DATABASES, and SHOW STRATEGIES in sequence. Previously this errored out with "URL needs a command path". The explicit command paths (`/d:WORD`, `/m:WORD`, `/show:…`) continue to work as before. Makes `recon dict://dict.dict.org/` a useful at-a-glance overview of a DICT server, mirroring how `memcached://host/` or `ntp://host/` behave without extra arguments.

## [0.24.13] - 2026-04-20

### Added

- **`redis://` accepts `-d` for an arbitrary RESP command.** Default behaviour (no `-d`) remains PING. With `-d`, recon splits the argument shell-style (whitespace, `"…"`, `'…'`, `\`-escapes) and sends the tokens as a RESP2 command. Examples: `recon redis://localhost -d PING`, `recon redis://localhost -d 'SET foo bar'`, `recon redis://localhost -d 'SET key "hello world"'`. The reply line is labelled with the echoed command, so `CLIENT: +OK` vs `CONFIG: ...` is self-describing.

## [0.24.12] - 2026-04-20

### Added

- **`rtsps://` URL scheme — RTSP over TLS.** Default port 322 (per IANA). Wraps the TCP connection in rustls before sending OPTIONS. Honours `-k` / `--insecure` to skip certificate verification. Listed in the `--version` `Protocols:` banner.

## [0.24.11] - 2026-04-20

### Added

- **`rtsp://` URL scheme.** Sends `OPTIONS <url> RTSP/1.0` over a plain TCP socket (RFC 2326), prints status line + response headers (e.g. `Public:` listing supported methods, `Server:`). Default port 554. Exit 7/28 classification. Hand-rolled; no new dependencies. Listed in the `--version` `Protocols:` banner.

## [0.24.10] - 2026-04-20

### Added

- **`ldap://` and `ldaps://` URL schemes.** Anonymous simple bind, then reads the RootDSE (objectClass=* at scope=base). Reports namingContexts, supportedLDAPVersion, vendorName/vendorVersion, supportedSASLMechanisms. Default ports 389 / 636. Exit 7/28/67 classification. Uses `ldap3` crate with rustls-ring backend. Both listed in the `--version` `Protocols:` banner.

### Dependencies

- Added `ldap3 = "0.12"` with `sync` + `tls-rustls-ring` features (no native-tls).

## [0.24.9] - 2026-04-20

### Added

- **`wss://` URL scheme.** Same semantics as `ws://` but over TLS. Uses tungstenite's `client_tls_with_config` with rustls-webpki-roots. Default port 443. Listed in the `--version` `Protocols:` banner.

## [0.24.8] - 2026-04-20

### Added

- **`ws://` URL scheme.** Opens a WebSocket connection, sends a Ping frame with an 8-byte nonce, waits for matching Pong, closes cleanly. Reports TCP connect latency, handshake latency, selected `Sec-WebSocket-*` headers, and Ping round-trip. Uses `tungstenite`. Exit 0 on successful Ping/Pong, 7 refused, 28 timed out, 67 on 401/403 handshake rejection. Listed in the `--version` `Protocols:` banner.

### Dependencies

- Added `tungstenite = "0.29"` with `rustls-tls-webpki-roots` for `wss://` support in the next release.

## [0.24.7] - 2026-04-20

### Added

- **`memcached://` URL scheme.** Connects over TCP, sends `version\r\n`, reports server version + round-trip. Append `/stats` to also dump `stats` output. Default port 11211. Exit 7/28 classification. Listed in the `--version` `Protocols:` banner.

## [0.24.6] - 2026-04-20

### Added

- **`redis://` URL scheme.** Connects via RESP2, optionally `AUTH`s with a password from userinfo (`redis://:PASSWORD@host`), sends `PING`. Reports connect latency + peer address + PING round-trip. Default port 6379. Exit 0 on PONG, 7 refused, 28 timed out, 67 AUTH rejected. Listed in the `--version` `Protocols:` banner.

## [0.24.5] - 2026-04-20

### Added

- **`dict://` URL scheme (RFC 2229).** Matches curl's URL grammar:
  - `dict://host[:port]/d:WORD[:DB[:STRAT]]` — DEFINE
  - `dict://host[:port]/m:WORD[:DB[:STRAT]]` — MATCH
  - `dict://host[:port]/show:server|databases|strategies|info:DB` — SHOW variants
  Default port 2628. Exit 0 on success, 7 on connect refused, 28 on timeout.
- Listed in the `--version` `Protocols:` banner.

## [0.24.4] - 2026-04-20

### Added

- **`drill://` URL scheme.** Alias for `dns://`. Same semantics. Listed in the `--version` `Protocols:` banner.

### Changed

- Internal: `dns://`, `dig://`, `drill://` dispatch now share one arm via `dns_scheme_rest` helper. No user-visible change.

## [0.24.3] - 2026-04-20

### Added

- **`dig://` URL scheme.** Alias for `dns://`. Same semantics (path shorthand for record types, `--dns-type` override). Listed in the `--version` `Protocols:` banner.

## [0.24.2] - 2026-04-20

### Added

- **`dns://` URL scheme.** `recon dns://example.com` is equivalent to `recon --dns example.com`. Path shorthand for record types: `dns://example.com/MX` or `dns://example.com/A,AAAA`. `--dns-type` flag, if supplied, overrides the path. Listed in the `--version` `Protocols:` banner.

## [0.24.1] - 2026-04-20

### Added

- **`whois://` URL scheme.** `recon whois://example.com` is equivalent to `recon --whois example.com`. Listed in the `--version` `Protocols:` banner.

## [0.24.0] - 2026-04-20

### Added

- **`file://` URL scheme at the top level.** `recon file:///tmp/x.txt` now reads the referenced local file and writes its bytes to stdout (or to `-o <path>` when supplied), matching curl's behaviour. Previously `file://` was only accepted by source-layer features (`--hash`, `--compress`, etc.). The scheme was already listed in the `--version` `Protocols:` banner. Accepts an empty host or `localhost`; rejects other hosts with a clear error.

## [0.23.0] - 2026-04-20

### Added

- **Six new URL-scheme protocols.** All listed in the `--version` `Protocols:` banner:
  - `tls://host[:port]/` — TLS handshake + certificate inspection. Equivalent to `recon --cert https://host:port/`. Default port 443.
  - `ping://host` — ICMP ping. Equivalent to `recon --ping <host>`.
  - `traceroute://host` — Traceroute. Equivalent to `recon --traceroute <host>`.
  - `tcp://host:port/` — **New** TCP connect probe. Reports connect latency + resolved/local addresses. Exits 0 on connect, 7 refused, 28 timed out.
  - `udp://host:port[/path]` — **New** UDP send-and-wait probe. Sends payload from `-d` (or empty) and waits `--wait-time` seconds (default 1s) for any response. Exits 0 regardless of response; UDP silence is ambiguous.
  - `ntp://host[:port]/` — **New** SNTPv4 probe. Reports stratum, reference identifier, offset from local clock, round-trip delay, precision, poll interval, and reference time. Default port 123.
- New flag `--wait-time <SECS>` under a `Protocol Probes` help heading (used by `udp://`).
- `recon --help protocols` topic and `PROTOCOLS (0.23.0)` section under `recon --examples`.

### Changed

- Internal `MqttExitCode` renamed to `ProtocolExitCode` so the exit-code tag is reusable across TCP / UDP / NTP probes. Display strings changed from `mqtt-exit-N` to `exit-N`. User-facing errors unaffected (`friendly_message` uses a typed downcast, not a string prefix).

### Dependencies

- None added — NTP is hand-rolled in ~80 lines (single 48-byte SNTPv4 request + parse).

## [0.22.0] - 2026-04-20

### Added

- **MQTT protocol support** — recon now speaks MQTT 3.1.1 and 5.0 against brokers at `mqtt://` (port 1883) and `mqtts://` (port 8883, TLS via rustls). Three modes:
  - **Probe** (default, no other MQTT flag): connect, dump CONNACK details, disconnect. Works for both protocol versions; MQTT 5 shows richer broker properties (Assigned Client ID, Maximum QoS, Retain Available, Maximum Packet Size, Topic Alias Maximum).
  - **Publish** (when `-d/--data` is set and the URL path has a topic): send one message with configurable `--qos 0|1|2` and `--retain`. QoS 1 waits for PubAck, QoS 2 for PubComp.
  - **Subscribe** (when `--subscribe <filter>` is set, repeatable): stream matching messages to stdout until Ctrl-C or `--count N` is reached. Default output is payload-only; `-v` prefixes topics (mosquitto_sub-style).
- `--mqtt-json` — structured output for probe (one JSON object) and subscribe (NDJSON). Non-UTF-8 subscribe payloads wrap as `{"base64": "..."}`.
- New flags: `--mqtt-version 3|5` (default 5), `--client-id <id>`, `--keepalive <secs>`, `--qos 0|1|2`, `--retain`, `--subscribe <filter>` (repeatable), `--count <N>`, `--mqtt-json`.
- Reuses existing flags: `-u user:pass` (MQTT username/password), `-k` (skip broker TLS verification), `-d` / `@file` / `@-` (publish payload), `--connect-timeout`, `-v`.
- `recon --help mqtt` topic and `MQTT (0.22.0)` section under `recon --examples`.
- `--version` `Protocols:` line now lists `mqtt` and `mqtts`.

### Changed

- `exit_code_for_http_error` extended to cover MQTT: exit 67 on CONNACK auth-failure reason codes (0x86 / 0x87), exit 130 on Ctrl-C during subscribe, exit 7 on TCP connect failure, exit 28 on timeout.
- `friendly_message` now cleanly surfaces MQTT error messages (via typed `MqttExitCode` downcast rather than string-prefix sniffing).

### Known limitations

- mqtts:// TLS is built against rustls 0.22 (pinned by rumqttc 0.24) in addition to recon's direct rustls 0.23 used by HTTPS/tls_probe/serve. Both majors coexist in the binary until rumqttc upgrades; adds ~300 KB. Documented in OUT-OF-SCOPE.md.

### Dependencies

- Added: `rumqttc = "0.24"` (MQTT client), `rand = "0.8"` (direct; client-id suffix), `ctrlc = "3"` (SIGINT handler for subscribe), `webpki-roots = "1"` (Mozilla CA roots for mqtts://, elevated from transitive).

## [0.21.1] - 2026-04-19

### Fixed

- `--version` `Protocols:` line now lists `file` — already-supported (via `src/source.rs::resolve_file_url`, used by `--compress`, `--hash`, `--checkdigit`, etc.) but previously missing from the banner.

## [0.21.0] - 2026-04-19

### Added

- `--version-short` — prints just `recon <version>` (the single-line form previously produced by `--version`). For scripts that only need the number.

### Changed

- `-V` / `--version` now prints a curl-compatible multi-line banner: version line with the underlying `reqwest` / `rustls` majors, `Release-Date:`, `Protocols:`, `Features:`. Enables curl-style introspection like `recon --version | grep HTTP2`. Scripts that previously matched only the first line still work (the first line still starts with `recon <version>`); scripts that expected the entire output to equal `recon <version>` should switch to `--version-short`.

## [0.20.1] - 2026-04-19

### Fixed

- `--json`, `--data-raw`, `--data-binary`, and `--data-urlencode` now auto-promote the HTTP method from GET to POST, matching curl. Previously only `-d/--data` promoted, so e.g. `recon --json '{}' https://api.example.com/` silently sent GET with a body and most servers returned 405. `-G/--get` still keeps the method at GET when combined with any of these flags.

### Changed

- Annotated `ResolvedSample::source_tag` with `#[allow(dead_code)]` — field is read only by unit tests that verify `resolve()` picks the correct source. Documents intent without silencing meaningful warnings.
- Removed unused `sanitize` import from `src/checkdigit/vat/ch.rs`. `cargo build` now emits zero warnings.

## [0.20.0] - 2026-04-19

### Added

- `--json <data>` — curl-compatible JSON shorthand (auto-sets `Content-Type: application/json` and `Accept: application/json` unless overridden via `-H`). Supports `@file` and `@-` (stdin).
- `--data-raw <data>` — send DATA literally, `@file` is NOT processed.
- `--data-binary <data>` — like `-d` but CR/LF are NOT stripped from `@file` content.
- `--data-urlencode <data>` — URL-encode for `application/x-www-form-urlencoded` bodies. Repeatable; joined with `&`. All five curl sub-forms supported: `content`, `=content`, `name=content`, `@file`, `name@file`.
- `--compressed` — request gzip / deflate / brotli / zstd encoding and auto-decompress the response.
- `--max-time <seconds>` — total operation timeout (fractional seconds accepted). Exit code 28 on timeout.
- `--fail-with-body` — like `-f` but writes the body before exiting non-zero on HTTP ≥ 400.
- `--create-dirs` — create missing parent directories for the `-o` output path.
- `--output-dir <dir>` — prefix for `-o` / `-O` output paths.
- `-J` / `--remote-header-name` — with `-O`, derive the filename from the response `Content-Disposition` header (RFC 6266 / RFC 5987). Parser rejects path traversal, null bytes, empty names, and Windows-reserved device names; UTF-8-correct decoding of `filename*=`.
- `--remote-time` — apply response `Last-Modified` as mtime on the saved file (supports IMF-fixdate, RFC 850, asctime).
- `-w` / `--write-out <format>` — format string emitted after the response body. Supports `%{var}` (20+ variables), `%{header{name}}`, `%{json}` (alphabetical JSON blob), `%{stderr}` / `%{stdout}` stream switches, `\n \t \r \\` escapes, and `@file` / `@-` format loading.

### Fixed

- `--connect-timeout` now correctly maps to `reqwest::ClientBuilder::connect_timeout` (connection phase only). Previously it was wired to `.timeout()` (total operation). **Behavior change:** if you relied on `--connect-timeout` as a total-time cap, switch to `--max-time`.
- `-d @file` now strips CR/LF characters from file content, matching curl semantics. Use `--data-binary @file` to preserve them.

### Changed

- HTTP error paths now return curl-compatible exit codes: `28` for timeout, `7` for connect failure, `1` for other errors.
- `-f` / `--fail-with-body` are now represented internally by a `FailMode { Off, OnError, OnErrorKeepBody }` enum for clean runtime dispatch.

### Known limitations

- `-w` connection-phase variables (`time_namelookup`, `time_connect`, `time_appconnect`, `time_pretransfer`) render as `0.000000`. reqwest 0.12's blocking client wraps an async hyper client internally, so clean connector instrumentation is deferred (tracked in [OUT-OF-SCOPE.md](OUT-OF-SCOPE.md)). The accurate variables (`time_total`, `time_starttransfer`, `time_redirect`) work correctly.

### Dependencies

- Added: `filetime = "0.2"`, `httpdate = "1"` (for `--remote-time`).
- Added to dev-dependencies: `tempfile = "3"`, `wiremock = "0.6"` (integration test harness).
- reqwest: added features `gzip`, `deflate`, `brotli`, `zstd` (for `--compressed`). Adds ~500KB to binary from dep duplication (reqwest pulls its own `brotli 8` alongside recon's direct `brotli 7`).

## [0.19.0] - 2026-04-19

### Added

- Non-EU European VAT check-digit support — 13 new country primary keywords:
  - `no-vat` — Norway MVA / orgnr (9 digits, weighted mod-11).
  - `uk-vat` (aliases `ukvat`, `gb-vat`, `gbvat`) — UK VAT (9 or 12 digits);
    dual algorithm (classic mod-97 + 97-55). Post-Brexit GB prefix accepted.
  - `ch-vat` — Swiss UID / IDE (9 digits, weighted mod-11); `CHE-` prefix
    and optional `MWST`/`IVA`/`TVA` suffix handled locally.
  - `li-vat` — Liechtenstein (uses the Swiss UID system; thin wrapper).
  - `ru-vat` — Russian INN; auto-detect 10-digit legal / 12-digit individual.
  - `ru-legal`, `ru-individual` — explicit sub-keywords for Russia.
  - `rs-vat` — Serbian PIB (9 digits, ISO 7064 MOD 11,10).
  - `is-vat` — Icelandic kennitala (10 digits, weighted mod-11).
  - `ua-vat` — Ukrainian; auto-detect 8-digit EDRPOU / 10-digit RNOKPP.
  - `ua-legal`, `ua-individual` — explicit sub-keywords for Ukraine.
  - `tr-vat` — Turkish VKN (10 digits, position-specific transforms).
  - `md-vat` — Moldovan IDNO (13 digits, weighted mod-10).
  - `by-vat` — Belarusian UNP (9 chars, weighted mod-11; alphanumeric
    second-character variant supported).
  - `mk-vat` — North Macedonian EDB (13 digits, weighted mod-11).
  - `me-vat` — Montenegrin PIB (8 digits, weighted mod-11).
- `KNOWN_PREFIXES` grew from 28 (EU-27 + GR) to 42 with the 14 non-EU
  European codes added. `strip_vat_prefix` gained a `GB ↔ UK` alias (mirror
  of the existing `EL ↔ GR` pattern).

### Not implemented

The following jurisdictions were researched but deferred because no
verifiable algorithmic check digit could be found:

- `al-vat` — Albania NIPT. The check letter algorithm is not publicly
  documented; stdnum-js's `al/nipt.ts` explicitly marks the check
  calculation as "not understood".
- `ba-vat` — Bosnia and Herzegovina JIB. No check digit algorithm found
  in any accessible source; neither python-stdnum nor stdnum-js has a
  module.
- `xk-vat` — Kosovo NUI. Newer system (introduced ~2019); no public
  algorithm documentation; no stdnum module exists.

These may ship in a future release if authoritative algorithm documentation
becomes available.

## [0.18.0] - 2026-04-19

### Breaking

- `--checkdigit` verify output now has four pipe-separated fields:
  `<formatted>|<type>|<valid|invalid>|<comment>`. Scripts using
  `cut -d'|' -f1..3` continue to work unchanged; scripts that asserted
  exactly 3 fields will need to accept the trailing pipe.
- VAT aliases renamed for consistency with the `<cc>vat` pattern:
  `svat` → `sevat`, `dvat` → `dkvat`. The old aliases now produce a
  friendly "did you mean …?" error.

### Added

- `comment` field on `Verdict::Valid` surfaces warnings and notes
  alongside a successful verification. Known comments include:
  - Swedish personnummer: "person ≥ 110 years old — likely data entry error".
  - Swedish VAT with non-01 suffix: "suffix NN (unusual — typically 01)".
- All 27 EU VAT keywords now accept input with or without the 2-letter
  country-code prefix. If a different *known* EU prefix is supplied
  (e.g. `DE5261…` under `--checkdigit pl-vat`), the verify errors with
  a clear mismatch message. Greek VAT accepts both `EL` and `GR`
  prefixes on input.

### Fixed

- `valid_ddmmyy` now applies the real leap-year rule when the full year
  is known. Previously, Feb 29 was accepted for all years because only
  2-digit yy was passed. Affects SE personnummer, DK CPR, FI
  henkilötunnus, BG EGN. Where the century genuinely cannot be derived,
  the function still accepts Feb 29 (forgiving fallback).

### Changed

- Swedish VAT accepts any non-`00` 2-digit suffix. Suffix `01` is the
  default; suffixes 02–99 (rare — used when one org.nr has multiple
  VAT-registered entities) now validate with a comment noting the
  unusual suffix.

### Reserved for 0.19.0

Non-EU European VAT / company-ID jurisdictions — NO, UK, CH, IS, LI,
RS, UA, TR, RU, BY, MD, MK, ME, AL, BA, XK.

## [0.17.0] - 2026-04-19

### Added

- Complete EU-27 VAT check-digit coverage — 22 new country algorithms:
  - Primary keywords (auto-detect where the country has sub-variants):
    `at-vat`, `be-vat`, `bg-vat`, `cy-vat`, `cz-vat`, `ee-vat`, `el-vat`
    (alias `gr-vat`), `es-vat`, `hr-vat`, `hu-vat`, `ie-vat`, `it-vat`,
    `lt-vat`, `lu-vat`, `lv-vat`, `mt-vat`, `nl-vat`, `pl-vat`, `pt-vat`,
    `ro-vat`, `si-vat`, `sk-vat`.
  - Explicit sub-variant keywords for multi-variant countries:
    - Spain: `es-nif` (citizen), `es-nie` (foreigner), `es-cif` (entity).
    - Bulgaria: `bg-egn` (personal, date-validated), `bg-bulstat` (legal).
    - Czech Republic: `cz-person`, `cz-legal`.
    - Latvia: `lv-person`, `lv-business`.
- Each country uses its native algorithm — weighted mod-11, weighted
  mod-10, mod-89, mod-97, mod-26 letter lookup, ISO 7064 MOD 11-10,
  or Luhn. Auto-detect surfaces the matched variant in the verify
  output's type field.
- Help topic updated with all 27 EU VAT keywords and sub-variants.
- ~116 new unit tests covering known VAT vectors per country.

### Changed

- `src/checkdigit/vat.rs` (a single file in 0.16.0) is now a directory
  `src/checkdigit/vat/` with one file per country (27 total). Each
  country's algorithm and tests live in a focused file. The five 0.16.0
  countries migrated with no logic change.

### Reserved for 0.18.0

- Non-EU European VAT / company-ID jurisdictions — NO, UK, CH, IS, LI,
  RS, UA, TR, RU, BY, MD, MK, ME, AL, BA, XK — will land as a follow-up
  release.

## [0.16.0] - 2026-04-19

### Added

- `--checkdigit <NAME>` / `--checkdigit-create <NAME>` — verify or compute
  check digits across 40 canonical algorithms (55 total keywords with aliases):
  - Luhn family: `luhn`, `creditcard` (auto-detect), `visa`, `mastercard`
    (alias `mc`), `amex`, `discover`, `jcb`, `imei`, `isin`, `npi`,
    `personnummer` (`se-id`), `sin` (`ca-sin`), `sa-id`.
  - EAN / GTIN family (mod 10 alternating 1×/3×): `ean13` (`ean`),
    `ean8`, `upca` (`upc`), `upce`, `isbn13`, `gtin8`, `gtin12`,
    `gtin13`, `gtin14` (`gtin`), `sscc`.
  - `isbn10` — mod 11, allows `X` as check digit.
  - Personal IDs via mod 11: `cpr` (`dk-id`), `bsn` (`nl-id`),
    `fodselsnummer` (`no-id`, two check digits). Post-2007 Danish CPRs
    may legitimately fail the check — noted in the help topic.
  - `henkilotunnus` (`fi-id`) — Finnish mod-31 with extended 2023 century
    markers (`A`-`F` for 2000s).
  - `iban` — mod 97 with 80+ country length table.
  - `vin` — transliterate + weighted mod 11; check at position 9; `I`,
    `O`, `Q` disallowed.
  - `mrz` — ICAO Doc 9303 passport / ID MRZ (TD1/TD2/TD3 formats).
  - `aba` (`us-routing`) — US bank routing number.
  - Cryptocurrency: `btc` (`bitcoin`), `ltc` (`litecoin`), `doge`
    (`dogecoin`) — base58check; `eth` (`ethereum`, `eip55`) — EIP-55
    mixed-case; `bech32` (`segwit`) — BIP-173.
  - EU VAT starter (5 countries): `se-vat`, `dk-vat`, `fi-vat`, `de-vat`,
    `fr-vat`. Remaining 22 EU countries + non-EU European jurisdictions
    reserved for 0.17.0.
- `--checkdigit-list` — standalone action: prints the full algorithm /
  alias table.
- `--raw` — strips grouping from output (applies to `--checkdigit` and
  `--checkdigit-create` only).
- New `recon --help checkdigit` help topic with complete documentation.
- New `CHECK DIGITS` section in `recon --examples`.
- New dependencies: `bs58 = "0.5"`, `bech32 = "0.11"`. Existing `sha2`
  and `sha3` reused for Bitcoin SHA-256d and Ethereum Keccak-256.

### Changed

- New `src/checkdigit/` module directory (17 files) following the same
  pattern as `src/email/` and `src/serve/`. 121 new unit tests.

### Internal

- `source::read_all` is now used by `--checkdigit` (matching `--hash` /
  `--encrypt`'s source-layer integration).

## [0.15.2] - 2026-04-19

### Changed

- `--help` output is now colorized (when stdout is a TTY): section headings
  (`HTTP Request:`, `Auth & TLS:`, etc.) and the `Usage:` line render in
  yellow + bold, flag names (`-H, --header`) in cyan, and argument
  placeholders (`<URL>`, `<PATH>`) in green. Matches the existing
  `--examples` palette. ANSI codes are suppressed when output is piped.

## [0.15.1] - 2026-04-19

### Changed

- `--help` output is now grouped into labeled sections (`HTTP Request`,
  `Auth & TLS`, `Output`, `Certificate Inspection`, `DNS`, `WHOIS`,
  `Network Tests`, `Email Protection`, `Cookies`, `File Server`, `JWT`,
  `Hashing`, `Compression`, `Encoding`, `Encryption`, `Sample Data`,
  `Editor`, `Meta`) matching the existing `--help <topic>` taxonomy.
  No flag names changed; only the rendering of `--help`.

### Added

- `CHANGELOG.md` at the repo root, covering versions 0.15.0 → 0.7.0
  (lifted from `HISTORY.md`), plus reconstructed entries for 0.6.0
  (JWT), 0.5.0 (SSH/Telnet), 0.4.1 (DMARC cross-validation fix), and
  0.1.0 (initial curl-clone baseline). Versions 0.2.0–0.4.0 are
  documented chronologically in `HISTORY.md`'s Feature Additions
  narrative.

### Removed

- The `## Version Log` section of `HISTORY.md`. Its content lives in
  `CHANGELOG.md` now. `HISTORY.md` retains the design-notes narrative
  (Origins, Architecture Decisions, Feature Additions, Naming History,
  Module Structure, Dependencies).

## [0.15.0] - 2026-04-19

### Added

- `--encrypt` / `--decrypt`: age-format encryption and decryption over any
  source (file, URL, stdin, file://). Output to stdout or `-o <FILE>`.
  Binary by default; `--armor` produces ASCII-armored output for paste
  into email or chat.
- `--passphrase-file <PATH>`, `$RECON_PASSPHRASE` env var, and an
  interactive hidden prompt as the three passphrase sources (priority in
  that order). The `--passphrase <TEXT>` literal flag is intentionally not
  offered (security footgun).
- `--recipient <AGE1... | PATH>` (repeatable): encrypt to one or more
  X25519 recipients. Mix with a passphrase in the same invocation; any
  one recipient or the passphrase decrypts.
- `--identity <PATH>` (repeatable): decrypt using an age private-key
  file. Skips blank and `#`-comment lines.
- `--encrypt-keygen`: standalone action that generates a fresh X25519
  key pair (age-compatible). Prints the public key as a comment and the
  private key on its own line.

## [0.14.1] - 2026-04-18

### Fixed

- `--encode code128`: no longer requires users to manually prepend a
  code-set marker (À/Ɓ/Ć). Code-set B (mixed ASCII) is now automatically
  selected when the input does not already start with one.
- `--encode ean13`: now accepts 12-digit input (body without check) in
  addition to the full 13-digit code; the check digit is computed automatically.
- `--encode upca`: now accepts 11-digit input (body without check) in
  addition to the full 12-digit code; the check digit is computed automatically.

## [0.14.0] - 2026-04-18

### Added

- `--encode <FORMAT>`: generate a QR, DataMatrix, Code 128, Code 39,
  EAN-13, or UPC-A code from the positional text. Use `-` or a pipe to
  read from stdin; `--from-file <PATH>` reads from disk.
- `--encode-format <ascii|svg|png>`: output format. When omitted, inferred
  from `-o <FILE>` extension (`.svg` / `.png`); defaults to ASCII otherwise.
- `--from-file <PATH>`: read encode input from a file instead of the
  positional argument. Mutually exclusive with a positional text value.
- `--encode-list`: standalone action listing the supported formats.

## [0.13.0]

### Added

- `--compress <ALGO>`: compress the input source (file, URL, stdin, file://)
  with gzip, deflate, zstd, brotli, or bzip2. Streams bytes; output goes to
  stdout or -o <FILE>.
- `--decompress [ALGO]`: decompress the input source. Without ALGO, the
  first 6 bytes are inspected and gzip / zstd / bzip2 are auto-detected.
  Deflate and brotli lack magic bytes; pass the algorithm explicitly for
  those.
- `--compression-level <LEVEL>`: quality for --compress. Accepts a number
  in the algorithm's native range or a word (fastest, fast, default, good,
  best). Invalid with --decompress.
- `--compress-list`: standalone action listing supported algorithms with
  their aliases, magic bytes, and level ranges.

## [0.12.0]

### Added

- `--hash <ALGO>`: compute a cryptographic hash of any source — local file,
  `file://` URL, HTTP(S) URL, or stdin. Supported: md5, sha1, sha256,
  sha384, sha512, sha3-256, sha3-512, blake3 (case-insensitive; hyphens
  and underscores accepted). HTTP sources honour every usual HTTP flag
  (`-H`, `-u`, `-L`, `-k`, `-A`, cookies, `-e`).
- `--hash-format <hex|base64|raw>`: digest output format. Default is
  lowercase hex. `raw` writes binary bytes with no trailing newline.
- `--hash-list`: standalone action that lists supported algorithms and
  their digest sizes. Does not require a URL.

### Changed

- Internal: new `src/source.rs` module unifies input-source handling
  (file, URL, stdin, `file://`) for feature flags that consume arbitrary
  bytes. No user-visible change on its own; backs `--hash` and the
  upcoming `--compress` / `--encrypt` / `--qr` / `--barcode` flags.

## [0.11.0]

### Added

- `-e, --referer <URL>`: send a Referer header (alias `--referrer` for the
  common misspelling). Overridden by any explicit `-H "Referer: …"`.
- `-O, --remote-name`: save the response body to a file named after the
  URL's final path segment (mirrors `curl -O`). Percent-decodes the name;
  errors when the URL has no filename or when the name would escape the
  current directory. Mutually exclusive with `-o/--output`.
- `-T, --upload-file <PATH>`: upload a local file as the request body.
  Defaults the method to PUT unless `-X` is set explicitly. Mutually
  exclusive with `-d/--data`.

### Changed

- `-X/--request` is now `Option<String>`; the effective method is resolved
  through a new `Args::effective_method()` helper that honours explicit
  `-X`, then `-T` (PUT), then `-d`/`-G` (POST/GET), then defaults to GET.
  No user-visible behaviour change for existing invocations.

## [0.10.1]

### Changed

- `--sample lorem` output now always begins with the words "Lorem ipsum" —
  the first paragraph's opener is fixed, the first two words in word mode
  are fixed, and character mode starts with the prefix (truncated if
  `--sample-count` is smaller than 11). Remaining content stays random.

## [0.10.0]

### Changed

- `--sample lorem` now produces randomized output instead of deterministic
  rotation through the corpus. Each invocation yields different text.

### Added

- `--sample-seed <N>`: optional u64 seed for lorem generation. When set,
  output is reproducible for that seed + count + unit. When omitted, a
  seed is derived from the current system time.
- Using `--sample-seed` with a non-lorem sample is a loud error.

## [0.9.0]

### Added

- `--sample <NAME[:FORMAT[:COUNT]]>`: fetch sample ecommerce data from known
  free APIs. Built-in names: `customer`, `product`, `order`, `category`,
  `address`, `image`, and a local `lorem` generator. Composes with `-p`,
  `-o`, `--editor`, `-i`, and verbose.
- `--sample-format <FMT>` and `--sample-count <N[p|w|c]>`: override format
  and count independently of the colon shortcut. Unit suffixes (`p`, `w`,
  `c`) are only valid for the local `lorem` sample; passing them to other
  samples is an error.
- `--sample-file [PATH]`: write sample output to file(s). Default filename
  is `sample-{{name}}.{{format}}` (bulk) or `sample-{{name}}-{{n}}.{{format}}`
  (per_item). Required when `per_item` sample is fetched with count > 1.
- `--sample-list`: standalone action listing all available samples (built-in
  plus user-configured). Does not require a URL.
- `[sampledata.<name>]` config sections in `~/.recon/config.toml`: add or
  override samples with `mode`, `default_format`, `count`, `urls.<fmt>`,
  `headers`, `basic_auth`, and `description`. `${ENV_VAR}` substitution is
  honored in URL, header, and basic_auth strings so secrets stay out of the
  config file.

## [0.8.0]

### Added

- `--editor [EDITOR]`: redirect response output to a file and open it in an editor.
  Accepts a built-in alias (`zed`, `code`, `cursor`, `subl`, `vim`, `nvim`, `nano`,
  `emacs`), a user-defined alias from `~/.recon/config.toml`, or a raw shell
  command. When the flag is given with no value, falls back to
  `[editor] default` in the config. The temp file is written to
  `/tmp/recon-<unix-ms>.<ext>` with the extension derived from the response
  `Content-Type`. By default stdout is silent; `-vv` or higher also mirrors the
  body to stdout.
- `--editor-cleanup`: remove all `/tmp/recon-*` temp files from previous
  `--editor` invocations. Standalone action — does not require a URL.
- `[editor]` config section in `~/.recon/config.toml`:
  `default` (optional alias used when `--editor` has no value) and
  `[editor.aliases]` (optional map of user-defined alias → command).

## [0.7.3]

### Changed

- SNI directory parsing now accepts both `<hostname>-cert.pem` and `<hostname>.pem` as certificate files when paired with `<hostname>-key.pem`, improving compatibility with mkcert-style output.

## [0.7.2]

### Changed

- `--serve-sni` now accepts an omitted value and defaults to `~/.recon/sni/`, so `recon --serve-sni` uses that directory automatically.

## [0.7.0]

### Added

- `--netstatus`: connectivity checker that runs configurable probes concurrently
  and exits non-zero if any check fails (suitable for scripting with `--silent`).
- `~/.recon/config.toml`: new general config file; `[netstatus]` is the first section.
- Supported probe schemes: `http://`, `https://`, `ping://`, `tcp://`, `tls://`,
  `dns://`, `ntp://`.
- Public IP cross-check: fetches from multiple sources and flags disagreement.
- `[[netstatus.dns_hijack_checks]]`: per-server DNS hijack detection with expected-IP assertion.

## [0.6.0]

### Added

- `--jwt-view`, `--jwt-sign`, `--jwt-validate`: sign, validate, and inspect JWT
  tokens without leaving the terminal. `--jwt-view` decodes and pretty-prints
  the header and payload without signature verification. `--jwt-sign` signs a
  JWT from a JSON payload, partial token (header.payload), or bare base64
  payload; `iat` is added automatically if missing. `--jwt-validate` verifies
  the HMAC signature and, with opt-in flags, checks individual claims.
- Claim flags for signing and validation: `--jwt-iss`, `--jwt-sub`, `--jwt-aud`,
  `--jwt-exp`, `--jwt-nbf`, `--jwt-iat`, `--jwt-jti`.
- Validation toggles: `--jwt-validate-exp`, `--jwt-validate-nbf`,
  `--jwt-validate-iat`, `--jwt-validate-iss`, `--jwt-validate-sub`,
  `--jwt-validate-aud`, `--jwt-validate-jti`, `--jwt-validate-full`.
- Supporting flags: `--jwt-secret` (HMAC secret), `--jwt-alg` (HS256, HS384,
  HS512; default HS256), `--jwt-json-report` (machine-readable output for
  `--jwt-view` and `--jwt-validate`).
- Input for JWT operations can come from `-d <string>`, `-d @file`, a
  positional file path, or stdin.

## [0.5.0]

### Added

- `ssh://[user@]host[:port]`: interactive SSH PTY shell on the remote server.
  Reuses the existing SCP auth stack (agent → key → password, host key
  verification via `~/.ssh/known_hosts`). Terminal resize is forwarded via
  SSH `window-change` requests. Shared auth helpers extracted into
  `src/ssh_auth.rs`.
- `telnet://host[:port]`: Telnet client with full IAC option negotiation per
  RFC 854. Accepts `WILL ECHO` and `WILL SUPPRESS-GO-AHEAD` from the server;
  rejects all others with DONT/WONT. Subnegotiation blocks are discarded.
  `0xFF` bytes in input are escaped as `IAC IAC`.

## [0.4.1]

### Fixed

- `--dmarc` run on its own no longer emits spurious `[⚠ WARN]` cross-validation
  notes suggesting the user add `--spf` or `--dkim`. Those notes were
  suggestions, not findings, and cluttered output when only DMARC was
  requested. The DMARC+SPF and DMARC+DKIM "not checked" entries were removed
  from `cross_validate()`.

## [0.1.0]

### Added

- Initial curl-clone baseline: HTTP/HTTPS requests (`GET`, `POST`, `PUT`,
  `DELETE`, `PATCH`, `HEAD`), custom headers (`-H`), request body (`-d`,
  `@file`), redirect following (`-L`, `--max-redirs`), file output (`-o`)
  with progress bar, silent (`-s`) and verbose (`-v`) modes, response
  header inclusion (`-i`), custom User-Agent (`-A`), connection timeout
  (`--connect-timeout`), fail-on-HTTP-error (`-f`).

---

*Versions 0.2.0 through 0.4.0 were developed before release tags were recorded.
Intermediate features (TLS certificate inspection, DNS, WHOIS, ping, traceroute,
redirect header tracing, prettification, status-only output, cookies, SCP,
email protection checks, per-topic help, file server, SNI, output model
overhaul) landed across that range and are documented chronologically in
[HISTORY.md](HISTORY.md) §4–§19.*
