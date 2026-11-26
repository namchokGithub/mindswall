# Changelog

All notable changes to MindsWall are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/). See
[docs/releasing.md](docs/releasing.md) for the release process.

## [Unreleased]

### Added

- Desktop app (Tauri 2) with tray controls, folders, gallery, history,
  monitors, profiles, editor, hotkeys, and start with Windows.
- CLI for scanning, setting wallpapers, running the slideshow, and
  controlling a running slideshow over a per-user named pipe.
- Folder watcher that updates the collection without a full rescan.
- Per-monitor slideshow folders, with connected and disconnected monitors
  noticed while the slideshow runs.
- The desktop app takes over a slideshow started from the CLI.
- Closing the window releases the WebView; "Start with Windows" starts in
  the tray without a window.
- GitHub Actions CI on Windows and a tag-triggered release workflow.
- Settings shows the log file location with a button to copy it.
- Monitors shows each display's name, position, and current wallpaper in a
  to-scale arrangement; displays can be renamed, and saved settings of a
  disconnected display can be moved to another one.
- Overview shows a thumbnail per display while a display has its own folders,
  and History names and filters displays.
- Every page shows the active profile and whether its settings changed;
  profiles can be updated with the current settings or reverted, and the tray
  checks the active profile.
- Light, dark, and system themes, a custom title bar, a fixed sidebar, hidden
  scroll bars, custom dropdowns, and text selection limited to file names and
  paths.
- Overview shows a card for the main slideshow and for each display with its
  own folders (the whole image over a blurred copy, in up to three columns),
  and a display with its own folders can go to its next or previous image
  alone.
- Selecting the Overview preview or a display opens the image details; the
  preview no longer shows the folder path.
- The app shows how many images the folder watcher added or removed.
- Folder paths show only their last folder; selecting a path copies it.
- Optional combined wallpapers: portrait images are shown side by side on a
  landscape monitor over a blurred background, with an adjustable gap.
