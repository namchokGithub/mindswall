<#
.SYNOPSIS
    Samples CPU and memory use of the MindsWall desktop app, its WebView2
    child processes, and the MindsWall CLI.

.DESCRIPTION
    Every IntervalSeconds for DurationSeconds, the script finds the MindsWall
    desktop process (by default mindswall-desktop.exe for development builds
    and MindsWall.exe for installed builds), all of its descendant processes
    (for example msedgewebview2.exe), and any running mindswall.exe CLI
    process. For each process it records CPU use since the previous sample,
    the working set, and private memory.

    CPU percent is computed from TotalProcessorTime deltas divided by the
    elapsed wall-clock time and the logical processor count, so 100 means
    every logical processor was busy for the whole interval.

    The script only reads local process information. It needs no network
    access and no administrator rights. Processes that exit during the run
    are skipped from that sample onward.

.PARAMETER DurationSeconds
    Total sampling duration in seconds. Default: 300.

.PARAMETER IntervalSeconds
    Seconds between samples. Default: 5.

.PARAMETER Label
    Scenario label used in the output file name, for example window-open or
    window-hidden.

.PARAMETER OutputDirectory
    Directory for the CSV file. Default: docs/performance/raw under the
    repository root.

.PARAMETER ProcessName
    Desktop process names to look for, without the .exe extension.
    Default: mindswall-desktop, MindsWall.

.PARAMETER CliProcessName
    CLI process name without the .exe extension. Default: mindswall.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\measure-performance.ps1 -Label window-hidden
