use std::io::ErrorKind;
use std::process::Command;

use crate::errors::{ArkcfgError, Result};

/// Represents a connected display output with its supported modes.
#[derive(Debug, Clone)]
pub struct Display {
    pub name: String,
    pub is_primary: bool,
    pub current_resolution: Option<String>,
    /// Mode *names* as reported by `xrandr` (e.g. `1920x1080` or a custom
    /// `1920x1080_60.00`). These are what `xrandr --output ... --mode <name>`
    /// accepts, so they are what we validate against.
    pub supported_resolutions: Vec<String>,
}

/// Parses the output of `xrandr` and returns a list of connected displays.
pub fn get_displays() -> Result<Vec<Display>> {
    let output = Command::new("xrandr").output().map_err(|e| {
        if e.kind() == ErrorKind::NotFound {
            // A missing `xrandr` is the most common cause here, and reporting it
            // as "No displays found. Is a display server running?" sent users
            // looking in the wrong place.
            ArkcfgError::XrandrMissing
        } else {
            ArkcfgError::CommandFailed(format!("failed to run xrandr: {}", e))
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        return Err(ArkcfgError::CommandFailed(if stderr.is_empty() {
            format!("xrandr exited with status {}", output.status)
        } else {
            stderr.to_string()
        }));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_xrandr_output(&stdout)
}

/// Parses a single mode line such as `   1920x1080     60.00*+  59.94`.
///
/// `xrandr` marks the active mode with `*` and the preferred mode with `+`.
/// Custom modes added with `--newmode` get an extra description line, e.g.
/// `   1920x1080_60.00 (0x2d4) 173.00MHz +HSync -VSync *current +preferred`,
/// whose leading token is a valid mode name, while the two indented lines that
/// follow it (`h: width ...`, `v: height ...`) are not.
///
/// A mode name starts with `<width>x<height>` and may carry a custom suffix
/// (`_60.00`, `i`, ...). Requiring that shape, plus a following refresh-rate
/// token, keeps the description lines out of the mode list.
fn parse_mode_line(line: &str) -> Option<(&str, bool)> {
    let mut parts = line.split_whitespace();
    let name = parts.next()?;

    // First token must be WIDTHxHEIGHT optionally followed by a suffix such as
    // "_60.00". Checking the shape (rather than merely requiring an 'x') also
    // excludes property lines like "-HSync: ..." and "h: width ...".
    let (width, rest) = name.split_once('x')?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if width.is_empty()
        || !width.chars().all(|c| c.is_ascii_digit())
        || digits.is_empty()
        || rest[digits.len()..].starts_with('.')
    {
        return None;
    }

    // A real mode line always carries at least one refresh-rate token; a line
    // with only a mode name is not something `xrandr` prints.
    let mut saw_rate = false;
    let mut is_current = false;
    for rate in parts {
        saw_rate = true;
        if rate.contains('*') {
            is_current = true;
        }
    }
    if !saw_rate {
        return None;
    }

    Some((name, is_current))
}

/// Internal parser for `xrandr` stdout.
fn parse_xrandr_output(output: &str) -> Result<Vec<Display>> {
    let mut displays: Vec<Display> = Vec::new();
    let mut current: Option<Display> = None;

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();

        // A display header line looks like one of:
        //   "HDMI-1 connected primary 1920x1080+0+0 (normal left ...) 509mm x 286mm"
        //   "DP-2 connected (normal left inverted right x axis y axis)"
        //   "VGA-1 disconnected (normal left inverted right x axis y axis)"
        //   "TV-1 unknown connection (normal left inverted right x axis y axis)"
        //
        // The connection state is the second token. `xrandr` prints
        // "unknown connection" for outputs whose connection status cannot be
        // determined; such outputs still expose modes, and the following mode
        // lines must not be attributed to the previous display.
        let connection = match parts.get(1).copied() {
            Some("connected") => Some(true),
            Some("disconnected") => Some(false),
            Some("unknown") if parts.get(2) == Some(&"connection") => Some(false),
            _ => None,
        };

        if let Some(is_connected) = connection {
            // A new output always ends the previous one. This includes outputs
            // we skip, so their modes are never attached to the display above.
            if let Some(d) = current.take() {
                displays.push(d);
            }

            if is_connected {
                let is_primary = parts.contains(&"primary");

                // When the output is enabled, the header carries a token such as
                // "1920x1080+0+0" or "1920x1080+1920+0"; the current resolution
                // is the part before the offset. A connected-but-disabled output
                // simply has no such token.
                let current_resolution = parts
                    .iter()
                    .find(|s| s.contains('x') && s.contains('+'))
                    .map(|s| s.split('+').next().unwrap_or_default().to_string());

                current = Some(Display {
                    name: parts[0].to_string(),
                    is_primary,
                    current_resolution,
                    supported_resolutions: Vec::new(),
                });
            }
        } else if let Some(display) = current.as_mut() {
            // Mode lines are indented; everything else belongs to the state
            // flags above this output (e.g. the trailing "Broadcast RGB: ..."
            // properties).
            if !(line.starts_with(' ') || line.starts_with('\t')) {
                continue;
            }

            let Some((name, is_current)) = parse_mode_line(line) else {
                continue;
            };
            // The '*' marker is authoritative for the active mode: the header
            // token only carries the resolution size, so for a custom mode such
            // as "1920x1080_60.00" the header still says "1920x1080+0+0".
            if is_current {
                display.current_resolution = Some(name.to_string());
            }
            if !display
                .supported_resolutions
                .iter()
                .any(|mode| mode == name)
            {
                display.supported_resolutions.push(name.to_string());
            }
        }
    }

    if let Some(d) = current {
        displays.push(d);
    }

    if displays.is_empty() {
        return Err(ArkcfgError::NoDisplaysFound);
    }

    Ok(displays)
}

/// Validates a mode name beginning with WIDTHxHEIGHT, returning the dimensions.
/// Xrandr custom modes may append a suffix such as `_60.00` or `i`; the caller
/// checks the full name against the modes reported for the selected display.
fn validate_resolution(resolution: &str) -> Result<(u32, u32)> {
    let invalid = || ArkcfgError::InvalidResolution(resolution.to_string());

    let (width, raw_height) = resolution.split_once('x').ok_or_else(invalid)?;
    // `u32::from_str` accepts a leading '+' or '-' and surrounding digits only,
    // so "+1920" would otherwise be accepted as 1920. Reject any sign so the
    // error message stays accurate.
    if width.starts_with(['+', '-']) || raw_height.starts_with(['+', '-']) {
        return Err(invalid());
    }
    let width = width.parse::<u32>().map_err(|_| invalid())?;
    let height_len = raw_height
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .map(char::len_utf8)
        .sum::<usize>();
    if height_len == 0 {
        return Err(invalid());
    }
    let (height_digits, suffix) = raw_height.split_at(height_len);
    // A second `x` is malformed (`1920x1080x60`), while other suffixes are
    // allowed because custom mode names are defined by Xrandr and are checked
    // against the selected display's reported modes below.
    if suffix.starts_with('x') || suffix.starts_with('.') {
        return Err(invalid());
    }
    let height = height_digits.parse::<u32>().map_err(|_| invalid())?;
    if width == 0 || height == 0 {
        return Err(invalid());
    }

    Ok((width, height))
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

    if target.supported_resolutions.is_empty() {
        return Err(ArkcfgError::CommandFailed(format!(
            "display '{}' reports no modes (is it enabled in the X server?)",
            target.name
        )));
    }

    // Check the resolution is supported. `xrandr --mode` takes a mode *name*,
    // so compare against the full names and list the valid ones on failure.
    if !target.supported_resolutions.iter().any(|r| r == resolution) {
        return Err(ArkcfgError::UnsupportedResolution {
            resolution: resolution.to_string(),
            display: target.name.clone(),
            supported: target.supported_resolutions.join(", "),
        });
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
        assert_eq!(displays[1].current_resolution.as_deref(), Some("640x480"));
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

    #[test]
    fn resolution_dimensions_reject_signs_and_garbage() {
        // `u32::from_str` accepts a leading '+'; the CLI contract does not.
        assert!(validate_resolution("+1920x1080").is_err());
        assert!(validate_resolution("1920x+1080").is_err());
        assert!(validate_resolution("-1920x1080").is_err());
        assert!(validate_resolution("").is_err());
        assert!(validate_resolution("x1080").is_err());
        assert!(validate_resolution("1920x").is_err());
        assert!(validate_resolution("1920X1080").is_err());
        assert!(validate_resolution("1920x1080 ").is_err());
    }

    #[test]
    fn unknown_connection_modes_are_not_attributed_to_previous_display() {
        // Real xrandr output: a TV output in "unknown connection" state still
        // prints modes. Those must not leak onto the display above it.
        let output = r#"eDP-1 connected primary 1920x1080+0+0 (normal left inverted right x axis y axis) 309mm x 174mm
   1920x1080     60.02*+  59.94
TV-1 unknown connection (normal left inverted right x axis y axis)
   848x480       59.94 +
   640x480       59.94 +
"#;

        let displays = parse_xrandr_output(output).unwrap();
        assert_eq!(displays.len(), 1);
        assert_eq!(displays[0].name, "eDP-1");
        // The assertion above already proves the TV's 848x480/640x480 modes
        // did not leak onto eDP-1.
        assert_eq!(displays[0].supported_resolutions, ["1920x1080"]);
    }

    #[test]
    fn parses_custom_mode_names_and_skips_description_lines() {
        // `cvt 1920x1080 60` then `--newmode`/`--addmode` produces a mode whose
        // name is "1920x1080_60.00" followed by a mode-description line.
        let output = r#"eDP-1 connected primary 1920x1080+0+0 (normal left inverted right x axis y axis) 309mm x 174mm
   1920x1080_60.00 (0x2d4) 173.00MHz +HSync -VSync *current +preferred
        h: width  1920 start 2048 end 2248 total 2576 skew    0 clock  67.16KHz
        v: height 1080 start 1083 end 1088 total 1116           clock  60.18Hz
   1024x768      60.00 +
"#;

        let displays = parse_xrandr_output(output).unwrap();
        assert_eq!(
            displays[0].supported_resolutions,
            ["1920x1080_60.00", "1024x768"]
        );
        // The active mode name is taken from the mode line's '*' marker.
        assert_eq!(
            displays[0].current_resolution.as_deref(),
            Some("1920x1080_60.00")
        );
    }

    #[test]
    fn property_and_description_lines_are_not_parsed_as_modes() {
        // `xrandr -q` prints output properties after the mode list. Their
        // indented "supported:"/"width:" lines must not become modes.
        let output = r#"eDP-1 connected primary 1920x1080+0+0 (normal left inverted right x axis y axis) 309mm x 174mm
   1920x1080     60.02*+  59.94
   Broadcast RGB: Automatic
        supported: Automatic, Full, Limited 16:235
   audio: auto
        supported: force-dvi, off, auto, on
"#;

        let displays = parse_xrandr_output(output).unwrap();
        assert_eq!(displays[0].supported_resolutions, ["1920x1080"]);
        assert_eq!(displays[0].current_resolution.as_deref(), Some("1920x1080"));
    }

    #[test]
    fn disabled_output_has_no_current_resolution_but_keeps_modes() {
        let output = r#"DP-2 connected (normal left inverted right x axis y axis)
   2560x1440     59.95 +
"#;
        let displays = parse_xrandr_output(output).unwrap();
        assert_eq!(displays[0].current_resolution, None);
        assert_eq!(displays[0].supported_resolutions, ["2560x1440"]);
    }

    #[test]
    fn duplicate_refresh_rates_produce_one_mode_entry() {
        let output = r#"VGA-1 connected primary 1280x1024+0+0 (normal left inverted right x axis y axis) 359mm x 287mm
   1280x1024      60.0*+   75.0
   1280x1024      60.0
"#;
        let displays = parse_xrandr_output(output).unwrap();
        assert_eq!(displays[0].supported_resolutions, ["1280x1024"]);
    }

    #[test]
    fn header_without_active_mode_reports_unknown_current() {
        // Connected-but-mirrored/rotated outputs can omit the "+x+y" token.
        let output = "HDMI-1 connected (normal left inverted right x axis y axis)\n   1024x768      60.00 +\n";
        let displays = parse_xrandr_output(output).unwrap();
        assert_eq!(displays[0].current_resolution, None);
        assert_eq!(displays[0].supported_resolutions, ["1024x768"]);
    }
}
