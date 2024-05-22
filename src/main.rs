//! Dynamic ScreenLock CLI
use rand::seq::IteratorRandom;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Parser;

mod config;
mod event;
mod graphics;
mod lock;

use crate::config::Settings;

const XDG_PREFIX: &'static str = "dynlock";

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
    fn find_file(p: &PathBuf, extensions: Vec<&str>) -> Result<PathBuf> {
        if !p.exists() {
            return Err(anyhow!(format!("no such file: {p:?}")));
        }
        Ok(if p.is_dir() {
            let mut rng = rand::thread_rng();
            let files = std::fs::read_dir(&p)
                .context(format!("failed to read dir: {p:?}"))?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|f| {
                    let ext = f
                        .extension()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    extensions.contains(&ext.as_str())
                });
            files
                .choose(&mut rng)
                .context(format!("no files present in dir {p:?}"))?
        } else {
            p.to_owned()
        })
    }
    /// Prepare CLI Flags for Use and Generate Settings
    pub fn settings(mut self) -> Result<Settings> {
        // attempt to resolve shader/background filepaths
        let fragpath = self.shader.clone().unwrap_or_else(|| {
            xdg::BaseDirectories::with_prefix(XDG_PREFIX)
                .expect("failed to read xdg base-dirs")
                .get_config_file("shaders")
        });
        let fragment = Self::find_file(&fragpath, vec!["glsl"]).context("failed to find shader")?;
        log::info!("loading fragment shader: {fragment:?}");
        if let Some(background) = self.background.as_ref() {
            let bpath = Self::find_file(background, vec!["png", "jpg", "jpeg"])
                .context("failed to find background")?;
            log::info!("loading background image: {bpath:?}");
            self.background = Some(bpath);
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
