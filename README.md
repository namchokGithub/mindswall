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
commands require Windows. If the desktop wallpaper API is unavailable and the
fit mode is Fill, the Windows fallback changes the image and keeps the existing
fit setting; other fit modes report an error. The slideshow log records each
successful change and each failure.

### Slideshow (Windows)

Copy `config.example.json` to `%APPDATA%\MindsWall\config\config.json` and
edit the folder paths. JSON paths need doubled backslashes. Enable or disable
each folder and set whether it includes subfolders. The interval is in seconds
(60–86400); set `shuffle` to `false` for sequential order. Config changes are
loaded when the slideshow starts. If the config file is invalid, it is renamed
to `config.json.invalid-<timestamp>` and default settings are used, so the
original file is kept. Folders or subfolders that cannot be read are logged
and skipped, and images that cannot be applied (for example, deleted files)
are skipped until the collection is reloaded.

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

Press Ctrl+C in the slideshow terminal to stop it. When the desktop app starts
its slideshow while `run` is active, the CLI hands over: it prints a message
and exits, and the app continues with the same image and paused state. Logs are stored under
`%APPDATA%\MindsWall\data\mindswall.log`. The desktop app shows this path under
**Settings** > **Log file**, with the Windows user name hidden as `******`;
the **Copy path** button copies the full path. The slideshow and Windows wallpaper
integration still need end-to-end testing in an interactive Windows session.

### Desktop app (development)

Install Node.js, Rust, and the Tauri Windows prerequisites (including WebView2
and the MSVC build tools), then from the project root run:

```powershell
npm.cmd --prefix ui install
npm.cmd --prefix ui run tauri dev
```

Use **Folders** to select image directories, then start the slideshow from
**Overview**. Closing the window releases its WebView to save memory while the
app keeps running in the system tray; use the tray menu to open the window
again or quit the app. **Settings** includes the per-user Windows startup
option; when started with Windows, the app starts in the tray without opening a
window (`--hidden`). Turn the startup option off and on once to update a
startup entry made by an earlier build. The app and the CLI use the same local configuration and control
pipe. Desktop interactions and startup behavior still require interactive
Windows verification.

The window uses its own minimal title bar in the app's colors, with minimize,
maximize, and close buttons; drag it to move the window and double-click it to
maximize. **Settings** > **Theme** chooses Light, Dark, or System (follows the
Windows app mode; stored as `theme` in the config). The sidebar stays in place
while the page scrolls, scroll bars are hidden, dropdowns use the app's own
style, and only file names and paths can be selected and copied.

Disabling or removing the last enabled folder stops the slideshow and applies
the Windows image at `%SystemRoot%\Web\Wallpaper\Windows\img0.jpg`. If the
slideshow was running, enabling a folder again resumes it; if it was paused,
it stays paused. When the slideshow service is not running, use **Start
slideshow** after enabling a folder. The app reports an error if the Windows
image cannot be applied.

Overview shows the current position and total in the active slideshow cycle.
The total includes enabled subfolders and reflects active filters and exclusions.
While the slideshow runs, enabled folders are watched: images that are added,
removed, or renamed update the collection about two seconds after the last
change, without a full rescan, and new images join the current cycle. Files
whose image header cannot be read yet, such as files still being copied, are
picked up by a later change. When the desktop app runs the slideshow, the
Overview total, the **Folders** image counts, and an open **Gallery** refresh
after such a change. Use **Rescan collection** to force a full rescan. Saving settings keeps the current cycle: the wallpaper changes only when
the current image leaves the collection or the fit mode changes.
Use **Quick settings** on Overview to change the slideshow interval and fit mode.

Feedback appears in a temporary snackbar on the page where the action was made.
Notifications are cleared when navigating to another page.

The desktop **Gallery** shows a loading state while it finds images, loads them
in pages, and keeps local thumbnails. It includes enabled subfolders and shows
active filters and exclusions. Select an image to preview it and view its file
and image details. Use **Load more** to view additional images. Choose a large
grid, compact grid, or list; the selected layout is remembered on this device.
You can set or exclude an image, then use
**Excluded** in Gallery to restore individual images or **Settings** to restore
all exclusions. Restored images still need to match enabled folders and filters. Settings
also contains fit, resolution, orientation, and aspect-ratio filters.

