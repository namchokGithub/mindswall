use crate::Error;
use std::path::Path;
pub trait WallpaperSetter {
    fn set_wallpaper(&self, path: &Path) -> Result<(), Error>;
}
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::SystemWallpaper;
#[cfg(not(windows))]
pub struct SystemWallpaper;
#[cfg(not(windows))]
impl WallpaperSetter for SystemWallpaper {
    fn set_wallpaper(&self, _path: &Path) -> Result<(), Error> {
        Err(Error::UnsupportedPlatform)
    }
}
