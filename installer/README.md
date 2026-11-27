# MindsWall Setup

A custom installer and uninstaller for MindsWall with its own window
(`mindswall-setup`, a Tauri program). Releases ship it as
`MindsWall_<version>_x64-setup.exe`, with the NSIS installer that Tauri builds
kept as a fallback. It has not been verified on Windows yet.

## Screens

`ui/` is a React app for a 760 × 480 window without a title bar. It follows
the Windows light or dark mode.

- Install: Welcome, Installation settings (location, desktop shortcut, launch
  after installing, start with Windows), progress, and Complete.
- Update: the same steps when an older version is installed; the location is
  kept.
- Uninstall: a confirmation with an option to delete settings, history, logs,
  and caches, progress, and Complete.
- Errors stop with the message and offer to try again; a notice says when
  MindsWall is open and will be closed first.

Run the screens in a browser with a mock setup program (the mock is used
whenever the page is not inside the setup program):

```powershell
npm install --prefix installer/ui
npm --prefix installer/ui run dev
```

Then open <http://127.0.0.1:1430/>. Add these to the address to see other
states: `?installed=0.9.0` (update), `?running=1` (MindsWall is open),
`?fail=1` (the work fails halfway), and `?mode=uninstall`.

## What it does

Everything is for the current user; no administrator rights are needed, and
the manifest asks Windows not to elevate.

Installing (`src/system.rs`, `install`):

1. Checks the location (`src/plan.rs`, `location_problem`) and creates it.
   The default is `%LOCALAPPDATA%\Programs\MindsWall`.
2. Ends MindsWall if it runs from that folder. Copies elsewhere, such as a
   portable one, are left alone.
3. Writes `MindsWall.exe`, keeping the old copy until the new one is in
   place, and copies the setup there as `uninstall.exe`.
4. Creates the Start menu shortcut and, if chosen, the desktop shortcut.
5. Adds the entry under **Settings** > **Apps**
   (`HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\MindsWall`).
6. Turns **Start with Windows** on or off with the same `Run` entry the app's
   own setting writes (`MindsWall`, started with `--hidden`).

Uninstalling (`uninstall.exe --uninstall`): the uninstaller copies itself to
the temporary folder and continues from there, so it can delete
`uninstall.exe`. It refuses to run where MindsWall is not installed. It ends
MindsWall and removes the program files and, only when they point at this
install, Start with Windows, both shortcuts, and the **Apps** entry, and
optionally `%APPDATA%\MindsWall`, `%LOCALAPPDATA%\MindsWall`, and
`%LOCALAPPDATA%\com.mindswall.desktop`. Files the user added to the install
folder are kept. The temporary copy, the emptied folder, and the setup
window's WebView data (`%TEMP%\MindsWall Setup WebView`) are deleted a few
seconds after it exits.

## Build

The setup embeds the app it installs. Build the app first, then the setup with
`MINDSWALL_APP_EXE` pointing at it:

```powershell
npm --prefix ui run tauri build
npm ci --prefix installer/ui
npm --prefix installer/ui run build
$env:MINDSWALL_APP_EXE = (Resolve-Path target\release\mindswall-desktop.exe).Path
cargo build --release -p mindswall-setup
```

The result is `target\release\mindswall-setup.exe`. Without
`MINDSWALL_APP_EXE` the setup still builds (for checks and tests) but contains
no app and refuses to install. Run `target\release\mindswall-setup.exe
--uninstall` to see the uninstaller.

`installer/ui/dist` must exist before Rust compiles the setup, because the
screens are built into the program.

## Known limits

- MindsWall is ended rather than asked to quit, so its tray icon may stay
  until the mouse passes over it.
- MindsWall Setup needs the WebView2 Runtime (included in Windows 11); unlike
  the NSIS installer, it cannot install it.
- Both installers use the same **Settings** > **Apps** entry. Use one or the
  other.
