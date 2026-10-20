# SessionStart hook (source: compact or resume). Re-injects the newest context diary so work can continue.
# A session with neither a branch nor a worktree (not a git folder) re-injects nothing.
# Only reads files. Never deletes anything.
$ErrorActionPreference = "Stop"

$projectDir = $env:CLAUDE_PROJECT_DIR
if (-not $projectDir) { $projectDir = (Get-Location).Path }
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
if (-not ($sessionId.Branch -or $sessionId.Worktree)) { exit 0 }
if (-not (Test-Path -LiteralPath $journalDir)) { exit 0 }

$newest = Get-ChildItem -LiteralPath $journalDir -Filter "*.md" -File |
    Sort-Object LastWriteTime -Descending | Where-Object { Test-DiaryOwner $_.FullName $sessionId } | Select-Object -First 1
if (-not $newest) { exit 0 }

[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
Write-Output "== Latest context diary ($($newest.FullName)) - read it, then continue from 'Next steps'; do not redo 'Do NOT redo' items =="
Get-Content -LiteralPath $newest.FullName -Encoding UTF8 -TotalCount 220
exit 0
