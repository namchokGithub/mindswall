use crate::{Error, scanner};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    pub path: PathBuf,
    pub enabled: bool,
    pub recursive: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub folders: Vec<Folder>,
    pub interval_seconds: u64,
    pub shuffle: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            folders: Vec::new(),
            interval_seconds: 600,
            shuffle: true,
        }
    }
}
impl Config {
    pub fn default_path() -> io::Result<PathBuf> {
        directories::ProjectDirs::from("", "", "MindsWall")
            .map(|d| d.config_dir().join("config.json"))
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "user config directory unavailable")
            })
    }
    pub fn load(path: &Path) -> io::Result<Self> {
        let contents = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e),
        };
        match serde_json::from_slice::<Self>(&contents) {
            Ok(config)
                if config.version == 1 && (60..=86400).contains(&config.interval_seconds) =>
            {
                Ok(config)
            }
            _ => {
                tracing::warn!(path = ?path, "invalid configuration; using defaults");
                Ok(Self::default())
            }
        }
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
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
    pub fn images(&self) -> Result<Vec<PathBuf>, Error> {
        let mut seen = std::collections::HashSet::new();
        let mut images = Vec::new();
        for folder in self.folders.iter().filter(|f| f.enabled) {
            for image in scanner::scan(std::slice::from_ref(&folder.path), folder.recursive)? {
                let identity = fs::canonicalize(&image).unwrap_or_else(|_| image.clone());
                if seen.insert(identity) {
                    images.push(image);
                }
            }
        }
        images.sort();
        Ok(images)
    }
}
