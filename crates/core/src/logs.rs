//! Slideshow log files: one file each time the slideshow starts, a new file
//! after midnight, and only the newest files kept.

use chrono::{DateTime, Local, NaiveDate};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How many log files are kept.
pub const KEEP_FILES: usize = 100;
/// How many days a log file is kept after its last line.
pub const KEEP_DAYS: u64 = 30;
/// Timestamp format of every log line: local time with its UTC offset.
pub const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.6f%:z";

const PREFIX: &str = "mindswall-";
const SUFFIX: &str = ".log";
/// The single log file used before logs were split.
const LEGACY_FILE: &str = "mindswall.log";
const LEGACY_MOVED: &str = "mindswall-legacy.log";

/// The folder that holds the log files, in the per-user data directory.
pub fn logs_dir() -> Option<PathBuf> {
    crate::paths::data_dir().map(|dir| dir.join("logs"))
}

/// The most recently written log file, if any.
pub fn latest_log() -> Option<PathBuf> {
    latest_in(&logs_dir()?).ok().flatten()
}

fn is_log_name(name: &str) -> bool {
    name.starts_with(PREFIX) && name.ends_with(SUFFIX)
}

/// The file name for a log started at `time`; `attempt` tells apart logs
/// started within the same second, such as when the CLI hands over to the app.
fn file_name(time: DateTime<Local>, attempt: u32) -> String {
    let stamp = time.format("%Y-%m-%d_%H-%M-%S");
    if attempt == 0 {
        format!("{PREFIX}{stamp}{SUFFIX}")
    } else {
        format!("{PREFIX}{stamp}-{}{SUFFIX}", attempt + 1)
    }
}

/// Log files in `dir` with the time each was last written.
fn log_files(dir: &Path) -> io::Result<Vec<(PathBuf, SystemTime)>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_name().to_str().is_some_and(is_log_name) {
            continue;
        }
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            files.push((entry.path(), metadata.modified()?));
        }
    }
    // Newest first; the name breaks ties because it starts with the time.
    files.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));
    Ok(files)
}

