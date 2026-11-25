use mindswall_core::{
    config::Config,
    ipc::{self, Command},
    scanner,
};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_autostart::ManagerExt;

#[derive(Default)]
struct ServiceState {
    starting: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
}

#[tauri::command]
fn get_config() -> Result<Config, String> {
    Config::load(&Config::default_path().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(config: Config) -> Result<String, String> {
    if config.version != 1 || !(60..=86400).contains(&config.interval_seconds) {
        return Err("Choose an interval between 60 seconds and 24 hours.".into());
    }
    config
        .save(&Config::default_path().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if ipc::send(Command::Status).is_ok() {
        let result = ipc::send(Command::Reload)
            .map_err(|e| format!("Settings saved, but the slideshow could not reload: {e}"))?;
        if !result.ok {
            return Err(format!(
                "Settings saved, but the slideshow could not reload: {}",
                result.message
            ));
        }
    }
    Ok("Settings saved.".into())
}

#[tauri::command]
fn folder_count(path: PathBuf, recursive: bool) -> Result<usize, String> {
    scanner::scan(&[path], recursive)
        .map(|images| images.len())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn control(action: &str) -> Result<String, String> {
    let command = match action {
        "next" => Command::Next,
        "previous" => Command::Previous,
        "pause" => Command::Pause,
        "resume" => Command::Resume,
        "status" => Command::Status,
        _ => return Err("Unknown action".into()),
    };
    let response = ipc::send(command).map_err(|e| e.to_string())?;
    if response.ok {
        Ok(response.message)
    } else {
        Err(response.message)
    }
}

#[tauri::command]
fn start_service(state: tauri::State<'_, ServiceState>) -> Result<(), String> {
    if ipc::send(Command::Status).is_ok() {
        return Ok(());
    }
    if state.starting.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    *state.error.lock().map_err(|e| e.to_string())? = None;
    let starting = state.starting.clone();
    let error = state.error.clone();
    std::thread::spawn(move || {
        if let Err(e) = mindswall_core::service::run_slideshow()
            && let Ok(mut slot) = error.lock()
        {
            *slot = Some(e.to_string());
        }
        starting.store(false, Ordering::SeqCst);
    });
    Ok(())
}

#[tauri::command]
fn service_error(state: tauri::State<'_, ServiceState>) -> Option<String> {
    state.error.lock().ok().and_then(|e| e.clone())
}

#[tauri::command]
fn current_preview() -> Result<Vec<u8>, String> {
    let status = ipc::send(Command::Status).map_err(|e| e.to_string())?;
    let (_, path) = status
        .message
        .split_once(": ")
        .ok_or("No current wallpaper")?;
    let path = std::path::Path::new(path);
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() > 15 * 1024 * 1024 {
        return Err("Image is too large for preview".into());
    }
    std::fs::read(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("MindsWall")
                .build(),
        )
        .manage(ServiceState::default())
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            folder_count,
            control,
            start_service,
            service_error,
            current_preview,
            get_autostart,
            set_autostart
        ])
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "Open MindsWall", true, None::<&str>)?;
            let previous = MenuItem::with_id(app, "previous", "Previous", true, None::<&str>)?;
            let next = MenuItem::with_id(app, "next", "Next", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "Pause", true, None::<&str>)?;
            let resume = MenuItem::with_id(app, "resume", "Resume", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &previous, &next, &pause, &resume, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("MindsWall")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    action => {
                        let command = match action {
                            "next" => Some(Command::Next),
                            "previous" => Some(Command::Previous),
                            "pause" => Some(Command::Pause),
                            "resume" => Some(Command::Resume),
                            _ => None,
                        };
                        if let Some(command) = command {
                            std::thread::spawn(move || {
                                let _ = ipc::send(command);
                            });
                        }
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            if get_config().is_ok_and(|config| config.folders.iter().any(|folder| folder.enabled)) {
                let _ = start_service(app.state());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running MindsWall");
}
