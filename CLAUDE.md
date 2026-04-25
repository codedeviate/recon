# recon — Claude Code Instructions

## Version Bumping

After every change, update the version in `Cargo.toml` according to these rules:

- **PATCH** (`0.x.N+1`): bug fixes, help/documentation text updates, minor changes that don't add or remove flags or features.
- **MINOR** (`0.N+1.0`): new flag or feature added, existing flag removed or significantly changed.
- **MAJOR** (`N+1.0.0`): breaking changes to existing behaviour (reserved, rare).

Update `Cargo.toml` and add an entry to `CHANGELOG.md` under a new
`## [X.Y.Z] - YYYY-MM-DD` heading at the top of the log (above older
versions, under `## [Unreleased]` if present). Group entries into the
keep-a-changelog subsections: `### Added`, `### Changed`, `### Fixed`,
`### Removed`, `### Deprecated`, `### Security`.

Also update `RELEASE_DATE` in `src/version.rs` to today's date
(`YYYY-MM-DD`) so `recon --version` reports the actual ship date of the
current build. Keep it in sync with the CHANGELOG heading for the new
version.

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
