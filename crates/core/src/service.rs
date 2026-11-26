use crate::{
    collage::{Collages, Combined},
    config::Config,
    ipc::{self, Command as Control, MonitorPlayback, PlaybackStatus, Response},
    monitors::{MonitorShows, SharedTarget},
    playback::Playback,
    utilities::elapsed_ms,
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

/// How often connected monitors are listed while any monitor has its own
/// folders, so connected and disconnected monitors are noticed.
const MONITOR_CHECK: Duration = Duration::from_secs(10);

/// How the slideshow service starts and whether it may hand over control.
#[derive(Debug, Clone, Default)]
pub struct ServiceOptions {
    /// Stop when the desktop app asks to take over. Set for the CLI.
    pub allow_handoff: bool,
    /// Continue from a slideshow handed over by the CLI: keep its current
    /// image and paused state instead of changing the wallpaper at start.
    pub resume: Option<PlaybackStatus>,
    /// How long to keep retrying the control pipe while a previous owner
    /// exits.
    pub listen_retry: Duration,
}

/// What changed when the service calls `on_change`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// The playback state changed: a new wallpaper, pause, resume, or a
    /// reloaded configuration.
    Playback,
    /// The folder watcher added, removed, or renamed images in the main or a
    /// per-monitor collection. The playback state may have changed as well.
    /// A renamed image counts as one removed and one added image; both counts
    /// are zero when only the order or a folder changed.
    Collection { added: usize, removed: usize },
}

enum Event {
    Control(ipc::Request),
    Files(Vec<PathBuf>),
}

