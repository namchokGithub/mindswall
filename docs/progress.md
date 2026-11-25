# MindsWall Progress Plan

This document turns [the requirements](requirements/requirement.md) into a
trackable development plan. It records what exists today, what is planned next,
and how each phase is considered done. Update the checkboxes and the status
table as work lands.

Status legend: `[ ]` not started, `[~]` in progress, `[x]` done and verified.

## Current State (2026-11-25)

- The workspace contains `mindswall-core` and the `mindswall` CLI. The CLI
  scans a hard-coded folder using the core scanner and prints image paths.
  Unreadable entries are silently skipped.
- The core uses `walkdir` and has a scanner integration test. Configuration,
  wallpaper switching, CI, and the desktop UI do not exist yet.

## Phase Overview

| Phase | Goal                                   | Priority coverage | Status      |
| ----- | -------------------------------------- | ----------------- | ----------- |
| 0     | Rust fundamentals and project skeleton | Foundation        | In progress |
| 1     | Wallpaper core library and CLI         | P1                | Not started |
| 2     | Slideshow engine and configuration     | P1                | Not started |
| 3     | Tauri desktop app and system tray      | P1, P2 (startup)  | Not started |
| 4     | Wallpaper utilities                    | P2                | Not started |
| 5     | Release hardening and distribution     | Quality, release  | Not started |
| 6     | Extended features                      | P3                | Not started |

Each phase builds on the previous one. Phases 1 and 2 keep all logic in a
GUI-independent Rust core so the CLI and the desktop app share the same code.

## Phase 0 — Rust Fundamentals and Project Skeleton

Goal: be comfortable with the Rust concepts the core needs, and shape the
repository so later phases do not require a large restructure.

- [x] Install Rust and Cargo
- [ ] Learn ownership, borrowing, and references
- [ ] Learn structs, enums, and traits
- [ ] Learn `Result`, `Option`, and error handling with `thiserror`
- [ ] Learn modules, crates, and filesystem APIs
- [x] Convert the repository to the Cargo workspace from D1, with
      `crates/core` and `crates/cli`
- [x] Move the scanner from `main.rs` into a `scanner` module in
      `mindswall-core`

Done when: `cargo check`, `cargo fmt --check`, and `cargo clippy` pass for the
whole workspace, and the scanner lives in its own module in the core crate.
The workspace checks pass; the learning items remain for the developer to complete.

## Phase 1 — Wallpaper Core

Goal: a reusable core that can find images and set one as the desktop
wallpaper, driven from a CLI.

- [ ] **Scanner**: scan multiple roots, optionally recursive; match
      `jpg`/`jpeg`/`png`/`bmp` case-insensitively; keep `PathBuf`s without
      assuming valid Unicode; report unreadable entries instead of dropping
      them silently
- [ ] **Wallpaper service**: define a platform-neutral `WallpaperSetter` trait
      in `wallpaper/mod.rs`; implement it for Windows in `wallpaper/windows.rs`
      using the `windows` crate (`IDesktopWallpaper`, with
      `SystemParametersInfoW` as a fallback); validate the file exists before
      calling the API
- [ ] **Random pick**: choose one random image from the scan result with `rand`
- [ ] **Error model**: one `thiserror` error enum for scan, config, and Windows
      API failures, with paths included in messages
- [ ] **CLI (`clap`)**: `scan <folder>...`, `set <file>`, `random <folder>...`,
      and `--recursive` / `--no-recursive`
- [ ] **Tests**: scanner tests using temporary directories (extensions, case,
      nesting, non-image files, missing folder)

Done when: on Windows 11, `mindswall random <folder>` changes the wallpaper
without administrator rights, scanner tests pass, and the README documents the
CLI.

## Phase 2 — Slideshow Engine

Goal: a background slideshow that rotates wallpapers on a timer, with a shuffle
queue that avoids repeats and persisted settings.

- [ ] **Multiple folders**: config holds a list of folders with per-folder
      enable flag and recursive flag; deduplicate images found through
      overlapping folders
- [ ] **Shuffle queue**: shuffle the full set, play it through without repeats,
      reshuffle when exhausted, and avoid showing the last image of the old
      cycle first in the new one; sequential order when shuffle is off
- [ ] **History cursor**: keep a short in-memory history so `Previous` returns
      to the actual previous image
- [ ] **Scheduler**: interval-based timer (for example 1 min to 24 h) that
      sleeps until the next change instead of polling; supports pause, resume,
      next, and previous; tolerates sleep/resume of the PC
