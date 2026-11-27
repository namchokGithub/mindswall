//! What the setup writes, independent of Windows: names, folders, registry
//! values, and checks. The Windows calls are in `system`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const APP_NAME: &str = "MindsWall";
pub const APP_EXE: &str = "MindsWall.exe";
pub const UNINSTALLER: &str = "uninstall.exe";
pub const SHORTCUT: &str = "MindsWall.lnk";
pub const PUBLISHER: &str = "NamchokGithub";
pub const HOMEPAGE: &str = "https://github.com/namchokGithub/mindswall";
/// The setup and the app's argument for starting the app in the tray, as
/// "Start with Windows" does.
pub const HIDDEN_ARG: &str = "--hidden";
pub const UNINSTALL_ARG: &str = "--uninstall";
/// The uninstaller copies itself to the temporary folder and runs from there
/// with this argument and the install folder, so it can delete itself.
pub const FROM_TEMP_ARG: &str = "--from-temp";

/// The app's entry under Windows' installed apps, for the current user.
pub const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\MindsWall";
/// "Start with Windows" entries, written the same way as the app's own
/// setting (the `tauri-plugin-autostart` crate) so both agree.
pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const STARTUP_APPROVED_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
/// Task Manager's "enabled" marker for a startup entry.
pub const STARTUP_ENABLED: [u8; 12] = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Install,
    Uninstall,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub version: String,
    pub location: String,
    pub start_with_windows: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupInfo {
    pub mode: Mode,
    pub version: String,
    pub default_location: String,
    pub installed: Option<Installed>,
    pub app_running: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallOptions {
    pub location: String,
    pub desktop_shortcut: bool,
    pub start_with_windows: bool,
    // "Launch after installation" is handled by the window, which calls
    // `launch_app` when setup finishes.
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallOptions {
    pub delete_data: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Progress {
    pub percent: u8,
    pub step: String,
}

/// Reports progress through `steps` named in order.
pub struct Steps<'a> {
    total: usize,
    done: usize,
    report: &'a dyn Fn(Progress),
}

impl<'a> Steps<'a> {
    pub fn new(total: usize, report: &'a dyn Fn(Progress)) -> Self {
        Self {
            total,
            done: 0,
            report,
        }
    }

    /// Announce the next step.
    pub fn start(&mut self, step: &str) {
        let percent = (self.done * 100 / self.total.max(1)).min(99) as u8;
        self.done += 1;
        (self.report)(Progress {
            percent,
            step: step.to_owned(),
        });
    }

    pub fn finish(&self) {
        (self.report)(Progress {
            percent: 100,
            step: "Done".into(),
        });
    }
}

/// Where MindsWall is installed when nothing is installed yet:
/// `%LOCALAPPDATA%\Programs\MindsWall`, as other per-user apps do.
pub fn default_location(local_app_data: &Path) -> PathBuf {
    local_app_data.join("Programs").join(APP_NAME)
}

/// The folder the user picked, with `MindsWall` added unless it already ends
/// with it, so the program is not put loose in, say, Documents.
pub fn with_app_folder(picked: &Path) -> PathBuf {
    let named = picked
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(APP_NAME));
    if named {
        picked.to_path_buf()
    } else {
        picked.join(APP_NAME)
    }
}

/// Why `location` cannot be used, if it cannot. Mirrors the check in the
/// setup window so a location typed there is checked again here.
pub fn location_problem(location: &str) -> Option<&'static str> {
    let text = location.trim();
    let bytes = text.as_bytes();
    let drive = bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && &bytes[1..3] == b":\\";
    let share = text.starts_with(r"\\")
        && text[2..]
            .split('\\')
            .filter(|part| !part.is_empty())
            .count()
            >= 2;
    if text.is_empty() {
        return Some("Choose a folder.");
    }
    if !drive && !share {
        return Some("Enter a full path, such as C:\\Users\\You\\Apps\\MindsWall.");
    }
    if drive && text[3..].trim_matches('\\').is_empty() {
        return Some("Choose a folder, not the whole drive.");
    }
    if drive {
        let first = text[3..]
            .split('\\')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(
            first.as_str(),
            "program files" | "program files (x86)" | "windows"
        ) {
            return Some(
                "This folder needs administrator rights. Choose a folder in your user profile.",
            );
        }
    }
    None
}

