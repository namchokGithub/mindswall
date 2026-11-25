pub mod scanner;
pub mod wallpaper;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("not a readable folder: {0:?}")]
    InvalidFolder(PathBuf),
    #[error("cannot scan {path:?}: {source}")]
    Scan {
        path: PathBuf,
        #[source]
        source: walkdir::Error,
    },
    #[error("not a readable file: {0:?}")]
    InvalidFile(PathBuf),
    #[error("no supported images found")]
    NoImages,
    #[error("cannot set wallpaper {path:?}: {message}")]
    Wallpaper { path: PathBuf, message: String },
    #[error("wallpaper setting is supported only on Windows")]
    UnsupportedPlatform,
}

pub fn random_image(images: &[PathBuf]) -> Result<&std::path::Path, Error> {
    use rand::seq::IndexedRandom;
    images
        .choose(&mut rand::rng())
        .map(|p| p.as_path())
        .ok_or(Error::NoImages)
}