- [ ] **JSON configuration**: `serde` + `serde_json`; stored in the per-user
      config directory (for example `%APPDATA%\MindsWall\config.json`); a
      `version` field for future migrations; atomic writes (temp file +
      rename); fall back to defaults with a logged warning on invalid JSON
- [ ] **`config.example.json`** in the repository root
- [ ] **Logging**: `tracing` with a file log in the per-user data directory
- [ ] **Single instance and IPC (D2)**: `run` refuses to start a second
      slideshow; it serves control requests over a per-user named pipe
- [ ] **CLI**: `run` (foreground slideshow), `next`, `previous`, `pause`,
      `resume`, `status` (sent to the running instance), `config show`
- [ ] **Tests**: shuffle no-repeat guarantee, reshuffle boundary, previous/next
      behavior, scheduler logic with an injectable clock, config round-trip and
      invalid-file handling

Done when: a slideshow runs for multiple cycles with no repeat inside a cycle,
settings survive restarts, and unit tests cover shuffle, scheduler, and config.

## Phase 3 — Desktop App

Goal: a Tauri 2 + React + TypeScript app that wraps the core, with tray control
and Windows startup.

- [ ] **Setup**: Tauri 2 project, React + TypeScript (Vite), shadcn/ui; the
      Tauri backend depends on the core crate rather than duplicating logic
- [ ] **Command layer**: Tauri commands for status, next/previous/pause,
      folder CRUD, and settings; events from core to UI for wallpaper changes
- [ ] **Overview dashboard**: current wallpaper preview, Previous / Next /
      Pause controls, interval, shuffle, include subfolders, prevent repeats
- [ ] **Folder management**: add via native folder picker, remove, enable or
      disable, recursive toggle, image count per folder, rescan
- [ ] **Settings page**: general options, start with Windows, log location
- [ ] **System tray**: icon with Next, Previous, Pause/Resume, Open, Quit;
      closing the window hides to tray; slideshow keeps running in background
- [ ] **Start with Windows**: per-user registration (`HKCU\...\Run` or the
      Tauri autostart plugin) so no administrator rights are needed
- [ ] **Single instance (D2)**: a second launch focuses the existing window;
      the app takes over the named pipe server so CLI control commands work
      while it runs
- [ ] **Resource check**: destroy or suspend the WebView when hidden so the
      background process can approach the RAM target

Done when: the app runs from the tray on Windows 11, all P1 controls work from
both the UI and the tray, settings persist, and startup registration works for
a standard user.

## Phase 4 — Wallpaper Utilities

Goal: complete the P2 feature set.

- [ ] **Multi-monitor**: enumerate monitors via `IDesktopWallpaper`; same image
      on all monitors or a separate image (and optionally separate folders) per
      monitor; handle monitors being connected or disconnected; Monitors page
- [ ] **Fit mode (D3)**: Fill, Fit, Stretch, Center, Tile, and Span setting
- [ ] **Image gallery**: grid of discovered images with cached, downscaled
      thumbnails (`image` crate, stored in the local cache directory); lazy
      loading or virtualized list for large libraries; set-as-wallpaper and
      exclude actions
- [ ] **Folder watcher**: `notify` with debouncing; update the image set and
      shuffle queue on add/remove/rename without a full rescan; skip images
      still being written
- [ ] **Resolution / aspect ratio filtering**: read image dimensions from
      headers only; filter by minimum resolution and orientation or aspect
      ratio tolerance; cache dimensions to avoid re-reading files
- [ ] **Wallpaper history**: persisted, size-limited list of shown images with
      timestamps and monitor; History page with re-apply action; handle files
      that no longer exist
- [ ] **Tests**: filter rules, history limits, watcher event-to-queue mapping

Done when: each P2 feature works on Windows 11 (multi-monitor verified on a
real multi-monitor setup), and gallery browsing stays responsive with a large
folder (for example 5,000+ images).

## Phase 5 — Release and Optimization

Goal: measured performance, robust error recovery, and repeatable releases.

- [ ] **Benchmarks**: measure idle CPU and background RAM with the window
      hidden and with it open; record method and results in `docs/`
- [ ] **Optimization**: fix whatever the measurements show exceeds the targets
- [ ] **Test coverage**: fill gaps in core unit tests; add integration tests
      for CLI commands
- [ ] **Error recovery**: missing or offline folders, deleted images, corrupt
      images, corrupt config, Windows API failures; the slideshow skips bad
      items and keeps running
- [ ] **Packaging (D4)**: per-user NSIS installer through the Tauri bundler
      and a portable ZIP; neither needs administrator rights
- [ ] **CI**: GitHub Actions on Windows for `fmt`, `clippy`, `test`, frontend
      lint and type-check, and build
