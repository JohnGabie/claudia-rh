# PreCompact hook. Enforces the "context diary before compaction" rule (CLAUDE.md).
# manual /compact: blocks (exit 2) until the newest diary (< 30 min) has no TODO-CONTEXT markers left.
# auto compaction (or unknown type): never blocks (it could stall the session); makes sure a skeleton with automatic facts exists.
# Reads the hook JSON from stdin; only an explicit "manual" blocks. An absent, unreadable or unknown type never blocks.
# Only reads and creates files. Never deletes anything.
$ErrorActionPreference = "Stop"

$projectDir = $env:CLAUDE_PROJECT_DIR
if (-not $projectDir) { $projectDir = (Get-Location).Path }
Set-Location -LiteralPath $projectDir

# The documented field is "trigger" ("manual" or "auto"); "compaction_type" is accepted as a fallback.
# When the type cannot be determined it stays "unknown", and "unknown" never blocks.
$type = "unknown"
try {
    $raw = [Console]::In.ReadToEnd()
    if ($raw) {
        $json = $raw | ConvertFrom-Json
        if ($json.trigger) { $type = [string]$json.trigger }
        elseif ($json.compaction_type) { $type = [string]$json.compaction_type }
    }
} catch { $type = "unknown" }

$latest = $null
# The diary is kept in the MAIN checkout (parent of the git common dir); falls back to the project folder.
function Get-MainRoot([string]$Dir) {
    $common = $null
    try { $common = (& git -C $Dir rev-parse --path-format=absolute --git-common-dir 2>$null) } catch { return $Dir }
    if ($LASTEXITCODE -ne 0 -or -not $common) { return $Dir }
    $common = ([string]($common | Select-Object -First 1)).Trim()
    if ((Split-Path -Leaf $common) -ne ".git") { return $Dir }
    $parent = [IO.Path]::GetFullPath((Split-Path -Parent $common))
    # Only accept the parent when it is itself a work tree root (rejects --separate-git-dir layouts).
    $top = $null
    try { $top = (& git -C $parent rev-parse --show-toplevel 2>$null) } catch { return $Dir }
    if ($LASTEXITCODE -ne 0 -or -not $top) { return $Dir }
    $top = [IO.Path]::GetFullPath(([string]($top | Select-Object -First 1)).Trim())
    if ($top.TrimEnd('\') -ine $parent.TrimEnd('\')) { return $Dir }
    # A separate git dir looks like a work tree to git but has no checked-out files: the first tracked file must exist on disk.
    $first = $null
    try { $first = (& git -C $parent -c core.quotepath=false ls-files 2>$null | Select-Object -First 1) } catch { return $Dir }
    if ($first -and -not (Test-Path -LiteralPath (Join-Path $parent ([string]$first)))) { return $Dir }
    return $parent
}
$journalDir = Join-Path (Get-MainRoot $projectDir) ".journal"
# Newest diary written by THIS session's branch (exact match on the "- Branch: <name> @" line the skeleton writes).
# When the branch cannot be determined, any diary matches.
function Get-SessionBranch([string]$Dir) {
    try {
        $b = (& git -C $Dir rev-parse --abbrev-ref HEAD 2>$null)
        if ($LASTEXITCODE -eq 0 -and $b) { return ([string]($b | Select-Object -First 1)).Trim() }
    } catch { }
    return $null
}
function Test-DiaryBranch([string]$Path, $Branch) {
    if (-not $Branch) { return $true }
    $m = Select-String -LiteralPath $Path -Pattern '^- Branch: (.*) @ ' | Select-Object -First 1
    return [bool]($m -and ($m.Matches[0].Groups[1].Value -ceq $Branch))
}
if (Test-Path -LiteralPath $journalDir) {
    $sessionBranch = Get-SessionBranch $projectDir
    $newest = Get-ChildItem -LiteralPath $journalDir -Filter "*.md" -File |
        Sort-Object LastWriteTime -Descending | Where-Object { Test-DiaryBranch $_.FullName $sessionBranch } | Select-Object -First 1
    if ($newest) { $latest = $newest.FullName }
}

$fresh = $false
if ($latest) {
    $age = ((Get-Date) - (Get-Item -LiteralPath $latest).LastWriteTime).TotalSeconds
    if ($age -lt 1800) { $fresh = $true }
}

if (-not $fresh) {
    $skeleton = Join-Path $PSScriptRoot "journal-skeleton.ps1"
    $latest = (& powershell -NoProfile -ExecutionPolicy Bypass -File $skeleton "pre-compact-$type" | Select-Object -Last 1)
}

if ($latest -and (Select-String -LiteralPath $latest -SimpleMatch "<!-- TODO-CONTEXT -->" -Quiet)) {
    if ($type -eq "manual") {
        [Console]::Error.WriteLine("Context diary incomplete: $latest still has '<!-- TODO-CONTEXT -->' markers. Ask the agent to fill every section (and remove the markers), then run /compact again.")
        exit 2
    }
    [Console]::Error.WriteLine("Auto-compaction proceeding; diary skeleton at $latest has only automatic facts.")
}
exit 0
