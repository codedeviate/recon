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

### 80. Shell subprocess binding — `shell()` and `shell_stream()` (0.87.0)

**What:** Two script-engine callables for running external commands.
`shell()` blocks and returns a Map of stdout/stderr/exit_code/success.
`shell_stream()` fires a Rhai FnPtr callback once per merged
stdout+stderr line as the child writes it, returning the exit code at
the end. Both accept either a String (run via `sh -c` / `cmd /C`) or
an Array (literal argv); both take an opts Map with `cwd`, `env`,
`env_clear`, `timeout_ms`, `merge_stderr`.

**Why this is its own module rather than another helper.** The
existing pattern in `bindings/helpers.rs` is for one-line wrappers
around stdlib calls (`env`, `now`, `sleep_ms`). Subprocess handling
needs three threads (stdout drain, stderr drain, wait), an mpsc
channel for line forwarding, and timeout-aware blocking — too much
state to belong in `helpers.rs`. A dedicated `bindings/shell.rs`
keeps that complexity local and gives Phase 2 (TUI panes) a clear
foothold.

**Why string-input goes through the platform shell by default.** The
target use case is "run a list of update commands" — `brew upgrade`,
`npm -g update`, `cargo install-update -a`, `apt-get update`. People
write those with `&&` chains, pipes, and `$VAR` expansion; they
don't want to remember to invoke `sh -c` for every call. Direct
argv is still one keystroke away (`shell([...])`) and is the right
default when the command name comes from untrusted input.

**Why the streaming callback runs on the script's own thread.**
Rhai's engine is `Send + Sync` under the `sync` feature, but calling
a `FnPtr` from a non-script thread requires re-entering the engine
with a fresh evaluation context — error-prone and surprising
(captured scope wouldn't resolve the way users expect). The
implementation instead spawns two forwarder threads that just push
String lines into an mpsc, and the main thread loops on `rx.recv`
inside the native function, invoking
`callback.call_within_context(ctx, (line,))` per arrival. That keeps
all Rhai-engine work on the same thread, captured variables behave
exactly like they do in a synchronous loop, and reentrancy
semantics match the array-method conventions (`.map`, `.filter`).

**Why timeout is whole-call, not per-line.** A common subprocess
(`brew upgrade`) might be silent for 30s while it resolves
dependencies, then write a burst of lines. A per-line idle timeout
would have to be set conservatively high enough to tolerate those
quiet stretches, defeating its purpose. A whole-call deadline is
predictable: "fail after N ms total" is what users want when they
add a timeout at all.

**Why stderr is always merged in `shell_stream`.** Separating them
would require either two callbacks (awkward) or a tagged line type
(invasive). Users staring at live output read the natural arrival
order anyway — the OS already serialises writes from the child's
two pipes by interleaving them at the read-end. Blocking `shell()`
still keeps them separate because parse-the-result workflows often
care which stream a line came from.

**What's deferred to Phase 2.** No TTY allocation, so commands that
prompt for input (`apt-get install` without `-y`, `passwd`) appear
to hang. No PTY means interactive tools also detect "not a terminal"
and switch to non-coloured / different-formatted output — not a bug,
but a known limitation. The TUI pane primitive that lands in the
next slice doesn't fix this either; full PTY support is a separate
ticket.

### 79. Script-engine string helpers, error hints, and the metadata feature (0.83.0–0.85.0)

**What:** Three connected additions to the script engine and REPL.

- 0.83.0: REPL meta-commands `:save-tidy` (compiles each history entry
  through the engine, appends missing `;`, drops entries that don't
  parse — the resulting file runs as a script) and `:functions` /
  `:function-list` (enumerates every callable the engine knows about).
- 0.84.0: a new `bindings/strutil.rs` module exposing eleven PHP-style
  free functions: `trim` / `ltrim` / `rtrim`, `strrev`, `strip_html`,
  `nl2br` / `br2nl`, `preg_match` / `preg_replace`, `printf` /
  `sprintf`. The `regex` crate became a direct dep (already in the
  lockfile transitively via the markdown/PDF path, so no new compile
  units).
- 0.84.1: a new `script/error_hint.rs` module that intercepts Rhai's
  `EvalAltResult::ErrorFunctionNotFound` and, when the name *is*
  registered under a different signature, appends an "Available
  overloads:" list before the error reaches the user.
- 0.85.0: an `arr.join(sep)` Array binding. Discovered directly via
  the new error hint — `texttv.nu` JSON's `content.join("\n\n")` hit
  a confusing overload-mismatch because Rhai 1.24's BasicArrayPackage
  doesn't ship Array.join in our configuration, and recon's existing
  `join(&mut ThreadHandle)` was the only registration users saw.

**Why the names are top-level rather than `text::*`-namespaced.**
The eleven strutil functions are recognisable PHP idioms. Scripts
ported from PHP read naturally when they look the same; `trim($s)`
→ `trim(s)`. The alternative — `text::trim(s)` or `strutil::trim(s)`
— would make every line a few characters longer for no gain, and
people would still type the bare names by muscle memory. Rhai's
existing `.trim()` String method keeps working alongside.

**Why `error_hint` rewrites instead of letting Rhai's default
through.** Rhai surfaces "Function not found: name (argTypes)" for
two distinct situations: (a) the name is genuinely unknown (typo),
(b) the name is fine but no overload accepts the runtime types. Case
(b) used to send users hunting for a missing import when the real
fix was `.body` or a `to_string`. The hint reads:

```text
error: Function not found: json_parse (map) (line 1, position 18)
note: `json_parse` is defined, but no overload accepts (map).
hint: check that you're passing the expected argument types ...
Available overloads:
  json_parse(_: string)
```

The case-(a) error stays untouched — when no sibling overloads
exist, the original one-liner is exactly the right message.

**The `metadata` feature side effect.** `error_hint` needs
`Engine::gen_fn_signatures()`, which lives behind the `metadata`
feature on the `rhai` crate. Enabling it also unlocked the
`:functions` REPL command in 0.83.0. But with `metadata` on, Rhai
parses `///` as a doc comment that must immediately precede a
function definition — and the shebang rewrite in
`src/script/engine.rs` was turning `#!/usr/bin/env -S recon
--script` into `///usr/bin/env -S recon --script`, which then broke
two pre-existing tests. Fix was a one-character change: `format!("//
{}", stripped)` instead of `format!("//{}", stripped)` — the space
keeps it a normal comment while preserving line numbers.

**Why discovering Array.join was a UX validation.** The
overload-hint feature from 0.84.1 immediately found a real gap in
the next release: a user's script used `content.join("\n\n")`,
Rhai's stdlib didn't expose it, and the hint pointed at exactly the
overload-mismatch rather than sending them hunting. Without the
hint they'd have seen "Function not found: join (array, string)" —
which reads like a missing import even though the actual issue was
a missing binding. With the hint they got "join is defined, but no
overload accepts (array, string); Available overloads: join(_: &mut
ThreadHandle)" — which immediately showed the gap. The fix landed
the same hour.

### 78. Interactive REPL (0.82.0)

**What:** `--export-pdf-page` switched from `agent-browser` (Chromium
screenshot of `file://*.pdf`) to a shell-out to `pdftoppm` from
poppler-utils. CLI surface and script binding unchanged.

**Why:** the 0.81.0 implementation produced a solid dark-gray image
on every PDF. Investigation in 0.81.2 showed Chromium ships without
the closed-source PDF viewer plugin (only Chrome-branded builds carry
it); the open-source build that agent-browser drives navigates to the
PDF URL successfully but renders an empty `<body>`, leaving the
viewport's default background as the only thing the screenshot
captures. The "is this even rendering?" verification step (sample
pixel variance) was missing from the 0.81.0 integration tests, so the
defect shipped silently.

The spec's planned HTML-wrapper fallback (`<embed src="…pdf">`) would
have hit the same wall — Chromium doesn't have the viewer to embed.

**Why pdftoppm vs. pdfium-render or mutool:** all three were on the
original brainstorm table; `pdftoppm` won the swap for three reasons:
shell-out matches recon's existing external-tool pattern
(agent-browser is shelled out, not linked); poppler is ubiquitous on
both Homebrew (macOS) and apt (Debian/Ubuntu); pdftoppm's
`-scale-to-x` / `-scale-to-y` flags map cleanly onto the existing
`--pdf-viewport` / `--pdf-scale` surface so the CLI didn't have to
change. The `--pdf-viewport` semantics did tighten — aspect ratio is
now preserved (previously stretched), with the viewport×scale
rectangle treated as an upper-bound box. Page geometry comes from a
companion `pdfinfo` call (same poppler package) so the binding can
compute a fit-within DPI without guessing.

The 0.81.0 `webp` crate dependency stayed; pdftoppm emits PNG / JPEG
directly, and WEBP is still transcoded in-process the same way.

**Lessons captured in tests:** the new `assert_png_has_content` helper
scans every 8th pixel and refuses a flat-color result. A 0.81.0-style
silent-rendering failure would now fail the integration suite.

### 76. PDF page → image export — `--export-pdf-page` (0.81.0)

**What:** New CLI flag `--export-pdf-page <PAGE> <PDF>` that renders a
single page of a PDF to PNG / JPEG / WEBP. Mirrored as
`pdf_export_page(pdf, page, [dest], [opts])` in the script engine.

**Why this approach (agent-browser backend):** recon already requires
`agent-browser` for HTML→PDF generation, so reusing it for PDF→image
adds no new external-tool surface for users. Considered and rejected:
- `pdfium-render` — bundling PDFium adds ~10–15 MB to the release
  binary, or pushes a separate runtime-library install onto users.
- shelling out to `pdftoppm` (poppler-utils) — zero new Rust deps but
  a second external CLI to install alongside agent-browser.
agent-browser drives Chrome, which renders PDFs via the same PDFium
engine pdfium-render would have linked — net visual fidelity is the
same.

**What's new internally:**
- `src/pdf_export.rs` — viewport + format parsing, `RenderOpts`,
  `render_page()` that drives `agent-browser set viewport → open →
  wait → screenshot → close` against a `file://…#page=N` URL with
  Chrome's PDF-viewer UI suppressed via `toolbar=0&navpanes=0&scrollbar=0`.
- `src/script/bindings/pdf.rs` — Rhai binding mirroring CLI shape.
- `webp = "0.3"` dependency for the PNG → WEBP transcode (Chrome's
  screenshot subcommand only emits PNG / JPEG).

**Deferred / out-of-scope:** multi-page export per call, HTTP/stdin
PDF source, SVG / vector output, page-count introspection.

### 75. `rquest` → `wreq` dependency migration (0.80.7)

Upstream `rquest` was renamed to `wreq` and every `rquest` crate
version on crates.io was yanked. recon-cli 0.80.6 was published with
the (now-yanked) `rquest =5.1.0` exact pin, which produced a noisy
warning on every `cargo install recon-cli --features impersonate`
invocation and risked breaking fresh installs without `--locked`.

**Why a plain rename, not an alternative crate.** Same author
(`0x676e67`), same GitHub repo (just moved from `rquest` to `wreq`),
same BoringSSL-backed API. `wreq 5.3.0` is essentially `rquest 5.2.0`
with the namespace renamed. The migration was line-by-line
mechanical: `rquest` → `wreq` and `rquest_util` → `wreq_util` across
`Cargo.toml`, `src/impersonate.rs`, `src/cli.rs` doc-comment,
`src/help.rs` topic, `src/examples.rs` notes, `docs/MANUAL.md`,
`OUT-OF-SCOPE.md`. Entry #72 of this file (the original 0.77.0
historical record) was left untouched — `rquest` was accurate then,
so the archaeology stays honest.

**Why not pin `=5.3.0`.** The previous `=5.1.0` exact pin was a
brittle choice (the yank trapped us at a yanked version with no
in-band upgrade path). Loosened to caret ranges (`5.3` for `wreq`,
`2.2` for `wreq-util`) so patch-level upstream fixes flow through
without another publish dance. If `wreq` ever ships a real breaking
change inside 5.x, the lock file pins specifically anyway.

**Why not the 6.0.0 release candidates.** As of the migration, `wreq`
had a `6.0.0-rc.28` series alongside the stable `5.3.0`. Stable was
the obvious pick — pre-release deps in a crates.io-published binary
are a recipe for the same churn we just escaped.

**Zero behavioural change.** All 5 impersonate integration tests
pass identically against `wreq 5.3.0`; `cargo build --release
--features impersonate` is warning-free; clippy clean.

### 74. GitHub Copilot CLI as `ai::*` backend (0.80.0)

Follow-up on the 0.79.0 `ai::*` foundation. A user with a Copilot
subscription asked for the same first-class treatment that claude,
codex, and gemini got. Adding a backend is a 80-line file under the
existing `AiBackend` trait — the architecture from entry #73 was
designed for exactly this.

**Why the standalone `copilot` CLI, not `gh copilot`.** Two products
share the "Copilot CLI" name. The newer standalone `copilot`
(GA October 2025) is an agentic CLI that behaves like Claude Code:
takes a prompt, returns a free-form response on stdout, supports
`--model` selection, plays nicely with shell redirection. The older
`gh copilot` extension was archived October 30, 2025 and was
fundamentally a different product — a shell-command suggester
(`gh copilot suggest -t shell …`) with TUI-only output and no
free-form chat mode. There is no realistic way to wrap `gh copilot`
inside the subprocess + free-form-chat shape `ai::send()` expects.
Skipping it cleanly was the right call; users who want it can still
wire it through the `cmd` backend in `~/.recon/config.toml` for
shell-suggestion-flavoured scripts, but they won't be doing chat.

