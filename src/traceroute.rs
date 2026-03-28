use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use std::process::{Command, Stdio};

use crate::util::parse_target;

pub fn run(input: &str, max_hops: u8) -> Result<()> {
    let (host, port) = parse_target(input);

    println!("Traceroute to {}", host.bold());
    println!("{}", "═".repeat(50));

    #[cfg(target_os = "windows")]
    {
        let _ = port; // tracert doesn't support port selection
        let status = Command::new("tracert")
            .arg("-h")
            .arg(max_hops.to_string())
            .arg(&host)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("Could not run tracert — is it available on this system?")?;

        if !status.success() {
            return Err(anyhow!("tracert exited with status: {status}"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new("traceroute");
        cmd.arg("-m").arg(max_hops.to_string());
        if let Some(p) = port {
            cmd.arg("-p").arg(p.to_string());
        }
        cmd.arg(&host);
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        let status = cmd
            .status()
            .context("Could not run traceroute — is it installed?")?;

        if !status.success() {
            return Err(anyhow!("traceroute exited with status: {status}"));
        }
    }

    Ok(())
}
