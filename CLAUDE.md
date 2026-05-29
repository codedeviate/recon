# recon — Claude Code Instructions

## Committing

After each logical sub-job within a larger task — e.g. after adding a module,
after wiring up CLI flags, after updating docs — create a git commit. Small,
focused commits make it easy to revert a specific step without losing unrelated
work. Don't batch everything into one commit at the end.

## Building — release-only by default

When building `recon`, build the **release** target only. Do **not**
also run `cargo build` (debug) as a follow-up — it doubles compile
time and disk footprint for no benefit in the normal verify-a-change
workflow. The release binary is what the manual-regen step, the
post-merge sanity check, and the user's day-to-day invocations all use.

Build the debug target only when:

- the user explicitly asks for it ("build debug", "give me a debug
  build", etc.), or
- a debug-only workflow requires it (e.g. running `cargo test` without
  `--release`, attaching a debugger, or chasing a bug that only
  reproduces in the dev profile).

Both feature variants of the release build still apply per the
post-merge hygiene section — `cargo build --release` and
`cargo build --release --no-default-features --features impersonate`.

**The impersonate variant MUST use `--no-default-features`.** The
default feature set includes `ssh` (scp/sftp/ssh via libssh2, built
against OpenSSL); the impersonate feature links BoringSSL via `wreq`,
and the two SSL backends collide at link time (`libssh2`'s
`OSSL_PARAM_*` symbols go unresolved). Dropping default features
drops `ssh`, so the impersonate binary links a single SSL backend.
The impersonate variant therefore has no scp/sftp/ssh probing — by
design. See GitHub issue #1.

## Version Bumping

After every change, bump the version according to these rules:

- **PATCH** (`0.x.N+1`): bug fixes, help/documentation text updates, minor changes that don't add or remove flags or features.
- **MINOR** (`0.N+1.0`): new flag or feature added, existing flag removed or significantly changed.
- **MAJOR** (`N+1.0.0`): breaking changes to existing behaviour (reserved, rare).

### The version number lives in five files — touch ALL of them

A version bump is incomplete if any of these is missing. They must
agree exactly: same `X.Y.Z`, same `YYYY-MM-DD`.

| File | Field |
|---|---|
| `Cargo.toml` | `version = "X.Y.Z"` |
| `src/version.rs` | `const RELEASE_DATE: &str = "YYYY-MM-DD"` |
| `CHANGELOG.md` | New `## [X.Y.Z] - YYYY-MM-DD` heading at the top, above older versions, under `## [Unreleased]` if present |
| `docs/MANUAL.md` cover (top of file) | `<div class="version">Version X.Y.Z</div>` and `<div class="date">YYYY-MM-DD</div>` |
| `README.md` badges header (top of file) | `release-v<X.Y.Z>-blue` in the "Latest release" badge URL |

Group CHANGELOG entries into the keep-a-changelog subsections:
`### Added`, `### Changed`, `### Fixed`, `### Removed`, `### Deprecated`,
`### Security`.

Then regenerate `docs/MANUAL.pdf` (see exposure-policy section 6 for
the command). `recon --version` should now report the new version and
the new date — that's the consistency check.

### Tagging implies three publish steps — same release, no exceptions

When you push a `vX.Y.Z` tag, the release isn't complete until all
three downstream surfaces are updated:

1. **GitHub release** — auto-generated notes attached to the tag.
2. **crates.io** — `recon-cli` published from `Cargo.toml`.
3. **Homebrew tap** — both formulae in `../homebrew-cli/Formula/`
   bumped (url + sha256) and committed.

A tag without these leaves the install paths out of sync: shields.io
badges turn stale, `cargo install recon-cli` and `brew upgrade recon`
keep returning the old version, and anyone following the install
instructions from the README installs an older binary. Treat all
three publishes as part of the tag — same flow, no follow-up commits
needed on the recon repo itself.

#### 1. GitHub release

```sh
gh release create vX.Y.Z --generate-notes
```

#### 2. crates.io

```sh
# From the recon repo root, with the new version already in Cargo.toml.
cargo publish
```

The crate name is `recon-cli` (per `Cargo.toml [package] name`). The
binary inside the crate is still `recon`. Requires `cargo login` to
have been run once (token stored in `~/.cargo/credentials.toml`).
`cargo publish` runs its own checks (clean working tree, no
path-dependencies, etc.) and aborts cleanly if anything is wrong —
fix the underlying issue rather than passing `--allow-dirty`.

#### 3. Homebrew tap (`../homebrew-cli/`)

The tap repo at `../homebrew-cli/` carries two formulae for recon:
`Formula/recon.rb` (default build) and `Formula/recon-impersonate.rb`
(the `impersonate` feature variant). **Both must be bumped on every
release** — they install conflicting binaries, but users may be on
either formula.

For each `.rb` file, update two fields:

- `url "https://github.com/codedeviate/recon/archive/refs/tags/vX.Y.Z.tar.gz"`
- `sha256 "<new-tarball-sha256>"`

Compute the sha256 from the GitHub-generated tarball after the
release exists. **Always pass `-H "Cache-Control: no-cache"` so the
fetch bypasses any intermediate caches (your ISP, corporate proxy,
local resolver) and goes through to GitHub's origin:**

```sh
curl -sL -H "Cache-Control: no-cache" \
    https://github.com/codedeviate/recon/archive/refs/tags/vX.Y.Z.tar.gz \
    | shasum -a 256
```

**Important: GitHub's auto-generated tarball CDN can serve a
transient/incomplete payload for the first minute or two after the
tag is pushed.** Run the `shasum` command twice with a short pause
between, and only proceed if both runs return the same hash. If they
differ, wait 30–60 seconds and re-check until the hash stabilises.
Using an unstable hash is the most common cause of "homebrew reports
wrong checksum" reports after a release — and it produces a confusing
failure mode where the formula matches what you computed but doesn't
match what users fetch later. v0.82.0 hit this exact race; v0.85.0
hit it again because the recheck without `Cache-Control: no-cache`
re-read the same cached payload twice (a "stable" but wrong hash) and
the mismatch surfaced only when users ran `brew install`.

The `Cache-Control: no-cache` rule isn't optional even on a "fresh"
shell. Network paths cache aggressively; a single `curl` without the
header is allowed to return whatever was last cached for that URL —
which can be an early CDN payload that no longer matches what GitHub
serves to homebrew clients.

Then commit and push the tap repo:

```sh
cd ../homebrew-cli
git add Formula/recon.rb Formula/recon-impersonate.rb
git commit -m "recon X.Y.Z"
git push origin main   # tap default branch is `main`, not `master`
```

(Tap commits follow the convention `<formula> X.Y.Z` — see
`git log --oneline` in `../homebrew-cli` for examples.)

If `shasum` produces a hash that, after pasting into `recon.rb`,
makes `brew install --build-from-source recon` fail with "SHA256
mismatch", recompute against the URL the formula points to (case
matters — `vX.Y.Z` not `VX.Y.Z`) and amend with a follow-up commit
like the existing `recon 0.81.3 — fix sha256` precedent.

