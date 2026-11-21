use std::io::{self, Write};
use std::process::Command;

use crate::errors::{ArkcfgError, Result};

/// Restarts the system via systemctl, then falls back to reboot(8).
pub fn cmd_restart(no_confirm: bool) -> Result<()> {
    confirm_action("Restart", no_confirm)?;
    println!("Restarting system...");
    power_off_or_restart(&["systemctl", "reboot"], &["reboot"])
}

/// Powers off the system via systemctl, then falls back to poweroff(8).
pub fn cmd_poweroff(no_confirm: bool) -> Result<()> {
    confirm_action("Power off", no_confirm)?;
    println!("Powering off system...");
    power_off_or_restart(&["systemctl", "poweroff"], &["poweroff"])
}

/// Runs the primary command, falling back to the secondary one.
///
/// If both fail, both errors are reported: the primary one explains why the
/// fallback was needed, and the secondary one explains why nothing worked.
fn power_off_or_restart(primary: &[&str], fallback: &[&str]) -> Result<()> {
    match attempt_power_cmd(primary) {
        Ok(()) => Ok(()),
        Err(primary_error) => attempt_power_cmd(fallback).map_err(|fallback_error| {
            ArkcfgError::CommandFailed(format!(
                "{}; fallback failed too: {}",
                primary_error, fallback_error
            ))
        }),
    }
}

/// Tries to execute a power command. Returns Ok if it succeeded.
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

/// Asks for confirmation before an irreversible action.
///
/// `arkcfg power` is reachable from a terminal by anyone at the keyboard, so a
/// typo (`arkcfg power restart`) would otherwise reboot the machine with no way
/// back. `--no-confirm` exists for scripts and the default is "no".
fn confirm_action(action: &str, no_confirm: bool) -> Result<()> {
    if no_confirm {
        return Ok(());
    }

    print!("{} the system? [y/N] ", action);
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;

    if matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        Ok(())
    } else {
        // Cancelling is a deliberate choice, not a failure; note it on stderr and
        // exit successfully so scripts that check the status are not misled.
        eprintln!("{} cancelled.", action.to_lowercase());
        Ok(())
    }
}
