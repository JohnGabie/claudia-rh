# Creates a context diary skeleton in .journal/ (git-ignored) with machine-collected facts.
# The context sections are left as TODO-CONTEXT markers for the agent to fill BEFORE compaction.
# Usage: powershell -NoProfile -File scripts/dev/journal-skeleton.ps1 [reason]   -> prints the file path
# Only creates files. Never deletes, moves or overwrites anything.
param([string]$Reason = "pre-compact")

$ErrorActionPreference = "Stop"

# --- Shared diary helpers (identical in journal-skeleton.ps1, pre-compact.ps1 and session-start.ps1) ---
# Output lines of a git command run in $Dir; empty when git fails or is missing. Windows PowerShell 5.1
# turns native stderr into a terminating error under "Stop", so stderr is dropped with "Continue" here.
function Get-GitLines([string]$Dir, [string[]]$GitArgs) {
    $ErrorActionPreference = "Continue"
    try {
        $out = @(& git -C $Dir @GitArgs 2>$null)
        if ($LASTEXITCODE -ne 0) { return @() }
        return @($out | ForEach-Object { [string]$_ })
    } catch { return @() }
}
# First non-empty output line of a git command, or $null.
function Get-GitLine([string]$Dir, [string[]]$GitArgs) {
    $l = @(Get-GitLines $Dir $GitArgs | Where-Object { $_.Trim() } | Select-Object -First 1)
    if ($l.Count -eq 0) { return $null }
    return $l[0].Trim()
}
function ConvertTo-NormPath([string]$P) { return [IO.Path]::GetFullPath($P.Trim()).TrimEnd('\') }
# True when $Parent is the main checkout that owns $CommonDir. git must see $Parent as the top of a work tree
# whose git dir is exactly $CommonDir. git cannot tell that apart from a separate git dir named ".git"
# (git init --separate-git-dir <store>\.git): for both it reports <store> as the main work tree. So a folder
# holding nothing but the git dir (and .journal) is rejected when HEAD has files to check out: the real work
# tree of that repository is elsewhere. A main checkout of an empty HEAD tree (or no commit) is accepted.
function Test-MainCheckout([string]$Parent, [string]$CommonDir) {
    $pgd = Get-GitLine $Parent @("rev-parse", "--path-format=absolute", "--git-dir")
    if (-not $pgd -or ((ConvertTo-NormPath $pgd) -ine $CommonDir)) { return $false }
    if ((Get-GitLine $Parent @("rev-parse", "--is-inside-work-tree")) -ne "true") { return $false }
    $ptop = Get-GitLine $Parent @("rev-parse", "--show-toplevel")
    if (-not $ptop -or ((ConvertTo-NormPath $ptop) -ine $Parent)) { return $false }
    $other = @(Get-ChildItem -LiteralPath $Parent -Force | Where-Object { $_.Name -ne ".git" -and $_.Name -ne ".journal" })
    if ($other.Count -gt 0) { return $true }
    return (-not (Get-GitLine $Parent @("ls-tree", "HEAD")))
}
# Root of the MAIN checkout, so the diary survives removing a worktree. Falls back to the session folder.
function Get-MainRoot([string]$Dir) {
    try {
        $gd = Get-GitLine $Dir @("rev-parse", "--path-format=absolute", "--git-dir")
        $cd = Get-GitLine $Dir @("rev-parse", "--path-format=absolute", "--git-common-dir")
        if (-not $gd -or -not $cd) { return $Dir }
        $gd = ConvertTo-NormPath $gd
        $cd = ConvertTo-NormPath $cd
        if ($gd -ieq $cd) {
            # Session is in the main checkout itself (also covers --separate-git-dir).
            $top = Get-GitLine $Dir @("rev-parse", "--show-toplevel")
            if (-not $top) { return $Dir }
            return ConvertTo-NormPath $top
        }
        # Linked worktree: the common dir must be "<main>\.git" and <main> a real main checkout of it.
        if ((Split-Path -Leaf $cd) -ne ".git") { return $Dir }
        $parent = Split-Path -Parent $cd
        if (Test-MainCheckout $parent $cd) { return $parent }
        return $Dir
    } catch { return $Dir }
}
# Identity of the session: Branch and Worktree (its top-level folder, null when unknown).
# Branch is null when HEAD is detached or unknown. In a repository without any commit
# "symbolic-ref" still names the unborn branch (e.g. master), so that case filters by branch.
function Get-SessionId([string]$Dir) {
    $branch = Get-GitLine $Dir @("symbolic-ref", "--short", "-q", "HEAD")
    $top = Get-GitLine $Dir @("rev-parse", "--show-toplevel")
    if ($top) { try { $top = ConvertTo-NormPath $top } catch { $top = $null } }
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

# Multi-line git output as text (stderr included), never a terminating error.
function Invoke-Git([string[]]$GitArgs) {
    $ErrorActionPreference = "Continue"
    try { (& git @GitArgs 2>&1 | ForEach-Object { [string]$_ } | Out-String).TrimEnd() } catch { "(git unavailable)" }
}
# Short, stable tag of a worktree folder (8 hex chars of its normalized path), so two sessions on detached
# HEAD (both named "HEAD") never get the same file name in the same second.
function Get-WorktreeTag([string]$Path) {
    $sha = [Security.Cryptography.SHA1]::Create()
    $bytes = $sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($Path.ToLowerInvariant()))
    return (-join ($bytes[0..3] | ForEach-Object { $_.ToString("x2") }))
}

$root = Get-GitLine (Get-Location).Path @("rev-parse", "--show-toplevel")
if (-not $root) { [Console]::Error.WriteLine("journal-skeleton: not inside a git repository"); exit 1 }
$root = ConvertTo-NormPath $root
Set-Location -LiteralPath $root
$mainRoot = Get-MainRoot $root

$safeReason = ($Reason -replace '[^A-Za-z0-9_-]', '-')
$sessionId = Get-SessionId $root
$worktree = if ($sessionId.Worktree) { $sessionId.Worktree } else { $root }
$branch = if ($sessionId.Branch) { $sessionId.Branch } else { "HEAD" }
$safeBranch = ($branch -replace '[^A-Za-z0-9_-]', '-').Trim('-')
if (-not $safeBranch) { $safeBranch = "nobranch" }
$tag = Get-WorktreeTag $worktree
$dir = Join-Path $mainRoot ".journal"
if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Path $dir | Out-Null }
# Keep the diary out of git status even when the repository ignore file lacks a rule. Never overwrites.
$ignoreFile = Join-Path $dir ".gitignore"
if (-not (Test-Path -LiteralPath $ignoreFile)) { [System.IO.File]::WriteAllText($ignoreFile, "*`n", (New-Object System.Text.UTF8Encoding($false))) }

$stamp = Get-Date -Format "yyyy-MM-dd-HHmmss"
$file = Join-Path $dir "$stamp-$safeBranch-$tag-$safeReason.md"
if (Test-Path -LiteralPath $file) { [Console]::Error.WriteLine("journal-skeleton: $file already exists"); exit 1 }

# A repository without any commit has no HEAD to abbreviate: record "unborn".
$short = Get-GitLine $root @("rev-parse", "--short", "HEAD")
if (-not $short) { $short = "unborn" }
$status = Invoke-Git @("status", "-sb")
$changed = @(Get-GitLines $root @("status", "--porcelain")).Count
$log = Invoke-Git @("log", "--oneline", "-n", "8")
$worktrees = Invoke-Git @("worktree", "list")
$fence = '```'
$when = Get-Date -Format "yyyy-MM-dd HH:mm"

$body = @"
# Context diary - $when - $safeReason

## Facts collected automatically
- Branch: $branch @ $short
- Worktree: $worktree
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
