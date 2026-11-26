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
