# Installing MindsWall

MindsWall runs on Windows 11 for the current user. Neither the installer nor
the portable build needs administrator rights; an elevation (UAC) prompt is
not expected.

No release has been published yet. This guide describes the planned release
files; see [releasing.md](releasing.md) for how they are built.

## Choose a build

| File | Use it when |
| --- | --- |
| `MindsWall_<version>_x64-setup.exe` | You want MindsWall installed for your user, with an entry in **Settings** > **Apps**. |
| `MindsWall_<version>_x64_portable.zip` | You want everything in one folder that you can move or delete, or you want the CLI (`mindswall.exe`). |
| `SHA256SUMS.txt` | Checking that a download is complete and unchanged. |

Use one build at a time. The installed and portable apps cannot run at the
same time.

## 1. Download and check the file

1. Download the files only from this repository's GitHub Releases page.
2. Check the file against `SHA256SUMS.txt` before running it:

   ```powershell
   Get-FileHash .\MindsWall_<version>_x64-setup.exe -Algorithm SHA256
   ```

   The hash must match the line for that file. If it does not, delete the file
   and do not run it.

## 2. Allow the unsigned build

The builds are not code-signed, so Windows may warn about them:

- **SmartScreen** may show **Windows protected your PC** the first time. Select
  **More info**, check that the app name is MindsWall, then select
  **Run anyway**.
- **Smart App Control** (Windows Security > App & browser control > Smart App
  Control) blocks unsigned apps and offers no **Run anyway**. Turning it off
  cannot be undone without resetting Windows, so consider
  [building from source](../README.md#desktop-app-development) instead.
- **Antivirus software** may scan or quarantine a new unsigned program. Check
  the file hash before allowing it.

Code signing is not planned for now, so these warnings remain.

## 3a. Install with the installer

1. Run `MindsWall_<version>_x64-setup.exe` and follow the steps.
2. Start MindsWall from the Start menu.

The installer includes the WebView2 setup. Windows 11 already has the WebView2
Runtime, so nothing is downloaded. On a computer without it, the setup installs
the runtime from Microsoft, which needs an internet connection once.

The installer does not include the CLI; use the portable ZIP for it.

## 3b. Use the portable ZIP

1. Unblock the ZIP before extracting, so the extracted programs do not trigger
   the warning again: right-click the ZIP, select **Properties**, tick
   **Unblock**, and select **OK**. Or run:

   ```powershell
   Unblock-File .\MindsWall_<version>_x64_portable.zip
   ```

2. Extract it to a folder you can write to in your user profile, such as
   `%USERPROFILE%\Apps\MindsWall`. Do not use `Program Files`.
3. Run `MindsWall.exe`.

The folder contains `MindsWall.exe` (the app), `mindswall.exe` (the CLI),
`portable.txt`, `LICENSE`, and `README.md`. While `portable.txt` is there, both
programs keep their files in folders next to them:

```
MindsWall\
  MindsWall.exe  mindswall.exe  portable.txt
  config\   config.json
  data\     history.json, logs\, webview\
  cache\    thumbnails, previews, combined wallpapers, edited images
```

Delete `portable.txt` to make the programs use the per-user folders instead.

Notes for the portable build:

- It needs the WebView2 Runtime, which Windows 11 includes.
- Combined and edited wallpapers are stored in `cache`, so a folder on a
  removable drive must stay connected while they are shown.
- **Start with Windows** remembers where `MindsWall.exe` is. After moving the
  folder, turn it off and on again in **Settings**.

## Where files are kept

| What | Installed | Portable |
| --- | --- | --- |
| Settings | `%APPDATA%\MindsWall\config\config.json` | `config\config.json` |
| History | `%APPDATA%\MindsWall\data\history.json` | `data\history.json` |
| Logs | `%APPDATA%\MindsWall\data\logs` | `data\logs` |
| Caches | `%LOCALAPPDATA%\MindsWall\cache` | `cache` |
| WebView data | `%LOCALAPPDATA%\com.mindswall.desktop` | `data\webview` |

The first line of each log file shows `portable=true` or `portable=false`.
**Settings** > **Log file** shows the current log.

## Upgrade

- **Installer:** quit MindsWall from the tray, then run the new
  `MindsWall_<version>_x64-setup.exe`. Settings and history are kept.
- **Portable:** quit MindsWall, then extract the new ZIP over the old folder,
  replacing the programs. The `config`, `data`, and `cache` folders are kept.

## Uninstall

1. In MindsWall, turn off **Start with Windows** in **Settings**, then quit it
   from the tray. The uninstaller does not remove the startup entry.
2. Remove the program:
   - **Installer:** **Settings** > **Apps** > **Installed apps** > MindsWall >
     **Uninstall**. The uninstaller's option to delete app data removes only
     the WebView data, not your settings.
   - **Portable:** delete the folder.
3. For the installed build, delete your settings, history, logs, and caches if
   you no longer want them:

   ```powershell
   Remove-Item -Recurse "$env:APPDATA\MindsWall", "$env:LOCALAPPDATA\MindsWall"
   ```

   This cannot be undone. Your image folders are not touched.

The Windows wallpaper stays as it was last set. If it was a combined or edited
image from the deleted cache, choose a new wallpaper in Windows Settings.

## Move between the installed and portable builds

Quit MindsWall first. Copy `config.json` (settings and profiles) and, if you
want it, `history.json` between the locations in
[Where files are kept](#where-files-are-kept). Caches are rebuilt as needed.
If **Start with Windows** was on, turn it off in the old build and on in the
new one.

## Build from source

See [Desktop app (development)](../README.md#desktop-app-development) in the
README.
