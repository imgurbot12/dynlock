//! Dynamic ScreenLock CLI
use rand::seq::IteratorRandom;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Parser;

mod config;
mod event;
mod graphics;
mod lock;
mod pid;

use crate::config::{Config, Settings};

const XDG_PREFIX: &'static str = "dynlock";
const DEFAULT_CONFIG: &'static str = "config.yaml";

#[derive(Debug, Clone, Parser)]
struct Cli {
    /// Dynlock configuration filepath
    #[clap(short, long)]
    config: Option<String>,
    /// Fragment shader file/search-directory
    #[clap(short, long)]
    shader: Option<String>,
    /// Background image/search-directory
    #[clap(short, long)]
    background: Option<String>,
    /// Screensaver mode does not lock
    #[clap(long)]
    screensave: Option<bool>,
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
    /// Find PathBuf from Option or Use XDG to Find Default
    fn find_config_path(path: Option<String>, default_name: &str) -> PathBuf {
        path.clone()
            .map(|v| shellexpand::tilde(&v.to_string()).to_string())
            .map(|v| PathBuf::from(v))
            .unwrap_or_else(|| {
                xdg::BaseDirectories::with_prefix(XDG_PREFIX)
                    .expect("failed to read xdg base-dirs")
                    .get_config_file(default_name)
            })
    }
    /// Prepare CLI Flags for Use and Generate Settings
    pub fn settings(self) -> Result<Settings> {
        // read configuration file according to settings
        let cfgpath = Self::find_config_path(self.config, DEFAULT_CONFIG);
        let config = match cfgpath.exists() {
            true => {
                log::info!("reading configuration: {cfgpath:?}");
                let cfgdata = std::fs::read_to_string(&cfgpath).context("failed to read config")?;
                serde_yaml::from_str(&cfgdata).context("failed to parse config")?
            }
            false => {
                log::warn!("config: {cfgpath:?} does not exist. using default");
                Config::default()
            }
        };
        // resolve fragment shader path
        let shader = self.shader.or(config.shader);
        let fragpath = Self::find_config_path(shader, "shaders");
        let fragment = Self::find_file(&fragpath, vec!["glsl"]).context("failed to find shader")?;
        log::info!("loading fragment shader: {fragment:?}");
        // resolve background image path
        let mut background = self.background.or(config.background).map(PathBuf::from);
        if let Some(bg) = background {
            let bpath = Self::find_file(&bg, vec!["png", "jpg", "jpeg"])
                .context("failed to find background")?;
            log::info!("loading background image: {bpath:?}");
            background = Some(bpath);
        }
        // load Shader from file (if present)
        let shader = std::fs::read_to_string(fragment).context("failed to read shader file")?;
        let lock = !self.screensave.unwrap_or(!config.lock);
        match lock {
            true => log::info!("running in screensaver mode!"),
            false => log::info!("running in lockscreen mode!"),
        }
        Ok(Settings {
            lock,
            shader,
            background,
        })
    }
}

fn main() -> Result<()> {
    env_logger::init();

    // ensure only one lock instance runs at a time
    println!("making lock?");
    let _lock = pid::PidLock::new()?;

    // parse cli and run lockscreen
    let cli = Cli::parse();
    let settings = cli.settings()?;

    // attempt to load shader from file
    lock::lock(settings)
}
