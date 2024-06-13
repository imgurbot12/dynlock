///! CLI Definitions for Dynlock
use clap::Parser;

/// Dynamic and Configurable Wayland Lockscreen
///
/// Generate and Render Custom Lockscreens using GLSL
/// Shaders with a Specified Background or Live Screenshot.
#[derive(Debug, Clone, Parser)]
#[clap(name = "dynlock", author = "Andrew Scott <imgurbot12@gmail.com>")]
pub struct Cli {
    /// Dynlock configuration filepath
    ///
    /// Defaults to `$XDG_CONFIG_DIR/dynlock/config.yaml` (if present)
    #[clap(short, long)]
    pub config: Option<String>,
    /// Fragment shader file/search-directory
    ///
    /// Defaults to `$XDG_CONFIG_DIR/dynlock/shaders` directory
    #[clap(short, long)]
    pub shader: Option<String>,
    /// Background image/search-directory
    ///
    /// Defaults to a Live Screenshot of the current screen
    #[clap(short, long)]
    pub background: Option<String>,
    /// Screensaver mode does not lock
    ///
    /// The Default Mode without A Configuration File is Lock Mode
    #[clap(long)]
    pub screensave: Option<bool>,
    /// Fork and daemonize process if enabled
    ///
    /// Useful for preventing more than once instance from running at once
    #[clap(short = 'f', long)]
    pub daemonize: bool,
    /// Optional Logfile for Logging Output
    #[clap(short, long)]
    pub logfile: Option<String>,
}
