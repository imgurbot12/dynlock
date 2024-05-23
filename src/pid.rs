//! PidLock Implementation

use std::fs::{remove_file, File};
use std::path::PathBuf;

use anyhow::{Context, Result};
use filelock_rs::FdLock;

pub struct PidLock {
    path: PathBuf,
    lock: File,
}

impl PidLock {
    pub fn new() -> Result<Self> {
        let path = xdg::BaseDirectories::new()
            .context("failed to read xdg base-dirs")?
            .get_runtime_file("dynlock.lock")
            .context("failed to locate lockfile")?;
        let lock = File::create(&path).context("failed to create lockfile")?;
        lock.try_lock_exclusive()
            .context("failed to lock lockfile")?;
        Ok(Self { path, lock })
    }
}

impl Drop for PidLock {
    fn drop(&mut self) {
        self.lock.unlock().expect("failed to unlock lockfile");
        remove_file(&self.path).expect("failed to remove lockfile");
    }
}
