use mindswall_core::{
    config::Config,
    ipc::{self, Command},
    scanner,
    service::Change,
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
    menu::{CheckMenuItem, Menu, MenuBuilder, SubmenuBuilder},
    tray::TrayIconBuilder,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
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
    fn folder_paths_include_profiles_and_grants_are_exact() {
        use mindswall_core::config::{Folder, Profile, ProfileSettings};
        let folder = |path: &str| Folder {
            path: path.into(),
            enabled: true,
            recursive: true,
        };
        let config = Config {
            folders: vec![folder("current")],
            profiles: vec![Profile {
                name: "Work".into(),
                settings: ProfileSettings {
                    folders: vec![folder("saved")],
                    interval_seconds: 600,
                    shuffle: true,
                    filter: Default::default(),
                    excluded: Vec::new(),
                    fit_mode: Default::default(),
                },
            }],
            ..Config::default()
        };
        let paths = folder_paths(&config);
        assert!(paths.contains(&PathBuf::from("current")));
        assert!(paths.contains(&PathBuf::from("saved")));
        let chosen = std::env::temp_dir().join("mindswall-granted-test");
        assert!(!is_granted(&chosen));
        grant(chosen.clone());
        assert!(is_granted(&chosen));
        assert!(!is_granted(&chosen.join("child")));
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
            let active = config.active_profile.as_deref() == Some(profile.name.as_str());
            let item = CheckMenuItem::with_id(
                app,
                format!("profile_{index}"),
                &profile.name,
                true,
                active,
                None::<&str>,
            )?;
            submenu = submenu.item(&item);
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

/// Paths the user chose in a native dialog during this session. Commands that
/// read or write user files accept only these or paths the app already
/// manages, so script running in the page cannot reach arbitrary files.
static GRANTED_PATHS: std::sync::LazyLock<Mutex<std::collections::HashSet<PathBuf>>> =
    std::sync::LazyLock::new(Default::default);

fn grant(path: PathBuf) -> PathBuf {
    if let Ok(mut granted) = GRANTED_PATHS.lock() {
        granted.insert(path.clone());
    }
    path
}

fn is_granted(path: &std::path::Path) -> bool {
    GRANTED_PATHS
        .lock()
        .is_ok_and(|granted| granted.contains(path))
}

fn file_dialog(app: &tauri::AppHandle) -> tauri_plugin_dialog::FileDialogBuilder<tauri::Wry> {
    let dialog = app.dialog().file();
    match app.get_webview_window("main") {
        Some(window) => dialog.set_parent(&window),
        None => dialog,
    }
}

fn picked(path: Option<tauri_plugin_dialog::FilePath>) -> Result<Option<PathBuf>, String> {
    path.map(|path| path.into_path().map_err(|e| e.to_string()))
        .transpose()
}

#[tauri::command]
async fn pick_folder(app: tauri::AppHandle) -> Result<Option<PathBuf>, String> {
    blocking(move || Ok(picked(file_dialog(&app).blocking_pick_folder())?.map(grant))).await
}

#[tauri::command]
async fn pick_image(app: tauri::AppHandle) -> Result<Option<PathBuf>, String> {
    blocking(move || {
        let dialog = file_dialog(&app).add_filter("Images", &["jpg", "jpeg", "png", "bmp"]);
        Ok(picked(dialog.blocking_pick_file())?.map(grant))
    })
    .await
}

/// Every folder path in the configuration, including those in profiles and
/// per-monitor folders.
fn folder_paths(config: &Config) -> std::collections::HashSet<PathBuf> {
    config
        .folders
        .iter()
        .chain(
            config
                .profiles
                .iter()
                .flat_map(|profile| &profile.settings.folders),
        )
        .chain(
            config
                .monitor_folders
                .iter()
                .flat_map(|entry| &entry.folders),
        )
        .map(|folder| folder.path.clone())
        .collect()
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
async fn apply_profile(app: tauri::AppHandle, name: String) -> Result<String, String> {
    let result = blocking(move || apply_profile_inner(name)).await;
    refresh_tray(&app);
    result
}

/// Save the current settings into the named profile.
#[tauri::command]
fn update_profile(app: tauri::AppHandle, name: String) -> Result<String, String> {
    let mut config = get_config()?;
    config.update_profile(&name).map_err(|e| e.to_string())?;
    config
        .save(&Config::default_path().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    refresh_tray(&app);
    Ok("Profile updated.".into())
}

#[derive(serde::Serialize)]
struct ProfileStatus {
    active: Option<String>,
    changed: bool,
}

/// The active profile and whether the current settings differ from it.
#[tauri::command]
fn profile_status() -> Result<ProfileStatus, String> {
    let config = get_config()?;
    Ok(ProfileStatus {
        changed: config.active_profile_changed(),
        active: config.active_profile,
    })
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

/// Ask for a destination and export the configuration. Returns `false` when
/// the user cancels.
#[tauri::command]
async fn export_backup(app: tauri::AppHandle) -> Result<bool, String> {
    blocking(move || {
        let dialog = file_dialog(&app)
            .add_filter("JSON", &["json"])
            .set_file_name("mindswall-backup.json");
        let Some(path) = picked(dialog.blocking_save_file())? else {
            return Ok(false);
        };
        get_config()?.save(&path).map_err(|e| e.to_string())?;
        Ok(true)
    })
    .await
}

/// Ask for a backup file and import it. Returns `false` when the user
/// cancels.
#[tauri::command]
async fn import_backup(app: tauri::AppHandle) -> Result<bool, String> {
    blocking(move || {
        let dialog = file_dialog(&app).add_filter("JSON", &["json"]);
        let Some(path) = picked(dialog.blocking_pick_file())? else {
            return Ok(false);
        };
        import_backup_inner(app, path).map(|_| true)
    })
    .await
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
    width: u32,
    height: u32,
}

/// Encode a downscaled PNG preview of an image.
fn png_preview(path: &std::path::Path, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let image = mindswall_core::utilities::open_scaled(path, (width, height))
        .map_err(|e| e.to_string())?
        .thumbnail(width, height);
    let mut preview = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut preview, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(preview.into_inner())
}

/// Resolve `path` only if it is an edited copy in the editor cache.
fn edited_copy(path: &std::path::Path) -> Result<PathBuf, String> {
    let cache = std::fs::canonicalize(edited_cache()?).map_err(|e| e.to_string())?;
    let source = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    if !source.starts_with(cache) || source.extension().is_none_or(|e| e != "png") {
        return Err("Image must be an edited copy".into());
    }
    Ok(source)
}

fn edited_cache() -> Result<PathBuf, String> {
    directories::ProjectDirs::from("", "", "MindsWall")
        .map(|d| d.cache_dir().join("edits"))
        .ok_or_else(|| "Cache directory unavailable".into())
}

fn thumbnail_cache() -> Result<PathBuf, String> {
    directories::ProjectDirs::from("", "", "MindsWall")
        .map(|d| d.cache_dir().join("thumbnails"))
        .ok_or_else(|| "Cache directory unavailable".into())
}

const MAX_EDITS: usize = 20;
const MAX_EDIT_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_THUMBNAILS: usize = 20_000;
const MAX_PREVIEWS: usize = 100;
const MAX_PREVIEW_BYTES: u64 = 200 * 1024 * 1024;
const MAX_THUMBNAIL_BYTES: u64 = 512 * 1024 * 1024;

/// Remove old edited copies, keeping any that appear in the wallpaper history
/// and the names in `also_keep`.
fn prune_edits(also_keep: &[&std::ffi::OsStr]) -> Result<(), String> {
    let history = mindswall_core::history::History::load(
        &mindswall_core::history::History::default_path().map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut keep: std::collections::HashSet<std::ffi::OsString> = history
        .entries
        .iter()
        .filter_map(|entry| entry.path.file_name().map(ToOwned::to_owned))
        .collect();
    keep.extend(also_keep.iter().map(|name| (*name).to_owned()));
    mindswall_core::utilities::prune_cache(&edited_cache()?, MAX_EDITS, MAX_EDIT_BYTES, &keep)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn edit_image(
    path: PathBuf,
    options: mindswall_core::editor::EditOptions,
) -> Result<EditedImage, String> {
    if !is_granted(&path) {
        return Err("Choose the image to edit first".into());
    }
    blocking(move || edit_image_inner(path, options)).await
}

fn edit_image_inner(
    path: PathBuf,
    options: mindswall_core::editor::EditOptions,
) -> Result<EditedImage, String> {
    let output = mindswall_core::editor::render(&path, &edited_cache()?, &options)
        .map_err(|e| e.to_string())?;
    if let Err(error) = prune_edits(&output.file_name().into_iter().collect::<Vec<_>>()) {
        eprintln!("Could not clean the edited image cache: {error}");
    }
    let (width, height) = image::image_dimensions(&output).map_err(|e| e.to_string())?;
    Ok(EditedImage {
        path: output,
        width,
        height,
    })
}

/// A downscaled preview of the image chosen for editing, sent as raw PNG bytes.
#[tauri::command]
async fn source_preview(path: PathBuf) -> Result<tauri::ipc::Response, String> {
    if !is_granted(&path) {
        return Err("Choose the image to edit first".into());
    }
    blocking(move || png_preview(&path, 800, 500))
        .await
        .map(tauri::ipc::Response::new)
}

/// The width and height of the image chosen for editing.
#[tauri::command]
async fn source_size(path: PathBuf) -> Result<(u32, u32), String> {
    if !is_granted(&path) {
        return Err("Choose the image to edit first".into());
    }
    blocking(move || mindswall_core::utilities::dimensions(&path).map_err(|e| e.to_string())).await
}

/// A downscaled preview of an edited copy, sent as raw PNG bytes.
#[tauri::command]
async fn edited_preview(path: PathBuf) -> Result<tauri::ipc::Response, String> {
    blocking(move || png_preview(&edited_copy(&path)?, 800, 500))
        .await
        .map(tauri::ipc::Response::new)
}

#[tauri::command]
async fn apply_edited(path: PathBuf) -> Result<(), String> {
    blocking(move || apply_edited_inner(path)).await
}

fn apply_edited_inner(path: PathBuf) -> Result<(), String> {
    let source = edited_copy(&path)?;
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
) -> Result<tauri::ipc::Response, String> {
    let state = state.0.clone();
    blocking(move || {
        if !state.lock().map_err(|e| e.to_string())?.contains(&path)
            && !get_config()?.excluded.contains(&path)
        {
            return Err("Image is not in the current collection".into());
        }
        cached_thumbnail(&path)
    })
    .await
    .map(tauri::ipc::Response::new)
}

fn cached_thumbnail(path: &std::path::Path) -> Result<Vec<u8>, String> {
    let thumb = mindswall_core::utilities::thumbnail(path, &thumbnail_cache()?)
        .map_err(|e| e.to_string())?;
    std::fs::read(thumb).map_err(|e| e.to_string())
}

#[tauri::command]
async fn history_thumbnail(path: PathBuf) -> Result<tauri::ipc::Response, String> {
    blocking(move || {
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
    .map(tauri::ipc::Response::new)
}

/// Gallery commands accept images in the current collection or exclusions.
fn gallery_image(gallery: &Mutex<Vec<PathBuf>>, path: &std::path::Path) -> Result<(), String> {
    // Release the gallery lock before reading the config or asking the
    // slideshow, which can take a while, so other gallery commands do not wait.
    let listed = gallery
        .lock()
        .map_err(|e| e.to_string())?
        .iter()
        .any(|image| image == path);
    if !listed && !get_config()?.excluded.iter().any(|image| image == path) && !is_shown(path) {
        return Err("Image is not in the current collection".into());
    }
    Ok(())
}

/// Whether the running slideshow shows `path` now, on the main slideshow or
/// on a monitor with its own folders, alone or in a combined wallpaper. Lets
/// Overview open image details before the gallery has been loaded.
fn is_shown(path: &std::path::Path) -> bool {
    let Ok(status) = overview_status_inner() else {
        return false;
    };
    let main = status.current.iter().chain(&status.combined);
    let monitors = status
        .monitors
        .iter()
        .flat_map(|show| show.current.iter().chain(&show.combined));
    main.chain(monitors).any(|image| image == path)
}

#[tauri::command]
async fn gallery_details(
    state: tauri::State<'_, GalleryState>,
    path: PathBuf,
) -> Result<mindswall_core::utilities::ImageProperties, String> {
    let gallery = state.0.clone();
    blocking(move || {
        gallery_image(&gallery, &path)?;
        mindswall_core::utilities::image_properties(&path).map_err(|e| e.to_string())
    })
    .await
}

/// A large preview for the gallery dialog, sent as raw PNG bytes.
#[tauri::command]
async fn gallery_preview(
    state: tauri::State<'_, GalleryState>,
    path: PathBuf,
) -> Result<tauri::ipc::Response, String> {
    let gallery = state.0.clone();
    blocking(move || {
        gallery_image(&gallery, &path)?;
        png_preview(&path, 1280, 960)
    })
    .await
    .map(tauri::ipc::Response::new)
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

/// A thumbnail of the image Windows shows on a monitor. The path comes from
/// Windows rather than the UI, so only current wallpapers can be read.
#[tauri::command]
async fn monitor_thumbnail(monitor: String) -> Result<tauri::ipc::Response, String> {
    blocking(move || {
        let path = mindswall_core::wallpaper::list_monitors()?
            .into_iter()
            .find(|info| info.id == monitor)
            .and_then(|info| info.current)
            .ok_or("This monitor has no wallpaper image")?;
        cached_thumbnail(&path)
    })
    .await
    .map(tauri::ipc::Response::new)
}

#[tauri::command]
async fn set_monitor_image(
    state: tauri::State<'_, GalleryState>,
    monitor: String,
    path: PathBuf,
) -> Result<(), String> {
    let gallery = state.0.clone();
    blocking(move || {
        if !is_granted(&path) && !gallery.lock().map_err(|e| e.to_string())?.contains(&path) {
            return Err("Choose the image for this monitor first".into());
        }
        mindswall_core::wallpaper::set_monitor_wallpaper(&monitor, &path)?;
        mindswall_core::history::History::record(&path, Some(monitor))
            .map_err(|e| format!("Wallpaper changed, but history could not be saved: {e}"))
    })
    .await
}

/// The project's home page, from `repository` in Cargo.toml.
const PROJECT_URL: &str = env!("CARGO_PKG_REPOSITORY");

#[derive(serde::Serialize)]
struct AppInfo {
    name: String,
    version: String,
    source: &'static str,
    license: &'static str,
}

#[tauri::command]
fn app_info(app: tauri::AppHandle) -> AppInfo {
    let package = app.package_info();
    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        source: PROJECT_URL,
        license: env!("CARGO_PKG_LICENSE"),
    }
}

/// Open a page of the project in the default browser. Only these fixed
/// addresses can be opened, never one sent by the page.
#[tauri::command]
async fn open_project_link(link: String) -> Result<(), String> {
    let url = match link.as_str() {
        "source" => PROJECT_URL.to_owned(),
        "issues" => format!("{PROJECT_URL}/issues"),
        "releases" => format!("{PROJECT_URL}/releases"),
        "license" => format!("{PROJECT_URL}/blob/main/LICENSE"),
        _ => return Err("Unknown link".into()),
    };
    blocking(move || mindswall_core::explorer::open_url(&url)).await
}

/// Open a folder from the settings in File Explorer. Other paths are refused.
#[tauri::command]
async fn open_folder(path: PathBuf) -> Result<(), String> {
    blocking(move || {
        let config = get_config()?;
        let known = config
            .folders
            .iter()
            .chain(
                config
                    .monitor_folders
                    .iter()
                    .flat_map(|entry| &entry.folders),
            )
            .any(|folder| folder.path == path);
        if !known {
            return Err("This folder is not in your settings".into());
        }
        mindswall_core::explorer::open_folder(&path)
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
    let known = folder_paths(&previous);
    if let Some(path) = folder_paths(&config)
        .into_iter()
        .find(|path| !known.contains(path) && !is_granted(path))
    {
        return Err(format!(
            "Add folders with the folder picker: {}",
            path.display()
        ));
    }
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
        if !is_granted(&path) && !folder_paths(&get_config()?).contains(&path) {
            return Err("Folder is not in the collection".into());
        }
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

/// Show the next or previous image on one monitor with its own folders.
#[tauri::command]
async fn monitor_control(monitor: String, action: String) -> Result<String, String> {
    blocking(move || {
        let command = match action.as_str() {
            "next" => Command::NextMonitor(monitor),
            "previous" => Command::PreviousMonitor(monitor),
            _ => return Err("Unknown action".into()),
        };
        let response = ipc::send(command).map_err(|e| e.to_string())?;
        if response.ok {
            Ok(response.message)
        } else {
            Err(response.message)
        }
    })
    .await
}

#[derive(serde::Serialize)]
struct OverviewStatus {
    state: String,
    current: Option<PathBuf>,
    position: Option<usize>,
    total: Option<usize>,
    /// Images shown next to `current` in a combined wallpaper.
    combined: Vec<PathBuf>,
    /// The combined wallpaper file, used for the preview.
    display: Option<PathBuf>,
    /// Connected monitors that run their own slideshow.
    monitors: Vec<ipc::MonitorPlayback>,
    /// Why the slideshow paused itself, such as a wallpaper chosen in Windows.
    paused_reason: Option<String>,
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
            combined: status.combined,
            display: status.display,
            monitors: status.monitors,
            paused_reason: status.paused_reason,
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
            combined: Vec::new(),
            display: None,
            monitors: Vec::new(),
            paused_reason: None,
        })
    }
}

#[tauri::command]
async fn start_service(
    app: tauri::AppHandle,
    state: tauri::State<'_, ServiceState>,
) -> Result<(), String> {
    let starting = state.starting.clone();
    let error = state.error.clone();
    blocking(move || start_service_inner(app, starting, error)).await
}

fn start_service_inner(
    app: tauri::AppHandle,
    starting: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
) -> Result<(), String> {
    let mut resume = None;
    if let Ok(response) = ipc::send(Command::Status) {
        // A slideshow this app did not start runs in the CLI. Ask it to hand
        // over so the app owns the pipe and can push changes to the UI. An
        // older CLI does not know the request; then keep using it.
        if !starting.load(Ordering::SeqCst)
            && let Ok(handoff) = ipc::send(Command::Handoff)
            && handoff.ok
        {
            resume = handoff.status;
        } else if response
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
        } else {
            return Ok(());
        }
    }
    if starting.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    *error.lock().map_err(|e| e.to_string())? = None;
    std::thread::spawn(move || {
        // Tell the UI about every change so it does not need to poll a
        // slideshow that this app runs.
        let notify = |status: &ipc::PlaybackStatus, change: Change| {
            let _ = app.emit("slideshow-changed", status);
            if let Change::Collection { added, removed } = change {
                // Folder counts and the gallery are not part of the status.
                let _ = app.emit(
                    "collection-changed",
                    serde_json::json!({ "added": added, "removed": removed }),
                );
            }
        };
        let options = mindswall_core::service::ServiceOptions {
            listen_retry: if resume.is_some() {
                std::time::Duration::from_secs(5)
            } else {
                std::time::Duration::ZERO
            },
            resume,
            ..Default::default()
        };
        if let Err(e) = mindswall_core::service::run_slideshow_with(options, notify)
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
async fn current_preview(path: PathBuf) -> Result<tauri::ipc::Response, String> {
    blocking(move || current_preview_inner(path))
        .await
        .map(tauri::ipc::Response::new)
}

fn current_preview_inner(path: PathBuf) -> Result<Vec<u8>, String> {
    let status = overview_status_inner()?;
    // Overview previews the main slideshow and each monitor with its own folders.
    let shown = [&status.current, &status.display].into_iter().chain(
        status
            .monitors
            .iter()
            .flat_map(|show| [&show.current, &show.display]),
    );
    if !shown.flatten().any(|image| image == &path) {
        return Err("Wallpaper has changed".into());
    }
    // A downscaled, cached copy keeps the transfer and decoding in the
    // WebView small; previews of images shown before load from the cache.
    let cache = preview_cache()?;
    let preview = || mindswall_core::utilities::preview(&path, &cache).map_err(|e| e.to_string());
    let (mut file, mut cached) = preview()?;
    let bytes = match std::fs::read(&file) {
        Ok(bytes) => bytes,
        // Another request cleaned the cache right after this one found the
        // file; make the preview again.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            (file, cached) = preview()?;
            std::fs::read(&file).map_err(|e| e.to_string())?
        }
        Err(error) => return Err(error.to_string()),
    };
    if !cached {
        let keep = file
            .file_name()
            .map(ToOwned::to_owned)
            .into_iter()
            .collect();
        if let Err(error) =
            mindswall_core::utilities::prune_cache(&cache, MAX_PREVIEWS, MAX_PREVIEW_BYTES, &keep)
        {
            eprintln!("Could not clean the preview cache: {error}");
        }
    }
    Ok(bytes)
}

fn preview_cache() -> Result<PathBuf, String> {
    directories::ProjectDirs::from("", "", "MindsWall")
        .map(|d| d.cache_dir().join("previews"))
        .ok_or_else(|| "Cache directory unavailable".into())
}

/// The log file being written, or the logs folder before any log exists.
#[tauri::command]
fn log_location() -> Result<String, String> {
    mindswall_core::logs::latest_log()
        .or_else(mindswall_core::logs::logs_dir)
        .map(|path| path.display().to_string())
        .ok_or_else(|| "User data directory unavailable".into())
}

/// Open the folder that holds the slideshow logs in File Explorer.
#[tauri::command]
async fn open_log_folder() -> Result<(), String> {
    blocking(|| {
        let folder = mindswall_core::logs::logs_dir().ok_or("User data directory unavailable")?;
        if !folder.is_dir() {
            return Err("There is no log yet. Start the slideshow first.".into());
        }
        mindswall_core::explorer::open_folder(&folder)
    })
    .await
}

/// The last 100 lines of the newest log file, oldest first.
#[tauri::command]
async fn recent_log() -> Result<Vec<String>, String> {
    blocking(|| match mindswall_core::logs::latest_log() {
        Some(path) => mindswall_core::utilities::tail_lines(&path, 100).map_err(|e| e.to_string()),
        None => Ok(Vec::new()),
    })
    .await
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

/// Launched by "Start with Windows": stay in the tray without a window.
const HIDDEN_ARG: &str = "--hidden";

/// Show and focus the main window, creating it when it was closed. Closing
/// the window destroys its WebView so the background process uses less
/// memory; the slideshow, tray, and shortcuts keep running.
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    // Creating a window from an event handler can deadlock on Windows, so
    // build it from another thread.
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(error) = create_main_window(&app) {
            eprintln!("Could not open the window: {error}");
        }
    });
}

fn create_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    let Some(config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .cloned()
    else {
        return Ok(());
    };
    match tauri::WebviewWindowBuilder::from_config(app, &config)?.build() {
        Ok(window) => {
            mindswall_core::utilities::log_memory("window opened");
            let _ = window.set_focus();
            Ok(())
        }
        // Another request created the window first.
        Err(_) if app.get_webview_window("main").is_some() => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if !args.iter().any(|arg| arg == HIDDEN_ARG) {
                show_main_window(app);
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
                .arg(HIDDEN_ARG)
                .build(),
        )
        .manage(ServiceState::default())
        .manage(GalleryState::default())
        .manage(HotkeyState::default())
        .invoke_handler(tauri::generate_handler![
            get_config,
            config_warning,
            pick_folder,
            pick_image,
            save_config,
            folder_count,
            control,
            monitor_control,
            open_folder,
            app_info,
            open_project_link,
            start_service,
            service_error,
            current_preview,
            overview_status,
            get_autostart,
            set_autostart,
            log_location,
            open_log_folder,
            recent_log,
            gallery_images,
            gallery_thumbnail,
            history_thumbnail,
            gallery_details,
            gallery_preview,
            apply_image,
            get_history,
            reapply_history,
            monitors,
            monitor_thumbnail,
            set_monitor_image,
            save_profile,
            apply_profile,
            update_profile,
            profile_status,
            delete_profile,
            export_backup,
            import_backup,
            edit_image,
            edited_preview,
            source_preview,
            source_size,
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
                    "open" => show_main_window(app),
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
            if !std::env::args().any(|arg| arg == HIDDEN_ARG) {
                create_main_window(app.handle())?;
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                if let Ok(config) = get_config()
                    && let Err(error) = configure_hotkeys(&handle, &config.hotkeys, false)
                    && let Ok(mut slot) = handle.state::<HotkeyState>().error.lock()
                {
                    *slot = Some(format!("Could not register shortcuts: {error}"));
                }
            });
            std::thread::spawn(|| {
                if let Err(error) = prune_edits(&[]) {
                    eprintln!("Could not clean the edited image cache: {error}");
                }
                let result = thumbnail_cache().and_then(|cache| {
                    mindswall_core::utilities::prune_cache(
                        &cache,
                        MAX_THUMBNAILS,
                        MAX_THUMBNAIL_BYTES,
                        &Default::default(),
                    )
                    .map_err(|e| e.to_string())
                });
                if let Err(error) = result {
                    eprintln!("Could not clean the thumbnail cache: {error}");
                }
            });
            if get_config().is_ok_and(|config| config.folders.iter().any(|folder| folder.enabled)) {
                let state = app.state::<ServiceState>();
                let (starting, error) = (state.starting.clone(), state.error.clone());
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    let _ = start_service_inner(handle, starting, error);
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building MindsWall")
        .run(|_, event| match event {
            // Closing the last window must not quit: the app keeps running
            // in the tray. Quit from the tray passes an exit code.
            tauri::RunEvent::ExitRequested {
                api, code: None, ..
            } => api.prevent_exit(),
            // The window's WebView is gone; give its memory back to Windows
            // while the app waits in the tray.
            tauri::RunEvent::WindowEvent {
                event: tauri::WindowEvent::Destroyed,
                ..
            } => {
                mindswall_core::utilities::log_memory("window closed");
                mindswall_core::utilities::trim_memory_soon();
            }
            _ => {}
        });
}
