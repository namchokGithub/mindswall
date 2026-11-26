# Performance measurement

This document describes how to measure MindsWall against its performance
targets. **No results have been recorded yet.** The targets below are goals,
not measured outcomes.

## Targets

From [progress.md](progress.md#performance-targets):

| Metric              | Target         |
| ------------------- | -------------- |
| Idle CPU            | < 0.1% average |
| Background core RAM | < 30 MB        |
| Network usage       | 0              |

## Method

1. **Build type.** Measure a release build, preferably the installed NSIS
   build (`npm.cmd --prefix ui run tauri build`, then install
   `target\release\bundle\nsis\*.exe`). The installed executable is expected to
   be `MindsWall.exe`; a release binary run directly from
   `target\release` is `mindswall-desktop.exe`. Do not measure
   `tauri dev`: it uses a debug build and the Vite dev server.
2. **Configuration.** Use a configuration with at least one enabled folder and
   a running slideshow. Record the interval and the approximate number of
   images in the results.
3. **Settle.** Start the app and wait 2 minutes before sampling so startup
   work (scanning, thumbnail cache cleanup) is finished.
4. **Scenario: window open.** Show the main window on **Overview** and leave it
   untouched. Run the script with `-Label window-open`.
5. **Scenario: window hidden.** Close the window so the app is hidden in the
   tray, wait 2 minutes, then run the script with `-Label window-hidden`.
6. **Network.** During each run, open Resource Monitor (`resmon.exe`), select
   the **Network** tab, and tick `MindsWall.exe` (or `mindswall-desktop.exe`)
   and its `msedgewebview2.exe` processes under **Processes with Network
   Activity**. Record whether any send/receive bytes or TCP connections appear.
   The target is that the processes never appear in that list.

Avoid using the computer during a run; other activity can affect the CPU
numbers of the WebView2 processes.

## Script

`scripts/measure-performance.ps1` works with Windows PowerShell 5.1 and
PowerShell 7 and needs no administrator rights or network access. It samples
the desktop process, all of its descendants (WebView2 processes are reported
with role `webview`), and any running `mindswall.exe` CLI process (role `cli`).
CPU percent is the `TotalProcessorTime` delta divided by elapsed time and the
logical processor count, so it is a percentage of the whole machine. Memory is
reported as working set and private bytes in MB.

Each run writes a CSV to `docs/performance/raw/<timestamp>-<label>.csv` with
the columns `timestamp`, `process_name`, `pid`, `role`, `cpu_percent`,
`working_set_mb`, and `private_mb`, then prints the average and maximum CPU and
memory per role and in total. Processes that exit during a run are skipped.

Run from the repository root:

```powershell
# Window open on Overview (5 minutes, sample every 5 seconds)
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\measure-performance.ps1 -Label window-open

# Window hidden in the tray
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\measure-performance.ps1 -Label window-hidden

# Other options
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\measure-performance.ps1 `
    -Label window-hidden -DurationSeconds 600 -IntervalSeconds 10 `
    -ProcessName MindsWall -OutputDirectory "$env:TEMP\mindswall-perf"
```

Use the `app` row for **Background core RAM** (the Rust process) and report
the `webview` and `total` rows alongside it. Use the private memory average
unless noted otherwise.

## Results

No results have been recorded yet. Add one row per run.

| Date | Commit | Windows build | Scenario | CPU avg (%) | CPU max (%) | RAM total (MB) | RAM app (MB) | RAM webview (MB) | Network | Notes |
| ---- | ------ | ------------- | -------- | ----------- | ----------- | -------------- | ------------ | ---------------- | ------- | ----- |
|      |        |               |          |             |             |                |              |                  |         |       |
