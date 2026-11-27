//! The setup's work on this computer: files, shortcuts, the registry, and the
//! running app. File handling is shared; Windows calls are in `win`, with
//! stand-ins elsewhere so the setup builds and its tests run on any system.

use crate::plan::{self, APP_EXE, InstallOptions, Steps, UNINSTALLER, UninstallOptions};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Something to delete after the setup has exited, because it is in use until
/// then: the setup's own copy, its WebView data, or the emptied folder.
#[derive(Debug, Clone, PartialEq)]
pub enum Leftover {
    File(PathBuf),
    /// Removed only if it is empty.
    EmptyFolder(PathBuf),
    /// Removed with everything in it.
    Folder(PathBuf),
}

/// `%LOCALAPPDATA%` and `%APPDATA%`.
fn user_folders() -> Result<(PathBuf, PathBuf), String> {
    let dirs = directories::BaseDirs::new().ok_or("Your user folders are unavailable.")?;
    Ok((
        dirs.data_local_dir().to_path_buf(),
        dirs.config_dir().to_path_buf(),
    ))
}

pub fn default_location() -> String {
    user_folders()
        .map(|(local, _)| plan::default_location(&local).display().to_string())
        .unwrap_or_default()
}

/// The WebView profile of the setup window, kept in the temporary folder.
pub fn setup_webview_dir() -> PathBuf {
    std::env::temp_dir().join("MindsWall Setup WebView")
}

/// Replace `target` with `contents`. The old file is kept as `.old` until the
/// new one is in place and restored if that fails, so a failed update leaves
/// the installed program working.
pub fn replace_file(target: &Path, contents: &[u8]) -> Result<(), String> {
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    let fresh = target.with_extension("exe.new");
    let old = target.with_extension("exe.old");
    fs::write(&fresh, contents).map_err(|e| format!("Could not write {name}: {e}"))?;
    let had_old = target.exists();
    if had_old {
        let _ = fs::remove_file(&old);
        if let Err(error) = fs::rename(target, &old) {
            let _ = fs::remove_file(&fresh);
            return Err(format!(
                "Could not replace {name}: {error}. Close MindsWall and try again."
            ));
        }
    }
    if let Err(error) = fs::rename(&fresh, target) {
        if had_old {
            let _ = fs::rename(&old, target);
        }
        let _ = fs::remove_file(&fresh);
        return Err(format!("Could not write {name}: {error}"));
    }
    // An old copy that is still running cannot be deleted yet; the next
    // update or the uninstaller removes it.
    let _ = fs::remove_file(&old);
    Ok(())
}

/// Delete the program files from `dir`, then `dir` itself if nothing else is
/// in it. Files the user put there are kept. Returns what could not be
/// deleted yet, such as the running uninstaller.
pub fn remove_program(dir: &Path) -> Result<Vec<Leftover>, String> {
    let exe = dir.join(APP_EXE);
    match fs::remove_file(&exe) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Could not remove {APP_EXE}: {error}. Close MindsWall and try again."
            ));
        }
    }
    let mut leftovers = Vec::new();
    for name in [
        UNINSTALLER,
        "MindsWall.exe.old",
        "MindsWall.exe.new",
        "uninstall.exe.old",
        "uninstall.exe.new",
    ] {
        let path = dir.join(name);
        if path.exists() && fs::remove_file(&path).is_err() {
            leftovers.push(Leftover::File(path));
        }
    }
    if fs::remove_dir(dir).is_err() && dir.exists() {
        leftovers.push(Leftover::EmptyFolder(dir.to_path_buf()));
    }
    Ok(leftovers)
}

/// Delete MindsWall's settings, history, logs, caches, and WebView data.
fn delete_data() -> Result<(), String> {
    let (local, roaming) = user_folders()?;
    for folder in plan::data_folders(&roaming, &local) {
        match fs::remove_dir_all(&folder) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("Could not delete {}: {error}", folder.display())),
        }
    }
    Ok(())
}

