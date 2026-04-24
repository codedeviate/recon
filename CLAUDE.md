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
update **all four** of the surfaces below in the same release. Missing
any one is a policy violation — fix it before the next release, even if
that means a patch release whose only job is closing the gap.

1. **`recon --help <topic>`**. Every new flag gets a `FlagHelp` entry in
   the relevant `TOPIC_*` static in `src/help.rs`. New feature areas get
   a whole new topic added to `topic_keys()` + `resolve_topic()`. Pick
   aliases generously (e.g. `charset` / `text` / `iconv` / `text-encoding`
   all routed to `TOPIC_TEXT_ENCODING`). Cross-reference from
   `TOPIC_PROTOCOLS` when a new URL scheme lands.

2. **`recon --examples`**. Every feature gets at least one `example()`
   block in `src/examples.rs` under a suitable `section()` — a new
   section if the feature is a new area, otherwise an addition to the
   closest existing one. Include a `note()` line calling out the
   non-obvious behaviour (defaults, detection priority, when the
   feature silently falls back).

3. **Script engine**. Features must be usable from Rhai scripts via
   `src/script/bindings/<area>.rs`. New functions or options map keys
   go in the relevant binding module; new feature areas get a new
   `bindings/<name>.rs` registered from `src/script/engine.rs`. Every
   CLI flag with a meaningful script equivalent gets a corresponding
   opts-map key (snake_case). If a CLI action is genuinely script-
   inappropriate (e.g. `--init` bootstrapping `~/.recon/`), document
   the exclusion in the plan's *Out of scope* section.

4. **Documentation**. Three files every release:
   - `CHANGELOG.md` under a new `## [X.Y.Z] - YYYY-MM-DD` heading,
     using the keep-a-changelog subsections.
   - `HISTORY.md` numbered entry covering design rationale for
     non-trivial changes.
   - `OUT-OF-SCOPE.md` — remove items that shipped, add new ones that
     were deliberately punted, with the reason.
   - For each newly-shipped script binding, add or extend a
     `script/<area>.rhai` example so the shipped script set stays in
     sync with the binding set. `tests/script_examples_it.rs` enforces
     that every `.rhai` file parses.

When a release is in progress and you notice one of the surfaces is
behind, stop and catch it up before the next feature lands. When
reviewing a diff before commit, mentally walk the four surfaces for
each new item and fix anything that's missing.

## Manual — every change also updates `docs/MANUAL.md` and regenerates the PDF

recon ships a long-form user manual alongside the built-in `--help`
and `--examples` surfaces:

- **`docs/MANUAL.md`** — the source document. Cover page, table of
  contents, then four parts: Getting started, CLI reference, Script
  engine, Appendices.
- **`docs/MANUAL.pdf`** — the rendered artifact, always produced from
  `docs/MANUAL.md` via recon itself.

**Policy — the manual is a first-class surface.** Every code change
that touches a user-visible flag, binding, function signature, or
behaviour MUST update `docs/MANUAL.md` in the same release. The four
surfaces from the exposure policy above (help / examples / script
engine / CHANGELOG+HISTORY+OUT-OF-SCOPE) become **five**; the manual
is the fifth. Missing it is a policy violation — fix it before the
next feature lands, even if that means a patch release whose only
job is closing the gap.

Specifically:

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

**Regenerate the PDF as soon as the markdown is updated.** Use recon
itself:

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

Requires `agent-browser` on PATH (Chrome-backed). The PDF is
committed alongside the markdown — both files are checked in.

When updating the manual, also bump the **Version:** and **Release
date:** lines at the top of `docs/MANUAL.md` to match the current
`Cargo.toml` version + `RELEASE_DATE` from `src/version.rs`.

If a change is docs-only (manual rewording, new example, typo fix),
that's a PATCH bump per the version-bumping rule above; always
regenerate the PDF so the binary artifact never drifts from the
markdown source.
