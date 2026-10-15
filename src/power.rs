use std::process::Command;

use crate::errors::{ArkcfgError, Result};

/// Restarts the system via systemctl, then falls back to reboot(8).
pub fn cmd_restart() -> Result<()> {
    println!("Restarting system...");
    attempt_power_cmd(&["systemctl", "reboot"]).or_else(|_| attempt_power_cmd(&["reboot"]))
}

/// Powers off the system via systemctl, then falls back to poweroff(8).
pub fn cmd_poweroff() -> Result<()> {
    println!("Powering off system...");
    attempt_power_cmd(&["systemctl", "poweroff"]).or_else(|_| attempt_power_cmd(&["poweroff"]))
}

/// Tries to execute a power command. Returns Ok if spawned, Err otherwise.
fn attempt_power_cmd(args: &[&str]) -> Result<()> {
    let (program, rest) = args.split_first().expect("args must not be empty");

    let status = Command::new(program)
        .args(rest)
        .status()
        .map_err(|e| ArkcfgError::CommandFailed(format!("failed to run '{}': {}", program, e)))?;

    if status.success() {
        Ok(())
    } else {
        Err(ArkcfgError::CommandFailed(format!(
            "'{}' exited with status {}",
            args.join(" "),
            status
        )))
    }
}