- [ ] **Release workflow**: tag-triggered build that produces release
      artifacts; changelog; version bump process
- [ ] **Docs**: README with installation, usage, and screenshots

Done when: performance targets are measured and met (or the gap is documented),
CI passes on Windows, and a tagged build produces both portable and installer
artifacts.

## Phase 6 — Extended Features (P3)

Planned after the first release; order may change based on feedback.

- [ ] **Wallpaper profiles**: named sets of folders and settings with quick
      switching from the UI and tray
- [ ] **Image editor**: crop, resize, blur, and dim applied to a generated copy
      in the cache; never modify original files
- [ ] **Global hotkeys**: configurable shortcuts for next, previous, and pause
      with conflict detection
- [ ] **Import / export**: back up and restore configuration and profiles as
      JSON, with validation on import

## Cross-Cutting Requirements

These apply to every phase:

- Local-first: no network access, telemetry, or update checks without an
  explicit decision.
- No administrator rights: all files and registry entries are per-user.
- Windows-specific code stays in the platform layer; the core stays testable
  on any OS.
- Supported formats JPG, JPEG, PNG, and BMP are preserved.
- Windows behavior is verified on Windows 11, not only by portable tests.
- README is updated whenever implemented behavior or instructions change.

## Performance Targets

| Metric              | Target         | Status       |
| ------------------- | -------------- | ------------ |
| Idle CPU            | < 0.1% average | Not measured |
| Background core RAM | < 30 MB        | Not measured |
| Network usage       | 0              | Not measured |

Targets are reported as achieved only after measurement (Phase 5).

## Decisions

Decided on 2026-11-25. Revisit only with a recorded reason.

### D1. Workspace layout: Cargo workspace with a shared core

The single-crate layout in the requirements does not fit Tauri, which expects
`src-tauri/`. The repository becomes a Cargo workspace so the CLI and the
desktop app share one core:

```text
mindswall/
├── Cargo.toml            # workspace root
├── crates/
│   ├── core/             # mindswall-core: scanner, shuffle, scheduler,
│   │                     # config, history, wallpaper trait + Windows impl
│   └── cli/              # mindswall-cli: `mindswall` binary (clap)
├── src-tauri/            # Tauri 2 app, depends on mindswall-core
├── ui/                   # React + TypeScript frontend (Vite, shadcn/ui)
├── docs/
└── config.example.json
```

- `mindswall-core` has no Tauri or CLI dependencies; Windows API code lives in
  its `wallpaper/windows.rs` behind `cfg(windows)`.
- The conversion happens in Phase 0; Phase 3 only adds `src-tauri/` and `ui/`.

### D2. Process model: one running instance owns the slideshow

- Only one process runs the slideshow at a time: `mindswall run` in Phase 2,
  then the tray app from Phase 3 on. Both enforce a single instance.
- The running instance listens on a Windows named pipe scoped to the current
  user. No TCP sockets, so network usage stays at zero.
- CLI control commands (`next`, `previous`, `pause`, `resume`, `status`) send
  a request over the pipe. With no running instance they report that clearly;
  one-shot commands (`scan`, `set`, `random`) run standalone.
- The IPC protocol is small JSON messages defined in `mindswall-core`, so the
  CLI and the app share the same types.

### D3. Wallpaper fit mode: default Fill, configurable in Phase 4

- Phase 1 to 3 apply wallpapers with the Fill position.
- Phase 4 adds a fit mode setting (Fill, Fit, Stretch, Center, Tile, Span)
  together with multi-monitor work, since both use `IDesktopWallpaper`.

### D4. Installer: per-user NSIS installer plus portable ZIP

- The installer is NSIS through the Tauri bundler, in per-user install mode,
  so no administrator rights are needed. MSI is not shipped, because the Tauri
  MSI (WiX) target installs per machine.
- The portable build is a ZIP with the executable. It stores config in the
  per-user config directory like the installed build.

### D5. Image formats: JPG, JPEG, PNG, and BMP only for v1

- Other formats such as WebP are out of scope until after the first release.
- A new format is added only when the scanner, thumbnail generation, and the
  Windows wallpaper API all support it, with tests.

## Risks

- WebView memory use may exceed the 30 MB background target unless the window
  is destroyed when hidden.
- `IDesktopWallpaper` behavior differs across monitor layouts and Windows
  builds; requires testing on real hardware.
- Very large libraries can make scanning, thumbnailing, and dimension reads
  slow; caching and incremental updates are needed.
- Network drives or removable folders may disappear while the slideshow runs.
