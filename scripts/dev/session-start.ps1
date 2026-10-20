# SessionStart hook (source: compact or resume). Re-injects the newest context diary so work can continue.
# Only reads files. Never deletes anything.
$ErrorActionPreference = "Stop"

$projectDir = $env:CLAUDE_PROJECT_DIR
if (-not $projectDir) { $projectDir = (Get-Location).Path }
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
if (-not ($sessionId.Branch -or $sessionId.Worktree)) { exit 0 }
if (-not (Test-Path -LiteralPath $journalDir)) { exit 0 }

$newest = Get-ChildItem -LiteralPath $journalDir -Filter "*.md" -File |
    Sort-Object LastWriteTime -Descending | Where-Object { Test-DiaryOwner $_.FullName $sessionId } | Select-Object -First 1
if (-not $newest) { exit 0 }

[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
Write-Output "== Latest context diary ($($newest.FullName)) - read it, then continue from 'Next steps'; do not redo 'Do NOT redo' items =="
Get-Content -LiteralPath $newest.FullName -Encoding UTF8 -TotalCount 220
exit 0
