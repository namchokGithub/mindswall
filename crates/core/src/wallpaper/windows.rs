use crate::{Error, wallpaper::WallpaperSetter};
use std::{os::windows::ffi::OsStrExt, path::Path};
use windows::{
    Win32::{
        System::Com::{
            CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
        },
        UI::{
            Shell::{DWPOS_FILL, DesktopWallpaper, IDesktopWallpaper},
            WindowsAndMessaging::{
                SPI_SETDESKWALLPAPER, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SystemParametersInfoW,
            },
        },
    },
    core::PCWSTR,
};

pub struct SystemWallpaper;

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
        let primary = unsafe {
            let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
            let initialized = init.is_ok();
            let result = if initialized {
                (|| -> windows::core::Result<()> {
                    let desktop: IDesktopWallpaper =
                        CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)?;
                    desktop.SetPosition(DWPOS_FILL)?;
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
