use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArkcfgError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("'xrandr' is not installed. Install it with: sudo pacman -S xorg-xrandr")]
    XrandrMissing,

    #[error(
        "No displays found. Is a display server running and is DISPLAY set?\n\
         hint: run `xrandr` to confirm the X server is reachable"
    )]
    NoDisplaysFound,

    #[error("Invalid resolution format '{0}'. Expected WIDTHxHEIGHT (e.g. 1920x1080)")]
    InvalidResolution(String),

    #[error("Resolution '{resolution}' is not supported by display '{display}'. Supported modes: {supported}")]
    UnsupportedResolution {
        resolution: String,
        display: String,
        supported: String,
    },

    #[error("Display '{0}' not found. Use `arkcfg display list` to see connected outputs")]
    DisplayNotFound(String),
}

pub type Result<T> = std::result::Result<T, ArkcfgError>;