### README.md badges — keep in sync with reality

The `README.md` header carries six shields.io badges. The release-version
badge changes every bump (covered by the table above); the others are
mostly static but need a sync sweep whenever the underlying fact changes.
The hard rule: **if any of these underlying facts changes, update the
matching badge in the same commit.**

| Badge | Underlying fact | When to update |
|---|---|---|
| `release-vX.Y.Z-blue` | `Cargo.toml` `version` | Every version bump. |
| `github-codedeviate%2Frecon` | Repo location | Only on repo rename / org change. |
| `crates.io` | Published crate name (`recon-cli`) | Only if the crate is republished under a new name. |
| `homebrew-codedeviate%2Fcli%2Frecon` | Tap formula path | Only if the tap or formula is renamed. |
| `license-MIT-blue` | `LICENSE` file | Only on a license change. |
| `rust-2021_edition_(MSRV_1.85)` | `Cargo.toml` `edition` + `rust-version` | Whenever `edition` or `rust-version` in `Cargo.toml` changes. |

`README.md` is a six-surface-policy artifact like the others — leaving
a badge stale (e.g. a Rust MSRV badge that no longer matches
`rust-version`) is the same class of violation as a stale CHANGELOG.

### HISTORY.md — only when there's design rationale to capture

For changes with notable design rationale (new crate chosen and why, new
module laid out, cross-cutting refactor), also add a numbered entry to
the `## Feature Additions` section of `HISTORY.md` or a subsection under
`## Architecture Decisions`. Routine bug fixes and flag additions do not
require a HISTORY.md entry — the changelog is enough.

