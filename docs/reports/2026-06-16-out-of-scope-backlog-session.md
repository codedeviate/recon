# OUT-OF-SCOPE backlog session (Tier 1 + Tier 2 + mTLS) — Session Report

**Date:** 2026-06-16
**Repository:** recon
**Branch:** master
**Shipped:** 0.96.1 → **0.100.0** (5 releases), all live on GitHub +
crates.io + Homebrew (brew upgrade verified end-to-end).

## Spec Summary

The user asked to prioritize the OUT-OF-SCOPE.md backlog for "do now"
items, then implement them one by one. The session covered all of Tier 1
(small, self-contained), a folded-in warning cleanup, the Tier 2 anchor
(`--pinnedpubkey` / `--curves`), and its mTLS follow-up. Each feature ran
its own brainstorm → spec → TDD → six-surface exposure → release cycle.
Specs live under `superpowers/recon/specs/2026-06-16-*`.

## Plan Summary

- **#1** AI verbose telemetry — thread script verbosity into `ai::*`;
  emit `* ai: …` per `.send()`.
- **#2** `--http2-fingerprint` — parse the Akamai H2 string into
  `wreq::Http2Config`; keep JA3/JA4 deferred (lossy / non-invertible).
- **#3** `--render-no-links` — add `no_links` to `RenderOpts`; new CLI flag
  + script opts keys.
- **#2.5** config_resolver dead-code cleanup (folded into 0.98.0).
- **#4** `--pinnedpubkey` + `--curves` — custom `rustls::ClientConfig` via
  `use_preconfigured_tls` (opt-in path).
