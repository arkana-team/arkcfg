use std::process::Command;

use crate::errors::{ArkcfgError, Result};

/// Represents a connected display output with its supported modes.
#[derive(Debug, Clone)]
pub struct Display {
    pub name: String,
    pub is_primary: bool,
    pub current_resolution: Option<String>,
    pub supported_resolutions: Vec<String>,
}

/// Parses the output of `xrandr` and returns a list of connected displays.
pub fn get_displays() -> Result<Vec<Display>> {
    let output = Command::new("xrandr")
        .output()
        .map_err(|e| ArkcfgError::CommandFailed(format!("failed to run xrandr: {}", e)))?;

    if !output.status.success() {
        return Err(ArkcfgError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_xrandr_output(&stdout)
}

/// Internal parser for `xrandr` stdout.
fn parse_xrandr_output(output: &str) -> Result<Vec<Display>> {
    let mut displays: Vec<Display> = Vec::new();
    let mut current: Option<Display> = None;

    for line in output.lines() {
        // A display header line looks like: "HDMI-1 connected primary 1920x1080+0+0 ..."
        // or: "VGA-1 connected 1280x1024+0+0 ..."
        // or: "HDMI-2 disconnected ..."
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() >= 2 && (parts[1] == "connected" || parts[1] == "disconnected") {
            // Save previous display
            if let Some(d) = current.take() {
                displays.push(d);
            }

            // Only track connected displays
            if parts[1] == "connected" {
                let name = parts[0].to_string();
                let is_primary = parts.contains(&"primary");

                // Current resolution may appear in the header line itself
                let current_resolution = parts.iter().find_map(|s| {
                    if s.contains('x') && s.contains('+') {
                        // e.g. "1920x1080+0+0" — strip offset
                        s.split('+').next().map(|r| r.to_string())
                    } else {
                        None
                    }
                });

                current = Some(Display {
                    name,
                    is_primary,
                    current_resolution,
                    supported_resolutions: Vec::new(),
                });
            }
        } else if let Some(ref mut display) = current {
            // Resolution mode lines are indented, e.g. "   1920x1080     60.00*+  50.00"
            if line.starts_with(' ') || line.starts_with('\t') {
                if let Some(res) = parts.first() {
                    if res.contains('x') {
                        display.supported_resolutions.push(res.to_string());
                    }
                }
            }
        }
    }

    // Push the last display
    if let Some(d) = current {
        displays.push(d);
    }

    if displays.is_empty() {
        return Err(ArkcfgError::NoDisplaysFound);
    }

    Ok(displays)
}

/// Validates a resolution string of the form WIDTHxHEIGHT.
fn validate_resolution(resolution: &str) -> Result<(u32, u32)> {
    let parts: Vec<&str> = resolution.split('x').collect();
    if parts.len() != 2 {
        return Err(ArkcfgError::InvalidResolution(resolution.to_string()));
    }
    let w = parts[0]
        .parse::<u32>()
        .map_err(|_| ArkcfgError::InvalidResolution(resolution.to_string()))?;
    let h = parts[1]
        .parse::<u32>()
        .map_err(|_| ArkcfgError::InvalidResolution(resolution.to_string()))?;
    Ok((w, h))
}

/// Lists all connected displays and their supported resolutions.
pub fn cmd_list() -> Result<()> {
    let displays = get_displays()?;
    for d in &displays {
        let primary_tag = if d.is_primary { " [primary]" } else { "" };
        let current_tag = d
            .current_resolution
            .as_deref()
            .map(|r| format!(" (current: {})", r))
            .unwrap_or_default();
        println!("Display: {}{}{}", d.name, primary_tag, current_tag);
        if d.supported_resolutions.is_empty() {
            println!("  (no modes reported)");
        } else {
            for res in &d.supported_resolutions {
                println!("  {}", res);
            }
        }
        println!();
    }
    Ok(())
}

/// Shows the current resolution of all connected displays.
pub fn cmd_current() -> Result<()> {
    let displays = get_displays()?;
    for d in &displays {
        let res = d
            .current_resolution
            .as_deref()
            .unwrap_or("(unknown)");
        let primary_tag = if d.is_primary { " [primary]" } else { "" };
        println!("{}{}: {}", d.name, primary_tag, res);
    }
    Ok(())
}

/// Sets the resolution of a display via xrandr.
pub fn cmd_set(resolution: &str, output: Option<&str>) -> Result<()> {
    // Validate format
    validate_resolution(resolution)?;

    let displays = get_displays()?;

    // Pick target display
    let target = if let Some(name) = output {
        displays
            .iter()
            .find(|d| d.name == name)
            .ok_or_else(|| ArkcfgError::DisplayNotFound(name.to_string()))?
    } else {
        // Default: primary display, or first connected
        displays
            .iter()
            .find(|d| d.is_primary)
            .or_else(|| displays.first())
            .ok_or(ArkcfgError::NoDisplaysFound)?
    };

    // Check the resolution is supported
    if !target
        .supported_resolutions
        .iter()
        .any(|r| r == resolution)
    {
        return Err(ArkcfgError::UnsupportedResolution(resolution.to_string()));
    }

    println!(
        "Setting display '{}' to resolution {}...",
        target.name, resolution
    );

    let status = Command::new("xrandr")
        .args(["--output", &target.name, "--mode", resolution])
        .status()
        .map_err(|e| ArkcfgError::CommandFailed(format!("failed to run xrandr: {}", e)))?;

    if !status.success() {
        return Err(ArkcfgError::CommandFailed(format!(
            "xrandr --output {} --mode {} failed",
            target.name, resolution
        )));
    }

    println!("Done.");
    Ok(())
}
