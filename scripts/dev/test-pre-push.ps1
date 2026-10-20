# Test for the pre-push hook (T12): real git, real hook files.
# Builds, under fresh temp folders created here, a work repository plus two bare repositories playing the roles of
# the public origin (folder named claudia-rh.git) and the backup (claudia-rh-backup.git). Pushes go ONLY to those
# temp bares. core.hooksPath points to a hooks folder INSIDE the temp work repository, where the hook files are copied.
# Nothing is deleted or reset; the temp folders are left behind. Never touches the real remotes of any repository.
# Usage: powershell -NoProfile -File scripts/dev/test-pre-push.ps1 [-HooksDir <folder with pre-push + pre-push.ps1>]
#   -> exit 0 when all cases pass, 1 otherwise. -HooksDir exists so mutants (copies in a temp folder) can be tested.
param([string]$HooksDir = (Join-Path $PSScriptRoot "git-hooks"))
$ErrorActionPreference = "Stop"
$fail = 0

trap {
    Write-Output "FAILED  unexpected exception (the rest of that step was skipped): $($_.Exception.Message)"
    $script:fail = 1
    continue
}

$GitExe = (Get-Command git).Source
$GitCmdDir = Split-Path -Parent $GitExe
$GitRoot = Split-Path -Parent $GitCmdDir

function New-TempPath([string]$prefix) {
    return (Join-Path ([IO.Path]::GetTempPath()) ($prefix + [guid]::NewGuid().ToString("N")))
}
function ConvertTo-Arg([string]$a) {
    if ($a -eq "") { return '""' }
    if ($a -notmatch '[\s"]') { return $a }
    return '"' + ($a -replace '"', '\"') + '"'
}
# Runs git with an optional per-process environment; returns Code, Out and Err (stderr kept apart).
function Invoke-Git([string]$Dir, [string[]]$GitArgs, [hashtable]$Env = @{}, [string]$GitPath = $GitExe) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $GitPath
    $all = @("-c", "user.name=t", "-c", "user.email=t@t", "-c", "init.defaultBranch=main") + $GitArgs
    $psi.Arguments = (($all | ForEach-Object { ConvertTo-Arg $_ }) -join " ")
    $psi.WorkingDirectory = $Dir
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    foreach ($k in $Env.Keys) { $psi.EnvironmentVariables[$k] = [string]$Env[$k] }
    $p = [System.Diagnostics.Process]::Start($psi)
    $o = $p.StandardOutput.ReadToEndAsync()
    $e = $p.StandardError.ReadToEndAsync()
    $p.WaitForExit()
    return [pscustomobject]@{ Code = $p.ExitCode; Out = $o.Result; Err = $e.Result }
}
function G([string[]]$GitArgs) { return (Invoke-Git $script:W $GitArgs) }

function Assert-Case([string]$name, [bool]$ok, [string]$why) {
    # Write-Host, so the helpers can also return their result object without mixing it into the report.
    if ($ok) { Write-Host "PASS    $name" }
    else { Write-Host "FAILED  $name :: $why"; $script:fail = 1 }
}
function Test-BareRef([string]$bare, [string]$ref) {
    $r = Invoke-Git $bare @("rev-parse", "--verify", "-q", $ref)
    return ($r.Code -eq 0)
}
# Push expected to pass: exit 0 and every listed ref now exists in the bare.
function Assert-Pass([string]$name, [string[]]$GitArgs, [string]$bare, [string[]]$refs, [hashtable]$Env = @{}) {
    $r = Invoke-Git $script:W $GitArgs $Env
    $missing = @($refs | Where-Object { -not (Test-BareRef $bare $_) })
    Assert-Case $name (($r.Code -eq 0) -and ($missing.Count -eq 0)) "exit $($r.Code), missing in bare: $($missing -join ',') stderr: $($r.Err)"
    return $r
}
# Push expected to be refused by the hook: exit != 0, message names the ref and the reason, bare did NOT receive the refs.
function Assert-Refuse([string]$name, [string[]]$GitArgs, [string]$bare, [string[]]$absentRefs, [string]$mustMention, [hashtable]$Env = @{}) {
    $r = Invoke-Git $script:W $GitArgs $Env
    $got = @($absentRefs | Where-Object { Test-BareRef $bare $_ })
    $msgOk = ($r.Err -match "REFUSED") -and ($r.Err -match "agents/versao.md") -and ($r.Err.Contains($mustMention))
    Assert-Case $name (($r.Code -ne 0) -and ($got.Count -eq 0) -and $msgOk) "exit $($r.Code), bare received: $($got -join ',') msgOk=$msgOk stderr: $($r.Err)"
    return $r
}

