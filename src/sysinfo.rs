use std::process::Command;

use crate::errors::{ArkcfgError, Result};

/// Runs fastfetch if available, falls back to neofetch, then to uname.
pub fn cmd_sysinfo() -> Result<()> {
    // Try fastfetch first (ArkanaOS ships it)
    if let Ok(status) = Command::new("fastfetch").status() {
        if status.success() {
            return Ok(());
        }
    }

    // Fall back to neofetch
    if let Ok(status) = Command::new("neofetch").status() {
        if status.success() {
            return Ok(());
        }
    }

    // Last resort: print basic kernel info
    eprintln!("warning: fastfetch and neofetch are not available. Falling back to uname.");
    let output = Command::new("uname")
        .arg("-a")
        .output()
        .map_err(|e| ArkcfgError::CommandFailed(format!("failed to run uname: {}", e)))?;

    println!("{}", String::from_utf8_lossy(&output.stdout).trim());
    Ok(())
}
