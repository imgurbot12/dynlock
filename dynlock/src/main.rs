//! Dynamic ScreenLock CLI
use std::fs::File;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use rand::seq::IteratorRandom;

mod event;
mod graphics;
mod lock;

use clap_builder::Parser;
use dynlock_lib::{Cli, Config, Settings};

const XDG_PREFIX: &'static str = "dynlock";
const DEFAULT_CONFIG: &'static str = "config.yaml";

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
pub fn settings(cli: Cli) -> Result<Settings> {
    // read configuration file according to settings
    let cfgpath = find_config_path(cli.config, DEFAULT_CONFIG);
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
    let shader = cli.shader.or(config.shader);
    let fragpath = find_config_path(shader, "shaders");
    let fragment = find_file(&fragpath, vec!["glsl"]).context("failed to find shader")?;
    log::info!("loading fragment shader: {fragment:?}");
    // resolve background image path
    let mut background = cli.background.or(config.background).map(PathBuf::from);
    if let Some(bg) = background {
        let bpath =
            find_file(&bg, vec!["png", "jpg", "jpeg"]).context("failed to find background")?;
        log::info!("loading background image: {bpath:?}");
        background = Some(bpath);
    }
    // load Shader from file (if present)
    let shader = std::fs::read_to_string(fragment).context("failed to read shader file")?;
    let lock = !cli.screensave.unwrap_or(!config.lock);
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

fn main() -> Result<()> {
    // parse cli and init logger
    let cli = Cli::parse();
    match &cli.logfile {
        None => env_logger::init(),
        Some(logfile) => {
            // set default loglevel via env
            if std::env::var("RUST_LOG").is_err() {
                std::env::set_var("RUST_LOG", "info");
            }
            // generate logging instance
            let target = Box::new(File::create(logfile).context("failed to create logfile")?);
            env_logger::Builder::from_env("RUST_LOG")
                .target(env_logger::Target::Pipe(target))
                .init();
        }
    };

    // convert cli flags into settings object
    let daemonize = cli.daemonize;
    let settings = settings(cli)?;

    // ensure only one lock instance runs at a time
    if daemonize {
        // find pid location for daemonization
        let pid = xdg::BaseDirectories::new()
            .context("failed to read xdg base-dirs")?
            .get_runtime_file("dynlock.lock")
            .context("failed to locate lockfile")?;
        // daemonize
        let daemon = daemonize::Daemonize::new().pid_file(pid);
        daemon.start().context("failed to daemonize")?;
    }

    // attempt to load shader from file
    lock::lock(settings)
}
