# PreCompact hook. Enforces the "context diary before compaction" rule (CLAUDE.md).
# manual /compact: blocks (exit 2) until the newest diary (< 30 min) has no TODO-CONTEXT markers left.
# auto compaction (or unknown type): never blocks (it could stall the session); makes sure a skeleton with automatic facts exists.
# Reads the hook JSON from stdin; only an explicit "manual" blocks. An absent, unreadable or unknown type never blocks.
# A session with neither a branch nor a worktree (not a git folder) never blocks.
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
    # The skeleton reports its own failures on stderr; never let them end this hook with a terminating error.
    $skeleton = Join-Path $PSScriptRoot "journal-skeleton.ps1"
    $latest = $null
    try {
        $ErrorActionPreference = "Continue"
        $made = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $skeleton "pre-compact-$type" 2>$null)
        if ($LASTEXITCODE -eq 0 -and $made.Count -gt 0) { $latest = ([string]$made[-1]).Trim() }
    } catch { $latest = $null } finally { $ErrorActionPreference = "Stop" }
    if ($latest -and -not (Test-Path -LiteralPath $latest)) { $latest = $null }
}

if ($latest -and (Select-String -LiteralPath $latest -SimpleMatch "<!-- TODO-CONTEXT -->" -Quiet)) {
    if ($type -eq "manual" -and ($sessionId.Branch -or $sessionId.Worktree)) {
        [Console]::Error.WriteLine("Context diary incomplete: $latest still has '<!-- TODO-CONTEXT -->' markers. Ask the agent to fill every section (and remove the markers), then run /compact again.")
        exit 2
    }
    [Console]::Error.WriteLine("Auto-compaction proceeding; diary skeleton at $latest has only automatic facts.")
}
exit 0
