use anyhow::{Context, Result};
use clap::Parser;
use log::{info, warn};
use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode,
    Result as NotifyResult, Watcher,
};
use serde::Deserialize;
use std::{
    collections::HashSet,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
};

mod vdj;

#[derive(Parser, Debug)]
#[command(
    name = "vdj_m3u_playlist_enforcer",
    version,
    about = "Watches VirtualDJ playlist descriptors defined in a config file and emits .m3u copies."
)]
struct Args {
    /// Path to the configuration file that defines the root folder to monitor.
    #[arg(
        long,
        short = 'c',
        value_name = "FILE",
        default_value = "/etc/vdj-m3u-playlist-enforcer/config.toml"
    )]
    config: PathBuf,

    /// Scan once and exit instead of watching for changes.
    #[arg(long)]
    once: bool,

    /// Override the root folder defined in the config file (useful for ad-hoc runs).
    #[arg(long, short = 'r', value_name = "PATH")]
    root: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct Config {
    root_folder: PathBuf,
}

impl Config {
    fn load(path: &Path) -> Result<Option<Self>> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err).with_context(|| format!("reading {}", path.display())),
        };

        let config =
            toml::from_str(&contents).with_context(|| format!("parsing {}", path.display()))?;
        Ok(Some(config))
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    run(args)
}

type ConversionTracker = Arc<Mutex<HashSet<PathBuf>>>;

fn run(args: Args) -> Result<()> {
    let Args {
        config: config_path,
        once,
        root,
    } = args;
    let config = Config::load(&config_path)?;
    let config_root = config.as_ref().map(|cfg| cfg.root_folder.clone());

    let root_override = root;
    let used_cli_root = root_override.is_some();

    let root_path = match root_override {
        Some(path) => path,
        None => match config_root {
            Some(path) => path,
            None => {
                info!(
                    "Configuration {} missing; create the file with a `root_folder` entry before starting.",
                    config_path.display()
                );
                return Ok(());
            }
        },
    };

    let root = root_path
        .canonicalize()
        .unwrap_or_else(|_| root_path.clone());
    if used_cli_root {
        info!("Using command-line root override: {}", root.display());
    } else {
        info!("Loaded configuration from {}", config_path.display());
        info!("Using configured root: {}", root.display());
    }

    info!("Scanning {} for .vdjfolder playlists", root.display());
    vdj::convert_all(&root)?;

    if once {
        return Ok(());
    }

    let tracker: ConversionTracker = Arc::new(Mutex::new(HashSet::new()));
    watch(root, tracker)
}

fn watch(root: PathBuf, tracker: ConversionTracker) -> Result<()> {
    info!("Watching {} for VirtualDJ playlist changes", root.display());

    let (sender, receiver) = mpsc::channel::<NotifyResult<Event>>();
    let mut watcher = RecommendedWatcher::new(sender, NotifyConfig::default())?;
    watcher.watch(&root, RecursiveMode::Recursive)?;

    while let Ok(res) = receiver.recv() {
        match res {
            Ok(event) => handle_event(event, &tracker),
            Err(e) => warn!("File watcher reported error: {}", e),
        }
    }

    Ok(())
}

fn handle_event(event: Event, tracker: &ConversionTracker) {
    if event.paths.is_empty() {
        return;
    }

    if event.paths.len() >= 2 {
        if handle_vdj_rename(&event.paths[0], &event.paths[1], tracker) {
            return;
        }
    }

    let mut handled = false;
    for path in event.paths.iter() {
        if handle_vdj_single_path(&event.kind, path, tracker) {
            handled = true;
        }
    }

    if handled {
        return;
    }
}

fn handle_vdj_single_path(kind: &EventKind, path: &Path, tracker: &ConversionTracker) -> bool {
    if !vdj::is_vdj_playlist(path) {
        return false;
    }

    match kind {
        EventKind::Create(_) | EventKind::Modify(_) => match vdj::convert(path) {
            Ok(result_path) => {
                info!(
                    "Converted VirtualDJ playlist {} -> {}",
                    path.display(),
                    result_path.display()
                );
                if let Ok(mut guard) = tracker.lock() {
                    guard.insert(path.to_path_buf());
                }
                if let Err(e) = fs::remove_file(path) {
                    warn!("Failed to remove descriptor {}: {}", path.display(), e);
                }
            }
            Err(e) => warn!("Failed to convert {}: {}", path.display(), e),
        },
        EventKind::Remove(_) => {
            if let Ok(mut guard) = tracker.lock() {
                if guard.remove(path) {
                    info!(
                        "Skipping removal for {} because it was just converted",
                        path.display()
                    );
                    return true;
                }
            }
            match vdj::remove(path) {
                Ok(_) => info!("Removed derived .m3u for {}", path.display()),
                Err(e) => warn!(
                    "Failed to remove derived playlist for {}: {}",
                    path.display(),
                    e
                ),
            }
        }
        _ => {}
    }

    true
}

fn handle_vdj_rename(from: &Path, to: &Path, tracker: &ConversionTracker) -> bool {
    let mut handled = false;

    if vdj::is_vdj_playlist(from) {
        handled = true;
        if let Err(e) = vdj::remove(from) {
            warn!(
                "Failed to remove derived playlist for {} after rename: {}",
                from.display(),
                e
            );
        }
    }

    if vdj::is_vdj_playlist(to) {
        handled = true;
        match vdj::convert(to) {
            Ok(result_path) => {
                info!(
                    "Converted VirtualDJ playlist {} -> {}",
                    to.display(),
                    result_path.display()
                );
                if let Ok(mut guard) = tracker.lock() {
                    guard.insert(to.to_path_buf());
                }
                if let Err(e) = fs::remove_file(to) {
                    warn!("Failed to remove descriptor {}: {}", to.display(), e);
                }
            }
            Err(e) => warn!("Failed to convert {} after rename: {}", to.display(), e),
        }
    }

    handled
}
