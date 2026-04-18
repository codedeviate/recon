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

For changes with notable design rationale (new crate chosen and why, new
module laid out, cross-cutting refactor), also add a numbered entry to
the `## Feature Additions` section of `HISTORY.md` or a subsection under
`## Architecture Decisions`. Routine bug fixes and flag additions do not
require a HISTORY.md entry — the changelog is enough.
