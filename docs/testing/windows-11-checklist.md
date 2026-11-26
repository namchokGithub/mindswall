# Windows 11 Manual Test Checklist

Manual verification for the features that `docs/progress.md` marks `[~]`
because they need an interactive Windows 11 session. Automated tests do not
replace these checks.

## How to use

- Copy this file for each test run, or record results in a table at the end.
- Mark each check `[x]` pass, `[!]` fail (add a note and log excerpt), or `[-]`
  skipped (add the reason).
- Run the checks in order; later sections assume earlier ones pass.
- Move an item in `docs/progress.md` to the Completed Log only after every
  check for it passes.

## Test environment

Record these before starting:

- [x] Windows 11 edition and build (`winver`):

Edition Windows 11 Home
Version 26H2
Installed on ‎16/‎11/‎2569
OS build 26300.9974
Experience Windows Feature Experience Pack 1000.26100.385.0

- [x] Account type: administrator
- [x] Monitors (count, resolution, scaling, arrangement): 1 monitor, 1920×1080, 100% scaling
- [x] Commit hash under test (`git rev-parse --short HEAD`): 2bf16d4
- [x] Build: `cargo run` / `tauri dev` / packaged build: npm.cmd --prefix ui run tauri dev

Prepare test data:

- [x] Folder A with at least 10 images mixing `.jpg`, `.jpeg`, `.png`, `.bmp`,
      including upper-case extensions such as `.JPG`
- [x] A subfolder inside Folder A with at least 3 images
- [x] Folder B with at least 5 images, plus one image also present in Folder A
      through an overlapping path (for deduplication)
- [x] At least one image with a non-ASCII file name (for example Thai text)
- [x] One non-image file (`.txt`) and one corrupt `.jpg` (a renamed text file)
- [x] Portrait, landscape, and square images in several resolutions (filters)
- [ ] A folder with 5,000+ images (gallery scale)

Useful paths:

- Config: `%APPDATA%\MindsWall\config\config.json`
- Log: `%APPDATA%\MindsWall\data\mindswall.log`
- History: `%APPDATA%\MindsWall\data\history.json`
- Cache: `%LOCALAPPDATA%\MindsWall\cache`

## 1. CLI one-shot commands (Phase 1)

Run as a standard user from the repository root.

- [x] `cargo run -p mindswall-cli -- --help` lists all commands

  ```
  Usage: mindswall.exe <COMMAND>

  Commands:
  scan
  random
  set
  run
  next
  previous
  pause
  resume
  status
  config
  help Print this message or the help of the given subcommand(s)

  Options:
  -h, --help Print help
  -V, --version Print version
  ```

- [x] `scan "<Folder A>"` lists all images, including the subfolder and upper-case
      extensions; excludes `.txt`
- [x] `scan --no-recursive "<Folder A>"` excludes the subfolder images
- [x] `scan "<Folder A>" "<Folder B>"` lists images from both folders
- [x] `scan` on a missing folder prints an error to stderr with the path and a
      non-zero exit code
- [x] Non-ASCII file names appear correctly in `scan` output
- [x] `set "<image>"` changes the desktop wallpaper without an elevation prompt
- [x] `set` with a non-ASCII file name changes the wallpaper
- [x] `set` on a missing file reports an error and leaves the wallpaper unchanged
- [x] `set` on the corrupt `.jpg` reports an error or leaves the desktop in a
      usable state (record which)
- [x] `random "<Folder A>"` changes the wallpaper and prints the chosen path
- [x] Running `random` several times picks different images

### Fallback path

The fallback (`SystemParametersInfoW`) runs only when `IDesktopWallpaper` fails.
Trigger it if possible (for example in a session where the COM API is
unavailable), otherwise mark it skipped.

- [ ] Fallback changes the wallpaper
- [ ] Fallback keeps the existing fit setting (current behavior; D3 says Fill —
      see the open decision in `docs/progress.md`)

## 2. CLI slideshow (Phase 2)

Set `interval_seconds` to `60` in the config for faster checks.

### Start and configuration

- [x] Copy `config.example.json` to the config path, edit folders, then
      `cargo run -p mindswall-cli -- run` starts and changes the wallpaper
- [x] `config show` prints the loaded configuration
- [x] A disabled folder contributes no images
- [x] `recursive: false` excludes subfolder images
- [x] An image reachable through overlapping folders appears only once per cycle
- [x] Invalid JSON in the config: `run` starts with defaults and the log shows
      `invalid configuration; using defaults`
