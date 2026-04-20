# Changelog

All notable changes to recon are recorded here. Format based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

For pre-0.4.1 design context and architectural notes, see [HISTORY.md](HISTORY.md).

## [Unreleased]

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
