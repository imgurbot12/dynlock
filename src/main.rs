//! Dynamic ScreenLock CLI
use rand::seq::IteratorRandom;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Parser;

mod config;
mod event;
mod graphics;
mod lock;

//TODO: is disolve supposed to flicker like that?

use crate::config::Settings;

#[derive(Debug, Clone, Parser)]
struct Cli {
    /// Fragment Shader File/Search-Directory for Lockscreen
    #[clap(short, long)]
    shader: Option<PathBuf>,
    /// Background Image/Search-Directory for LockScreen
    #[clap(short, long)]
    background: Option<PathBuf>,
}

impl Cli {
    /// Attempt to Find Valid Filepath of Certain Extension
    fn find_file(p: &PathBuf, ext: &str) -> Result<PathBuf> {
        if !p.exists() {
            return Err(anyhow!(format!("no such file: {p:?}")));
        }
        Ok(if p.is_dir() {
            let mut rng = rand::thread_rng();
            let files = std::fs::read_dir(&p)
                .context(format!("failed to read dir: {p:?}"))?
                .filter_map(|f| f.ok())
                .filter(|f| f.path().ends_with(ext));
            files
                .choose(&mut rng)
                .context(format!("no files present in {p:?}"))?
                .path()
        } else {
            p.to_owned()
        })
    }
    /// Prepare CLI Flags for Use and Generate Settings
    pub fn settings(mut self) -> Result<Settings> {
        // attempt to Resolve Shader/Background FilePaths
        let fragment = if let Some(shader) = self.shader.as_ref() {
            Self::find_file(shader, ".glsl").context("failed to find shader")?
        } else {
            todo!()
        };
        if let Some(background) = self.background.as_ref() {
            self.shader =
                Some(Self::find_file(background, "").context("failed to find background")?);
        }
        // load Shader from File (if present)
        let shader = std::fs::read_to_string(fragment).context("failed to read shader file")?;
        Ok(Settings {
            shader,
            background: self.background,
        })
    }
}

fn main() -> Result<()> {
    // enable log and set default level
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // parse cli and run lockscreen
    let cli = Cli::parse();
    let settings = cli.settings()?;

    // attempt to load shader from file
    lock::lock(settings)
}
