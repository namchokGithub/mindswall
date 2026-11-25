use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Recursively find supported image files, skipping entries that cannot be read.
pub fn scan(folder: &Path) -> Vec<PathBuf> {
    WalkDir::new(folder)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    ["jpg", "jpeg", "png", "bmp"]
                        .iter()
                        .any(|supported| ext.eq_ignore_ascii_case(supported))
                })
        })
        .map(|entry| entry.into_path())
        .collect()
}
