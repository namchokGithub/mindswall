use crate::Error;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Images found by [`scan_lenient`], together with the folders and entries
/// that could not be read.
#[derive(Debug, Default)]
pub struct ScanReport {
    pub images: Vec<PathBuf>,
    pub skipped: Vec<Error>,
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["jpg", "jpeg", "png", "bmp"]
                .iter()
                .any(|supported| ext.eq_ignore_ascii_case(supported))
        })
}

/// Scan each root, returning an error if a directory entry cannot be read.
pub fn scan(roots: &[PathBuf], recursive: bool) -> Result<Vec<PathBuf>, Error> {
    let mut images = Vec::new();
    for root in roots {
        if !root.is_dir() {
            return Err(Error::InvalidFolder(root.clone()));
        }
        let walker = WalkDir::new(root).max_depth(if recursive { usize::MAX } else { 1 });
        for entry in walker {
            let entry = entry.map_err(|source| Error::Scan {
                path: source.path().unwrap_or(root).to_owned(),
                source,
            })?;
            if entry.file_type().is_file() && is_image(entry.path()) {
                images.push(entry.into_path());
            }
        }
    }
    Ok(images)
}

/// Scan each root, skipping folders and entries that cannot be read and
/// reporting them in [`ScanReport::skipped`] instead of failing the whole scan.
pub fn scan_lenient(roots: &[PathBuf], recursive: bool) -> ScanReport {
    let mut report = ScanReport::default();
    for root in roots {
        if !root.is_dir() {
            report.skipped.push(Error::InvalidFolder(root.clone()));
            continue;
        }
        let walker = WalkDir::new(root).max_depth(if recursive { usize::MAX } else { 1 });
        for entry in walker {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() && is_image(entry.path()) {
                        report.images.push(entry.into_path());
                    }
                }
                Err(source) => report.skipped.push(Error::Scan {
                    path: source.path().unwrap_or(root).to_owned(),
                    source,
                }),
            }
        }
    }
    report
}
