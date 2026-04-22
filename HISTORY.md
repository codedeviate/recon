# recon — Project History & Design Notes

## Overview

**recon** is a versatile network reconnaissance CLI tool written in Rust. It started as a basic curl clone and evolved into a multi-protocol network investigation tool covering HTTP/HTTPS requests, TLS certificate inspection, DNS lookups, WHOIS queries, ping, and traceroute.

---

## Versioning

recon follows semantic versioning (`MAJOR.MINOR.PATCH`). Release notes for
each version are kept in [CHANGELOG.md](CHANGELOG.md). The rules governing
version bumps:

- **MINOR** — a new feature or flag is added, removed, or significantly changed.
- **PATCH** — bug fixes, documentation/help text updates, and other changes
  that do not add or remove features or flags.
- **MAJOR** — reserved for breaking changes to existing behaviour.

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

### 42. Script parity for encode / encrypt / checkdigit / sample / jwt / email / netstatus (0.40.0)

Closes the remaining script-binding gaps. Seven new static modules, one per feature group that had a CLI flag but no script surface.

**encode.** `encode::qr` / `datamatrix` / `barcode` wrap the existing `src/encode.rs::encode()` + renderers. Returns PNG Blob by default; `encode::encode(fmt, data, "ascii"|"svg"|"png")` picks the output form. Zero refactor needed — the core already took primitive inputs.

**encrypt.** Added two new `pub` helpers to `src/encrypt.rs`: `encrypt_bytes_recipients(plain, recipients, armor)` and `decrypt_bytes_identities(ciphertext, identity_paths)` — in-memory wrappers around age's writer/reader APIs. Script binding delegates to them. `keygen()` directly uses `age::x25519::Identity::generate()` and `.expose_secret()`. Passphrase mode is CLI-only because prompting interactively doesn't fit a script context; users who need passphrase encryption can shell out to `recon --encrypt`.

**checkdigit.** The registry already exposed `SPECS` (array of `&'static Spec` with `verify_fn` / `create_fn` function pointers) and `resolve(name)`. Binding calls them directly — no refactor.

**sample.** The CLI's generation path is request-based (fetches from remote URLs). For scripts, generation is redundant with `http()`, so the binding is informational only: `list()`, `spec(name)`, `url(name, format)` surface the built-in registry metadata. Documented as a deliberate design choice.

**jwt.** Three primitives already existed: `parse_input`, `sign_claims`, `check_token`. The binding wraps them with a Rhai Map ↔ serde_json::Value converter (essentially the same shape as the `json_stringify` converter shipped in 0.27.0). Signature verification via `check_token` returns a Vec of `CheckResult` — the binding aggregates into a single `#{valid, checks, header, payload}` map.

**email.** Each of the six checks (SPF/DMARC/DKIM/MTA-STS/BIMI/TLS-RPT) is an `async fn check(&TokioAsyncResolver, host [, selector [, insecure]]) -> anyhow::Result<CheckResult>`. Binding builds a throwaway current-thread runtime per call, spins a default-config hickory resolver inside it, awaits the check. The futures aren't `Send` (hickory internals), so the binding uses non-Send `Pin<Box<dyn Future>>` — fine because the current-thread runtime doesn't need Send. `email::all(host)` runs five of them back-to-back in one runtime for efficiency.

**netstatus.** Promoted `probe_http` and `probe_tcp` to `pub(crate)` (they already returned a structured `ProbeResult`). Binding runs a default set (HTTP to example.com + TCP to 1.1.1.1:443 + TCP to 8.8.8.8:53) in `check()` and aggregates via the existing `overall_status()` helper. Individual probes exposed for custom configurations.

**Verification.** Single smoke script that exercises all 5 local-only bindings (qr, Luhn verify, sample list, JWT round-trip, age keygen) passes end-to-end. Email + netstatus need network; ignored tests are `#[ignore]` by default.

**Tests added: +23.** 945 → 968 total, 2 ignored.

### 41. Custom DNS resolvers (0.39.0)

Third of three curl-compat round-out releases. HTTP requests gain a custom DNS path that doesn't go through the system's `getaddrinfo`.

**`--dns-servers <LIST>`**. Comma-separated `IP` or `IP:PORT`. Parser accepts both forms; default port 53. Each server is registered twice in hickory's `ResolverConfig` — once as UDP, once as TCP — matching the behaviour of the default system resolvers (UDP with TCP fallback on truncation).

**`--dns-ipv4-addr` / `--dns-ipv6-addr`**. Bind addresses for outgoing DNS queries. Set on `NameServerConfig::bind_addr` per-protocol-family. When one of these is set without `--dns-servers`, the resolver defaults to `1.1.1.1:53` rather than the system servers — system resolvers don't honour the bind address, so inheriting them would silently void the user's setting. Better to be explicit.

**`--dns-interface`**. Accepted but not plumbed. hickory 0.24's public API binds via `SocketAddr` only; interface-name binding would need a custom socket factory using `SO_BINDTODEVICE` (Linux) or `IP_BOUND_IF` (macOS). Rather than ship a half-working flag, recon errors out with a pointer at `--dns-ipv4-addr` / `--dns-ipv6-addr`. Documented in OUT-OF-SCOPE.

**Resolver impl.** `CustomResolver` wraps an `Arc<TokioAsyncResolver>`. `Resolve::resolve` returns `Box::pin(async move { inner.lookup_ip(host).await.map(...) })` — delegates directly to hickory's native async API. No blocking shim needed because reqwest's blocking client runs the connector inside a current-thread runtime it owns.

**reqwest compatibility wrinkles.** `reqwest::dns::Name` doesn't impl `Display` — `as_str()` method returns `&str`. Both `tls_config` and certain hickory `NameServerConfig` fields in reqwest's own examples are version-skewed; hickory 0.24 dropped `tls_config` from the public struct. Caught both during the first build iteration.

**Script parity.** Four opts keys on `http(url, opts)`: `dns_servers`, `dns_ipv4_addr`, `dns_ipv6_addr`, `dns_interface`. Same behaviour as CLI — `dns_interface` errors out even from scripts.

**Smoke verification (four cases pass):** `--dns-servers 1.1.1.1` against example.com succeeds; unreachable `127.0.0.1:8` fails with connection timeout after `--connect-timeout`; `--dns-interface eth0` gives the "not yet plumbed" error; malformed input errors at parse time. 945 tests passing; 6 new for nameserver parsing.

### 40. Rate control: `--limit-rate`, `--speed-limit`, `--speed-time` (0.38.0)

Second of three curl-compat round-out releases. Throttling + slow-transfer abort for HTTP downloads.