/// Whether `dir` holds an installed MindsWall: the app or its uninstaller.
pub fn is_installed_in(dir: &Path) -> bool {
    dir.join(APP_EXE).is_file() || dir.join(UNINSTALLER).is_file()
}

/// The installed version and folder, from the entry in Settings > Apps.
pub fn installed() -> Option<(String, PathBuf)> {
    win::installed()
}

pub fn start_with_windows_in(dir: &Path) -> bool {
    win::run_entry().is_some_and(|command| plan::runs_app_in(&command, dir))
}

pub fn app_running(dir: &Path) -> bool {
    !win::app_processes(&dir.join(APP_EXE)).is_empty()
}

/// Close MindsWall if it runs from `dir`. Copies elsewhere, such as a
/// portable one, are left alone.
pub fn close_app(dir: &Path) -> Result<(), String> {
    win::end_processes(&dir.join(APP_EXE))
}

pub fn launch(dir: &Path) -> Result<(), String> {
    std::process::Command::new(dir.join(APP_EXE))
        .current_dir(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not start MindsWall: {e}"))
}

/// Install `payload` as MindsWall into the folder in `options`, in 7 steps.
pub fn install(
    options: &InstallOptions,
    payload: &[u8],
    version: &str,
    steps: &mut Steps,
) -> Result<PathBuf, String> {
    if payload.is_empty() {
        return Err("This setup does not contain MindsWall. Download the setup again.".into());
    }
    if let Some(problem) = plan::location_problem(&options.location) {
        return Err(problem.into());
    }
    let dir = PathBuf::from(options.location.trim());
    steps.start("Preparing the folder");
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    steps.start("Closing MindsWall");
    close_app(&dir)?;
    steps.start("Copying MindsWall.exe");
    replace_file(&dir.join(APP_EXE), payload)?;
    steps.start("Copying the uninstaller");
    let setup =
        std::env::current_exe().map_err(|e| format!("Could not find the setup program: {e}"))?;
    let uninstaller = dir.join(UNINSTALLER);
    if !same_file(&setup, &uninstaller) {
        let bytes =
            fs::read(&setup).map_err(|e| format!("Could not read the setup program: {e}"))?;
        replace_file(&uninstaller, &bytes)?;
    }
    steps.start("Creating shortcuts");
    let exe = dir.join(APP_EXE);
    win::create_shortcut(win::Place::StartMenu, &exe, &dir)?;
    if options.desktop_shortcut {
        win::create_shortcut(win::Place::Desktop, &exe, &dir)?;
    } else {
        win::remove_shortcut(win::Place::Desktop, &dir);
    }
    steps.start("Registering with Windows");
    let size_kb = ((payload.len()
        + fs::metadata(&uninstaller)
            .map(|m| m.len() as usize)
            .unwrap_or(0))
        / 1024) as u32;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    win::register(&plan::uninstall_values(
        &dir,
        version,
        size_kb,
        &plan::install_date(now),
    ))?;
    steps.start("Setting Start with Windows");
    set_start_with_windows(&dir, options.start_with_windows)?;
    steps.finish();
    Ok(dir)
}

/// The number of steps `uninstall` reports.
pub fn uninstall_steps(options: &UninstallOptions) -> usize {
    if options.delete_data { 6 } else { 5 }
}

/// Remove MindsWall from `dir`. Returns what is deleted after the setup exits.
pub fn uninstall(
    dir: &Path,
    options: &UninstallOptions,
    steps: &mut Steps,
) -> Result<Vec<Leftover>, String> {
    if !is_installed_in(dir) {
        return Err(format!("MindsWall is not installed in {}.", dir.display()));
    }
    steps.start("Closing MindsWall");
    close_app(dir)?;
    steps.start("Removing Start with Windows");
    set_start_with_windows(dir, false)?;
    steps.start("Removing shortcuts");
    win::remove_shortcut(win::Place::StartMenu, dir);
    win::remove_shortcut(win::Place::Desktop, dir);
    steps.start("Removing MindsWall.exe");
    let leftovers = remove_program(dir)?;
    if options.delete_data {
        steps.start("Deleting settings and caches");
        delete_data()?;
    }
    steps.start("Removing the Windows entry");
    win::unregister(dir);
    steps.finish();
    Ok(leftovers)
}

/// Turn "Start with Windows" on for the app in `dir`, or off if the current
/// entry is for that app (an entry for a portable copy is kept).
fn set_start_with_windows(dir: &Path, on: bool) -> Result<(), String> {
    if on {
        win::set_run_entry(&plan::run_command(&dir.join(APP_EXE)))
    } else {
        if start_with_windows_in(dir) {
            win::remove_run_entry();
        }
        Ok(())
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Run the uninstaller from a copy in the temporary folder, so the copy in
/// `dir` can be deleted. Returns `Ok` when the copy has started.
pub fn relaunch_from_temp(dir: &Path) -> io::Result<()> {
    let exe = std::env::current_exe()?;
    let copy = std::env::temp_dir().join(format!("MindsWall Uninstall {}.exe", std::process::id()));
    fs::copy(&exe, &copy)?;
    std::process::Command::new(&copy)
        .arg(plan::UNINSTALL_ARG)
        .arg(plan::FROM_TEMP_ARG)
        .arg(dir)
        .spawn()
        .map(|_| ())
}

/// Delete `leftovers` a few seconds after this program exits.
pub fn delete_after_exit(leftovers: &[Leftover]) {
    win::delete_after_exit(leftovers);
}

/// The command that deletes `leftovers` once the setup has exited: it waits
/// about two seconds, then deletes each one, ignoring failures.
pub fn cleanup_script(leftovers: &[Leftover]) -> String {
    let mut script = String::from("ping -n 3 127.0.0.1 >nul");
    for leftover in leftovers {
        let command = match leftover {
            Leftover::File(path) => format!("del /f /q \"{}\"", path.display()),
            Leftover::EmptyFolder(path) => format!("rd \"{}\"", path.display()),
            Leftover::Folder(path) => format!("rd /s /q \"{}\"", path.display()),
        };
        script.push_str(" & ");
        script.push_str(&command);
    }
    script.push_str(" >nul 2>&1");
    script
}

#[cfg(windows)]
mod win {
    use super::{APP_EXE, Leftover};
    use crate::plan::{self, SHORTCUT, Value};
    use std::path::{Path, PathBuf};
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoTaskMemFree, CoUninitialize, IPersistFile, STGM_READ,
    };
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
        PROCESS_TERMINATE, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
    };
    use windows::Win32::UI::Shell::{
        FOLDERID_Desktop, FOLDERID_Programs, IShellLinkW, KNOWN_FOLDER_FLAG, SHGetKnownFolderPath,
        SLGP_RAWPATH, ShellLink,
    };
    use windows::core::{HSTRING, Interface, PWSTR};
    use windows_registry::CURRENT_USER;

    pub enum Place {
        StartMenu,
        Desktop,
    }

    pub fn installed() -> Option<(String, PathBuf)> {
        let key = CURRENT_USER.open(plan::UNINSTALL_KEY).ok()?;
        let version = key.get_string("DisplayVersion").ok()?;
        let location = key.get_string("InstallLocation").ok()?;
        Some((version, PathBuf::from(location)))
    }

    pub fn register(values: &[(&str, Value)]) -> Result<(), String> {
        fn fail(error: impl std::fmt::Display) -> String {
            format!("Could not register MindsWall with Windows: {error}")
        }
        let key = CURRENT_USER.create(plan::UNINSTALL_KEY).map_err(fail)?;
        for (name, value) in values {
            match value {
                Value::Text(text) => key.set_string(*name, text),
                Value::Number(number) => key.set_u32(*name, *number),
            }
            .map_err(fail)?;
        }
        Ok(())
    }

    /// Remove the Settings > Apps entry if it is for the install in `dir`,
    /// not for another one such as the NSIS installer's.
    pub fn unregister(dir: &Path) {
        let ours = CURRENT_USER
            .open(plan::UNINSTALL_KEY)
            .and_then(|key| key.get_string("InstallLocation"))
            .is_ok_and(|location| plan::same_folder(&location, dir));
        if ours {
            let _ = CURRENT_USER.remove_tree(plan::UNINSTALL_KEY);
        }
    }

    pub fn run_entry() -> Option<String> {
        CURRENT_USER
            .open(plan::RUN_KEY)
            .ok()?
            .get_string(plan::APP_NAME)
            .ok()
    }

    pub fn set_run_entry(command: &str) -> Result<(), String> {
        CURRENT_USER
            .create(plan::RUN_KEY)
            .and_then(|key| key.set_string(plan::APP_NAME, command))
            .map_err(|e| format!("Could not turn on Start with Windows: {e}"))?;
        // Mark it enabled for Task Manager, as the app's own setting does,
        // in case it was disabled there before.
        if let Ok(key) = CURRENT_USER
            .options()
            .write()
            .open(plan::STARTUP_APPROVED_KEY)
        {
            let _ = key.set_bytes(
                plan::APP_NAME,
                windows_registry::Type::Bytes,
                &plan::STARTUP_ENABLED,
            );
        }
        Ok(())
    }

    pub fn remove_run_entry() {
        if let Ok(key) = CURRENT_USER.options().write().open(plan::RUN_KEY) {
            let _ = key.remove_value(plan::APP_NAME);
        }
        if let Ok(key) = CURRENT_USER
            .options()
            .write()
            .open(plan::STARTUP_APPROVED_KEY)
        {
            let _ = key.remove_value(plan::APP_NAME);
        }
    }

    fn same_path(a: &Path, b: &Path) -> bool {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    }

    fn image_path(process: HANDLE) -> Option<PathBuf> {
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;
        // SAFETY: `buffer` holds `size` characters and outlives the call.
        unsafe {
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut size,
            )
        }
        .ok()?;
        Some(PathBuf::from(String::from_utf16_lossy(
            &buffer[..size as usize],
        )))
    }

    /// Process IDs of programs running from `exe`.
    pub fn app_processes(exe: &Path) -> Vec<u32> {
        let mut found = Vec::new();
        // SAFETY: the snapshot handle is closed below; `entry` is sized as
        // the API requires.
        unsafe {
            let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
                return found;
            };
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut more = Process32FirstW(snapshot, &mut entry).is_ok();
            while more {
                let length = entry
                    .szExeFile
                    .iter()
                    .position(|&unit| unit == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..length]);
                if name.eq_ignore_ascii_case(APP_EXE)
                    && let Ok(process) = OpenProcess(
                        PROCESS_QUERY_LIMITED_INFORMATION,
                        false,
                        entry.th32ProcessID,
                    )
                {
                    if image_path(process).is_some_and(|path| same_path(&path, exe)) {
                        found.push(entry.th32ProcessID);
                    }
                    let _ = CloseHandle(process);
                }
                more = Process32NextW(snapshot, &mut entry).is_ok();
            }
            let _ = CloseHandle(snapshot);
        }
        found
    }

    /// End every program running from `exe` and wait for it to exit.
    pub fn end_processes(exe: &Path) -> Result<(), String> {
        for id in app_processes(exe) {
            // SAFETY: the process handle is closed after use.
            unsafe {
                let process = OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, false, id)
                    .map_err(|e| format!("Could not close MindsWall: {e}"))?;
                let ended = TerminateProcess(process, 0);
                WaitForSingleObject(process, 5000);
                let _ = CloseHandle(process);
                ended.map_err(|e| format!("Could not close MindsWall: {e}"))?;
            }
        }
        Ok(())
    }

    fn folder(place: &Place) -> Option<PathBuf> {
        let id = match place {
            Place::StartMenu => &FOLDERID_Programs,
            Place::Desktop => &FOLDERID_Desktop,
        };
        // SAFETY: the returned string is freed with CoTaskMemFree.
        unsafe {
            let path = SHGetKnownFolderPath(id, KNOWN_FOLDER_FLAG(0), None).ok()?;
            let text = path.to_string().ok();
            CoTaskMemFree(Some(path.0 as *const _));
            text.map(PathBuf::from)
        }
    }

    pub fn create_shortcut(place: Place, target: &Path, dir: &Path) -> Result<(), String> {
        let link = folder(&place)
            .ok_or("Could not find the Start menu or desktop folder.")?
            .join(SHORTCUT);
        // SAFETY: COM is initialized for this thread for the calls below and
        // uninitialized afterwards when this call initialized it.
        unsafe {
            let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
            let result = (|| -> windows::core::Result<()> {
                let shell: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
                shell.SetPath(&HSTRING::from(target))?;
                shell.SetWorkingDirectory(&HSTRING::from(dir))?;
                shell.SetDescription(&HSTRING::from("Local-first wallpaper manager"))?;
                shell.SetIconLocation(&HSTRING::from(target), 0)?;
                shell
                    .cast::<IPersistFile>()?
                    .Save(&HSTRING::from(link.as_path()), true)
            })();
            if initialized {
                CoUninitialize();
            }
            result.map_err(|e| format!("Could not create a shortcut: {e}"))
        }
    }

    /// The program a shortcut opens.
    fn shortcut_target(link: &Path) -> Option<String> {
        // SAFETY: COM is initialized for this thread for the calls below and
        // uninitialized afterwards when this call initialized it; `buffer`
        // outlives the call that fills it.
        unsafe {
            let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
            let result = (|| -> windows::core::Result<String> {
                let shell: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
                shell
                    .cast::<IPersistFile>()?
                    .Load(&HSTRING::from(link), STGM_READ)?;
                let mut buffer = [0u16; 1024];
                shell.GetPath(&mut buffer, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)?;
                let length = buffer
                    .iter()
                    .position(|&unit| unit == 0)
                    .unwrap_or(buffer.len());
                Ok(String::from_utf16_lossy(&buffer[..length]))
            })();
            if initialized {
                CoUninitialize();
            }
            result.ok()
        }
    }

    /// Delete MindsWall's shortcut in `place` if it opens the app in `dir`.
    /// A shortcut to another copy, or one that cannot be read, is kept.
    pub fn remove_shortcut(place: Place, dir: &Path) {
        let Some(link) = folder(&place).map(|folder| folder.join(SHORTCUT)) else {
            return;
        };
        if shortcut_target(&link).is_some_and(|target| plan::runs_app_in(&target, dir)) {
            let _ = std::fs::remove_file(link);
        }
    }

    pub fn delete_after_exit(leftovers: &[Leftover]) {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        if leftovers.is_empty() {
            return;
        }
        let script = super::cleanup_script(leftovers);
        let _ = std::process::Command::new("cmd.exe")
            .raw_arg(format!("/d /s /c \"{script}\""))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
}

