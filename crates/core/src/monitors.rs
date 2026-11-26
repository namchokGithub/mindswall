//! Separate slideshows for monitors that have their own folders.
//!
//! The main slideshow covers every connected monitor without its own
//! folders. A monitor with its own folders runs its own slideshow, which keeps
//! its cycle while the monitor is disconnected and continues when it returns.

use crate::{config::Config, slideshow::Slideshow};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

/// Where the main slideshow applies its wallpaper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedTarget {
    /// Every monitor, through a single desktop-wide call.
    Desktop,
    /// Only these monitors, one call each. Empty when every connected monitor
    /// has its own slideshow.
    Monitors(Vec<String>),
}

#[derive(Default)]
pub struct MonitorShows {
    shows: BTreeMap<String, Slideshow>,
    connected: Vec<String>,
}

impl MonitorShows {
    /// Build or update the per-monitor slideshows from the configuration.
    /// Existing slideshows keep their cycle. A monitor whose folders contain
    /// no images is logged and falls back to the main slideshow. Returns the
    /// monitors whose slideshow was created or replaced.
    pub fn configure(&mut self, config: &Config) -> Vec<String> {
        let mut created = Vec::new();
        let mut configured = HashSet::new();
        for entry in &config.monitor_folders {
            if !entry.folders.iter().any(|folder| folder.enabled) {
                continue;
            }
            let images = match config.images_in(&entry.folders) {
                Ok(images) if !images.is_empty() => images,
                Ok(_) => {
                    tracing::warn!(monitor = %entry.monitor, "monitor folders contain no images");
                    continue;
                }
                Err(error) => {
                    tracing::warn!(monitor = %entry.monitor, %error, "could not scan monitor folders");
                    continue;
                }
            };
            configured.insert(entry.monitor.clone());
            let updated = self
                .shows
                .get_mut(&entry.monitor)
                .is_some_and(|show| show.update(images.clone(), config.shuffle).is_ok());
            if !updated && let Ok(show) = Slideshow::new(images, config.shuffle) {
                self.shows.insert(entry.monitor.clone(), show);
                created.push(entry.monitor.clone());
            }
        }
        self.shows.retain(|monitor, _| configured.contains(monitor));
        created
    }

    /// Whether any monitor has its own folders configured, which means the
    /// connected monitors must be tracked.
    pub fn is_configured(config: &Config) -> bool {
        config
            .monitor_folders
            .iter()
            .any(|entry| entry.folders.iter().any(|folder| folder.enabled))
    }

    /// Record the connected monitors. Returns the monitors that were not
    /// connected before.
    pub fn set_connected(&mut self, monitors: Vec<String>) -> Vec<String> {
        let added = monitors
            .iter()
            .filter(|monitor| !self.connected.contains(monitor))
            .cloned()
            .collect();
        self.connected = monitors;
        added
    }

    pub fn connected(&self) -> &[String] {
        &self.connected
    }

    /// The connected monitors that run their own slideshow.
    pub fn active(&self) -> Vec<String> {
        self.connected
            .iter()
            .filter(|monitor| self.shows.contains_key(*monitor))
            .cloned()
            .collect()
    }

    pub fn shared_target(&self) -> SharedTarget {
        if self.active().is_empty() {
            SharedTarget::Desktop
        } else {
            SharedTarget::Monitors(
                self.connected
                    .iter()
                    .filter(|monitor| !self.shows.contains_key(*monitor))
                    .cloned()
                    .collect(),
            )
        }
    }

    pub fn current(&self, monitor: &str) -> Option<&Path> {
        self.shows.get(monitor).and_then(Slideshow::current)
    }

    /// Advance every connected monitor's slideshow. Errors are logged so one
    /// monitor cannot stop the others.
    pub fn advance(&mut self, apply: &impl Fn(&str, &Path) -> Result<(), String>) {
        for monitor in self.active() {
            if let Some(show) = self.shows.get_mut(&monitor)
                && let Err(error) = show.next_available(|path| apply(&monitor, path))
            {
                tracing::warn!(%monitor, %error, "monitor wallpaper update failed");
            }
        }
    }

    /// Like [`MonitorShows::advance`], but `apply` also receives each
    /// monitor's unplayed images and returns those it showed together with
    /// the new image (see [`Slideshow::next_available_with`]).
    pub fn advance_with(
        &mut self,
        apply: &impl Fn(&str, &Path, &[PathBuf]) -> Result<Vec<PathBuf>, String>,
    ) {
        for monitor in self.active() {
            if let Some(show) = self.shows.get_mut(&monitor)
                && let Err(error) =
                    show.next_available_with(|path, upcoming| apply(&monitor, path, upcoming))
            {
                tracing::warn!(%monitor, %error, "monitor wallpaper update failed");
            }
        }
    }