**`--limit-rate <RATE>`** → `RateLimitedWriter` wraps the output path in `output.rs`. On each `write()`, the wrapper computes the wall-clock time the pinned rate would have required for the bytes-so-far and sleeps the delta. Simple, low-jitter. Parse accepts curl's grammar: `100K`, `2M`, `1.5G`, `500B`, bare bytes. K/M/G/T multipliers are 1024-based; trailing `B` is tolerated; unknown suffix = error.

**`--speed-limit <BYTES>` + `--speed-time <SECS>`** → `SpeedWatchWriter` samples throughput on each write (checks capped at once per second). When the rolling average stays below `speed_limit` B/s for the entire `speed_time` window, the next write returns `io::ErrorKind::TimedOut`. First `speed_time` seconds are grace (TCP ramp-up can undershoot).

**Layered composition.** Both wrappers implement `Write` over `Box<dyn Write + 'a>`, so `--limit-rate` + `--speed-limit` together produces `SpeedWatchWriter<RateLimitedWriter<W>>`. `wrap_with_rate_control` in output.rs builds the chain conditionally. Lifetime parameter `'a` lets us wrap the `StdoutSink::writer()` return (borrowed lifetime) as well as the owned `File` case.

**Script parity.** ScriptDefaults + http opts overlay gain three fields: `limit_rate` (string), `speed_limit` (i64), `speed_time` (i64, default 30). Scripts get the same throttling + slow-abort knobs on every `http()` / `https()` call.

**Smoke verification.** 10 KB download @ `--limit-rate 5K` took 2.9s (expected ~2s + TLS/DNS overhead). Bad suffix (`100X`) errors with "unknown suffix 'x'". 939 tests passing; 5 new for parse + writer-wrap behaviour.

### 39. TLS minimum version + `--cacert` + `--interface` (0.37.0)

First of three curl-compat round-out releases. Four flags, all thin wrappers over reqwest's `ClientBuilder`:

