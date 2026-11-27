//! MindsWall Setup: installs MindsWall for the current user, and removes it
//! again when started with `--uninstall`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// On other systems the Windows parts are stand-ins, so some items go unused.
#![cfg_attr(not(windows), allow(dead_code))]

mod plan;
mod system;

use plan::{
    FROM_TEMP_ARG, InstallOptions, Installed, Mode, Progress, SetupInfo, Steps, UNINSTALL_ARG,
    UninstallOptions,
};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use system::Leftover;
use tauri::ipc::Channel;
use tauri::{Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

/// The MindsWall program this setup installs, embedded by `build.rs`.
static PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/MindsWall.exe"));
const VERSION: &str = env!("CARGO_PKG_VERSION");

struct Setup {
    mode: Mode,
    /// The folder to uninstall from, in uninstall mode.
    uninstall_dir: Option<PathBuf>,
    /// The folder MindsWall was installed to in this run.
    installed_dir: Mutex<Option<PathBuf>>,
    /// What to delete after the setup exits.
    leftovers: Mutex<Vec<Leftover>>,
}

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let uninstalling = args.iter().any(|arg| arg == UNINSTALL_ARG);
    let from_temp = args
        .iter()
        .position(|arg| arg == FROM_TEMP_ARG)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from);
    let mut leftovers = vec![Leftover::Folder(system::setup_webview_dir())];
    let uninstall_dir = if !uninstalling {
        None
    } else if let Some(dir) = from_temp {
        // This is the temporary copy; delete it after it exits.
        if let Ok(exe) = std::env::current_exe() {
            leftovers.push(Leftover::File(exe));
        }
        Some(dir)
    } else {
        let dir = system::installed().map(|(_, dir)| dir).or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf))
        });
        // Continue from a temporary copy so this file can be deleted. If the
        // copy cannot start, uninstall from here; this file is then deleted
        // after exit.
        if let Some(dir) = &dir
            && cfg!(windows)
            && system::relaunch_from_temp(dir).is_ok()
        {
            return;
        }
        dir
    };
    let mode = if uninstalling {
        Mode::Uninstall
    } else {
        Mode::Install
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Setup {
            mode,
            uninstall_dir,
            installed_dir: Mutex::new(None),
            leftovers: Mutex::new(leftovers),
        })
        .invoke_handler(tauri::generate_handler![
            setup_info,
            browse_folder,
            close_app,
            install,
            uninstall,
            launch_app
        ])
        .setup(move |app| {
            let title = match mode {
                Mode::Install => "MindsWall Setup",
                Mode::Uninstall => "Uninstall MindsWall",
            };
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title(title)
                .inner_size(760.0, 480.0)
                .resizable(false)
                .maximizable(false)
                .decorations(false)
                .shadow(true)
                .center()
                .data_directory(system::setup_webview_dir())
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start MindsWall Setup")
        .run(|app, event| {
            if let RunEvent::Exit = event
                && let Ok(leftovers) = app.state::<Setup>().leftovers.lock()
            {
                system::delete_after_exit(&leftovers);
            }
        });
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}

/// The folder MindsWall is or will be installed in.
fn current_location(setup: &Setup) -> Option<PathBuf> {
    setup
        .uninstall_dir
        .clone()
        .or_else(|| system::installed().map(|(_, dir)| dir))
}

#[tauri::command]
async fn setup_info(setup: State<'_, Setup>) -> Result<SetupInfo, String> {
    let mode = setup.mode;
    let location = current_location(&setup);
    blocking(move || {
        let installed = system::installed().and_then(|(version, dir)| {
            dir.join(plan::APP_EXE).is_file().then(|| Installed {
                version,
                start_with_windows: system::start_with_windows_in(&dir),
                location: dir.display().to_string(),
            })
        });
        let default_location = system::default_location();
        let app_running =
            system::app_running(&location.unwrap_or_else(|| PathBuf::from(&default_location)));
        Ok(SetupInfo {
            mode,
            version: VERSION.into(),
            default_location,
            installed,
            app_running,
        })
    })
    .await
}

#[tauri::command]
async fn browse_folder(app: tauri::AppHandle, current: String) -> Result<Option<String>, String> {
    let mut dialog = app
        .dialog()
        .file()
        .set_title("Choose where to install MindsWall");
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&window);
    }
    if let Some(parent) = Path::new(current.trim())
        .parent()
        .filter(|parent| parent.is_dir())
    {
        dialog = dialog.set_directory(parent);
    }
    blocking(move || {
        Ok(dialog
            .blocking_pick_folder()
            .and_then(|folder| folder.into_path().ok())
            .map(|folder| plan::with_app_folder(&folder).display().to_string()))
    })
    .await
}

#[tauri::command]
async fn close_app(setup: State<'_, Setup>) -> Result<(), String> {
    let Some(dir) = current_location(&setup) else {
        return Ok(());
    };
    blocking(move || system::close_app(&dir)).await
}

#[tauri::command]
async fn install(
    options: InstallOptions,
    progress: Channel<Progress>,
    setup: State<'_, Setup>,
) -> Result<(), String> {
    let dir = blocking(move || {
        let report = |step: Progress| {
            let _ = progress.send(step);
        };
        system::install(&options, PAYLOAD, VERSION, &mut Steps::new(7, &report))
    })
    .await?;
    *setup.installed_dir.lock().map_err(|e| e.to_string())? = Some(dir);
    Ok(())
}

#[tauri::command]
async fn uninstall(
    options: UninstallOptions,
    progress: Channel<Progress>,
    setup: State<'_, Setup>,
) -> Result<(), String> {
    let dir = setup
        .uninstall_dir
        .clone()
        .ok_or("MindsWall is not installed.")?;
    let leftovers = blocking(move || {
        let report = |step: Progress| {
            let _ = progress.send(step);
        };
        let total = system::uninstall_steps(&options);
        system::uninstall(&dir, &options, &mut Steps::new(total, &report))
    })
    .await?;
    setup
        .leftovers
        .lock()
        .map_err(|e| e.to_string())?
        .extend(leftovers);
    Ok(())
}

#[tauri::command]
async fn launch_app(setup: State<'_, Setup>) -> Result<(), String> {
    let dir = setup
        .installed_dir
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("MindsWall is not installed yet.")?;
    blocking(move || system::launch(&dir)).await
}
