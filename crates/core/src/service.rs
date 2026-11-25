use crate::{
    config::Config,
    ipc::{self, Command as Control, Response},
    scheduler::Scheduler,
    slideshow::Slideshow,
    wallpaper::{SystemWallpaper, WallpaperSetter},
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

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
    let mut show = Slideshow::new(config.images()?, config.shuffle)?;
    let (tx, rx) = mpsc::channel();
    ipc::listen(tx).map_err(|e| {
        format!("cannot start slideshow control listener (another instance may be running): {e}")
    })?;
    let mut timer = Scheduler::new(Duration::from_secs(config.interval_seconds), Instant::now());
    SystemWallpaper.set_wallpaper(show.next())?;
    println!("Slideshow running. Press Ctrl+C to stop.");
    loop {
        let event = match timer.wait(Instant::now()) {
            Some(wait) => rx.recv_timeout(wait),
            None => rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected),
        };
        match event {
            Ok((command, reply)) => {
                let now = Instant::now();
                let result: Result<String, String> = match command {
                    Control::Pause => {
                        timer.pause(now);
                        Ok("paused".into())
                    }
                    Control::Resume => {
                        timer.resume(now);
                        Ok("running".into())
                    }
                    Control::Status => Ok(format!(
                        "{}: {}",
                        if timer.is_paused() {
                            "paused"
                        } else {
                            "running"
                        },
                        show.current()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()
                    )),
                    Control::Reload => (|| -> Result<String, String> {
                        let config =
                            Config::load(&Config::default_path().map_err(|e| e.to_string())?)
                                .map_err(|e| e.to_string())?;
                        let mut candidate = Slideshow::new(
                            config.images().map_err(|e| e.to_string())?,
                            config.shuffle,
                        )
                        .map_err(|e| e.to_string())?;
                        SystemWallpaper
                            .set_wallpaper(candidate.next())
                            .map_err(|e| e.to_string())?;
                        show = candidate;
                        let paused = timer.is_paused();
                        timer = Scheduler::new(Duration::from_secs(config.interval_seconds), now);
                        if paused {
                            timer.pause(now);
                        }
                        Ok("settings reloaded".into())
                    })(),
                    Control::Next => {
                        let mut candidate = show.clone();
                        let path = candidate.next();
                        let result = SystemWallpaper
                            .set_wallpaper(path)
                            .map(|()| path.display().to_string())
                            .map_err(|e| e.to_string());
                        if result.is_ok() {
                            show = candidate;
                        }
                        timer.reset(now);
                        result
                    }
                    Control::Previous => {
                        let mut candidate = show.clone();
                        match candidate.previous() {
                            Some(path) => {
                                let result = SystemWallpaper
                                    .set_wallpaper(path)
                                    .map(|()| path.display().to_string())
                                    .map_err(|e| e.to_string());
                                if result.is_ok() {
                                    show = candidate;
                                }
                                timer.reset(now);
                                result
                            }
                            None => Err("no previous wallpaper".into()),
                        }
                    }
                };
                let response = match result {
                    Ok(message) => Response { ok: true, message },
                    Err(message) => {
                        tracing::warn!("{message}");
                        Response { ok: false, message }
                    }
                };
                let _ = reply.send(response);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let mut candidate = show.clone();
                match SystemWallpaper.set_wallpaper(candidate.next()) {
                    Ok(()) => show = candidate,
                    Err(error) => tracing::warn!(%error, "wallpaper update failed"),
                };
                timer.reset(Instant::now());
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("control listener stopped".into());
            }
        }
    }
}
