use std::process::Command;

use crate::errors::{ArkcfgError, Result};

/// Runs fastfetch if available, falls back to neofetch, then to uname.
pub fn cmd_sysinfo() -> Result<()> {
    // Try fastfetch first (ArkanaOS ships it), then fall back to neofetch.
    for command in ["fastfetch", "neofetch"] {
        match Command::new(command).status() {
            Ok(status) if status.success() => return Ok(()),
            _ => {}
        }
    }

    // Last resort: print basic kernel info
    eprintln!("warning: fastfetch and neofetch are not available. Falling back to uname.");
    let output = Command::new("uname")
        .arg("-a")
        .output()
        .map_err(|e| ArkcfgError::CommandFailed(format!("failed to run uname: {}", e)))?;

    if !output.status.success() {
        return Err(ArkcfgError::CommandFailed(format!(
            "uname exited with status {}",
            output.status
        )));
    }

    println!("{}", String::from_utf8_lossy(&output.stdout).trim());
    Ok(())
}