**Combine portrait images** (Settings, off by default) shows a portrait image on
a landscape monitor together with the next portrait images of the current
cycle, side by side at full height over a blurred, darkened copy of the first
image, with an adjustable space between them (0–32 px, default 4). Images that
are shown together count as shown in the cycle. The combined image is made at
the monitor's resolution (all monitors together for Span, the primary monitor
for a shared slideshow, each monitor's own size for monitors with their own
folders), works with every fit mode, and is cached in
`%LOCALAPPDATA%\MindsWall\cache\collages` (the 10 most recent files).
Previous shows the same combination again. Overview shows a "+N images" badge,
and History notes how many images were combined. The original files are never
changed. This setting is not part of profiles. **History**
shows thumbnails for the most recent 10 applied images locally. With more than
one display, each entry names its display or the main slideshow, and a filter
shows one display; the limit of 10 covers all displays together.

**Monitors** draws the display arrangement to scale, like Windows display
settings, with each display's current wallpaper. Displays are numbered left to
right and show the name the monitor reports (for example its model), its
resolution, and whether it is the primary display. Use the pencil button to
give a display your own name (up to 40 characters, stored in `monitor_names`);
the name is used in Monitors, Overview, and History. **Choose image** applies an
image to one display; a running slideshow can replace it at its next change.

In **Monitors**, a display can also get its own folders. It then runs its own
slideshow at the same interval, with the same shuffle, filter, exclusion, and
fit settings, while other displays share the main slideshow. Next, Previous,
and Pause apply to every display. The slideshow still needs at least one
enabled folder in **Folders**. While any display has its own folders, the
slideshow lists monitors every 10 seconds: a reconnected display gets its image
again and continues its own cycle, and a disconnected display is skipped.
Folders and names of disconnected displays stay saved under **Saved monitor
settings** until you choose **Forget**, or **Move** them to a connected display
that has no folders of its own, for example when a monitor was moved to another
port and got a new device path. The configuration keys them by the Windows
monitor device path. Profiles do not include per-monitor folders or names.
While a display has its own folders, Overview labels the large preview as the
main slideshow and shows a thumbnail for each display below it.
Windows may retain unavailable display records, such as displays connected
earlier or turned off; Monitors skips records whose dimensions cannot be read
(`0x80004005`) while showing the displays that are available. These skips are
not written to the log, because they are normal and repeat on every listing.
If the history file is invalid, it is renamed to
`history.json.invalid-<timestamp>` and a new history starts. Exclusions match
paths without regard to letter case or `/` and `\` separators.

Local caches are limited: at startup the app keeps at most 20,000 thumbnails
(512 MB), and the Editor keeps the 20 most recent edited copies (1 GB) plus any
copy listed in History. Caches are stored under `%LOCALAPPDATA%\MindsWall\cache`.
While the window is minimized, the app stops polling the slideshow status.

Folders, images, and backup files are chosen through native dialogs opened by
the app. The desktop app only reads or writes paths chosen this way during the
current session, or paths that are already part of the collection, history,
or configuration.

These utilities remain in development. Overview updates when the app's
slideshow changes. A slideshow run from a CLI build without hand-over support
is checked every 30 seconds while the window is open. Multi-monitor behavior and the gallery with a
large collection still need interactive Windows verification.

**Profiles** saves named collections and can import or export settings and
profiles as JSON. A profile stores folders, interval, shuffle, filters,
exclusions, and fit mode. The active profile is shown at the top of every page
and checked in the tray **Profiles** submenu. When those settings change after
a profile was applied, it is marked **changed**: **Update profile** saves the
current settings into it, and **Revert** applies its saved settings again. The **Editor** creates a cached copy with optional crop,
resize, blur, and dim; the original image remains untouched. The Editor shows
a loading indicator while creating a preview and keeps its work when switching
pages during the same app session. Global shortcuts
can be recorded using the keyboard in **Settings**, and saved profiles are available from the
system tray. These features still need interactive Windows verification. A
running slideshow may replace an edited wallpaper at its next change.

### Continuous integration and performance

The GitHub Actions workflow in `.github/workflows/ci.yml` runs on Windows for
pushes and pull requests. It builds the frontend, then runs `cargo fmt --check`,
Clippy with warnings denied, and the Rust tests, and builds the NSIS installer
as a downloadable workflow artifact. Pushing a `v*.*.*` tag runs
`.github/workflows/release.yml`, which builds the installer and a portable ZIP
and attaches them to a draft GitHub Release; see
[docs/releasing.md](docs/releasing.md) for the version bump and changelog
steps. See
[docs/performance.md](docs/performance.md) for how to measure CPU and memory
use with `scripts/measure-performance.ps1`; no results have been recorded yet.

See the [requirements](docs/requirements/requirement.md) for the feature scope,
architecture, and development roadmap.

---

© 2026 Namchok Singhachai. MindsWall is released under the [MIT License](LICENSE).