**Invocation shape: stdin + `-s --no-color`.** `copilot` accepts the
prompt either via `-p "..."` or on stdin. recon uses stdin
universally (avoids argv length limits when contexts accumulate),
so this backend follows suit. `-s` (silent) strips the
session-metadata header that would otherwise leak into stdout; the
recon-side parser only wants the model's reply. `--no-color`
suppresses ANSI codes — important because some flatten/quote
downstream consumers don't strip them. System prompts are inlined
into the body (`SystemDelivery::Inline`) since `copilot` has no
`--system-prompt` flag. Model pass-through values include `auto`
(default), `gpt-5.3-codex`, `claude-sonnet-4.6`, `claude-haiku-4.5`.

**Auth.** Reuses Copilot's own `GH_TOKEN` / `GITHUB_TOKEN`
mechanism or its interactive `/login` flow — recon doesn't touch
credentials, matching the design rationale from #73 (no API-key
story in recon).

### 73. AI script-engine bindings — `ai::*` (0.79.0)

Diagnostic recon scripts had every primitive *except* "ask the model
a question and use its answer in the next step." The script engine
already exposes most of recon's domain (http, dns, tls, ftp, encode,
hash, jwt …) as Rhai bindings; an LLM that can answer a question
about a probe's output is a natural orchestrator on top of that.

**Why subprocess in v1, not an SDK.** The two natural shapes were
(a) shell out to an already-installed agent CLI the user has
authenticated (Claude Code, Codex, Gemini CLI) or (b) embed an SDK
(`async-openai`, anthropic-sdk-rust) and let recon talk to the API
directly. (a) reuses the user's existing auth, adds zero new crates,
keeps recon's binary size flat, and matches the project's "Unix tool
that orchestrates other Unix tools" character. (b) is more
controllable and opens streaming / tool-calling but introduces an
API-key story, a tokio runtime, and several megabytes of deps. For
a single-developer diagnostic tool the trade-off lands clearly on
(a). The builder API is shaped so (b) can drop in later behind the
same `AiBackend` trait without changing a single script.

