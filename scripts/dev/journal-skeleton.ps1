# Creates a context diary skeleton in .journal/ (git-ignored) with machine-collected facts.
# The context sections are left as TODO-CONTEXT markers for the agent to fill BEFORE compaction.
# Usage: powershell -NoProfile -File scripts/dev/journal-skeleton.ps1 [reason]   -> prints the file path
# Only creates files. Never deletes, moves or overwrites anything.
param([string]$Reason = "pre-compact")

$ErrorActionPreference = "Stop"

function Invoke-Git([string[]]$GitArgs) {
    try { (& git @GitArgs 2>&1 | Out-String).TrimEnd() } catch { "(git unavailable)" }
}

$root = $null
try { $root = (& git rev-parse --show-toplevel 2>$null) } catch { $root = $null }
if (-not $root) { [Console]::Error.WriteLine("journal-skeleton: not inside a git repository"); exit 1 }
Set-Location $root

$safeReason = ($Reason -replace '[^A-Za-z0-9_-]', '-')
$dir = Join-Path $root ".journal"
if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Path $dir | Out-Null }

$stamp = Get-Date -Format "yyyy-MM-dd-HHmmss"
$file = Join-Path $dir "$stamp-$safeReason.md"
if (Test-Path -LiteralPath $file) { [Console]::Error.WriteLine("journal-skeleton: $file already exists"); exit 1 }

$branch = Invoke-Git @("rev-parse", "--abbrev-ref", "HEAD")
$short = Invoke-Git @("rev-parse", "--short", "HEAD")
$status = Invoke-Git @("status", "-sb")
$changed = @((& git status --porcelain 2>$null)).Count
$log = Invoke-Git @("log", "--oneline", "-n", "8")
$worktrees = Invoke-Git @("worktree", "list")
$fence = '```'
$when = Get-Date -Format "yyyy-MM-dd HH:mm"

$body = @"
# Context diary - $when - $safeReason

## Facts collected automatically
- Branch: $branch @ $short
- Working tree: $changed changed files
$fence
$status
$fence
- Last commits:
$fence
$log
$fence
- Worktrees:
$fence
$worktrees
$fence

## Context (agent fills every section, then deletes the HTML comment markers)
### 1. Goal and current phase  <!-- TODO-CONTEXT -->
### 2. Decisions made and WHY (include rejected options)  <!-- TODO-CONTEXT -->
### 3. User preferences and corrections (verbatim where possible)  <!-- TODO-CONTEXT -->
### 4. Done and verified (with evidence: command, exit code, output)  <!-- TODO-CONTEXT -->
### 5. In flight (branches, worktrees, background agents and their ids, files being edited)  <!-- TODO-CONTEXT -->
### 6. Next steps, in order  <!-- TODO-CONTEXT -->
### 7. Open questions for the user  <!-- TODO-CONTEXT -->
### 8. Gotchas and commands that worked or failed  <!-- TODO-CONTEXT -->
### 9. Files to re-read first after compaction  <!-- TODO-CONTEXT -->
### 10. Do NOT redo (already done or already decided)  <!-- TODO-CONTEXT -->
### 11. Promote to durable docs (spec, plan, docs/) - list items and whether done  <!-- TODO-CONTEXT -->
"@

[System.IO.File]::WriteAllText($file, $body, (New-Object System.Text.UTF8Encoding($false)))
Write-Output $file