## Exposure policy — every feature must reach every surface

Every time a new flag, function, protocol probe, or script binding lands,
update **all six** of the surfaces below **in the same release**.
Missing any one is a policy violation — fix it before the next release,
even if that means a patch release whose only job is closing the gap.

The surfaces, in order:

1. `recon --help <topic>` (long-form deep dive)
2. `recon --examples` (curated scenarios)
3. `recon --flags` (curl-style alphabetical index — auto-generated, but the doc comment matters)
4. **Script engine** (`http(url, opts)` opts-map keys + script demos)
5. **Documentation trio** (`CHANGELOG.md`, `HISTORY.md`, `OUT-OF-SCOPE.md`)
6. **Manual** (`docs/MANUAL.md` + regenerated `docs/MANUAL.pdf`)

Surface-by-surface rules below; the per-flag pre-commit checklist at
the **bottom of this file** is the operational summary.

### 1. `recon --help <topic>`

Every new flag gets a `FlagHelp` entry in the relevant `TOPIC_*`
static in `src/help.rs`. New feature areas get a whole new topic
added to `topic_keys()` + `resolve_topic()`. Pick aliases generously
(e.g. `charset` / `text` / `iconv` / `text-encoding` all routed to
`TOPIC_TEXT_ENCODING`). Cross-reference from `TOPIC_PROTOCOLS` when a
new URL scheme lands.

### 2. `recon --examples`

Every feature gets at least one `example()` block in
`src/examples.rs` under a suitable `section()` — a new section if
the feature is a new area, otherwise an addition to the closest
existing one. Include a `note()` line calling out the non-obvious
behaviour (defaults, detection priority, when the feature silently
falls back).

### 3. `recon --flags` — auto-generated, doc-comment-driven

Every flag added to `src/cli.rs` automatically shows up in `recon
--flags` because the listing is generated from clap's introspection.
But the short description is the first line of the flag's clap doc
comment, truncated to ~52 characters. So:

- The doc comment's **first sentence** must be self-contained and
  make sense on its own (not "…continued from the previous paragraph").
- The first sentence should fit within ~52 characters. Longer sentences
  are truncated with an ellipsis — readable but not ideal.
- Explicit `value_name` is set when the default clap-derived uppercase
  version would look ugly (`<CHECKDIGIT_LIST>` vs just omitting it
  for a bool).

Run `recon --flags | grep <new-flag>` after adding a flag to confirm
the first-line summary reads well.

### 4. Script engine — **the surface most likely to be missed**

Pre-arc-history experience: it's easy to add CLI flags fast and forget
about the script engine for several releases at a stretch. Don't.

**The hard rule**: every new flag in `src/cli.rs` that affects
request shape, output, transport, retry, auth, or protocol behaviour
gets a matching opts-map key in `src/script/bindings/http.rs`'s
`build_args` function (or the relevant binding for non-HTTP probes).
**Same release, same commit.** No "we'll catch it up later".

The opts key is `snake_case` and matches the Rust field name on
`Args`. Repeatable flags (`Vec<String>`) accept either a single
string or a Rhai array of strings via `opts_clone_array`.

