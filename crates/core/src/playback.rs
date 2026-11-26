use crate::{config::Config, scheduler::Scheduler, slideshow::Slideshow};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub struct Playback {
    pub show: Option<Slideshow>,
    pub timer: Scheduler,
    pub default_image: Option<PathBuf>,
    paused_before_empty: bool,
}

impl Playback {
    pub fn new(config: &Config, now: Instant) -> Result<Self, String> {
        let show = Slideshow::new(config.images().map_err(|e| e.to_string())?, config.shuffle)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            show: Some(show),
            timer: Scheduler::new(Duration::from_secs(config.interval_seconds), now),
            default_image: None,
            paused_before_empty: false,
        })
    }

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
        let mut candidate =
            Slideshow::new(config.images().map_err(|e| e.to_string())?, config.shuffle)
                .map_err(|e| e.to_string())?;
        apply(candidate.next())?;
        let paused = if self.show.is_some() {
            self.timer.is_paused()
        } else {
            self.paused_before_empty
        };
        self.show = Some(candidate);
        self.default_image = None;
        self.timer = Scheduler::new(Duration::from_secs(config.interval_seconds), now);
        if paused {
            self.timer.pause(now);
        }
        Ok(())
    }
}