fn latest_in(dir: &Path) -> io::Result<Option<PathBuf>> {
    match log_files(dir) {
        Ok(files) => Ok(files.into_iter().next().map(|(path, _)| path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Which of `files` (newest first) to delete: all but the newest `keep`,
/// and any not written for longer than `max_age`.
fn to_remove(
    files: &[(PathBuf, SystemTime)],
    now: SystemTime,
    keep: usize,
    max_age: Duration,
) -> Vec<PathBuf> {
    files
        .iter()
        .enumerate()
        .filter(|(index, (_, modified))| {
            *index >= keep || now.duration_since(*modified).is_ok_and(|age| age > max_age)
        })
        .map(|(_, (path, _))| path.clone())
        .collect()
}

/// Delete old log files in `dir`, never `current`. Returns how many were
/// deleted; a file that cannot be deleted is left for the next try.
fn prune(dir: &Path, current: &Path) -> io::Result<usize> {
    let files = log_files(dir)?;
    let max_age = Duration::from_secs(KEEP_DAYS * 24 * 60 * 60);
    Ok(to_remove(&files, SystemTime::now(), KEEP_FILES, max_age)
        .into_iter()
        .filter(|path| path != current)
        .filter(|path| fs::remove_file(path).is_ok())
        .count())
}

/// Move the single log file used by earlier versions into `dir`, so it is
/// listed and eventually deleted with the others.
fn move_legacy(data: &Path, dir: &Path) {
    let legacy = data.join(LEGACY_FILE);
    let moved = dir.join(LEGACY_MOVED);
    if legacy.is_file() && !moved.exists() {
        let _ = fs::rename(legacy, moved);
    }
}

/// A log writer that starts a new file each time it is created and again
/// after midnight, local time. Each file begins with a line naming the app,
/// its version, Windows, and the app's memory use.
pub struct LaunchLog {
    dir: PathBuf,
    path: PathBuf,
    file: File,
    day: NaiveDate,
}

impl LaunchLog {
    /// Start a new log file in the standard logs folder, after moving the
    /// old single log file there and deleting old files.
    pub fn start() -> io::Result<Self> {
        let unavailable = || io::Error::other("user data directory unavailable");
        let data = crate::paths::data_dir().ok_or_else(unavailable)?;
        let dir = logs_dir().ok_or_else(unavailable)?;
        fs::create_dir_all(&dir)?;
        move_legacy(&data, &dir);
        Self::start_in(&dir, Local::now())
    }

    fn start_in(dir: &Path, now: DateTime<Local>) -> io::Result<Self> {
        let (path, file) = create_file(dir, now)?;
        let mut log = Self {
            dir: dir.to_path_buf(),
            path,
            file,
            day: now.date_naive(),
        };
        log.write_header(now, "log started")?;
        let _ = prune(&log.dir, &log.path);
        Ok(log)
    }

    /// The file being written.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn write_header(&mut self, now: DateTime<Local>, message: &str) -> io::Result<()> {
        let memory = crate::utilities::memory_mb();
        let app = std::env::current_exe()
            .ok()
            .and_then(|exe| {
                exe.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_default();
        let mut line = format!(
            "{}  INFO mindswall_core::logs: {message} app={app:?} version={} os={:?} pid={} portable={}",
            now.format(TIME_FORMAT),
            env!("CARGO_PKG_VERSION"),
            os_version(),
            std::process::id(),
            crate::paths::is_portable(),
        );
        if let Some(memory) = memory {
            line.push_str(&format!(
                " ram_mb={} commit_mb={}",
                memory.working, memory.commit
            ));
        }
        line.push('\n');
        self.file.write_all(line.as_bytes())
    }

    /// Continue in a new file when the local date has changed since the
    /// current file was started. If the new file cannot be created, writing
    /// continues in the current one.
    fn roll_if_new_day(&mut self, now: DateTime<Local>) {
        if now.date_naive() == self.day {
            return;
        }
        self.day = now.date_naive();
        let Ok((path, file)) = create_file(&self.dir, now) else {
            return;
        };
        let _ = self.file.flush();
        self.path = path;
        self.file = file;
        let _ = self.write_header(now, "log continued after midnight");
        let _ = prune(&self.dir, &self.path);
    }

    fn write_at(&mut self, now: DateTime<Local>, buf: &[u8]) -> io::Result<usize> {
        self.roll_if_new_day(now);
        self.file.write(buf)
    }
}

impl Write for LaunchLog {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_at(Local::now(), buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

/// Create a new log file named after `now`, adding a number when a file with
/// that name already exists.
fn create_file(dir: &Path, now: DateTime<Local>) -> io::Result<(PathBuf, File)> {
    let mut attempt = 0;
    loop {
        let path = dir.join(file_name(now, attempt));
        match OpenOptions::new().append(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && attempt < 99 => {
                attempt += 1;
            }
            Err(error) => return Err(error),
        }
    }
}

/// The Windows edition, version and build, such as
/// "Windows 11 Pro 24H2 build 26100.2894", as the registry reports it.
#[cfg(windows)]
fn os_version() -> String {
    use windows::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
    };
    use windows::core::{HSTRING, w};

    let key = w!(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
    let text = |name: &str| -> Option<String> {
        let mut buffer = [0u16; 256];
        let mut size = std::mem::size_of_val(&buffer) as u32;
        // SAFETY: `buffer` holds `size` bytes and outlives the call.
        unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                key,
                &HSTRING::from(name),
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut size),
            )
        }
        .ok()
        .ok()?;
        let length = buffer.iter().position(|&unit| unit == 0).unwrap_or(0);
        Some(String::from_utf16_lossy(&buffer[..length]))
    };
    let number = |name: &str| -> Option<u32> {
        let mut value = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        // SAFETY: `value` holds `size` bytes and outlives the call.
        unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                key,
                &HSTRING::from(name),
                RRF_RT_REG_DWORD,
                None,
                Some((&mut value as *mut u32).cast()),
                Some(&mut size),
            )
        }
        .ok()
        .ok()?;
        Some(value)
    };
    let build = text("CurrentBuildNumber").unwrap_or_default();
    // Windows 11 still reports "Windows 10" as its product name.
    let product = text("ProductName").unwrap_or_else(|| "Windows".into());
    let product = match build.parse::<u32>() {
        Ok(number) if number >= 22000 => product.replacen("Windows 10", "Windows 11", 1),
        _ => product,
    };
    let mut version = product;
    if let Some(display) = text("DisplayVersion") {
        version.push_str(&format!(" {display}"));
    }
    version.push_str(&format!(" build {build}"));
    if let Some(revision) = number("UBR") {
        version.push_str(&format!(".{revision}"));
    }
    version
}

#[cfg(not(windows))]
fn os_version() -> String {
    std::env::consts::OS.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(day: u32, hour: u32, minute: u32, second: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 11, day, hour, minute, second)
            .single()
            .expect("an unambiguous local time")
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn each_start_gets_its_own_file_named_after_the_local_time() {
        let dir = tempfile::tempdir().unwrap();
        let first = LaunchLog::start_in(dir.path(), at(27, 10, 34, 58)).unwrap();
        let second = LaunchLog::start_in(dir.path(), at(27, 10, 34, 58)).unwrap();
        assert_ne!(first.path(), second.path());
        assert_eq!(
            names(dir.path()),
            [
                "mindswall-2026-11-27_10-34-58-2.log",
                "mindswall-2026-11-27_10-34-58.log"
            ]
        );
        let text = fs::read_to_string(first.path()).unwrap();
        assert!(text.starts_with("2026-11-27T10:34:58.000000"), "{text}");
        assert!(
            text.contains("INFO mindswall_core::logs: log started app="),
            "{text}"
        );
        assert!(text.contains(&format!("version={}", env!("CARGO_PKG_VERSION"))));
    }

    #[test]
    fn a_new_file_starts_after_midnight() {
        let dir = tempfile::tempdir().unwrap();
        let mut log = LaunchLog::start_in(dir.path(), at(27, 23, 59, 0)).unwrap();
        log.write_at(at(27, 23, 59, 30), b"before midnight\n")
            .unwrap();
        let first = log.path().to_path_buf();
        log.write_at(at(28, 0, 0, 1), b"after midnight\n").unwrap();
        log.flush().unwrap();
        assert_eq!(
            log.path(),
            dir.path().join("mindswall-2026-11-28_00-00-01.log")
        );
        let before = fs::read_to_string(first).unwrap();
        let after = fs::read_to_string(log.path()).unwrap();
        assert!(before.ends_with("before midnight\n") && !before.contains("after"));
        assert!(after.contains("log continued after midnight"), "{after}");
        assert!(after.ends_with("after midnight\n"));
    }

    #[test]
    fn only_the_newest_and_recent_files_are_kept() {
        let now = SystemTime::now();
        let day = Duration::from_secs(24 * 60 * 60);
        let files: Vec<_> = (0..5)
            .map(|index| {
                (
                    PathBuf::from(format!("{index}.log")),
                    now - day * index * 10,
                )
            })
            .collect();
        // Ages 0, 10, 20, 30 and 40 days, newest first.
        let removed = to_remove(
            &files,
            now,
            3,
            Duration::from_secs(KEEP_DAYS) * 24 * 60 * 60,
        );
        assert_eq!(removed, [PathBuf::from("3.log"), PathBuf::from("4.log")]);
        let removed = to_remove(&files, now, 2, day * 365);
        assert_eq!(
            removed,
            [
                PathBuf::from("2.log"),
                PathBuf::from("3.log"),
                PathBuf::from("4.log")
            ]
        );
    }

    #[test]
    fn pruning_keeps_the_current_file_and_ignores_other_files() {
        let dir = tempfile::tempdir().unwrap();
        for index in 0..KEEP_FILES + 3 {
            fs::write(dir.path().join(format!("mindswall-{index:04}.log")), "").unwrap();
        }
        fs::write(dir.path().join("notes.txt"), "").unwrap();
        let current = dir.path().join("mindswall-0000.log");
        // The files share a time, so the name orders them and 0000 is oldest.
        assert_eq!(prune(dir.path(), &current).unwrap(), 2);
        let left = names(dir.path());
        assert!(left.contains(&"notes.txt".to_owned()));
        assert!(left.contains(&"mindswall-0000.log".to_owned()));
        assert!(!left.contains(&"mindswall-0001.log".to_owned()));
        assert_eq!(left.len(), KEEP_FILES + 2);
    }

    #[test]
    fn the_old_single_log_moves_into_the_logs_folder_once() {
        let data = tempfile::tempdir().unwrap();
        let dir = data.path().join("logs");
        fs::create_dir(&dir).unwrap();
        fs::write(data.path().join(LEGACY_FILE), "old").unwrap();
        move_legacy(data.path(), &dir);
        assert_eq!(fs::read_to_string(dir.join(LEGACY_MOVED)).unwrap(), "old");
        fs::write(data.path().join(LEGACY_FILE), "newer").unwrap();
        move_legacy(data.path(), &dir);
        assert_eq!(fs::read_to_string(dir.join(LEGACY_MOVED)).unwrap(), "old");
        assert_eq!(latest_in(&dir).unwrap(), Some(dir.join(LEGACY_MOVED)));
        assert_eq!(latest_in(&data.path().join("missing")).unwrap(), None);
    }
}
