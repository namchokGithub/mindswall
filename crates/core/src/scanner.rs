use crate::Error;
use std::path::PathBuf;
use walkdir::WalkDir;

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
            if entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| {
                        ["jpg", "jpeg", "png", "bmp"]
                            .iter()
                            .any(|supported| ext.eq_ignore_ascii_case(supported))
                    })
            {
                images.push(entry.into_path());
            }
        }
    }
    Ok(images)
}
