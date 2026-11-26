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
- [ ] Fallback keeps the existing Windows fit setting (as recorded in D3)
- [ ] With a fit mode other than Fill, a desktop API failure reports an error
      instead of using the fallback

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
- [ ] After the 2026-11-26 logging change, each successful change adds a
      `wallpaper changed` line with the image path

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

- [ ] Delete the next image while the slideshow runs: the failure is logged,
      the image is skipped, and the next change shows another image
- [ ] Delete several images, then press `next` repeatedly: every press shows
      an available image
- [ ] Disconnect a network or removable folder: record behavior
- [ ] Add a folder that is missing or offline next to a working folder: the
      slideshow starts with the working folder, and the log shows
      `skipping unreadable location`
- [ ] Add a drive root such as `D:\`: protected folders are skipped and the
      remaining images are used
- [ ] Write invalid JSON to the config file and start the app: the file is
      renamed to `config.json.invalid-<timestamp>`, the app shows a warning
      with that path, and saving a setting does not change the renamed file

### Settings changes during a slideshow

- [ ] Change the interval, shuffle, or a filter that keeps the current image:
      the wallpaper does not change and images shown in this cycle do not
      repeat before the cycle ends
- [ ] Change the fit mode: the current image is applied again with the new
      mode
- [ ] Exclude the current image in Gallery: the wallpaper changes to another
      image
- [ ] While a large folder is rescanned or settings are saved, the window
      stays responsive (it can be moved and pages can be switched)

### Control pipe, caches, and history

- [ ] Build the CLI and the app with different Rust toolchains (or use a
      packaged app with a CLI built later): CLI control commands still reach
      the app
- [ ] On a machine with two accounts, start the app as user B first, then as
      user A: user A's CLI reports
      `the slideshow control pipe is owned by another user` instead of
      controlling user B's slideshow
- [ ] Close the window to the tray: within a few seconds no `msedgewebview2.exe`
      process belonging to MindsWall remains in Task Manager (Details tab,
      check the command line or parent process)
- [ ] Reopen the window from the tray: status and preview load immediately
- [ ] Create more than 20 Editor previews: only the 20 newest stay in
      `%LOCALAPPDATA%\MindsWall\cache\edits`, plus any edited copy in History
- [ ] Write invalid JSON to `history.json`: History shows an empty list, the
      file is renamed to `history.json.invalid-<timestamp>`, and the next
      wallpaper change is recorded
- [ ] Exclude an image, then edit `config.json` so the exclusion uses a
      different letter case: the image stays excluded

### Folder watcher and slideshow events

- [ ] Copy new images into a watched folder while the slideshow runs: within
      a few seconds the Overview total increases, and the log shows
      `collection updated from folder changes`
- [ ] Copy a large image slowly (or over the network): it is added after the
      copy finishes and never applied while incomplete
- [ ] Delete or rename an image in a watched folder: the total updates; if it
      was the current wallpaper, another image is applied
- [ ] Add or remove a subfolder with images in a recursive folder; repeat
      with a non-recursive folder, whose subfolders must be ignored
- [ ] Images in a folder added through **Folders** are watched after saving
- [ ] With the app running the slideshow, Overview updates immediately after
      each automatic change, without waiting for a poll
- [ ] With the slideshow run from the CLI, Overview still updates within 30
      seconds while the window is open

### File access from the desktop app

- [ ] **Add folder**, Editor **Choose image**, Monitors image selection,
      backup export, and backup import each open a native dialog
- [ ] Cancelling a backup export or import shows no success message and
      changes nothing
- [ ] Backup export writes the chosen file; import restores it
- [ ] Folders saved in a profile can still be counted and used after an app
      restart

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

- [ ] Closing the window closes it while the app stays in the tray; the
      slideshow keeps running (the earlier pass hid the window instead)
- [ ] Tray **Open MindsWall** opens a new window and focuses it; choosing it
      twice quickly opens only one window
- [ ] Minimize the window, then choose tray **Open MindsWall**: the window is
      restored
- [ ] Open a page, close the window, and reopen it: the app starts on
      Overview and no error is shown
- [x] Tray **Previous**, **Next**, **Pause**, and **Resume** work while the
      window is hidden
- [x] Tray **Quit** exits the process (check Task Manager) and stops the
      slideshow

### Single instance and CLI interplay

- [x] Launching the app a second time focuses the existing window; no second
      process remains
- [ ] With the window closed, launching the app again opens a window
- [x] While the app runs, CLI `status`, `next`, `previous`, `pause`, `resume`
      control the app's slideshow
- [x] While the app runs, CLI `run` refuses to start
- [x] Start CLI `run` first, then the app: record behavior (recorded before
      the handoff was implemented)
- [ ] Start CLI `run` first, then the app: the CLI prints
      `Slideshow handed over to the MindsWall app.` and exits, the wallpaper
      does not change at the handover, and Overview shows the same image
- [ ] Pause CLI `run`, then start the app: the app's slideshow stays paused
- [ ] After the handover, CLI `status`, `next`, and `pause` control the app,
      and Overview updates without waiting for the 30-second poll

### Start with Windows

- [x] Enabling **Start with Windows** as a standard user succeeds without an
      elevation prompt
- [x] An entry appears under
      `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (or Task Manager >
      Startup apps)
