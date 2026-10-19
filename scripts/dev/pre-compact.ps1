# PreCompact hook. Enforces the "context diary before compaction" rule (CLAUDE.md).
# manual /compact: blocks (exit 2) until the newest diary (< 30 min) has no TODO-CONTEXT markers left.
# auto compaction: never blocks (it could stall the session); makes sure a skeleton with automatic facts exists.
# Reads the hook JSON from stdin; "compaction_type" defaults to "manual" when absent or unreadable.
# Only reads and creates files. Never deletes anything.
$ErrorActionPreference = "Stop"

$projectDir = $env:CLAUDE_PROJECT_DIR
if (-not $projectDir) { $projectDir = (Get-Location).Path }
Set-Location -LiteralPath $projectDir

$type = "manual"
try {
    $raw = [Console]::In.ReadToEnd()
    if ($raw) {
        $json = $raw | ConvertFrom-Json
        if ($json.compaction_type) { $type = [string]$json.compaction_type }
    }
} catch { $type = "manual" }

$latest = $null
$journalDir = Join-Path $projectDir ".journal"
if (Test-Path -LiteralPath $journalDir) {
    $newest = Get-ChildItem -LiteralPath $journalDir -Filter "*.md" -File |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
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
