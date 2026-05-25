//! Layered TOML config resolver — reads `/etc/recon/<name>.toml` (system)
//! and `~/.recon/<name>.toml` (user), deep-merges them with user winning,
//! and returns a single `toml::Value`. Used by `src/config.rs` and the gh
//! script binding.
//!
//! See `docs/MANUAL.md` "Configuration files" for the public model;
//! see `~/Development/Starweb/superpowers/recon/specs/2026-05-25-layered-config-design.md`
//! for design rationale.

#[allow(unused_imports)]
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct LayerOpts {
    pub skip_system:     bool,
    pub skip_user:       bool,
    pub system_override: Option<PathBuf>,
    pub user_override:   Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct Resolved {
    pub system: Option<PathBuf>,
    pub user:   Option<PathBuf>,
}
