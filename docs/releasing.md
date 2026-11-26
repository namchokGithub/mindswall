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
3. Builds the per-user NSIS installer with `npm --prefix ui run tauri build`
   and the CLI with `cargo build --release -p mindswall-cli`.
4. Packages `MindsWall_<version>_x64_portable.zip` with `MindsWall.exe`,
   `mindswall.exe`, `LICENSE`, and `README.md`, and writes `SHA256SUMS.txt`.
5. Uploads the files as a workflow artifact and attaches them to a draft
   release whose notes are the changelog section.

The portable build needs the Microsoft Edge WebView2 Runtime, which Windows 11
includes. It stores settings in the same per-user folders as the installed
build. Neither build needs administrator rights.

Before publishing the draft, install the NSIS build and run the portable ZIP on
Windows 11 as a standard user and follow `docs/testing/windows-11-checklist.md`.
