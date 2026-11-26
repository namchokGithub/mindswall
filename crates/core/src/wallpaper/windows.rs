use crate::{Error, wallpaper::WallpaperSetter};
use std::{os::windows::ffi::OsStrExt, path::Path};
use windows::{
    Win32::{
        System::Com::{
            CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
        },
        UI::{
            Shell::{
                DWPOS_CENTER, DWPOS_FILL, DWPOS_FIT, DWPOS_SPAN, DWPOS_STRETCH, DWPOS_TILE,
                DesktopWallpaper, IDesktopWallpaper,
            },
            WindowsAndMessaging::{
                SPI_SETDESKWALLPAPER, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SystemParametersInfoW,
            },
        },
    },
    core::PCWSTR,
};

pub struct SystemWallpaper;

fn fit_position(
    mode: crate::utilities::FitMode,
) -> windows::Win32::UI::Shell::DESKTOP_WALLPAPER_POSITION {
    match mode {
        crate::utilities::FitMode::Fill => DWPOS_FILL,
        crate::utilities::FitMode::Fit => DWPOS_FIT,
        crate::utilities::FitMode::Stretch => DWPOS_STRETCH,
        crate::utilities::FitMode::Center => DWPOS_CENTER,
        crate::utilities::FitMode::Tile => DWPOS_TILE,
        crate::utilities::FitMode::Span => DWPOS_SPAN,
    }
}
fn configured_fit() -> crate::utilities::FitMode {
    crate::config::Config::default_path()
        .ok()
        .and_then(|p| crate::config::Config::load(&p).ok())
        .map(|c| c.fit_mode)
        .unwrap_or_default()
}

fn with_desktop<T>(f: impl FnOnce(&IDesktopWallpaper) -> Result<T, String>) -> Result<T, String> {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|e| format!("Could not initialize the Windows desktop API: {e}"))?;
        let result = match CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL) {
            Ok(desktop) => {
                let result = f(&desktop);
                drop(desktop);
                result
            }
            Err(error) => Err(format!("Could not open the Windows desktop API: {error}")),
        };
        CoUninitialize();
        result
    }
}

pub fn list_monitors() -> Result<Vec<crate::wallpaper::MonitorInfo>, String> {
    with_desktop(|desktop| unsafe {
        let count = desktop
            .GetMonitorDevicePathCount()
            .map_err(|e| format!("Could not count monitors: {e}"))?;
        let mut monitors = Vec::new();
        let mut unavailable = None;
        for index in 0..count {
            let id = desktop
                .GetMonitorDevicePathAt(index)
                .map_err(|e| format!("Could not get monitor {index}: {e}"))?;
            let name = id.to_string().map_err(|e| e.to_string());
            let rect = desktop.GetMonitorRECT(PCWSTR(id.0));
            windows::Win32::System::Com::CoTaskMemFree(Some(id.0.cast()));
            let name = name?;
            let rect = match rect {
                Ok(rect) => rect,
                Err(error) if error.code() == windows::Win32::Foundation::E_FAIL => {
                    tracing::warn!(monitor_index = index, %error, "skipping a monitor without readable dimensions");
                    unavailable = Some(format!(
                        "Could not read monitor {index} dimensions: {error}"
                    ));
                    continue;
                }
                Err(error) => {
                    return Err(format!(
                        "Could not read monitor {index} dimensions: {error}"
                    ));
                }
            };
            if rect.right <= rect.left || rect.bottom <= rect.top {
                continue;
            }
            monitors.push(crate::wallpaper::MonitorInfo {
                id: name,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
            });
        }
        if monitors.is_empty()
            && let Some(error) = unavailable
        {
            return Err(error);
        }
        Ok(monitors)
    })
}

pub fn set_monitor_wallpaper(monitor: &str, path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err("Image does not exist".into());
    }
    let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let monitor: Vec<u16> = monitor.encode_utf16().chain(Some(0)).collect();
    with_desktop(|desktop| unsafe {
        desktop
            .SetPosition(fit_position(configured_fit()))
            .map_err(|e| format!("Could not set the wallpaper fit mode: {e}"))?;
        desktop
            .SetWallpaper(PCWSTR(monitor.as_ptr()), PCWSTR(wide.as_ptr()))
            .map_err(|e| format!("Could not set wallpaper for this monitor: {e}"))
    })
}

impl WallpaperSetter for SystemWallpaper {
    fn set_wallpaper(&self, path: &Path) -> Result<(), Error> {
        if !path.is_file() {
            return Err(Error::InvalidFile(path.to_owned()));
        }
        let absolute = std::fs::canonicalize(path).map_err(|err| Error::Wallpaper {
            path: path.to_owned(),
            message: err.to_string(),
        })?;
        let wide: Vec<u16> = absolute.as_os_str().encode_wide().chain(Some(0)).collect();
        // COM initialization is balanced only when it succeeds; fallback works if COM is unavailable.
        let mode = configured_fit();
        let position = fit_position(mode);
        let primary = unsafe {
            let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
            let initialized = init.is_ok();
            let result = if initialized {
                (|| -> windows::core::Result<()> {
                    let desktop: IDesktopWallpaper =
                        CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)?;
                    desktop.SetPosition(position)?;
                    desktop.SetWallpaper(PCWSTR::null(), PCWSTR(wide.as_ptr()))
                })()
            } else {
                Err(init.unwrap_err())
            };
            if initialized {
                CoUninitialize();
            }
            result
        };
        if primary.is_ok() {
            return Ok(());
        }
        if mode != crate::utilities::FitMode::Fill {
            return Err(Error::Wallpaper {
                path: path.to_owned(),
                message: format!(
                    "cannot apply the selected fit mode: {}",
                    primary.unwrap_err()
                ),
            });
        }
        unsafe {
            SystemParametersInfoW(
                SPI_SETDESKWALLPAPER,
                0,
                Some(wide.as_ptr() as *mut _),
                SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
            )
        }
        .map_err(|err| Error::Wallpaper {
            path: path.to_owned(),
            message: format!("desktop API: {}; fallback: {err}", primary.unwrap_err()),
        })
    }
}
