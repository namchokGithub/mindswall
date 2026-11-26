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

    #[test]
    fn accepts_recorded_keyboard_codes() {
        let hotkeys = mindswall_core::config::Hotkeys {
            next: "Ctrl+Alt+ArrowRight".into(),
            previous: "Ctrl+Shift+KeyA".into(),
            pause: "F12".into(),
        };
        assert_eq!(parse_hotkeys(&hotkeys).unwrap().len(), 3);
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

/// Run blocking work (folder scans, Windows API calls, slideshow IPC) on a
/// worker thread. Synchronous commands run on the main thread and would
/// freeze the window while they wait.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
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
struct GalleryState(std::sync::Arc<Mutex<Vec<PathBuf>>>);

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
async fn apply_profile(name: String) -> Result<String, String> {
    blocking(move || apply_profile_inner(name)).await
}

fn apply_profile_inner(name: String) -> Result<String, String> {
    let mut config = get_config()?;
    config.apply_profile(&name).map_err(|e| e.to_string())?;
    save_config_inner(config)
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
async fn apply_edited(path: PathBuf) -> Result<(), String> {
    blocking(move || apply_edited_inner(path)).await
}

fn apply_edited_inner(path: PathBuf) -> Result<(), String> {
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
async fn gallery_images(
    state: tauri::State<'_, GalleryState>,
    offset: usize,
) -> Result<GalleryPage, String> {
    let state = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if offset == 0 {
            let scanned = get_config()?.images().map_err(|e| e.to_string())?;
            *state.lock().map_err(|e| e.to_string())? = scanned;
        }
        let images = state.lock().map_err(|e| e.to_string())?;
        Ok(GalleryPage {
            total: images.len(),
            paths: images.iter().skip(offset).take(48).cloned().collect(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn gallery_thumbnail(
    state: tauri::State<'_, GalleryState>,
    path: PathBuf,
) -> Result<Vec<u8>, String> {
    let state = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if !state.lock().map_err(|e| e.to_string())?.contains(&path)
            && !get_config()?.excluded.contains(&path)
        {
            return Err("Image is not in the current collection".into());
        }
        cached_thumbnail(&path)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn cached_thumbnail(path: &std::path::Path) -> Result<Vec<u8>, String> {
    let cache = directories::ProjectDirs::from("", "", "MindsWall")
        .ok_or("Cache directory unavailable")?
        .cache_dir()
        .join("thumbnails");
    let thumb = mindswall_core::utilities::thumbnail(path, &cache).map_err(|e| e.to_string())?;
    std::fs::read(thumb).map_err(|e| e.to_string())
}

#[tauri::command]
async fn history_thumbnail(path: PathBuf) -> Result<Vec<u8>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let history = mindswall_core::history::History::load(
            &mindswall_core::history::History::default_path().map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if !history.entries.iter().any(|entry| entry.path == path) {
            return Err("Image is not in the wallpaper history".into());
        }
        cached_thumbnail(&path)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(serde::Serialize)]
struct GalleryDetails {
    properties: mindswall_core::utilities::ImageProperties,
    preview: Vec<u8>,
}

#[tauri::command]
async fn gallery_details(
    state: tauri::State<'_, GalleryState>,
    path: PathBuf,
) -> Result<GalleryDetails, String> {
    let state = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if !state.lock().map_err(|e| e.to_string())?.contains(&path)
            && !get_config()?.excluded.contains(&path)
        {
            return Err("Image is not in the current collection".into());
        }
        let properties =
            mindswall_core::utilities::image_properties(&path).map_err(|e| e.to_string())?;
        let image = image::open(&path)
            .map_err(|e| e.to_string())?
            .thumbnail(1280, 960);
        let mut preview = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut preview, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(GalleryDetails {
            properties,
            preview: preview.into_inner(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn apply_image(state: tauri::State<'_, GalleryState>, path: PathBuf) -> Result<(), String> {
    let gallery = state.0.clone();
    blocking(move || {
        if !gallery.lock().map_err(|e| e.to_string())?.contains(&path) {
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
    })
    .await
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
async fn reapply_history(path: PathBuf, monitor: Option<String>) -> Result<(), String> {
    blocking(move || reapply_history_inner(path, monitor)).await
}

fn reapply_history_inner(path: PathBuf, monitor: Option<String>) -> Result<(), String> {
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
async fn monitors() -> Result<Vec<mindswall_core::wallpaper::MonitorInfo>, String> {
    blocking(mindswall_core::wallpaper::list_monitors).await
}

#[tauri::command]
async fn set_monitor_image(monitor: String, path: PathBuf) -> Result<(), String> {
    blocking(move || {
        mindswall_core::wallpaper::set_monitor_wallpaper(&monitor, &path)?;
        mindswall_core::history::History::record(&path, Some(monitor))
            .map_err(|e| format!("Wallpaper changed, but history could not be saved: {e}"))
    })
    .await
}

#[tauri::command]
fn get_config() -> Result<Config, String> {
    Config::load(&Config::default_path().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

#[tauri::command]
fn config_warning() -> Option<String> {
    Config::take_quarantined().map(|backup| {
        format!(
            "The configuration file was invalid and was moved to {}. Default settings are in use.",
            backup.display()
        )
    })
}

#[tauri::command]
async fn save_config(config: Config) -> Result<String, String> {
    blocking(move || save_config_inner(config)).await
}

fn save_config_inner(config: Config) -> Result<String, String> {
    config.validate().map_err(|e| e.to_string())?;
    let previous = get_config()?;
    let disabled_last_folder = previous.folders.iter().any(|folder| folder.enabled)
        && !config.folders.iter().any(|folder| folder.enabled);
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
    } else if disabled_last_folder {
        let path = mindswall_core::wallpaper::default_wallpaper_path().map_err(|e| {
            format!("Settings saved, but Windows default wallpaper could not be applied: {e}")
        })?;
        SystemWallpaper.set_wallpaper(&path).map_err(|e| {
            format!("Settings saved, but Windows default wallpaper could not be applied: {e}")
        })?;
    }
    Ok(if disabled_last_folder {
        "All folders disabled. Using Windows default wallpaper."
    } else {
        "Settings saved."
    }
    .into())
}

#[tauri::command]
async fn folder_count(path: PathBuf, recursive: bool) -> Result<usize, String> {
    blocking(move || {
        if !path.is_dir() {
            return Err(mindswall_core::Error::InvalidFolder(path).to_string());
        }
        Ok(scanner::scan_lenient(&[path], recursive).images.len())
    })
    .await
}

#[tauri::command]
async fn control(action: String) -> Result<String, String> {
    blocking(move || control_inner(&action)).await
}

fn control_inner(action: &str) -> Result<String, String> {
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
async fn overview_status() -> Result<OverviewStatus, String> {
    blocking(overview_status_inner).await
}

fn overview_status_inner() -> Result<OverviewStatus, String> {
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
async fn start_service(state: tauri::State<'_, ServiceState>) -> Result<(), String> {
    let starting = state.starting.clone();
    let error = state.error.clone();
    blocking(move || start_service_inner(starting, error)).await
}

fn start_service_inner(
    starting: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
) -> Result<(), String> {
    if let Ok(response) = ipc::send(Command::Status) {
        if response
            .status
            .as_ref()
            .is_some_and(|status| status.state == "stopped")
        {
            let result = ipc::send(Command::Reload).map_err(|e| e.to_string())?;
            return if result.ok {
                Ok(())
            } else {
                Err(result.message)
            };
        }
        return Ok(());
    }
    if starting.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    *error.lock().map_err(|e| e.to_string())? = None;
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
async fn current_preview(path: PathBuf) -> Result<Vec<u8>, String> {
    blocking(move || current_preview_inner(path)).await
}

fn current_preview_inner(path: PathBuf) -> Result<Vec<u8>, String> {
    let status = overview_status_inner()?;
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
            config_warning,
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
            history_thumbnail,
            gallery_details,
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
                                    if let Err(error) = apply_profile_inner(profile.name.clone()) {
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
                let state = app.state::<ServiceState>();
                let (starting, error) = (state.starting.clone(), state.error.clone());
                std::thread::spawn(move || {
                    let _ = start_service_inner(starting, error);
                });
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