**Why a builder, not a flat function.** The original sketch was
`ai::complete(prompt, opts)` with an options map. That works fine
for one-shot calls but doesn't compose well when scripts need to
accumulate context from multiple recon outputs (the cert, the
banner, the prior probe's result). The builder lets each producer
call `.context(x)` independently without anyone owning a "build
the options map" responsibility. Multi-turn replay falls out of the
same shape: `.assistant(x).user(y)` is just two more method calls
on the same object.

**Why `.send()` does not auto-append the assistant reply.** The
alternative (`.send()` records its own response into `turns` so the
next call sees full history) is more ergonomic but Hidden Magic™.
Explicit replay (`req.assistant(reply).user(next)`) keeps the
builder's state predictable. Auto-append can be added later as
`.send_and_remember()` if it turns out painful.

**Subprocess discipline.** Every backend spawns via
`std::process::Command` only — never `sh -c`, never argv built
from a script-supplied string. Prompts always pipe through stdin
(no shell-quoting, no argv length limit). The runner enforces the
`.timeout()` kill switch via a worker thread + `recv_timeout` on
an `mpsc` channel, so no async runtime is required.

**Custom backends via config.** The `cmd` backend reads
`[ai.backends.<name>]` from `~/.recon/config.toml`: argv is
user-supplied, with optional `model_flag` / `system_flag` names so
recon knows how to append `.model(...)` and `.system(...)` values.
This is the recompile-free escape hatch for users wiring up agent
CLIs that the built-in set doesn't cover.

**Library-surface decision: a thin `src/lib.rs`.** The integration
test in `tests/script_ai_it.rs` needs access to internal types
(`AiBackend`, `BackendCtx`, `Registry`, `Request`, `register_with_registry`).
recon-cli is primarily a binary crate; the existing convention in
`tests/` is to spawn the binary as a subprocess. For this new
script-engine surface, that pattern doesn't work — the test needs
to inject a `MockBackend` into the engine, which only the lib API
can do. Solution: a minimal `src/lib.rs` that exposes the `ai`
module tree via `#[path]`, plus `config`. The doc-comment in
`lib.rs` explains the constraint. As a side-effect, the four backend
files use `super::super::*` relative imports so the same source
files compile under both the binary's module tree and the lib's
`#[path]`-pointed tree — this is load-bearing for the lib build.

**Test budget.** ~40 unit tests across `request`, `flatten`,
`runner`, `resolve`, the four backends; 7 integration tests in
`tests/script_ai_it.rs` driven by a `MockBackend` so the suite
doesn't depend on any real CLI being installed.

### 72. TLS+H2 browser fingerprint impersonation — `--impersonate` (0.77.0)

A user building a captcha server asked for a way to make recon mimic real browser TLS / HTTP/2 fingerprints. recon's reqwest+rustls stack produces a stable, distinctive ClientHello easily detected by Cloudflare-class fingerprinters; even setting a Chrome `User-Agent` doesn't help when the server is reading JA3 / JA4 / Akamai-H2 settings.

**Why `rquest` over alternatives.** Hand-rolling rustls to mimic Chrome was ruled out — rustls deliberately doesn't expose extension ordering or H2 SETTINGS frame ordering, both of which servers actually fingerprint. `reqwest_impersonate` is unmaintained and lags Chrome versions by a year. `curl-impersonate` would mean shelling out and losing access to all of recon's own request shaping. `rquest` 5.1.0 is the actively-maintained reqwest fork built on BoringSSL; it provides the low-level emulation primitives. Its sister crate `rquest-util` 2.2.1 ships the named-profile catalogue (`Emulation::Chrome131`, `Emulation::Firefox128`, …, ~50 variants covering Chrome 100–136, Firefox 109–136, Safari 15.3–18.3.1, Edge 101–134, OkHttp 3.9–5, mobile profiles).

**Why opt-in via Cargo feature.** Activating impersonation pulls in BoringSSL (a C build, several minutes for a clean compile) and adds ~5–10 MB to the binary. The default `cargo install recon` workflow shouldn't pay that cost. `--features impersonate` is off by default; the release pipeline will publish a separate `recon-impersonate` artifact for users who need fingerprinting. The Cargo feature exposes the four CLI flags unconditionally (always present at parse time so `--help` and `--flags` are stable across builds), but passes them only through to the implementation when the feature is on; without the feature, the flags error at runtime with a "rebuild with --features impersonate" hint.

**Parallel-path architecture, not a unified abstraction.** `rquest` and `reqwest` have similar but non-identical APIs (different TLS backend, different builder shape, no shared trait). Building a trait abstraction over both would force every existing recon HTTP feature through the lowest-common-denominator surface and double the test matrix forever. Instead, dispatch happens at the top of `client::execute`: if any impersonation flag is set, the request is routed through a new `src/impersonate.rs` module that owns its own `rquest::Client`; otherwise the existing reqwest+rustls path runs unchanged. The impersonate path supports a deliberate **subset** of recon's feature surface, with a documented incompat list (`--ciphers`, `--tlsv1.2`, `--tlsv1.3`, `--client-cert`, `--client-key`, `--cacert`).

**rquest 5.1.0 is async-only — no `rquest::blocking`.** The original plan assumed a `blocking` feature analogous to reqwest's, but rquest 5 dropped blocking support. recon's pipeline above `client::execute` is blocking, so `impersonate::execute` builds a current-thread `tokio::Runtime` per call and `block_on`s the rquest future. Tokio with `rt-multi-thread` was already a direct dep, so this added zero new crates. The runtime is current-thread (not multi-thread) since recon doesn't spawn parallel requests on the impersonation path.

**Response type bridge.** rquest returns its own `Response` type. recon's output / metrics pipeline downstream is keyed on `reqwest::blocking::Response`. The bridge: in `convert_response`, buffer the rquest response body in memory, build an `http::Response<Vec<u8>>` from the status / headers / version, then `reqwest::blocking::Response::from(http_response)`. Body buffering is acceptable for v1 because the captcha-testing use case sees small bodies. URL preservation in the converted response is a known limitation of `reqwest::blocking::Response::from(http::Response)` — there's no public setter for url on the blocking type. The synthesised response carries reqwest's placeholder URL (`http://no.url.provided.local/`); `impersonate::execute` overrides `metrics.url_effective` from `args.target_url()` after `snapshot_response_for_impersonate` runs so `--write-out %{url_effective}` and Rhai `r.url` see the real value. Redirect handling on the impersonate path is also out of scope for v1, so this single override is sufficient.

**v1 scope reduction.** The original spec promised three additional flags — `--ja3 <string>`, `--ja4 <string>`, `--http2-fingerprint <string>` — for raw fingerprint overrides. While drafting Task 4 it became clear that rquest 5.1.0 is a lower-level toolkit (`TlsConfig` builders for cipher list / sigalgs / curves / extension order) rather than a turnkey "set this JA3 string" library. JA3 strings don't capture sigalgs or extension order, so reconstructing a `TlsConfig` from a JA3 would be lossy and partial. JA4's cipher and extension components are SHA-256 truncations and fundamentally non-invertible. Each parser would be 100–200 lines of brittle TLS plumbing that produces partial fingerprints. Decision: ship `--impersonate` end-to-end in v1 (the captcha-server use case is "test against realistic browser traffic" — named profiles cover it) and defer raw overrides to v0.78 once a real captured-fingerprint case lands. The flags stay in the CLI for forward-compatibility (so `--help` and `--flags` are stable across versions); they error at runtime with a clear "deferred to v0.78" pointer.

**Side-effect on the default build: `ssh2` switched to vendored OpenSSL.** Adding `--features impersonate` causes BoringSSL (via `boring2-sys`) to be linked. BoringSSL exports `libssl` / `libcrypto` symbols but omits OpenSSL 3-only symbols like `_OSSL_PARAM_construct_utf8_string` that `libssh2-sys` needs. Without intervention, `cargo build --features impersonate` fails at the link step. Fix: add `features = ["vendored-openssl"]` to the existing `ssh2 = "0.9"` dep, which makes `libssh2-sys` statically bundle its own OpenSSL build independent of BoringSSL. This change applies to the default build too — it's not feature-gated — but the cost is ~5–10 seconds of additional build time and no runtime difference. The alternative (gating ssh2 behind a Cargo feature, or splitting the binary further) was rejected as disproportionate to the cost.

**Profile-name normalisation.** rquest_util's `Emulation` enum serializes with underscores and dots: `chrome_131`, `safari_17.5`, `okhttp_5`. The natural CLI ergonomic is hyphens (`chrome-131`). The `parse_emulation` helper accepts either: it lowercases and replaces `-` with `_` before deserialising via `serde_json::from_value(Value::String(...))`. Users can type whichever feels natural; the help text documents the upstream form.

**`friendly_message` filter compatibility.** recon's `main.rs` runs every error through a filter that rewrites any error message containing "tls" or "certificate" to the generic "TLS/certificate error" message — useful for hiding rustls handshake noise from end users, but it silently swallows messages from the new module unless they're allowlisted. The dispatch-stub messages (the "rebuild with --features impersonate" hint and the "not yet implemented" stub) were rephrased to drop "TLS" / "certificate" entirely; future contributors who add user-facing strings here should keep that discipline. The `validate_combination` errors are a harder case — they have to name the offending flags (`--tlsv1.2`, `--tlsv1.3`, `--cacert`), and `--tlsv1.X` literally contains the substring "tls". Rephrasing wasn't an option, so the fix was an explicit early-return in `friendly_message` keyed on the substring `"browser fingerprint impersonation"` (which appears in every validate_combination message). Adding the early-return up-front instead of trying to scrub the strings means future incompat-list additions only need to include the agreed phrase to survive the filter. Regression test in `tests/impersonate_it.rs::validate_combination_errors_survive_friendly_message_filter` locks the behaviour in.

**Test budget: +4.** Three integration tests in `tests/impersonate_it.rs` (chrome named profile against httpbin, hyphenated profile name accepted, invalid profile name produces helpful error) plus one parameterised test asserting the three deferred flags error out with the v0.78 pointer. Network-dependent tests skip cleanly when `httpbin.org` is unreachable. Existing test suite still passes against both feature builds (debug + release × default + impersonate).

### 71. Script-context constants — `script_path` / `script_dir` / `script_name` (0.76.1)

Follow-up on the 0.76.0 dotenv release. The shipped `load_dotenv(path)` API took the path verbatim, so the user's stated workflow — "multiple scripts in a directory sharing a `.env` plus per-script `.env.<scriptname>` overlays" — only worked if the user either hardcoded absolute paths or `cd`'d into the script directory before running recon. Neither is the workflow they described.

**Three `Scope::push_constant` calls.** `src/script/engine.rs:48` already had the resolved absolute path in hand for Rhai's module resolver (`ast.set_source(...)`); just wasn't exposing it to the script. Pushed three constants alongside `args` and `flags`: `script_path` (resolved absolute), `script_dir` (its parent), and `script_name` (file stem, basename minus extension). Six lines of plumbing in `engine::run_file`. The natural overlay idiom now writes:

```rhai
load_dotenv(script_dir + "/.env");
load_dotenv(script_dir + "/.env." + script_name);
```

**`script_name` is not redundant with `args[0]`.** The first cut of the demo used `args[0]` as the overlay name (`script_dir + "/.env." + args[0]`). End-to-end smoke produced `.env./tmp/dotenv-overlay/demo.rhai` because `args[0]` is whatever the user typed on the command line — when invoked with `recon --script /abs/path/demo.rhai`, `args[0]` is the full path, not `demo`. `script_name` always reduces to just the stem regardless of how the script was invoked. Caught the bug in the smoke test before shipping; added `script_name` and a `script_name_is_file_stem` unit test to lock the behaviour in.

**`load_dotenv` itself unchanged.** The temptation was to make `load_dotenv` auto-resolve relative paths against `script_dir`. Rejected as surprising magic: relative-path semantics would differ between `load_dotenv(".env")` and any other recon function that takes a path. Users compose `script_dir + "/.env"` explicitly. Easier to reason about, no environment-variable plumbing, and absolute paths still work the way they always have.

**Demo rewrite.** The 0.76.0 demo wrote tempfiles under `/tmp/recon-dotenv-demo.env` and loaded them by absolute path — exercised the API but not the workflow. Rewrote to expect sibling `.env` and `.env.<script_name>` files and degrade gracefully via `try { ... } catch { ... }` when they don't exist. Users who copy `script/dotenv.rhai` into a directory of their own scripts get the layered behaviour for free.

**Test budget: +4.** Three tests for the new constants (`script_path_constant_is_resolved_path`, `script_dir_constant_is_parent_of_script_path`, `script_name_is_file_stem`) plus one end-to-end load_dotenv-via-script_dir test. 1266 → 1270 passing.

### 70. Script engine — `.env` loading + `env_all()` (0.76.0)

A user reported that scripts couldn't easily layer config across a directory of related Rhai files. The pattern they wanted: one common `.env` shared by every script in a directory, plus a per-script `.env.<scriptname>` whose values override the common file. Process-environment *reads* were already exposed via `env(name)` / `env(name, default)` (helpers.rs:18-26), but `.env` parsing didn't exist anywhere in recon.

**`dotenvy` over `dotenv`.** The `dotenv` crate is unmaintained (last release 2020) and carries an open soundness advisory. `dotenvy` is its actively-maintained fork, same API surface, ~12 KB compiled with no transitive deps. Direct replacement, no comparison shopping needed.

**`from_path_iter`, not `from_path` / `from_path_override`.** The naive pick was to dispatch on the override flag and call one of dotenvy's two convenience functions. But those functions silently set env vars themselves and return only `Result<(), Error>` — recon couldn't count what was actually set or implement the override branch consistently. `from_path_iter` returns an iterator over `Result<(String, String)>`, leaving recon in control of both the override decision and the count. The implementation became a six-line for-loop in `helpers.rs::load_dotenv_impl`.

**Override semantics: default ON.** dotenvy's `from_path` defaults to non-override, but the user's stated workflow ("common.env, then .env.<scriptname>") only works if the second load wins. Non-override default would silently make the per-script file a no-op for any key the common file already set — a footgun that would surface as "why isn't my override working?" support questions. So `load_dotenv(path)` overrides existing values; `load_dotenv(path, false)` is the explicit opt-out for callers who want shell-env to take priority over file values.

**camelCase aliases.** The user's request used `loadDotEnv` (camelCase), but every other recon binding is snake_case (`file_read`, `json_parse`, `sleep_ms`). Registered both: canonical name `load_dotenv` matches the codebase, alias `loadDotEnv` accepts the user's preferred spelling. Same for `env_all` / `envAll`. Cost: one extra `register_fn` per alias; benefit: zero pushback on naming.

**Concurrency footnote (deliberately not enforced in code).** `std::env::set_var` is technically unsound under concurrent reads on some platforms, and recon ships threading bindings (`thread_spawn`). Adding a runtime guard ("error if any thread is alive") would be defensive overkill for a function that's almost always called once at script startup. Documented the constraint in the help text, the manual, and the inline comment, but left enforcement to the user — Rhai scripts are sequential by default, and the typical call site is the first non-comment line.

**Test budget: +8.** Seven tests cover the new bindings (env_all snapshot + camelCase alias, load_dotenv basic set, override-default-true, override-explicit-false, layered common→specific, missing-file error path, camelCase alias) plus `script/dotenv.rhai` which `tests/script_examples_it.rs` validates as parseable. 1258 → 1266 passing.

### 69. agent-browser global options in script engine (0.75.0)

A user reported that a Rhai script using `agentBrowser::open(url)` against a server with a self-signed TLS certificate hit a Chrome cert-rejection wall with no escape hatch. Investigation: `src/script/bindings/agent_browser.rs` exposed ~30 action verbs (open, click, type, etc.) but ZERO of agent-browser's 25 global launch / security / session options. The shared transport `crate::agent_browser::run_cmd(args, json)` had no slot for prepending global flags.

**Module-level defaults + per-call overrides.** Brainstorming weighed three patterns: (A) module-level defaults set once via `set_default_options(opts)`, (B) per-call opts on every verb, (C) hybrid — module defaults plus per-call overrides on launch verbs only. C won: a one-line setup at script start fixes the immediate workflow, and per-call opts on launch verbs (`open`, `screenshot`, `snapshot`, `pdf`, `eval`) cover the cases where a single script switches contexts mid-run.

**Translation layer.** A central `opts_to_argv(map) -> Vec<String>` translates a Rhai opts-map into agent-browser argv. Bool flags emit only when true; strings and ints emit the typed value; repeatable flags (`extension`, `browser_args`) accept either a single string or an array; `headers` accepts either a JSON string or a Rhai map auto-serialized to JSON. Unknown opts-map keys error with a sorted listing of all valid keys for typo-correction. Type mismatches error with the expected type.

**`browser_args` rename.** agent-browser's `--args <browser-args>` accepts arbitrary launch flags. Naming it `args` in Rhai would be too generic and clash with common script patterns. Renamed to `browser_args` in the opts map; the binding translates back to `--args` when emitting argv.

**Process-wide defaults.** Defaults live in a `OnceLock<Mutex<Vec<String>>>` inside the bindings module, shared across Rhai engines. Justified because agent-browser sessions are OS-level (one Chrome process per session name) — process-wide defaults match the real session model.

**Last-wins concatenation.** Per-call opts are appended after defaults. agent-browser's flag parser uses last-wins for repeated single-value flags, so a per-call `user_agent` overrides a default `user_agent` automatically.

**Test budget: +16.** One `run_cmd_with_options` smoke test in `src/agent_browser.rs`. Eleven `opts_to_argv` translation tests (bool true / false, string, int, repeatable single / array, headers string / map, unknown key, type mismatch) plus three round-trip tests for the defaults state plus one merge-order test in `src/script/bindings/agent_browser.rs`. 1242 → 1258 passing. Demo script `script/agent-browser-options.rhai` parses cleanly via the existing `script_examples_it.rs` test.

### 68. PDF metadata flags — author / subject / keywords (0.74.0)

Final release of the 0.71.0–0.74.0 four-release plumb-through sweep, addressing items from `OUT-OF-SCOPE.md`. Three new CLI flags populate PDF document metadata when generating PDFs via `--md-to-pdf` or `--html-to-pdf`.

**Implementation: binary Info-dict patch, not agent-browser argv or HTML meta tags alone.**
agent-browser's `pdf` subcommand has no metadata flags (verified: `agent-browser pdf --help` shows only `<path>`). Chrome's headless `printToPDF` (Chrome 147) does not translate `<meta name="author">` etc. into PDF Info dict fields. The chosen approach is a post-generation binary patch of the PDF file: after agent-browser writes the PDF, `patch_pdf_info` reads the bytes, finds the `1 0 obj` Info dict, inserts `/Author`, `/Subject`, `/Keywords` entries before the closing `>>`, then updates every xref-table entry whose byte offset was after the insertion point and rewrites `startxref`. Chrome PDFs use traditional cross-reference tables (not cross-reference streams), making the patch straightforward. HTML `<meta>` tags are also injected in the generated HTML `<head>` as a belt-and-suspenders measure. Verified clean with `pdfinfo` — no xref warnings, all four metadata fields populated.

**No new crate.** The entire implementation is in `src/docs_pdf.rs` (~120 LOC), using only `std::fs` and `anyhow`. `lopdf` was considered but rejected: it adds a ~200 KB compiled dependency for a one-time patch of a well-understood 20-byte-entry format. The patch is best-effort (failure prints a warning and leaves the PDF intact).

**Manual dogfood.** `docs/MANUAL.pdf` is now regenerated with `--doc-title`, `--doc-author`, `--doc-subject`, `--doc-keywords` all set, providing a working example baked into the build process documented in `CLAUDE.md`.

**Sweep complete.** Four releases over 2026-05-01 closed 17 items from `OUT-OF-SCOPE.md`. Remaining items in the four targeted headers are now sharper — every deferred item has a precise upstream-blocker rationale (e.g. "suppaftp 6 has no `account()` method", "lettre 0.11 builds MailParameter internally", "rustls 0.23 has no public cipher-list API"). Future scope:
- `--pinnedpubkey` + `--curves` blocked on the use_preconfigured_tls migration (its own focused effort, ~80–120 LOC).
- `--proxy-pass` blocked on reqwest 0.12 not exposing passphrase-accepting Identity variants.
- Other-markup → PDF (reST, AsciiDoc, Org) blocked on pure-Rust parser ecosystem maturity.

**Test budget: +0.** Metadata verification is via `pdfinfo` on the generated artifact; not a code-test surface. 1242 passing maintained throughout the sweep.

### 67. curl-parity misc — remote-name-all, -#, proxy-pass (0.73.0)

Three small curl-compatibility flags shipped in a single focused pass. No new architecture or new crates.

**`--remote-name-all` shipped clean.** The `--input-file` loop clones `args` per URL; inserting `if per.remote_name_all { per.remote_name = true; }` before the `execute_with_retry` call was the entire implementation. The flag is opt-in and composes with `--output-dir` and `-#`.

**`-#, --progress-bar` shipped clean.** `make_progress_bar` in `output.rs` grew a `hash_style: bool` parameter; the two call sites (`output.rs` file-save path and `scp.rs`) were updated together. The hash bar uses indicatif's `progress_chars("##-")` with a `[{bar:40.#->}]` template to render `#` fill and `-` empty space. Progress is also activated when `-#` is set without `--progress`.

**`--proxy-pass` deferred with runtime warning.** reqwest 0.12's `Identity::from_pem` has no passphrase-accepting variant — the rustls path parses unencrypted PKCS#8/RSA/EC keys only; the native-tls path exposes `from_pkcs12_der(der, password)` for PKCS#12 archives (not PEM + separate key). No `--proxy-cert` / `--proxy-key` flags exist in recon yet either. The flag is declared, triggers a clear `eprintln!` warning at runtime, and is documented as Deferred in OUT-OF-SCOPE.md. When proxy mTLS certificates land, the passphrase plumb-through will follow.

**Test budget: +0.** 1242 passing maintained. The three flags are integration surfaces; unit-testing the indicatif template string and the `remote_name` force-set is not meaningful without a full HTTP server fixture.

### 66. TLS hardening — CRL + proxy-CA plumb-through (0.72.0)

After 0.66.0 shipped a wave of TLS / proxy CLI stubs, the original 0.72.0 plan was to ship five TLS-tuning flags: `--crlfile`, `--pinnedpubkey`, `--curves`, `--proxy-capath`, `--proxy-ca-native`. A Phase-1 rustls-API audit during this release narrowed the ship list to three.

**`--crlfile` shipped clean.** reqwest 0.12.28 has first-class CRL support: `ClientBuilder::add_crls(impl IntoIterator<Item = CertificateRevocationList>)` calls `WebPkiServerVerifier::builder_with_provider(...).with_crls(...).build()` internally. `reqwest::tls::CertificateRevocationList::from_pem_bundle(&[u8])` parses multi-CRL PEM blobs. Total integration: ~10 lines mirroring the existing `--cacert` block.

**`--proxy-capath` and `--proxy-ca-native` shipped clean.** reqwest doesn't expose per-proxy TLS roots — the global `ClientBuilder` TLS config covers both server and proxy connections. These flags exist for curl-parity and augment the same global config. Implementation mirrors the existing `--capath` directory walker.

**`--pinnedpubkey` and `--curves` punted.** Both require `use_preconfigured_tls`, which type-erases the entire rustls config and bypasses ALL of reqwest's high-level TLS setters. Switching the HTTP path to it means migrating 8 existing flags (`-k`, `--cacert`, `--capath`, `--ca-native`, `--tlsv1.2`, `--tls-max`, `--cert`/`--key`) onto a custom `build_rustls_client_config(args) -> rustls::ClientConfig` helper. That migration is tractable but is its own focused effort, not a single-flag plumb-through. Marked in OOS as "ship together when the migration happens" with a sharper note.

**Audit-found nuance: `SECP521R1` not in ring.** When `--curves` does ship, P-521 will need a graceful per-curve error under the ring backend. ring stops at P-384; aws-lc-rs adds P-521. Recon currently builds with ring (Cargo.toml feature flag).

**Test budget: +0.** No new tests this release — all three flags are integration surfaces best validated by smoke testing against real CRLs and proxies. 1242 passing maintained.

### 65. 0.65.0 stubs become real — per-protocol plumb-through (0.71.0)

After 0.65.0 shipped a wave of curl-parity FTP / SMTP / IMAP / POP3 / Telnet flags as CLI stubs (clap accepted them but they had no effect on the underlying protocol probes), this release does the actual plumb-through — for the items where the underlying crate (suppaftp 6, lettre 0.11, imap 3-alpha, ssh2 0.9) actually exposes the necessary primitive.

**Phase-1 audit drove the ship list.** Three Explore agents combed `~/.cargo/registry/src/.../{suppaftp,lettre,imap}-*/src/` looking for each method curl assumes exists: `custom_command`, `nlst`, `set_passive_nat_workaround`, `MailParameter::Other`, AUTH-mechanism options. The audit produced a per-flag ship/defer/adapt verdict before any code was touched.

**Surprising wins from the audit.** `--ftp-skip-pasv-ip` was originally pegged as upstream-blocked but `suppaftp::FtpStream::set_passive_nat_workaround(bool)` exists and matches curl's semantics exactly (replace server-advertised PASV IP with control-channel peer IP). Shipped as a one-liner.

**Surprising losses.** `--mail-auth` was expected to be a moderate-effort lettre integration but the high-level `SmtpTransport::send(message)` builds the `MailParameter` vec internally with no external injection point. Forking the send path was rejected as out of scope; the flag now emits a runtime warning and was moved to `OUT-OF-SCOPE.md`. `--sasl-ir` turned out to be unconditionally on for PLAIN/XOAUTH2 and unconditionally off for LOGIN — there is no toggle to expose. `--login-options` and `--sasl-authzid` have no parameter-passing surface at all in the imap 3-alpha crate.

**No pop3 crate.** Recon's POP3 probe was implemented without the `pop3` crate dep, which means the `pop3`-side flags listed in OUT-OF-SCOPE.md were never going to land via crate-API plumbing. They're now categorised as needing a from-scratch SASL implementation, not a plumb-through.

**Telnet `--telnet-option` deferred.** `src/telnet.rs` is currently a probe that opens TCP and reads the banner — no IAC option negotiation. Wiring `--telnet-option` would require building telnet option negotiation infrastructure, which is genuine feature work, not stub-plumbing.

**Test budget: +3.** Three new unit tests for the SSH `--pubkey` alias resolver. FTP / TFTP / SMTP changes verified via live smoke tests against `test.rebex.net` (FTP) and code reading (no test SMTP server in CI). 1239 → 1242 passing.

### 64. stdin/clipboard polish — native clipboard + auto-detect (0.70.0)

Polish release after 0.69.0's `--stdin` debut. Three changes that all touch the same `--stdin`-pipeline code paths in `src/main.rs` and `src/output.rs`, so they ship together rather than as three separate releases.

**`arboard` for clipboard, not shelling out.** Considered shelling to `pbpaste`/`pbcopy` (macOS), `xclip`/`wl-copy` (Linux), `Get-Clipboard`/`clip` (Windows). Three-way platform branching for what's a 30-LOC primitive. `arboard` 3.x covers all three platforms (Cocoa AppKit, X11 + Wayland, Windows OLE) with a uniform API and adds about 200 KB compressed binary size — a fair trade for one place to maintain. Opted out of the `image-data` default feature (no image-clipboard support needed; saves the `image` crate dep). Linux Wayland coverage via the `wayland-data-control` feature.

**`--clipboard <DIR>` with context-aware default.** Three flags pile up in the help output (`--from-clipboard`, `--to-clipboard`, `--clipboard`), but the user-facing ergonomics win — `recon --clipboard --prettify-as json` is the killer line. Bare `--clipboard` resolves to `out` when there's already an input (URL, `--stdin`, etc.) and `in` otherwise; this was the user's design call. Kept the resolution logic in `main.rs` post-parse so the rest of the codebase only ever sees `from_clipboard` / `to_clipboard` bools.

**`BodySink` enum replacing the `(final_path, sink_writer)` pair.** The 0.69.0 helper already handled file vs sink branching internally. Adding editor and clipboard sinks via more parameters would have ballooned the signature. A small `BodySink<'a>` enum (`Writer` / `File` / `Editor` / `Clipboard`) is one parameter, four variants, internal dispatch — net cleaner. `Editor` and `Clipboard` accumulate into a `Vec<u8>` because both need the full output before they act. Removing the duplicate `run_with_editor` in main.rs (~46 lines) was a side benefit. The `-vv` mirror-to-stdout behaviour for `--editor` (documented in `recon --help editor`) is preserved inside `BodySink::Editor` — was almost dropped during the refactor; caught in code review.

**Auto-detect stdin via `std::io::IsTerminal`.** Stable since 1.70 — no new deps. URL was previously required via clap's `required_unless_present_any`; dropping that and validating in `main.rs` instead lets recon either auto-promote piped stdin or emit a clean usage error for TTY-no-args. The mode-flag exclusion list (which used to live in clap) now lives in a `any_no_url_mode_flag(args)` helper — single source of truth.

**Test budget: +8.** New `tests/clipboard_it.rs` (7 tests, 3 gated on `RECON_CLIPBOARD_TESTS` env var). Auto-detect test in `stdin_prettify_it.rs`. 1231 → 1239 passing.

### 63. Offline payload prettify — `--stdin` + `--prettify-as` (0.69.0)

A small but high-leverage feature: prettify a JSON / XML / YAML / CSV / TSV / HTML payload that's already on the local machine — clipboard, file, pipe — without making an HTTP request. Drives the `pbpaste | recon --stdin --prettify-as json` workflow.

**Two flags rather than one with optional value.** The natural-feeling syntax (`recon --prettify json`) was rejected after weighing clap's optional-value parsing against the cost of ambiguity with positional URLs (`recon -p https://example.com` would risk eating the URL as the prettify value). A separate `--prettify-as <FORMAT>` flag is fully backwards-compatible — `-p / --prettify` keeps its existing boolean semantics — and zero-ambiguity by construction. The implicit-prettify shim (`--prettify-as` sets `args.prettify = true` if not already) means users only need one flag in practice.

**Helper extraction in `src/output.rs`.** The existing prettify + output-charset path lived inline inside `write_response_to`. Pulled it into `pub fn write_processed_body(args, raw, content_type, output_charset_label, final_path, sink_writer)` so both the HTTP response path and the `--stdin` mode share one implementation. The helper takes the body as `&[u8]` and a content-type string separately so the caller can synthesise an empty content-type (forcing body sniffing) when running over stdin. Signature uses `&mut dyn Write` rather than a generic `W: Write` because `sink.writer()` returns a borrowed `Box<dyn Write + '_>` that doesn't satisfy a `'static` generic bound.

**Strict vs lenient prettify.** When `--prettify-as` is set, parse errors propagate (`prettify::run(...)?`) so the user sees them — they explicitly picked a format. When format is auto-detected, the legacy lenient `unwrap_or(body_str)` fallback is kept so a server returning `text/plain` for what looks like JSON but isn't doesn't bork an otherwise-fine request.

**`--stdin` joins the early-mode dispatch family.** Lives alongside `--iconv`, `--init`, `--encode`, `--hash`, `--editor-cleanup`, `--sample-list` in `src/main.rs` — all single-purpose flag-triggered modes that exit before HTTP setup. Required adding `"stdin"` to clap's `required_unless_present_any` list so the URL positional becomes optional, mirroring every other no-URL mode.

**Test budget: +9.** New `tests/stdin_prettify_it.rs` covers auto-detect, forced format, implicit-prettify, strict mode, unknown format → exit 2, mutex with URL → exit 2, empty stdin, XML, raw passthrough. 1221 → 1230 passing.

### 62. Waiting-arc kickoff — recon-own items (0.61.0)

Release 1 of the 6-release arc that implements everything in OUT-OF-SCOPE.md's "Waiting" bucket. User's priority was "recon's own flags first", so this release clears every pre-existing recon wishlist item before the arc moves on to the curl / wget catalogues.

**Check digits — Latin-American tax IDs.** New `src/checkdigit/tax_id.rs` module with 8 algorithms: Brazilian CPF + CNPJ (two mod-11 check digits on weighted sums; all-identical-digit inputs rejected by convention), Argentinian CUIT / CUIL (same algorithm, different labels), Chilean RUT (with 'K' check-char, weights cycle 2-7 from the right), Peruvian RUC (mod-11 with 10/11 fallback), Australian ABN (ISO/IEC 7064 MOD 89; create() refuses because no single-digit inverse exists), Mexican RFC (13-char person / 12-char company, alphabet map with Ñ=24). Each ~30 LOC. CNPJ needed a custom '/' stripper since the general `sanitize()` doesn't remove it.

**Check digits — 110+ year warnings** extended from Swedish personnummer to Danish CPR (`mod11.rs`), Finnish henkilötunnus (`mod31.rs`), Norwegian fødselsnummer (`mod11.rs`), and Bulgarian EGN (`vat/bg.rs`). The Norwegian case needed fresh century-decoding logic — FNR encodes the century in the NNN digits rather than via a separator char, with four branches (000-499 → 1900s, 500-749 + YY 54-99 → 1800s, 500-999 + YY 00-39 → 2000s, 900-999 + YY 40-99 → 1900s). `current_year()` in `country_id.rs` promoted to `pub(crate)` so sibling modules can reach it.

**`--decode-all`.** Uses rxing's `helpers::detect_multiple_in_file` — already in the rxing dep (0.55.0). Thin wrapper: file path for filesystem inputs, tempfile for stdin/Blob script callers. Output is tab-separated `<FORMAT>\t<TEXT>`, one line per detection. Exits non-zero when the image contains no detectable codes.

**HRT under 1D barcodes.** Scope decision during implementation: full ab_glyph-based PNG text rendering would need a bundled TTF font (~50-100 KB compiled) and pixel-level positioning work. Shipped SVG + ASCII HRT in 0.61.0 (both trivial — ASCII centers the text in an extra line below, SVG emits a `<text>` element with `font-family="monospace, sans-serif"`). PNG HRT is explicitly deferred in OUT-OF-SCOPE.md. Default-on for EAN-13 / UPC-A, off for Code128 / Code39 (where the text is often arbitrary enough to be noisier than useful). `--hrt` / `--no-hrt` flags override.

**MQTT mTLS.** rumqttc 0.24 takes a rustls 0.22 `ClientConfig`. recon's `--client-cert` is typed against reqwest, so a new `build_client_auth_material()` in `src/mqtt.rs` parses the PEM bundle directly into rumqttc's rustls-0.22 `CertificateDer` + `PrivateKeyDer` types (using the existing `pem` crate). `build_rustls_config()` gained a `client_auth: Option<(chain, key)>` parameter. Supports PEM / PKCS#8 / PKCS#1 / SEC1 keys; encrypted keys and DER formats error with the same `openssl` recipes as the HTTPS path (0.54.0).

**`--interface` name resolution.** New `src/iface.rs` module. `resolve_interface(spec)` tries an IP-literal parse first; on failure falls back to `libc::getifaddrs()` + `CStr::from_ptr()` to walk the interface list. Prefers non-loopback addresses; falls back to loopback when no routable address exists (`lo` / `lo0`). Windows gets a clear error pointing at the IP-literal form. ~120 LOC including three `unsafe` blocks, all scoped to FFI boundaries.

**Test budget: +15.** Check-digit module gained 12 new tests (8 tax IDs + round-trip helpers + edge cases). iface module gained 3 (IP literal, lo/lo0 resolution, unknown-name error). 1155 → 1170 passing.

### 61. Cover pages + chapter page breaks in PDF output (0.59.0)

Follow-up to 0.58.0's doc conversions. User asked whether recon's md→PDF flow could produce a proper book-style cover page plus page breaks between major chapters. Both doable via existing Chrome printToPDF primitives; wired through two new flags + a TOC placement marker.

**`--unsafe-html` flips comrak's `unsafe_`.** By default comrak escapes raw HTML blocks in markdown (safe for rendering untrusted input). Book-style covers need styled `<div class="cover">`, custom page-break markers, and similar — all raw HTML. Flag defaults to off; users opt in when they own the markdown input.

**`--page-break-on-h1` is a CSS injection.** `main > h1:not(:first-of-type) { break-before: page; page-break-before: always; }`. The `:not(:first-of-type)` skips the opening H1 so the document doesn't start with a pointless blank page. Modern browsers + printToPDF honour both the newer `break-before` and the legacy `page-break-before`.

**Cover page is pure CSS + raw HTML.** No dedicated `--cover <PATH>` flag needed — authors write `<div class="cover">…</div>` in the markdown (with `--unsafe-html`) and the bundled stylesheet handles layout: `min-height: 90vh` + centered flex, `.subtitle` / `.version` / `.date` / `.author` / `.meta` child classes, `<hr>` as a narrow divider, automatic `break-after: page` so the cover sits on its own page. Inside the cover, `<h1>` in raw HTML (not markdown `#`) stops the TOC generator from picking it up — discovered during smoke test when the cover's "recon" heading bled into the TOC.

**`<!-- toc -->` marker lets authors position the TOC.** Previously TOC was injected at the top of every document unconditionally. Now `markdown_to_html` scans the post-comrak body for `<!-- toc -->`; if found, the auto-TOC replaces the marker in-place and the top-of-body injection is suppressed. Lets users put the TOC after a cover page (common book-style convention). The marker survives only with `--unsafe-html` on since comrak strips HTML comments otherwise — which is the typical pairing anyway.

**Manual migration.** `docs/MANUAL.md` swapped its hand-written numbered TOC for the `<!-- toc -->` marker, added a cover block, and now renders to a 71-page PDF (up from 67 pre-cover-breaks). `CLAUDE.md`'s recommended regenerate command gained `--unsafe-html --page-break-on-h1`.

**Tests added: +4.** raw-HTML passthrough on / off, page-break-on-h1 CSS emission, suppression when flag off. 1142 → 1146 passing.

### 60. Document conversions — markdown / HTML / PDF with linkable TOC (0.58.0)

Three conversions in one release: `--md-to-html` (pure-Rust), `--html-to-pdf` (via agent-browser), `--md-to-pdf` (pipelined md-to-html + html-to-pdf). Linkable tables of contents on all of them.

**Crate choice — `comrak` over `pulldown-cmark`.** Both are actively maintained pure-Rust CommonMark parsers. `comrak` wins because it emits `id="slug"` on headings natively (via `ExtensionOptions::header_ids`). pulldown-cmark would need a separate slugification + post-processing pass. Size difference is negligible (~600 KB vs ~150 KB), and TOC anchor IDs are the load-bearing feature for this release.

**PDF backend — agent-browser, not chromiumoxide.** Considered both. `chromiumoxide` gives us in-process Chrome DevTools Protocol control; agent-browser is an external CLI that recon already integrates with (screenshot / snapshot / open / close / pdf). Picked agent-browser because:
1. Zero new code paths — `agent_browser::run_cmd(&["pdf", path], false)` is already the call pattern for other browser features.
2. Zero new Cargo deps on top of `comrak`.
3. One fewer Chrome-finder discovery path to maintain (agent-browser handles its own Chromium resolution).

**HTML shell.** Generated document wraps comrak's body fragment in a full `<!doctype html>` with a `<title>`, inlined bundled CSS (@page margins, serif body, monospace code, table borders, page-break-inside:avoid on `<pre>`), an optional injected `<nav class="toc">`, and the body inside `<main>`. Chrome's printToPDF honors `@page` for margins + `avoid-inside` for hint preservation, so the PDF looks book-like rather than "browser-print".

**TOC generation — walked AST, not regex.** Single pass over `root.descendants()`, filtering `NodeValue::Heading(h)` where `h.level <= toc_depth`, extracting text via a small recursive walker, slugifying to match comrak's own header_ids rule. Then render a nested `<ul>` tree with `<a href="#slug">` entries. Chrome preserves those anchors as clickable internal PDF links.

**Source-loader reuse.** `docs::load_source()` clones the CLI args, clears the three doc-conversion flags, sets `url = Some(src)`, and calls `source::read_all()`. Same pipeline as `--compare` (0.53.0) — URL sources get every HTTP flag (-H, -u, -L, -k, cookies, proxy, HSTS) for free, stdin via `-` is handled, file:// URLs work.

**`agent-browser open file:// → pdf → close`.** Tempfile lifecycle: write → canonicalize → open → pdf → close → NamedTempFile drops at scope end. Close is attempted on every error path (via `let _ = run_cmd(&["close"], …)` after the main flow) so agent-browser doesn't leak a browser session.

**Default CSS.** ~50 lines inlined in `src/docs.rs`. Opinionated but tasteful: 11pt body, 24pt H1, border-bottom headings, #f5f5f5 code blocks, collapse tables, `@page A4 18mm 20mm` margins, `<nav.toc>` with a light-grey card. Users can append custom CSS via `--doc-css` or replace entirely with `--no-default-css --doc-css print.css`.

**Tests added: +7.** Heading IDs emitted, TOC generated + depth limit honored, default CSS inlined + suppressible, GFM tables, slugify strips punctuation, custom CSS appended, script binding round-trips. 1133 → 1140 passing.

### 59. Script TCP + UDP server primitives (0.57.0)

Closes out the queued wishlist arc started back in the serialized-foraging-sutherland plan. TCP listen + accept + read + write, UDP bind + recv_from + send_to. Accept-with-timeout on the TCP side lets the main loop poll for shutdown between connections.

**Handle types wrap `Arc<Mutex<Option<T>>>`.** The `Option<T>` is there so `*_close` can drop the underlying socket without dropping the handle (scripts can still hold references). `Arc<Mutex<>>` gives us Send+Sync for the 0.56.0 `thread_spawn` pairing — accept on main, spawn a closure per connection. This is the primary use case for these bindings.

**Accept with timeout — set_nonblocking + spin.** `std::net::TcpListener` has no `accept_with_timeout`. For the timeout form we flip to non-blocking, poll with 20ms sleeps until either the accept succeeds or the deadline passes, then flip back to blocking for subsequent reads. ~80 LOC in `src/script/bindings/tcp_server.rs`. Clean, no tokio, no external crates.

**`tcp_read_line` vs `tcp_read(n)`.** Both supported. `read_line` is the common case for text protocols (SMTP lines, HTTP request lines, custom line-based IPC). `tcp_read(n, timeout_ms)` gives byte-precise control for binary protocols or peek-ahead patterns.

**ICMP deferred.** Originally planned for 0.57.0; pulled out to keep scope tight. recon already has `ping()` (ICMP + TCP ping modes) for basic reachability. Raw-socket send/recv for arbitrary ICMP types is niche; can land as 0.58.x or later if asked.

**No CLI flag surface.** The plan called this out explicitly — server workflows are multi-step and belong in scripts, not single-shot CLI invocations. `recon --serve` already covers quick HTTP serving. The new script bindings are purely additive.

**Tests added: +3.** listen + close round-trip, accept_timeout errors cleanly, UDP bind+close round-trip. 1130 → 1133 passing.

### 58. Script concurrency primitives — thread_spawn / channels / join (0.56.0)

Long-queued from the wishlist plan. The user wanted fork/thread/concurrency primitives "not only for servers" — this release lands the general-purpose surface so 0.57.0's TCP/UDP/ICMP server bindings can accept multiple connections concurrently.

**`rhai` `sync` feature flip.** Enabling the feature took one line in Cargo.toml but broke two binding modules that stored their state in `Rc<RefCell<_>>`. Both (`src/script/bindings/browser.rs` and `src/script/bindings/sqlite.rs`) were swapped to `Arc<Mutex<_>>`. No semantic change — the single-threaded-by-default behaviour is preserved; the sync feature just makes Send+Sync available for cross-thread moves. Per-value lock overhead runs ~10-15% on hot paths per rhai's own benchmarks, which is irrelevant for the diagnostic workloads recon runs.

**Spawn-worker design.** Each `thread_spawn` call builds a fresh engine in the worker thread via `build_engine(&defaults)`. Alternatives considered and rejected:
- `Arc<Engine>` shared between threads — Engine isn't `Clone` even with `sync`, and wrapping it adds lifetime friction to every binding registration.
- Single-engine, blocking-pool pattern — would require an executor + task queue; much more code for the same observable behaviour.

Fresh-engine-per-spawn is a few ms of registration overhead per spawn, dwarfed by any real script work. The AST is shared via `rhai::Shared<AST>` (cheap Arc clone). `ScriptDefaults` is `Arc<ScriptDefaults>` so CLI-flag inheritance reaches every worker.

**Nested `thread_spawn`.** The worker engine re-registers the threading bindings so nested `thread_spawn` / `send` / `recv` work inside spawned closures. Without this, a closure trying to spawn a sub-task or send on a channel would fail with "function not found".

**Rhai reserved-word gotcha.** `spawn` is a reserved keyword in Rhai. The binding is exposed as `thread_spawn`. The help topic mentions this explicitly so users don't hit the same rake.

**Channels — stdlib mpsc.** `std::sync::mpsc::channel` for unbounded, `sync_channel(capacity)` for bounded. Receiver wrapped in `Arc<Mutex<Receiver<Dynamic>>>` so it can be cloned across closures (Rhai custom types must be Clone). Sender stays thin — std's `Sender::clone` works natively. Channels transport `Dynamic` values, which Rhai clones on the send side; the receiver owns its copy.

**`FnPtr::call` is the dispatch primitive.** When a closure is passed into `thread_spawn`, it's captured as a `FnPtr` — a reference plus captured scope. In the worker, `fn_ptr.call::<Dynamic>(&engine, &ast, args)` runs it. Errors surface as strings via `.to_string()` and get re-raised from `join`.

**Tests added: +4.** tid returns an integer, channel send+recv, bounded channel try_send fills, recv_timeout errors when empty. 1126 → 1130 passing.

### 57. rxing-powered decode + Aztec/PDF417/MaxiCode (0.55.0)

Closes the big encoding-deferred block in OUT-OF-SCOPE.md: image→text decoding plus three new 2D encode formats. Single new crate (`rxing`) lands — pure-Rust port of ZXing, Google's canonical multi-format barcode library.

**Why `rxing` and not re-rolling.** ZXing is the reference implementation for Aztec, PDF417, and MaxiCode. Each format has finicky details (PDF417's stack-of-rows encoding, MaxiCode's hexagonal fixed-size grid, Aztec's five-layer structural variants). Writing one of these correctly is a multi-week project; writing three would dwarf the rest of this release. rxing ships all three plus the universal decoder under MIT/Apache dual licence. Binary growth: ~2.5 MB stripped release.

**Decoder design.** `rxing::helpers::detect_in_file` is the happy path — it opens an image file, runs format autodetection, and returns text + format. For stdin / in-memory blobs (the `decode_bytes` path), we write to `tempfile::NamedTempFile`, decode, and let the temp drop at end-of-scope. Routing through luma-pixel APIs (`detect_in_luma`) would bypass PNG/JPEG header parsing entirely and require pre-decoded pixel data — not what we want for a CLI that's handed a PNG.

**Coexistence with existing encoders.** QR and DataMatrix still flow through the `qrcode` and `datamatrix` crates for ASCII/SVG/PNG rendering, because recon's existing rendering pipeline is tuned for those output types. rxing only runs for Aztec/PDF417/MaxiCode encoding and for all decoding. A BitMatrix adapter (~12 lines) bridges rxing's matrix type into the recon `BitMatrix` shape that the existing render_{ascii,svg,png} functions consume.

**Format-name normalisation.** rxing's internal `BarcodeFormat::QR_CODE` vs curl/recon convention `qr`. `src/decode.rs::format_name` + `parse_format` provide a bidirectional mapping so CLI and script bindings speak the lowercase-hyphen form while rxing's typed enums stay internal.

**What's NOT shipped.** `--encode-hints` (passing codewords / ECI options / compact-vs-full selection for Aztec) — rxing's `encode_with_hints` API is there but the user-facing surface would need thinking. Revisit if demand surfaces. Multi-code scanning in one image (`detect_multiple_in_file`) also deferred — simple first.

**Tests added: +5.** parse_format round-trip, format_name stability for common types. Plus smoke-tested manually via the round-trip encode → decode of all three new formats. 1124 → 1129 passing.

### 56. Client-certificate package — mTLS (0.54.0)

Ships `--client-cert` / `--client-key` / `--cert-type` / `--key-type` / `--pass` as one release per the OUT-OF-SCOPE.md note that `--key-type` is a trap unless it arrives with the rest of the cluster. Closes the HTTP/curl-compat item.

**Flag-name collision with existing `--cert` inspection.** recon shipped `--cert` first as a bool for server-cert inspection (the "show me the CN, issuer, expiry" feature). curl's `--cert` means the client cert. Rather than break the inspection flag, new client-cert flag is `--client-cert` with the curl-compatible short `-E`. Split keeps both working.

**Loader design — `src/client_cert.rs`.** One byte-level concat path regardless of whether the caller started with a combined PEM or a cert+key pair. `reqwest::Identity::from_pem` accepts both, so handing it the concat keeps the loader tiny (~150 LOC) and collapses two code paths into one.

**Format matrix.**
- PEM cert + PEM key (split or combined): works.
- DER cert: rejected with `openssl x509 -inform DER -outform PEM` recipe.
- DER key: rejected with `openssl pkcs8 -in key.der -inform DER -out key.pem` recipe.
- ENG key: rejected immediately (rustls has no engine concept; pointing users at PEM keyfiles).
- Encrypted PKCS#8 (ENCRYPTED PRIVATE KEY block): detected by text scan and rejected with `openssl pkcs8 -in key.enc -out key.pem`.

All four refusal paths give a specific error message + a concrete recipe. Deferring in-process DER parsing + PKCS#8 decryption (would need the `pkcs8` crate and a key-conversion shim) beats shipping a half-working DER path that silently fails in a handshake.

**No CI integration test for a live mTLS handshake.** Setting up a mock mTLS server in CI adds meaningful test infra for one feature. Unit tests cover all five refusal paths + the format-validation happy path up to `Identity::from_pem`. Manual smoke against `https://client.badssl.com/` documented in the examples.

**Script opts-map keys.** Five new keys: `client_cert`, `client_key`, `cert_type`, `key_type`, `pass`. Mirrors the CLI one-to-one per the exposure policy.

**Tests added: +7.** Five refusal paths, one no-cert short-circuit, one format-validation-when-no-cert. 1117 → 1124 passing.

### 55. `--compare` + streaming file I/O + raw-print (0.53.0)

Quick-wins bundle — five independent items folded into one release because each on its own is small and the docs/test churn amortises well. Marks the opening of the post-curl-parity wishlist plan (`serialized-foraging-sutherland`).

**`--compare <A> <B>`** — `src/compare.rs`. Reuses the existing `src/source.rs` uniformity: each side is a URL, local path, or `-` (stdin), and HTTP(S) sides flow through `client::execute` so every request flag (`-H`, `-u`, `-L`, `-k`, cookies, proxy, HSTS) applies to a compare source just as it would to a normal request. The dispatch is a per-call `args.clone()` with `url` rewritten to the chosen side — `Args` gained a `Clone` derive for this single use case and it'll see further use in the planned script-threading release.

**Diff crate choice — `similar` 2.x.** Two viable options: `similar` (BurntSushi-style unified-diff + sequence matcher, no external deps, MIT) and `diffy` (simpler but tighter). Picked `similar` because it gives us unified + side-by-side + line-level granularity from one API and is the de-facto standard in the Rust cargo/ecosystem tooling. ~80 KB added to the dep tree.

**Binary detection heuristic.** NUL byte in the first 8 KiB on either side → skip line diff, emit byte-count delta. Same pattern as `diff -a`. No MIME sniffing, no false-positive paranoia around UTF-16 (rare enough to pay for with a `--compare-format unified` override). Exit codes follow `diff`: 0 identical, 1 differ, 2+ load error.

**`sxs` (side-by-side) format.** Column width computed from terminal width via `crossterm::terminal::size`. Simple left/right truncation (no word-wrap). Good enough for quick visual scans; users who want polished side-by-side piping should use `git diff --color-words` or similar.

**Script-binding design.** `compare(a, b)` takes two Blobs OR two strings already in memory. Deliberately does NOT accept URLs — scripts that want to compare URLs should fetch with `http()` first and then pass the response bytes to `compare()`. Keeps the binding focused and composable; keeps the URL-dispatch story entirely in CLI territory.

**Streaming file I/O.** `FileHandle` is a newtype around `Arc<Mutex<File>>` — not `Rc<RefCell<File>>`. Chose the sync-safe flavour deliberately: when the 0.56.0 rhai-threading release flips on the `sync` feature, every custom type registered into the engine needs to be `Send + Sync`. Building the handle that way today means zero refactor when the switch happens. Full set: `file_open/read/read_all/write/seek/tell/flush/close` plus whole-file conveniences `file_write_all`, `file_append_all`, `file_exists`, `file_size`, `file_delete`. Modes: `r`, `w` (truncate+create), `rw`, `rwc` (read+write+create+truncate, a.k.a. `w+`), `a` (append+create), `ra` (append+read).

**Raw-print bindings.** Rhai's built-in `print()` appends a newline and goes through the engine debug callback. `print_raw(s|blob)` writes byte-precise to stdout and flushes. `eprint(s)` + `eprint_raw(s|blob)` are the stderr siblings. `flush()` is explicit. Small (~40 lines) but user-requested and unblocks progress bars, sub-line output, and binary-precise output.

**QR error-correction tuning.** `--qr-level <L|M|Q|H>` + `qr_level` script-opts key. Trivial pass-through to `qrcode::QrCode::with_error_correction_level`. Shipping as a tiny convenience ahead of the bigger 0.55.0 encoding overhaul (rxing-based decoding + Aztec/PDF417/MaxiCode).

**Tests added: +18.** Compare unit tests (parse, verdict, exit code, binary detection, line delta), streaming-file tests (round-trip, append, exists/size/delete, streaming handle read-write-seek, unknown mode), compare-binding tests. 1099 → 1117 passing.

### 54. HSTS persistent cache (0.52.0)

Third and final curl-parity phase-6 release. Closes the `--hsts` gap. `reqwest` has zero HSTS primitives so I hand-rolled a ~300-line store (parse / match / update / save) in `src/hsts.rs`.

**File format chosen for curl compatibility.** Plain TSV, one entry per line: `hostname expires_unix`. A leading `.` on the hostname indicates `includeSubDomains`. Comment lines start with `#`. No per-entry metadata beyond expiry + the subdomains flag — matches what RFC 6797 requires. You can swap files between curl and recon without conversion.

**Integration touch-points in `src/client.rs::execute` + `src/main.rs`.**
- *Before sending*: the main.rs hook (right after the ipfs:// rewrite) inspects `args.url` / `args.url_flag`. If either is `http://` and the hostname has a non-expired cache entry, rewrite in place to `https://`. Prints a stderr line at default verbosity (suppressed by `-s`) so users know an upgrade happened.
- *After receiving*: `src/client.rs::update_hsts_from_response` parses the `Strict-Transport-Security` header from the final response. Gates on `scheme == "https"` because RFC 6797 says STS directives from `http://` are non-authoritative. Calls `store.update_from_sts_header(host, value)` which returns true when the store changed — only then save (keeps I/O idempotent).

**Subdomain matching.** `store.matches(host)` walks up the dotted hierarchy: exact match first, then each parent suffix checked for an entry with `include_subdomains = true`. So a cache entry for `.app` matches `myapp.app` but a bare `example.com` entry does not match `foo.example.com`. Standard HSTS semantics.

**Atomic save via `tempfile::NamedTempFile::persist`.** Write to a sibling tempfile in the cache's parent dir, then rename into place. Matches what the existing cookie jar save path does (via `rusqlite` transactions). A save that fails mid-write can't corrupt the existing cache.

**RFC 6797 behaviour NOT implemented.**
- HSTS preload list isn't bundled (16 MB blob; scripts can pre-populate the file from the Chromium preload list if they want).
- `preload` directive from server STS headers is parsed but not acted on (recon can't add entries to the global preload list).

**`-k` still works with `--hsts`.** HSTS upgrades `http://` to `https://`, then `-k` disables cert verification after the scheme change. Useful for diagnosing self-signed-cert HSTS-protected hosts; risky in production. Documented in the help topic.

**Tests added: +10.** STS header parsing (plain max-age, quoted, with includeSubDomains, missing max-age), store round-trip (save + load), load-missing-file UX, malformed-line tolerance, subdomain matching, expiry enforcement, insert-then-remove-on-zero. 1088 → 1098 passing.

### 53. Unix-domain sockets (0.51.0)

Second curl-parity phase-6 release. Hand-rolled minimal HTTP/1.1 client over `std::os::unix::net::UnixStream`. Sits in `src/unix_socket.rs` as a peer to `client.rs`, not as a feature flag on top of reqwest — because reqwest's blocking client has zero UDS support, and building the async-inside-sync bridge (tokio runtime + custom `Connect` trait impl + re-implementing every reqwest feature we already depend on for TCP) was out of proportion for the use case.

**Scope intentionally narrow.** HTTP/1.1 only. No TLS (makes no sense over a local socket). No HTTP/2 (local peers don't need it). No redirects (UDS endpoints don't redirect). No chunked-decoding (Docker, systemd, kubelet all return Content-Length). This cuts the implementation to ~350 lines, about 80% of which is verbatim header parsing that I can trust to be correct because the protocol is ancient and stable.

**URL grammar tolerant.** Accepts `http://host/path`, `https://host/path` (the `https://` scheme is a lie — transport stays plaintext over UDS — but users type it habitually, so we accept it), and `/path` alone (Host defaults to `localhost`). The host string becomes the `Host:` request header; the path is what goes on the wire.

**Dispatch placement.** main.rs checks `args.unix_socket.is_some()` right before the fallback HTTP path. When set, routes to `unix_socket::run(&args)` which handles verbose-header rendering + body writing (to stdout or -o). When unset, the normal HTTP path runs as before.

**Script-binding integration.** `http(url, #{ unix_socket: "/path" })` — adding one opts key + a dispatch check inside `do_request`. UDS response shape matches `http()`'s (url, final_url, status, body, body_bytes, charset, headers, http_version, duration_ms) so scripts can't tell the difference. `charset` is always `()` since UDS payloads aren't decoded (content-type parsing skipped).

**Why the shape match matters.** The script-binding policy from CLAUDE.md says every surface gets every feature. If UDS returned a different shape, scripts would have to branch on transport. Now a script like `http(url, #{unix_socket: sock})` composes into any existing pipeline that works with `http()`.

**Docker API smoke test.** `recon --unix-socket /var/run/docker.sock http://localhost/_ping` returned "OK\n" with the full Docker response-header set (Api-Version, Server: Docker/29.4.0, etc.). End-to-end verified.

**Tests added: +7.** URL-grammar coverage (http://, https://, path-only, non-HTTP scheme rejection), status-line parser (standard, no-reason-phrase, malformed). 1081 → 1088 passing.

### 52. Proxy suite — HTTP / HTTPS-to-proxy / SOCKS5 (0.50.0)

First of three planned curl-parity phase-6 releases. Surprising discovery during scoping: recon shipped **zero proxy support** prior to 0.50.0 — not even `$HTTP_PROXY` env-var honoring. Every request went direct to the origin. So what the user framed as "HTTPS-proxy" became a full proxy suite.

**reqwest's Proxy API as the foundation.** reqwest 0.12 has `Proxy::all(url)` that auto-dispatches based on the URL's scheme: http://, https://, socks5://, socks5h://. Enabling the `socks` feature flag on reqwest adds the SOCKS variants. One `builder.proxy(p)` call installs the whole stack. Trying to use `Proxy::http` / `Proxy::https` for finer control would've forced us to reason about which scheme a user's target URL has *before* the proxy is picked — not useful.

**Env-var precedence matches curl.** `$HTTPS_PROXY` wins for https:// targets, `$HTTP_PROXY` for http://, `$ALL_PROXY` as final fallback. Both the SHOUTY and lowercase forms are read. The CLI flag (`--proxy` / `-x`) beats any env var. Empty or whitespace-only env values are ignored (some shells export `HTTP_PROXY=""` which we treat as "no proxy").

**Credential resolution order.** `--proxy-user USER:PASS` beats URL userinfo (`http://alice:secret@proxy:3128`). Neither overrides the origin's `-u` flag — these are distinct namespaces.

**`--noproxy` grammar.** Comma-separated entries. Exact hostname match, `.suffix` matches subdomains (so `.internal` matches `foo.internal` and `bar.internal` but NOT `internal`), `*` bypasses everything. Falls back to `$NO_PROXY`. Delegates to `reqwest::NoProxy::from_string` for the actual matching — reqwest has the same parsing rules as curl.

**`--proxy-cacert` global-scope caveat.** reqwest 0.12 doesn't expose a per-proxy TLS config; adding a CA root via `ClientBuilder::add_root_certificate` applies it globally (both proxy and origin connections see it). Documented in the help topic. For most users this is fine — a corporate trust root typically signs both the proxy and the origin. Users who need scoped trust should use `curl` for that specific request.

**Script-binding extension.** Five new opts keys on `http(url, opts)`: `proxy`, `proxy_user`, `noproxy`, `proxy_insecure`, `proxy_cacert`. All flow through the existing `build_args` overlay — zero new code paths in the script surface.

**Ancillary: `recon --version` now sorted + backfilled.** The PROTOCOLS list was stale; everything shipped in 0.44.0-0.49.0 (ftp, sftp, tftp, gopher, pop3, imap, ipfs, smtp, and their TLS variants) was missing. Added them + sorted case-insensitively so the output reads like curl's. FEATURES list got ~15 new tokens covering the capabilities shipped since the last refresh. Five-line change in `src/version.rs`.

**`docs/curl-parity-matrix.md` (new).** Quick-reference covering every curl feature: shipped, always-on via Rust, architectural N/A, deferred. Lands alongside 0.50.0 so users asking "does recon support X?" have a one-click answer. Explicitly tracks the 5 already-present items (AsynchDNS, Largefile, libz, threadsafe, HTTPS-proxy primitives) that a user ran through — they deserve documented coverage, not silent presence.

**Tests added: +6.** Proxy-URL resolution (explicit > env > none), https-target picks HTTPS_PROXY, empty env = None, credential priority, noproxy flag vs env. 1075 → 1081 passing. No integration test against a live HTTP proxy; curl's integration tests would be good to borrow but we rely on reqwest's unit tests for the proxy primitives. Smoke test would be any user running `--proxy http://their-corp-proxy:3128 https://httpbin.org/ip` and seeing the origin IP change to the proxy's.

### 51. IPFS / IPNS via HTTP gateway (0.49.0)

Closes the three-release protocol-coverage arc. Unlike 0.47.0 (file-transfer) and 0.48.0 (mail-retrieval), IPFS isn't "a protocol to speak" in the conventional sense — it's a content-addressing system whose canonical access path is already HTTP. The implementation is therefore a ~80-line URL rewriter plus a script-side convenience function.

**Why rewrite instead of native?** `rust-ipfs` is alpha (0.x for years), has a libp2p dep tree that would bloat the recon binary by several MB, and requires either a local IPFS node or peer discovery to resolve content. By contrast, public HTTP gateways (ipfs.io, cloudflare-ipfs.com, dweb.link) and local Kubo daemons are how the ecosystem actually serves IPFS content. A rewrite captures 100% of the real-world use case with 0% of the dep weight.

**Rewrite mechanics.** `src/ipfs.rs::rewrite_url` splits on the scheme, extracts the CID / IPNS-name + optional path, and joins with the gateway URL. Gateway resolution: `--ipfs-gateway` CLI flag → `$RECON_IPFS_GATEWAY` env → `https://ipfs.io` default. Trailing slashes on the gateway are trimmed so `https://ipfs.io/` and `https://ipfs.io` produce identical output. All unit-tested.

**Dispatch integration.** Rather than add a new `else if` branch to the URL-scheme dispatch in main.rs, I rewrite the URL in place (`args.url` and `args.url_flag`) immediately after argument parsing. The HTTP pipeline then sees the gateway URL and does its normal thing. This means every HTTP flag — headers, output path, TLS verification, rate limiting, charset transcoding, everything — works for IPFS URLs without a single special case downstream.

**Script binding.** Just `ipfs_url(url [, opts])` — a thin wrapper over `rewrite_url`. Scripts that want to fetch can compose with `http()`: `http(ipfs_url("ipfs://bafy..."))`. Alternatively, scripts can pass the `ipfs://` URL directly to `http()` — but since `http()` doesn't auto-rewrite script-internal URLs (no argument interception at that layer), `ipfs_url()` is the explicit path for scripts.

**Tests added: +10.** 1066 → 1076 passing. URL-rewrite cases (default gateway, custom gateway, env var, trailing-slash handling, non-IPFS passthrough) plus the script-binding suite.

### 50. Mail retrieval — POP3/POP3S + IMAP/IMAPS (0.48.0)

Second of three planned protocol-coverage releases. Text-protocol POP3 is hand-rolled mirroring SMTP; IMAP's LITERAL + FETCH parsing is complex enough to justify a crate dependency.

**POP3 hand-roll.** ~400 lines. Three session variants (plain, implicit TLS via pop3s://, STARTTLS upgrade via `--stls`) share read_line / capability-fetch / auth / retrieve helpers but have to be split at the TLS boundary because the `BufReader<TcpStream>` vs. `rustls::StreamOwned<_>` types are different and the `Read + Write` trait object would need more setup than the duplication saves. The plain session and the TLS session have parallel implementations — ~30 lines of duplication for a clean borrow story.

**IMAP via `imap = "3.0.0-alpha.15"`.** The stable 2.x releases don't carry the `rustls-tls` feature; 3.x is still alpha but has `ClientBuilder::mode(ConnectionMode::{Tls, StartTls, Plaintext})` which cleanly covers `imap://` + `imaps://`. `Client::capabilities()` + `Client::login()` → `Session`; from there `examine()` / `uid_fetch()` do the heavy lifting.

**URL path grammar.** Both probes match curl's convention: empty path = probe, non-empty = target. POP3 uses the path as a message number (RETR N). IMAP uses it as a mailbox name optionally followed by `;UID=N` for a fetch. Parse the URL once, dispatch to the right action inside the session.

**Gotchas.**
1. imap 3-alpha's `ClientBuilder` has no TLS-verifier override hook — `--insecure` is accepted at the CLI but not plumbed through for IMAP. Documented as a known limitation in CHANGELOG; tracked for a follow-up when the upstream API exposes it. POP3's hand-rolled TLS respects `--insecure` normally.
2. imap 2.x has a `rustls-tls` feature that was removed by 3.x alpha (which uses `rustls-connector` unconditionally). Picked 3.x despite the alpha tag because 2.x's TLS story is worse.
3. `Client::logout()` only exists on `Session` (authenticated). Unauthenticated probe path drops the `Client` to close; no explicit LOGOUT is needed before the underlying socket closes.

**Tests added: +10.** 1056 → 1066 passing. URL parsing (mailbox vs. UID vs. probe for IMAP; message number vs. probe for POP3), STAT parsing, URL credential extraction. Smoke-tested against `pop.gmail.com:995` and `imap.gmail.com:993` — both return capability lists as expected.

### 49. File-transfer protocol family — FTP/FTPS, SFTP, TFTP, Gopher (0.47.0)

First of three planned releases expanding recon's protocol surface toward curl's coverage. Four URL schemes landed together because they share testing scaffolding and docs structure even though the underlying code paths are wildly different (suppaftp crate for FTP, ssh2 for SFTP, hand-rolled UDP for TFTP, hand-rolled TCP for Gopher).

**FTP (suppaftp).** Added `suppaftp = "6"` with `rustls` feature for consistency with recon's existing HTTPS stack. The crate's type system uses `ImplFtpStream<T>` where `T` is `NoTlsStream` or `RustlsStream` — plain FTP and FTPS are genuinely different types, not just a flag. The internal `TlsStream` trait used as a generic bound is private in suppaftp's API, so the code duplicates the path-op logic (list vs. retrieve) for each stream type rather than reaching for a `do_path_op<T>` generic. ~10 lines of duplication in exchange for compiling.

**SFTP (ssh2::Sftp).** Reuses the existing SSH auth machinery from `src/ssh_auth.rs` (shared by scp:// and ssh://) verbatim — `resolve_credentials`, `verify_host_key`, `authenticate`. Path semantics match curl: `sftp://user@host/` lists home, `sftp://user@host/dir/` lists the directory, no trailing slash retrieves the file. ssh2's `Sftp::readdir(path)` returns `Vec<(PathBuf, FileStat)>`; I flatten `file_name()` to a basename string and expose `size`, `is_dir`, and `mode` per entry. Zero new crate deps — ssh2 was already in recon from 0.5.0.

**TFTP (hand-rolled).** Following the `ntp_probe.rs` pattern, hand-rolled over `UdpSocket`. Wire format is trivial: RRQ packet → DATA blocks → ACK each, terminate when a DATA packet is smaller than the negotiated block size. RFC 2348 `blksize` option negotiation: if the client asks, the server replies with an OACK packet; the client ACKs block 0 to start the transfer. ~220 lines including robust re-ACK-on-duplicate handling.

**Gopher (hand-rolled).** ~50 lines of protocol: TCP connect, send `selector\r\n`, read until close. TLS variant (`gophers://`) wraps the TCP stream in rustls using a webpki root store; `-k` installs a NoopVerifier (same pattern as RTSP). URL grammar extracts a single-char item type from the first path segment for informational display; the remainder of the path is the selector that goes on the wire.

**API warts worth noting:**
1. `suppaftp::RustlsStream` is `pub(crate)` — can't be named in a generic bound without reaching into `sync_ftp::tls` which is also private. Hence the duplicated path-op functions.
2. suppaftp's `into_secure` keeps the `T` parameter but requires `TlsConnector<Stream = T>`. You can't start with a plain `FtpStream` (T = `NoTlsStream`) and upgrade to RustlsStream — you have to start with `RustlsFtpStream` (T = `RustlsStream`) from the beginning and then call `into_secure`. Caught that on the second compile iteration.
3. Implicit FTPS (port 990, TLS before greeting) needs a different suppaftp entry point (`connect_secure_implicit`) with a different signature that wants a `ToSocketAddrs` + connector; deferred to a follow-up because none of our target test servers use it.

**URL-parsing consistency.** All four probes settled on the same parse skeleton: strip scheme, split on first `/`, separate authority (host + optional port) from path, preserve trailing-slash semantics where it matters (FTP, SFTP use it for list-vs-retrieve). FTP additionally percent-decodes URL userinfo because `url::Url` doesn't.

**Tests added: +19.** 1037 → 1056 passing. Per-protocol URL parsers each get 4-7 unit tests; credential resolution for FTP gets its own battery. No integration tests against live servers — smoke-tested manually against `ftp.gnu.org`, `test.rebex.net` (FTP/FTPS/SFTP demo server), `gopher.floodgap.com`.

**OUT-OF-SCOPE.md cleanup.** Removed FTP, TFTP, GOPHER, SFTP from the "permanently out of scope" catchall (the blanket statement has been progressively trimmed since 0.44.0 when SMTP first punched through it). Only SMB/SMBS remains deferred with its own rationale block.

### 48. Encryption expansion — PGP shell-out + rekey (0.46.0)

Three items from OUT-OF-SCOPE.md's "Encryption" section; only two shipped this release.

**PGP via `gpg` subprocess.** Chose shell-out over pure-Rust OpenPGP crates (`pgp` / rpgp, `sequoia-openpgp`) to avoid adding a new crypto surface to audit. `gpg` is practically universal on Linux / macOS; the subprocess approach also inherits the user's existing keyring configuration — no separate key import step. Downside: Windows users need to install `gnupg` separately (same as git commit signing). `require_gpg_binary()` runs `gpg --version` on every operation and emits a clear "install gnupg" error when absent.

**Backend auto-detection.** `detect_backend(args)` classifies recipients: `age1…` prefix or existing file path → age; anything else (hex fingerprint, email, key-id) → PGP. Explicit `--pgp` / `--age` overrides the heuristic. A mix of age + PGP recipients is currently classified as PGP — no cross-backend recipient bundles in one invocation (would require both backends to encrypt to a shared symmetric key, out of scope).

**Decrypt auto-routing.** `run_decrypt` now peeks at the input's magic bytes. `"-----BEGIN PGP MESSAGE-----"` (armored) or packet-tag high-bit-set first byte (binary) → PGP. Age files (`"age-encryption.org/v1"` / `"-----BEGIN AGE ENCRYPTED FILE-----"`) take the existing path. `--pgp` forces PGP regardless of magic bytes.

**`--rekey` as key rotation.** Decrypt-then-encrypt flow. Reuses existing `--identity` (old keys) and `--recipient` (new keys) rather than inventing new flag names. Source format sniffed; target backend follows the same auto-detection as plain `--encrypt`. Cross-backend rotation works (age → PGP or PGP → age) by pairing `--rekey` with `--pgp` / `--age`. Plaintext passes through memory briefly — no on-disk intermediate. Trade-off vs. atomic rotation: none; the input is untouched.

**Mixed recipient + passphrase dropped.** Plan said this was achievable ("age's file format supports this natively"). Reality: age 0.11's `Encryptor::with_recipients` hardcodes a rejection — `"scrypt::Recipient can't be used with other recipients"`. Producing the right header requires bypassing the Encryptor and writing stanzas directly, which is a significant re-implementation of age's core. Removed from this release, documented in OUT-OF-SCOPE.md with the rationale. One unit test + ~50 lines of implementation deleted.

**Hardware-backed keys (`age-plugin-*`) stays deferred.** age 0.11 doesn't expose plugin hooks in its public API. Implementing the plugin-protocol state machine ourselves (stdin/stdout framing between recon and e.g. `age-plugin-yubikey`) would be ~200 lines plus ongoing compatibility surface — deferred until either age bumps its API or a concrete user asks. GPG smartcard keys come for free via the new shell-out: `gpg` already knows how to talk to YubiKey / SmartCard-HSM / etc. when the user's keyring is configured.

**Tests added: +8.** `detect_backend` (5 cases: all age, hex fingerprint triggers PGP, email triggers PGP, explicit `--age` overrides, `--pgp`+`--age` rejected), magic-byte detection (`looks_like_pgp`, `looks_like_age`), age rekey round-trip. PGP-specific tests would require a test keyring — not added; smoke-tested manually with a local `gpg --quick-gen-key` key.

**Smoke tested.** age rekey round-trip with `recon --encrypt-keygen` generating old + new keys, encrypting then `--rekey`ing then decrypting with the new identity: roundtrips cleanly. Tests: 1027 → 1035 passing.

### 47. MQTT 5 power-user features (0.45.0)

The six items deferred from 0.22.0. All six are pure MQTT-5 connect/publish/subscribe property machinery — rumqttc 0.24 had the structs (`ConnectProperties`, `LastWill`, `PublishProperties`, `SubscribeProperties`) from day one; recon just never wired them through.

**Implementation was mostly plumbing.** `setup_options_v5` now builds a `ConnectProperties` when any of `session_expiry`, `user_properties`, `auth_method`, or `auth_data` is set, and attaches a `LastWill` when `--will-topic` is given. `publish_v5` switches to `publish_with_properties` when `publish_properties(args)` returns `Some`, falling back to the no-property path otherwise (keeps the wire bytes minimal for simple publishes). Same pattern for `subscribe_v5` + `subscribe_with_properties`.

**v3 silent-ignore.** The v5 properties are v5-only at the protocol level. Rather than gate each flag with an explicit `--mqtt-version` check, the flags route through only in the v5 setup path. On `--mqtt-version 3`, the fields are collected but never consulted. Documented in the help topic ("Ignored on --mqtt-version 3").

**API gotcha worth noting.** `LastWill::new(topic, payload, ...)` takes `topic: impl Into<String>` and `payload: impl Into<Vec<u8>>`. First try passed `topic.as_bytes()` for the topic — that's `&[u8]` which doesn't impl `Into<String>`. Use `.as_str()` for the topic or build via the public field `LastWill { topic: Bytes, ... }`.

**`parse_user_properties` helper** — splits `KEY=VAL` specs from the repeatable `--user-property` flag. Same function feeds both `ConnectProperties` and `SubscribeProperties`. `--user-property` on publish goes through `PublishProperties.user_properties` (not ConnectProperties).

**Script binding expansion.** Opts map accepts user-properties as either an Array of `"key=value"` strings or an Array of `#{key, value}` maps — both are natural idioms, and the ambiguity is cheap to handle. Will-message opts are a nested Map rather than four flat keys (`will.topic`, `will.payload`, `will.qos`, `will.retain`) so scripts can build it once and reuse.

**OUT-OF-SCOPE.md diff.** Removed the six landed items. Kept the two that remain deferred: client-cert mTLS (blocked on recon's HTTPS stack gaining mTLS too — unify when both land) and the dual-rustls-major coexistence (wait for rumqttc to bump to rustls 0.23).

**Tests added: +0.** The new code paths are exercised indirectly by the existing MQTT test suite — no regressions. Explicit tests for property propagation would need a mock broker that inspects incoming packet properties, which isn't shipped today. Smoke-tested against `test.mosquitto.org` by running `recon mqtt://test.mosquitto.org/recon --user-property env=test -d 'hi'` and confirming the packet structure via `mosquitto_sub -v`.

### 46. SMTP / SMTPS probe + mail delivery + DKIM signing — 0.44.0

Motivating case: recon already has DNS-side email-security checks (SPF, DMARC, DKIM-record, MTA-STS, BIMI, TLS-RPT) but nothing that actually talks to an SMTP server on the wire. Deferred from early brainstorming under "pending demand"; now shipped as `smtp://` / `smtps://` probe + send modes.

**Two modes in one binary, split on `--mail-from`.** Probe mode hand-rolls the SMTP conversation over `TcpStream` so every EHLO line is preserved verbatim. Send mode delegates to `lettre` because DKIM canonicalisation + correct MIME framing + authenticated-submission plumbing is a lot of code to maintain ourselves. This split means users get full diagnostic detail when they want to see what the server advertised, and a real SMTP client (with DKIM) when they want to deliver.

**lettre 0.11 + ring for RustCrypto.** Added `lettre = { version = "0.11", default-features = false, features = ["smtp-transport", "rustls-tls", "ring", "builder", "dkim"] }`. Picked rustls over native-tls for consistency with the existing HTTPS stack. The `dkim` feature pulls in `rsa` and `ed25519-dalek` — algorithm is auto-detected from the PEM (Ed25519 keys are short and lack the `RSA PRIVATE KEY` guard).

**API wrinkles worth remembering.** `MessageBuilder::header<H: Header>` takes typed headers (Subject, From, To, Content-Type…); raw `"X-Custom: value"` inputs need `builder.raw_header(HeaderValue::new(HeaderName::new_from_ascii(name)?, value))`. The DKIM `header_names` list is `Vec<HeaderName>`, not `Vec<String>` — no FromStr impl. `ProtocolExitCode::LoginDenied` is the 67 variant; "AuthRequired" doesn't exist (I reached for that name first and had to correct).

**Probe-mode implementation detail.** Reads the greeting, then sends EHLO, then reads the multi-line reply, then QUIT (errors on QUIT ignored — some servers drop the connection on sight). Capabilities parser strips the leading `250-` / `250 ` prefix and drops the greeting echo (first line). `parse_ehlo_capabilities` returns `Vec<String>` — "AUTH LOGIN PLAIN", "SIZE 14680064", "STARTTLS", etc. — callers that want the AUTH mechanisms parse the `AUTH ` line themselves.

**Send-mode DKIM.** Requires the private key PEM + selector + (optional) domain. Algorithm inferred heuristically: if the PEM contains `BEGIN PRIVATE KEY` without `RSA PRIVATE KEY` and is short, assume Ed25519; otherwise RSA. Signed headers: From, To, Subject, Date (the standard minimal set for interop). Canonicalisation: relaxed/relaxed.

**Exit-code mapping.** `LoginDenied` when the error message contains "auth"; `CouldntConnect` otherwise. `OperationTimedOut` propagates naturally from `lettre`'s timeout errors wrapped in the catch-all match.

**OUT-OF-SCOPE.md cleanup.** Removed the SMTP line from the "Deferred" section. Updated the "permanent" exclusion line (which listed LDAP, RTSP, DICT etc. as non-HTTP-only — stale since those all shipped as probes between 0.24.0 and 0.44.0) to reflect reality: only FTP / TFTP / GOPHER / SMB / POP3 / IMAP remain permanently excluded.

**Tests added: +9.** URL parsing, default-port resolution, IPv6-literal handling, EHLO parser, queued-ID extraction, script binding opts mapping. No integration tests spinning up a real SMTP server in-process — deferred until a `mailin-embedded`-style dev-dep is added. Smoke testing covered by running against `smtp://gmail-smtp-in.l.google.com:25/` and `smtp://smtp.gmail.com:587/` manually.

### 45. Text encoding (charsets, iconv) — 0.43.0

Motivating use case: a PHP service talking UTF-8 and a Perl service talking ISO-8859-1 that exchange data via recon. Before 0.43.0, non-UTF-8 responses were silently mangled (`response.text()` lossy-decodes as UTF-8), and request bodies were sent as the shell's UTF-8 regardless of the declared Content-Type charset. Three silent traps fixed in one release.

**Library choices.** `encoding_rs 0.8` (the WHATWG-compliant charset crate that Servo and `reqwest`'s internals already depend on) + `chardetng 0.1` (Mozilla's pure-Rust heuristic auto-detector). Both are the canonical picks; no alternative was seriously considered. Combined binary-size impact: ~220 KB.

**Source-charset priority.** Locked at design time: explicit `--source-charset` > response `Content-Type: ...; charset=NAME` > BOM sniff > chardetng heuristic > windows-1252 fallback. The fallback matches browser behaviour (HTML5 spec says windows-1252 for text/* without a declared charset). Users who want strictness can pass `--source-charset` explicitly.

**Where transcoding plugs in.**
- *Response side:* `src/output.rs::write_response_to` had two paths — a prettify path that already buffered via `response.text()` and a raw path that streamed via `io::copy`. When `--output-charset` is set, both paths now buffer via `response.bytes()` and transcode. The streaming zero-copy path is preserved verbatim when no charset flag is set — critical for multi-GB binary downloads.
- *Request side:* every `request.body(...)` call site in `src/client.rs` (six of them, one per body-source flag) routes through a new `apply_request_body()` helper. Helper checks `--request-charset-passthrough` first, then `--request-charset` explicit, then scans `args.header` for an explicit `Content-Type: ...; charset=X`. UTF-8 targets pass through; anything else gets decoded-then-re-encoded via `encoding_rs`.

**Unmappable characters.** Substituted with `?` and a stderr warning (suppressed by `-s`). Matches iconv's `-c` default. An emoji destined for ISO-8859-1 becomes `?`; a French accented letter passes through fine.

**Script surface.** `text::*` static module mirrors the `compression::*` / `archive::*` pattern. Key insight: since scripts already get `r.body` as a lossy UTF-8 String, the fix isn't to change `body` (breaks scripts that today decode ASCII fine); it's to *add* `body_bytes` (raw Blob) and `charset` (resolved String or `()`) alongside. Scripts that need correctness use `text::decode(r.body_bytes, r.charset ?? "windows-1252")` — one line — and scripts that were already handling ASCII keep working unchanged.

**Standalone `--iconv` mode.** Once the core transcoding helper existed, exposing it as a file/stdin mode cost ~80 lines in `src/iconv.rs`. Format matches `iconv(1)`'s `-f FROM -t TO` but packed into a single `FROM:TO` flag for parseability. Blank `FROM` means auto-detect — niche but occasionally useful when the source format is unknown. Early-intercept in `main.rs` alongside `--init` / `--browser-screenshot`; no HTTP pipeline involvement.

**Tests added: +39.** 979 → 1018 total. Breakdown: `text_encoding::tests` (17 — resolve/alias/parse-content-type/transcode/detect/strip_bom/encode), `iconv::tests` (4 — spec parsing), `text` binding tests (11 — scripting round-trips), `charset_it.rs` integration (7 — end-to-end wiremock).

**Named gotchas during implementation.**
1. `parse_content_type_charset` initially used `?` on `split_once('=')` inside the per-part loop — first part `"text/html"` has no `=`, so the whole function returned None. Fixed with `let-else continue`.
2. `list_charsets` and `iconv` needed to be added to the `required_unless_present_any` list on the positional `url` arg, or clap enforced a URL argument before the early-intercept code could run.
3. The script response-charset sniff has to clone `response.headers()` into an owned `HeaderMap` because `response.bytes()` moves `response` — a common reqwest snag.

### 44. Scriptable `browser()` with sticky sessions (0.42.0)

Until now every `http(url, opts)` call rebuilt a reqwest client from scratch, loaded the jar from `args.cookiejar` if set at CLI level, and threw the client away. Scripts that wanted session-sticky behaviour had to either copy the same opts map on every line or reach for `sqlite("cookiejar:NAME")` and hand-apply cookie rules. `browser()` gives scripts a stateful handle that holds configuration + a jar across calls. Script-only; no CLI flag maps here.

**Handle pattern.** Mirrors `SqliteHandle` at `src/script/bindings/sqlite.rs:24-28`:

```rust
#[derive(Clone)]
pub struct BrowserHandle {
    state: Rc<RefCell<BrowserState>>,
}
```

Cheap-clone + interior mutability lets `&mut self` method closures work without fighting Rhai's `!Sync` engine, and lets a script pass the same handle around without aliasing issues.

**Jar strategy.** `BrowserState::jar` is a `JarLocation` enum: `Temp(tempfile::NamedTempFile)` (default) or `Named(String)` (→ `~/.recon/jars/NAME.db`). The temp file auto-deletes when the browser is dropped. `use_persistent_session(name)` simply swaps the variant — the cookie-loading code path in `src/client.rs:119-138` doesn't care whether the jar file is permanent or temp; it just opens the path and calls the existing `cookie_header` / `save_cookies` helpers. Zero refactor of the request pipeline. Zero new cookie-storage code.

**Request dispatch.** `do_request` reuses `http::build_args` (promoted to `pub(crate)` in this release) to build an `Args`, then overlays browser-level state (user-agent, session headers, timeouts, basic-auth, jar path) in a specific order: CLI defaults → browser config → per-call opts always wins. `args.cookiejar` is always set to the browser's jar path, regardless of what was on the CLI.

**Body coercion.** `coerce_body(Dynamic)` accepts String, Blob, Map, Array. Maps and arrays auto-serialise to JSON via `helpers::dynamic_to_json` (promoted to `pub(crate)` in this release). When a JSON body goes out, `content-type: application/json` is appended to `args.header` unless the script already set one (case-insensitive check). String/Blob bodies pass through verbatim — no auto content-type.

**Multiple browsers in one script.** No concurrency primitives needed. Rhai is single-threaded; the engine isn't `Sync`. "Parallel browsers" just means multiple `BrowserHandle` instances, each with its own `Rc<RefCell<BrowserState>>` and its own `NamedTempFile`. Scripts interleave calls as needed. This was the design the user actually wanted — confirmed before implementation.

**`--help browser` reassignment.** Before 0.42.0, `recon --help browser` resolved to `TOPIC_AGENT_BROWSER` (real browser automation). Now it resolves to `TOPIC_BROWSER` (the scripting session handle). The real-automation topic keeps its `agent-browser` and `agentbrowser` keys. Reasoning: scripts are the common discovery path; someone typing `--help browser` is more likely building a session-sticky scraper than reaching for the external `agent-browser` CLI.

**Tests added: +10.** 970 → 980 total, 2 ignored. All wiremock-based, covering sticky cookies, per-browser isolation, header persistence, Map-body-to-JSON, String-body passthrough, opts-override-browser precedence, cookies listing, clear-cookies wipe, fresh-swap on `use_persistent_session`, and ephemeral-vs-named `session_name()` behaviour.

### 43. Per-module example scripts + docs sweep (0.41.0)

Follow-up to #42: with seven new bindings landed, the `script/` directory gets a companion `.rhai` file per binding module so users can see the minimal idiom at a glance. Before 0.41.0 the directory had five `browser-*.rhai` recipes and nothing else.

**Scope.** One example per module, deliberately minimal (~15 lines each). 21 new files:

- Protocol probes: `http`, `tcp`, `ping`, `dns`, `tls`, `ntp`, `redis`, `ws`, `dict`, `ldap`, `whois`, `memcached`, `rtsp`, `mqtt`.
- Data primitives: `file`, `hash`, `compression`, `archive`, `sqlite`.
- Domain tools (new in 0.40.0): `encode`, `encrypt`, `checkdigit`, `sample`, `jwt`, `email`, `netstatus`.
- `agent-browser.rhai` as a minimal example beyond the existing `browser-*.rhai` recipes.

**Convention per file.**
- Header comment: `// Usage: recon --script <name> [ARGS]` plus a one-paragraph description.
- Single positional arg defaulted from a sensible test target (`example.com`, `Cargo.toml`, `127.0.0.1:6379`, etc.) so every script works with zero args.
- `return 0` on success, non-zero when an upstream precondition fails.
- Scripts hitting external services (`redis`, `memcached`, `mqtt`) start with a `tcp()` reachability probe and return 2 (skip) when the service isn't listening — keeps the "run all examples" smoke loop from flaking.

**README rewrite.** `script/README.md` went from a few paragraphs about the browser examples to a categorised index: four tables (Protocol probes / Data primitives / Domain tools / Browser automation) with one-line descriptions per file. Usage section covers direct execution, installation into `~/.recon/script/`, and the guard pattern.

**Mechanical parse validation.** `tests/script_examples_it.rs` walks `script/` and calls `engine.compile_file()` on every `.rhai`, with a heuristic that distinguishes true parse errors from missing-symbol errors (bare `Engine::new()` doesn't have recon bindings registered, so "identifier not bound" errors are benign at parse time). A second test verifies `script/README.md` contains a reference to every `.rhai` file — catches README drift when new examples are added.

**Docs sweep.** `TOPIC_SCRIPT` help examples now point at the shipped `script/` directory (ls, run, copy-to-global idioms). `recon --examples` SCRIPTING section gains a "Browse per-module example scripts" block. Both place `script/` as the first thing a user reaches for when wondering "how do I use X from a script".

**Tests added: +2.** 968 → 970 total. (The 21 new scripts don't count as individual tests; the parse-validation test covers them collectively.)

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

### 78. Interactive REPL (0.82.0)

Added a Rhai REPL mode (`recon --repl`) that reuses the existing
script engine. Design decisions worth recording:

**Persistent engine + persistent scope.** Each line compiles into its
own AST; `let` bindings persist via the shared `Scope`, and
user-defined `fn`s persist via an explicit `Vec<AST>` that's
re-merged before each eval. Mirrors Rhai's own example repl.

**Colon meta-command prefix.** Considered bare-words (`help`,
`quit`) but they'd collide with valid Rhai identifiers. Considered
`.cmd` (sqlite3 style) but `:cmd` is the convention in psql, ghci,
lldb, and Rhai's example. The prefix is unambiguous regardless of
what the user `let`s.

**Threading deferred.** `thread_spawn` needs a `Shared<AST>` handle
to dispatch into. In REPL mode there's no static AST. v1 stubs
`thread_spawn` with `"not available in REPL mode"`; future work could
either accumulate every line's AST (with last-wins on duplicate fn
names — needs care) or refactor spawn to take the AST at call time.

**rustyline over reedline / hand-rolled.** rustyline is the de-facto
Rust readline; reedline (nushell's) is bigger; hand-rolled would
mean no up-arrow / Ctrl-R / history, which defeats the purpose of a
REPL.

**Flag-mutation via shadowing.** `:set` updates `state.defaults` and
rebuilds the `flags` scope binding. Rhai's `Scope::set_value` panics
on constants, so the implementation pushes a new `flags` constant
that shadows the previous one (scope lookup walks newest-first). One
extra entry per `:set` is negligible for REPL session length.

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
