# Releasing MindsWall

Releases are built by `.github/workflows/release.yml` when a version tag is
pushed. The workflow produces a draft GitHub Release; a maintainer reviews and
publishes it by hand.

## Version bump

The version appears in these files and must match the tag:

- `crates/core/Cargo.toml`
- `crates/cli/Cargo.toml`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `ui/package.json` (and `ui/package-lock.json`, updated by
  `npm --prefix ui version <version> --no-git-tag-version`)
- `installer/Cargo.toml`
- `installer/tauri.conf.json`
- `installer/ui/package.json` (and `installer/ui/package-lock.json`, updated
  by `npm --prefix installer/ui version <version> --no-git-tag-version`)

The UI footer in `ui/src/main.tsx` also shows the version; update it too.

1. Update the versions above and run `cargo check` so `Cargo.lock` follows.
2. In `CHANGELOG.md`, rename `## [Unreleased]` to
   `## [<version>] - <YYYY-MM-DD>` and add a new empty `## [Unreleased]`
   section above it.
3. Commit, for example `chore(release): prepare <version>`.
4. Tag and push:

   ```powershell
   git tag v<version>
   git push origin main v<version>
   ```

## What the workflow does

1. Runs the CI checks (format, Clippy, tests) through `ci.yml`.
2. Fails if any version above differs from the tag or `CHANGELOG.md` has no
   section for the version.
3. Builds the app and the NSIS installer with `npm --prefix ui run tauri
   build`, the CLI with `cargo build --release -p mindswall-cli`, and
   MindsWall Setup with `cargo build --release -p mindswall-setup` after
   building its screens (`npm --prefix installer/ui run build`) and setting
   `MINDSWALL_APP_EXE` to the release `mindswall-desktop.exe`, which the setup
   embeds.
4. Names MindsWall Setup `MindsWall_<version>_x64-setup.exe` and the NSIS
   installer `MindsWall_<version>_x64-nsis-setup.exe`, packages
   `MindsWall_<version>_x64_portable.zip` with `MindsWall.exe`,
   `mindswall.exe`, `portable.txt`, `LICENSE`, and `README.md`, and writes
   `SHA256SUMS.txt`. Neither installer includes the CLI.
5. Uploads the files as a workflow artifact and attaches them to a draft
   release. Its notes are the changelog section followed by a fixed
   **Downloads** section that lists the files (GitHub adds the source code
   archives) and warns that the builds are not code-signed and may be blocked
   by SmartScreen, Smart App Control, or antivirus software, linking to the
   README steps.

MindsWall Setup (`installer/`, see its README) has its own window and needs
the WebView2 Runtime, which Windows 11 includes. The NSIS installer embeds the
WebView2 bootstrapper (`webviewInstallMode` `embedBootstrapper` in
`src-tauri/tauri.conf.json`), so it can install the runtime where it is
missing; the runtime itself still comes from Microsoft. The NSIS installer is
kept as a fallback while MindsWall Setup is new; both register under the same
**Settings** > **Apps** entry, so use one or the other. The portable build
needs the runtime already installed.

A setup built without `MINDSWALL_APP_EXE` (as `cargo build` and the CI checks
do) contains no app and refuses to install.

`portable.txt` next to the programs makes the build portable: settings,
history, logs, caches, and the WebView profile are kept in `config`, `data`,
and `cache` folders beside it (see `crates/core/src/paths.rs`). Neither build
needs administrator rights. Neither build is code-signed.

The user guide for these files is [installing.md](installing.md); update it
when the files or their behavior change.

The CI workflow also uploads `mindswall-setup.exe` as an artifact on every
push, for testing MindsWall Setup without a release.

Before publishing the draft, install with MindsWall Setup and run the portable ZIP on
Windows 11 as a standard user and follow `docs/testing/windows-11-checklist.md`.
