# Test for the context-diary hooks (pre-compact.ps1, session-start.ps1, journal-skeleton.ps1).
# Simulated input only. Every repo it touches is a fresh temp folder created here; nothing is deleted.
# Usage: powershell -NoProfile -File scripts/dev/test-diario.ps1   -> exit 0 when all cases pass, 1 otherwise.
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

Check "manual compaction with an unfilled diary is blocked" (Invoke-Hook $pre '{"compaction_type":"manual"}') 2
Check "auto compaction is never blocked" (Invoke-Hook $pre '{"compaction_type":"auto"}') 0

$diary = Get-ChildItem (Join-Path $repo ".journal") -Filter "*.md" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$text = [IO.File]::ReadAllText($diary.FullName).Replace("<!-- TODO-CONTEXT -->", "filled by the test")
[IO.File]::WriteAllText($diary.FullName, $text)

Check "manual compaction with a complete diary passes" (Invoke-Hook $pre '{"compaction_type":"manual"}') 0
Check "unreadable stdin falls back to manual and still passes with a complete diary" (Invoke-Hook $pre "not json") 0

$out = @(powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scripts "session-start.ps1"))
Check "session-start re-injects the newest diary" ([int](($out | Select-Object -First 1) -match "Latest context diary")) 1

Remove-Item Env:CLAUDE_PROJECT_DIR
exit $fail