- **`--tlsv1.2` / `--tlsv1.3`** → `ClientBuilder::min_tls_version(Version::TLS_1_2 | TLS_1_3)`. One-line add. Both flags together: the higher minimum wins (1.3 beats 1.2).
- **`--cacert <PATH>`** → read PEM, parse `reqwest::Certificate::from_pem`, `add_root_certificate`. Trust-additive (doesn't replace the system store), so self-signed corporate CAs slot in without also disabling verification with `-k`.
- **`--interface <IP>`** → `ClientBuilder::local_address(ip)` — IP literal only. Interface names (`eth0`, `en0`) require OS-specific lookup (`if_nametoindex` + `getifaddrs` on Unix, `GetAdapterAddresses` on Windows) that isn't worth the platform split until someone asks. Error message is explicit about the literal-only constraint.

**Script parity.** `ScriptDefaults` gets four new fields (`tlsv12`, `tlsv13`, `cacert`, `interface`); `http_binding::build_args` overlays them from per-call opts; the `flags` global visible to scripts picks them up too. Scripts that already set `insecure: true` now have the full TLS-knob set available in the same opts map.

**Deferred rationale updated in OUT-OF-SCOPE.md.** `--key-type` moved from "unimplemented curl flag" to its own entry noting it needs full client-cert support first. `--cert-status` similarly marked needing a custom rustls `ServerCertVerifier`.

### 38. Script parity for compression + archive (0.36.0)

Closes the script-parity gap retroactively for the 0.34.0 and 0.35.0 compression / archive work. Also establishes the policy going forward: every new CLI feature ships a Rhai binding alongside.

**`compression` static module.** All nine algorithms (the five from 0.13.0 + four from 0.34.0) exposed as `compression::compress(algo, blob [, level])` and `compression::decompress([algo,] blob)`. Both delegate to the same `crate::compression::compress` / `decompress` path the CLI uses — `Box<dyn Read>` over an in-memory `Cursor<Vec<u8>>`. Level arg accepts either an integer (per-algo native range) or a word (fastest/fast/default/good/best) via the existing `parse_level` / `resolve_native_level` machinery. Level-less algos (lz4, snappy) throw when a level is passed. `decompress(blob)` without an algo argument auto-detects via `detect_from_magic`; for deflate/brotli (no signature), it throws with a hint.

**`archive` static module.** `create(dest, sources)` and `extract(src, dest_dir)` wrap `crate::archive::create` / `extract` one-to-one, including the extension-based format detection and magic-byte fallback for `extract`. Sources come in as a Rhai Array of path strings. Both functions return the file count as `i64`. `detect(path)` returns the format label (`"zip"` / `"tar.gz"` / …) or `()`.

**Rhai `set_native_fn` quirk.** In rhai 1.24, `Module::set_native_fn` requires closures to return `Result<T, Box<EvalAltResult>>` — plain-value returns don't satisfy the `RhaiNativeFunc<_, _, _, _, true>` trait bound. Fn closures like `compression::list` (infallible) and `detect` (also infallible) had to be wrapped in `Ok(...)` to compile. Worth noting for future static-module additions.

**Policy shift.** Up to now, script bindings were registered function-by-function as new primitives landed. Going forward, any new CLI flag gets a matching script surface in the same release. The three upcoming curl-compat releases (TLS in 0.37.0, rate control in 0.38.0, DNS overrides in 0.39.0) follow this — each adds both a CLI flag and an opts-map field on the relevant script binding.

### 37. Archive tools: `--archive` / `--extract` (0.35.0)

Ships the zip / tar / tar.gz / tar.xz / tar.bz2 archive workflow as two unified CLI flags rather than four or six format-specific ones. `--archive DEST FILE...` creates, `--extract SRC [-o DIR]` unpacks. Format inferred from the extension: `.zip`, `.tar`, `.tar.gz` / `.tgz`, `.tar.xz` / `.txz`, `.tar.bz2` / `.tbz2`.

**Unified flags vs per-format flags.** The alternative was `--zip` / `--unzip` / `--tar` / `--untar` + a `--tar-compress` companion. That grows the flag surface and forces users to memorise which flag pairs with which. Extension-based detection cleanly maps user-intent onto the filename they type anyway — `recon --archive backup.tar.gz ...` is self-describing without needing a `--compress=gzip` follow-up.

**Trailing positional sources via argv pre-split.** Clap's derive model binds the first positional to `Args.url`; a Vec field would eat everything after, fighting with url. The `--script`-era solution was to pre-split argv before clap sees it — `split_script_trailing` in cli.rs splits on `--script PATH`, everything after populates `script_args`. This release extends that function to also split on `--archive DEST`. Both flags share the same `script_args` Vec as a "trailing positional sources" slot, with mutual-exclusion enforced implicitly by the dispatch order in main.rs (archive checked before script). No new clap attributes, no fight with `url`'s positional binding.

**Magic-byte fallback for `--extract`.** Extension-based detection is primary, but some downloads arrive with opaque names (`.dat`, `.bin`). `detect_from_magic` reads the first 512 bytes and checks for PK\x03\x04 (ZIP), 1f 8b (gzip → tar.gz assumed), fd 37 7a 58 5a 00 (xz → tar.xz), BZh (bzip2 → tar.bz2), and `ustar` at offset 257 (uncompressed tar). Detection is used only when extension-based detection returns None, so extension wins when both are present (which covers the common case of archives named correctly).

**In-module walkdir.** Recursive directory listing is a 30-line `std::fs::read_dir` recursion. Adding the `walkdir` crate for this alone didn't feel worthwhile — the in-module helper under `archive::walkdir` does the job, returns a simple `Vec<Entry>`, and keeps the dep tree one crate smaller.

**Omitted deliberately.** Password-protected ZIP (zip crate supports AES but adds aes/hmac crates to the tree), symlink / xattr preservation beyond crate defaults, include/exclude patterns, list / dry-run mode, stdin / stdout streaming, 7z / rar (no mature pure-Rust library for either), multi-volume archives.

**Deps added: two.** `zip = "2"` (features `deflate`, `bzip2`; no AES) and `tar = "0.4"`. `flate2` / `xz2` / `bzip2` are all reused from 0.34.0, no new stream-compression deps.

### 36. Four more stream-compression algos: lz4, xz, snappy, zlib (0.34.0)

Picks up the four long-parked OUT-OF-SCOPE items from the compression track. Five → nine algorithms in `--compress` / `--decompress`. Each one slotted into the existing `Algo` / `parse_algo` / `compress` / `decompress` machinery with minimal surface change:

- **lz4** via `lz4_flex` (pure Rust). Frame format (the streaming variant, distinguishable from the block format by its `04 22 4d 18` magic). The encoder is **writer-side** in lz4_flex (wraps the output, not the source), so `compress()` grew a special arm that handles lz4 via `std::io::copy(source, encoder)` before the general `Box<dyn Read>` match. Everyone else is read-side.
- **xz** via `xz2`. Full 0-9 level range like gzip. `xz2::read::XzEncoder` / `XzDecoder` fit the read-side pattern directly.
- **snappy** via `snap`. Frame format. No level setting — another level-less algo.
- **zlib** via the existing `flate2`. No new dep; just exposes the already-linked `ZlibEncoder` / `ZlibDecoder`.

**Levelless-algo handling.** Lz4 and Snappy don't expose a level knob. Rather than silently ignore `--compression-level`, `Algo::is_levelless()` is a new method; the `run_compress` entry point checks it and errors out with a clear message when the user passes a level against one of them. `level_range()` returns `(0, 0)` and `default_level()` returns `0` for these two.

**Zlib magic-byte detection.** Unique in not having a constant prefix. Per RFC 1950 §2.2, the header is CMF (always 0x78 in practice — 32 KB window + deflate compression method) + FLG where the big-endian 16-bit composition must be divisible by 31. `detect_from_magic` grew a special-case arm after the table loop that checks exactly this. Gzip (`1f 8b`) and zlib (`78 xx`) remain distinguishable.

**Dep cost.** Three new direct deps (`lz4_flex`, `xz2`, `snap`), plus `xz2` pulls `lzma-sys` for the C library. ~24s build impact on a clean target — acceptable for production value.

**OUT-OF-SCOPE.md cleanup.** The `Compression (0.13.0): lz4, xz, snappy, zlib` line is removed in this release.

### 35. agent-browser bindings + `--browser-screenshot` flag (0.33.0)

Scripts gain browser-automation primitives by wrapping the external `agent-browser` CLI (a playwright-ish tool distributed via Homebrew / npm / cargo). Rather than link a browser driver into recon (huge dep surface), the binding is a thin shell-out to `agent-browser <subcommand>`. A `--browser-screenshot URL` CLI flag exposes the most common one-shot flow without needing a script.

**Static module over import.** Registered via `Engine::register_static_module("agentBrowser", module)`, so scripts write `agentBrowser::open(...)` without any `import` statement. The user-facing status constants `agentBrowser::available: bool` and `agentBrowser::version: String` are module-level variables set via `Module::set_var` at engine build time. Function wrappers are attached with `Module::set_native_fn`. Name conflicts handled: Rhai reserves `type`, so we expose `type_text` instead; `is visible/enabled/checked` becomes `is_visible/is_enabled/is_checked` (same pattern as other predicate renames in the codebase).

**Graceful degradation.** Availability detection runs once at first access via `OnceLock<AgentBrowserState>` — calls `agent-browser --version` and parses the output. When `!available`, `run_cmd` still compiles the argv but the Command spawn returns `NotFound`, which the error path converts to "agent-browser: binary not found on PATH. Install via ...". Scripts can gate the whole block with `if !agentBrowser::available { return 2; }`; uncaught errors surface the install hint.

**JSON envelope unwrapping.** agent-browser's JSON output is `{success, data, error}`. The wrapper's `run_json` helper strips the envelope: `success:true` → return `data`, `success:false` → throw with the error message, no envelope → pass through. Saved scripts ~2 levels of nested map access per call. Predicate wrappers (`is_visible`, etc.) pull the matching key (`visible`, `enabled`, `checked`) out of `data` and coerce to bool.

**Shared module layout.** `src/agent_browser.rs` owns state + `run_cmd` + `run_screenshot_cli`. Both `src/script/bindings/agent_browser.rs` (the Rhai binding) and the `--browser-screenshot` intercept in `main.rs` delegate to it. Keeps the CLI flag path out of the script-bindings tree — matches the layering of other shared logic (`cert.rs`, `source.rs`).

**Project `script/` folder.** New top-level directory with `README.md` and five reference scripts. Users can run them in place (`recon --script script/browser-title.rhai https://example.com`) or copy into `~/.recon/script/` for bare-name invocation. Every script starts with the guard pattern so `agent-browser` being missing produces a clean exit 2 rather than a runtime error.

**Validation step.** Before committing Task 1 we ran: availability probe (expect available=true), real browser flow (open + screenshot + close, confirm PNG on disk), CLI-flag path (produces 1280×577 PNG), graceful-degradation path (`PATH=/usr/bin:/bin` hides the binary; `available=false`, exit 0), error-on-call path (calling a function when unavailable produces the install hint). All five pass.

**Out of scope.** iOS simulator coverage (`-p ios`), interactive `chat`/`stream`/`skills` commands, env-override options (`AGENT_BROWSER_PROFILE`, `AGENT_BROWSER_HEADED`), install/profiles/device subcommands. Can add later if demand materialises.

### 34. Rhai `import` support with global-dir fallback (0.32.0)

Scripts gain a way to share logic: `import "name" as alias;` now works. Two resolvers chained via `ModuleResolversCollection`:

1. **Default `FileModuleResolver`** — resolves relative to the importing script's directory. Natural for sibling imports (`import "helpers"` from `/tmp/foo.rhai` finds `/tmp/helpers.rhai`). Also handles absolute paths and `../` traversals.
2. **Fallback `FileModuleResolver::new_with_path(~/.recon/script/)`** — picks up shared modules for scripts living outside the global dir.

Scripts already in the global dir are handled by resolver 1 (their directory IS the global dir), so resolver 2 is a noop in that case — no special-casing.

**Why the `ast.set_source(path)` change?** Rhai's default `FileModuleResolver` resolves relative paths against whichever of (a) its own `base_path` or (b) the AST's source path is set. Without `set_source`, there's no source — resolver 1 sees `base_path = None`, no source, falls back to `PathBuf::default()` (empty), appends `"name.rhai"`, tries to open `./name.rhai` from CWD, fails. Switching from `engine.eval_with_scope(source_text)` to `engine.compile_with_scope + ast.set_source(script_path) + engine.eval_ast_with_scope` makes resolver 1 see the importing script's directory. Took a failed integration test (exit code 1, "Module not found") to catch this — worth noting because the Rhai docs don't call it out loudly.

**Why a collection rather than a custom resolver?** The `ModuleResolversCollection::push` + `FileModuleResolver` combo covers the desired behaviour in ~8 lines. A custom `ModuleResolver` impl would be tempting for smarter path juggling but adds maintenance surface with no win for this use case.

**Out of scope.** No `RECON_SCRIPT_PATH=dir1:dir2` multi-path, no per-project `./recon_modules/`, no HTTPS-URL modules, no module signing.

### 33. Auto-paging for help and examples (0.31.0)

`recon --examples` prints ~1000 lines; `recon --help script` prints ~80. Both scrolled past unread unless users piped manually. This release routes those outputs through `$PAGER` (default `less -FRX`) when stdout is a TTY, matching git's convention. Short outputs (`recon --help version`) still appear instantly because `less -F` exits when content fits on one screen — so auto-paging isn't disruptive for small help topics.

**Implementation.** `src/pager.rs` spawns the pager with stdin piped, then calls `libc::dup2` to point `STDOUT_FILENO` at the pager's stdin. After that, every `println!` in `help::render_topic` and `examples::print` flows through the pipe. The Child is held by the caller so it's not reaped mid-output; when the caller's scope ends (function returns), the Child drops, the pipe closes, the pager sees EOF and exits cleanly (or continues waiting for keystrokes if content doesn't fit).

