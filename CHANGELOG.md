# Changelog

All notable changes to MindsWall are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/). See
[docs/releasing.md](docs/releasing.md) for the release process.

## [Unreleased]

## [1.0.0] - 2026-11-28

### Added

- MindsWall Setup, a per-user installer and uninstaller with its own window
  (install location, shortcuts, launch after installing, Start with Windows,
  updates, and an option to delete settings when uninstalling). The NSIS
  installer remains as a fallback.
- Portable mode: `portable.txt` next to the programs keeps all files in that
  folder.
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
  scroll bars, custom dropdowns, text selection limited to file names and
  paths, and one spinner for every loading state.
- Overview shows a card for the main slideshow and for each display with its
  own folders (the whole image over a blurred copy, in up to three columns),
  and a display with its own folders can go to its next or previous image
  alone.
- Selecting the Overview preview or a display opens the image details; the
  preview no longer shows the folder path.
- The app shows how many images the folder watcher added or removed.
- Folder paths show only their last folder; selecting a path copies it.
- Folders and Monitors folders can be opened in File Explorer, and removing
  a folder or forgetting a monitor asks for confirmation.
- Monitor thumbnails and Overview cards keep the previous image while the new
  one loads, instead of flashing a placeholder.
- The Editor previews the chosen image with its size, compares it with the
  edited copy at the same scale, keeps the aspect ratio when resizing (or
  warns about stretching), and has a **Clear** button.
- Settings can open the log folder and show the last 100 log lines.
- Overview cards load cached, downscaled previews, and the log records how
  long each wallpaper change and preview takes.
- Image details of a combined wallpaper move through each of its images with
  arrows or the Left and Right keys.
- Adding a folder no longer opens every image to find duplicates, and the
  log records how long folder scans and settings reloads take.
- New profile names are limited to 50 characters.
- An About page shows the version and opens the source code, releases,
  issues, and license on GitHub.
- Thumbnails, previews, and combined wallpapers decode large JPEG images at a
  reduced size, the Gallery builds two thumbnails at a time, and timing log
  lines record memory use.
- Optional combined wallpapers: an image that leaves much of a monitor empty
  (portrait, square, or landscape on a portrait monitor) is shown with other
  images in up to three rows, like a mosaic, over a blurred background, with
  an adjustable gap; small images are never enlarged more than twice.
  **Combine layout** chooses between this mosaic and the original side-by-side
  portrait row.