The exclusion list — flags that genuinely don't have a per-call
script equivalent and may be omitted:

- **Mode-selecting flags** that don't fire an HTTP request:
  `--init`, `--examples`, `--flags`, `--help`, `--version`,
  `--script` (it IS the engine), `--checkdigit*`, `--encode*`,
  `--decode*`, `--encrypt*`, `--decrypt*`, `--compress`,
  `--decompress`, `--hash`, `--archive`, `--extract`, `--sample*`,
  `--jwt-*`, `--md-to-html`, `--md-to-pdf`, `--html-to-pdf`,
  `--compare` (the script binding is `compare(a, b)` over Blobs;
  the URL-load logic is CLI-only).
- **Pre-clap argv expansion**: `--config / -K`, `--input-file`,
  `--disable / -q`, `--next` (none make sense per-call inside a
  script).
- **Process-level redirects**: `--stderr` (rewires stderr globally;
  per-call wouldn't help). The opts key may exist for completeness
  but document it as no-op.

Anything else MUST have an opts key. When in doubt: add it.

**New binding modules**: new feature areas get a new
`bindings/<name>.rs` registered from `src/script/engine.rs`.

**Demo scripts**: any flag that's interesting enough to demo on the
CLI deserves a demo script too. Put it under `script/<area>.rhai`.
`tests/script_examples_it.rs` verifies every `.rhai` parses, so a
broken demo is caught at `cargo test` time.

**Verification (do this every commit)**:

```sh
# Every CLI field name should appear in build_args:
grep -oE 'pub [a-z_]+:' src/cli.rs | awk '{print $2}' | sed 's/://' \
  | while read f; do
      grep -q "\"$f\"" src/script/bindings/http.rs || \
        echo "MISSING opts key: $f"
    done
```

(Acceptable misses: the mode-selecting / pre-clap / stderr exclusion
list above. Anything else is a violation.)

### 5. Documentation trio

Three files **every release**:

- `CHANGELOG.md` under a new `## [X.Y.Z] - YYYY-MM-DD` heading, using
  keep-a-changelog subsections (`### Added`, `### Changed`, `### Fixed`,
  `### Removed`, `### Deprecated`, `### Security`).
- `HISTORY.md` numbered entry covering design rationale for non-trivial
  changes (new crate chosen and why, new module laid out, cross-cutting
  refactor, scope reductions). Routine flag additions don't need a
  HISTORY entry — the changelog is enough.
- `OUT-OF-SCOPE.md` — remove items that shipped, add new ones that
  were deliberately punted (with the reason and the right bucket:
  Waiting / Deferred / Not yet supported / Out of scope).

### 6. Manual — `docs/MANUAL.md` + regenerated PDF

Every code change that touches a user-visible flag, binding, function
signature, or behaviour MUST update `docs/MANUAL.md` in the same
release.

- **New CLI flag**: add it to the relevant Part II section's table,
  plus at least one example showing how to use it.
- **New / changed script binding function**: add its signature to
  the relevant Part III section's function table, plus at least one
  example (prefer 2–3 for non-trivial functions).
- **New binding module**: new Part III section, with intro paragraph
  + function table + multiple examples.
- **Behaviour change in an existing flag/binding**: update the
  description, and add a note if the change is not backwards-compatible.
- **New help topic, example section, or script example**: optional
  cross-link from the manual (the manual is not a duplicate of
  `--help` or `--examples` — it complements them), but at minimum
  make sure the relevant Part II / III section covers the underlying
  feature.

After updating the markdown, **regenerate the PDF**:

```sh
./target/release/recon --md-to-pdf docs/MANUAL.md \
    --toc --toc-depth 3 --gfm \
    --unsafe-html --page-break-on-h1 \
    --doc-title 'recon User Manual' \
    -o docs/MANUAL.pdf
```

The `--unsafe-html` flag is required because the manual uses a styled
`<div class="cover">` block for its title page. `--page-break-on-h1`
gives every top-level `#` heading its own PDF page.

Requires `agent-browser` on PATH. The PDF is committed alongside the
markdown — both files are checked in.

Bump the **Version:** and **Release date:** lines at the top of
`docs/MANUAL.md` to match the current `Cargo.toml` version +
`RELEASE_DATE`.

Docs-only changes (manual rewording, new example, typo fix) get a
PATCH bump per the version-bumping rule. **Always regenerate the PDF
so the binary artifact never drifts from the markdown source.**

---

## Pre-commit checklist — walk this for every change

Before `git commit`, mentally trace each new flag / binding / behaviour
change through this list. If any line is unchecked, fix it first.

For a **new CLI flag** in `src/cli.rs`:

- [ ] Doc comment first sentence is ≤ 52 chars and self-contained.
- [ ] `value_name` set when the default uppercase looks ugly.
- [ ] Field name appears in `src/script/bindings/http.rs::build_args`
      as a snake_case opts key (unless on the exclusion list above).
- [ ] `recon --help <topic>` covers the flag (new or extended topic).
- [ ] `recon --examples` shows at least one usage.
- [ ] `recon --flags` line reads cleanly (`recon --flags | grep
      <flag>`).
- [ ] `docs/MANUAL.md` Part II table + an example.
- [ ] `CHANGELOG.md` `## [X.Y.Z]` entry under `### Added` (or
      `### Changed`).
- [ ] PDF regenerated (`docs/MANUAL.pdf`).
- [ ] Version bumped (PATCH for docs-only, MINOR for new flag, MAJOR
      for breaking).

For a **new script binding function** in `src/script/bindings/<area>.rs`:

- [ ] Function registered in `src/script/engine.rs`.
- [ ] At least one demo invocation in a `script/<area>.rhai` file.
      `tests/script_examples_it.rs` will fail-fast if the .rhai
      doesn't parse.
- [ ] `recon --help script` (or topic-specific help) covers the
      binding.
- [ ] `docs/MANUAL.md` Part III table row + example.

For **any new file under `script/*.rhai`** (whether or not it
introduces a new binding module):

- [ ] `script/README.md` lists the new file in the appropriate
      section table. `tests/script_examples_it.rs::readme_indexes_every_script`
      enforces this — every `script/*.rhai` must have a corresponding
      row. The "new binding module" checklist below repeats the
      requirement, but this rule fires earlier (i.e. for any
      `ai-*.rhai`, `http-*.rhai`, additional probes, etc., not just
      first-of-its-kind).

For a **new binding module**:

- [ ] All of the above PLUS:
- [ ] `src/script/bindings/mod.rs` `pub mod <name>;` declaration.
- [ ] `src/script/engine.rs::build_engine` calls `<name>::register(…)`.
- [ ] New `script/<area>.rhai` shipping at minimum one usage.
- [ ] `script/README.md` lists the new script.

For a **HISTORY-worthy change** (new crate, new module, cross-cutting
refactor, scope reduction during implementation):

- [ ] `HISTORY.md` numbered entry with rationale (why this approach,
      what was rejected, what's deferred).

---

## Why this is structured as a checklist

The original exposure policy was prose — "every feature must reach
every surface". That's the right principle but easy to skim past.
The 0.61.0–0.66.0 Waiting-arc shipped 90 CLI flags and the script
engine sat untouched the whole time, requiring a 0.66.1 catch-up
release to plumb 60 opts-map keys retroactively.

The checklist above is the operational version of the principle: walk
it line-by-line for every change, treat unchecked boxes as blockers,
and never accept "we'll catch it up later" as an answer to a missing
surface.

---

## Post-merge hygiene — rebuild local artifacts after merging a worktree

After merging a worktree branch into master, the source / `Cargo.toml` /
`CHANGELOG.md` / `docs/MANUAL.md` / `docs/MANUAL.pdf` all arrive on
master cleanly via the merge commit — those are tracked in git. **The
compiled binary at `target/release/recon` is not tracked in git, and is
not updated by the merge.** It still reflects whatever was last built
directly inside the master checkout, which is usually one or more
releases behind: the worktree built its binary inside its own
`target/`, which got wiped by `ExitWorktree --remove`.

This matters because:

- `./target/release/recon --version` reports the OLD version, not the
  one that just shipped.
- Re-running `./target/release/recon --md-to-pdf docs/MANUAL.md ...` on
  master would use the stale binary to regenerate the PDF (probably
  harmless today, but a future change to the renderer would silently
  produce inconsistent output).
- Any further local `recon` invocations through `target/release/` are
  testing yesterday's code.

### Required after `git push origin master`

Run from the master checkout (the main repo dir, not a worktree):

```sh
cargo build --release                                          # default-feature build (includes ssh)
cargo build --release --no-default-features --features impersonate  # impersonate variant (drops ssh — see issue #1)
cargo test  --release                | tail -3                 # default suite
cargo test  --release --no-default-features --features impersonate --test impersonate_it | tail -3  # feature-gated
./target/release/recon --version                               # sanity: new version + date
```

Both feature variants must be warning-free. Both test runs must pass.
Skipping the impersonate build means a regression there ships silently
because the default build doesn't compile `src/impersonate.rs`.

The impersonate commands **must** carry `--no-default-features`. With
the default `ssh` feature on, libssh2 (OpenSSL) and BoringSSL (wreq)
collide at link time and the build fails — that's GitHub issue #1.
`--no-default-features` drops `ssh`, leaving a single SSL backend.

If the merge included changes to anything the binary itself produces
as a build artifact (currently only `docs/MANUAL.pdf`, regenerated by
running `recon --md-to-pdf`), the PDF should already be up to date
from the worktree's release commit — no second regen needed. If for
any reason it isn't (e.g. you suspect drift), regenerate from master
with the same command the worktree's release commit used and amend
in a quick patch commit:

```sh
./target/release/recon --md-to-pdf docs/MANUAL.md \
    --toc --toc-depth 3 --gfm \
    --unsafe-html --page-break-on-h1 \
    --doc-title 'recon User Manual' \
    -o docs/MANUAL.pdf
```

### Worktree base ref — `EnterWorktree` branches from origin, not local master

A trap closely related to the binary-staleness one above: when you
create a new worktree right after merging-but-before-pushing, the
worktree branches from `origin/<default-branch>`, NOT from local
master. The new branch is therefore *behind* local master by exactly
the commits you just merged but haven't pushed yet.

You'll notice when `git log --oneline -3` inside the new worktree
shows a tip that pre-dates your last release.

Two ways to avoid the trap:

1. **Push first, then create the worktree** — the simplest fix.
   After every merge, run `git push origin master` *before* opening
   a fresh worktree. Then the worktree's base ref is up to date by
   construction.

2. **Fast-forward inside the new worktree** — if you've already
   entered:

   ```sh
   git merge --ff-only master
   ```

   This brings local master's commits into the worktree branch. If
   there are no divergent commits (and there shouldn't be — you just
   created the branch), it's a clean fast-forward.

The first option is preferred because it also satisfies the
"target/release/recon is now in sync" rebuild rule above: by the
time the new worktree exists, both origin/master and target/release
are at the latest release.

### Why this is its own rule

The worktree workflow makes it easy to forget: the build, the tests,
and the PDF regen all happened cleanly inside the worktree, so it
*feels* like everything is ready. But the worktree's `target/` lives
under `.claude/worktrees/<branch>/target/`, and `ExitWorktree --remove`
deletes the whole directory — binary included. The master checkout's
`target/release/recon` was last built whenever you last invoked cargo
directly on master, which may have been several releases ago.

The fix is one `cargo build --release` after every push (plus the
`--features impersonate` variant). Cheap, and the alternative is
investigating why `recon --version` lies later.