**Colour preservation.** The `colored` crate auto-strips ANSI escapes when stdout isn't a TTY — and after dup2, our stdout is a pipe, not a TTY. `pager::activate` calls `colored::control::set_override(true)` immediately after dup2 so escapes keep flowing; `less -R` renders them. Without the override, paged help would be monochrome.

**Control surface.** Three opt-outs:
- `--no-pager` flag (mirrors `git --no-pager`).
- `$RECON_NO_PAGER` env var (for shell profiles / CI images).
- `$PAGER=cat` (or any other binary) overrides the default command; also a de-facto opt-out when set to something trivial.

Paging is automatically skipped when stdout isn't a TTY (redirects, pipes), when the configured pager fails to spawn (missing binary), or when `libc::dup2` returns an error (rare, but we kill the spawned pager and fall through).

**Platform.** Unix-only. The `activate` function is `#[cfg(unix)]`; Windows gets a no-op stub. Proper Windows paging would need `more.exe` or a cross-platform pager crate — deferred until demand materialises.

**Dep impact.** Added `libc = "0.2"` as a direct dep. It was already transitive via tokio/reqwest/ssh2; making it direct surfaces one call site (`dup2`) without pulling any new code into the tree.

### 32. `recon --init` — bootstrap `~/.recon/` layout (0.30.0)

Adds a one-shot `--init` flag that materialises the directory layout and config file users eventually need. Idempotent: every action prints `created`, `wrote`, or `skipped (exists)`, so re-running after a partial setup fills in the blanks without touching edits.

**Subdirectory scope** (user-locked). Creates all three recon-managed subdirs: `script/`, `jars/`, `sni/`. `jars/` and `sni/` would otherwise appear lazily when `--cookiejar` or `--serve-sni` first writes to them — pre-creating them is a discoverability win (users `ls ~/.recon/` and see the layout) at the cost of two empty dirs the user may never touch. Not creating: TLS assets (`cert.pem`, `key.pem`), age passphrase file — those are user-owned data that init has no business manufacturing.

**Config skeleton** (user-locked: fully-commented). ~35 lines of TOML with every section commented out and an example row or two per section. Parses cleanly as `ReconConfig::default` (tested), so the file is a docs artefact until the user uncomments something. Covers `[editor]`, `[editor.aliases]`, `[netstatus]`, `[[netstatus.dns_hijack_checks]]`, `[sampledata.NAME]`. Deliberately no `version` field — `ReconConfig` doesn't carry one yet and this patch isn't the place to introduce schema versioning.

**Implementation shape.** `src/init.rs` with `run()` (resolves `$HOME`) and `init_at(home)` (pure, injectable — tests target a tempdir without mutating the process environment). Two helpers: `ensure_dir` and `ensure_file`. Paths are open-coded against the home dir rather than calling into `config::config_path()` and `script::script_dir()` — init is allowed to know the layout directly; keeping the path computation local avoids visibility churn on the other modules.

