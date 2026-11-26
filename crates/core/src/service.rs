use crate::{
    config::Config,
    ipc::{self, Command as Control, PlaybackStatus, Response},
    playback::Playback,
    wallpaper::{SystemWallpaper, WallpaperSetter, default_wallpaper_path},
};
use notify::{RecursiveMode, Watcher};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

/// Quiet period after the last file event before changes are applied, so
/// files that are still being copied can settle.
const FILE_SETTLE: Duration = Duration::from_secs(2);

enum Event {
    Control(ipc::Request),
    Files(Vec<PathBuf>),
}

fn snapshot(playback: &Playback) -> PlaybackStatus {
    PlaybackStatus {
        state: if playback.show.is_none() {
            "stopped"
        } else if playback.timer.is_paused() {
            "paused"
        } else {
            "running"
        }
        .to_owned(),
        current: playback
            .show
            .as_ref()
            .and_then(|show| show.current().map(Path::to_path_buf))
            .or_else(|| playback.default_image.clone()),
        position: playback.show.as_ref().and_then(|show| show.position()),
        total: playback.show.as_ref().map_or(0, |show| show.total()),
    }
}

/// Watch the enabled folders and forward file events to the slideshow loop.
/// Folders that cannot be watched are logged and skipped.
fn watch(config: &Config, events: mpsc::Sender<Event>) -> Option<notify::RecommendedWatcher> {
    let handler = move |result: notify::Result<notify::Event>| match result {
        Ok(event) if !event.kind.is_access() && !event.paths.is_empty() => {
            let _ = events.send(Event::Files(event.paths));
        }
        Ok(_) => {}
        Err(error) => tracing::warn!(%error, "folder watcher error"),
    };
    let mut watcher = match notify::recommended_watcher(handler) {
        Ok(watcher) => watcher,
        Err(error) => {
            tracing::warn!(%error, "could not start the folder watcher");
            return None;
        }
    };
    for folder in config.folders.iter().filter(|folder| folder.enabled) {
        let mode = if folder.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        if let Err(error) = watcher.watch(&folder.path, mode) {
            tracing::warn!(path = ?folder.path, %error, "could not watch folder");
        }
    }
    Some(watcher)
}

fn apply(path: &Path) -> Result<(), crate::Error> {
    if let Err(error) = SystemWallpaper.set_wallpaper(path) {
        tracing::warn!(%error, "could not apply wallpaper");
        return Err(error);
    }
    tracing::info!(path = ?path, "wallpaper changed");
    if let Err(error) = crate::history::History::record(path, None) {
        tracing::warn!(%error, "could not record wallpaper history");
    }
    Ok(())
}

pub fn run_slideshow() -> Result<(), Box<dyn std::error::Error>> {
    run_slideshow_with(|_| {})
}

