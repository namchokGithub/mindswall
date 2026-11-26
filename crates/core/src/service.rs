use crate::{
    config::Config,
    ipc::{self, Command as Control, PlaybackStatus, Response},
    playback::Playback,
    wallpaper::{SystemWallpaper, WallpaperSetter, default_wallpaper_path},
};
use std::{path::Path, sync::mpsc, time::Instant};

fn apply(path: &Path) -> Result<(), crate::Error> {
    if let Err(error) = SystemWallpaper.set_wallpaper(path) {
        tracing::warn!(%error, "could not apply wallpaper");
        return Err(error);
    }
    if let Err(error) = crate::history::History::record(path, None) {
        tracing::warn!(%error, "could not record wallpaper history");
    }
    Ok(())
}

pub fn run_slideshow() -> Result<(), Box<dyn std::error::Error>> {
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
    let config = Config::load(&path)?;
    let mut playback = Playback::new(&config, Instant::now())?;
    let (tx, rx) = mpsc::channel();
    ipc::listen(tx).map_err(|e| {
        format!("cannot start slideshow control listener (another instance may be running): {e}")
    })?;
    playback
        .show
        .as_mut()
        .expect("initial slideshow exists")
        .next_available(apply)?;
    println!("Slideshow running. Press Ctrl+C to stop.");
    loop {
        let event = match playback.timer.wait(Instant::now()) {
            Some(wait) => rx.recv_timeout(wait),
            None => rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected),
        };
        match event {
            Ok((command, reply)) => {
                let now = Instant::now();
                let status = if matches!(command, Control::Status) {
                    Some(PlaybackStatus {
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
                    })
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
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(show) = playback.show.as_mut() {
                    if let Err(error) = show.next_available(apply) {
                        tracing::warn!(%error, "wallpaper update failed");
                    }
                    playback.timer.reset(Instant::now());
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("control listener stopped".into());
            }
        }
    }
}
