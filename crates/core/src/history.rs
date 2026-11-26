use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const HISTORY_LIMIT: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub path: PathBuf,
    pub timestamp: u64,
    pub monitor: Option<String>,
    /// Images shown next to `path` in a combined wallpaper.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub combined: Vec<PathBuf>,
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
    /// Load the history, or an empty one when the file does not exist. An
    /// invalid file is moved aside to `<name>.invalid-<unix seconds>` so new
    /// entries can be recorded again.
    pub fn load(path: &Path) -> io::Result<Self> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e),
        };
        match serde_json::from_slice::<Self>(&bytes) {
            Ok(mut history) => {
                history.entries.truncate(HISTORY_LIMIT);
                Ok(history)
            }
            Err(error) => {
                let backup = crate::config::quarantine(path)?;
                tracing::warn!(
                    path = ?path,
                    backup = ?backup,
                    %error,
                    "invalid wallpaper history moved aside; starting a new history"
                );
                Ok(Self::default())
            }
        }
    }
    pub fn push(&mut self, entry: HistoryEntry) {
        self.entries.insert(0, entry);
        self.entries.truncate(HISTORY_LIMIT);
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
        Self::record_combined(path, monitor, Vec::new())
    }
    /// Record a wallpaper made from `path` and the images combined with it.
    pub fn record_combined(
        path: &Path,
        monitor: Option<String>,
        combined: Vec<PathBuf>,
    ) -> io::Result<()> {
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
            combined,
        });
        history.save(&file)?;
        Ok(())
    }
}
