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