- [x] `interval_seconds` outside 60–86400 is rejected or handled (record which)
- [x] The log file is created and records wallpaper changes and errors

### Single instance and control pipe

- [x] A second `run` in another terminal refuses to start with
      `cannot start slideshow control listener (another instance may be running)`
- [x] `status` reports the current state and image
- [x] `next` changes the wallpaper immediately
- [x] `previous` returns to the actual previous image
- [x] `previous` repeatedly walks back through history, then `next` moves
      forward again
- [x] `pause` stops changes; no change happens after the interval passes
- [x] `status` shows the paused state
- [x] `resume` restarts changes at the interval
- [x] With no slideshow running, `status`/`next` report
      `slideshow is not running or cannot be reached`
- [x] Ctrl+C stops the slideshow; a new `run` starts afterwards

### Shuffle and cycles

Use Folder B (5+ images) so a cycle is short. Note each image shown, for
example from the log.

- [x] Shuffle on: no image repeats until every image has been shown once
- [x] Shuffle on: run at least 3 full cycles; the first image of a new cycle is
      never the last image of the previous cycle
- [x] Shuffle off: images appear in sequential order and wrap around
- [x] Config changes take effect after restarting `run`

### Timing and power

- [x] Changes happen at the configured interval (±5 seconds) over 10 minutes
- [x] Sleep the PC for at least 2 minutes and wake it: the slideshow keeps
      running and changes at the next due time without a burst of changes
- [x] Lock and unlock the session: the slideshow keeps running
- [x] Task Manager shows near-zero CPU between changes (note values; formal
      benchmarks are Phase 5)

### Errors

- [ ] Delete the next image while the slideshow runs: the failure is logged and
      the slideshow keeps running (record whether it skips or stops changing)
- [ ] Disconnect a network or removable folder: record behavior

## 3. Desktop app (Phase 3)

Start with `npm.cmd --prefix ui run tauri dev` (stop any CLI `run` first).

### Window and pages

- [x] The app opens and every page loads: Overview, Folders, Gallery, Monitors,
      History, Profiles, Editor, Settings
- [x] Logo and tray icon display correctly

### Folders

- [x] **Add folder** opens the native picker and adds the folder
- [x] The image count appears and matches `scan`
- [x] **Include subfolders** changes the count
- [x] Disabling a folder excludes its images from the slideshow
- [ ] Disabling the last enabled folder applies the Windows default image and
      stops slideshow changes
- [ ] Enabling a folder again resumes a previously running slideshow and keeps
      a previously paused slideshow paused
- [ ] With the slideshow stopped, disabling the last enabled folder applies
      the Windows default image
- [x] Removing a folder updates the config file
- [x] Non-ASCII folder paths display and work

### Overview and settings

- [x] **Start slideshow** starts it; status shows Playing
- [x] Previous, Pause/Resume, and Next work; status updates
- [x] Changing **Change picture every** takes effect (record whether a restart
      is needed)
- [x] Toggling **Shuffle wallpapers** takes effect
- [x] Settings persist after quitting and reopening the app

### Tray

- [x] Closing the window hides it to the tray; the slideshow keeps running
- [x] Tray **Open MindsWall** shows and focuses the window
- [x] Tray **Previous**, **Next**, **Pause**, and **Resume** work while the
      window is hidden
- [x] Tray **Quit** exits the process (check Task Manager) and stops the
      slideshow

### Single instance and CLI interplay

- [x] Launching the app a second time focuses the existing window; no second
      process remains
- [x] While the app runs, CLI `status`, `next`, `previous`, `pause`, `resume`
      control the app's slideshow
- [x] While the app runs, CLI `run` refuses to start
- [x] Start CLI `run` first, then the app: record behavior (handoff is not yet
      implemented)

### Start with Windows

- [x] Enabling **Start with Windows** as a standard user succeeds without an
      elevation prompt
