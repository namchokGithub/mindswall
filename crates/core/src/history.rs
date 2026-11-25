use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub path: PathBuf,
    pub timestamp: u64,
    pub monitor: Option<String>,
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
}
impl History {
    pub fn default_path() -> io::Result<PathBuf> {
        directories::ProjectDirs::from("", "", "MindsWall")
            .map(|d| d.data_dir().join("history.json"))
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "user data directory unavailable")
            })
    }
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }
    pub fn push(&mut self, entry: HistoryEntry) {
        self.entries.insert(0, entry);
        self.entries.truncate(100);
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent"))?;
        fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer(&mut file, self)?;
        file.flush()?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
    pub fn record(path: &Path, monitor: Option<String>) -> io::Result<()> {
        let file = Self::default_path()?;
        let parent = file
            .parent()
            .ok_or_else(|| io::Error::other("missing history directory"))?;
        fs::create_dir_all(parent)?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(parent.join("history.lock"))?;
        lock.lock()?;
        let mut history = Self::load(&file)?;
        history.push(HistoryEntry {
            path: path.to_owned(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            monitor,
        });
        history.save(&file)?;
        Ok(())
    }
}