#[cfg(not(windows))]
mod win {
    //! Stand-ins so the setup builds on other systems for checks and tests.
    use super::Leftover;
    use crate::plan::Value;
    use std::path::{Path, PathBuf};

    const ONLY_WINDOWS: &str = "MindsWall Setup runs on Windows.";

    pub enum Place {
        StartMenu,
        Desktop,
    }

    pub fn installed() -> Option<(String, PathBuf)> {
        None
    }
    pub fn register(_: &[(&str, Value)]) -> Result<(), String> {
        Err(ONLY_WINDOWS.into())
    }
    pub fn unregister(_: &Path) {}
    pub fn run_entry() -> Option<String> {
        None
    }
    pub fn set_run_entry(_: &str) -> Result<(), String> {
        Err(ONLY_WINDOWS.into())
    }
    pub fn remove_run_entry() {}
    pub fn app_processes(_: &Path) -> Vec<u32> {
        Vec::new()
    }
    pub fn end_processes(_: &Path) -> Result<(), String> {
        Ok(())
    }
    pub fn create_shortcut(_: Place, _: &Path, _: &Path) -> Result<(), String> {
        Err(ONLY_WINDOWS.into())
    }
    pub fn remove_shortcut(_: Place, _: &Path) {
        let _ = (Place::StartMenu, Place::Desktop);
    }
    pub fn delete_after_exit(_: &[Leftover]) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::APP_NAME;

