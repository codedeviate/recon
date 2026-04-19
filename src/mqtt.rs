//! MQTT client: probe, publish, subscribe.
//!
//! Dispatched from `main.rs` on `mqtt://` and `mqtts://` URLs. See the
//! design document at:
//! /Users/thomas/Development/Starweb/superpowers/recon/specs/2026-04-19-mqtt-support-design.md

use anyhow::{anyhow, Result};

use crate::cli::Args;

pub fn run(_url: &str, _args: &Args) -> Result<()> {
    Err(anyhow!("mqtt: not yet implemented"))
}
