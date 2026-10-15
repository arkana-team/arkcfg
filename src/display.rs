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
                match parts.first() {
                    Some(res) if res.contains('x') => {
                        display.supported_resolutions.push(res.to_string());
                    }
                    _ => {}
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
    if w == 0 || h == 0 {
        return Err(ArkcfgError::InvalidResolution(resolution.to_string()));
    }
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
        let res = d.current_resolution.as_deref().unwrap_or("(unknown)");
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
    if !target.supported_resolutions.iter().any(|r| r == resolution) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_connected_outputs_and_modes() {
        let output = r#"Screen 0: minimum 8 x 8, current 2560 x 1080, maximum 32767 x 32767
eDP-1 connected primary 1920x1080+0+0 (normal left inverted right x axis y axis) 309mm x 174mm
   1920x1080     60.00*+  59.93
   1280x720      60.00
HDMI-1 disconnected (normal left inverted right x axis y axis)
DP-1 connected 640x480+1920+0 (normal left inverted right x axis y axis) 0mm x 0mm
   640x480       59.94*+
"#;

        let displays = parse_xrandr_output(output).unwrap();
        assert_eq!(displays.len(), 2);
        assert_eq!(displays[0].name, "eDP-1");
        assert!(displays[0].is_primary);
        assert_eq!(displays[0].current_resolution.as_deref(), Some("1920x1080"));
        assert_eq!(displays[0].supported_resolutions, ["1920x1080", "1280x720"]);
        assert_eq!(displays[1].name, "DP-1");
        assert!(!displays[1].is_primary);
    }

    #[test]
    fn rejects_output_without_connected_displays() {
        let error =
            parse_xrandr_output("Screen 0: minimum 8 x 8\nHDMI-1 disconnected\n").unwrap_err();
        assert!(matches!(error, ArkcfgError::NoDisplaysFound));
    }

    #[test]
    fn resolution_dimensions_must_be_positive() {
        assert_eq!(validate_resolution("1920x1080").unwrap(), (1920, 1080));
        assert!(validate_resolution("0x1080").is_err());
        assert!(validate_resolution("1920x0").is_err());
        assert!(validate_resolution("1920x1080x60").is_err());
    }
}
