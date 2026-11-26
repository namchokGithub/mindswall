use crate::{Error, scanner};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

static QUARANTINED: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Move an invalid data file aside to `<name>.invalid-<unix seconds>`.
/// Returns `None` when another process already moved or removed it.
pub(crate) fn quarantine(path: &Path) -> io::Result<Option<PathBuf>> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "configuration has no name"))?;
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    for attempt in 0..100u32 {
        let mut backup_name = name.to_owned();
        backup_name.push(format!(".invalid-{seconds}"));
        if attempt > 0 {
            backup_name.push(format!("-{attempt}"));
        }
        let backup = path.with_file_name(backup_name);
        if backup.exists() {
            continue;
        }
        return match fs::rename(path, &backup) {
            Ok(()) => Ok(Some(backup)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io::Error::new(
                e.kind(),
                format!("invalid configuration {path:?} could not be backed up: {e}"),
            )),
        };
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("invalid configuration {path:?} could not be backed up: too many backups"),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub path: PathBuf,
    pub enabled: bool,
    pub recursive: bool,
}
/// Folders that feed a separate slideshow on one monitor. The monitor is
/// identified by its Windows device path, as listed by `list_monitors`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorFolders {
    pub monitor: String,
    pub folders: Vec<Folder>,
}
/// The desktop app's color theme. `System` follows the Windows app mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}
/// Combine portrait images side by side on a landscape monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CollageSettings {
    pub enabled: bool,
    /// Space between images in pixels, filled by the blurred background.
    pub gap: u32,
}
impl Default for CollageSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            gap: 4,
        }
    }
}
/// A name the user gave a monitor, shown instead of its device path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorName {
    pub monitor: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSettings {
    pub folders: Vec<Folder>,
    pub interval_seconds: u64,
    pub shuffle: bool,
    pub filter: crate::utilities::ImageFilter,
    pub excluded: Vec<PathBuf>,
    pub fit_mode: crate::utilities::FitMode,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub settings: ProfileSettings,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotkeys {
    pub next: String,
    pub previous: String,
    pub pause: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub folders: Vec<Folder>,
    pub interval_seconds: u64,
    pub shuffle: bool,
    #[serde(default)]
    pub filter: crate::utilities::ImageFilter,
    #[serde(default)]
    pub excluded: Vec<PathBuf>,
    #[serde(default)]
    pub fit_mode: crate::utilities::FitMode,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub active_profile: Option<String>,
    #[serde(default)]
    pub hotkeys: Hotkeys,
    /// Monitors with their own folders. Other monitors show the main
    /// slideshow.
    #[serde(default)]
    pub monitor_folders: Vec<MonitorFolders>,
    /// Names the user gave monitors. A monitor without a name is shown with
    /// the name Windows reports for it.
    #[serde(default)]
    pub monitor_names: Vec<MonitorName>,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub collage: CollageSettings,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            folders: Vec::new(),
            interval_seconds: 600,
            shuffle: true,
            filter: Default::default(),
            excluded: Vec::new(),
            fit_mode: Default::default(),
            profiles: Vec::new(),
            active_profile: None,
            hotkeys: Hotkeys::default(),
            monitor_folders: Vec::new(),
            monitor_names: Vec::new(),
            theme: Theme::System,
            collage: CollageSettings::default(),
        }
    }
}
impl Config {
    pub fn validate(&self) -> io::Result<()> {
        let mut shortcuts = std::collections::HashSet::new();
        for key in [
            &self.hotkeys.next,
            &self.hotkeys.previous,
            &self.hotkeys.pause,
        ] {
            if key.len() > 80
                || (!key.trim().is_empty() && !shortcuts.insert(key.trim().to_lowercase()))
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid or duplicate hotkey",
                ));
            }
        }
        if self.version != 1
            || !(60..=86400).contains(&self.interval_seconds)
            || self.profiles.len() > 100
            || self.folders.len() > 1000
            || self.excluded.len() > 100000
            || self.monitor_folders.len() > 64
            || self.monitor_names.len() > 64
            || self.collage.gap > crate::collage::MAX_GAP
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid configuration version, interval, or collection size",
            ));
        }
        let mut monitors = std::collections::HashSet::new();
        for entry in &self.monitor_folders {
            if entry.monitor.trim().is_empty()
                || entry.monitor.len() > 512
                || entry.folders.len() > 1000
                || !monitors.insert(entry.monitor.to_lowercase())
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid or duplicate monitor folders",
                ));
            }
        }
        let mut named = std::collections::HashSet::new();
        for entry in &self.monitor_names {
            let name = entry.name.trim();
            if entry.monitor.trim().is_empty()
                || entry.monitor.len() > 512
                || name.is_empty()
                || name.chars().count() > 40
                || name.len() != entry.name.len()
                || !named.insert(entry.monitor.to_lowercase())
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid or duplicate monitor name",
                ));
            }
        }
        let mut names = std::collections::HashSet::new();
        for profile in &self.profiles {
            let name = profile.name.trim();
            if name.is_empty()
                || name.len() > 80
                || !names.insert(name.to_lowercase())
                || !(60..=86400).contains(&profile.settings.interval_seconds)
                || profile.settings.folders.len() > 1000
                || profile.settings.excluded.len() > 100000
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid or duplicate profile",
                ));
            }
        }
        if self
            .active_profile
            .as_ref()
            .is_some_and(|name| !self.profiles.iter().any(|p| &p.name == name))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Active profile does not exist",
            ));
        }
        Ok(())
    }
    pub fn parse_backup(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Backup is too large",
            ));
        }
        let config: Self = serde_json::from_slice(bytes).map_err(io::Error::other)?;
        config.validate()?;
        Ok(config)
    }
    pub fn save_profile(&mut self, name: &str) -> io::Result<()> {
        let name = name.trim();
        if name.is_empty() || name.len() > 80 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Profile name must contain 1–80 characters",
            ));
        }
        if self
            .profiles
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case(name))
        {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Profile name already exists",
            ));
        }
        let profile = Profile {
            name: name.to_owned(),
            settings: self.profile_settings(),
        };
        self.profiles.push(profile);
        self.active_profile = Some(name.to_owned());
        self.validate()
    }
    /// The current settings that a profile stores.
    pub fn profile_settings(&self) -> ProfileSettings {
        ProfileSettings {
            folders: self.folders.clone(),
            interval_seconds: self.interval_seconds,
            shuffle: self.shuffle,
            filter: self.filter.clone(),
            excluded: self.excluded.clone(),
            fit_mode: self.fit_mode,
        }
    }
    /// Whether the current settings differ from the active profile's saved
    /// settings. `false` when no profile is active.
    pub fn active_profile_changed(&self) -> bool {
        self.active_profile.as_ref().is_some_and(|name| {
            self.profiles
                .iter()
                .find(|profile| &profile.name == name)
                .is_some_and(|profile| profile.settings != self.profile_settings())
        })
    }
    /// Save the current settings into an existing profile and make it active.
    pub fn update_profile(&mut self, name: &str) -> io::Result<()> {
        let settings = self.profile_settings();
        let profile = self
            .profiles
            .iter_mut()
            .find(|p| p.name == name)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Profile not found"))?;
        profile.settings = settings;
        self.active_profile = Some(name.to_owned());
        self.validate()
    }
    pub fn apply_profile(&mut self, name: &str) -> io::Result<()> {
        let profile = self
            .profiles
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Profile not found"))?
            .clone();
        self.folders = profile.settings.folders;
        self.interval_seconds = profile.settings.interval_seconds;
        self.shuffle = profile.settings.shuffle;
        self.filter = profile.settings.filter;
        self.excluded = profile.settings.excluded;
        self.fit_mode = profile.settings.fit_mode;
        self.active_profile = Some(profile.name);
        Ok(())
    }
    pub fn default_path() -> io::Result<PathBuf> {
        directories::ProjectDirs::from("", "", "MindsWall")
            .map(|d| d.config_dir().join("config.json"))
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "user config directory unavailable")
            })
    }
    /// Load the configuration, or defaults when the file does not exist.
    ///
    /// An invalid file is moved aside to `<name>.invalid-<unix seconds>` before
    /// defaults are returned, so a later save cannot overwrite the user's
    /// settings. The backup path is available from [`Config::take_quarantined`].
    pub fn load(path: &Path) -> io::Result<Self> {
        let contents = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e),
        };
        match Self::parse_backup(&contents) {
            Ok(config) => Ok(config),
            Err(error) => {
                let backup = quarantine(path)?;
                tracing::warn!(
                    path = ?path,
                    backup = ?backup,
                    %error,
                    "invalid configuration moved aside; using defaults"
                );
                if let Some(backup) = backup
                    && let Ok(mut slot) = QUARANTINED.lock()
                {
                    *slot = Some(backup);
                }
                Ok(Self::default())
            }
        }
    }
    /// Return and clear the backup path of the last invalid configuration
    /// that [`Config::load`] moved aside in this process.
    pub fn take_quarantined() -> Option<PathBuf> {
        QUARANTINED.lock().ok().and_then(|mut slot| slot.take())
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        self.validate()?;
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "configuration has no parent")
        })?;
        fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(&mut file, self)?;
        file.flush()?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|err| err.error)?;
        Ok(())
    }
    /// Collect the images of all enabled folders. Folders and entries that
    /// cannot be read are logged and skipped so one unavailable location does
    /// not hide the rest of the collection.
    pub fn images(&self) -> Result<Vec<PathBuf>, Error> {
        self.images_in(&self.folders)
    }
    /// Collect the images of the enabled folders in `folders`, applying this
    /// configuration's filter and exclusions.
    pub fn images_in(&self, folders: &[Folder]) -> Result<Vec<PathBuf>, Error> {
        let mut seen = std::collections::HashSet::new();
        let mut images = Vec::new();
        let excluded: std::collections::HashSet<_> = self
            .excluded
            .iter()
            .map(|path| crate::utilities::path_key(path))
            .collect();
        for folder in folders.iter().filter(|f| f.enabled) {
            let report =
                scanner::scan_lenient(std::slice::from_ref(&folder.path), folder.recursive);
            for error in &report.skipped {
                tracing::warn!(%error, "skipping unreadable location");
            }
            for image in report.images {
                let identity = fs::canonicalize(&image).unwrap_or_else(|_| image.clone());
                if excluded.contains(&crate::utilities::path_key(&image)) {
                    continue;
                }
                let filtered = self.filter != Default::default();
                if filtered
                    && !crate::utilities::dimensions(&image)
                        .is_ok_and(|(w, h)| crate::utilities::matches_filter(w, h, &self.filter))
                {
                    continue;
                }
                if seen.insert(identity) {
                    images.push(image);
                }
            }
        }
        images.sort();
        Ok(images)
    }
    /// Update a collection for paths reported by a folder watcher without
    /// rescanning every folder. Changed paths and anything below them are
    /// removed, then files and folders that still exist are checked again.
    /// New images must have a readable header, so files still being written
    /// are skipped until a later change event.
    pub fn apply_changes(
        &self,
        current: &[PathBuf],
        changed: &std::collections::HashSet<PathBuf>,
    ) -> Vec<PathBuf> {
        self.apply_changes_in(&self.folders, current, changed)
    }
    /// Like [`Config::apply_changes`] for a collection built from `folders`.
    pub fn apply_changes_in(
        &self,
        folders: &[Folder],
        current: &[PathBuf],
        changed: &std::collections::HashSet<PathBuf>,
    ) -> Vec<PathBuf> {
        let excluded: std::collections::HashSet<_> = self
            .excluded
            .iter()
            .map(|path| crate::utilities::path_key(path))
            .collect();
        let filtered = self.filter != Default::default();
        let accept = |image: &Path| {
            scanner::is_image(image)
                && image.is_file()
                && folders.iter().any(|folder| {
                    folder.enabled
                        && image.starts_with(&folder.path)
                        && (folder.recursive || image.parent() == Some(folder.path.as_path()))
                })
                && !excluded.contains(&crate::utilities::path_key(image))
                && crate::utilities::dimensions(image).is_ok_and(|(w, h)| {
                    !filtered || crate::utilities::matches_filter(w, h, &self.filter)
                })
        };
        let mut images: Vec<PathBuf> = current
            .iter()
            .filter(|image| !changed.iter().any(|path| image.starts_with(path)))
            .cloned()
            .collect();
        for path in changed {
            if path.is_dir() {
                let report = scanner::scan_lenient(std::slice::from_ref(path), true);
                images.extend(report.images.into_iter().filter(|image| accept(image)));
            } else if accept(path) {
                images.push(path.clone());
            }
        }
        images.sort();
        images.dedup();
        images
    }
}
