use crate::Error;
use std::path::Path;
#[derive(serde::Serialize)]
pub struct MonitorInfo {
    pub id: String,
    pub width: i32,
    pub height: i32,
}
pub trait WallpaperSetter {
    fn set_wallpaper(&self, path: &Path) -> Result<(), Error>;
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
