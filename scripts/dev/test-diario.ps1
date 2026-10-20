# Test for the context-diary hooks (pre-compact.ps1, session-start.ps1, journal-skeleton.ps1).
# Simulated input only. Every folder it touches is a fresh temp folder created here; nothing is deleted.
# Usage: powershell -NoProfile -File scripts/dev/test-diario.ps1   -> exit 0 when all cases pass, 1 otherwise.
# The hook input format of a real Claude Code compaction is UNVERIFIED; these cases cover the documented
# "trigger" field, the "compaction_type" fallback, and inputs that must never block.
$ErrorActionPreference = "Stop"
$scripts = $PSScriptRoot
$pre = Join-Path $scripts "pre-compact.ps1"
$fail = 0

function New-TempRepo {
    $t = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $t | Out-Null
    git -C $t init -q
    git -C $t -c user.name=t -c user.email=t@t commit -q --allow-empty -m init
    return $t
}
function New-TempPlainDir {
    $t = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $t | Out-Null
    return $t
}
function Invoke-Hook($script, $json) {
    # The hook writes its refusal to stderr; do not let that become a terminating error here.
    $ErrorActionPreference = "Continue"
    $json | powershell -NoProfile -ExecutionPolicy Bypass -File $script 2>&1 | Out-Null
    return $LASTEXITCODE
}
function Check($name, $got, $want) {
    if ($got -eq $want) { Write-Output "PASS  $name (exit $got)" }
    else { Write-Output "FAILED  $name (expected $want, got $got)"; $script:fail = 1 }
}

$repo = New-TempRepo
$env:CLAUDE_PROJECT_DIR = $repo

Check "trigger manual with an unfilled diary is blocked" (Invoke-Hook $pre '{"hook_event_name":"PreCompact","trigger":"manual"}') 2
Check "trigger auto is never blocked" (Invoke-Hook $pre '{"hook_event_name":"PreCompact","trigger":"auto"}') 0
Check "compaction_type manual (fallback field) is blocked" (Invoke-Hook $pre '{"compaction_type":"manual"}') 2
Check "compaction_type auto (fallback field) is never blocked" (Invoke-Hook $pre '{"compaction_type":"auto"}') 0
Check "unknown type (no field) never blocks" (Invoke-Hook $pre '{}') 0
Check "unreadable stdin never blocks" (Invoke-Hook $pre "not json") 0

$diary = Get-ChildItem (Join-Path $repo ".journal") -Filter "*.md" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$text = [IO.File]::ReadAllText($diary.FullName).Replace("<!-- TODO-CONTEXT -->", "filled by the test")
[IO.File]::WriteAllText($diary.FullName, $text)

Check "trigger manual with a complete diary passes" (Invoke-Hook $pre '{"trigger":"manual"}') 0

$out = @(powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scripts "session-start.ps1"))
Check "session-start re-injects the newest diary" ([int](($out | Select-Object -First 1) -match "Latest context diary")) 1

# Worktree cases: the diary must land in the MAIN checkout, named after the worktree branch.
# The temp repository and worktree are left behind on purpose (nothing is removed).
$mainRepo = New-TempRepo
$wt = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-wt-" + [guid]::NewGuid().ToString("N"))
git -C $mainRepo worktree add -q -b "feat/wt-teste" $wt
$env:CLAUDE_PROJECT_DIR = $wt
Check "worktree: manual pre-compact on an unfilled diary is blocked" (Invoke-Hook $pre '{"trigger":"manual"}') 2
$inMain = @(Get-ChildItem (Join-Path $mainRepo ".journal") -Filter "*.md" -ErrorAction SilentlyContinue)
Check "worktree: the diary is created in the main checkout .journal" ([int]($inMain.Count -eq 1)) 1
Check "worktree: nothing is created in the worktree .journal" ([int](Test-Path -LiteralPath (Join-Path $wt ".journal"))) 0
Check "worktree: the diary name contains the worktree branch" ([int]($inMain.Count -ge 1 -and $inMain[0].Name -like "*-feat-wt-teste-*")) 1
$wtOut = @(powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scripts "session-start.ps1"))
$wtFirst = [string]($wtOut | Select-Object -First 1)
Check "worktree: session-start re-injects the main checkout diary" ([int](($wtFirst -match "Latest context diary") -and $wtFirst.Contains((Join-Path $mainRepo ".journal")))) 1

$env:CLAUDE_PROJECT_DIR = New-TempPlainDir
Check "a folder that is not a git repository does not crash or block" (Invoke-Hook $pre '{"trigger":"manual"}') 0

Remove-Item Env:CLAUDE_PROJECT_DIR
exit $fail
