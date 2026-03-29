# recon — Claude Code Instructions

## Version Bumping

After every change, update the version in `Cargo.toml` according to these rules:

- **PATCH** (`0.x.N+1`): bug fixes, help/documentation text updates, minor changes that don't add or remove flags or features.
- **MINOR** (`0.N+1.0`): new flag or feature added, existing flag removed or significantly changed.
- **MAJOR** (`N+1.0.0`): breaking changes to existing behaviour (reserved, rare).

Update `Cargo.toml` and record the change in `HISTORY.md` under a new numbered section at the top of the log.
