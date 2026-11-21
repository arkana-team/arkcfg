use std::io::{ErrorKind, IsTerminal};
use std::process::Command;

use crate::errors::{ArkcfgError, Result};

/// Runs fastfetch if available, falls back to neofetch, then to uname.
pub fn cmd_sysinfo() -> Result<()> {
    // Try fastfetch first (arkanaOS ships it), then fall back to neofetch.
    // A missing binary is the normal case that should trigger the fallback; any
    // other failure (e.g. permission denied) is reported instead of silently
    // cascading, so a broken install is visible.
    for command in ["fastfetch", "neofetch"] {
        match Command::new(command).status() {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => {
                return Err(ArkcfgError::CommandFailed(format!(
                    "'{}' exited with status {}",
                    command, status
                )));
            }
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(ArkcfgError::CommandFailed(format!(
                    "failed to run '{}': {}",
                    command, e
                )));
            }
        }
    }

    // Last resort: print basic kernel info. Warn on stderr only when it is a
    // terminal, so the message cannot corrupt piped output.
    if std::io::stderr().is_terminal() {
        eprintln!("warning: fastfetch and neofetch are not available. Falling back to uname.");
    }
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