fn snapshot(playback: &Playback, collages: &Collages, monitors: &MonitorShows) -> PlaybackStatus {
    let shown = playback
        .show
        .as_ref()
        .and_then(|show| show.current())
        .and_then(|current| collages.remembered(MAIN, current));
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
        combined: shown
            .as_ref()
            .map(|shown| shown.combined.clone())
            .unwrap_or_default(),
        display: shown.map(|shown| shown.file),
        monitors: monitors
            .active()
            .into_iter()
            .filter_map(|monitor| {
                let show = monitors.show(&monitor)?;
                let current = show.current().map(Path::to_path_buf);
                let shown = current
                    .as_deref()
                    .and_then(|current| collages.remembered(&monitor, current));
                Some(MonitorPlayback {
                    monitor,
                    current,
                    position: show.position(),
                    total: show.total(),
                    combined: shown
                        .as_ref()
                        .map(|shown| shown.combined.clone())
                        .unwrap_or_default(),
                    display: shown.map(|shown| shown.file),
                })
            })
            .collect(),
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
    let mut watched = HashSet::new();
    let folders = config.folders.iter().chain(
        config
            .monitor_folders
            .iter()
            .flat_map(|entry| &entry.folders),
    );
    for folder in folders.filter(|folder| folder.enabled) {
        if !watched.insert((folder.path.clone(), folder.recursive)) {
            continue;
        }
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

/// Key of the main slideshow's combined wallpapers.
const MAIN: &str = "main";

fn record(path: &Path, monitor: Option<String>, shown: Option<Combined>) {
    let combined = shown.map(|shown| shown.combined).unwrap_or_default();
    if let Err(error) = crate::history::History::record_combined(path, monitor, combined) {
        tracing::warn!(%error, "could not record wallpaper history");
    }
}

/// Apply an image, or the combined wallpaper made from it, to the whole
/// desktop.
fn apply(path: &Path, shown: Option<Combined>) -> Result<(), String> {
    let file = shown.as_ref().map_or(path, |shown| shown.file.as_path());
    let started = Instant::now();
    if let Err(error) = SystemWallpaper.set_wallpaper(file) {
        tracing::warn!(%error, "could not apply wallpaper");
        return Err(error.to_string());
    }
    let set_ms = elapsed_ms(started);
    let combined = shown.as_ref().map_or(0, |shown| shown.combined.len());
    record(path, None, shown);
    tracing::info!(path = ?path, combined, set_ms, history_ms = elapsed_ms(started) - set_ms, "wallpaper changed");
    Ok(())
}

/// Apply an image, or the combined wallpaper made from it, to one monitor.
fn apply_monitor(monitor: &str, path: &Path, shown: Option<Combined>) -> Result<(), String> {
    let file = shown.as_ref().map_or(path, |shown| shown.file.as_path());
    let started = Instant::now();
    crate::wallpaper::set_monitor_wallpaper(monitor, file).inspect_err(|error| {
        tracing::warn!(%monitor, %error, "could not apply monitor wallpaper");
    })?;
    let set_ms = elapsed_ms(started);
    let combined = shown.as_ref().map_or(0, |shown| shown.combined.len());
    record(path, Some(monitor.to_owned()), shown);
    tracing::info!(%monitor, path = ?path, combined, set_ms, history_ms = elapsed_ms(started) - set_ms, "monitor wallpaper changed");
    Ok(())
}

/// Apply an image from the main slideshow. When some monitors run their own
/// slideshow, only the other monitors are changed; it succeeds when at least
/// one of them accepts the image.
fn apply_shared(target: &SharedTarget, path: &Path, shown: Option<Combined>) -> Result<(), String> {
    let SharedTarget::Monitors(monitors) = target else {
        return apply(path, shown);
    };
    if monitors.is_empty() {
        return Ok(());
    }
    let file = shown.as_ref().map_or(path, |shown| shown.file.as_path());
    let started = Instant::now();
    let mut failure = None;
    let mut applied = false;
    for monitor in monitors {
        match crate::wallpaper::set_monitor_wallpaper(monitor, file) {
            Ok(()) => applied = true,
            Err(error) => {
                tracing::warn!(%monitor, %error, "could not apply monitor wallpaper");
                failure = Some(error);
            }
        }
    }
    if !applied {
        return Err(failure.unwrap_or_default());
    }
    let set_ms = elapsed_ms(started);
    let combined = shown.as_ref().map_or(0, |shown| shown.combined.len());
    record(path, None, shown);
    tracing::info!(path = ?path, monitors = monitors.len(), combined, set_ms, history_ms = elapsed_ms(started) - set_ms, "wallpaper changed");
    Ok(())
}

/// The size of the image the main slideshow needs: the primary monitor, all
/// monitors together for the Span fit mode, or the first monitor it covers.
fn shared_size(target: &SharedTarget, span: bool) -> Option<(u32, u32)> {
    let monitors = crate::wallpaper::list_monitors().ok()?;
    let size = |monitor: &crate::wallpaper::MonitorInfo| {
        Some((
            u32::try_from(monitor.width).ok()?,
            u32::try_from(monitor.height).ok()?,
        ))
    };
    match target {
        SharedTarget::Desktop if span => {
            let left = monitors.iter().map(|m| m.x).min()?;
            let top = monitors.iter().map(|m| m.y).min()?;
            let right = monitors.iter().map(|m| m.x + m.width).max()?;
            let bottom = monitors.iter().map(|m| m.y + m.height).max()?;
            Some((
                u32::try_from(right - left).ok()?,
                u32::try_from(bottom - top).ok()?,
            ))
        }
        SharedTarget::Desktop => monitors
            .iter()
            .find(|monitor| monitor.primary)
            .or_else(|| monitors.first())
            .and_then(size),
        SharedTarget::Monitors(ids) => ids
            .iter()
            .find_map(|id| monitors.iter().find(|monitor| &monitor.id == id))
            .and_then(size),
    }
}

fn monitor_size(id: &str) -> Option<(u32, u32)> {
    let monitors = crate::wallpaper::list_monitors().ok()?;
    let monitor = monitors.into_iter().find(|monitor| monitor.id == id)?;
    Some((
        u32::try_from(monitor.width).ok()?,
        u32::try_from(monitor.height).ok()?,
    ))
}

/// Show an image of the main slideshow, combined with some of the `upcoming`
/// images when portrait images are combined. Without `upcoming`, the last
/// combination shown for the image is reused. Returns the combined images.
fn show_shared(
    collages: &Collages,
    target: &SharedTarget,
    path: &Path,
    upcoming: &[PathBuf],
) -> Result<Vec<PathBuf>, String> {
    let started = Instant::now();
    let shown = collages.combine(MAIN, path, upcoming, || {
        shared_size(target, collages.span())
    });
    let combine_ms = elapsed_ms(started);
    let used = shown
        .as_ref()
        .map(|shown| shown.combined.clone())
        .unwrap_or_default();
    apply_shared(target, path, shown)?;
    tracing::info!(
        combine_ms,
        total_ms = elapsed_ms(started),
        "wallpaper change timing"
    );
    Ok(used)
}

/// Show an image of a monitor's own slideshow; see [`show_shared`].
fn show_monitor(
    collages: &Collages,
    monitor: &str,
    path: &Path,
    upcoming: &[PathBuf],
) -> Result<Vec<PathBuf>, String> {
    let started = Instant::now();
    let shown = collages.combine(monitor, path, upcoming, || monitor_size(monitor));
    let combine_ms = elapsed_ms(started);
    let used = shown
        .as_ref()
        .map(|shown| shown.combined.clone())
        .unwrap_or_default();
    apply_monitor(monitor, path, shown)?;
    tracing::info!(%monitor, combine_ms, total_ms = elapsed_ms(started), "monitor wallpaper change timing");
    Ok(used)
}

/// Show the main slideshow's current image again, for example after the
/// monitors that share it changed.
fn reapply_shared(playback: &Playback, target: &SharedTarget, collages: &Collages) {
    let current = playback
        .show
        .as_ref()
        .and_then(|show| show.current().map(Path::to_path_buf))
        .or_else(|| playback.default_image.clone());
    if let Some(path) = current
        && let Err(error) = show_shared(collages, target, &path, &[])
    {
        tracing::warn!(%error, "could not apply wallpaper to the shared monitors");
    }
}

/// List connected monitors. Returns whether the set changed and which
/// monitors were added.
fn check_monitors(monitors: &mut MonitorShows) -> (bool, Vec<String>) {
    match crate::wallpaper::list_monitors() {
        Ok(list) => {
            let ids: Vec<String> = list.into_iter().map(|monitor| monitor.id).collect();
            let before = monitors.connected().to_vec();
            let added = monitors.set_connected(ids);
            (before != monitors.connected(), added)
        }
        Err(error) => {
            tracing::warn!(%error, "could not list monitors");
            (false, Vec::new())
        }
    }
}

/// Every image in the main and the per-monitor collections.
fn collection(playback: &Playback, monitors: &MonitorShows) -> HashSet<PathBuf> {
    playback
        .show
        .iter()
        .flat_map(|show| show.images())
        .chain(monitors.images())
        .cloned()
        .collect()
}

fn earliest(waits: [Option<Duration>; 3]) -> Option<Duration> {
    waits.into_iter().flatten().min()
}

/// Run the slideshow from the CLI. It stops when the desktop app takes over.
const LOG_FILE: &str = "mindswall.log";

/// The slideshow log file in the per-user data directory. The file exists
/// only after a slideshow has started.
pub fn log_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "MindsWall").map(|dirs| dirs.data_dir().join(LOG_FILE))
}

