//! Where MindsWall keeps its files.
//!
//! Installed builds use the per-user folders (`%APPDATA%\MindsWall` for
//! settings, history and logs, `%LOCALAPPDATA%\MindsWall\cache` for caches).
//! A portable build has a `portable.txt` file next to the program; it keeps
//! everything in `config`, `data` and `cache` folders next to the program
//! instead, so the whole folder can be moved or deleted.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The file that marks a portable build when it sits next to the program.
pub const PORTABLE_MARKER: &str = "portable.txt";

/// The folder next to the program when it is a portable build.
pub fn portable_root() -> Option<&'static Path> {
    static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();
    ROOT.get_or_init(|| {
        let exe = std::env::current_exe().ok()?;
        portable_root_in(exe.parent()?)
    })
    .as_deref()
}

/// Whether this is a portable build.
pub fn is_portable() -> bool {
    portable_root().is_some()
}

fn portable_root_in(dir: &Path) -> Option<PathBuf> {
    dir.join(PORTABLE_MARKER)
        .is_file()
        .then(|| dir.to_path_buf())
}

fn project() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("", "", "MindsWall")
}

/// Settings (`config.json`).
pub fn config_dir() -> Option<PathBuf> {
    match portable_root() {
        Some(root) => Some(root.join("config")),
        None => project().map(|dirs| dirs.config_dir().to_path_buf()),
    }
}

/// History and logs.
pub fn data_dir() -> Option<PathBuf> {
    match portable_root() {
        Some(root) => Some(root.join("data")),
        None => project().map(|dirs| dirs.data_dir().to_path_buf()),
    }
}

/// Thumbnails, previews, combined wallpapers and edited images.
pub fn cache_dir() -> Option<PathBuf> {
    match portable_root() {
        Some(root) => Some(root.join("cache")),
        None => project().map(|dirs| dirs.cache_dir().to_path_buf()),
    }
}

/// The WebView2 profile for a portable build. Installed builds return `None`
/// and keep the WebView's default per-user folder.
pub fn webview_dir() -> Option<PathBuf> {
    portable_root().map(|root| root.join("data").join("webview"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_folder_with_the_marker_is_portable() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(portable_root_in(dir.path()), None);
        // A folder with the marker's name does not count.
        std::fs::create_dir(dir.path().join(PORTABLE_MARKER)).unwrap();
        assert_eq!(portable_root_in(dir.path()), None);
        let other = tempfile::tempdir().unwrap();
        std::fs::write(other.path().join(PORTABLE_MARKER), "").unwrap();
        assert_eq!(
            portable_root_in(other.path()),
            Some(other.path().to_path_buf())
        );
    }

    #[test]
    fn test_programs_are_not_portable() {
        // Test binaries run from the build folder, which has no marker.
        assert!(!is_portable());
        assert!(webview_dir().is_none());
    }
}
