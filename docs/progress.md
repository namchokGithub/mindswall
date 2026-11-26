# MindsWall Progress Plan

This document turns [the requirements](requirements/requirement.md) into a
trackable development plan. It records what exists today, what is planned next,
and how each phase is considered done. Update the checkboxes and the status
table as work lands.

Status legend: `[ ]` not started, `[~]` in progress, `[x]` done and verified.
Completed items move from the phase checklists to the
[Completed Log](#completed-log) at the end of this document.

## Current State (2026-11-26)

- The CLI provides `scan`, `set`, and `random` commands using the shared core.
  Scanning supports multiple folders and reports unreadable entries.
- The slideshow engine, JSON configuration, control commands and Windows named
  pipe are implemented. A Tauri desktop app is under development with the
  Phase 4 utilities (gallery, filters, fit modes, history, monitors) and the
  Phase 6 extended features (profiles, editor, hotkeys, import/export).
- The [Windows 11 manual checklist](testing/windows-11-checklist.md) records
  passing checks for core CLI, slideshow, desktop, utilities, and hotkeys at
  commit `2bf16d4`. Some checks remain incomplete; CI and release workflows
  exist but have not run yet.
- Commits `ca0a272`, `f8d2046`, and `f4f7d30` fixed slideshow recovery, IPC,
  cache, and file-access issues found in a code review. Their automated tests
  pass, but the matching checklist sections have not been run on Windows yet;
  see [Recent changes awaiting verification](#recent-changes-awaiting-verification).

## Remaining Work (2026-11-26)

### Gaps

- **Windows 11 verification**: checked items in the manual checklist are
  recorded as passed. The checklist sections added for the recent changes
  (Errors, Settings changes during a slideshow, Control pipe, caches, and
  history, and File access from the desktop app) are still open. Fallback,
  offline operation, the 5,000-image gallery check, and other unchecked cases
  remain open. Reorganizing overlapping checklist sections, including the
  environment summary, is deferred until the user asks for it.
- **Performance notes**: the checklist reports 9–10 MB RAM without a process or
  open/hidden breakdown, so it does not establish the performance target.
- **Logging**: the slideshow service now logs successful wallpaper changes as
  well as failures. Re-check the log file on Windows, because the earlier
  checklist pass was recorded before successful changes were logged.
- **Implemented, not verified on Windows** (2026-11-26, not committed yet):
  closing the window releases the WebView and autostart opens only the tray;
  the app takes over a slideshow started from the CLI; per-monitor slideshow
  folders with monitor connect and disconnect handling; the tag-triggered
  release workflow. See Recent changes awaiting verification.
- **Phase 5 partly started**: error recovery is implemented in code but not
  verified on Windows. CI and the release workflow exist but have not run;
  benchmarks have a method and no results; the installer and portable ZIP have
  not been produced.

### Recent changes awaiting verification

Implemented on 2026-11-26 with automated tests on macOS and a Windows-target
compile check. Each needs the matching manual checks before it moves to the
Completed Log.

- **Slideshow recovery** (`ca0a272`): images that cannot be applied are
  skipped and removed until the next reload; unreadable or offline folders and
  subfolders are logged and skipped; an invalid config file is renamed to
  `config.json.invalid-<timestamp>` before defaults are used, and the app
  shows a warning.
- **Settings changes keep the cycle** (`ca0a272`): saving settings updates the
  collection without restarting the shuffle cycle; the wallpaper changes only
  when the current image leaves the collection or the fit mode changes.
- **Responsive desktop commands** (`ca0a272`): slow Tauri commands run off the
  main thread, and slideshow IPC requests time out after 30 seconds.
- **Control pipe** (`f8d2046`): the pipe name uses a stable FNV-1a hash, and
  clients refuse a pipe server that runs as another user.
- **Resource use** (`f8d2046`): thumbnails and previews are sent as raw bytes;
  status polling stops while the window is hidden; the edited-image cache keeps
  20 copies (1 GB) plus copies in History, and the thumbnail cache is limited
  to 20,000 files (512 MB) at startup.
- **History and exclusions** (`f8d2046`): an invalid `history.json` is renamed
  aside and a new history starts; exclusions use a hash set and, on Windows,
  ignore letter case and `/` versus `\`.
- **Folder watcher and slideshow events** (2026-11-26, not committed yet):
  enabled folders are watched while the slideshow runs and changes update the
  collection without a full rescan; the app's slideshow sends a
  `slideshow-changed` event to the UI, and fallback polling is every 30
  seconds. The first Windows pass found that Folders counts and Gallery did
  not refresh after a watcher change; a `collection-changed` event now
  refreshes them (re-check on Windows).
- **Window lifecycle** (2026-11-26, not committed yet): closing the window
  destroys it and its WebView; the tray, slideshow, and shortcuts keep running,
  and **Open MindsWall** creates a new window. "Start with Windows" registers
  `--hidden`, so the app starts in the tray without a window.
- **CLI handoff** (2026-11-26, not committed yet): a new `handoff` pipe command
  asks a slideshow started with `mindswall run` to stop. The app then takes the
  pipe (retrying for up to 5 seconds) and continues from the same image and
  paused state. The app refuses `handoff` for its own slideshow.
- **Per-monitor slideshows** (2026-11-26, not committed yet):
  `monitor_folders` in the config gives a monitor its own folders and
  slideshow; other monitors share the main slideshow, applied per monitor while
  any monitor has its own. While configured, monitors are listed every 10
  seconds; a reconnected monitor gets its image again and keeps its cycle. The
  Monitors page adds and removes folders per monitor.
- **Release workflow** (2026-11-26, not committed yet): pushing a `v*.*.*` tag
  runs the CI checks, verifies the manifest versions and `CHANGELOG.md`, builds
  the NSIS installer and a portable ZIP with checksums, and creates a draft
  GitHub Release. The process is in [releasing.md](releasing.md).
- **File access** (`f4f7d30`): file and folder dialogs open from Rust, and
  commands accept only paths chosen in this session or already managed by the
  app.

### Next steps (in order)

1. Complete the outstanding Windows 11 checks in the
   [manual test checklist](testing/windows-11-checklist.md), starting with the
   sections for the recent changes.
2. Push the branch so the new Windows CI runs, and fix anything it reports.
   Push a test tag on a fork or test repository to try the release workflow.
3. Finish the remaining Phase 3 work.
4. Verify the folder watcher, slideshow events, window lifecycle, CLI
   handoff, and per-monitor slideshows on Windows (per-monitor checks need two
   or more displays).
5. Measure performance with `scripts/measure-performance.ps1`, then optimize.
6. Produce the first installer and portable ZIP through the release workflow
   when the user decides to release.

Phase 6 development began before release at the user's request. Continue to
track its untested features alongside the remaining P1 and P2 work.

## Phase Overview

| Phase | Goal                                   | Priority coverage | Status      |
| ----- | -------------------------------------- | ----------------- | ----------- |
| 0     | Rust fundamentals and project skeleton | Foundation        | Complete    |
| 1     | Wallpaper core library and CLI         | P1                | In progress |
| 2     | Slideshow engine and configuration     | P1                | In progress |
| 3     | Tauri desktop app and system tray      | P1, P2 (startup)  | In progress |
| 4     | Wallpaper utilities                    | P2                | In progress |
| 5     | Release hardening and distribution     | Quality, release  | In progress |
| 6     | Extended features                      | P3                | In progress |

Each phase builds on the previous one. Phases 1 and 2 keep all logic in a
GUI-independent Rust core so the CLI and the desktop app share the same code.

## Phase 0 — Rust Fundamentals and Project Skeleton

Goal: be comfortable with the Rust concepts the core needs, and shape the
repository so later phases do not require a large restructure.

Done when: `cargo check`, `cargo fmt --check`, and `cargo clippy` pass for the
whole workspace, and the scanner lives in its own module in the core crate.
The learning items were marked complete, the scanner is in the core crate, and
workspace `cargo check`, `cargo fmt --all --check`, and `cargo clippy` passed
on 2026-11-26.

## Phase 1 — Wallpaper Core

Goal: a reusable core that can find images and set one as the desktop
wallpaper, driven from a CLI.

- [~] **Wallpaper service**: define a platform-neutral `WallpaperSetter` trait
  in `wallpaper/mod.rs`; implement it for Windows in `wallpaper/windows.rs`
  using the `windows` crate (`IDesktopWallpaper`, with
  `SystemParametersInfoW` as a fallback); validate the file exists before
  calling the API
- [~] **Error model**: one `thiserror` error enum for scan, config, and Windows
  API failures, with paths included in messages

Done when: on Windows 11, `mindswall random <folder>` changes the wallpaper
without administrator rights, scanner tests pass, and the README documents the
CLI.

The checklist records passing CLI and wallpaper-change checks, including the
corrupt-image case. The fallback checks are unchecked and may be marked skipped with a reason if the
COM failure cannot be triggered. The fallback keeps the existing Windows fit
setting, as recorded in D3.

## Phase 2 — Slideshow Engine

Goal: a background slideshow that rotates wallpapers on a timer, with a shuffle
queue that avoids repeats and persisted settings.

- [~] **Logging**: `tracing` with a file log in the per-user data directory

Done when: a slideshow runs for multiple cycles with no repeat inside a cycle,
settings survive restarts, and unit tests cover shuffle, scheduler, and config.

The checklist records passing interactive checks for slideshow cycles, timing,
sleep/resume, history navigation, and control commands. Missing-image and
disconnected-folder cases remain unchecked. Images that cannot be applied are
now skipped and logged (`ca0a272`); this needs Windows verification. Successful
wallpaper changes are logged since 2026-11-26; Phase 2 remains in progress
until the log is re-checked on Windows.

## Phase 3 — Desktop App

Goal: a Tauri 2 + React + TypeScript app that wraps the core, with tray control
and Windows startup.

- [~] **Setup**: Tauri 2 project, React + TypeScript (Vite), custom UI
  controls (D6); the Tauri backend depends on the core crate rather than duplicating logic
- [~] **Command layer**: Tauri commands for status, next/previous/pause,
  folder CRUD, and settings; events from core to UI for wallpaper changes
- [~] **Overview dashboard**: current wallpaper preview, Previous / Next /
  Pause controls, interval, shuffle, include subfolders, prevent repeats
- [~] **Folder management**: add via native folder picker, remove, enable or
  disable, recursive toggle, image count per folder, rescan
- [~] **Settings page**: general options, start with Windows, log location
  (log path with a copy button added 2026-11-26; awaiting Windows check)
- [~] **Single instance (D2)**: a second launch focuses the existing window;
  the app takes over the named pipe server so CLI control commands work
  while it runs

- [~] **Resource check**: destroy or suspend the WebView when hidden so the
      background process can approach the RAM target. Implemented on
      2026-11-26 by destroying the window on close and starting hidden from
      autostart; RAM not measured yet

Done when: the app runs from the tray on Windows 11, all P1 controls work from
both the UI and the tray, settings persist, and startup registration works for
a standard user.

The checklist records passing desktop, tray, CLI control, and startup checks,
including the checked standard-user startup scenario. The UI uses custom
controls (D6). Releasing the WebView on close, event-driven updates,
and the handoff from a running CLI instance are implemented and await Windows
verification.
The checked folder-management and overview actions passed; a dedicated rescan
control and the remaining settings and overview requirements still need review.

## Phase 4 — Wallpaper Utilities

Goal: complete the P2 feature set.

- [~] **Multi-monitor**: enumerate monitors via `IDesktopWallpaper`; same image
  on all monitors or a separate image (and optionally separate folders) per
  monitor; handle monitors being connected or disconnected; Monitors page.
  Per-monitor folders and connect/disconnect handling (10-second polling
  while configured) were implemented on 2026-11-26; need verification on a
  real multi-monitor setup
- [~] **Image gallery**: grid of discovered images with cached, downscaled
  thumbnails (`image` crate, stored in the local cache directory); lazy
  loading or virtualized list for large libraries; set-as-wallpaper and
  exclude actions

- [~] **Wallpaper history**: persisted list of the 10 most recently shown
  images with timestamps and monitor; History page with thumbnails and a
  re-apply action; handles files that no longer exist. The checks passed with
  the earlier 100-entry limit; the 10-entry limit and thumbnails still need
  Windows verification.

- [~] **Folder watcher**: `notify` with debouncing; update the image set and
      shuffle queue on add/remove/rename without a full rescan; skip images
      still being written. Implemented on 2026-11-26 with a 2-second quiet
      period and core tests for event-to-collection mapping; needs Windows
      verification

- [~] **Tests**: filter rules, history limits, watcher event-to-queue mapping

Done when: each P2 feature works on Windows 11 (multi-monitor verified on a
real multi-monitor setup), and gallery browsing stays responsive with a large
folder (for example 5,000+ images).

The GUI now includes paged thumbnails, manual per-monitor images, filters, fit
modes, local history, the folder watcher, per-monitor slideshow folders, and
display-change handling; the last three await Windows verification.
The fixed aspect ratio tolerance is 5%. Gallery scale remains untested.
Reapplying history outside the active collection changes the desktop directly;
the running slideshow may replace it at its next change.

## Phase 5 — Release and Optimization

Goal: measured performance, robust error recovery, and repeatable releases.

- [~] **Benchmarks**: measure idle CPU and background RAM with the window
      hidden and with it open; record method and results in `docs/`. The
      method and `scripts/measure-performance.ps1` are in
      [performance.md](performance.md); no results are recorded yet
- [ ] **Optimization**: fix whatever the measurements show exceeds the targets
- [~] **Test coverage**: fill gaps in core unit tests; add integration tests
      for CLI commands. Recovery, cache, and history tests were added on
      2026-11-26; a CLI integration test covers `scan` and errors.
- [~] **Error recovery**: missing or offline folders, deleted images, corrupt
      images, corrupt config, Windows API failures; the slideshow skips bad
      items and keeps running. Implemented in `ca0a272` and `f8d2046` (see
      Recent changes awaiting verification); needs Windows verification
- [~] **Packaging (D4)**: per-user NSIS installer through the Tauri bundler
      and a portable ZIP; neither needs administrator rights. The bundle
      targets only NSIS with `installMode: currentUser` since 2026-11-26 (it
      previously used `all`, which included the per-machine MSI); the
      installer build and the portable ZIP are not yet produced or verified
- [~] **CI**: GitHub Actions on Windows for `fmt`, `clippy`, `test`, frontend
      lint and type-check, and build. `.github/workflows/ci.yml` was added on
      2026-11-26 and also builds the NSIS installer; it has not run yet, and
      the frontend tests in `ui/tests` are not run because `ui/package.json`
      has no test script
- [~] **Release workflow**: tag-triggered build that produces release
      artifacts; changelog; version bump process.
      `.github/workflows/release.yml`, `CHANGELOG.md`, and
      [releasing.md](releasing.md) were added on 2026-11-26; the workflow
      has not run
- [ ] **Docs**: README with installation, usage, and screenshots

Done when: performance targets are measured and met (or the gap is documented),
CI passes on Windows, and a tagged build produces both portable and installer
artifacts.

## Phase 6 — Extended Features (P3)

Development started before release at the user's request.

- [~] **Wallpaper profiles**: named sets of folders and settings with quick
  switching from the UI and tray
- [~] **Image editor**: crop, resize, blur, and dim applied to a generated copy
  in the cache; never modify original files
- [~] **Import / export**: back up and restore configuration and profiles as
  JSON, with validation on import

The checklist records passing global hotkey checks, including conflicts and
restart. Interactive testing of profiles, image editing, and backup dialogs is
still required. Edited images are applied from the local cache; a running
slideshow may replace one on its next change. No release work is included.

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
├── ui/                   # React + TypeScript frontend (Vite, custom controls)
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
- When the app starts its slideshow while `mindswall run` owns the pipe, it
  sends `handoff`; the CLI replies with its status and exits, and the app
  continues from that status. The app never hands over its own slideshow.
- CLI control commands (`next`, `previous`, `pause`, `resume`, `status`) send
  a request over the pipe. With no running instance they report that clearly;
  one-shot commands (`scan`, `set`, `random`) run standalone.
- The IPC protocol is small JSON messages defined in `mindswall-core`, so the
  CLI and the app share the same types.

### D3. Wallpaper fit mode: default Fill, configurable in Phase 4

- Wallpapers are applied through `IDesktopWallpaper` with the configured fit
  mode. The default mode is Fill.
- Phase 4 adds a fit mode setting (Fill, Fit, Stretch, Center, Tile, Span)
  together with multi-monitor work, since both use `IDesktopWallpaper`.
- When `IDesktopWallpaper` fails, the `SystemParametersInfoW` fallback is used
  only if the configured mode is Fill. The fallback changes the image and
  keeps the existing Windows fit setting; it does not force Fill. Other modes
  report an error instead of applying an image with the wrong fit.

Revised on 2026-11-26 to match the implemented behavior. The fallback API sets
only the image; forcing Fill there would require writing the per-user
`WallpaperStyle` registry values.

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

### D6. UI components: custom React controls instead of shadcn/ui

- The UI keeps its custom controls and stylesheet with `lucide-react` icons.
  shadcn/ui is not used, which avoids adding Tailwind and Radix dependencies
  and rewriting working screens.
- Revisit only if the custom controls become hard to keep accessible or
  consistent.

## Risks

- WebView memory use may exceed the 30 MB background target while the window
  is open; the window is now destroyed when closed, but this is not measured.
- Display changes are detected by polling every 10 seconds, and only while a
  monitor has its own folders. Monitor device paths may change when a display
  moves to another port, which leaves its saved folders unused.
- `IDesktopWallpaper` behavior differs across monitor layouts and Windows
  builds; requires testing on real hardware.
- Very large libraries can make scanning, thumbnailing, and dimension reads
  slow; caching and incremental updates are needed.
- Network drives or removable folders may disappear while the slideshow runs.

## Completed Log

Items moved here from the phase checklists once done. Dates are taken from the
commit that delivered them, except for learning items without a recorded date.

### Learning items — date not recorded

- **Phase 0**: Ownership, borrowing, references, structs, enums, traits,
  `Result`, `Option`, error handling, modules, crates, and filesystem APIs were
  marked complete.

### 2026-11-25

- **Phase 0**: Installed Rust and Cargo.
- **Phase 0**: Converted the repository to the Cargo workspace from D1, with
  `crates/core` and `crates/cli` (`0911d64`).
- **Phase 0**: Moved the scanner from `main.rs` into a `scanner` module in
  `mindswall-core` (`0911d64`).
- **Phase 1 — Scanner**: scans multiple roots, optionally recursive; matches
  `jpg`/`jpeg`/`png`/`bmp` case-insensitively; keeps `PathBuf`s without
  assuming valid Unicode; reports unreadable entries instead of dropping them
  silently (`af68080`).
- **Phase 1 — Random pick**: chooses one random image from the scan result with
  `rand` (`af68080`).
- **Phase 1 — Tests**: scanner tests using temporary directories (extensions,
  case, nesting, non-image files, missing folder) (`af68080`).
- **Phase 1 — CLI (`clap`)**: scan, set, random, recursive options, error cases,
  and non-ASCII paths were marked passed in the `2bf16d4` manual checklist
  (`af68080`).
- **Phase 2 — Multiple folders**: config holds a list of folders with
  per-folder enable and recursive flags; images found through overlapping
  folders are deduplicated (`36a4142`).
- **Phase 2 — Shuffle queue**: shuffles the full set, plays it through without
  repeats, reshuffles when exhausted, and avoids showing the last image of the
  old cycle first in the new one; sequential order when shuffle is off
  (`36a4142`).
- **Phase 2 — JSON configuration**: `serde` + `serde_json`; stored in the
  per-user config directory; a `version` field for future migrations; atomic
  writes (temp file + rename); falls back to defaults with a logged warning on
  invalid JSON (`36a4142`). Since `ca0a272`, an invalid file is first renamed
  to `config.json.invalid-<timestamp>`; that behavior awaits Windows
  verification.
- **Phase 2 — `config.example.json`** in the repository root (`36a4142`).
- **Phase 2 — Tests**: shuffle no-repeat guarantee, reshuffle boundary,
  previous/next behavior, scheduler logic with an injectable clock, config
  round-trip and invalid-file handling (`36a4142`).
- **Phase 2 — History cursor and scheduler**: previous/next navigation,
  pause/resume, interval timing, and sleep/resume; manual checks recorded for
  `2bf16d4` (`36a4142`).
- **Phase 2 — Single instance, IPC, and CLI controls**: standalone slideshow
  control and second-instance checks recorded for `2bf16d4` (`36a4142`).
- **Phase 4 — Resolution and aspect-ratio filtering**: resolution, orientation,
  5% aspect-ratio tolerance, and empty-result checks recorded for `2bf16d4`
  (`7fdc651`).
- **Phase 3 — System tray and startup**: tray controls, hide, quit, and startup
  scenarios were marked passed in the `2bf16d4` manual checklist (`5e82fdb`).
- **Phase 4 — Fit modes**: all listed modes were marked passed in the
  `2bf16d4` manual checklist (`7fdc651`). Wallpaper history moved back to
  Phase 4 as `[~]` after its limit changed to 10 entries.
- **Phase 6 — Global hotkeys**: next, previous, pause/resume, conflict handling,
  hidden-window behavior, and restart checks recorded for `2bf16d4` (`c0acdf7`).