- [x] An entry appears under
      `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (or Task Manager >
      Startup apps)
- [x] Sign out and sign in: the app starts (record whether it starts hidden or
      shown, and whether the slideshow resumes)
- [x] Disabling it removes the entry; the app no longer starts at sign-in

### Resources (observation only)

- [x] Record RAM of all MindsWall and WebView2 processes with the window open
      and hidden (target < 30 MB is expected to fail until WebView suspension
      exists): 9-10MB

## 4. Wallpaper utilities (Phase 4)

### Fit mode

For each mode, apply a landscape and a portrait image and confirm the desktop
matches the Windows **Personalize > Background** setting of the same name.

- [x] Fill
- [x] Fit
- [x] Stretch
- [x] Center
- [x] Tile
- [x] Span (multi-monitor only)

### Filters

- [x] **Orientation** Landscape/Portrait/Square limits the slideshow and gallery
- [x] **Aspect ratio** 16:9, 16:10, 4:3, 1:1 includes images within 5% only
- [x] **Minimum resolution** excludes smaller images; `0` accepts any size
- [x] Invalid input (negative or above 100000) shows the validation message
- [x] A filter that matches no images shows a clear message instead of failing
      silently

### Gallery

- [x] Thumbnails load in pages; the cache folder fills with `.png` thumbnails
- [x] Reopening the gallery reuses cached thumbnails (faster second load)
- [x] **Set** applies the image as wallpaper
- [x] **Exclude** removes the image from the slideshow and gallery
- [x] Excluded images can be restored in **Settings**
- [x] The corrupt `.jpg` does not break the gallery
- [ ] With 5,000+ images: the first page appears quickly and scrolling stays
      responsive (record load time and RAM)

### History

- [x] Each applied wallpaper appears with a timestamp
- [x] Re-apply sets the image again
- [x] A deleted image is shown as missing or handled without a crash
- [x] After more than 100 changes, only the latest 100 are kept
- [ ] With the new limit, only the latest 10 entries appear and are retained
      after the next wallpaper change
- [ ] History thumbnails appear for available images; missing images show a
      placeholder without affecting other entries
- [x] History survives an app restart

### Monitors (needs two or more displays)

- [ ] Open **Monitors** after disconnecting a previously used display: available
      displays still load if Windows retains an inaccessible display record
- [x] **Monitors** lists every connected display
- [x] Applying an image to one monitor changes only that monitor
- [x] A running slideshow later replaces the per-monitor image (known behavior)
- [x] Disconnect a monitor while the app runs: no crash; record the list and
      wallpaper behavior
- [x] Reconnect it: record behavior (automatic handling is not yet implemented)
- [x] Mixed scaling (for example 100% and 150%): wallpapers render correctly

## 5. Extended features (Phase 6)

### Profiles

- [ ] Save the current folders and settings as a named profile
- [ ] Switching profiles in the app changes folders and settings
- [ ] The tray **Profiles** submenu lists profiles and switches them
- [ ] The tray submenu updates after adding or deleting a profile
- [ ] Profiles persist after restart

### Import and export

- [ ] Export writes a JSON file through the native save dialog
- [ ] Import of that file restores settings and profiles
- [ ] Import of invalid JSON or out-of-range values is rejected with a message
      and leaves the current config unchanged

### Editor

- [ ] Creating a preview shows a loading state and prevents changing edit
      settings until the preview succeeds or fails
- [ ] Crop, resize, blur (0–20), and dim (0–90) each produce the expected result
- [ ] The edited copy is written to the cache; the original file is unchanged
      (compare size and modified time)
- [ ] Applying the edited image sets it as wallpaper
- [ ] Out-of-range crop or dimensions show an error
- [ ] A running slideshow later replaces the edited wallpaper (known behavior)

### Global hotkeys

Hotkeys are empty by default; configure them in **Settings**.

- [x] Next, Previous, and Pause hotkeys work while another app has focus
- [x] The Pause hotkey toggles between pause and resume
- [x] Hotkeys work while the window is hidden in the tray
- [x] Assigning the same shortcut to two actions is rejected
- [x] An invalid shortcut string is rejected
- [x] A shortcut already used by another app reports `Could not register shortcuts` or a clear conflict message
- [x] Hotkeys still work after an app restart

## 6. Local-first checks

- [ ] With the network adapter disabled, every check above that passed still
      passes
- [ ] Resource Monitor > Network shows no MindsWall traffic during a slideshow
- [ ] No elevation prompt appeared during any check

## Results

| Section         | Pass | Fail | Skipped | Notes |
| --------------- | ---- | ---- | ------- | ----- |
| 1 CLI one-shot  |      |      |         |       |
| 2 CLI slideshow |      |      |         |       |
| 3 Desktop app   |      |      |         |       |
| 4 Utilities     |      |      |         |       |
| 5 Extended      |      |      |         |       |
| 6 Local-first   |      |      |         |       |
