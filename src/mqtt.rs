//! MQTT client: probe, publish, subscribe.
//!
//! Dispatched from `main.rs` on `mqtt://` and `mqtts://` URLs. Three modes
//! gated by CLI flags: probe (default), publish (with `-d` + topic in URL),
//! subscribe (with `--subscribe <filter>`).

use anyhow::{anyhow, Result};

use crate::cli::Args;

pub fn run(_url: &str, _args: &Args) -> Result<()> {
    Err(anyhow!("mqtt: not yet implemented"))
}