/// The "Start with Windows" command for the app at `exe`.
pub fn run_command(exe: &Path) -> String {
    format!("\"{}\" {HIDDEN_ARG}", exe.display())
}

/// The command Windows runs to uninstall from `dir`.
pub fn uninstall_command(dir: &Path) -> String {
    format!("\"{}\" {UNINSTALL_ARG}", dir.join(UNINSTALLER).display())
}

/// The program a command such as a "Start with Windows" entry runs, without
/// quotes or arguments.
fn command_program(command: &str) -> &str {
    let command = command.trim();
    if let Some(rest) = command.strip_prefix('"') {
        return rest.split('"').next().unwrap_or_default();
    }
    // The app's own setting writes the path without quotes, so the program
    // ends where ".exe" does.
    let lower = command.to_ascii_lowercase();
    match lower.find(".exe") {
        Some(end) => &command[..end + 4],
        None => command.split(' ').next().unwrap_or_default(),
    }
}

/// Whether `command` runs MindsWall from `dir`, comparing paths as Windows
/// does: ignoring case and the direction of slashes.
pub fn runs_app_in(command: &str, dir: &Path) -> bool {
    let normalize = |text: &str| {
        text.replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    normalize(command_program(command)) == normalize(&dir.join(APP_EXE).to_string_lossy())
}

/// Whether `recorded`, a folder as stored in the registry, is `dir`.
pub fn same_folder(recorded: &str, dir: &Path) -> bool {
    let normalize = |text: &str| {
        text.trim()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    !recorded.trim().is_empty() && normalize(recorded) == normalize(&dir.to_string_lossy())
}

/// A registry value in the uninstall entry.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Number(u32),
}

/// The values Windows shows for the app in Settings > Apps.
pub fn uninstall_values(
    dir: &Path,
    version: &str,
    size_kb: u32,
    date: &str,
) -> Vec<(&'static str, Value)> {
    let exe = dir.join(APP_EXE);
    vec![
        ("DisplayName", Value::Text(APP_NAME.into())),
        ("DisplayVersion", Value::Text(version.into())),
        ("Publisher", Value::Text(PUBLISHER.into())),
        ("DisplayIcon", Value::Text(format!("{},0", exe.display()))),
        ("InstallLocation", Value::Text(dir.display().to_string())),
        ("UninstallString", Value::Text(uninstall_command(dir))),
        ("URLInfoAbout", Value::Text(HOMEPAGE.into())),
        ("InstallDate", Value::Text(date.into())),
        ("EstimatedSize", Value::Number(size_kb)),
        ("NoModify", Value::Number(1)),
        ("NoRepair", Value::Number(1)),
    ]
}

/// A date as `YYYYMMDD`, the form Windows expects for `InstallDate`, from
/// seconds since 1970-01-01 (UTC).
pub fn install_date(unix_seconds: u64) -> String {
    // Days to a civil date, from Howard Hinnant's `civil_from_days`.
    let days = (unix_seconds / 86_400) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}")
}

