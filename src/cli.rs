use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "arkcfg",
    author = "arkanaOS Team",
    // Take the version from Cargo.toml so it cannot drift from the package
    // version (it previously appeared in Cargo.toml, here, and Cargo.lock).
    version,
    about = "System configuration tool for arkanaOS"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Display resolution settings
    Display {
        #[command(subcommand)]
        action: DisplayAction,
    },

    /// Show system information (via fastfetch/neofetch)
    Sysinfo,

    /// Power management options
    Power {
        #[command(subcommand)]
        action: PowerAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum DisplayAction {
    /// List all connected displays and their supported resolutions
    List,

    /// Set the resolution of a display
    Set {
        /// Resolution in WIDTHxHEIGHT format (e.g. 1920x1080).
        /// Custom mode names reported by `display list` are also accepted.
        resolution: String,

        /// Target display output (e.g. HDMI-1, VGA-1). Defaults to the primary display.
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Show the current resolution of all connected displays
    Current,
}

#[derive(Subcommand, Debug)]
pub enum PowerAction {
    /// Restart the system
    Restart {
        /// Do not ask for confirmation (for scripts)
        #[arg(short = 'y', long)]
        no_confirm: bool,
    },

    /// Power off the system
    Poweroff {
        /// Do not ask for confirmation (for scripts)
        #[arg(short = 'y', long)]
        no_confirm: bool,
    },
}
