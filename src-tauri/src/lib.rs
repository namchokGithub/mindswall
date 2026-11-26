use mindswall_core::{
    config::Config,
    ipc::{self, Command},
    scanner,
    wallpaper::{SystemWallpaper, WallpaperSetter},
};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{
    Emitter, Manager,
    menu::{Menu, MenuBuilder, SubmenuBuilder},
    tray::TrayIconBuilder,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Default)]
struct HotkeyState {
    registered: Mutex<std::collections::HashMap<u32, (Shortcut, &'static str)>>,
    error: Mutex<Option<String>>,
}

fn configure_hotkeys(
    app: &tauri::AppHandle,
    hotkeys: &mindswall_core::config::Hotkeys,
    persist: bool,
) -> Result<(), String> {
    let next = parse_hotkeys(hotkeys)?;
    let state = app.state::<HotkeyState>();
    let mut current = state.registered.lock().map_err(|e| e.to_string())?;
    let mut added = Vec::new();
    for (id, (shortcut, _)) in &next {
        if !current.contains_key(id) {
            if let Err(error) = app.global_shortcut().register(*shortcut) {
                for item in added {
                    let _ = app.global_shortcut().unregister(item);
                }
                return Err(format!(
                    "Shortcut is unavailable or already in use: {error}"
                ));
            }
            added.push(*shortcut);
        }
    }
    if persist {
        let result = (|| {
            let mut config = get_config()?;
            config.hotkeys = hotkeys.clone();
            config
                .save(&Config::default_path().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())
        })();
        if let Err(error) = result {
            for item in added {
                let _ = app.global_shortcut().unregister(item);
            }
            return Err(error);
        }
    }
    for (id, (shortcut, _)) in current.iter() {
        if !next.contains_key(id) {
            let _ = app.global_shortcut().unregister(*shortcut);
        }
    }
    *current = next;
    if let Ok(mut error) = state.error.lock() {
        *error = None;
    }
    Ok(())
}

fn parse_hotkeys(
    hotkeys: &mindswall_core::config::Hotkeys,
) -> Result<std::collections::HashMap<u32, (Shortcut, &'static str)>, String> {
    let mut next = std::collections::HashMap::new();
    for (value, action) in [
        (&hotkeys.next, "next"),
        (&hotkeys.previous, "previous"),
        (&hotkeys.pause, "pause"),
    ] {
        if value.trim().is_empty() {
            continue;
        }
        let shortcut: Shortcut = value
            .trim()
            .parse()
            .map_err(|e| format!("Invalid shortcut {value}: {e}"))?;
        if next.insert(shortcut.id(), (shortcut, action)).is_some() {
            return Err("Shortcuts must be different".into());
        }
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_and_equivalent_shortcuts() {
        let mut hotkeys = mindswall_core::config::Hotkeys {
            next: "Ctrl+Alt+Right".into(),
            previous: "Alt+Control+ArrowRight".into(),
            pause: String::new(),
        };
        assert!(parse_hotkeys(&hotkeys).is_err());
        hotkeys.previous = "invalid shortcut".into();
        assert!(parse_hotkeys(&hotkeys).is_err());
        hotkeys.previous = "Ctrl+Alt+Left".into();
        assert_eq!(parse_hotkeys(&hotkeys).unwrap().len(), 2);
    }
}

#[tauri::command]
fn hotkey_error(state: tauri::State<'_, HotkeyState>) -> Option<String> {
    state.error.lock().ok().and_then(|e| e.clone())
}

#[tauri::command]
async fn set_hotkeys(
    app: tauri::AppHandle,
    hotkeys: mindswall_core::config::Hotkeys,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || configure_hotkeys(&app, &hotkeys, true))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Default)]
struct ServiceState {
    starting: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
}

fn tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let mut builder = MenuBuilder::new(app)
        .text("open", "Open MindsWall")
        .text("previous", "Previous")
        .text("next", "Next")
        .text("pause", "Pause")
        .text("resume", "Resume");
    if let Ok(config) = get_config()
        && !config.profiles.is_empty()
    {
        let mut submenu = SubmenuBuilder::new(app, "Profiles");
        for (index, profile) in config.profiles.iter().enumerate() {
            submenu = submenu.text(format!("profile_{index}"), &profile.name);
        }
        builder = builder.item(&submenu.build()?);
    }
    builder.text("quit", "Quit").build()
}

