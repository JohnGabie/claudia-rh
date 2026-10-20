# SessionStart hook (source: compact or resume). Re-injects the newest context diary so work can continue.
# Only reads files. Never deletes anything.
$ErrorActionPreference = "Stop"

$projectDir = $env:CLAUDE_PROJECT_DIR
if (-not $projectDir) { $projectDir = (Get-Location).Path }
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
if (-not (Test-Path -LiteralPath $journalDir)) { exit 0 }

$sessionBranch = Get-SessionBranch $projectDir
$newest = Get-ChildItem -LiteralPath $journalDir -Filter "*.md" -File |
    Sort-Object LastWriteTime -Descending | Where-Object { Test-DiaryBranch $_.FullName $sessionBranch } | Select-Object -First 1
if (-not $newest) { exit 0 }

[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
Write-Output "== Latest context diary ($($newest.FullName)) - read it, then continue from 'Next steps'; do not redo 'Do NOT redo' items =="
Get-Content -LiteralPath $newest.FullName -Encoding UTF8 -TotalCount 220
exit 0