/// The folders that hold MindsWall's settings, history, logs, caches, and
/// WebView data for an installed (not portable) app.
pub fn data_folders(roaming: &Path, local: &Path) -> [PathBuf; 3] {
    [
        roaming.join(APP_NAME),
        local.join(APP_NAME),
        local.join("com.mindswall.desktop"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_location_needs_no_administrator_rights() {
        assert_eq!(
            default_location(Path::new(r"C:\Users\You\AppData\Local")),
            Path::new(r"C:\Users\You\AppData\Local")
                .join("Programs")
                .join("MindsWall")
        );
    }

    #[test]
    fn a_picked_folder_gets_its_own_mindswall_folder() {
        assert_eq!(
            with_app_folder(Path::new("D:/Apps")),
            Path::new("D:/Apps").join("MindsWall")
        );
        assert_eq!(
            with_app_folder(Path::new("D:/Apps/mindswall")),
            Path::new("D:/Apps/mindswall")
        );
    }

    #[test]
    fn only_usable_locations_are_accepted() {
        assert_eq!(location_problem(r"C:\Users\You\Apps\MindsWall"), None);
        assert_eq!(location_problem(r"\\server\share\MindsWall"), None);
        assert!(location_problem("").is_some());
        assert!(location_problem("MindsWall").is_some());
        assert!(location_problem(r"C:\").is_some());
        assert!(location_problem(r"\\server").is_some());
        assert!(location_problem(r"C:\Program Files\MindsWall").is_some());
        assert!(location_problem(r"c:\program files (x86)").is_some());
        assert!(location_problem(r"C:\Windows\MindsWall").is_some());
        assert_eq!(
            location_problem(r"D:\Program Files Portable\MindsWall"),
            None
        );
    }

    #[test]
    fn startup_entries_are_matched_to_the_install_folder() {
        let dir = Path::new(r"C:\Users\You\AppData\Local\Programs\MindsWall");
        let ours = run_command(&dir.join(APP_EXE));
        assert_eq!(
            ours,
            format!("\"{}\" --hidden", dir.join(APP_EXE).display())
        );
        assert!(runs_app_in(&ours, dir));
        // The app's own setting writes the path without quotes.
        assert!(runs_app_in(
            &format!("{} --hidden", dir.join(APP_EXE).display()),
            dir
        ));
        assert!(runs_app_in(&ours.to_uppercase(), dir));
        assert!(!runs_app_in(r#""D:\Portable\MindsWall.exe" --hidden"#, dir));
        assert!(!runs_app_in("", dir));
    }

    #[test]
    fn recorded_folders_are_compared_as_windows_does() {
        let dir = Path::new(r"C:\Users\You\AppData\Local\Programs\MindsWall");
        assert!(same_folder(
            r"c:\users\you\appdata\local\programs\mindswall\",
            dir
        ));
        assert!(!same_folder(r"C:\Users\You\AppData\Local\MindsWall", dir));
        assert!(!same_folder("", dir));
    }

    #[test]
    fn the_uninstall_entry_points_at_the_uninstaller() {
        let dir = Path::new(r"C:\Apps\MindsWall");
        let values = uninstall_values(dir, "1.0.0", 15_000, "20261127");
        let text = |name: &str| {
            values
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.clone())
        };
        assert_eq!(text("DisplayVersion"), Some(Value::Text("1.0.0".into())));
        assert_eq!(
            text("UninstallString"),
            Some(Value::Text(format!(
                "\"{}\" --uninstall",
                dir.join("uninstall.exe").display()
            )))
        );
        assert_eq!(text("EstimatedSize"), Some(Value::Number(15_000)));
    }

    #[test]
    fn install_dates_use_the_windows_format() {
        assert_eq!(install_date(0), "19700101");
        assert_eq!(install_date(951_782_400), "20000229");
        // 2026-11-27 12:00 UTC.
        assert_eq!(install_date(1_795_780_800), "20261127");
    }

    #[test]
    fn progress_counts_up_to_done() {
        let seen = std::cell::RefCell::new(Vec::new());
        let report = |progress: Progress| seen.borrow_mut().push(progress.percent);
        let mut steps = Steps::new(4, &report);
        for step in ["a", "b", "c", "d"] {
            steps.start(step);
        }
        steps.finish();
        assert_eq!(*seen.borrow(), [0, 25, 50, 75, 100]);
    }
}
