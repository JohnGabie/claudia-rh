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
$journalDir = Join-Path (Get-MainRoot $projectDir) ".journal"
$sessionId = Get-SessionId $projectDir
if (Test-Path -LiteralPath $journalDir) {
    $newest = Get-ChildItem -LiteralPath $journalDir -Filter "*.md" -File |
        Sort-Object LastWriteTime -Descending | Where-Object { Test-DiaryOwner $_.FullName $sessionId } | Select-Object -First 1
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
    if ($type -eq "manual" -and ($sessionId.Branch -or $sessionId.Worktree)) {
        [Console]::Error.WriteLine("Context diary incomplete: $latest still has '<!-- TODO-CONTEXT -->' markers. Ask the agent to fill every section (and remove the markers), then run /compact again.")
        exit 2
    }
    [Console]::Error.WriteLine("Auto-compaction proceeding; diary skeleton at $latest has only automatic facts.")
}
exit 0
