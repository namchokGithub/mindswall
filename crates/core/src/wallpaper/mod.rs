use crate::Error;
use std::path::{Path, PathBuf};
/// A connected monitor. Coordinates are in desktop pixels; the primary
/// monitor's top-left corner is at (0, 0).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitorInfo {
    /// Windows device path, used as the monitor key in the configuration.
    pub id: String,
    /// The name the monitor reports, such as "DELL P2419H", when available.
    pub name: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub primary: bool,
    /// The image Windows shows on this monitor, when it reports one.
    pub current: Option<PathBuf>,
}
/// How many recently set wallpapers are remembered.
const REMEMBERED: usize = 64;

/// Files MindsWall set as wallpaper during this run, newest last, compared
/// by [`path_key`].
static APPLIED: std::sync::Mutex<std::collections::VecDeque<String>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());

/// A path as Windows may report it back: without the `\\?\` prefix that
/// `canonicalize` adds, with backslashes, and ignoring case.
fn path_key(path: &Path) -> String {
    let text = path.to_string_lossy().replace('/', "\\");
    let text = text
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .or_else(|| text.strip_prefix(r"\\?\").map(str::to_owned))
        .unwrap_or(text);
    text.to_lowercase()
}

/// Remember that MindsWall set `path` as a wallpaper.
pub fn remember_applied(path: &Path) {
    if let Ok(mut applied) = APPLIED.lock() {
        remember_in(&mut applied, path);
    }
}

fn remember_in(applied: &mut std::collections::VecDeque<String>, path: &Path) {
    let key = path_key(path);
    applied.retain(|known| *known != key);
    applied.push_back(key);
    while applied.len() > REMEMBERED {
        applied.pop_front();
    }
}

/// Whether the wallpaper on `monitors` was set by something other than
/// MindsWall, such as the Windows Settings app: some monitor shows an image
/// MindsWall did not set, or no monitor shows an image (a solid colour).
/// Returns a short description for the log, or `None` when the wallpaper is
/// MindsWall's or nothing was set yet in this run, so it cannot be told.
pub fn changed_outside(monitors: &[MonitorInfo]) -> Option<String> {
    let applied = APPLIED.lock().ok()?;
    changed_outside_of(&applied, monitors)
}

fn changed_outside_of(
    applied: &std::collections::VecDeque<String>,
    monitors: &[MonitorInfo],
) -> Option<String> {
    if applied.is_empty() || monitors.is_empty() {
        return None;
    }
    if let Some(other) = monitors
        .iter()
        .filter_map(|monitor| monitor.current.as_deref())
        .find(|current| !applied.contains(&path_key(current)))
    {
        return Some(other.display().to_string());
    }
    monitors
        .iter()
        .all(|monitor| monitor.current.is_none())
        .then(|| "no image (a solid colour)".to_owned())
}

pub trait WallpaperSetter {
    fn set_wallpaper(&self, path: &Path) -> Result<(), Error>;
}

#[cfg(windows)]
pub fn default_wallpaper_path() -> Result<std::path::PathBuf, String> {
    let root = std::env::var_os("SystemRoot").ok_or("Windows system directory is unavailable")?;
    let path = std::path::PathBuf::from(root)
        .join("Web")
        .join("Wallpaper")
        .join("Windows")
        .join("img0.jpg");
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!(
            "Windows default wallpaper was not found: {}",
            path.display()
        ))
    }
}

#[cfg(not(windows))]
pub fn default_wallpaper_path() -> Result<std::path::PathBuf, String> {
    Err("Windows default wallpaper is only available on Windows".into())
}
#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{SystemWallpaper, list_monitors, set_monitor_wallpaper};
#[cfg(not(windows))]
pub struct SystemWallpaper;
#[cfg(not(windows))]
impl WallpaperSetter for SystemWallpaper {
    fn set_wallpaper(&self, _path: &Path) -> Result<(), Error> {
        Err(Error::UnsupportedPlatform)
    }
}
#[cfg(not(windows))]
pub fn list_monitors() -> Result<Vec<MonitorInfo>, String> {
    Err("Monitors require Windows".into())
}
#[cfg(not(windows))]
pub fn set_monitor_wallpaper(_: &str, _: &Path) -> Result<(), String> {
    Err("Monitors require Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(current: Option<&str>) -> MonitorInfo {
        MonitorInfo {
            id: "monitor".into(),
            name: None,
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            primary: true,
            current: current.map(PathBuf::from),
        }
    }

    #[test]
    fn only_wallpapers_set_elsewhere_count_as_changed() {
        let mut applied = std::collections::VecDeque::new();
        let ours = r"\\?\C:\Pictures\Ours.jpg";
        // Nothing set yet: the current wallpaper cannot be judged.
        assert_eq!(changed_outside_of(&applied, &[monitor(None)]), None);
        remember_in(&mut applied, Path::new(ours));
        // Windows reports the path without the prefix and in another case.
        assert_eq!(
            changed_outside_of(&applied, &[monitor(Some(r"c:\pictures\ours.JPG"))]),
            None
        );
        let other = r"C:\Windows\Web\Wallpaper\Windows\img19.jpg";
        assert_eq!(
            changed_outside_of(&applied, &[monitor(Some(ours)), monitor(Some(other))]).as_deref(),
            Some(other)
        );
        // One monitor without an image is not enough; all of them is a colour.
        assert_eq!(
            changed_outside_of(&applied, &[monitor(Some(ours)), monitor(None)]),
            None
        );
        assert!(changed_outside_of(&applied, &[monitor(None), monitor(None)]).is_some());
        assert_eq!(changed_outside_of(&applied, &[]), None);
    }

    #[test]
    fn only_recent_wallpapers_are_remembered() {
        let mut applied = std::collections::VecDeque::new();
        for index in 0..=REMEMBERED {
            remember_in(
                &mut applied,
                &PathBuf::from(format!(r"D:\Walls\{index}.jpg")),
            );
        }
        assert!(changed_outside_of(&applied, &[monitor(Some(r"D:\Walls\0.jpg"))]).is_some());
        assert_eq!(
            changed_outside_of(&applied, &[monitor(Some(r"D:\Walls\1.jpg"))]),
            None
        );
        // Setting a wallpaper again keeps one entry for it.
        remember_in(&mut applied, Path::new(r"D:\Walls\1.jpg"));
        assert_eq!(applied.len(), REMEMBERED);
    }
}