/// Run the slideshow and call `on_change` after every state change, such as a
/// new wallpaper, pause, resume, or a reloaded collection.
pub fn run_slideshow_with(
    on_change: impl Fn(&PlaybackStatus),
) -> Result<(), Box<dyn std::error::Error>> {
    let dirs = directories::ProjectDirs::from("", "", "MindsWall")
        .ok_or("user data directory unavailable")?;
    std::fs::create_dir_all(dirs.data_dir())?;
    let appender = tracing_appender::rolling::never(dirs.data_dir(), "mindswall.log");
    let (writer, _guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(writer)
        .try_init()
        .ok();
    let path = Config::default_path()?;
    let mut config = Config::load(&path)?;
    let mut playback = Playback::new(&config, Instant::now())?;
    let (control_tx, control_rx) = mpsc::channel();
    ipc::listen(control_tx).map_err(|e| {
        format!("cannot start slideshow control listener (another instance may be running): {e}")
    })?;
    let (event_tx, event_rx) = mpsc::channel();
    let forward = event_tx.clone();
    std::thread::spawn(move || {
        for request in control_rx {
            if forward.send(Event::Control(request)).is_err() {
                break;
            }
        }
    });
    let mut watcher = watch(&config, event_tx.clone());
    playback
        .show
        .as_mut()
        .expect("initial slideshow exists")
        .next_available(apply)?;
    on_change(&snapshot(&playback));
    println!("Slideshow running. Press Ctrl+C to stop.");
    let mut changed_files = HashSet::new();
    let mut files_due: Option<Instant> = None;
    loop {
        let now = Instant::now();
        let wait = match (
            playback.timer.wait(now),
            files_due.map(|due| due.saturating_duration_since(now)),
        ) {
            (Some(timer), Some(files)) => Some(timer.min(files)),
            (timer, files) => timer.or(files),
        };
        let event = match wait {
            Some(wait) => event_rx.recv_timeout(wait),
            None => event_rx
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected),
        };
        match event {
            Ok(Event::Files(paths)) => {
                changed_files.extend(paths);
                files_due = Some(Instant::now() + FILE_SETTLE);
            }
            Ok(Event::Control((command, reply))) => {
                let now = Instant::now();
                let notify_change = !matches!(command, Control::Status);
                let mut reloaded = None;
                let status = if matches!(command, Control::Status) {
                    Some(snapshot(&playback))
                } else {
                    None
                };
                let result: Result<String, String> = match command {
                    Control::Pause => {
                        if playback.show.is_none() {
                            Err("No enabled folders".into())
                        } else {
                            playback.timer.pause(now);
                            Ok("paused".into())
                        }
                    }
                    Control::Resume => {
                        if playback.show.is_none() {
                            Err("No enabled folders".into())
                        } else {
                            playback.timer.resume(now);
                            Ok("running".into())
                        }
                    }
                    Control::Status => Ok(format!(
                        "{}: {}",
                        status
                            .as_ref()
                            .map_or("stopped", |status| status.state.as_str()),
                        status
                            .as_ref()
                            .and_then(|status| status.current.as_ref())
                            .map(|path| path.display().to_string())
                            .unwrap_or_default()
                    )),
                    Control::Reload => (|| -> Result<String, String> {
                        let config =
                            Config::load(&Config::default_path().map_err(|e| e.to_string())?)
                                .map_err(|e| e.to_string())?;
                        playback.reload(
                            &config,
                            now,
                            &|path| apply(path).map_err(|e| e.to_string()),
                            &default_wallpaper_path,
                        )?;
                        reloaded = Some(config);
                        Ok(if playback.show.is_none() {
                            "No enabled folders; using Windows default wallpaper"
                        } else {
                            "settings reloaded"
                        }
                        .into())
                    })(),
                    Control::Set(path) => {
                        if let Some(mut candidate) = playback.show.clone() {
                            match candidate.select(path.clone()).and_then(|()| apply(&path)) {
                                Ok(()) => {
                                    playback.show = Some(candidate);
                                    playback.timer.reset(now);
                                    Ok(path.display().to_string())
                                }
                                Err(error) => Err(error.to_string()),
                            }
                        } else {
                            Err("No enabled folders".into())
                        }
                    }
                    Control::Next => match playback.show.as_mut() {
                        Some(show) => {
                            let result = show
                                .next_available(apply)
                                .map(|path| path.display().to_string())
                                .map_err(|e| e.to_string());
                            playback.timer.reset(now);
                            result
                        }
                        None => Err("No enabled folders".into()),
                    },
                    Control::Previous => match playback.show.as_mut() {
                        Some(show) => match show.previous_available(apply) {
                            Some(result) => {
                                playback.timer.reset(now);
                                result
                                    .map(|path| path.display().to_string())
                                    .map_err(|e| e.to_string())
                            }
                            None => Err("no previous wallpaper".into()),
                        },
                        None => Err("No enabled folders".into()),
                    },
                };
                let response = match result {
                    Ok(message) => Response {
                        ok: true,
                        message,
                        status,
                    },
                    Err(message) => {
                        tracing::warn!("{message}");
                        Response {
                            ok: false,
                            message,
                            status: None,
                        }
                    }
                };
                let _ = reply.send(response);
                if let Some(new_config) = reloaded {
                    config = new_config;
                    changed_files.clear();
                    files_due = None;
                    // Drop the old watcher first so folders are not watched twice.
                    drop(watcher.take());
                    watcher = watch(&config, event_tx.clone());
                }
                if notify_change {
                    on_change(&snapshot(&playback));
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let now = Instant::now();
                if files_due.is_some_and(|due| now >= due) {
                    files_due = None;
                    let changed = std::mem::take(&mut changed_files);
                    match playback.apply_file_changes(&config, &changed, now, &|path| {
                        apply(path).map_err(|e| e.to_string())
                    }) {
                        Ok(true) => {
                            tracing::info!(
                                paths = changed.len(),
                                "collection updated from folder changes"
                            );
                            on_change(&snapshot(&playback));
                        }
                        Ok(false) => {}
                        Err(error) => tracing::warn!(%error, "could not apply folder changes"),
                    }
                }
                if playback.timer.is_due(now)
                    && let Some(show) = playback.show.as_mut()
                {
                    if let Err(error) = show.next_available(apply) {
                        tracing::warn!(%error, "wallpaper update failed");
                    }
                    playback.timer.reset(Instant::now());
                    on_change(&snapshot(&playback));
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("control listener stopped".into());
            }
        }
    }
}
