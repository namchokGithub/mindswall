use crate::{config::Config, scheduler::Scheduler, slideshow::Slideshow, utilities::FitMode};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub struct Playback {
    pub show: Option<Slideshow>,
    pub timer: Scheduler,
    pub default_image: Option<PathBuf>,
    /// Why the slideshow paused itself, such as the wallpaper being changed
    /// in Windows. `None` when it runs or the user paused it.
    pub paused_reason: Option<String>,
    paused_before_empty: bool,
    fit_mode: FitMode,
}

impl Playback {
    pub fn new(config: &Config, now: Instant) -> Result<Self, String> {
        let show = Slideshow::new(config.images().map_err(|e| e.to_string())?, config.shuffle)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            show: Some(show),
            timer: Scheduler::new(Duration::from_secs(config.interval_seconds), now),
            default_image: None,
            paused_reason: None,
            paused_before_empty: false,
            fit_mode: config.fit_mode,
        })
    }

    /// Apply a changed configuration. A running slideshow keeps its current
    /// cycle and wallpaper unless the current image left the collection or
    /// the fit mode changed.
    pub fn reload(
        &mut self,
        config: &Config,
        now: Instant,
        apply: &impl Fn(&Path) -> Result<(), String>,
        fallback: &impl Fn() -> Result<PathBuf, String>,
    ) -> Result<(), String> {
        if !config.folders.iter().any(|folder| folder.enabled) {
            if self.show.is_some() {
                self.paused_before_empty = self.timer.is_paused();
            }
            self.show = None;
            self.default_image = None;
            self.timer.pause(now);
            let path = fallback()?;
            apply(&path)?;
            self.default_image = Some(path);
            return Ok(());
        }
        let images = config.images().map_err(|e| e.to_string())?;
        self.replace_images(images, config, now, apply)
    }

    /// Apply file changes reported by a folder watcher. Returns whether the
    /// collection changed. Does nothing when no slideshow is running.
    pub fn apply_file_changes(
        &mut self,
        config: &Config,
        changed: &std::collections::HashSet<PathBuf>,
        now: Instant,
        apply: &impl Fn(&Path) -> Result<(), String>,
    ) -> Result<bool, String> {
        let Some(show) = self.show.as_ref() else {
            return Ok(false);
        };
        let images = config.apply_changes(show.images(), changed);
        if images.as_slice() == show.images() {
            return Ok(false);
        }
        self.replace_images(images, config, now, apply)?;
        Ok(true)
    }

    fn replace_images(
        &mut self,
        images: Vec<PathBuf>,
        config: &Config,
        now: Instant,
        apply: &impl Fn(&Path) -> Result<(), String>,
    ) -> Result<(), String> {
        let interval = Duration::from_secs(config.interval_seconds);
        match self.show.as_mut() {
            None => {
                let mut candidate =
                    Slideshow::new(images, config.shuffle).map_err(|e| e.to_string())?;
                candidate.next_available(apply)?;
                self.show = Some(candidate);
                self.default_image = None;
                self.timer = Scheduler::new(interval, now);
                if self.paused_before_empty {
                    self.timer.pause(now);
                }
            }
            Some(show) => {
                let mut candidate = show.clone();
                let current = candidate.current().map(Path::to_path_buf);
                candidate
                    .update(images, config.shuffle)
                    .map_err(|e| e.to_string())?;
                let changed = match current {
                    Some(path) if candidate.contains(&path) => {
                        if self.fit_mode != config.fit_mode && apply(path.as_path()).is_err() {
                            candidate.next_available(apply)?;
                            true
                        } else {
                            false
                        }
                    }
                    _ => {
                        candidate.next_available(apply)?;
                        true
                    }
                };
                *show = candidate;
                if self.timer.interval() != interval {
                    let paused = self.timer.is_paused();
                    self.timer = Scheduler::new(interval, now);
                    if paused {
                        self.timer.pause(now);
                    }
                } else if changed {
                    self.timer.reset(now);
                }
            }
        }
        self.fit_mode = config.fit_mode;
        Ok(())
    }
}
