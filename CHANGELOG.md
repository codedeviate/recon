# Changelog

All notable changes to recon are recorded here. Format based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

For pre-0.4.1 design context and architectural notes, see [HISTORY.md](HISTORY.md).

## [Unreleased]

## [0.80.4] - 2026-05-17

### Added

- Four focused `ai::*` demo scripts under `script/`, complementing
  the existing kitchen-sink `ai.rhai`. Each is pinned to
  `claude` + `sonnet` so it runs out of the box without
  `RECON_AI_*` env / config, and asks for a one-to-three-word
  answer to keep token cost negligible.
  - `ai-simple.rhai` — smallest possible builder use.
  - `ai-system.rhai` — same prompt with two different system
    prompts to show how `.system()` steers tone / length.
  - `ai-context.rhai` — accumulating `.context()` blocks; classify
    a synthetic HTTP response.
  - `ai-multiturn.rhai` — manual multi-turn replay using
    `.assistant()` + `.user()`.

  All four parse against the registered Rhai engine
  (`tests/script_examples_it.rs`); none actually invoke a real
  CLI without an installed backend.

## [0.80.3] - 2026-05-17

### Changed

- Removed stale `v0.78` version targets from the `--ja3` / `--ja4` /
  `--http2-fingerprint` deferred-feature documentation. The original
  0.77.0 plan promised these in v0.78, but 0.78–0.80 shipped without
  them and the version reference outlived its accuracy. Updated:
  - `OUT-OF-SCOPE.md` section heading and body — now reads
    "(since 0.77.0)" with a parenthetical note explaining why the
    target was dropped.
  - `src/impersonate.rs` runtime error — now reads "not implemented
    yet" with a pointer to OUT-OF-SCOPE.md rather than "tracked for
    v0.78".
  - `src/help.rs` `--help impersonate` topic — three FlagHelp entries
    + the V1 SCOPE paragraph.
  - `src/examples.rs` impersonate examples + note.
  - `docs/MANUAL.md` two tables (Part II flag table, Part III script
    binding options).
  - `tests/impersonate_it.rs` test renamed and re-asserted against
    the new version-agnostic message.

  No behaviour change. Found during the 0.80.x OUT-OF-SCOPE.md audit.

## [0.80.2] - 2026-05-16

### Changed

- Two CLAUDE.md exposure-policy gaps from 0.80.0 closed:
  - `script/README.md` AI-agent-CLIs table now lists `copilot`
    alongside `claude / codex / gemini`.
  - `HISTORY.md` numbered entry #74 added for the Copilot backend.
    Documents the design rationale that the CHANGELOG only summarised
    — specifically the `copilot` (standalone) vs `gh copilot`
    (deprecated extension) split, the choice of stdin + `-s --no-color`
    invocation, and the inlined system-prompt delivery.

## [0.80.1] - 2026-05-16

### Changed

- Codebase-wide clippy sweep — 46 → 0 warnings. Mostly mechanical
  idiom upgrades: `strip_prefix` instead of slicing-after-starts_with,
  `rem_euclid` instead of manual modulo, `checked_div`, `split_once`,
  `!RangeInclusive::contains`, `is_empty()` for length-zero checks,
  `sort_by_key`, `if let` for unwrap-after-is_some / single-arm match,
  `vec![...]` literal instead of push-after-new, `#[derive(Default)]`
  on `QrLevel`, needless-borrow and needless-question-mark cleanups.
  Two type aliases (`MailboxStats`, `ParsedUrl`) factored out of
  `pop3_probe` to address `type_complexity`; the long parameter list
  on `run_session_*` is now explicitly annotated with
  `#[allow(clippy::too_many_arguments)]` since a parameter struct
  would obscure the call sites. The `HasherKind` `large_enum_variant`
  warning is similarly silenced with a comment — boxing the large
  variants would add per-update allocations on the streaming hot path.
- Bumped `RELEASE_DATE` to 2026-05-16 (matches `--version` banner to
  the actual ship date).

## [0.80.0] - 2026-05-15

### Added

- `ai::*` — new built-in `copilot` backend for the GitHub Copilot CLI
  (the standalone agentic CLI, GA October 2025 — distinct from the now-
  deprecated `gh copilot` extension). Invocation: `copilot -s --no-color
  [--model M]` with the prompt piped on stdin; `-s` strips session
  metadata, `--no-color` keeps stdout machine-parseable. System prompts
  are inlined into the body (the standalone CLI has no
  `--system-prompt` flag). Auth uses `GH_TOKEN` / `GITHUB_TOKEN` or the
  CLI's own `/login`. Documented model values include `auto` (the
  default), `gpt-5.3-codex`, `claude-sonnet-4.6`, `claude-haiku-4.5`.

  The deprecated `gh copilot` extension is intentionally not wired up:
  its archived upstream and TUI-only output make it unsuitable for the
  subprocess pattern (it can only suggest shell commands, not return
  free-form chat).

## [0.79.0] - 2026-05-15

### Added

- `ai::*` Rhai script-engine bindings — a small builder API that lets
  scripts dispatch a prompt to one of several agent CLIs.
  - `ai::ask(prompt)` one-liner; `ai::request()` builder with
    `.backend`, `.model`, `.system`, `.context` (accumulating),
    `.prompt` / `.user`, `.assistant` (multi-turn replay),
    `.max_tokens`, `.temperature`, `.timeout`, `.send`, `.send_full`.
  - Subprocess backends in v1: `claude` (`claude -p`), `codex`
    (`codex exec`), `gemini` (`gemini --prompt`), and `cmd`
    (user-defined via `[ai.backends.<name>]` in
    `~/.recon/config.toml`).
  - Three-layer config: per-request → env (`RECON_AI_BACKEND`,
    `RECON_AI_MODEL`, `RECON_AI_TIMEOUT`) → `[ai]` config section.
    No PATH fallback.
  - All failures throw a Rhai script error prefixed `ai:` so callers
    can `try { … } catch (e) { … }` cleanly.
  - HTTP backends (Anthropic / OpenAI / Ollama), streaming,
    tool-calling, and per-script token budgets are intentionally
    deferred; the builder is forward-compatible.

## [0.78.2] - 2026-05-15

### Fixed

- Build warnings — three lingering compiler diagnostics resolved:
  - `src/docs.rs:200` `comrak_options(opts: &DocOptions) -> Options`
    triggered `mismatched_lifetime_syntaxes` because the elided
    lifetime in the parameter is also present in the return type.
    Made it explicit: `Options<'_>`.
  - `src/client.rs::snapshot_response_for_impersonate` was a
    `pub(crate)` wrapper called only from `src/impersonate.rs`, which
    is itself behind `#[cfg(feature = "impersonate")]`. Without the
    feature the function was unused and triggered `dead_code`. Gated
    with the same cfg.
  - `src/ssh_auth.rs::verify_host_key` was a thin wrapper around
    `verify_host_key_with_pins(.., None, None)` with zero call sites
    anywhere in the tree. Deleted; the with-pins variant is the only
    caller already in use.

  Default release build is now warning-free; the `--features
  impersonate` build remains warning-free as well.

## [0.78.1] - 2026-05-15

### Changed

- `OUT-OF-SCOPE.md` — bucket discipline sweep. ~20 items that were
  sitting under **Waiting** were actually upstream-blocked or had
  internal scope trade-offs, contradicting the bucket's own
  definition ("can be done, not asked for"). Moved them to
  **Not yet supported** (true upstream blocks: FTP `--ftp-account` /
  `--ftp-ssl-control` family, SMTP `--mail-rcpt-allowfails` /
  `--sasl-ir`, IMAP `--login-options` / `--sasl-authzid`, Telnet
  `--telnet-option`, `--append`, the proxy + TLS-tuning per-flag
  stubs, `--tr-encoding`, "Other markup → PDF") or **Deferred**
  (internal trade-offs with no upstream block: `--suppress-connect-headers`,
  `--path-as-is`, `--proxy-pass`). The Waiting list now correctly
  contains only the two items it should: tax-ID coverage gaps and
  PNG HRT. Added a "Bucket discipline" note to the Process section
  to prevent future drift. The 0.66.2 sweep was the previous
  canonical clean; this is its 0.78.1 follow-up.

## [0.78.0] - 2026-05-15

### Added

- `--encode-hints KEY=VAL` (repeatable) — pass per-call hints through to
  rxing's `encode_with_hints` for the Aztec and PDF417 encoders.
  Supported keys: `charset` (CharacterSet / ECI), `eclevel`
  (ErrorCorrection — Aztec % EC words, PDF417 `0..8`), `aztec-layers`
  (`-4..-1` compact, `0` auto, `1..32` full), `pdf417-compact`,
  `pdf417-compaction`, `pdf417-auto-eci`, `margin`. Unknown keys and
  hints on non-rxing formats (qr / datamatrix / code128 / code39 /
  ean13 / upca) error so typos fail loud rather than silently
  no-op. Retires the matching "Waiting" entry in OUT-OF-SCOPE.md.

## [0.77.14] - 2026-05-15

### Fixed

- `--editor URL` (space-separated) — clap's `num_args = 0..=1` on
  `--editor` greedily eats the next token, so `recon --editor
  https://example.com` was landing the URL on `--editor` and erroring
  out for missing input. Rescue: after argv parse, if `args.editor`
  contains `://` and no positional URL is set, swap the value onto
  `args.url` and let `--editor` fall back to the configured default.
  The documented workarounds (`--editor=value`, or `--url` first)
  still work; this just removes the surprise. OUT-OF-SCOPE.md entry
  retired.

## [0.77.13] - 2026-05-08

### Changed

