///! Lockscreen Configuration Settings
use std::path::PathBuf;

use serde::Deserialize;

#[inline]
fn _true() -> bool {
    true
}

/// Configuration Settings for Dynlock
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(default = "_true")]
    pub lock: bool,
    pub shader: Option<String>,
    pub background: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lock: true,
            shader: None,
            background: None,
        }
    }
}

/// Lockscreen Configuration Settings
pub struct Settings {
    pub lock: bool,
    pub shader: String,
    pub background: Option<PathBuf>,
}
