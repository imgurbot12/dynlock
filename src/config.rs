///! Lockscreen Configuration Settings
use std::path::PathBuf;

/// Lockscreen Configuration Settings
pub struct Settings {
    pub shader: String,
    pub background: Option<PathBuf>,
}