- [x] Sign out and sign in: the app starts (record whether it starts hidden or
      shown, and whether the slideshow resumes)
- [x] Disabling it removes the entry; the app no longer starts at sign-in
- [ ] Turn **Start with Windows** off and on, then check the `Run` entry ends
      with `--hidden`
- [ ] Sign out and sign in: the app starts in the tray without opening a
      window, and the slideshow runs

### Resources (observation only)

- [x] Record RAM of all MindsWall and WebView2 processes with the window open
      and hidden (target < 30 MB is expected to fail until WebView suspension
      exists): 9-10MB
- [ ] Run `scripts/measure-performance.ps1` with the window open and again
      with it closed to the tray; record the totals in `docs/performance.md`

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
- [ ] After more than 10 changes, only the latest 10 are kept, and they are
      still kept after the next wallpaper change
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
- [x] Reconnect it: record behavior (recorded before automatic handling was
      implemented)
- [ ] **Add folder** on one monitor: within one interval that monitor shows
      images from its folders while the others keep the main slideshow
- [ ] **Next**, **Previous**, and **Pause** affect every monitor
- [ ] Disconnect a monitor that has its own folders: within about 10 seconds
      the log records `connected monitors changed`; no crash
- [ ] Reconnect it: within about 10 seconds it shows its own image again and
      continues its cycle
- [ ] Disconnect a monitor that shows the main slideshow, then reconnect it:
      it shows the main slideshow's current image
- [ ] A disconnected monitor appears under **Saved monitor folders**;
      **Forget** removes it from `monitor_folders`
- [ ] Remove the last folder of a monitor: it returns to the main slideshow
- [ ] Add, delete, and rename images in a monitor's folder: that monitor's
      collection follows the change, as for the main folders
- [ ] With only one monitor and no per-monitor folders, the log shows no
      monitor checks and wallpapers still change for the whole desktop
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
- [ ] Switch to another page while a preview is processing, then return before
      and after it finishes: the edit settings and preview remain available
- [ ] A preview failure while on another page is reported after returning to
      Editor without showing a notification on the other page
- [ ] Crop, resize, blur (0–20), and dim (0–90) each produce the expected result
- [ ] The edited copy is written to the cache; the original file is unchanged
      (compare size and modified time)
- [ ] Applying the edited image sets it as wallpaper
- [ ] Out-of-range crop or dimensions show an error
- [ ] A running slideshow later replaces the edited wallpaper (known behavior)

### Global hotkeys

- [ ] Record a shortcut by pressing its keys, save it, and verify it works
- [ ] Escape cancels recording, Clear disables a shortcut, and duplicate or
      unavailable shortcuts report an error
Hotkeys are empty by default; configure them in **Settings**.

- [x] Next, Previous, and Pause hotkeys work while another app has focus
- [x] The Pause hotkey toggles between pause and resume
- [x] Hotkeys work while the window is hidden in the tray
- [x] Assigning the same shortcut to two actions is rejected
- [x] An invalid shortcut string is rejected
- [x] A shortcut already used by another app reports `Could not register shortcuts` or a clear conflict message
- [x] Hotkeys still work after an app restart

### Release workflow

- [ ] Push a tag such as `v0.1.0-test1` on a fork or test repository: the
      version check fails because the tag differs from the manifest versions,
      and no draft release is made
- [ ] Push a matching `v<version>` tag: the workflow passes, and the draft
      release has the NSIS installer, the portable ZIP, and `SHA256SUMS.txt`
- [ ] The portable ZIP runs from a user folder as a standard user and uses the
      same settings as the installed build

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
