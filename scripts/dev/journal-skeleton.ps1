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

# --- Shared diary helpers (identical in journal-skeleton.ps1, pre-compact.ps1 and session-start.ps1) ---
function ConvertTo-NormPath([string]$P) { return [IO.Path]::GetFullPath($P.Trim()).TrimEnd('\') }
# Root of the MAIN checkout, so the diary survives removing a worktree. Falls back to the session folder.
function Get-MainRoot([string]$Dir) {
    try {
        $gd = (& git -C $Dir rev-parse --path-format=absolute --git-dir 2>$null)
        if ($LASTEXITCODE -ne 0 -or -not $gd) { return $Dir }
        $cd = (& git -C $Dir rev-parse --path-format=absolute --git-common-dir 2>$null)
        if ($LASTEXITCODE -ne 0 -or -not $cd) { return $Dir }
        $gd = ConvertTo-NormPath ([string]($gd | Select-Object -First 1))
        $cd = ConvertTo-NormPath ([string]($cd | Select-Object -First 1))
        if ($gd -ieq $cd) {
            # Session is in the main checkout itself (also covers --separate-git-dir).
            $top = (& git -C $Dir rev-parse --show-toplevel 2>$null)
            if ($LASTEXITCODE -ne 0 -or -not $top) { return $Dir }
            return ConvertTo-NormPath ([string]($top | Select-Object -First 1))
        }
        # Linked worktree: the common dir must be "<main>\.git" and a real folder.
        if ((Split-Path -Leaf $cd) -ne ".git") { return $Dir }
        $parent = Split-Path -Parent $cd
        if (Test-Path -LiteralPath (Join-Path $parent ".git") -PathType Container) { return $parent }
        return $Dir
    } catch { return $Dir }
}
# Identity of the session: Branch (null when detached or unknown) and Worktree (its top-level folder, null when unknown).
function Get-SessionId([string]$Dir) {
    $branch = $null; $top = $null
    try {
        $b = (& git -C $Dir symbolic-ref --short -q HEAD 2>$null)
        if ($LASTEXITCODE -eq 0 -and $b) { $branch = ([string]($b | Select-Object -First 1)).Trim() }
        $t = (& git -C $Dir rev-parse --show-toplevel 2>$null)
        if ($LASTEXITCODE -eq 0 -and $t) { $top = ConvertTo-NormPath ([string]($t | Select-Object -First 1)) }
    } catch { }
    return [pscustomobject]@{ Branch = $branch; Worktree = $top }
}
# True when the diary belongs to this session: exact branch when named, exact worktree path otherwise.
# With neither known, nothing matches.
function Test-DiaryOwner([string]$Path, $Id) {
    if ($Id.Branch) {
        $m = Select-String -LiteralPath $Path -Pattern '^- Branch: (.*) @ ' | Select-Object -First 1
        return [bool]($m -and ($m.Matches[0].Groups[1].Value -ceq $Id.Branch))
    }
    if ($Id.Worktree) {
        $m = Select-String -LiteralPath $Path -Pattern '^- Worktree: (.*)$' | Select-Object -First 1
        return [bool]($m -and ((ConvertTo-NormPath $m.Matches[0].Groups[1].Value) -ieq $Id.Worktree))
    }
    return $false
}
# --- end of shared helpers ---
$mainRoot = Get-MainRoot $root

$safeReason = ($Reason -replace '[^A-Za-z0-9_-]', '-')
$sessionId = Get-SessionId $root
$branch = if ($sessionId.Branch) { $sessionId.Branch } else { "HEAD" }
$safeBranch = ($branch -replace '[^A-Za-z0-9_-]', '-').Trim('-')
if (-not $safeBranch) { $safeBranch = "nobranch" }
$dir = Join-Path $mainRoot ".journal"
if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Path $dir | Out-Null }
# Keep the diary out of git status even when the repository ignore file lacks a rule. Never overwrites.
$ignoreFile = Join-Path $dir ".gitignore"
if (-not (Test-Path -LiteralPath $ignoreFile)) { [System.IO.File]::WriteAllText($ignoreFile, "*`n", (New-Object System.Text.UTF8Encoding($false))) }

$stamp = Get-Date -Format "yyyy-MM-dd-HHmmss"
$file = Join-Path $dir "$stamp-$safeBranch-$safeReason.md"
if (Test-Path -LiteralPath $file) { [Console]::Error.WriteLine("journal-skeleton: $file already exists"); exit 1 }

$short = (& git rev-parse --short HEAD 2>$null); if ($LASTEXITCODE -ne 0 -or -not $short) { $short = "unborn" }
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
- Worktree: $($sessionId.Worktree)
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