- **#5** mTLS follow-up — make `--client-cert`/`--client-key` compose with
  pinning/curves (close the error-on-combine gap from #4).

## What Actually Happened

### #1 — AI verbose telemetry (v0.96.2, `107ae24`)
- **Deviation from spec:** spec said `register(&mut Engine, ScriptDefaults)`.
  The `ai` module is mounted twice — binary tree + a `lib.rs` `#[path]`
  re-mount that deliberately avoids `cli::Args` (which `ScriptDefaults`
  pulls in). So `register` takes a bare `verbose: u8`. Caught at first compile.

### #2 — `--http2-fingerprint` (v0.97.0, `119bf93`)
- Feasibility confirmed from wreq/hyper2/http2-fork source: `Http2Config`
  exposes every Akamai field; `.http2(closure)` after `.emulation(profile)`
  overrides H2 per-field while keeping profile TLS.
- **Test caught a real bug:** the reworded `--ja3/--ja4` error contained
  "TLS", which `main.rs::friendly_message` rewrites to a generic placeholder
  — swallowing the message. (First of three encounters with this trap.)
- `impersonate.rs` → `impersonate/mod.rs` to host the parser submodule.

### #3 — `--render-no-links` (v0.98.0, `92a5298`)
- To plan. Plain mode → `link_footnotes(false)`; coloured mode → skip the
  `Link` annotation styling.
- **Test-target gotcha:** `render.rs` / `impersonate/*` / `tls_config.rs`
  unit tests live in the binary crate — `cargo test --lib <filter>` finds
  nothing; must use `--bin recon <filter>`.

### #2.5 — config_resolver dead-code cleanup (folded into 0.98.0, `afbeae5`)
- Pre-existing `system_candidates` / `user_path` dead-code warnings (not
  from this session's work) were folded into the not-yet-tagged 0.98.0
  rather than spun into a throwaway 0.98.1.

### #4 — `--pinnedpubkey` + `--curves` (v0.99.0, `1db37ec`)
- **Security finding:** `--pinnedpubkey` was previously parsed but **never
  enforced** — a silent no-op giving a false sense of security. Now
  fail-closed.
- reqwest has no setter for either, so both route through a new
  `src/tls_config.rs` building a `rustls::ClientConfig` via
  `use_preconfigured_tls`. **Opt-in** (`needs_custom_tls`): the common path
  keeps reqwest's high-level setters, extracted into
  `client::configure_native_tls`; the custom path reproduces
  cacert/capath/ca-native/crlfile/tlsv1.x/tls-max/insecure.
- Pinning: `PinnedKeyVerifier` checks SHA-256 of the leaf SPKI (via the
  existing `x509-parser`), enforced even under `-k`. Curves: override
  `kx_groups` on a cloned ring `CryptoProvider`; P-521 errors.
- **`friendly_message` trap hit AGAIN** (×2 and ×3): the combine error and
  the pin-mismatch surface both had to avoid "TLS"/"certificate"; added a
  `pinnedpubkey` root-cause branch so mismatches surface clearly.
- **Tooling landmine:** ran `cargo fmt` to fix a block-indent — it
  reformatted the **entire hand-formatted repo** (231 files). Recovered via
  a recoverable `git stash` (the classifier correctly blocked a
  non-recoverable bulk `git checkout`), then re-applied the change cleanly
  by extracting `configure_native_tls` so the diff stayed scoped.

### #5 — mTLS composes with pinning/curves (v0.100.0, `2d94eb8`)
- Closed the error-on-combine gap from #4. Extracted
  `client_cert::load_combined_client_pem` (shared by the reqwest `Identity`
  path and a new `build_rustls_client_auth`); the custom config now
  terminates with `.with_client_auth_cert` when a client identity is present.
- Same PEM-only scope + DER/ENG/encrypted-key errors as the plain mTLS path.
- Test fixture: generated a throwaway self-signed EC cert+key via openssl,
  embedded as constants for the `build_rustls_client_auth` unit tests.

## Final Outcome

**Commits (all on master, all released):**
- `107ae24` feat(ai): per-.send() verbose telemetry (-v / -vv) — 0.96.2
- `119bf93` feat(impersonate): implement --http2-fingerprint (Akamai) — 0.97.0
- `92a5298` feat(render): add --render-no-links toggle — 0.98.0
- `afbeae5` fix(config): remove dead-code wrappers in config_resolver
- `1db37ec` feat(tls): implement --pinnedpubkey and --curves — 0.99.0
- `2d94eb8` feat(tls): allow mTLS with --pinnedpubkey / --curves — 0.100.0

**New modules:** `src/tls_config.rs`, `src/impersonate/h2_fingerprint.rs`,
`tests/pinnedpubkey_it.rs` (+ extended `tests/impersonate_it.rs`,
`tests/script_ai_it.rs`).

**Tests:** final default suite **1678 passed / 2 ignored** (23 suites);
impersonate variant builds clean + 7 integration tests; tls_config 10 unit,
h2_fingerprint 14 unit, client_cert 9 unit, pinnedpubkey_it 6 integration.
Both build variants 0-errors and warning-free (the config_resolver warnings
were fixed in `afbeae5`).

**Release:** each version tagged + GitHub release + crates.io + both
Homebrew formulae (recon + recon-impersonate). `brew fetch recon` verified
the 0.100.0 tarball sha matches the formula (no repeat of the v0.82/v0.85
sha-race), and `recon 0.100.0` installs/upgrades cleanly.

## Gotchas worth remembering (saved to project memory)

1. **Binary-crate tests need `--bin recon`, not `--lib`** (modules declared
   in `main.rs`: render, impersonate, tls_config).
2. **`cargo fmt` is NOT safe repo-wide** — the codebase is hand-formatted;
   it reformats everything. Match surrounding style by hand.
3. **The `make trim` auto-trim hook breaks incremental rebuilds** — repeated
   `No such file or directory` on `.fingerprint/`. `cargo clean -p
   recon-cli --release` reliably recovers. Worth revisiting the hook.
4. **`main.rs::friendly_message` swallows any error containing "TLS"/
   "certificate"** — keep those words out of impersonate/tls error strings
   (hit three times this session).
5. **agent-browser PDF regen flakes when run as a detached background
   command** (gets reaped) — re-run succeeds.

## Remaining backlog

- **Tier 3** (all larger / lower-ROI, none a natural "next"): HTTP backends
  for `ai::*` (gated on a real use case), alt-svc cache (~300 LOC, low
  value), typst Chrome-free md→PDF (+15–25 MB), wget recursive/mirror
  cluster (own spec, ~800–1200 LOC).
- **Tier 2 leftovers:** `--client-cert`/`--cacert` etc. with `--impersonate`
  (BoringSSL mTLS); `recon --ai PROMPT` CLI surface; `--pinnedpubkey`
  public-key file-path form (currently errors with a clear hint).