# --- Fixture ---
$root = New-TempPath "prepush-"
$W = Join-Path $root "work"
$pubBare = Join-Path (Join-Path $root "pub") "claudia-rh.git"
$bkpBare = Join-Path (Join-Path $root "bkp") "claudia-rh-backup.git"
foreach ($d in @($W, $pubBare, $bkpBare)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
Invoke-Git $pubBare @("init", "-q", "--bare") | Out-Null
Invoke-Git $bkpBare @("init", "-q", "--bare") | Out-Null
Invoke-Git $W @("init", "-q") | Out-Null
Set-Content -LiteralPath (Join-Path $W "a.txt") -Value "one"
G @("add", "a.txt") | Out-Null
G @("commit", "-q", "-m", "c1") | Out-Null
foreach ($b in @("dev", "feat/x", "fix/y", "sync/dev-2026-10-20", "recovery", "wip/a")) { G @("branch", $b) | Out-Null }
foreach ($t in @("v0.3.0", "archive/x", "pre-rebase/x", "backup/x")) { G @("tag", $t) | Out-Null }
$head = (G @("rev-parse", "HEAD")).Out.Trim()
for ($i = 1; $i -le 21; $i++) { G @("update-ref", "refs/heads/feat/many$i", $head) | Out-Null }
for ($i = 1; $i -le 20; $i++) { G @("update-ref", "refs/heads/fix/lim$i", $head) | Out-Null }

$hooks = Join-Path $W "hooks-under-test"
New-Item -ItemType Directory -Path $hooks | Out-Null
foreach ($f in @("pre-push", "pre-push.ps1")) { Copy-Item -LiteralPath (Join-Path $HooksDir $f) -Destination (Join-Path $hooks $f) }
G @("config", "core.hooksPath", $hooks.Replace("\", "/")) | Out-Null
G @("remote", "add", "origin", $pubBare.Replace("\", "/")) | Out-Null
G @("remote", "add", "backup", $bkpBare.Replace("\", "/")) | Out-Null
Write-Output "fixture: $root"

# --- Public remote: allowed ---
Assert-Pass "origin: feat/x passes" @("push", "origin", "feat/x") $pubBare @("refs/heads/feat/x") | Out-Null
Assert-Pass "origin: fix/y passes" @("push", "origin", "fix/y") $pubBare @("refs/heads/fix/y") | Out-Null
Assert-Pass "origin: sync/dev-2026-10-20 passes" @("push", "origin", "sync/dev-2026-10-20") $pubBare @("refs/heads/sync/dev-2026-10-20") | Out-Null
Assert-Pass "origin: tag v0.3.0 passes" @("push", "origin", "v0.3.0") $pubBare @("refs/tags/v0.3.0") | Out-Null
Assert-Pass "origin: exactly 20 allowed refs pass" @("push", "origin", "refs/heads/fix/lim*:refs/heads/fix/lim*") $pubBare @("refs/heads/fix/lim1", "refs/heads/fix/lim20") | Out-Null

# --- Public remote: refused ---
Assert-Refuse "origin: main refused" @("push", "origin", "main") $pubBare @("refs/heads/main") "refs/heads/main" | Out-Null
Assert-Refuse "origin: dev refused" @("push", "origin", "dev") $pubBare @("refs/heads/dev") "refs/heads/dev" | Out-Null
Assert-Refuse "origin: recovery refused" @("push", "origin", "recovery") $pubBare @("refs/heads/recovery") "refs/heads/recovery" | Out-Null
Assert-Refuse "origin: wip/a refused" @("push", "origin", "wip/a") $pubBare @("refs/heads/wip/a") "refs/heads/wip/a" | Out-Null
Assert-Refuse "origin: tag archive/x refused" @("push", "origin", "refs/tags/archive/x") $pubBare @("refs/tags/archive/x") "refs/tags/archive/x" | Out-Null
Assert-Refuse "origin: tag pre-rebase/x refused" @("push", "origin", "refs/tags/pre-rebase/x") $pubBare @("refs/tags/pre-rebase/x") "refs/tags/pre-rebase/x" | Out-Null
Assert-Refuse "origin: tag backup/x refused" @("push", "origin", "refs/tags/backup/x") $pubBare @("refs/tags/backup/x") "refs/tags/backup/x" | Out-Null
Assert-Refuse "origin: --all refused" @("push", "origin", "--all") $pubBare @("refs/heads/main", "refs/heads/dev", "refs/heads/recovery", "refs/heads/wip/a", "refs/heads/feat/many1") "refs in one push" | Out-Null
Assert-Refuse "origin: --tags with internal tags refused" @("push", "origin", "--tags") $pubBare @("refs/tags/archive/x", "refs/tags/pre-rebase/x", "refs/tags/backup/x") "refs/tags/" | Out-Null
$del = Invoke-Git $W @("push", "origin", ":feat/x")
Assert-Case "origin: deleting feat/x refused and ref kept" (($del.Code -ne 0) -and (Test-BareRef $pubBare "refs/heads/feat/x") -and ($del.Err -match "refs/heads/feat/x") -and ($del.Err -match "deleting")) "exit $($del.Code) stderr: $($del.Err)"
Assert-Refuse "origin: 21 allowed refs refused (limit 20)" @("push", "origin", "refs/heads/feat/many*:refs/heads/feat/many*") $pubBare @("refs/heads/feat/many1", "refs/heads/feat/many21") "21 refs in one push" | Out-Null

# --- Backup passes with no restriction ---
Assert-Pass "backup: main passes" @("push", "backup", "main") $bkpBare @("refs/heads/main") | Out-Null
Assert-Pass "backup: recovery passes" @("push", "backup", "recovery") $bkpBare @("refs/heads/recovery") | Out-Null
Assert-Pass "backup: --all passes" @("push", "backup", "--all") $bkpBare @("refs/heads/dev", "refs/heads/wip/a", "refs/heads/feat/many21") | Out-Null
Assert-Pass "backup: --tags passes" @("push", "backup", "--tags") $bkpBare @("refs/tags/archive/x", "refs/tags/pre-rebase/x", "refs/tags/backup/x") | Out-Null

# --- Escape hatch (set only in the child process) ---
$allow = Assert-Pass "origin: CLAUDIA_ALLOW_PUBLIC_PUSH=1 lets main through" @("push", "origin", "main") $pubBare @("refs/heads/main") @{ CLAUDIA_ALLOW_PUBLIC_PUSH = "1" }
Assert-Case "origin: escape hatch prints a warning" ($allow.Err -match "WARNING") "stderr: $($allow.Err)"

# --- Fail closed ---
G @("branch", "feat/z") | Out-Null
G @("branch", "feat/w") | Out-Null
Assert-Refuse "origin: logic throws -> refused (fail closed)" @("push", "origin", "feat/z") $pubBare @("refs/heads/feat/z") "unexpectedly" @{ CLAUDIA_PREPUSH_TEST_THROW = "1" } | Out-Null
Assert-Pass "backup: logic throws -> still passes" @("push", "backup", "feat/z") $bkpBare @("refs/heads/feat/z") @{ CLAUDIA_PREPUSH_TEST_THROW = "1" } | Out-Null
# PowerShell unavailable: PATH holds only the Git folders (git itself and its sh), no System32\WindowsPowerShell.
$cleanPath = (Join-Path $GitRoot "cmd") + ";" + (Join-Path $GitRoot "usr\bin") + ";" + (Join-Path $GitRoot "mingw64\bin")
Assert-Refuse "origin: PowerShell not found -> refused (fail closed)" @("push", "origin", "feat/w") $pubBare @("refs/heads/feat/w") "could not run" @{ PATH = $cleanPath } | Out-Null
Assert-Pass "backup: PowerShell not found -> still passes" @("push", "backup", "feat/w") $bkpBare @("refs/heads/feat/w") @{ PATH = $cleanPath } | Out-Null

if ($fail -eq 0) { Write-Output "ALL PASS" } else { Write-Output "SOME CASES FAILED" }
exit $fail