    /// Every image of the per-monitor slideshows; an image in several of
    /// them is listed once per slideshow.
    pub fn images(&self) -> impl Iterator<Item = &PathBuf> {
        self.shows.values().flat_map(Slideshow::images)
    }

    /// The slideshow of a monitor with its own folders.
    pub fn show(&self, monitor: &str) -> Option<&Slideshow> {
        self.shows.get(monitor)
    }

    /// The slideshow of a connected monitor, or why it has none.
    fn active_show(&mut self, monitor: &str) -> Result<&mut Slideshow, String> {
        if !self.connected.iter().any(|connected| connected == monitor) {
            return Err("This monitor is not connected".into());
        }
        self.shows
            .get_mut(monitor)
            .ok_or_else(|| "This monitor shares the main slideshow".into())
    }

    /// Move only `monitor` forward; see [`MonitorShows::advance_with`].
    /// Fails when the monitor is not connected or has no own slideshow.
    pub fn advance_one(
        &mut self,
        monitor: &str,
        apply: &impl Fn(&str, &Path, &[PathBuf]) -> Result<Vec<PathBuf>, String>,
    ) -> Result<PathBuf, String> {
        self.active_show(monitor)?
            .next_available_with(|path, upcoming| apply(monitor, path, upcoming))
    }

    /// Go back only on `monitor`. Fails when it has no earlier image.
    pub fn previous_one(
        &mut self,
        monitor: &str,
        apply: &impl Fn(&str, &Path) -> Result<(), String>,
    ) -> Result<PathBuf, String> {
        self.active_show(monitor)?
            .previous_available(|path| apply(monitor, path))
            .unwrap_or_else(|| Err("No previous wallpaper on this monitor".into()))
    }

    /// Go back on every connected monitor that has an earlier image.
    pub fn previous(&mut self, apply: &impl Fn(&str, &Path) -> Result<(), String>) {
        for monitor in self.active() {
            if let Some(show) = self.shows.get_mut(&monitor)
                && let Some(Err(error)) = show.previous_available(|path| apply(&monitor, path))
            {
                tracing::warn!(%monitor, %error, "monitor wallpaper update failed");
            }
        }
    }

    /// Show the current image again on each of `monitors` that runs its own
    /// slideshow, or its next image when it has none yet.
    pub fn reapply(
        &mut self,
        monitors: &[String],
        apply: &impl Fn(&str, &Path) -> Result<(), String>,
    ) {
        for monitor in monitors {
            let Some(show) = self.shows.get_mut(monitor) else {
                continue;
            };
            let result = match show.current().map(Path::to_path_buf) {
                Some(path) if apply(monitor, &path).is_ok() => Ok(path),
                _ => show.next_available(|path| apply(monitor, path)),
            };
            if let Err(error) = result {
                tracing::warn!(%monitor, %error, "monitor wallpaper update failed");
            }
        }
    }

    /// Apply folder watcher changes to every per-monitor slideshow. A
    /// connected monitor whose current image was removed moves on. Returns
    /// whether any collection changed.
    pub fn apply_file_changes(
        &mut self,
        config: &Config,
        changed: &HashSet<PathBuf>,
        apply: &impl Fn(&str, &Path) -> Result<(), String>,
    ) -> bool {
        let mut any = false;
        for entry in &config.monitor_folders {
            let Some(show) = self.shows.get_mut(&entry.monitor) else {
                continue;
            };
            let images = config.apply_changes_in(&entry.folders, show.images(), changed);
            if images.as_slice() == show.images() {
                continue;
            }
            any = true;
            let current = show.current().map(Path::to_path_buf);
            if images.is_empty() || show.update(images, config.shuffle).is_err() {
                tracing::warn!(monitor = %entry.monitor, "monitor folders no longer contain images");
                continue;
            }
            let connected = self.connected.contains(&entry.monitor);
            if connected
                && current.is_some_and(|path| !show.contains(&path))
                && let Err(error) = show.next_available(|path| apply(&entry.monitor, path))
            {
                tracing::warn!(monitor = %entry.monitor, %error, "monitor wallpaper update failed");
            }
        }
        any
    }
}
