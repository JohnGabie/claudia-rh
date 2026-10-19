# SessionStart hook (source: compact or resume). Re-injects the newest context diary so work can continue.
# Only reads files. Never deletes anything.
$ErrorActionPreference = "Stop"

$projectDir = $env:CLAUDE_PROJECT_DIR
if (-not $projectDir) { $projectDir = (Get-Location).Path }
$journalDir = Join-Path $projectDir ".journal"
if (-not (Test-Path -LiteralPath $journalDir)) { exit 0 }

$newest = Get-ChildItem -LiteralPath $journalDir -Filter "*.md" -File |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $newest) { exit 0 }

[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
Write-Output "== Latest context diary ($($newest.FullName)) - read it, then continue from 'Next steps'; do not redo 'Do NOT redo' items =="
Get-Content -LiteralPath $newest.FullName -Encoding UTF8 -TotalCount 220
exit 0