- `Cargo.toml` exclude list — final pre-publish tightening. Added
  `.git/` and `.worktrees/` (defense-in-depth against accidental
  inclusion if cargo's git-aware filtering is ever bypassed),
  `.idea/` and `.claude/` (IDE / agent dirs — already gitignored,
  belt-and-suspenders), `HISTORY.md` (172 KB internal design-rationale
  doc, was tracked despite being gitignored, was leaking into the
  tarball), `OUT-OF-SCOPE.md` (25 KB internal scope notes, also
  leaking). Removed `BREW.md` (redundant — gitignored and never
  tracked, so cargo never packaged it). Normalised `.gitignore`
  spelling. Verified end-to-end with `cargo publish --dry-run`:
  303 files, 706 KiB compressed, server-side validation green.

## [0.77.12] - 2026-05-07

### Changed

- `Cargo.toml` — added an `exclude = [...]` list so `cargo publish`
  ships a smaller, source-only crate to crates.io. Excluded:
  `dump.rdb` (Redis runtime state, also gitignored), `target/` (build
  output, redundant but explicit), `docs/MANUAL.pdf` (2 MB binary
  artifact regenerable from `MANUAL.md`), `.github/` (CI config not
  useful to consumers), `/.gitignore`, `CLAUDE.md` (development
  instructions), `BREW.md` (homebrew tap notes, also gitignored).
  Aligns with a new sibling repo that orchestrates homebrew + crates.io
  release publishing for this project — keeps the published crate
  payload focused on what `cargo install recon-cli` actually needs to
  build.

## [0.77.11] - 2026-05-07

### Changed

- `script/browser-iso8859.rhai` — rewrote the demo so it actually
  proves the outbound transcode rather than just hoping the user
  notices it happened. Now compares UTF-8 vs Latin-1 byte counts via
  `text::encode(body, charset).len()`, asserts the echoed
  `Content-Length` equals the Latin-1 length (24 vs 26 — the strong
  proof recon transcoded), checks for httpbin's `�`-mangling of
  the Latin-1 bytes when it forced UTF-8 decoding (orthogonal proof
  the bytes weren't UTF-8), and finishes with a server-independent
  `text::encode → text::decode` round-trip. Added a long header
  comment explaining the test methodology — particularly the
  counter-intuitive bit that httpbin's mangling is itself the
  evidence the transcode worked.

## [0.77.10] - 2026-05-07

### Changed

- Untracked `dump.rdb` from the repo and added it to `.gitignore`.
  The file was a 121-byte Redis RDB snapshot committed in 0.24.13
  that kept getting clobbered locally whenever `redis-server` ran
  from the project root (Redis defaults to `dir ./` +
  `dbfilename dump.rdb`), so it surfaced in every `git status` as
  modified-but-bytewise-identical noise. Runtime state, not source.

## [0.77.9] - 2026-05-07

### Fixed

- `src/decode.rs` — `decode_bytes` / `decode_all_bytes` wrote the
  in-memory image to a tempfile with a `.img` suffix, then handed the
  path to rxing, whose underlying `image` crate decoder picks the
  format from the file extension. `.img` isn't recognized, so every
  decode failed with a misleading "file not found or cannot be
  opened". Now we sniff PNG / JPEG / GIF / WebP / BMP / TIFF magic
  bytes and use a real extension (defaulting to `.png` when no header
  matches — most callers of `encode::decode` are round-tripping a
  `encode::qr(...)` blob anyway).
- `src/rtsp_probe.rs::parse_url` — URLs of the shape
  `rtsp://user:pass@host:port/path` were being passed to DNS as the
  literal string `user:pass@host:port`, because `rsplit_once(':')` on
  the whole authority split on the userinfo's colon, not the
  host:port colon. The parser now strips `user[:pass]@` first
  (captured into a `userinfo` field for future RTSP auth wiring).
  Same patch adds IPv6 bracket support: `[::1]`, `[fe80::1]:8554`,
  `rtsps://demo:pw@[2001:db8::1]:443/` all parse cleanly. Five new
  unit tests cover the new shapes.
- `script/*.rhai` (ftp, gopher, imap, memcached, mqtt, pop3, redis,
  rtsp, sftp, smtp) — `tcp(...)` raises a Rhai exception on connect
  failure / timeout, so the long-standing `if !t.ok { skip }` guards
  in every demo were dead code. Demos crashed with stack traces
  whenever the public test server was down. Each guard is now wrapped
  in try/catch with a 5-second probe timeout (2 seconds for
  loopback-only memcached/redis), so the demo prints a clean
  "<host> unreachable — skipping" and exits 2.
- `script/rtsp.rhai` — header now lists five public RTSP demo
  endpoints (Wowza, rtsp.stream pattern + movie, zephyr.rtsp.stream
  with stream key, IPVM ONVIF) so users have somewhere to go when
  the default Wowza endpoint is down (which it is at the time of
  writing). Default behavior unchanged.
- `script/netrc.rhai` — the `netrc_optional: true` demo printed
  `optional netrc: 401` against httpbin and looked broken to anyone
  without an `httpbin.org` entry in `~/.netrc` (the expected case).
  Demo now annotates the 401 as expected, then synthesizes a netrc
  in `/tmp` and exercises the success path via `netrc_file=`,
  proving both halves of the feature in one run. Cleans the temp
  file at the end.
- `script/doc-convert.rhai` — second PDF was written to
  `<out>.pdf.2`, which Finder/macOS can't double-click open
  (no `.pdf` extension at the tail). Now writes to `<out>-2.pdf`.

## [0.77.8] - 2026-05-06

### Fixed

- `script/agent-browser-find.rhai` — replaced bare
  `agentBrowser::back()` with a resilient `open(url)` and wrapped the
  final `close()` in try/catch, so steps that follow a navigation-
  inducing click no longer throw "Inspected target navigated or
  closed". Each per-locator failure is still caught individually as
  intended.
- `script/agent-browser-interaction.rhai` — same `back()` → `open(url)`
  pattern, plus try/catch around `keyboard_type`, `keyboard_insert`,
  `scroll`, `scrollintoview`, `wait`, and `close`. Heavy SPAs
  (e.g. major news sites) trigger mid-action navigation that puts the
  inspected target in a transient bad state; previously a single
  bare call after the first click would abort the whole demo.
- `script/agent-browser-pdf.rhai` — the second PDF used to come out
  empty when the user passed any of the example invocations.
  `agent-browser pdf <path>` only renders the *current* page, so
  `--user-agent` and `--args` are launch-time options that belong on
  `open()`, not on `pdf()`. Putting them on `pdf()` made agent-browser
  start a fresh blank session with the requested args and render
  that — hence the empty PDF. Demo now passes the launch options to
  `open(url, opts)` and follows with a plain `pdf(path)`. Both PDFs
  now contain real rendered content.
- `script/agent-browser-screenshot.rhai` — `out.trim()` returned `()`
  (Rhai's String::trim is mutating). Output became literally `()`
  instead of the agent-browser stdout. Copy-then-trim-in-place pattern
  applied; `close()` also wrapped in try/catch.
- `script/batch-spider.rhai` — two bugs:
  - `let url = raw.trim();` assigned `()` to `url` because `trim()`
    is mutating, so `url.len()` errored with `Function not found:
    len (())`.
  - `file_read(path).to_string()` returned the hex-debug view of the
    Blob (`[68747470733a...]`), not the UTF-8 decoded text. The for
    loop iterated once, treating the entire file-as-hex as a single
    URL and dispatching a bogus request. Switched to
    `text::decode(file_read(path), "utf-8")` for the real text body.
- `script/doc-convert.rhai` — same `file_read(...).to_string()`
  Blob-debug bug. Was silently generating 354 KB of HTML containing
  hex characters from `CHANGELOG.md`, with a 519 KB mostly-empty PDF.
  After the fix: 271 KB of properly-rendered markdown HTML and a
  3.3 MB PDF with real content. The bug didn't show up in the
  earlier sweep because the script's exit code stayed 0.
- `script/pop3.rhai` — `r.banner.trim()` had the same mutating-trim
  problem, printing `()` instead of the banner text. Same
  copy-then-trim-in-place pattern applied.

## [0.77.7] - 2026-05-06

### Fixed

- `script/agent-browser-cmd.rhai` failed at the cookies-set step with
  `agent-browser: exit 1: ✗ CDP error (Network.setCookies): Invalid
  cookie fields`. The script was passing `"session=abc123"` as a single
  argv element, but agent-browser's `cookies set` subcommand expects
  name and value as two separate positional args (`set <name> <value>
  [options]`). Split into `"session", "abc123"`. Verified end-to-end
  against a real URL: cookies get + set, storage, tabs, network
  capture, console, errors, mouse, snapshot diff, and close all
  succeed; script exits 0.

## [0.77.6] - 2026-05-06

### Fixed

- `recon --script -` now reads the script body from stdin, matching the
  heredoc example documented in `docs/MANUAL.md` (under "Proxy
  routing" → "Script equivalent"). Previously the manual example
  failed with `error: could not find script '-'` because the resolver
  only treated `-` as a literal path. Implementation factors the
  source-loading half of `engine::run_file` into a new public
  `engine::run_source(source, source_path, source_dir, source_name,
  args)` that the stdin code path can call directly without a
  temporary file. `script_path` is set to `<stdin>`, `script_dir` to
  the current working directory, and `script_name` to `stdin` so
  scripts that reference these constants behave predictably.
  Doc-comment on `--script` updated to mention `-`.

## [0.77.5] - 2026-05-06

### Fixed

- Sweep of `script/*.rhai`: every script in the gallery now runs
  without Rhai static-shape errors. Three classes of bug were fixed
  across 12 scripts:
  - **`for (val, key) in <map>` destructuring iteration** (left over
    from an older Rhai version) failed with "For loop expects iterable
    type" in `email.rhai` and `ldap.rhai`. Rewrote to walk
    `map.keys()` and index for the value.
  - **Chained mutating string methods** — `String::replace` and
    `String::trim` are non-pure in this Rhai build (they mutate in
    place and return `()`), so chains like
    `url.replace("ftp://", "").replace("ftps://", "").split("/").shift()`
    blew up on the second call. Replaced the host-port extraction
    in `ftp.rhai`, `gopher.rhai`, `imap.rhai`, `pop3.rhai`,
    `smtp.rhai`, `sftp.rhai`, `tftp.rhai`, and `clipboard.rhai`
    with a `sub_string` + `index_of` based extraction that doesn't
    rely on chaining.
  - **Map `.contains_key()` not registered** — used the `in`
    operator instead in `email.rhai` and `checkdigit.rhai`.
  - **Array `.join()` not registered in this Rhai build** — replaced
    with manual string concatenation in `imap.rhai`.
- Demo robustness: `client-cert.rhai` now exits cleanly with a hint
  when the cert path doesn't exist instead of erroring out;
  `time-cond.rhai` only attempts the second (etag-compare) fetch
  when the cache file actually exists; `tcp-echo.rhai` exits 2
  cleanly when the listen port is already in use; `udp-listen.rhai`
  treats EAGAIN ("Resource temporarily unavailable") the same way
  it already treats timeouts; `impersonate.rhai` wraps the
  feature-gated call in a try/catch so the default rustls build
  exits 2 with a rebuild hint instead of throwing.

## [0.77.4] - 2026-05-06

### Fixed

- `script/dns.rhai` failed at runtime with two distinct errors:
  - `Non-pure method 'split' cannot be called on constant (line 8)` —
    `args` is pushed into Rhai's Scope as a constant (see HISTORY entry
    71), so non-pure methods like `split` can't be called directly on
    `args[N]`. Fixed by binding `args[2]` to a local `let raw = args[2]`
    first, then splitting the local.
  - `For loop expects iterable type (line 18)` — the previous
    `for (records, typ) in r.records` destructuring iteration over a
    Map isn't enabled in recon's Rhai build. Rewrote the loop to walk
    `r.records.keys()` and index back into the map for the values.
  Both fixes verified against the three documented invocations
  (`dns example.com`, `dns example.com A,MX`, and the default).

## [0.77.3] - 2026-05-05

### Added

- Full crates.io package metadata in `Cargo.toml`: `description`,
  `authors = ["Thomas Björk <codedv8@gmail.com>"]`, `homepage`,
  `repository`, `documentation`, `readme`, `keywords`, `categories`,
  `rust-version = "1.85"`, plus an `exclude` list that keeps the
  published tarball lean (no `dump.rdb`, `target/`, `docs/MANUAL.pdf`,
  `homebrew/`, or `.github/`).
- Homebrew formula files under `homebrew/Formula/` for the user's
  tap (`codedeviate/homebrew-recon`):
  - `recon.rb` — default rustls build.
  - `recon-impersonate.rb` — feature-on build, `depends_on "cmake"`
    for BoringSSL, conflicts with `recon` because both install the
    same binary name.
  - `homebrew/README.md` documents the tap layout, SHA256 fill-in,
    and per-release update workflow.
- `README.md` install section split into Homebrew, crates.io, and
  from-source paths.

### Changed

- **Crate name on crates.io is `recon-cli`** (the bare `recon` name
  has been parked since 2019). The binary stays `recon`, so
  `cargo install recon-cli` produces an executable invoked as `recon`.
  Documented inline in `Cargo.toml` and in the README install section.
- Repository URL swept from `thomas-starweb/recon` to
  `codedeviate/recon` in `README.md` and `docs/MANUAL.md`.
- Bumped MSRV declaration to 1.85 (driven by `rquest`, `rquest-util`,
  and `clap` — the highest `rust-version` in the dep tree). The MSRV
  applies regardless of feature flags; the `impersonate` feature does
  not raise it further.

## [0.77.2] - 2026-05-05

### Added

- `LICENSE` file at the repo root with the MIT license text (copyright
  2026 Thomas Björk). The manual and README have referenced MIT since
  the project's start; this commit makes the licence explicit and
  machine-readable for crates.io / GitHub.
- `license = "MIT"` field in `[package]` of `Cargo.toml` so
  cargo-published metadata, GitHub, and crates.io can recognise the
  licence without inspecting the LICENSE file.

## [0.77.1] - 2026-05-05

### Added

- `README.md` — top-level project README with install, quick-start
  examples, feature highlights, browser-fingerprint impersonation
  callout, and pointers to the manual / changelog / history /
  out-of-scope / parity-matrix / script gallery.

### Changed

- `docs/curl-parity-matrix.md` — refreshed for 0.77.0. Last-sweep date
  bumped from 0.50.0 (2026-04-23) to 0.77.0 (2026-05-05). Added rows
  for HTTPS-proxy (called out separately from the generic Proxy row),
  HTTP3 (deferred — reqwest 0.12 H3 not yet stable), TLS-SRP (deferred
  — neither rustls nor BoringSSL exposes SRP), ECH (deferred —
  rustls 0.23 has unstable ECH only), IDN (deferred — url crate
  parses but no explicit Punycode CLI), and PSL (deferred — cookie
  jar doesn't yet consult the public-suffix list). Added a "Curl
  flags shipped in recon" subsection covering mTLS (0.54.0),
  CRL (0.72.0), --time-cond, --range, -C resume, --retry cluster
  (0.64.0), --netrc (0.63.0), --xattr, -K config files, multipart
  -F / form opts, --oauth2-bearer, --limit-rate, --interface, and
  --write-out. Added a "Beyond curl" subsection enumerating
  recon-only features (TLS impersonation, email-protection sweep,
  multi-protocol probes, document conversion, codecs, barcodes,
  JWT, Rhai script engine, sample-data, built-in serve, topic help,
  examples, flag listing).

## [0.77.0] - 2026-05-05

### Added

- New optional Cargo feature `impersonate` (off by default) that
  pulls in `rquest` (BoringSSL) plus `rquest-util` and enables
  browser TLS+H2 fingerprint impersonation. Build with
  `cargo build --features impersonate`; the default release artifact
  remains rustls-only.
- `--impersonate <PROFILE>` — the working v1 surface. Forwards to
  `rquest_util::Emulation` with profile names like `chrome_131`,
  `firefox_128`, `safari_17.5`, `edge_131`, `okhttp_5`,
  `chrome_android_131`, `safari_ios_17.4.1`. Hyphens are accepted
  as a convenience (`chrome-131` ≡ `chrome_131`). Roughly 50
  profiles available; see `recon --help impersonate` for the full
  list.
- `--ja3 <STRING>`, `--ja4 <STRING>`, `--http2-fingerprint <STRING>`
  — reserved in the CLI for forward-compatibility but error at
  runtime with a "deferred to v0.78" message. Use `--impersonate`
  for now (named profiles cover the captcha-testing use case).
- Matching script `http()` opts keys: `impersonate`, `ja3`, `ja4`,
  `http2_fingerprint`. Same v1 scope reduction applies to the
  three deferred keys.
- `recon --help impersonate` topic with aliases `ja3`, `ja4`,
  `fingerprint`, `tls-fingerprint`, `browser-fingerprint`,
  `http2-fingerprint`. Cross-referenced from `--help cert` and
  `--help protocols`.
- `--examples` — new "BROWSER FINGERPRINT IMPERSONATION" section
  with named-profile examples for Chrome / Firefox / Safari / Edge /
  mobile / OkHttp, fingerprint-verification against tls.peet.ws,
  and an honest note that the raw-override flags currently error.
- Demo script `script/impersonate.rhai` — minimal `http()` call
  with the `impersonate` opts key.

- `Makefile` — new convenience targets for the impersonate feature:
  `build-impersonate`, `release-impersonate`, `all-impersonate`,
  `check-impersonate`, `test-impersonate`, `run-impersonate`,
  `install-impersonate`, `ci-impersonate`. Existing targets accept
  `FEATURES="--features ..."` for ad-hoc combinations.

### Changed

- `client::execute` dispatches to a new `impersonate::execute`
  module at the top of the function when any of the four
  impersonation flags is set; the default rustls path is unchanged
  for all other invocations.
- `ssh2` dependency now uses `features = ["vendored-openssl"]` so
  `libssh2-sys` links its own statically-bundled OpenSSL instead of
  the system library. Necessary because BoringSSL (pulled in by
  rquest under `--features impersonate`) exports `libssl`/`libcrypto`
  but omits OpenSSL 3-only symbols that `libssh2-sys` needs at link
  time. Side-effect of the impersonation work that affects the
  default build too: small build-time cost (~5–10 seconds), no
  runtime difference. Documented as entry #72 in HISTORY.md.

## [0.76.2] - 2026-05-04

### Changed

- `HISTORY.md` — added entry #71 covering 0.76.1's
  `script_path` / `script_dir` / `script_name` constants. The 0.76.1
  release shipped without a HISTORY entry; this backfills the
  design rationale (why `script_name` is needed alongside `args[0]`,
  why `load_dotenv` itself wasn't changed to auto-resolve relative
  paths against `script_dir`, the args[0]-vs-script_name bug caught
  in smoke testing).
- `OUT-OF-SCOPE.md` — recorded two deliberate non-goals from the
  0.76.0 / 0.76.1 dotenv work that previously lived only in code
  comments and CHANGELOG prose: auto-loading `.env` at script start
  (Deferred — explicit-only by design, with rationale) and the
  edition-2024 `std::env::set_var` migration (Not yet supported —
  blocks on a recon edition bump from 2021 → 2024).

## [0.76.1] - 2026-05-04

### Added

- `script_path`, `script_dir`, and `script_name` — read-only String
  constants pushed into every Rhai script's Scope alongside `args`
  and `flags`. `script_path` is the resolved absolute path of the
  running script, `script_dir` is its parent directory, and
  `script_name` is the file stem (basename minus extension). The
  natural overlay idiom is now:

  ```rhai
  load_dotenv(script_dir + "/.env");                       // shared
  load_dotenv(script_dir + "/.env." + script_name);        // per-script
  ```

  Closes the gap between the 0.76.0 `load_dotenv` API and the original
  use case (multiple scripts in a directory sharing a `.env` plus
  per-script `.env.<scriptname>` overlays). `args[0]` is unsuitable
  for the overlay name because it's the as-typed value (the full path
  when the user runs `--script /tmp/x/demo.rhai`); `script_name`
  always reduces to just `demo`.

### Changed

- `script/dotenv.rhai` — rewritten to demonstrate the directory-
  overlay pattern using `script_dir`. The 0.76.0 version wrote its
  own tempfiles under `/tmp` and loaded them by absolute path, which
  showed the API but not the workflow. The new demo expects sibling
  `.env` and `.env.<args[0]>` files next to the script and degrades
  gracefully with `try { ... } catch { ... }` when they don't exist.
- `recon --help script` and `recon --examples` updated with
  `script_path` / `script_dir` entries and a sibling-`.env` example.
- `docs/MANUAL.md` Part III env section gained rows for the two new
  constants and the canonical overlay snippet now uses
  `script_dir + "/.env"` instead of hardcoded `/etc/myapp` paths.

## [0.76.0] - 2026-05-04

### Added

- Script engine — `.env` loading and full-environment snapshot. Three
  new bindings in `src/script/bindings/helpers.rs`:
  - `load_dotenv(path) -> int` parses a `.env` file and sets each
    `KEY=VALUE` in the process environment, **overriding** existing
    values. Returns the count of vars set. Aliased as `loadDotEnv`
    for camelCase callers.
  - `load_dotenv(path, override) -> int` — explicit two-arg form.
    Pass `false` to leave pre-existing env (e.g. shell exports) in
    place; returns 0 for any key already set.
  - `env_all() -> Map` snapshots the entire process environment as a
    Rhai map. Aliased as `envAll`.
  Default-override semantics make the layered pattern work:
  `load_dotenv(".env"); load_dotenv(".env.<scriptname>")` — the
  second load wins, mirroring how shell scripts source common
  defaults followed by per-script overrides.
- New shipped example: `script/dotenv.rhai` demonstrates the layered
  workflow end-to-end (writes two .env files, loads them, prints
  resolved values, exercises camelCase aliases).
- New dependency: `dotenvy = "0.15"` (maintained successor to the
  unmaintained `dotenv` crate). Used solely for `.env` parsing.

## [0.75.2] - 2026-05-02

### Fixed

- `docs/MANUAL.md` — Part III "agent-browser bindings" function table
  had stale signatures dating back to the binding's first cut:
  - `find(selector)` → corrected to `find(locator, value, action)` /
    `find(locator, value, action, text)` (semantic locators, not CSS).
  - `get(selector)` → corrected to `get(what)` / `get(what, sel)` with
    the full `what` enum (`text` / `html` / `value` / `attr <name>` /
    `title` / `url` / `count` / `box` / `styles` / `cdp-url`).
  - `type(selector, text)` → `type_text(selector, text)` (renamed
    because `type` is reserved in Rhai).
  - `eval(js)` → `eval_js(js)` / `eval_js(js, opts)` (the 0.75.1 alias
    that's actually callable from script position).
  - `snapshot()` row expanded to show all 4 overloads.
  - Added rows for `back` / `forward` / `reload`, `dblclick`, `hover`,
    `focus`, `check`, `uncheck`, `keyboard_insert`, `scroll`,
    `scrollintoview`, `wait`, `is_visible` / `is_enabled` /
    `is_checked`, and per-call `opts` on the launch verbs.

### Added

- New "Existence check" subsection in the manual showing the
  `agentBrowser::get("count", sel).count > 0` pattern. The predicate
  functions (`is_visible` etc.) raise an error when no element matches;
  `get("count", ...)` is the no-raise existence check.
- Cross-reference from the manual to the six demo scripts shipped in
  0.75.1 so users land on a working example for each part of the
  binding surface.

## [0.75.1] - 2026-05-02

### Added

- Six new agent-browser demo scripts covering the full binding surface:
  `agent-browser-find.rhai` (semantic locators: role / text / label /
  placeholder / alt / title / testid / first / last / nth),
  `agent-browser-interaction.rhai` (click, dblclick, hover, focus, check,
  uncheck, fill, type_text, press, scroll, scrollintoview, keyboard
  primitives), `agent-browser-inspect.rhai` (snapshot, eval_js, get,
  is_visible / is_enabled / is_checked, wait), `agent-browser-navigation.rhai`
  (open / back / forward / reload / close / close_all),
  `agent-browser-pdf.rhai` (PDF rendering with per-call options),
  `agent-browser-cmd.rhai` (cmd() escape hatch for cookies / storage /
  tabs / network / console).

### Fixed

- `agentBrowser::eval` was unparseable from Rhai scripts because Rhai's
  parser reserves `eval` as a keyword even in module-namespaced position.
  The typed binding has been kept (Rust-side callers are unaffected) and
  a parallel `eval_js` alias is now registered for script use. Both
  the 1-arg `eval_js(js)` and 2-arg `eval_js(js, opts)` overloads are
  available.

## [0.75.0] - 2026-05-01

### Added

- `agentBrowser::set_default_options(opts)` / `default_options()` /
  `clear_default_options()` — module-level defaults state in the
  agent-browser script bindings. All 25 of agent-browser's global
  launch / security / session options are exposed as snake_case
  opts-map keys (`ignore_https_errors`, `user_agent`, `proxy`,
  `headers`, `profile`, `extension`, `browser_args`, etc.). Defaults
  apply to every binding call automatically.
- Per-call opts overloads on `agentBrowser::open`, `screenshot`,
  `snapshot`, `pdf`, `eval`. Per-call opts concatenate after defaults
  so they override at the agent-browser flag-parser level.
- `headers` opts-map key accepts either a JSON string or a Rhai map
  (auto-serialized via serde_json).
- `extension` and `browser_args` accept either a single string or a
  Rhai array of strings (each emits its own `--extension` /
  `--args` flag).

### Changed

- `src/agent_browser.rs` — added `run_cmd_with_options(opts, args, json)`
  helper. Original `run_cmd` unchanged for backwards compatibility.

## [0.74.0] - 2026-05-01

### Added

- `--doc-author <STR>`, `--doc-subject <STR>`, `--doc-keywords <STR>` —
  populate PDF document metadata for `--md-to-pdf` / `--html-to-pdf`.
  Implemented via post-generation binary patch of the PDF Info dictionary
  (Chrome's printToPDF does not read `<meta>` tags for author/subject/keywords).
  HTML `<meta name="author|description|keywords">` tags are also injected
  in the generated HTML for maximum compatibility.
  Verifiable via `pdfinfo <output>.pdf`.

### Changed

- The recon manual PDF (`docs/MANUAL.pdf`) is now regenerated with all
  four `--doc-*` metadata fields populated, dogfooding the new flags.
- `OUT-OF-SCOPE.md`: removed "PDF metadata beyond title" from the
  Document conversions Waiting list. Other-markup → PDF (reST, AsciiDoc,
  Org) remains deferred — no production-ready pure-Rust parsers for those
  formats as of mid-2025.

## [0.73.0] - 2026-05-01

### Added

- `--remote-name-all` — apply `-O` (filename from URL) to every URL
  processed via `--input-file`. Curl-parity for multi-URL invocations.
- `-#, --progress-bar` — alternate indicatif progress-bar style using
  `#` characters. Curl-parity. Also activates the progress bar (no
  separate `--progress` needed). Works with `-o` / `-O` file saves.
- `--proxy-pass <PASS>` — passphrase for `--proxy-key` when the HTTPS
  proxy's private key is encrypted. Accepted for curl parity; proxy
  mTLS passphrase support is not exposed by reqwest 0.12 — the flag
  emits a runtime warning and has no effect. Deferred.

### Changed

- `OUT-OF-SCOPE.md`: removed shipped items from "curl flags — leftover
  after the 0.61.0–0.66.0 Waiting-arc". Sharpened deferral notes on
  `--suppress-connect-headers` (architectural), `--path-as-is`
  (reqwest::Url normalises), `--tr-encoding` (reqwest no opt-out), and
  FTP gap flags (blocked on suppaftp 6).

## [0.72.0] - 2026-05-01

### Added

- `--crlfile <PATH>` — load PEM-encoded X.509 CRLs and pass to
  reqwest's `add_crls`. Server certs found in any loaded CRL are
  rejected during the TLS handshake. Multi-CRL bundles supported.
- `--proxy-capath <DIR>` — directory walker that adds `.pem`/`.crt`/
  `.cer` files as root certificates for proxy TLS verification.
  Mirrors `--capath`.
- `--proxy-ca-native` — disables built-in webpki roots so only the
  OS native roots are trusted. Same global toggle as `--ca-native`;
  the separate flag exists for curl-parity.

### Changed

- `OUT-OF-SCOPE.md`: removed shipped items from "Per-flag plumb-through
  (0.66.0 stubs → real)". Sharpened deferral notes on `--pinnedpubkey`
  and `--curves` (require migrating 8 existing TLS flags onto a custom
  `rustls::ClientConfig` via `use_preconfigured_tls` — out of scope for
  a single-flag plumb-through release; tracked as its own future
  effort). `--ciphers`, `--tls13-ciphers`, and proxy-side cipher /
  pinning / CRL flags remain blocked on rustls 0.23 / reqwest 0.12
  not exposing the necessary primitives.

## [0.71.0] - 2026-05-01

### Added

- FTP: `--list-only` swaps `LIST` → `NLST` for filenames-only listing.
- FTP: `-Q / --quote <CMD>` runs an arbitrary FTP verb before the
  listing step, via `suppaftp::FtpStream::custom_command`. Repeatable.
- FTP: `--ftp-skip-pasv-ip` calls `set_passive_nat_workaround(true)`,
  replacing the server-advertised PASV IP with the control-channel
  peer IP (matches curl's behaviour).
- FTP: `--disable-epsv`, `--disable-eprt`, `--ftp-pasv` emit a
  verbose-mode confirmation when set (suppaftp 6 is passive-only by
  default; these are explicit-assertion aliases).
- TFTP: `--tftp-no-options` emits a verbose-mode confirmation
  (recon's TFTP probe is already vanilla RFC 1350; no RFC 2347
  options have ever been sent).
- SSH: `--pubkey <PATH>` now plumbs through as an alias for
  `--ssh-pubkey`. When both are set, `--ssh-pubkey` wins.
- Script bindings: `ftp(url, opts)` gained `list_only`, `quote`,
  `ftp_skip_pasv_ip`, `disable_epsv`, `disable_eprt`, `ftp_pasv`
  opts-map keys for parity with the new CLI flags.

### Changed

- SMTP `--mail-auth <ADDR>`: now emits a clear runtime warning
  explaining that lettre 0.11's high-level `SmtpTransport::send`
  API does not expose envelope parameters. The flag is accepted
  but not forwarded. Moved to `OUT-OF-SCOPE.md` Deferred bucket.
- `OUT-OF-SCOPE.md`: removed shipped items from "Per-protocol
  plumb-through (0.65.0 stubs → real)". Sharpened deferral notes
  on `--sasl-ir`, `--mail-rcpt-allowfails`, `--ftp-method`,
  `--ftp-account`, `--login-options`, `--sasl-authzid`,
  `--telnet-option`, `-a/--append`, `--ftp-create-dirs` with the
  specific upstream-crate API gap that blocks each.

## [0.70.0] - 2026-04-30

### Added

- `--from-clipboard` flag — read body from the system clipboard. Mutex
  with `--stdin` and a URL. Backed by the cross-platform `arboard`
  crate (macOS pasteboard, Linux X11/Wayland, Windows).
- `--to-clipboard` flag — write output to the system clipboard. Mutex
  with `-o` and `--editor`. UTF-8 text only.
- `--clipboard [<DIR>]` flag — `DIR` = `in` / `out` / `both`. Bare
  `--clipboard` (no value) auto-resolves direction from context: `out`
  when an input source is already given, `in` otherwise.
- `clipboard::get()` and `clipboard::set(text)` Rhai script bindings —
  same primitive that powers the CLI flags, available to scripts.
- New dep: `arboard` 3.x with `wayland-data-control` feature.

### Changed

- `--stdin` mode now honours `--editor` (closes 0.69.0 deferred gap).
  `--editor -vv` still mirrors body to stdout as documented.
- `src/output.rs` — replaced `(final_path, sink_writer)` parameter pair
  in `write_processed_body` with a single `sink: BodySink` enum
  (`Writer` / `File` / `Editor` / `Clipboard`). HTTP and stdin paths
  share the same dispatch. Removed the duplicate `run_with_editor`
  function in `src/main.rs`.
- Auto-detect stdin: when no URL or input flag is given and stdin is
  not a TTY, recon treats it as if `--stdin` was passed. Interactive
  invocation (TTY stdin) without a URL still produces a usage error.
- `load_editor_config` moved from `src/main.rs` to `src/editor.rs`
  (single source of truth — used by both the HTTP path and the new
  BodySink::Editor dispatch in output.rs).

### Removed

- `OUT-OF-SCOPE.md` entry deferring `--editor` with `--stdin` —
  shipped in this release.

## [0.69.0] - 2026-04-30

### Added

- `--stdin` flag — run the post-fetch pipeline (prettify, `--output-charset`,
  `-o`) over a body read from stdin, with no HTTP request.
  Enables `pbpaste | recon --stdin --prettify-as json` for prettifying
  payloads from the clipboard or any pipe. Mutually exclusive with a URL.
- `--prettify-as <FORMAT>` flag — force the prettify format
  (`json` | `xml` | `html` | `yaml` | `csv` | `tsv` | `auto`). Implies
  `-p`. Useful when servers return the wrong `Content-Type` or when
  body sniffing guesses wrong. When forced, parse errors propagate as
  exit 1; auto-detect mode keeps the legacy lenient fallback.
- Script binding parity: `prettify` and `prettify_as` opts-map keys on
  `http(url, opts)`. Setting `prettify_as` implies `prettify: true`.

### Changed

- `src/output.rs` — extracted the buffered-output pipeline (charset
  transcode + prettify + write) into `pub fn write_processed_body`,
  shared between the HTTP response path and the new `--stdin` mode.
  Pure refactor for the existing HTTP path; no behaviour change there.

## [0.68.7] - 2026-04-26

### Added

- **Top-level `Makefile`** providing a small task runner over `cargo`. Targets
  cover the common dev loop (`build`, `release`, `all`, `check`, `test`,
  `fmt`, `fmt-check`, `clippy`, `lint`, `ci`), running and installing
  (`run`, `install`, `uninstall`), docs (`doc`, `pdf` — wraps the manual-PDF
  regeneration command from CLAUDE.md), introspection helpers (`flags`,
  `examples`, `bump-check`, `size`), and disk reclamation (`clean`,
  `clean-all`, `distclean`). `make` with no argument prints a colourised
  help listing. Recipes shell out to `cargo` and the release binary —
  no new build dependency.

## [0.68.6] - 2026-04-25

### Fixed

- **PDF comments now visually distinct when wrapping** (`src/docs.rs`): injected
  a small JavaScript post-processor into rendered HTML that wraps `# …` comment
  lines (and inline ` # …` tails) in `<span class="c">` elements styled
  `color: #6a737d`. This ensures that when a long comment wraps to the next
  line the continuation is still gray — indistinguishable-from-code wrapping
  was confusing in the PDF manual. `docs/MANUAL.pdf` regenerated.

## [0.68.5] - 2026-04-25

### Fixed

- **PDF code blocks no longer clip long lines** (`src/docs.rs`): the default
  `pre` CSS used `overflow-x: auto` which creates a scrollbar in browsers
  but silently clips content in PDFs (no scrolling exists). Added
  `white-space: pre-wrap` and `overflow-wrap: break-word` so long lines
  wrap within the page instead. `docs/MANUAL.pdf` regenerated.

## [0.68.4] - 2026-04-25

### Changed

- **Stop incremental build cache from accumulating on disk.** Added two
  dev-profile settings to `Cargo.toml`:
  - `[profile.dev] incremental = false` — disables the incremental
    compilation cache (`target/debug/incremental/`) which had grown to
    19 GB. Going forward no new cache entries are written. Existing stale
    artifacts can be reclaimed with:
    `rm -rf target/debug/incremental`
  - `[profile.dev.package."*"] debug = false` — strips full DWARF debug
    info from all 80+ compiled dependency crates, significantly reducing
    the size of new `target/debug/deps/` builds. The recon crate itself
    retains full debug info (no change to debuggability of recon source).
  No functionality change; all 1216 tests pass.

## [0.68.3] - 2026-04-25

### Changed

- **Release binary size reduced from 31.6 MB to 23.1 MB** (~27%) by adding
  a `[profile.release]` section to `Cargo.toml`:
  - `lto = "thin"` — cross-crate dead code elimination via thin LTO
  - `codegen-units = 1` — single codegen unit for better inlining and
    dead code removal (amplifies LTO)
  - `strip = "symbols"` — strips symbol table from the binary (66 K
    symbols → 434 remaining)
  No functionality or runtime performance change; debug builds are unaffected.

## [0.68.2] - 2026-04-25

### Fixed

- **Manual: incorrect `??` examples using `env()`** (`docs/MANUAL.md`):
  `env()` always returns a `String` (empty string when unset), never `()`,
  so `env("X") ?? "default"` never fires the fallback. Fixed two wrong
  examples in the Operators section — the `??` description now uses map
  lookups (which genuinely return `()` for missing keys) and adds a note
  directing users to the two-argument `env("X", "default")` form.

## [0.68.1] - 2026-04-25

### Fixed

- **Shebang script arg fallback** (`script/shebang.rhai`, examples, manual):
  `args[1] ?? "example.com"` threw an out-of-bounds exception when no
  argument was supplied because Rhai evaluates `args[1]` eagerly before `??`
  can act. Fixed to `if args.len() > 1 { args[1] } else { "example.com" }`.

## [0.68.0] - 2026-04-25

### Added

- **Shebang support for executable `.rhai` scripts.** Add
  `#!/usr/bin/env -S recon --script` as the first line of any script,
  `chmod +x` the file, and run it directly without typing
  `recon --script`. The `#!` line is converted to a `//` Rhai comment
  before compilation, preserving line numbers in error messages.
  - `recon --help shebang` documents the feature.
  - New example in `recon --examples` under "SCRIPTING (--script)".
  - Manual section: "Shebang — executable scripts" (Part III).

## [0.67.1] - 2026-04-25

### Changed

- **`docs/MANUAL.md`** — expanded Part III "Script language (Rhai)" with a
  comprehensive in-line language reference inserted before the "CLI
  inheritance" subsection. New subsections:
  - **Types and literals** — full type table (i64, f64, bool, (), char,
    String, Array, Map, Blob) with literal forms and examples.
  - **Operators** — arithmetic, comparison, logical, bitwise, string
    concatenation, null-coalescing (`??`), and ranges.
  - **Control flow** — if/else/ternary, while, loop, for (range / array /
    map), break/continue, return.
  - **Functions and closures** — named functions, closures, function
    pointers; hoisting rule documented.
  - **Error handling** — try/catch, throw, assert.
  - **Standard built-in functions** — type inspection/conversion, string
    methods (15+), array methods (17+), map methods, math functions
    (abs/sqrt/pow/trig/log/PI/E), and range iteration; each sub-section
    includes a reference table and a worked code example.
  - PDF regenerated.

## [0.67.0] - 2026-04-25

### Added

- **Wget-style batch flags** picked from `OUT-OF-SCOPE.md` low-hanging
  list. All long-form only — recon reserves single-letter flags for
  curl compatibility.
  - **`--wait <SECS>`** — fixed-seconds delay between URLs in a
    multi-URL invocation (e.g. with `--input-file`). Skipped before
    the first URL. Overrides `--rate` when both are set.
  - **`--tries <N>`** — total attempts per URL (wget semantics:
    `tries = retries + 1`). Overrides `--retry` when both are set.
    `--tries 1` disables retries; `--tries 0` is rejected at parse
    time. No infinite-retries support — use `--retry-max-time` as a
    ceiling.
  - **`--accept <LIST>`** — comma-separated filename-suffix accept
    list (case-insensitive). e.g. `--accept jpg,png` keeps only URLs
    whose final path segment ends in those suffixes. Suffixes match
    with or without a leading dot. URLs with empty final segments
    fail (matches wget).
  - **`--reject <LIST>`** — comma-separated filename-suffix reject
    list. Combines with `--accept` (URL must pass both filters).
    URLs with empty final segments pass.
- **Script-engine opts** mirroring the new CLI flags
  (`src/script/bindings/http.rs`):
  - `wait` (u64), `tries` (u32), `accept` (string), `reject` (string).
- **Help topic** `recon --help wget` covering the four new flags
  alongside the previously-shipped wget-compat set (`--input-file`,
  `--continue`, `--continue-at`, `--spider`, `--timestamping`).
  Topic aliases: `wait`, `tries`, `accept`, `reject`, `input-file`,
  `spider`, `timestamping`, `batch`.
- **Examples section** "WGET-STYLE BATCH FETCHING (0.67.0)" in
  `recon --examples` covering the four flags + the precedence rules.
- **Demo script** `script/wget-batch.rhai` exercising the new opts
  keys; indexed in `script/README.md`.
- **`src/wget_filter.rs`** — small helper module (`parse_suffix_list`,
  `should_keep`) with unit tests for the accept/reject matching.

### Changed

- **`OUT-OF-SCOPE.md`** — `-A` / `-R` removed from the standalone
  "wget standalone wins — leftover" line; the wget recursive cluster
  bucket now notes that `--accept` / `--reject` shipped in 0.67.0 as
  flat-list filters (the recursive variant remains deferred).

## [0.66.2] - 2026-04-25

### Changed

- **`CLAUDE.md` exposure policy hardened** to prevent the kind of
  gap that the Waiting-arc opened (90 CLI flags shipped without
  matching script-engine opts-map keys, fixed retroactively in
  0.66.1).
  - Six surfaces now numbered explicitly: --help, --examples,
    --flags, script engine, docs trio, manual + PDF.
  - Script-engine section gained a hard rule ("every new flag that
    affects request shape, output, transport, retry, auth, or
    protocol behaviour gets a matching opts-map key in
    `src/script/bindings/http.rs::build_args`, same release, same
    commit"), plus an explicit exclusion list of mode-selecting /
    pre-clap / process-level flags that are exempt.
  - Verification one-liner included so the developer can grep
    `cli.rs` field names against `http.rs` opts keys.
- **Pre-commit checklist** at the bottom of CLAUDE.md replaces the
  prose "mentally walk the surfaces" guidance with explicit
  checkbox items per change type (new flag, new binding function,
  new binding module, HISTORY-worthy change).
- Manual section collapsed into Surface 6 of the unified policy
  rather than living as a separate top-level section.

- **`OUT-OF-SCOPE.md` swept against the actual shipped flag set**.
  The "Additional curl flags (`curl --help all` sweep)" subsection
  added in 0.59.1 was wildly stale — most items shipped during the
  0.61.0–0.66.0 arc and were never removed from Waiting. This sweep:
  - Removes ~50 items from Waiting that shipped in 0.61.0–0.66.0
    (forms, netrc, range, time-cond, etag, retry, proto filter,
    input-file, continue, oauth2, xattr, spider, connect-to,
    tcp-nodelay, no-keepalive, capath, ca-native, tls-max,
    no-clobber, remove-on-error, create-file-mode, dump-header,
    stderr, no-progress-meter, styled-output, http1.1, http2,
    http2-prior-knowledge, append, crlf, hostpubsha256,
    hostpubmd5, pubkey, compressed-ssh, --config / -K, --disable
    / -q, all proxy-cluster + TLS-tuning stubs, all per-protocol
    stubs).
  - Reorganises the leftovers into clear themed sub-sections:
    "curl flags — leftover after the Waiting-arc",
    "Per-protocol plumb-through (0.65.0 stubs → real)",
    "Per-flag plumb-through (0.66.0 stubs → real)".
  - Moves architecturally-blocked flags from Waiting to
    Not-Yet-Supported (`--digest`, `--http1.0`, etc.).
  - Adds an audit-cadence note at the bottom: walk this file
    against `recon --flags` at the end of every multi-release arc
    so the divergence doesn't compound again.

### Note

This release contains no code changes — purely policy + working-notes
updates. Release date bumped to 2026-04-25 per the version-bumping
rule.

## [0.66.1] - 2026-04-24

### Added

Fills the script-engine gap that the Waiting-arc (0.61.0→0.66.0) left
open: every flag added in that arc now has a matching opts-map key in
`http(url, opts)`, and the most important ones get demo scripts.

**Script opts-map keys added** (for every CLI flag 0.61.0→0.66.0):

- 0.62.0: `range`, `max_filesize`, `url_query` (string or array),
  `request_target`, `disallow_username_in_url`, `time_cond`,
  `etag_compare`, `etag_save`, `timestamping`, `output_dir`,
  `remove_on_error`, `create_file_mode`, `no_clobber`, `no_buffer`,
  `dump_header`, `stderr`, `styled_output`, `no_progress_meter`,
  `show_error`, `capath`, `ca_native`, `tls_max`, `tcp_nodelay`,
  `no_keepalive`, `keepalive_time`, `connect_to` (string or array),
  `oauth2_bearer`, `xattr`, `spider`.
- 0.63.0: `form` (array), `form_string` (array), `form_escape`,
  `netrc`, `netrc_file`, `netrc_optional`, `http1_1`, `http2`,
  `http2_prior_knowledge`, `append`, `crlf`.
- 0.64.0: `retry`, `retry_all_errors`, `retry_connrefused`,
  `retry_delay`, `retry_max_time`, `rate`, `proto`, `proto_default`,
  `proto_redir`, `continue_auto`, `continue_at`.
- 0.65.0: `hostpubsha256`, `hostpubmd5`, `pubkey`, `compressed_ssh`.
- 0.66.0: `preproxy`, `proxy_header` (array), `proxy_http2`,
  `proxytunnel`, `proxy_capath`, `proxy_ca_native`, `proxy_crlfile`,
  `proxy_ciphers`, `proxy_tls13_ciphers`, `proxy_pinnedpubkey`,
  `ciphers`, `tls13_ciphers`, `curves`, `crlfile`, `pinnedpubkey`.

Total: ~60 new opts keys, all routed through `http()`'s existing
`build_args` helper.

**Demo scripts** for the headline new features:
- `script/retry.rhai` — retry cluster (transient, all-errors,
  connrefused, delay, max-time).
- `script/form.rhai` — multipart uploads with curl grammar.
- `script/netrc.rhai` — .netrc-backed Basic auth.
- `script/time-cond.rhai` — conditional GETs via
  If-Modified-Since + ETag.
- `script/batch-spider.rhai` — bulk link check (spider + retry
  + rate limiting via script-side loop).
- `script/oauth2.rhai` — Bearer token auth + 401 refresh pattern.
- `script/range.rhai` — byte-range fetches with max-filesize cap.

All demos are picked up by `tests/script_examples_it.rs` which
verifies every shipped `.rhai` parses cleanly.

### Note on deferrals

Opts keys for flags that are themselves CLI-only (`--config`,
`--input-file`, `--stderr`) or that don't make sense at the
per-call opts level (`--no-buffer`) are still included at the
opts-map level for completeness; they're plumbed through to the
per-call `Args` copy but have no effect beyond that.

## [0.66.0] - 2026-04-24

### Added

Final release of the Waiting-arc (0.61.0 → 0.66.0). Proxy extras +
TLS tuning + multi-config. 17 new flags.

**Fully implemented:**
- `-K, --config <FILE>` (+ include via `@other-file`) — load flags
  from a config file, curl's format. `#` / `;` comments, quoted
  values, `key = value` or `--flag value` forms, include-cycle
  detection (8-level limit).
- `-q, --disable` — ignore default config.

**Accepted at the CLI (plumbing deferred per the plan):**
- Proxy cluster: `--preproxy <URL>`, `--proxy-header`, `--proxy-http2`,
  `--proxytunnel` (long form only; `-p` is used by `--prettify`),
  `--proxy-capath`, `--proxy-ca-native`, `--proxy-crlfile`,
  `--proxy-ciphers`, `--proxy-tls13-ciphers`, `--proxy-pinnedpubkey`.
- TLS tuning: `--ciphers`, `--tls13-ciphers`, `--curves`,
  `--crlfile`, `--pinnedpubkey`.

The accepted-but-deferred flags get their plumb-through when reqwest
/ rustls expose the necessary knobs or when someone has a concrete
use case. Most require a custom `ServerCertVerifier` or cipher-list
parser that doesn't exist in the current rustls feature surface.

### Features tokens

`config-file`.

### End of the Waiting-arc

Six releases shipped over the arc (0.61.0 → 0.66.0):
- 0.61.0 — recon-own items (check digits, HRT, multi-decode, MQTT
  mTLS, --interface names)
- 0.62.0 — curl easy wins (~30 flags)
- 0.63.0 — forms + netrc + HTTP version + upload tweaks
- 0.64.0 — retry + proto filter + batch fetch
- 0.65.0 — per-protocol knobs (SSH pins + stubs)
- 0.66.0 — proxy extras + TLS tuning + --config

Total: ~90 new CLI flags, +46 tests (1155 → 1201), 4 new modules
(`src/iface.rs`, `src/netrc.rs`, `src/retry.rs`,
`src/proto_filter.rs`, `src/input_file.rs`, `src/config_file.rs`,
`src/checkdigit/tax_id.rs`).

## [0.65.0] - 2026-04-24

### Added

Release 5 of the Waiting-arc. Per-protocol knobs covering FTP,
SMTP, IMAP / POP3, Telnet, and SSH. 21 new flags.

**SSH host-key pinning + compression** (fully implemented):
- `--hostpubsha256 <SHA>` — accept only matching SHA-256 (hex or
  base64). Takes precedence over known_hosts.
- `--hostpubmd5 <HEX>` — legacy MD5 form.
- `--compressed-ssh` — ssh2 transport compression.
- Applied to ssh://, scp://, sftp://.
- `--pubkey <PATH>` accepted for curl parity; ssh2 authentication
  path already reads public keys alongside `--privkey`.

**FTP** (flag surface; deeper plumbing in a follow-up):
- `--disable-epsv`, `--disable-eprt`, `--ftp-pasv` — PASV-mode
  explicit forms; suppaftp already defaults to PASV.
- `--ftp-method <MODE>` — accepted (multicwd / nocwd / singlecwd).
- `--ftp-create-dirs` — accepted (FTP upload itself is still
  server-dependent).
- `-Q, --quote <CMD>` — accepted, repeatable.
- `--ftp-skip-pasv-ip` — accepted.
- `-l, --list-only` — accepted.
- `--tftp-no-options` — accepted.

**SMTP / IMAP / POP3** (flag surface):
- `--mail-auth <ADDR>` — AUTH address distinct from MAIL FROM.
- `--mail-rcpt-allowfails` — accepted.
- `--sasl-ir`, `--sasl-authzid <ID>`, `--login-options <STR>`
  — accepted for curl parity.

**Telnet:**
- `--telnet-option <OPT=VAL>` — accepted, repeatable.

### Scope note

SSH pinning + `--compressed-ssh` are fully implemented (the ssh2
crate exposes both cleanly). The remaining per-protocol flags are
accepted at the CLI level but not yet plumbed into the underlying
protocol modules — suppaftp, lettre, imap, pop3. Each needs a
small targeted patch. They're documented as "accepted for curl
parity" in --help so users can use them in scripts today and
pick up the implementation transparently later.

### Features tokens

`ssh-compress`, `ssh-pinning`.

## [0.64.0] - 2026-04-24

### Added

Release 4 of the Waiting-arc. Retry cluster + protocol restriction
+ wget-style batching. 11 new flags.

**Retry layer** (`src/retry.rs`):
- `--retry <N>` — retry count (default 0).
- `--retry-all-errors` — also retry non-transient errors (4xx, parse).
- `--retry-connrefused` — retry ECONNREFUSED specifically.
- `--retry-delay <SECS>` — fixed delay (else exponential backoff).
- `--retry-max-time <SECS>` — total wall-clock cap.
- `--rate <N/s|N/m|N/h>` — request rate limit (in batches).

**Protocol restriction** (`src/proto_filter.rs`):
- `--proto <LIST>` — curl syntax: `=https` (set), `+ftp` (add),
  `-ftp` (remove), `all` (wildcard).
- `--proto-default <SCHEME>` — default scheme for URLs without one.
- `--proto-redir <LIST>` — filter applied to redirect targets
  (accepted; redirect-time enforcement in a follow-up).

**Wget batching** (`src/input_file.rs`):
- `--input-file <PATH>` — URL list; `#` comments; `-` for stdin.
  Iterates each URL through the full HTTP pipeline including retry.
- `--continue` (wget bool) = auto-resume via Range: bytes=<size>-
  from the -o target.
- `-C, --continue-at <OFFSET>` (curl form) — explicit byte offset
  or `-` for auto-detect.

### Features tokens

`input-file`, `proto-filter`, `resume`, `retry`.

## [0.63.0] - 2026-04-24

### Added

Release 3 of the Waiting-arc. Forms + netrc + HTTP version pinning +
upload tweaks.

**Multipart forms:**
- `-F, --form <NAME=VALUE>` — curl-compatible grammar:
  `name=literal` / `name=@file` / `name=@file;type=MIME` /
  `name=@file;filename=NAME` / `name=<file` / `name=<-` (stdin).
  Repeatable; each `-F` adds a new part.
- `--form-string <NAME=VALUE>` — literal value; `@` / `<` NOT interpreted.
- `--form-escape` — backslash-escape special chars in field names
  and filenames.

**.netrc support** (new `src/netrc.rs`):
- `-n, --netrc` — require `~/.netrc` (or `$NETRC`) for credentials.
- `--netrc-file <FILE>` — override path.
- `--netrc-optional` — use if present, silent otherwise.
- Host-exact match → `default` block fallback. Parses `machine`,
  `default`, `login`, `password`, `account`, `macdef` (body skipped).

**HTTP version pinning:**
- `--http1.1` — reqwest `.http1_only()`.
- `--http2` — no-op (ALPN already prefers HTTP/2 for https://).
  Accepted for curl parity.
- `--http2-prior-knowledge` — reqwest `.http2_prior_knowledge()`.

**Upload tweaks:**
- `-T -` — upload body from stdin (curl convention).
- `--crlf` — LF → CRLF in the request body / upload.
- `-a, --append` — flag accepted; wires into FTP/SFTP append mode
  in a future per-protocol release (0.65.0). No-op for HTTP.

### Features tokens

`http-version-pinning`, `multipart`, `netrc`.

## [0.62.0] - 2026-04-24

### Added

Release 2 of the Waiting-implementation arc: curl's "easy wins"
cluster. ~30 new CLI flags, mostly thin wrappers around reqwest
builder calls or single-header injections.

**Request shape:**
- `-r, --range <RANGE>` — sets the Range header (bytes=RANGE form).
- `--url-query <DATA>` — appends URL-encoded query params
  (same grammar as --data-urlencode).
- `--request-target <PATH>` — accepted but errors cleanly
  (reqwest 0.12 has no hook for the request-line target).
- `--disallow-username-in-url` — reject URLs with a userinfo
  component (security hardening).
- `--max-filesize <BYTES>` — abort before streaming body when
  Content-Length exceeds the limit. K/M/G suffixes.

**Conditional requests:**
- `-z, --time-cond <TIME|FILE>` — If-Modified-Since from date
  string OR local file mtime. Prefix `-` inverts to
  If-Unmodified-Since.
- `--etag-compare <FILE>` — read an ETag from FILE, send
  If-None-Match.
- `--etag-save <FILE>` — write the response's ETag to FILE.
- `--timestamping` — wget-style shortcut for `-z <OUTPUT_FILE>`.

**Output control:**
- `--remove-on-error` — unlink the -o target on any error.
- `--no-clobber` — refuse to overwrite existing -o target.
- `--create-file-mode <MODE>` — chmod (Unix only) after creation.
- `-N, --no-buffer` — declared (documented as no-op for now;
  Rust stdout is line-buffered when TTY, unbuffered otherwise).
- `-D, --dump-header <FILE>` — save response headers to FILE.
- `--stderr <FILE>` — dup2 stderr onto FILE early in main().
- `--styled-output` / `--no-styled-output` — force / disable
  color output (currently advisory; recon auto-detects).
- `--no-progress-meter` — hide the progress bar even during
  file downloads.
- `--show-error` — accepted for curl compat (recon always shows
  errors; this is a no-op documented as such).

**Connection / TLS tuning:**
- `--capath <DIR>` — add every *.pem / *.crt / *.cer in DIR
  as a trusted root.
- `--ca-native` — disable bundled roots (pairs with --cacert /
  --capath).
- `--tls-max <VERSION>` — cap at 1.2 or 1.3.
- `--tcp-nodelay` — disable Nagle.
- `--no-keepalive` / `--keepalive-time <SECS>` — TCP keepalive.
- `--connect-to <H1:P1:H2:P2>` — per-host resolver override.

**Auth, metadata, wget standalone:**
- `--oauth2-bearer <TOKEN>` — Authorization: Bearer shortcut.
- `--xattr` — write URL + MIME type as extended attributes
  on the -o target (macOS / Linux).
- `--spider` — HEAD-only link check. Prints `<STATUS> <URL>`,
  exits non-zero if not 2xx.

**`--version` feature tokens**: `range`, `conditional-get`,
`spider`, `xattr`.

### Changed

- New `.cargo/config.toml` bumps `RUST_MIN_STACK` to 16 MiB so
  unit tests can allocate the (now-larger) `Args` struct without
  overflowing the default 2 MB test thread stack.

## [0.61.0] - 2026-04-24

### Added

Start of the Waiting-implementation arc (0.61.0 → 0.66.0 planned).
Release 1 focuses on recon's own pre-existing Waiting items.

- **Check digits — Latin-American + Australian + Mexican tax IDs**:
  - `br_cpf` (Brazilian CPF, 11 digits, two mod-11 check digits)
  - `br_cnpj` (Brazilian CNPJ, 14 digits, two mod-11 check digits)
  - `ar_cuit` / `ar_cuil` (Argentinian CUIT / CUIL)
  - `cl_rut` (Chilean RUT, with 'K' check-char support)
  - `pe_ruc` (Peruvian RUC)
  - `au_abn` (Australian ABN, ISO/IEC 7064 MOD 89; verify only)
  - `mx_rfc` (Mexican RFC, person + company forms)
- **Check digits — 110+ year warning** extended from Swedish
  personnummer to Danish CPR, Finnish henkilötunnus, Norwegian
  fødselsnummer, and Bulgarian EGN.
- **`--decode-all <IMAGE>`** — scan an image for every barcode
  (not just the first). One line per detection. Script binding:
  `encode::decode_all(blob)` → array of `#{ text, format }`.
- **`--hrt` / `--no-hrt`** — human-readable text row under 1D
  barcodes. Default on for EAN-13 / UPC-A, off for Code128 / Code39.
  Implemented for ASCII and SVG output; PNG HRT deferred
  (see OUT-OF-SCOPE.md).
- **MQTT mTLS** — `--client-cert` / `--client-key` / `--cert-type`
  / `--key-type` / `--pass` now plumb through `rumqttc`'s
  rustls ClientConfig. Works with `mqtts://` URLs.
- **`--interface` name resolution** — `--interface eth0` / `en0`
  now works via `libc::getifaddrs` on Linux / macOS. IP-literal
  form still works. Windows keeps literal-only with a clear error
  (GetAdapterAddresses is a separate follow-up).
- **`recon --version` feature tokens**: `decode-all`,
  `interface-name-resolution`, `latam-tax-ids`, `mqtt-mtls`.

### Changed

- `OUT-OF-SCOPE.md` Waiting section trimmed to remove items shipped
  in this release. PNG HRT added as a deferred follow-up.

## [0.60.0] - 2026-04-24

### Added

- **`--flags`** — alphabetical curl-style flag listing. Format:
  `(short, ) --long <VALUE>  short description`, sorted by long name,
  one flag per line, descriptions capped at ~52 characters (first
  sentence of each flag's clap doc comment). Auto-paged through
  `$PAGER`. Complements `--help` (topic deep-dives) and `--examples`
  (curated scenarios) — this is the quick lookup index.
- **Help topic**: `recon --help flags` explains the listing format
  plus grep-friendly usage patterns.
- **`--version` Features token**: `flag-listing`.

### Changed

- **`CLAUDE.md`** gains a "Fifth surface" note under the exposure
  policy: every flag's **first sentence** must be self-contained and
  fit within ~52 chars so the `--flags` listing stays scannable.
  Doesn't require a separate registration step (clap introspection
  handles it automatically) but flag authoring has to keep the
  headline tight.
- **`OUT-OF-SCOPE.md`** grew a new subsection under **Waiting**
  cataloguing ~150 curl flags not in recon, grouped by theme (HTTP
  version pinning, forms, conditional requests, byte-range, output
  control, retry/rate, parallel transfers, protocol restriction,
  proxy, TLS tuning, DoH, FTP, SMTP/IMAP/POP3, SSH, tracing,
  multi-config, variables, telnet, xattr, legacy). Pros/cons for the
  ~15 high-value items; compact list for the long tail. Assessment
  paragraph recommends a 7-release phasing if curl-parity ever
  becomes a goal; `-F / --form` and `-n / --netrc` flagged as the
  two most-requested omissions.

### Regenerated

- **`docs/MANUAL.md`** + **`docs/MANUAL.pdf`** — gained a `--flags`
  subsection under Meta flags. Version bumped to 0.60.0.

## [0.59.1] - 2026-04-24

### Changed

- `OUT-OF-SCOPE.md` retitled "Out of Scope & Wishlist" to match how
  it's actually being used (the "Waiting" section = wishlist items).
- Added a "wget features (things curl — and therefore recon —
  doesn't have)" subsection under **Waiting**. Pros / cons for 16
  wget-unique features, grouped into "standalone wins" (input-file,
  timestamping, spider, continue, http1.0) and the "recursive /
  mirror cluster" (recursive + level + mirror + page-requisites +
  convert-links + accept/reject + domain filters + no-parent +
  quota + wait + tries + background). Includes an assessment
  paragraph on phasing if wget-parity ever becomes a goal.
- No code changes.

## [0.59.0] - 2026-04-24

### Added

- **`--unsafe-html`** — allow raw HTML passthrough in markdown (comrak's
  `unsafe_` option). Needed for cover pages and explicit page-break
  markers. Off by default; assume the markdown input is trusted when on.
- **`--page-break-on-h1`** — start a new PDF page before every top-level
  `#` heading except the first. Injects `break-before: page` CSS; no
  visible effect in HTML output (Chrome's printToPDF honours it).
- **Cover-page CSS** — the bundled default stylesheet styles
  `<div class="cover">` as a full-page centered block with automatic
  page break after. Supports `.subtitle`, `.version`, `.date`,
  `.author`, `.meta` child classes plus `<hr>` dividers.
- **`<!-- toc -->` marker** — when present in the rendered body, the
  auto-generated TOC is injected at that marker instead of at the top
  of the document. Lets users place the TOC after a cover page.
- **Script opts keys**: `unsafe_html`, `page_break_on_h1` on
  `md_to_html` / `md_to_pdf` / `html_to_pdf`.
- **`recon --version` Features token**: `pdf-cover-page`.

### Changed

- **`docs/MANUAL.md`** gained a styled cover page (raw HTML inside
  `<div class="cover">`) and relies on `<!-- toc -->` placement so the
  auto-TOC lands on page 2. `--page-break-on-h1` gives every top-level
  section its own page. Regenerated `docs/MANUAL.pdf`: 67 → 71 pages.

## [0.58.2] - 2026-04-24

### Added

- **`docs/MANUAL.md`** — comprehensive user manual covering every
  CLI flag, every script binding, and many examples. 2450 lines of
  markdown organized in four parts: Getting started, CLI reference,
  Script engine, Appendices.
- **`docs/MANUAL.pdf`** — PDF rendering of the manual, produced via
  recon's own `--md-to-pdf` flow (with `--toc --toc-depth 3 --gfm`).
  67 pages, linkable table of contents.

### Changed

- **CLAUDE.md** gains a "Manual" section under the exposure policy.
  The exposure policy's four surfaces become five — the manual is a
  first-class surface that must be updated alongside every code
  change to a user-visible flag, binding, or behaviour. The PDF is
  regenerated whenever the markdown changes; both files are checked
  in.

## [0.58.1] - 2026-04-24

### Changed

- Reorganized `OUT-OF-SCOPE.md` into four explicit buckets by reason:
  **Waiting** (can be done, not asked), **Deferred** (possible but
  put off), **Not yet supported** (blocked on upstream), **Out of
  scope** (architecturally / policy-declined). No content removed;
  entries reshuffled to the most honest bucket. Process notes at the
  bottom spell out when entries can migrate between buckets as the
  world changes.

## [0.58.0] - 2026-04-24

### Added

Document conversions — markdown → HTML / HTML → PDF / markdown → PDF
with linkable tables of contents.

- **`--md-to-html <SRC>`** — markdown → HTML via the `comrak` crate
  (CommonMark + GFM, pure Rust). SRC = path / URL / `-` (stdin).
  Output via `-o PATH` or stdout.
- **`--md-to-pdf <SRC>`** — markdown → PDF by pipelining md-to-html
  into agent-browser's `pdf` command. `-o PATH` required.
- **`--html-to-pdf <SRC>`** — HTML → PDF via agent-browser.
- **`--toc` / `--toc-depth N` / `--toc-title STR`** — inject a
  linkable table of contents into the generated HTML. Chrome's
  printToPDF preserves anchor links so the TOC stays clickable in the
  PDF.
- **`--doc-title STR`** — sets `<title>` + PDF metadata title.
- **`--doc-css PATH`** — inline a custom stylesheet (appended after
  the bundled print-friendly default).
- **`--no-default-css`** — skip the bundled default; useful with
  `--doc-css` for a full override.
- **`--gfm`** — enable GitHub-flavored extensions (tables, task
  lists, strikethrough, autolinks, footnotes, tagfilter). Sensible
  defaults (tables, strikethrough, autolinks, task lists) are on
  without `--gfm`.
- **Script bindings**: `md_to_html(src, opts)`, `md_to_pdf(src, dest,
  opts)`, `html_to_pdf(src, dest)`. Source is a literal string or
  Blob; scripts fetch / load via existing `http()` / `file_read()`.
- **`recon --version` Features tokens**: `markdown`, `pdf-export`.

### Technical

- New `src/docs.rs` (~390 LOC) — comrak-driven HTML render, TOC
  generation, bundled print CSS (@page + serif body + monospace code
  + table borders).
- New `src/docs_pdf.rs` — agent-browser orchestration (write HTML to
  tempfile → open → pdf → close → tempfile drop).
- PDF generation requires agent-browser on PATH; missing binary
  reports a clear error with `brew install agent-browser` /
  `npm install -g agent-browser` hint.
- New crate dep: `comrak = "0.28"`. No HTML→PDF engine added; Chrome
  via the existing agent-browser integration is the PDF backend.

### Out of scope

Deferred to follow-ups: pure-Rust HTML→PDF (no mature renderer
exists); `typst`-based md→PDF alternative (binary bloat + requires a
hand-rolled md→typst translator); other markup (RST, AsciiDoc, Org);
custom page sizes / margins beyond what agent-browser's `pdf`
exposes; encrypted-PKCS#8 in-process decryption.

## [0.57.0] - 2026-04-24

### Added

Script-side TCP + UDP server primitives. Pairs with 0.56.0's
thread_spawn so scripts can accept on the main thread and hand each
connection to a worker — the classic concurrent server shape.

- **TCP**: `tcp_listen`, `tcp_accept(listener [, timeout_ms])`,
  `tcp_read(conn, n, timeout_ms)`, `tcp_read_line(conn, timeout_ms)`,
  `tcp_write(conn, blob|str)`, `tcp_peer_addr(conn)`, `tcp_close(conn)`,
  `tcp_close_listener(l)`.
- **UDP**: `udp_bind(addr)`, `udp_recv_from(sock, max_len [, timeout_ms])`,
  `udp_send_to(sock, blob|str, addr)`, `udp_close(sock)`.
- **New script examples**: `script/tcp-echo.rhai` (concurrent echo
  server), `script/udp-listen.rhai` (UDP beacon listener).
- **`recon --version` Features token**: `script-servers`.

### Deferred

- **ICMP raw-socket primitives** — recon already has `ping()` for
  reachability checks; full ICMP type/code send/recv would add raw
  sockets + kernel permission handling. Documented in
  `OUT-OF-SCOPE.md`; revisit when users ask for specific traffic-
  generation or monitoring use cases.
- **CLI server flags** (`recon --listen …`) — deliberately not added.
  Server workflows are multi-step; scripts are the right layer. Use
  `recon --serve` for the pre-built HTTP server.

## [0.56.0] - 2026-04-24

### Added

Script concurrency primitives. Scripts can now fan out work across OS
threads, coordinate via MPSC channels, and gather results — opening
the door to parallel probe suites, stream processing, and the 0.57.0
script-server release.

- **`thread_spawn(fn_ptr)`** — spawn a closure on a fresh OS thread.
  Optional forms: `thread_spawn(fn, arg)`, `thread_spawn(fn, args_array)`.
  Returns a `ThreadHandle`. `spawn` alone is reserved by Rhai.
- **`join(handle)`** — block on the handle; returns the closure's
  return value or raises the worker's error.
- **`channel()`** — unbounded MPSC. Returns `[sender, receiver]`.
- **`channel_bounded(n)`** — bounded MPSC with capacity `n`;
  `try_send` returns false on full.
- **`send(tx, val)` / `try_send(tx, val)` / `recv(rx)` /
  `recv(rx, timeout_ms)` / `try_recv(rx)`** — channel ops.
- **`tid()`** — current thread ID (stable within a run).
- **`sleep(ms)`** — alias of `sleep_ms` for thread-side readability.
- **`recon --version` Features token**: `script-concurrency`.

### Changed

- **`rhai` crate**: flipped on the `sync` feature. Makes the engine
  Send+Sync at a small per-value locking overhead (~10-15% on hot
  paths, irrelevant for diagnostic scripts).
- **`BrowserHandle`** + **`SqliteHandle`**: swapped internal
  `Rc<RefCell<…>>` state for `Arc<Mutex<…>>` so they survive the
  thread-boundary crossing that the sync-feature unlocks.

### Technical

New `src/script/bindings/thread.rs` (~230 LOC). Spawn worker builds a
fresh engine per thread (~ms) and re-registers threading primitives so
nested `thread_spawn` calls work. Shares the compiled AST via
`rhai::Shared<AST>` so workers dispatch the same program text. Parent's
`ScriptDefaults` are Arc-shared to each worker, preserving CLI-flag
inheritance across threads.

## [0.55.0] - 2026-04-24

### Added

Encoding & decoding expansion backed by the `rxing` crate (pure-Rust
port of ZXing). Closes four of the encoding-deferred entries in
`OUT-OF-SCOPE.md`.

- **`--decode <IMAGE>`** — scan a PNG / JPEG / WebP / GIF / BMP for a
  barcode, QR, DataMatrix, Aztec, PDF417, or MaxiCode. Accepts a path
  or `-` for stdin. Output: `<FORMAT>\t<TEXT>`.
- **`--decode-hints <LIST>`** — comma-separated format restriction
  (qr, datamatrix, aztec, pdf417, maxicode, code128, code39, code93,
  codabar, ean13, ean8, itf, upca, upce, rss14). Speeds up scanning
  and disambiguates codes that share prefixes.
- **`--encode aztec`, `--encode pdf417`** — two new 2D barcode formats.
  Render to ASCII / SVG / PNG like the existing QR and DataMatrix
  paths. MaxiCode is available as a decode-only format (ZXing /
  rxing ship no MaxiCode encoder today).
- **Script binding `encode::decode(blob)`** — scans an in-memory image
  Blob; returns `#{ text, format }`.
- **`encode::list()`** — now enumerates the three new formats.
- **`recon --version` Features tokens**: `aztec`, `decode`,
  `maxicode`, `pdf417`.

### Technical

- New `src/decode.rs` wrapping `rxing::helpers::detect_in_file`, with
  a byte path via tempfile for stdin / in-memory blobs (rxing's
  decoder pipeline expects a filesystem path for format autodetection).
- Existing `src/encode.rs` extended with an `encode_via_rxing` branch
  for the new formats; QR and DataMatrix continue to flow through the
  original `qrcode` / `datamatrix` crates for ASCII/SVG rendering
  continuity. BitMatrix adapter converts rxing's matrix type into
  recon's own render-ready shape.

### Out of scope

Removed from `OUT-OF-SCOPE.md`: image→text decoding, Aztec, PDF417.
Partially closed: MaxiCode (decode works; encode requires a separate
encoder library that doesn't exist in pure Rust). Still deferred: logo
overlay / colour customisation, multi-code composition, MaxiCode
encoding, multi-barcode scanning, --encode-hints.

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
