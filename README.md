# MindsWall

A lightweight, local-first wallpaper manager for Windows that does not require
an internet connection or administrator privileges.

## Project status

The project is in early development. The Rust CLI can scan folders and set or randomly select a wallpaper on Windows. The desktop interface and slideshow are planned.

## Planned features

- Manage multiple image folders and search their subfolders.
- Change wallpapers on a schedule and shuffle images without repeats until the set is exhausted.
- Control the app from the system tray and save settings.
- Support multiple monitors, an image gallery, wallpaper history, and file change detection.
- Add wallpaper profiles, image editing tools, and keyboard shortcuts.

## Technology

- **Target platform:** Windows 11
- **Backend:** Rust
- **Planned desktop UI:** Tauri 2, React, and TypeScript
- **Planned distribution:** Portable EXE and installer

## Getting started

Install Rust and Cargo, then run the CLI from the repository root:

```powershell
cargo run -p mindswall-cli -- scan "C:\Pictures"
cargo run -p mindswall-cli -- scan --no-recursive "C:\Pictures" "D:\Wallpapers"
cargo run -p mindswall-cli -- set "C:\Pictures\wallpaper.jpg"
cargo run -p mindswall-cli -- random "C:\Pictures"
```

Scanning includes subfolders by default; use `--no-recursive` to restrict it to
the supplied folders or `--recursive` to make the default explicit. Supported
extensions are JPG, JPEG, PNG, and BMP. Errors are reported to stderr. Wallpaper
commands require Windows. The Windows fallback uses the existing fit setting if
the desktop wallpaper API is unavailable.

### Slideshow (Windows)

Copy `config.example.json` to `%APPDATA%\MindsWall\config\config.json` and
edit the folder paths. JSON paths need doubled backslashes. Enable or disable
each folder and set whether it includes subfolders. The interval is in seconds
(60–86400); set `shuffle` to `false` for sequential order. Config changes are
loaded when the slideshow starts.

```powershell
New-Item -ItemType Directory -Force "$env:APPDATA\MindsWall\config"
Copy-Item config.example.json "$env:APPDATA\MindsWall\config\config.json"
# Edit the copied config.json before starting.
cargo run -p mindswall-cli -- run
```

Keep that terminal open. From another terminal, run any of these commands:

```powershell
cargo run -p mindswall-cli -- status
cargo run -p mindswall-cli -- next
cargo run -p mindswall-cli -- previous
cargo run -p mindswall-cli -- pause
cargo run -p mindswall-cli -- resume
cargo run -p mindswall-cli -- config show
```

Press Ctrl+C in the slideshow terminal to stop it. Logs are stored under
`%APPDATA%\MindsWall\data\mindswall.log`. The slideshow and Windows wallpaper
integration still need end-to-end testing in an interactive Windows session.

### Desktop app (development)

Install Node.js, Rust, and the Tauri Windows prerequisites (including WebView2
and the MSVC build tools), then from the project root run:

```powershell
npm.cmd --prefix ui install
npm.cmd --prefix ui run tauri dev
```

Use **Folders** to select image directories, then start the slideshow from
**Overview**. Closing the window hides it to the system tray; use the tray
menu to open or quit the app. **Settings** includes the per-user Windows
startup option. The app and the CLI use the same local configuration and control
pipe. Desktop interactions and startup behavior still require interactive
Windows verification.

Disabling or removing the last enabled folder stops the slideshow and applies
the Windows image at `%SystemRoot%\Web\Wallpaper\Windows\img0.jpg`. If the
slideshow was running, enabling a folder again resumes it; if it was paused,
it stays paused. When the slideshow service is not running, use **Start
slideshow** after enabling a folder. The app reports an error if the Windows
image cannot be applied.

Overview shows the current position and total in the active slideshow cycle.
The total includes enabled subfolders and reflects active filters and exclusions.
Use **Rescan collection** to pick up file changes; this starts a new cycle.
Use **Quick settings** on Overview to change the slideshow interval and fit mode.

Feedback appears in a temporary snackbar on the page where the action was made.
Notifications are cleared when navigating to another page.

The desktop **Gallery** shows a loading state while it finds images, loads them
in pages, and keeps local thumbnails. You
can set or exclude an image, and restore exclusions in **Settings**. Settings
also contains fit, resolution, orientation, and aspect-ratio filters. **History**
keeps the most recent 100 applied images locally, and **Monitors** lists
connected displays and can apply an image to one display. A running slideshow
can replace a per-monitor selection at its next change.

These utilities remain in development. Folder watching and per-monitor slideshow
sources are not yet implemented. Multi-monitor behavior and the gallery with a
large collection still need interactive Windows verification.

**Profiles** saves named collections and can import or export settings and
profiles as JSON. The **Editor** creates a cached copy with optional crop,
resize, blur, and dim; the original image remains untouched. Global shortcuts
can be configured in **Settings**, and saved profiles are available from the
system tray. These features still need interactive Windows verification. A
running slideshow may replace an edited wallpaper at its next change.

See the [requirements](docs/requirements/requirement.md) for the feature scope,
architecture, and development roadmap.

---

© 2026 Namchok Singhachai. MindsWall is released under the [MIT License](LICENSE).
