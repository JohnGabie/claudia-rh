# Test for the pre-push hook (T12): real git, real hook files.
# Builds, under fresh temp folders created here, a work repository plus two bare repositories playing the roles of
# the public origin (folder named claudia-rh.git) and the backup (claudia-rh-backup.git). Pushes go ONLY to those
# temp bares. core.hooksPath points to a hooks folder INSIDE the temp work repository, where the hook files are copied.
# Nothing is deleted or reset; the temp folders are left behind. Never touches the real remotes of any repository.
# Usage: powershell -NoProfile -File scripts/dev/test-pre-push.ps1 [-HooksDir <folder with pre-push + pre-push.ps1>]
#   -> exit 0 when all cases pass, 1 otherwise. -HooksDir exists so mutants (copies in a temp folder) can be tested.
param([string]$HooksDir = (Join-Path $PSScriptRoot "git-hooks"),
      [string]$AttributesFile = (Join-Path (Join-Path $PSScriptRoot "..\..") ".gitattributes"))
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

# Runs the sh launcher directly with a URL and simulated stdin (no real push needed). PowerShell stays available
# unless -Path is given (then PATH holds only the Git folders: the sh fallback decides).
function Invoke-Launcher([string]$Url, [string]$Stdin, [string]$Path = "") {
    $sh = Join-Path $GitRoot "usr\bin\sh.exe"
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $sh
    $psi.Arguments = (@((Join-Path $script:hooks "pre-push").Replace("\", "/"), "origin", $Url) | ForEach-Object { ConvertTo-Arg $_ }) -join " "
    $psi.WorkingDirectory = $script:W
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    if ($Path) { $psi.EnvironmentVariables["PATH"] = $Path }
    else { $psi.EnvironmentVariables["PATH"] = (Join-Path $GitRoot "usr\bin") + ";" + $env:PATH }
    $p = [System.Diagnostics.Process]::Start($psi)
    $o = $p.StandardOutput.ReadToEndAsync()
    $e = $p.StandardError.ReadToEndAsync()
    $p.StandardInput.Write($Stdin)
    $p.StandardInput.Close()
    $p.WaitForExit()
    return [pscustomobject]@{ Code = $p.ExitCode; Out = $o.Result; Err = $e.Result }
}

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

# --- URL normalization (T19): the launcher is called directly with a URL and a simulated "main" update line ---
$zeros = "0" * 40
$mainLine = "refs/heads/main $head refs/heads/main $zeros`n"
$publicUrls = @(
    "https://github.com/JohnGabie/claudia-rh/.",
    "https://github.com/JohnGabie/claudia-rh/x/..",
    "https://github.com/JohnGabie/claudia%2Drh",
    "https://github.com/JohnGabie/Claudia-RH.git/",
    "git@github.com:JohnGabie/claudia-rh.git",
    "ssh://git@github.com/JohnGabie/claudia-rh.git",
    "https://x:y@github.com/JohnGabie/claudia-rh",
    "https://github.com/JohnGabie/claudia-rh.git/./",
    "https://github.com/JohnGabie/claudia%252Drh",
    "https://github.com/..",
    ""
)
foreach ($u in $publicUrls) {
    $r = Invoke-Launcher $u $mainLine
    Assert-Case "url public (PowerShell logic): '$u' -> main refused" (($r.Code -eq 1) -and ($r.Err -match "REFUSED")) "exit $($r.Code) stderr: $($r.Err)"
}
foreach ($u in @("https://github.com/JohnGabie/claudia-rh-backup.git", "https://github.com/JohnGabie/claudia%2Drh-backup", "https://x:y@github.com/JohnGabie/claudia-rh-backup/")) {
    $r = Invoke-Launcher $u $mainLine
    Assert-Case "url not public (PowerShell logic): '$u' -> main passes" ($r.Code -eq 0) "exit $($r.Code) stderr: $($r.Err)"
}
# Same URLs with PowerShell unavailable: the sh fallback must reach the same verdict (fail closed).
foreach ($u in $publicUrls) {
    $r = Invoke-Launcher $u $mainLine $cleanPath
    Assert-Case "url public (sh fallback): '$u' -> refused" (($r.Code -eq 1) -and ($r.Err -match "REFUSED")) "exit $($r.Code) stderr: $($r.Err)"
}
foreach ($u in @("https://github.com/JohnGabie/claudia-rh-backup.git", "https://x:y@github.com/JohnGabie/claudia-rh-backup/")) {
    $r = Invoke-Launcher $u $mainLine $cleanPath
    Assert-Case "url not public (sh fallback): '$u' -> passes" ($r.Code -eq 0) "exit $($r.Code) stderr: $($r.Err)"
}

# --- Force push (T19): feat/ff is created, fast-forwarded, then rewritten ---
function Get-BareSha([string]$bare, [string]$ref) { return (Invoke-Git $bare @("rev-parse", "--verify", "-q", $ref)).Out.Trim() }
$tree = (G @("rev-parse", "HEAD^{tree}")).Out.Trim()
$c2 = (G @("commit-tree", $tree, "-p", $head, "-m", "c2")).Out.Trim()
$c3 = (G @("commit-tree", $tree, "-p", $head, "-m", "c3-sibling")).Out.Trim()
G @("update-ref", "refs/heads/feat/ff", $head) | Out-Null
Assert-Pass "origin: creating feat/ff passes" @("push", "origin", "feat/ff") $pubBare @("refs/heads/feat/ff") | Out-Null
G @("update-ref", "refs/heads/feat/ff", $c2) | Out-Null
Assert-Pass "origin: fast-forward of feat/ff passes" @("push", "origin", "feat/ff") $pubBare @("refs/heads/feat/ff") | Out-Null
Assert-Case "origin: bare feat/ff is now c2" ((Get-BareSha $pubBare "refs/heads/feat/ff") -eq $c2) "bare has $(Get-BareSha $pubBare 'refs/heads/feat/ff')"
G @("update-ref", "refs/heads/feat/ff", $c3) | Out-Null
Assert-Refuse "origin: force push to a sibling commit refused" @("push", "-f", "origin", "feat/ff") $pubBare @() "force push" | Out-Null
Assert-Case "origin: bare feat/ff still c2 after the rewrite" ((Get-BareSha $pubBare "refs/heads/feat/ff") -eq $c2) "bare has $(Get-BareSha $pubBare 'refs/heads/feat/ff')"
G @("update-ref", "refs/heads/feat/ff", $head) | Out-Null
Assert-Refuse "origin: force push back to an older commit refused" @("push", "-f", "origin", "feat/ff") $pubBare @() "force push" | Out-Null
Assert-Case "origin: bare feat/ff still c2 after the reset" ((Get-BareSha $pubBare "refs/heads/feat/ff") -eq $c2) "bare has $(Get-BareSha $pubBare 'refs/heads/feat/ff')"
# Old remote object missing locally: a second, unrelated work repository that never fetched c2.
$W2 = Join-Path $root "work2"
New-Item -ItemType Directory -Path $W2 | Out-Null
Invoke-Git $W2 @("init", "-q") | Out-Null
Set-Content -LiteralPath (Join-Path $W2 "b.txt") -Value "other"
Invoke-Git $W2 @("add", "b.txt") | Out-Null
Invoke-Git $W2 @("commit", "-q", "-m", "other") | Out-Null
Invoke-Git $W2 @("branch", "feat/ff") | Out-Null
Invoke-Git $W2 @("config", "core.hooksPath", $hooks.Replace("\", "/")) | Out-Null
Invoke-Git $W2 @("remote", "add", "origin", $pubBare.Replace("\", "/")) | Out-Null
$mainWork = $script:W
$script:W = $W2
Assert-Refuse "origin: old remote object missing locally refused, asks for git fetch" @("push", "-f", "origin", "feat/ff") $pubBare @() "git fetch origin" | Out-Null
Assert-Case "origin: bare feat/ff still c2 after the missing-object push" ((Get-BareSha $pubBare "refs/heads/feat/ff") -eq $c2) "bare has $(Get-BareSha $pubBare 'refs/heads/feat/ff')"
$script:W = $mainWork
# The backup takes the same force push.
G @("update-ref", "refs/heads/feat/ff", $c2) | Out-Null
Assert-Pass "backup: feat/ff c2 passes" @("push", "backup", "feat/ff") $bkpBare @("refs/heads/feat/ff") | Out-Null
G @("update-ref", "refs/heads/feat/ff", $c3) | Out-Null
Assert-Pass "backup: force push of feat/ff passes" @("push", "-f", "backup", "feat/ff") $bkpBare @("refs/heads/feat/ff") | Out-Null
Assert-Case "backup: bare feat/ff is now c3" ((Get-BareSha $bkpBare "refs/heads/feat/ff") -eq $c3) "bare has $(Get-BareSha $bkpBare 'refs/heads/feat/ff')"
# The escape hatch still frees the force push on the public remote.
Assert-Pass "origin: CLAUDIA_ALLOW_PUBLIC_PUSH=1 lets the force push through" @("push", "-f", "origin", "feat/ff") $pubBare @("refs/heads/feat/ff") @{ CLAUDIA_ALLOW_PUBLIC_PUSH = "1" } | Out-Null
Assert-Case "origin: bare feat/ff is c3 after the escape hatch" ((Get-BareSha $pubBare "refs/heads/feat/ff") -eq $c3) "bare has $(Get-BareSha $pubBare 'refs/heads/feat/ff')"

# --- Published tags are never moved or recreated (T19, round 2) ---
G @("tag", "v1.0.0", $head) | Out-Null
Assert-Pass "origin: new lightweight tag v1.0.0 passes" @("push", "origin", "refs/tags/v1.0.0") $pubBare @("refs/tags/v1.0.0") | Out-Null
G @("tag", "-f", "v1.0.0", $c2) | Out-Null
$before = Get-BareSha $pubBare "refs/tags/v1.0.0"
Assert-Refuse "origin: lightweight tag moved forward refused" @("push", "-f", "origin", "refs/tags/v1.0.0") $pubBare @() "tags are never moved or recreated" | Out-Null
Assert-Case "origin: bare v1.0.0 object unchanged" ((Get-BareSha $pubBare "refs/tags/v1.0.0") -eq $before) "bare has $(Get-BareSha $pubBare 'refs/tags/v1.0.0')"
G @("tag", "-a", "v2.0.0", "-m", "first", $c2) | Out-Null
Assert-Pass "origin: new annotated tag v2.0.0 passes" @("push", "origin", "refs/tags/v2.0.0") $pubBare @("refs/tags/v2.0.0") | Out-Null
G @("tag", "-f", "-a", "v2.0.0", "-m", "second", $c2) | Out-Null
$before2 = Get-BareSha $pubBare "refs/tags/v2.0.0"
Assert-Case "local v2.0.0 was really recreated (different object)" ((G @("rev-parse", "refs/tags/v2.0.0")).Out.Trim() -ne $before2) "same object"
Assert-Refuse "origin: annotated tag recreated on the same commit refused" @("push", "-f", "origin", "refs/tags/v2.0.0") $pubBare @() "tags are never moved or recreated" | Out-Null
Assert-Case "origin: bare v2.0.0 object unchanged" ((Get-BareSha $pubBare "refs/tags/v2.0.0") -eq $before2) "bare has $(Get-BareSha $pubBare 'refs/tags/v2.0.0')"
G @("tag", "v9.9.9", $head) | Out-Null
Assert-Pass "origin: brand new tag v9.9.9 passes" @("push", "origin", "refs/tags/v9.9.9") $pubBare @("refs/tags/v9.9.9") | Out-Null
# The backup takes the same movement of a tag.
G @("tag", "-f", "v1.0.0", $head) | Out-Null
Assert-Pass "backup: tag v1.0.0 (old position) passes" @("push", "-f", "backup", "refs/tags/v1.0.0") $bkpBare @("refs/tags/v1.0.0") | Out-Null
G @("tag", "-f", "v1.0.0", $c2) | Out-Null
Assert-Pass "backup: moving tag v1.0.0 forward passes" @("push", "-f", "backup", "refs/tags/v1.0.0") $bkpBare @("refs/tags/v1.0.0") | Out-Null
Assert-Case "backup: bare v1.0.0 is c2" ((Get-BareSha $bkpBare "refs/tags/v1.0.0") -eq $c2) "bare has $(Get-BareSha $bkpBare 'refs/tags/v1.0.0')"

# --- Checkout with core.autocrlf=true: the sh launcher must stay LF and the hook must still run ---
# Source repo holds the hook files and the repository .gitattributes; the clone is made with autocrlf=true.
$src = Join-Path $root "crlf-src"
$clone = Join-Path $root "crlf-clone"
$pub2 = Join-Path (Join-Path $root "pub2") "claudia-rh.git"
foreach ($d in @($src, $pub2)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
Invoke-Git $pub2 @("init", "-q", "--bare") | Out-Null
Invoke-Git $src @("init", "-q") | Out-Null
$srcHooks = Join-Path $src "scripts\dev\git-hooks"
New-Item -ItemType Directory -Path $srcHooks -Force | Out-Null
foreach ($f in @("pre-push", "pre-push.ps1")) { Copy-Item -LiteralPath (Join-Path $HooksDir $f) -Destination (Join-Path $srcHooks $f) }
if (Test-Path -LiteralPath $AttributesFile) { Copy-Item -LiteralPath $AttributesFile -Destination (Join-Path $src ".gitattributes") }
Invoke-Git $src @("add", "-A") | Out-Null
Invoke-Git $src @("commit", "-q", "-m", "hooks") | Out-Null
$cl = Invoke-Git $root @("clone", "-q", "-c", "core.autocrlf=true", $src, $clone)
$script:W = $clone
$crBytes = -1
$launcher = Join-Path $clone "scripts\dev\git-hooks\pre-push"
if (Test-Path -LiteralPath $launcher) { $crBytes = @([IO.File]::ReadAllBytes($launcher) | Where-Object { $_ -eq 13 }).Count }
Assert-Case "autocrlf clone: pre-push launcher has 0 CR bytes" ($crBytes -eq 0) "CR bytes: $crBytes (clone exit $($cl.Code) $($cl.Err))"
Invoke-Git $clone @("config", "core.hooksPath", (Join-Path $clone "scripts/dev/git-hooks").Replace("\", "/")) | Out-Null
Invoke-Git $clone @("remote", "set-url", "origin", $pub2.Replace("\", "/")) | Out-Null
Invoke-Git $clone @("branch", "feat/x") | Out-Null
Assert-Pass "autocrlf clone: hook runs, feat/x passes" @("push", "origin", "feat/x") $pub2 @("refs/heads/feat/x") | Out-Null
Assert-Refuse "autocrlf clone: hook runs, main refused" @("push", "origin", "main") $pub2 @("refs/heads/main") "refs/heads/main" | Out-Null

if ($fail -eq 0) { Write-Output "ALL PASS" } else { Write-Output "SOME CASES FAILED" }
exit $fail
