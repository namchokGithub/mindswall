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
- **Planned distribution:** source code, a portable ZIP (with `MindsWall.exe`),
  and a per-user installer through GitHub Releases. Builds are not
  code-signed, so Windows may block them; see
  [Installing a release build](#installing-a-release-build-windows-smartscreen).

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
the **Copy path** button copies the full path, **Open folder** opens the log
folder in File Explorer, and **View log** shows the last 100 lines (user names
in paths hidden), with a button to read them again. The slideshow and Windows wallpaper
integration still need end-to-end testing in an interactive Windows session.

### Desktop app (development)

Install Node.js, Rust, and the Tauri Windows prerequisites (including WebView2
and the MSVC build tools), then from the project root run:

```powershell
npm.cmd --prefix ui install
npm.cmd --prefix ui run tauri dev
```

Use **Folders** to select image directories, then start the slideshow from
**Overview**. Folder paths on **Folders** and **Monitors** show only the last
folder (for example `******\wallpapers`) so the rest stays out of screenshots;
select a path to copy the full path. On **Folders** and **Monitors**, each folder also
has an **Open folder** (File Explorer) button, and removing a folder asks for
confirmation first; only folders in the
settings can be opened. Closing the window releases its WebView to save memory while the
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

Overview shows the current file name and its position and total in the active
slideshow cycle. Select the preview to open the image details (location, size,
and dates); for a combined wallpaper, arrows or the Left and Right keys move
through each of its images.
The total includes enabled subfolders and reflects active filters and exclusions.
While the slideshow runs, enabled folders are watched: images that are added,
removed, or renamed update the collection about two seconds after the last
change, without a full rescan, and new images join the current cycle. Files
whose image header cannot be read yet, such as files still being copied, are
picked up by a later change. When the desktop app runs the slideshow, the
Overview total, the **Folders** image counts, and an open **Gallery** refresh
after such a change, and an open window shows how many images were added or
removed (a renamed image counts as one of each). No notice is shown while the
window is closed. Use **Rescan collection** to force a full rescan. Saving settings keeps the current cycle: the wallpaper changes only when
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

**Combine images to fill the screen** (Settings, off by default) has two
layouts, chosen under **Combine layout** (`collage.layout`). **Side by side
(v1)** (`row`) shows a portrait image on a landscape monitor with the next
portrait images of the cycle, side by side at full height; other images are
skipped and stay in the cycle, and the search stops at the first portrait image
that no longer fits. **Mosaic (v2)** (`mosaic`, the default) helps when an
image, fitted alone, would cover less than 80% of a monitor, such as a portrait
or square image on a landscape monitor. The image is shown with other images
from the current cycle in one to three rows, like a mosaic, over a blurred,
darkened copy of it, with an adjustable space between them (0–32 px, default
4). Up to five other images are chosen from the next eight usable images of the
cycle, keeping their order, to cover as much of the monitor as possible; a
combination must cover clearly more (15 points) than the image alone. No image
is enlarged more than twice its size, so small images are shown alone, and
images more than four times as wide as tall (or the other way round) are not
added. Images that are shown together count as shown in the cycle; images that
were not chosen stay in it. The combined image is made at
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
and Pause on Overview and in the tray apply to every display. The slideshow still needs at least one
enabled folder in **Folders**. While any display has its own folders, the
slideshow lists monitors every 10 seconds: a reconnected display gets its image
again and continues its own cycle, and a disconnected display is skipped.
Folders and names of disconnected displays stay saved under **Saved monitor
settings** until you choose **Forget** (after a confirmation, as for removing a
folder), or **Move** them to a connected display
that has no folders of its own, for example when a monitor was moved to another
port and got a new device path. The configuration keys them by the Windows
monitor device path. Profiles do not include per-monitor folders or names.
Overview shows one card for the main slideshow while any connected display
shares it, and one card for each connected display with its own folders. Each
card shows the whole image over a blurred copy of it, the file name, and the
position in its cycle. One card fills the row, two cards share it, and more
cards use three columns. For example, with one display that has its own
folders, only that display's card is shown. Pointing at a display's card
shows Previous and Next buttons that change only that display; the shared
interval timer is not restarted. Displays that share the main slideshow, and
every display in the Span fit mode, change only together. Selecting a card
opens the details of its image.
Windows may retain unavailable display records, such as displays connected
earlier or turned off; Monitors skips records whose dimensions cannot be read
(`0x80004005`) while showing the displays that are available. These skips are
not written to the log, because they are normal and repeat on every listing.
If the history file is invalid, it is renamed to
`history.json.invalid-<timestamp>` and a new history starts. Exclusions match
paths without regard to letter case or `/` and `\` separators.

Local caches are limited: at startup the app keeps at most 20,000 thumbnails
(512 MB), and the Editor keeps the 20 most recent edited copies (1 GB) plus any
copy listed in History. Overview cards use JPEG previews of at most 1600×900
pixels, kept in the `previews` cache (the 100 most recent, at most 200 MB), so
an image that was shown before appears at once. Caches are stored under
`%LOCALAPPDATA%\MindsWall\cache`. The log records how long each wallpaper
change takes (`wallpaper change timing`, with `combine_ms` and `total_ms`;
`wallpaper changed`, with `set_ms` and `history_ms`; and, while images are
combined, `combined wallpaper prepared`, with `monitor_ms`, `sizes_ms`,
`plan_ms`, and `render_ms`) and each preview (`preview created` or `preview
read from cache`). Scanning folders logs `collection scanned` (images,
`scan_ms`, `filter_ms`), and saving settings logs `settings reloaded`
(`scan_ms`, `apply_ms`). A scan resolves each folder once instead of opening
every image, so adding a large folder stays quick; the same image reached
through another spelling of a folder, a nested folder, or a link to a folder is
still listed once.
While the window is minimized, the app stops polling the slideshow status.

Folders, images, and backup files are chosen through native dialogs opened by
the app. The desktop app only reads or writes paths chosen this way during the
current session, or paths that are already part of the collection, history,
or configuration.

These utilities remain in development. Overview updates when the app's
slideshow changes. A slideshow run from a CLI build without hand-over support
is checked every 30 seconds while the window is open. Multi-monitor behavior and the gallery with a
large collection still need interactive Windows verification.

**About** shows the app version, how it treats your data, and buttons that open
the source code, releases, issue tracker, and license on GitHub in your
browser; only these fixed project addresses can be opened, and nothing is sent
from the app. **Profiles** saves named collections and can import or export settings and
profiles as JSON. Profile names have 1–50 characters (names up to 80
characters saved by earlier versions still load). A profile stores folders, interval, shuffle, filters,
exclusions, and fit mode. The active profile is shown at the top of every page
and checked in the tray **Profiles** submenu. When those settings change after
a profile was applied, it is marked **changed**: **Update profile** saves the
current settings into it, and **Revert** applies its saved settings again. The **Editor** creates a cached copy with optional crop,
resize, blur, and dim; the original image remains untouched. The chosen image
and its size appear as soon as it is picked, the edited preview is shown next
to it at the same scale with both sizes, and **Clear** starts over with default
settings. **Keep aspect ratio** (on by default) fills in the other side when
width or height changes, using the crop size when cropping; with it off, a
warning shows when the new size stretches the image. The Editor shows
a spinner on **Create preview** (and dims the previous edit) while creating a
preview and keeps its work when switching
pages during the same app session. Global shortcuts
can be recorded using the keyboard in **Settings**, and saved profiles are available from the
system tray. These features still need interactive Windows verification. A
running slideshow may replace an edited wallpaper at its next change.

### Installing a release build (Windows SmartScreen)

No release has been published yet. Release builds are planned as a per-user
NSIS installer (`MindsWall_<version>_x64-setup.exe`) and a portable ZIP
(`MindsWall_<version>_x64_portable.zip`), with `SHA256SUMS.txt`. They are not
code-signed, so Windows SmartScreen may show **Windows protected your PC** the
first time you run them. Installing does not need administrator rights; an
elevation (UAC) prompt is not expected.

1. Download the files only from this repository's GitHub Releases page.
2. Check the file against `SHA256SUMS.txt` before running it:

   ```powershell
   Get-FileHash .\MindsWall_<version>_x64-setup.exe -Algorithm SHA256
   ```

   The hash must match the line for that file. If it does not, delete the file
   and do not run it.
3. Run the installer. If SmartScreen appears, select **More info**, check that
   the app name is MindsWall, then select **Run anyway**.
4. For the portable ZIP, unblock it before extracting so the extracted
   executable does not trigger the warning again: right-click the ZIP, select
   **Properties**, tick **Unblock**, and select **OK**. Alternatively run:

   ```powershell
   Unblock-File .\MindsWall_<version>_x64_portable.zip
   ```

   Then extract it to a folder in your user profile and run `MindsWall.exe`.

**Run anyway** is not offered when Smart App Control is on (Windows Security >
App & browser control > Smart App Control), because it blocks unsigned apps.
Turning Smart App Control off cannot be undone without resetting Windows, so
consider building from source instead (see
[Desktop app (development)](#desktop-app-development)). Some antivirus products
may also scan or quarantine new unsigned executables; check the file hash
before allowing it. Code signing is planned to remove these warnings.

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
