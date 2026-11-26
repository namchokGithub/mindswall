//! Opening folders in File Explorer.

use std::path::Path;

/// Open `path` in File Explorer. Fails when it is not an existing folder.
#[cfg(windows)]
pub fn open_folder(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::{
            System::Com::{
                COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
            },
            UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        },
        core::{PCWSTR, w},
    };
    if !path.is_dir() {
        return Err("This folder is not available".into());
    }
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: `wide` is NUL-terminated and outlives the call. COM is
    // initialized first, as the ShellExecuteW documentation recommends, and
    // uninitialized only when this call initialized it.
    let code = unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
        let result = ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
        if init.is_ok() {
            CoUninitialize();
        }
        result.0 as isize
    };
    // Values above 32 mean success.
    if code <= 32 {
        return Err(format!("Could not open the folder (error {code})"));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn open_folder(_: &Path) -> Result<(), String> {
    Err("Opening folders is supported only on Windows".into())
}