fn refresh_tray(app: &tauri::AppHandle) {
    if let Some(tray) = app.tray_by_id("main")
        && let Ok(menu) = tray_menu(app)
    {
        let _ = tray.set_menu(Some(menu));
    }
}

#[derive(Default)]
struct GalleryState(Mutex<Vec<PathBuf>>);

#[derive(serde::Serialize)]
struct GalleryPage {
    paths: Vec<PathBuf>,
    total: usize,
}

#[tauri::command]
fn save_profile(app: tauri::AppHandle, name: String) -> Result<String, String> {
    let mut config = get_config()?;
    config.save_profile(&name).map_err(|e| e.to_string())?;
    config
        .save(&Config::default_path().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    refresh_tray(&app);
    Ok("Profile saved.".into())
}

#[tauri::command]
fn apply_profile(name: String) -> Result<String, String> {
    let mut config = get_config()?;
    config.apply_profile(&name).map_err(|e| e.to_string())?;
    save_config(config)
}

#[tauri::command]
fn delete_profile(app: tauri::AppHandle, name: String) -> Result<String, String> {
    let mut config = get_config()?;
    config.profiles.retain(|profile| profile.name != name);
    if config.active_profile.as_deref() == Some(&name) {
        config.active_profile = None;
    }
    config
        .save(&Config::default_path().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    refresh_tray(&app);
    Ok("Profile removed.".into())
}

#[tauri::command]
fn export_backup(path: PathBuf) -> Result<(), String> {
    get_config()?.save(&path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn import_backup(app: tauri::AppHandle, path: PathBuf) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || import_backup_inner(app, path))
        .await
        .map_err(|e| e.to_string())?
}

fn import_backup_inner(app: tauri::AppHandle, path: PathBuf) -> Result<String, String> {
    let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > 16 * 1024 * 1024 {
        return Err("Backup is too large".into());
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let config = Config::parse_backup(&bytes).map_err(|e| format!("Invalid backup: {e}"))?;
    let previous = get_config()?;
    let destination = Config::default_path().map_err(|e| e.to_string())?;
    configure_hotkeys(&app, &config.hotkeys, false)?;
    if let Err(error) = config.save(&destination) {
        let _ = configure_hotkeys(&app, &previous.hotkeys, false);
        return Err(error.to_string());
    }
    refresh_tray(&app);
    if ipc::send(Command::Status).is_ok() {
        let response = ipc::send(Command::Reload)
            .map_err(|e| format!("Backup imported, but slideshow reload failed: {e}"))?;
        if !response.ok {
            return Err(format!(
                "Backup imported, but slideshow reload failed: {}",
                response.message
            ));
        }
    }
    Ok("Backup imported.".into())
}

#[derive(serde::Serialize)]
struct EditedImage {
    path: PathBuf,
    preview: Vec<u8>,
}

fn edited_cache() -> Result<PathBuf, String> {
    directories::ProjectDirs::from("", "", "MindsWall")
        .map(|d| d.cache_dir().join("edits"))
        .ok_or_else(|| "Cache directory unavailable".into())
}

#[tauri::command]
async fn edit_image(
    path: PathBuf,
    options: mindswall_core::editor::EditOptions,
) -> Result<EditedImage, String> {
    tauri::async_runtime::spawn_blocking(move || edit_image_inner(path, options))
        .await
        .map_err(|e| e.to_string())?
}

fn edit_image_inner(
    path: PathBuf,
    options: mindswall_core::editor::EditOptions,
) -> Result<EditedImage, String> {
    let output = mindswall_core::editor::render(&path, &edited_cache()?, &options)
        .map_err(|e| e.to_string())?;
    let image = image::open(&output)
        .map_err(|e| e.to_string())?
        .thumbnail(800, 500);
    let mut preview = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut preview, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(EditedImage {
        path: output,
        preview: preview.into_inner(),
    })
}

#[tauri::command]
fn apply_edited(path: PathBuf) -> Result<(), String> {
    let cache = std::fs::canonicalize(edited_cache()?).map_err(|e| e.to_string())?;
    let source = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
    if !source.starts_with(cache) || source.extension().is_none_or(|e| e != "png") {
        return Err("Image must be an edited copy".into());
    }
    SystemWallpaper
        .set_wallpaper(&source)
        .map_err(|e| e.to_string())?;
    mindswall_core::history::History::record(&source, None)
        .map_err(|e| format!("Wallpaper changed, but history could not be saved: {e}"))
}

#[tauri::command]
fn gallery_images(
    state: tauri::State<'_, GalleryState>,
    offset: usize,
) -> Result<GalleryPage, String> {
    let mut images = state.0.lock().map_err(|e| e.to_string())?;
    if offset == 0 {
        *images = get_config()?.images().map_err(|e| e.to_string())?;
    }
    Ok(GalleryPage {
        total: images.len(),
        paths: images.iter().skip(offset).take(48).cloned().collect(),
    })
}

#[tauri::command]
fn gallery_thumbnail(
    state: tauri::State<'_, GalleryState>,
    path: PathBuf,
) -> Result<Vec<u8>, String> {
    if !state.0.lock().map_err(|e| e.to_string())?.contains(&path) {
        return Err("Image is not in the current collection".into());
    }
    let cache = directories::ProjectDirs::from("", "", "MindsWall")
        .ok_or("Cache directory unavailable")?
        .cache_dir()
        .join("thumbnails");
    let thumb = mindswall_core::utilities::thumbnail(&path, &cache).map_err(|e| e.to_string())?;
    std::fs::read(thumb).map_err(|e| e.to_string())
}

#[tauri::command]
fn apply_image(state: tauri::State<'_, GalleryState>, path: PathBuf) -> Result<(), String> {
    if !state.0.lock().map_err(|e| e.to_string())?.contains(&path) {
        return Err("Image is not in the current collection".into());
    }
    if ipc::send(Command::Status).is_ok() {
        let result = ipc::send(Command::Set(path.clone())).map_err(|e| e.to_string())?;
        return if result.ok {
            Ok(())
        } else {
            Err(result.message)
        };
    }
    SystemWallpaper
        .set_wallpaper(&path)
        .map_err(|e| e.to_string())?;
    mindswall_core::history::History::record(&path, None)
        .map_err(|e| format!("Wallpaper changed, but history could not be saved: {e}"))
}

#[tauri::command]
fn get_history() -> Result<Vec<mindswall_core::history::HistoryEntry>, String> {
    mindswall_core::history::History::load(
        &mindswall_core::history::History::default_path().map_err(|e| e.to_string())?,
    )
    .map(|h| h.entries)
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn reapply_history(path: PathBuf, monitor: Option<String>) -> Result<(), String> {
    let history = mindswall_core::history::History::load(
        &mindswall_core::history::History::default_path().map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if !history
        .entries
        .iter()
        .any(|entry| entry.path == path && entry.monitor == monitor)
    {
        return Err("Image is not in the wallpaper history".into());
    }
    if monitor.is_none()
        && get_config()?
            .images()
            .map_err(|e| e.to_string())?
            .contains(&path)
        && ipc::send(Command::Status).is_ok()
    {
        let response = ipc::send(Command::Set(path.clone())).map_err(|e| e.to_string())?;
        return if response.ok {
            Ok(())
        } else {
            Err(response.message)
        };
    }
    if let Some(id) = &monitor {
        mindswall_core::wallpaper::set_monitor_wallpaper(id, &path)?;
    } else {
        SystemWallpaper
            .set_wallpaper(&path)
            .map_err(|e| e.to_string())?;
    }
    mindswall_core::history::History::record(&path, monitor)
        .map_err(|e| format!("Wallpaper changed, but history could not be saved: {e}"))
}

#[tauri::command]
fn monitors() -> Result<Vec<mindswall_core::wallpaper::MonitorInfo>, String> {
    mindswall_core::wallpaper::list_monitors()
}

#[tauri::command]
fn set_monitor_image(monitor: String, path: PathBuf) -> Result<(), String> {
    mindswall_core::wallpaper::set_monitor_wallpaper(&monitor, &path)?;
    mindswall_core::history::History::record(&path, Some(monitor))
        .map_err(|e| format!("Wallpaper changed, but history could not be saved: {e}"))
}

#[tauri::command]
fn get_config() -> Result<Config, String> {
    Config::load(&Config::default_path().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(config: Config) -> Result<String, String> {
    config.validate().map_err(|e| e.to_string())?;
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
        "reload" => Command::Reload,
        _ => return Err("Unknown action".into()),
    };
    let response = ipc::send(command).map_err(|e| e.to_string())?;
    if response.ok {
        Ok(response.message)
    } else {
        Err(response.message)
    }
}

#[derive(serde::Serialize)]
struct OverviewStatus {
    state: String,
    current: Option<PathBuf>,
    position: Option<usize>,
    total: Option<usize>,
}

#[tauri::command]
fn overview_status() -> Result<OverviewStatus, String> {
    let response = ipc::send(Command::Status).map_err(|e| e.to_string())?;
    if !response.ok {
        return Err(response.message);
    }
    if let Some(status) = response.status {
        Ok(OverviewStatus {
            state: status.state,
            current: status.current,
            position: status.position,
            total: Some(status.total),
        })
    } else {
        let (state, path) = response
            .message
            .split_once(": ")
            .ok_or("Invalid slideshow status")?;
        Ok(OverviewStatus {
            state: state.to_owned(),
            current: (!path.is_empty()).then(|| PathBuf::from(path)),
            position: None,
            total: None,
        })
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
fn current_preview(path: PathBuf) -> Result<Vec<u8>, String> {
    let status = overview_status()?;
    if status.current.as_ref() != Some(&path) {
        return Err("Wallpaper has changed".into());
    }
    let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > 15 * 1024 * 1024 {
        let cache = directories::ProjectDirs::from("", "", "MindsWall")
            .ok_or("Cache directory unavailable")?
            .cache_dir()
            .join("thumbnails");
        let thumbnail =
            mindswall_core::utilities::thumbnail(&path, &cache).map_err(|e| e.to_string())?;
        return std::fs::read(thumbnail).map_err(|e| e.to_string());
    }
    std::fs::read(&path).map_err(|e| e.to_string())
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
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let action = app
                        .state::<HotkeyState>()
                        .registered
                        .try_lock()
                        .ok()
                        .and_then(|keys| keys.get(&shortcut.id()).map(|(_, action)| *action));
                    if let Some(action) = action {
                        std::thread::spawn(move || {
                            let command = match action {
                                "next" => Command::Next,
                                "previous" => Command::Previous,
                                "pause" => match ipc::send(Command::Status) {
                                    Ok(status) if status.message.starts_with("paused:") => {
                                        Command::Resume
                                    }
                                    _ => Command::Pause,
                                },
                                _ => return,
                            };
                            let _ = ipc::send(command);
                        });
                    }
                })
                .build(),
        )
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("MindsWall")
                .build(),
        )
        .manage(ServiceState::default())
        .manage(GalleryState::default())
        .manage(HotkeyState::default())
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            folder_count,
            control,
            start_service,
            service_error,
            current_preview,
            overview_status,
            get_autostart,
            set_autostart,
            gallery_images,
            gallery_thumbnail,
            apply_image,
            get_history,
            reapply_history,
            monitors,
            set_monitor_image,
            save_profile,
            apply_profile,
            delete_profile,
            export_backup,
            import_backup,
            edit_image,
            apply_edited,
            set_hotkeys,
            hotkey_error
        ])
        .setup(|app| {
            let menu = tray_menu(app.handle())?;
            let mut tray = TrayIconBuilder::with_id("main")
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
                        if let Some(index) = action
                            .strip_prefix("profile_")
                            .and_then(|value| value.parse::<usize>().ok())
                        {
                            let handle = app.clone();
                            std::thread::spawn(move || {
                                if let Ok(config) = get_config()
                                    && let Some(profile) = config.profiles.get(index)
                                {
                                    if let Err(error) = apply_profile(profile.name.clone()) {
                                        eprintln!("Could not apply profile: {error}");
                                    }
                                    refresh_tray(&handle);
                                    let _ = handle.emit("config-changed", ());
                                }
                            });
                            return;
                        }
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
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                if let Ok(config) = get_config()
                    && let Err(error) = configure_hotkeys(&handle, &config.hotkeys, false)
                    && let Ok(mut slot) = handle.state::<HotkeyState>().error.lock()
                {
                    *slot = Some(format!("Could not register shortcuts: {error}"));
                }
            });
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