#>
[CmdletBinding()]
param(
    [ValidateRange(1, 86400)]
    [int]$DurationSeconds = 300,

    [ValidateRange(1, 3600)]
    [int]$IntervalSeconds = 5,

    [ValidatePattern('^[A-Za-z0-9._-]+$')]
    [string]$Label = 'unlabeled',

    [string]$OutputDirectory,

    [string[]]$ProcessName = @('mindswall-desktop', 'MindsWall'),

    [string]$CliProcessName = 'mindswall'
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'

if (-not $OutputDirectory) {
    $repoRoot = Split-Path -Parent $PSScriptRoot
    $OutputDirectory = Join-Path (Join-Path (Join-Path $repoRoot 'docs') 'performance') 'raw'
}

$logicalProcessors = [Environment]::ProcessorCount
$appNames = @($ProcessName | ForEach-Object { $_.ToLowerInvariant() -replace '\.exe$', '' })
$cliName = ($CliProcessName.ToLowerInvariant() -replace '\.exe$', '')

function Get-ProcessTable {
    # Win32_Process gives ParentProcessId, which Get-Process lacks in
    # Windows PowerShell 5.1. Querying it needs no elevation.
    $table = @{}
    foreach ($p in Get-CimInstance -ClassName Win32_Process -Property ProcessId, ParentProcessId, Name) {
        $table[[int]$p.ProcessId] = $p
    }
    return $table
}

function Get-TrackedProcesses {
    param([hashtable]$Table)

    $tracked = @{}
    $roots = @()
    foreach ($p in $Table.Values) {
        $name = ([string]$p.Name).ToLowerInvariant() -replace '\.exe$', ''
        if ($appNames -contains $name) {
            $tracked[[int]$p.ProcessId] = 'app'
            $roots += [int]$p.ProcessId
        }
        elseif ($name -eq $cliName) {
            $tracked[[int]$p.ProcessId] = 'cli'
        }
    }

    # Walk descendants breadth-first. Windows can reuse a parent's PID, so
    # only accept children whose parent is already tracked as app/webview.
    $queue = New-Object System.Collections.Generic.Queue[int]
    foreach ($id in $roots) { $queue.Enqueue($id) }
    while ($queue.Count -gt 0) {
        $parent = $queue.Dequeue()
        foreach ($p in $Table.Values) {
            $id = [int]$p.ProcessId
            if ([int]$p.ParentProcessId -eq $parent -and $id -ne $parent -and -not $tracked.ContainsKey($id)) {
                $tracked[$id] = 'webview'
                $queue.Enqueue($id)
            }
        }
    }
    return $tracked
}

if (-not (Test-Path -LiteralPath $OutputDirectory)) {
    New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
}
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$csvPath = Join-Path $OutputDirectory ("{0}-{1}.csv" -f $stamp, $Label)

$initialTracked = Get-TrackedProcesses -Table (Get-ProcessTable)
if (-not (@($initialTracked.Values) -contains 'app')) {
    Write-Warning ("No desktop process found (looked for: {0}). Start MindsWall or pass -ProcessName." -f ($appNames -join ', '))
}

Write-Host ("Sampling every {0}s for {1}s ({2} logical processors). Output: {3}" -f $IntervalSeconds, $DurationSeconds, $logicalProcessors, $csvPath)

# Previous CPU time per PID. The key includes the start time so a reused PID
# is not compared with an unrelated earlier process.
$previous = @{}
$rows = New-Object System.Collections.Generic.List[object]
$clock = [System.Diagnostics.Stopwatch]::StartNew()
$lastTick = $clock.Elapsed
$deadline = [TimeSpan]::FromSeconds($DurationSeconds)
$firstSample = $true

while ($true) {
    $now = $clock.Elapsed
    $elapsedSeconds = ($now - $lastTick).TotalSeconds
    $lastTick = $now
    $timestamp = (Get-Date).ToString('o')
    $tracked = Get-TrackedProcesses -Table (Get-ProcessTable)
    $current = @{}

    foreach ($entry in $tracked.GetEnumerator()) {
        $processId = [int]$entry.Key
        $role = [string]$entry.Value
        try {
            $proc = Get-Process -Id $processId -ErrorAction Stop
            $cpuTime = $proc.TotalProcessorTime
            $key = '{0}:{1}' -f $processId, $proc.StartTime.Ticks
            $workingSetMb = [math]::Round($proc.WorkingSet64 / 1MB, 2)
            $privateMb = [math]::Round($proc.PrivateMemorySize64 / 1MB, 2)
            $name = $proc.ProcessName
        }
        catch {
            # The process exited (or became inaccessible) since the
            # Win32_Process query; skip it for this and later samples.
            continue
        }
        $current[$key] = $cpuTime

        if ($firstSample -or -not $previous.ContainsKey($key) -or $elapsedSeconds -le 0) {
            # The first observation of a process only establishes a baseline.
            continue
        }
        $deltaSeconds = ($cpuTime - $previous[$key]).TotalSeconds
        $cpuPercent = [math]::Round(100 * $deltaSeconds / ($elapsedSeconds * $logicalProcessors), 4)

        $rows.Add([pscustomobject]@{
                timestamp      = $timestamp
                process_name   = $name
                pid            = $processId
                role           = $role
                cpu_percent    = $cpuPercent
                working_set_mb = $workingSetMb
                private_mb     = $privateMb
            })
    }

    $previous = $current
    $firstSample = $false

    if ($now -ge $deadline) { break }
    $remaining = ($deadline - $clock.Elapsed).TotalSeconds
    $sleep = [math]::Min([double]$IntervalSeconds, [math]::Max(0.0, $remaining))
    if ($sleep -gt 0) { Start-Sleep -Milliseconds ([int]($sleep * 1000)) }
    else { break }
}

if ($rows.Count -eq 0) {
    Write-Warning 'No samples were recorded. Check that MindsWall is running.'
    exit 1
}

$rows | Export-Csv -LiteralPath $csvPath -NoTypeInformation -Encoding UTF8
Write-Host ("Wrote {0} rows to {1}" -f $rows.Count, $csvPath)

# Aggregate per sample timestamp first, so averages describe the whole role
# (for example all WebView2 processes together) rather than single processes.
function Get-Summary {
    param([string]$Name, [object[]]$Samples)

    $perTick = $Samples | Group-Object timestamp | ForEach-Object {
        [pscustomobject]@{
            cpu     = ($_.Group | Measure-Object cpu_percent -Sum).Sum
            working = ($_.Group | Measure-Object working_set_mb -Sum).Sum
            private = ($_.Group | Measure-Object private_mb -Sum).Sum
        }
    }
    $cpu = $perTick | Measure-Object cpu -Average -Maximum
    $ws = $perTick | Measure-Object working -Average -Maximum
    $pm = $perTick | Measure-Object private -Average -Maximum
    [pscustomobject]@{
        role             = $Name
        samples          = $cpu.Count
        cpu_avg_percent  = [math]::Round($cpu.Average, 4)
        cpu_max_percent  = [math]::Round($cpu.Maximum, 4)
        ws_avg_mb        = [math]::Round($ws.Average, 2)
        ws_max_mb        = [math]::Round($ws.Maximum, 2)
        private_avg_mb   = [math]::Round($pm.Average, 2)
        private_max_mb   = [math]::Round($pm.Maximum, 2)
    }
}

$summary = @()
foreach ($role in @('app', 'webview', 'cli')) {
    $samples = @($rows | Where-Object { $_.role -eq $role })
    if ($samples.Count -gt 0) { $summary += Get-Summary -Name $role -Samples $samples }
}
$summary += Get-Summary -Name 'total' -Samples @($rows)

Write-Host ''
Write-Host ("Summary ({0}): CPU percent of all logical processors; memory in MB" -f $Label)
$summary | Format-Table -AutoSize | Out-String | Write-Host