pub fn run_slideshow() -> Result<(), Box<dyn std::error::Error>> {
    run_slideshow_with(
        ServiceOptions {
            allow_handoff: true,
            ..ServiceOptions::default()
        },
        |_, _| {},
    )
}

/// Run the slideshow and call `on_change` after every state change, such as a
/// new wallpaper, pause, resume, a reloaded configuration, or images changed in
/// a watched folder. Returns `Ok` when the slideshow was handed over to the
/// desktop app.
pub fn run_slideshow_with(
    options: ServiceOptions,
    on_change: impl Fn(&PlaybackStatus, Change),
) -> Result<(), Box<dyn std::error::Error>> {
    let log = log_path().ok_or("user data directory unavailable")?;
    let log_dir = log.parent().ok_or("user data directory unavailable")?;
    std::fs::create_dir_all(log_dir)?;
    let appender = tracing_appender::rolling::never(log_dir, LOG_FILE);
    let (writer, _guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(writer)
        .try_init()
        .ok();
    let path = Config::default_path()?;
    let mut config = Config::load(&path)?;
    let mut playback = Playback::new(&config, Instant::now())?;
    let collages = Collages::new(&config);
    // Monitor slideshows: `monitor_next` may combine upcoming images, and
    // `monitor_again` shows an image again with its last combination.
    let monitor_next = |monitor: &str, path: &Path, upcoming: &[PathBuf]| {
        show_monitor(&collages, monitor, path, upcoming)
    };
    let monitor_again =
        |monitor: &str, path: &Path| show_monitor(&collages, monitor, path, &[]).map(|_| ());
    let (control_tx, control_rx) = mpsc::channel();
    let listen_until = Instant::now() + options.listen_retry;
    loop {
        match ipc::listen(control_tx.clone()) {
            Ok(()) => break,
            Err(error)
                if error.kind() != std::io::ErrorKind::PermissionDenied
                    && Instant::now() < listen_until =>
            {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(error) => {
                return Err(format!(
                    "cannot start slideshow control listener (another instance may be running): {error}"
                )
                .into());
            }
        }
    }
    drop(control_tx);
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

    let mut monitors = MonitorShows::default();
    monitors.configure(&config);
    let mut tracking = MonitorShows::is_configured(&config);
    if tracking {
        check_monitors(&mut monitors);
    }
    let mut monitors_due = tracking.then(|| Instant::now() + MONITOR_CHECK);

    let show = playback.show.as_mut().expect("initial slideshow exists");
    let resumed = options
        .resume
        .as_ref()
        .and_then(|status| status.current.clone())
        .filter(|path| show.contains(path))
        .is_some_and(|path| show.select(path).is_ok());
    if !resumed {
        let target = monitors.shared_target();
        show.next_available_with(|path, upcoming| show_shared(&collages, &target, path, upcoming))?;
    }
    if options
        .resume
        .as_ref()
        .is_some_and(|status| status.state == "paused")
    {
        playback.timer.pause(Instant::now());
    }
    monitors.reapply(&monitors.active(), &monitor_again);
    on_change(&snapshot(&playback, &collages, &monitors), Change::Playback);
    println!("Slideshow running. Press Ctrl+C to stop.");

    let mut changed_files = HashSet::new();
    let mut files_due: Option<Instant> = None;
    loop {
        let now = Instant::now();
        let wait = earliest([
            playback.timer.wait(now),
            files_due.map(|due| due.saturating_duration_since(now)),
            monitors_due.map(|due| due.saturating_duration_since(now)),
        ]);
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
                if matches!(command, Control::Handoff) {
                    if options.allow_handoff {
                        let _ = reply.send(Response {
                            ok: true,
                            message: "handed over".into(),
                            status: Some(snapshot(&playback, &collages, &monitors)),
                        });
                        tracing::info!("slideshow handed over to the desktop app");
                        println!("Slideshow handed over to the MindsWall app.");
                        return Ok(());
                    }
                    let _ = reply.send(Response {
                        ok: false,
                        message: "the desktop app owns the slideshow".into(),
                        status: None,
                    });
                    continue;
                }
                let notify_change = !matches!(command, Control::Status);
                let mut reloaded = None;
                let status = if matches!(command, Control::Status) {
                    Some(snapshot(&playback, &collages, &monitors))
                } else {
                    None
                };
                let target = monitors.shared_target();
                let shared = |path: &Path| show_shared(&collages, &target, path, &[]).map(|_| ());
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
                        let started = Instant::now();
                        let new_config =
                            Config::load(&Config::default_path().map_err(|e| e.to_string())?)
                                .map_err(|e| e.to_string())?;
                        collages.configure(&new_config);
                        monitors.configure(&new_config);
                        tracking = MonitorShows::is_configured(&new_config);
                        if tracking {
                            check_monitors(&mut monitors);
                        }
                        let new_target = monitors.shared_target();
                        playback.reload(
                            &new_config,
                            now,
                            &|path| show_shared(&collages, &new_target, path, &[]).map(|_| ()),
                            &default_wallpaper_path,
                        )?;
                        // Monitors may have moved between the main slideshow
                        // and their own folders, so show each one's image again.
                        let scan_ms = elapsed_ms(started);
                        if tracking || new_target != target {
                            reapply_shared(&playback, &new_target, &collages);
                            monitors.reapply(&monitors.active(), &monitor_again);
                        }
                        tracing::info!(
                            scan_ms,
                            apply_ms = elapsed_ms(started) - scan_ms,
                            total_ms = elapsed_ms(started),
                            "settings reloaded"
                        );
                        reloaded = Some(new_config);
                        Ok(if playback.show.is_none() {
                            "No enabled folders; using Windows default wallpaper"
                        } else {
                            "settings reloaded"
                        }
                        .into())
                    })(),
                    Control::Set(path) => {
                        if let Some(mut candidate) = playback.show.clone() {
                            match candidate
                                .select(path.clone())
                                .map_err(|e| e.to_string())
                                .and_then(|()| shared(&path))
                            {
                                Ok(()) => {
                                    playback.show = Some(candidate);
                                    playback.timer.reset(now);
                                    Ok(path.display().to_string())
                                }
                                Err(error) => Err(error),
                            }
                        } else {
                            Err("No enabled folders".into())
                        }
                    }
                    Control::Next => match playback.show.as_mut() {
                        Some(show) => {
                            let result = show
                                .next_available_with(|path, upcoming| {
                                    show_shared(&collages, &target, path, upcoming)
                                })
                                .map(|path| path.display().to_string());
                            monitors.advance_with(&monitor_next);
                            playback.timer.reset(now);
                            result
                        }
                        None => Err("No enabled folders".into()),
                    },
                    Control::Previous => match playback.show.as_mut() {
                        Some(show) => match show.previous_available(shared) {
                            Some(result) => {
                                monitors.previous(&monitor_again);
                                playback.timer.reset(now);
                                result.map(|path| path.display().to_string())
                            }
                            None => Err("no previous wallpaper".into()),
                        },
                        None => Err("No enabled folders".into()),
                    },
                    // Only this monitor changes; the shared timer keeps running.
                    Control::NextMonitor(monitor) => monitors
                        .advance_one(&monitor, &monitor_next)
                        .map(|path| path.display().to_string()),
                    Control::PreviousMonitor(monitor) => monitors
                        .previous_one(&monitor, &monitor_again)
                        .map(|path| path.display().to_string()),
                    Control::Handoff => unreachable!("handled above"),
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
                    monitors_due = tracking.then(|| Instant::now() + MONITOR_CHECK);
                    // Drop the old watcher first so folders are not watched twice.
                    drop(watcher.take());
                    watcher = watch(&config, event_tx.clone());
                }
                if notify_change {
                    on_change(&snapshot(&playback, &collages, &monitors), Change::Playback);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let now = Instant::now();
                let mut changed = false;
                let mut collection_changed = None;
                if monitors_due.is_some_and(|due| now >= due) {
                    monitors_due = Some(now + MONITOR_CHECK);
                    let (layout_changed, added) = check_monitors(&mut monitors);
                    if layout_changed {
                        tracing::info!(
                            monitors = monitors.connected().len(),
                            "connected monitors changed"
                        );
                        reapply_shared(&playback, &monitors.shared_target(), &collages);
                        monitors.reapply(&added, &monitor_again);
                        changed = true;
                    }
                }
                if files_due.is_some_and(|due| now >= due) {
                    files_due = None;
                    let paths = std::mem::take(&mut changed_files);
                    let before = collection(&playback, &monitors);
                    let mut updated = false;
                    let target = monitors.shared_target();
                    match playback.apply_file_changes(&config, &paths, now, &|path| {
                        show_shared(&collages, &target, path, &[]).map(|_| ())
                    }) {
                        Ok(true) => {
                            tracing::info!(
                                paths = paths.len(),
                                "collection updated from folder changes"
                            );
                            updated = true;
                        }
                        Ok(false) => {}
                        Err(error) => tracing::warn!(%error, "could not apply folder changes"),
                    }
                    if monitors.apply_file_changes(&config, &paths, &monitor_again) {
                        tracing::info!("monitor collections updated from folder changes");
                        updated = true;
                    }
                    if updated {
                        let after = collection(&playback, &monitors);
                        collection_changed = Some(Change::Collection {
                            added: after.difference(&before).count(),
                            removed: before.difference(&after).count(),
                        });
                    }
                }
                if playback.timer.is_due(now)
                    && let Some(show) = playback.show.as_mut()
                {
                    let target = monitors.shared_target();
                    if let Err(error) = show.next_available_with(|path, upcoming| {
                        show_shared(&collages, &target, path, upcoming)
                    }) {
                        tracing::warn!(%error, "wallpaper update failed");
                    }
                    monitors.advance_with(&monitor_next);
                    playback.timer.reset(Instant::now());
                    changed = true;
                }
                if let Some(change) = collection_changed {
                    on_change(&snapshot(&playback, &collages, &monitors), change);
                } else if changed {
                    on_change(&snapshot(&playback, &collages, &monitors), Change::Playback);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("control listener stopped".into());
            }
        }
    }
}