**Out of scope.** No `--force` / `--overwrite` (user explicitly said skip-don't-overwrite). No cert generation. No migration hook.

### 31. SQLite script bindings — `sqlite(spec [, mode])` (0.29.0)

Scripts can now open SQLite databases and run arbitrary SQL. `sqlite("/path.db")` opens a file; `sqlite(":memory:")` creates an ephemeral handle; `sqlite("cookiejar")` and `sqlite("cookiejar:NAME")` resolve to recon's own jar files at `~/.recon/jars/NAME.db` (default jar when no name is given). The handle exposes four methods: `query` / `query_one` / `query_value` / `exec`.

**Handle type.** `SqliteHandle { conn: Rc<RefCell<Connection>>, path: Rc<PathBuf> }`. `Clone` (cheap Rc bump), `!Send` (matches Rhai's default config). Registered via `engine.register_type_with_name::<SqliteHandle>("SqliteHandle")` and four `register_fn` arms per method name (two arities per method: with and without params array). Dropping the last clone closes the connection.

**Spec resolution.** Three-branch: (1) `:memory:` → `Connection::open_in_memory`, (2) contains `/`, `\`, or ends with `.db` → literal path, (3) else alias lookup. Aliases are `prefix[:arg]`; `cookiejar` is the only alias in the first cut, but the match arm is trivial to extend. Windows-style `C:\foo.db` hits the literal-path branch via the backslash check, so colon-form aliases don't clash.

**Read-write default for `cookiejar`.** User picked this over read-only despite the footgun risk — scripts that insert / delete rows on recon's own jar are rare but deliberate. Scripts that want read-only can pass `"ro"` as the second arg, or the literal filesystem path with `"ro"`.

**Parameter binding.** Positional `?` only (no named binding). `()` → NULL, `bool` → INTEGER 0/1, `i64` → INTEGER, `f64` → REAL, String → TEXT, Blob → BLOB. Unsupported types throw with an "index N" pointer so scripts can find the bad arg.

**Row conversion.** `rusqlite::types::ValueRef` → Rhai Dynamic: Null → `()`, Integer → i64, Real → f64, Text → String (via lossy UTF-8), Blob → Rhai Blob. Column names come straight from `Statement::column_names()`; multi-column queries return one key per column even if two columns share a name (second wins — rare in practice).

**Omitted deliberately.** No transactions / `begin`/`commit`, no named parameters, no prepared-statement reuse, no ATTACH, no PRAGMA helpers. Two-line fixes when scripts actually demand them.

### 30. Script CLI introspection — `args` and `flags` constants (0.28.0)

Scripts gain two read-only globals: `args` (Rhai array) and `flags` (Rhai map). `args[0]` is the `--script` value as the user typed it (so `recon --script health` exposes `"health"`, not the resolved `~/.recon/script/health.rhai` path — matches the "argv[0] is what was typed" convention scripts everywhere inherit from Unix). `args[1..]` are trailing positional arguments. `flags` surfaces the subset of CLI flags that `ScriptDefaults` also captures, plus `-d/--data` and `-o/--output`.

Implemented via `rhai::Scope::push_constant` — the natural Rhai idiom for injecting host-provided top-level values. Scripts reading `args[0]` or `flags.insecure` get them; scripts trying to mutate (`args.push(…)`) get a Rhai error. `run_file` now calls `eval_with_scope` instead of `eval`.

**argv split decision.** Clap's derive model assigns the first trailing positional to the first declared positional field. `Args` already has `url: Option<String>` at the top, which would happily swallow `recon --script foo bar` → `url = Some("bar")`. Rather than fight clap's positional ordering (which would require `#[arg(last = true)]` forcing a `--` separator, or reshuffling all of Args), `main.rs` now splits argv on the `--script PATH` boundary before clap runs: everything up to and including PATH goes to clap; everything after becomes `script_args`. Clean: no new clap attributes, trivial to test, handles both `--script PATH` and `--script=PATH` forms. The `script_args` field is `#[arg(skip)]` so clap ignores it.

Shared helper `Args::parse_with_script_split(argv)` lets both `main.rs` and unit tests run the same argv-split-then-parse pipeline.

**Exposed flag scope.** The set mirrors `ScriptDefaults::from_args` verbatim, extended with `data` and `output`. Mode flags (`--hash`, `--encode`, `--cookiejar`, serve config, mqtt config) are deliberately excluded — they don't apply in script mode and exposing them would mislead. Missing optional scalars become `()` so scripts can `if flags.user_agent != () {}` without `contains_key` noise; always-present fields (`headers`, bool flags, numeric flags) always hold a usable default.

### 29. Script hashing + pretty-printed JSON (0.27.0)

Scripts gain two small but frequently-needed capabilities: hash digests as a function call, and a `json_stringify` prettify variant.

**Hashes — shared with the CLI.** Every algorithm `--hash` already supports is now also a Rhai function: `md5(x)`, `sha1(x)`, `sha256(x)`, `sha384(x)`, `sha512(x)`, `sha3_256(x)`, `sha3_512(x)`, `blake3(x)`. CRC32 joins the list on both surfaces (`--hash crc32` + `crc32()` in scripts) via a new `crc32fast` dep. A generic `hash("sha256", x)` and `hash("sha256", x, "base64")` complement the per-algo forms for loops. Input accepts both String (UTF-8 bytes) and Rhai Blob, so `md5(file_read("...")` works without conversion.

Under the hood, `hash::digest_string(algo, bytes, format)` is a new shared helper that both the CLI's `--hash` path and the script bindings delegate to. Keeps "how to compute a digest and format it" in one place.

**`json_stringify` overloads.** Previously one-arg only (compact output). Now also:
- `json_stringify(v, true)` → 2-space pretty (uses `serde_json::to_string_pretty`).
- `json_stringify(v, false)` → same as compact (lets callers feature-flag).
- `json_stringify(v, n)` for integer n → n-space pretty (1..=8 clamped; `n <= 0` falls back to compact — so `json_stringify(v, is_verbose ? 4 : 0)` is a clean toggle).

Implementation uses `serde_json::Serializer::with_formatter(PrettyFormatter::with_indent(&spaces))` for the integer-indent path.

**Decisions:**
- **CRC32 added to both CLI and scripts** (user pick). Symmetry with the rest of the algo list over script-only scope. Dep cost (`crc32fast`) is ~zero.
- **`hash(algo, x)` format defaults to hex, not base64.** Matches `--hash`'s default (Format::Hex) and the most common script use (checksum comparison).
- **Raw-bytes format omitted from scripts** — `digest_string(_, _, Format::Raw)` would produce lossy strings. Scripts should stay on hex/base64; those who want raw bytes should cast digest → blob via `file_read` workflows.
- **Per-algo function names use underscores** (`sha3_256`, not `sha3-256`) because Rhai identifiers don't allow hyphens. The generic `hash("sha3-256", x)` accepts all three hyphen / underscore / no-separator variants via `hash::parse_algo`.

### 28. Embedded Rhai scripting engine — `--script PATH.rhai` (0.25.0 → 0.25.18)

An embedded scripting layer that turns recon into a single-binary Bruno/Postman alternative. A Rhai script can call every probe recon ships as a function returning a structured map, chain requests, branch on results, loop, and produce a process exit code via `return N`. `--script` is mutually exclusive with a positional URL.

Shipped as nineteen incremental patch releases (0.25.0 through 0.25.18). Each release lands one binding + changelog entry; the pattern follows the same TDD-lite cadence the 0.24.x batch used.

**Crate pick: Rhai.** Embedded, sandboxed, no stdlib of its own, scripts stay data-driven. Default features used (no `sync` — the engine is `!Send` / `!Sync` which matters for tests that cross tokio's `spawn_blocking` boundary; see notes below). Clean rustls story — Rhai pulls nothing in the network layer.

**API surface registered on the engine:**
- HTTP: `http(url)`, `http(url, opts)`, `https(...)`, `request(opts)` — full curl semantics (cookies, redirects, body, headers), returns `#{url, final_url, status, body, headers, http_version, duration_ms}`.
- Probes (one per protocol recon supports): `tcp`, `ping`, `dns`, `tls`, `ntp`, `redis`, `ws` / `wss`, `dict`, `ldap` / `ldaps`, `whois`, `memcached`, `rtsp` / `rtsps`, `mqtt_pub` / `mqtt_sub`, `file_read`.
- Helpers: `print` (Rhai built-in), `sleep_ms`, `env`, `now`, `now_ms`, `assert`, `json_parse`, `json_stringify`.

**Probe-extraction pattern.** Task 5 established the template the rest of the tasks follow: for each probe module that previously did "network work + stdout + return unit", extract a pure `probe()` / `fetch_*()` function that returns a typed result struct, then rewrite `run()` as `probe() + print`. Across twelve modules this adds ~600 lines net (struct definitions + thin `run()` wrappers) but the pure cores are now reusable by the script binding. The one deliberate exception is `mqtt.rs` (44KB): mqtt bindings wrap `mqtt::run` by synthesising an `Args` struct, so mqtt_sub's protocol output still flows through stdout rather than being collected into the return map. Carving a pure-collection subscribe codepath out of `mqtt.rs` is deferred.

**Errors and exit codes.** Task 3 installed a thread-local `LAST_PROTOCOL_EXIT_CODE` that `convert::anyhow_to_rhai` populates when it walks the anyhow chain: if a frame is a `ProtocolExitCode` (MQTT/RTSP/etc. tag that from the 0.22.0 design), that integer is stashed; otherwise `reqwest::Error::is_connect()` / `is_timeout()` falls back to 7 / 28. The engine's top-level error path consumes the stash when an uncaught exception bubbles out of the script. So `https("bad://")` or `tcp("tcp://127.0.0.1:1")` raises a Rhai exception that — if the script doesn't `try`/catch it — produces the same process exit code the CLI's `exit_code_for_http_error` would have.

**CLI-flag inheritance.** `ScriptDefaults::from_args` snapshots the relevant Args fields once at engine-build time (`-H`, `-k`, `--connect-timeout`, `--max-time`, `-L`, `-A`, `-e`, `-u`, `--wait-time`, `--ping-count`, verbosity, method). Each binding captures a `Clone` of this struct in its closure, then overlays per-call opts maps on top. This matches the user's explicit brainstorm preference ("inherited as defaults") over the alternative "scripts are self-contained" — so `recon -H 'X-Api-Key: abc' --script flow.rhai` behaves the way an HTTP-heavy user expects.

**Test shape.** Unit tests for the HTTP binding hop into `tokio::task::spawn_blocking` because blocking reqwest inside a wiremock tokio runtime panics ("Cannot drop a runtime in a context where blocking is not allowed"). Inside the spawn_blocking closure we build the engine and extract `(status, body)` as plain types, since `rhai::Map` is `!Send` and can't cross the boundary. Tests for ldap, memcached, mqtt, rtsp, dict, ntp, whois are live-network-gated or minimal (protocol-specific mocks are out of scope for this release).

**What's not in 0.25.x (deliberate).**
- No async/await in scripts — all probes are blocking, same as the CLI.
- No `file_write` — scripts are read-only. Principle of least surprise.
- No remote script URLs (`--script https://...`) — security hazard for the first cut.
- No sandbox config for network egress — scripts can hit anything recon can.
- No structured per-message return from mqtt_sub (see above).

### 27. Second protocol batch — file, whois, dns/dig/drill, dict, redis, memcached, ws/wss, ldap/ldaps, rtsp/rtsps (0.24.0 → 0.24.14)

A wide second pass on recon's protocol surface, shipped as fifteen incremental patch releases. Brings the `--version` `Protocols:` banner from 14 entries to 25 and covers the remaining commonly-needed URL schemes that curl or its adjacent tools expose.

Rough groupings:

- **Aliases for existing flags** (no new behaviour, just URL-scheme entry points): `file://` (curl-style), `whois://`, `dns://` / `dig://` / `drill://`. The DNS family accepts a path-as-type shorthand (`dns://example.com/MX,AAAA`) overridden by `--dns-type` when both are supplied. `dig://` and `drill://` fold into a single `dns_scheme_rest` helper so all three share one dispatch arm.
- **Standalone probes (hand-rolled, no new deps)**: `dict://` (RFC 2229 with curl's full URL grammar — `/d:WORD[:DB[:STRAT]]`, `/m:WORD[:DB[:STRAT]]`, `/show:server|databases|strategies|info:DB`), `memcached://` (text protocol `version` + optional `/stats`), `rtsp://` / `rtsps://` (OPTIONS request over TCP / TLS, port 322 for rtsps per IANA).
- **Standalone probes pulling one crate**: `redis://` (RESP2, connect + `PING`, optional `AUTH` from URL userinfo, optional arbitrary command via shell-split `-d`), `ws://` / `wss://` (tungstenite 0.29; TCP connect → HTTP Upgrade → Ping frame with nonce → wait for matching Pong), `ldap://` / `ldaps://` (ldap3 0.12, anonymous simple bind + RootDSE at scope=base).

Design choices worth noting:

- **`ProtocolExitCode` tag reused everywhere.** All new probe modules attach `.context(ProtocolExitCode::…)` for curl-compat exit codes (7/28/67/130). The typed chain-walking lookup established for MQTT in 0.22.0 handles them without changes.
- **`-d` as the natural extension point for redis command passthrough.** Rather than introducing `--redis-cmd`, reuse `-d`; split shell-style (whitespace + `"…"` + `'…'` + `\`-escapes), send as a RESP2 array, label the reply line with the echoed command for self-describing output. Mirrors the UDP probe's reuse of `-d`.
- **`dict://host/` with no command path = server-info probe.** Originally errored out; now emits SHOW SERVER + SHOW DATABASES + SHOW STRATEGIES in sequence, matching the bare-URL UX of `memcached://host/` and `ntp://host/` (you point at a server, it tells you what's there).
- **rustls crypto-provider installation is now lazy per module.** `ws_probe` and `rtsp_probe` each call `rustls::crypto::ring::default_provider().install_default().ok()` on their TLS path. Idempotent; keeps recon's rustls usage consistent with `tls_probe.rs` and `mqtt.rs`.
- **`ldap3` pulls its own `tls-rustls-ring` feature;** rustls 0.23 matches recon's direct version so no dual-major situation (unlike MQTT via rumqttc). Tungstenite 0.29 also lands on rustls 0.23, clean.
- **`rtsps://` handshake is completed explicitly** (`ClientConnection::complete_io`) before the first write, so cert-verify failures surface as "TLS handshake with HOST failed" rather than "write OPTIONS over TLS" (the error you get if the handshake is only triggered by the first write).

The `--version` protocol list is alphabetized: `dict dig dns drill file http https ldap ldaps memcached mqtt mqtts ntp ping redis rtsp rtsps scp ssh tcp telnet tls traceroute udp whois ws wss`.

Each release shipped after `cargo build && cargo build --release && cargo test` all passed, and each new probe was smoke-verified against a real server — public (dict.dict.org, ldap.forumsys.com, ws.postman-echo.com, pool.ntp.org), local daemon (redis-server, memcached), or a nc-simulated server (RTSP). Tests 767 → 811.

---

### 26. Protocol URL schemes batch (0.23.0)

Six new URL-scheme probes round out recon's protocol surface: three convenience aliases for existing flags (`tls://` ≈ `--cert`, `ping://` ≈ `--ping`, `traceroute://` ≈ `--traceroute`) and three new standalone probes (`tcp://`, `udp://`, `ntp://`).

Design choices worth noting:

- **Unified exit-code tag.** `MqttExitCode` → `ProtocolExitCode`. All probe failures carry the same `anyhow` context tag, which `main.rs::exit_code_for_http_error` walks via the typed chain lookup established for MQTT. TCP / UDP / NTP error classification reuses the existing `is_connect_io_kind` helper from `mqtt.rs` for consistent `io::ErrorKind` → curl-compat exit code mapping.
- **Hand-rolled SNTPv4.** A single 48-byte request with a straightforward response parse; no point in pulling in `sntpc` just for that. Reports stratum, reference identifier (ASCII code for stratum 1, IPv4 of upstream peer for stratum ≥ 2), offset, round-trip delay, precision, poll interval, reference timestamp. RFC 4330 §5 offset/delay formulas.
- **UDP semantics are deliberately weak.** UDP has no connection; "port reachable" is ambiguous. The probe sends one datagram, waits `--wait-time` seconds, and reports whatever it received (or explicitly reports ambiguous silence). Exit 0 in all cases unless `send_to` itself fails.
- **Thin TLS/ping/traceroute wrappers** route through existing modules via tiny `parse_plain_host` / `rewrite_tls_scheme` helpers. No duplication.

This batch completes the original user-requested protocol list (`tcp`, `udp`, `ntp`, `tls`, `mqtt`, `mqtts`, `ping`, `traceroute`) — `mqtt` / `mqtts` were already in place from 0.22.0.

---

### 25. MQTT protocol support (0.22.0)

recon gains a first-class MQTT client covering the three common use cases against a broker: probe (recon's characteristic "connect and report" shape), publish, and subscribe. Both MQTT 3.1.1 and 5.0 are supported, selected via `--mqtt-version`.

Key design choices:

- **URL-driven mode dispatch**: probe is the default, `-d` + URL-topic means publish, `--subscribe <filter>` means subscribe. Reuses recon's existing `-u`, `-k`, `-d @file`, `-v`, `--connect-timeout` semantics verbatim — no fork in CLI grammar.
- **rumqttc sync wrapper over tokio** — rumqttc 0.24 is async-native; we spin up a current-thread tokio runtime per operation via the shared `build_mqtt_runtime` helper. Keeps the MQTT module's public surface blocking (matches the rest of recon) without a crate-wide async migration.
- **Shared setup helpers** — `setup_options_v5` / `setup_options_v3` / `build_mqtt_runtime` collapsed four initial copies of the MqttOptions assembly into one place before subscribe would have added two more. Explicit dedup commit landed pre-subscribe.
- **Dedicated `--mqtt-json` flag** instead of overloading `--json` (which takes a value for HTTP body). Cleaner than the alternative and unambiguous.
- **`MqttExitCode` context tag** — errors attach `.context(MqttExitCode::...)` so `exit_code_for_http_error` can map to curl-compatible exit codes (7 connect-refused, 28 timeout, 67 auth-denied, 130 Ctrl-C) alongside the existing reqwest classifications. Chain-walked rather than top-only downcasted so a future `.context(...)` wrap cannot hide the tag.
- **`writeout.rs`-style token helpers** — publish topic from URL path, subscribe filters from repeatable flag. `emit_subscribe_text` and `emit_subscribe_json` mirror the `-w` renderer split (text vs JSON) established in 0.20.0.

rumqttc 0.24 pins rustls 0.22, while recon's HTTPS stack (via reqwest) uses rustls 0.23. Rather than adding a second direct `rustls` dep, the MQTT module aliases `rumqttc::tokio_rustls::rustls as mqtt_rustls` internally. Both majors coexist in the binary; adds ~300 KB until rumqttc bumps (tracked in OUT-OF-SCOPE.md).

MQTT 5 features deferred to OUT-OF-SCOPE.md for now: user properties, will/testament messages, session expiry interval, content-type / response-topic / correlation-data properties, enhanced authentication. Client certificates (mTLS) deferred consistent with HTTP's current surface.

---

### 24. curl compatibility quick-wins batch (0.20.0)

Twelve high-frequency curl flags shipped in one release, making recon a credible curl drop-in for the 80/20 HTTP(S) use case. Architectural foundations introduced to support this batch (and future telemetry work) rather than one-off wiring:

- **`RequestMetrics` + `PhaseTiming`** (`src/metrics.rs`) — central per-request instrumentation struct. Fields: start/end timestamps, size counters (upload / download / header), redirect count + URL, response snapshot (status / version / headers). Populated incrementally by the client during the request/response lifecycle; consumed by the `-w` renderer. The `phase: Arc<Mutex<PhaseTiming>>` handle will hold DNS / TCP / TLS phase durations once the connector-instrumentation work lands (deferred per OUT-OF-SCOPE.md).
- **`FailMode` enum** (`src/fail.rs`) replacing the `-f` bool. Three states — `Off` / `OnError` / `OnErrorKeepBody` — clarify the three-way contract between `-f` and `--fail-with-body`. The response-handling flow branches on mode: `OnError` aborts before body write; `OnErrorKeepBody` writes body first, then returns error so the process still exits non-zero.
- **`writeout.rs`** — dedicated format-string parser + renderer. Token enum is a public data type (`Literal`, `Variable`, `Header`, `Json`, `StderrSwitch`, `StdoutSwitch`) so future templating features can reuse it. Parser is char-based (preserves UTF-8 in literals); renderer reads metrics only (no live Response), making it composable with the body-consuming write path.
- **`remote_name.rs`** — stand-alone RFC 6266 Content-Disposition parser. Prefers `filename*=` (RFC 5987 extended form) with UTF-8-correct percent-decoding; sanitizes against path traversal, null bytes, and Windows-reserved device names; parameter-boundary-aware matching rejects `x-filename=` and quoted-value false-positives.
- **Bug fix with behavioral impact:** `--connect-timeout` had been wired to reqwest's total-operation timeout (`.timeout()`) since the flag was introduced; 0.20.0 corrects this to `.connect_timeout()`, and introduces `--max-time` for the total-operation slot. Users who depended on the old behavior need to migrate to `--max-time`.

Roadmap note: this is tier 1 of the planned 0.20.0 → 0.3x.0 curl-compat expansion. Future tiers (not yet specced) include `--limit-rate`, `--resolve`, advanced retry, HTTP/3, and the connector-instrumentation work that will fill in `-w`'s phase timings.

---

### 23. Non-EU European VAT Check-Digit Support (0.19.0)

13 new country-code VAT / company-ID check-digit algorithms covering the non-EU
European jurisdictions (NO, UK, CH, LI, RU, RS, IS, UA, TR, MD, BY, MK, ME).

- Four multi-variant algorithms with auto-detection: `ru-vat` (10-digit legal / 12-digit
  individual), `ua-vat` (8-digit EDRPOU / 10-digit RNOKPP), each with explicit
  sub-keywords (`ru-legal`, `ru-individual`, `ua-legal`, `ua-individual`).
- UK VAT supports dual check-digit algorithms (classic mod-97 and 97-55) and
  accepts both `GB` and `UK` prefixes on input (`GB ↔ UK` alias mirroring `EL ↔ GR`).
- Swiss UID handles `CHE-` prefix and optional `MWST`/`IVA`/`TVA` suffix; Liechtenstein
  is a thin wrapper over the same algorithm.
- `KNOWN_PREFIXES` grew from 28 to 42 with the 14 new non-EU country codes.
- Three jurisdictions deferred: Albania NIPT (no verified check letter algorithm),
  Bosnia JIB (no algorithm in any public source), Kosovo NUI (no public documentation).

---

### 22. JWT Tokens (`--jwt-view`, `--jwt-sign`, `--jwt-validate`) (0.6.0)

Sign, validate, and inspect JWT tokens without leaving the terminal.

- `--jwt-view`: Decode and display JWT header and payload as pretty-printed JSON. No signature verification.
- `--jwt-sign`: Sign a JWT from a JSON payload, partial token (header.payload), or bare base64 payload. Adds `iat` = now automatically if missing. Claim flags (`--jwt-iss`, `--jwt-sub`, `--jwt-aud`, `--jwt-exp`, `--jwt-nbf`, `--jwt-iat`, `--jwt-jti`) inject claims only when not already present.
- `--jwt-validate`: Verify the JWT HMAC signature. Without extra flags: signature check only. Opt-in claim checks via `--jwt-validate-exp`, `--jwt-validate-nbf`, `--jwt-validate-iat`, `--jwt-validate-iss`, `--jwt-validate-sub`, `--jwt-validate-aud`, `--jwt-validate-jti`. `--jwt-validate-full` enables all checks. Exits non-zero if any check fails.
- `--jwt-secret <SECRET>`: HMAC secret (required for sign and validate).
- `--jwt-alg <ALG>`: Algorithm override: HS256 (default), HS384, HS512.
- `--jwt-json-report`: Machine-readable JSON output for `--jwt-view` and `--jwt-validate`.
- Input from `-d <string>`, `-d @file`, file path positional argument, or stdin.

---

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

### 21. SSH Interactive Shell & Telnet Client (`ssh://`, `telnet://`) — 0.5.0

**SSH:** `ssh://[user@]host[:port]` opens a fully interactive PTY shell on the remote server. Reuses the existing SCP auth stack (agent → key → password, host key verification via `~/.ssh/known_hosts`). Terminal resize is forwarded via SSH `window-change` requests. Shared auth helpers extracted into `src/ssh_auth.rs`.

**Telnet:** `telnet://host[:port]` connects a Telnet client with full IAC option negotiation per RFC 854. Accepts `WILL ECHO` and `WILL SUPPRESS-GO-AHEAD` from the server; rejects all others with DONT/WONT. Subnegotiation blocks are discarded. `0xFF` bytes in input are escaped as `IAC IAC`.

**Both** use a non-blocking single-threaded event loop driven by `crossterm` key and resize events. Raw terminal mode is restored via RAII guard even on panic. Connection timeout (`--connect-timeout`) is respected.

**New dependency:** `crossterm = "0.28"` for raw terminal mode and event handling.

---

### 20. Bug fix: spurious cross-validation warnings when running `--dmarc` alone (0.4.1)

Running `--dmarc` without `--spf` or `--dkim` previously produced `[⚠ WARN]` cross-validation entries suggesting the user add those flags. These were suggestions, not real findings, and cluttered the output when only DMARC was requested. The DMARC+SPF and DMARC+DKIM "not checked" notes have been removed from `cross_validate()`.

Also added `CLAUDE.md` to the repository with versioning instructions so Claude Code applies the correct PATCH/MINOR/MAJOR bump automatically on each change.

---

### 19. SNI Multi-Certificate Support (`--serve-sni`)

**Goal:** Allow the HTTPS server to present different certificates based on the hostname the client requests (Server Name Indication).

**Flag:** `--serve-sni <MAPPING>` — repeatable, auto-detects three formats:
- **Inline:** `hostname:cert.pem:key.pem` (contains `:`)
- **Directory:** path to a directory containing `<hostname>-cert.pem` and `<hostname>-key.pem` files
- **Config file:** path to a file with `hostname cert.pem key.pem` lines

**Behaviour:** Implies `--serve-tls` with default port 443. Multiple values can be mixed. Unmatched hostnames use the default cert (`~/.recon/cert.pem`) if it exists, otherwise the TLS handshake fails.

**Implementation:** Custom `ResolvesServerCert` trait implementation with a hostname→CertifiedKey HashMap and optional default fallback. Uses `rustls::crypto::ring::sign::any_supported_type` for key loading.

**Module introduced:** `src/serve/sni.rs`

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