    #[test]
    fn replacing_a_file_keeps_no_temporary_copies() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join(APP_EXE);
        replace_file(&exe, b"first").unwrap();
        replace_file(&exe, b"second").unwrap();
        assert_eq!(fs::read(&exe).unwrap(), b"second");
        let mut names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        assert_eq!(names, [APP_EXE]);
    }

    #[test]
    fn a_failed_replacement_keeps_the_old_file() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join(APP_EXE);
        fs::write(&exe, b"working").unwrap();
        // A folder where the new copy would go makes writing it fail.
        fs::create_dir(exe.with_extension("exe.new")).unwrap();
        assert!(replace_file(&exe, b"broken").is_err());
        assert_eq!(fs::read(&exe).unwrap(), b"working");
    }

    #[test]
    fn removing_the_program_keeps_files_the_user_added() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join(APP_NAME);
        fs::create_dir(&app).unwrap();
        for name in [APP_EXE, UNINSTALLER, "MindsWall.exe.old", "notes.txt"] {
            fs::write(app.join(name), b"").unwrap();
        }
        assert_eq!(
            remove_program(&app).unwrap(),
            [Leftover::EmptyFolder(app.clone())]
        );
        let left: Vec<_> = fs::read_dir(&app)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, ["notes.txt"]);
        fs::remove_file(app.join("notes.txt")).unwrap();
        fs::write(app.join(APP_EXE), b"").unwrap();
        assert!(remove_program(&app).unwrap().is_empty());
        assert!(!app.exists());
    }

    #[test]
    fn an_empty_setup_refuses_to_install() {
        let options = InstallOptions {
            location: r"C:\Users\You\Apps\MindsWall".into(),
            desktop_shortcut: false,
            start_with_windows: false,
        };
        let report = |_| {};
        let error = install(&options, &[], "1.0.0", &mut Steps::new(7, &report)).unwrap_err();
        assert!(error.contains("does not contain MindsWall"), "{error}");
    }

    #[test]
    fn uninstalling_from_a_folder_without_mindswall_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("notes.txt"), b"").unwrap();
        let report = |_| {};
        let options = UninstallOptions { delete_data: true };
        let error = uninstall(dir.path(), &options, &mut Steps::new(6, &report)).unwrap_err();
        assert!(error.contains("not installed"), "{error}");
        assert!(dir.path().join("notes.txt").exists());
        assert!(!is_installed_in(dir.path()));
        fs::write(dir.path().join(UNINSTALLER), b"").unwrap();
        assert!(is_installed_in(dir.path()));
    }

    #[test]
    fn the_cleanup_waits_then_deletes_each_leftover() {
        let script = cleanup_script(&[
            Leftover::File(PathBuf::from(r"C:\Temp\MindsWall Uninstall 7.exe")),
            Leftover::EmptyFolder(PathBuf::from(r"C:\Apps\MindsWall")),
            Leftover::Folder(PathBuf::from(r"C:\Temp\MindsWall Setup WebView")),
        ]);
        assert_eq!(
            script,
            r#"ping -n 3 127.0.0.1 >nul & del /f /q "C:\Temp\MindsWall Uninstall 7.exe" & rd "C:\Apps\MindsWall" & rd /s /q "C:\Temp\MindsWall Setup WebView" >nul 2>&1"#
        );
    }
}
