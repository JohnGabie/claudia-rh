# Test for verify-backup.ps1 (P12 backup invariant report).
# Builds, under fresh temp folders created here, a work repository and local bare repositories playing the
# roles of origin and backup. Pushes go ONLY to those temp bare repositories. Nothing is deleted or reset:
# to "remove" a ref in a scenario, the scenario simply gets a new pair of bare repositories.
# Never touches the real origin/backup remotes of any repository.
# Usage: powershell -NoProfile -File scripts/dev/test-verify-backup.ps1 [-ScriptPath <verify-backup.ps1 copy>]
#   -> exit 0 when all cases pass, 1 otherwise. -ScriptPath exists so mutants can be tested.
param([string]$ScriptPath = (Join-Path $PSScriptRoot "verify-backup.ps1"))
$ErrorActionPreference = "Stop"
$fail = 0

trap {
    Write-Output "FAILED  unexpected exception (the rest of that step was skipped): $($_.Exception.Message)"
    $script:fail = 1
    continue
}

function New-TempPath([string]$prefix) {
    return (Join-Path ([IO.Path]::GetTempPath()) ($prefix + [guid]::NewGuid().ToString("N")))
}
# Runs git; returns stdout lines. stderr is dropped (PS 5.1 would turn it into a terminating error).
function Invoke-G {
    $ErrorActionPreference = "Continue"
    $out = @(& git -c user.name=t -c user.email=t@t -c init.defaultBranch=main -c advice.detachedHead=false @args 2>$null)
    return ,@($out | ForEach-Object { [string]$_ })
}
function New-Bare {
    $p = New-TempPath "vb-bare-"
    New-Item -ItemType Directory -Path $p | Out-Null
    Invoke-G init -q --bare $p | Out-Null
    return $p
}
function Add-Commit([string]$repo, [string]$file, [string]$text) {
    Set-Content -LiteralPath (Join-Path $repo $file) -Value $text
    Invoke-G -C $repo add $file | Out-Null
    Invoke-G -C $repo commit -q -m $text | Out-Null
    return (Invoke-G -C $repo rev-parse HEAD)[0]
}
function Push-To([string]$bare, [string]$refspec) {
    Invoke-G -C $script:W push -q $bare $refspec | Out-Null
}
function Set-Remotes([string]$o, [string]$b) {
    foreach ($pair in @(@("origin", $o), @("backup", $b))) {
        $has = Invoke-G -C $script:W remote
        if ($has -contains $pair[0]) { Invoke-G -C $script:W remote set-url $pair[0] $pair[1] | Out-Null }
        else { Invoke-G -C $script:W remote add $pair[0] $pair[1] | Out-Null }
    }
}
# Runs the script under test with default remote names inside the work repository.
function Invoke-Verify {
    $ErrorActionPreference = "Continue"
    $text = (& powershell -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoDir $script:W 2>&1 | ForEach-Object { [string]$_ }) -join "`n"
    return [pscustomobject]@{ Code = $LASTEXITCODE; Text = $text }
}
function Assert-Case([string]$name, [bool]$ok, [string]$why) {
    if ($ok) { Write-Output "PASS    $name" }
    else { Write-Output "FAILED  $name :: $why"; $script:fail = 1 }
}
function Assert-Status($r, [string]$name, [string]$status, [string]$ref) {
    $re = "(?m)^" + [regex]::Escape($status) + "\s+" + [regex]::Escape($ref) + "\s"
    Assert-Case $name ($r.Text -match $re) "expected '$status $ref' in output:`n$($r.Text)"
}
function Assert-Exit($r, [string]$name, [int]$code) {
    Assert-Case $name ($r.Code -eq $code) "expected exit $code, got $($r.Code). Output:`n$($r.Text)"
}

# --- Work repository: c1 <- c2 on main; "other" branches off c1 with d1 (diverges from main) ---
$W = New-TempPath "vb-work-"
New-Item -ItemType Directory -Path $W | Out-Null
Invoke-G init -q $W | Out-Null
$c1 = Add-Commit $W "a.txt" "c1"
$c2 = Add-Commit $W "a.txt" "c2"
Invoke-G -C $W checkout -q -b other $c1 | Out-Null
$d1 = Add-Commit $W "b.txt" "d1"
Invoke-G -C $W checkout -q main | Out-Null
Invoke-G -C $W tag v1 $c1 | Out-Null
Invoke-G -C $W tag -a v2 -m "annotated v2" $c1 | Out-Null
Invoke-G -C $W tag -a v2b -m "second tag object, same commit" $c1 | Out-Null

# 1. Backup equal to origin (branches, lightweight and annotated tags) -> exit 0
$o = New-Bare; $b = New-Bare
foreach ($t in @($o, $b)) { Push-To $t "main:refs/heads/main"; Push-To $t "refs/tags/v1"; Push-To $t "refs/tags/v2" }
Set-Remotes $o $b
$r = Invoke-Verify
Assert-Exit $r "1 equal: exit 0" 0
Assert-Status $r "1 equal: branch OK" "OK" "refs/heads/main"
Assert-Status $r "1 equal: lightweight tag OK" "OK" "refs/tags/v1"
Assert-Status $r "1 equal: annotated tag OK" "OK" "refs/tags/v2"

# 2. Backup ahead on the same branch -> OK-ANCESTRAL, exit 0
$o = New-Bare; $b = New-Bare
Push-To $o "${c1}:refs/heads/main"; Push-To $b "${c2}:refs/heads/main"
Set-Remotes $o $b
$r = Invoke-Verify
Assert-Exit $r "2 backup ahead: exit 0" 0
Assert-Status $r "2 backup ahead: OK-ANCESTRAL" "OK-ANCESTRAL" "refs/heads/main"

# 3. Origin branch missing on the backup -> FALTA, exit 1
$o = New-Bare; $b = New-Bare
Push-To $o "main:refs/heads/main"; Push-To $o "other:refs/heads/other"; Push-To $b "main:refs/heads/main"
Set-Remotes $o $b
$r = Invoke-Verify
Assert-Exit $r "3 missing branch: exit 1" 1
Assert-Status $r "3 missing branch: FALTA" "FALTA" "refs/heads/other"
Assert-Status $r "3 missing branch: other branch still OK" "OK" "refs/heads/main"

# 4. Origin tag missing on the backup -> FALTA, exit 1
$o = New-Bare; $b = New-Bare
Push-To $o "main:refs/heads/main"; Push-To $b "main:refs/heads/main"; Push-To $o "refs/tags/v1"
Set-Remotes $o $b
$r = Invoke-Verify
Assert-Exit $r "4 missing tag: exit 1" 1
Assert-Status $r "4 missing tag: FALTA" "FALTA" "refs/tags/v1"

# 5. Backup behind -> ATRASADO, exit 1
$o = New-Bare; $b = New-Bare
Push-To $o "${c2}:refs/heads/main"; Push-To $b "${c1}:refs/heads/main"
Set-Remotes $o $b
$r = Invoke-Verify
Assert-Exit $r "5 backup behind: exit 1" 1
Assert-Status $r "5 backup behind: ATRASADO" "ATRASADO" "refs/heads/main"

# 6. Diverged histories -> DIVERGE, exit 1
$o = New-Bare; $b = New-Bare
Push-To $o "${c2}:refs/heads/main"; Push-To $b "${d1}:refs/heads/main"
Set-Remotes $o $b
$r = Invoke-Verify
Assert-Exit $r "6 diverged: exit 1" 1
Assert-Status $r "6 diverged: DIVERGE" "DIVERGE" "refs/heads/main"

# 7. Ref only on the backup -> listed, does not change the exit code
$o = New-Bare; $b = New-Bare
Push-To $o "main:refs/heads/main"; Push-To $b "main:refs/heads/main"; Push-To $b "other:refs/heads/recovery"
Set-Remotes $o $b
$r = Invoke-Verify
Assert-Exit $r "7 only on backup: exit 0" 0
Assert-Status $r "7 only on backup: listed" "SO-BACKUP" "refs/heads/recovery"

# 8. Nonexistent remote -> exit 2 with a message
$o = New-Bare
Set-Remotes $o (New-TempPath "vb-nao-existe-")
$r = Invoke-Verify
Assert-Exit $r "8 missing remote: exit 2" 2
Assert-Case "8 missing remote: message" ($r.Text -match "ERRO") "expected an ERRO message, got:`n$($r.Text)"

# 9. Object missing locally (commit made in a separate clone, pushed only to the backup) -> NAO-VERIFICAVEL, exit 1
$o = New-Bare; $b = New-Bare
Push-To $o "${c1}:refs/heads/main"; Push-To $b "${c1}:refs/heads/main"
$clone = New-TempPath "vb-clone-"
Invoke-G clone -q $b $clone | Out-Null
$x = Add-Commit $clone "c.txt" "only-in-clone"
Invoke-G -C $clone push -q origin "HEAD:refs/heads/main" | Out-Null
Set-Remotes $o $b
$hasX = (Invoke-G -C $W cat-file -t $x)
Assert-Case "9 setup: work repo lacks the clone commit" ($hasX.Count -eq 0) "the work repo unexpectedly has $x"
$r = Invoke-Verify
Assert-Exit $r "9 missing object: exit 1" 1
Assert-Status $r "9 missing object: NAO-VERIFICAVEL" "NAO-VERIFICAVEL" "refs/heads/main"

# 10. Annotated tag with a different tag object but the same commit -> OK (peeled commit is compared)
$o = New-Bare; $b = New-Bare
Push-To $o "main:refs/heads/main"; Push-To $b "main:refs/heads/main"
Push-To $o "refs/tags/v2:refs/tags/t"; Push-To $b "refs/tags/v2b:refs/tags/t"
Set-Remotes $o $b
$r = Invoke-Verify
Assert-Exit $r "10 annotated tag, same commit: exit 0" 0
Assert-Status $r "10 annotated tag, same commit: OK" "OK" "refs/tags/t"

# 8b. Nonexistent ORIGIN (backup fine) -> exit 2 with a clean message
$b = New-Bare
Set-Remotes (New-TempPath "vb-nao-existe-") $b
$r = Invoke-Verify
Assert-Exit $r "8b missing origin: exit 2" 2
Assert-Case "8b missing origin: message" ($r.Text -match "ERRO") "expected an ERRO message, got:`n$($r.Text)"
Assert-Case "8b missing origin: no RemoteException noise" ($r.Text -notmatch "RemoteException") "noise in:`n$($r.Text)"

# 11. Tags must point to the SAME commit: any difference is TAG-DIFERE, exit 1 (branches may be ahead)
Invoke-G -C $W tag -a a2 -m "annotated at c2" $c2 | Out-Null
# 11a. lightweight tag, backup ahead
$o = New-Bare; $b = New-Bare
Push-To $o "main:refs/heads/main"; Push-To $b "main:refs/heads/main"
Push-To $o "${c1}:refs/tags/t"; Push-To $b "${c2}:refs/tags/t"
Set-Remotes $o $b
$r = Invoke-Verify
Assert-Exit $r "11a lightweight tag, backup ahead: exit 1" 1
Assert-Status $r "11a lightweight tag, backup ahead: TAG-DIFERE" "TAG-DIFERE" "refs/tags/t"
# 11b. annotated tag, backup ahead
$o = New-Bare; $b = New-Bare
Push-To $o "main:refs/heads/main"; Push-To $b "main:refs/heads/main"
Push-To $o "refs/tags/v2:refs/tags/t"; Push-To $b "refs/tags/a2:refs/tags/t"
Set-Remotes $o $b
$r = Invoke-Verify
Assert-Exit $r "11b annotated tag, backup ahead: exit 1" 1
Assert-Status $r "11b annotated tag, backup ahead: TAG-DIFERE" "TAG-DIFERE" "refs/tags/t"
# 11c. tag moved on the origin (backup behind): also TAG-DIFERE
$o = New-Bare; $b = New-Bare
Push-To $o "main:refs/heads/main"; Push-To $b "main:refs/heads/main"
Push-To $o "${c2}:refs/tags/t"; Push-To $b "${c1}:refs/tags/t"
Set-Remotes $o $b
$r = Invoke-Verify
Assert-Exit $r "11c tag moved on origin: exit 1" 1
Assert-Status $r "11c tag moved on origin: TAG-DIFERE" "TAG-DIFERE" "refs/tags/t"
# 11d. branch ahead stays OK-ANCESTRAL while an equal tag is OK
$o = New-Bare; $b = New-Bare
Push-To $o "${c1}:refs/heads/main"; Push-To $b "${c2}:refs/heads/main"
Push-To $o "${c1}:refs/tags/t"; Push-To $b "${c1}:refs/tags/t"
Set-Remotes $o $b
$r = Invoke-Verify
Assert-Exit $r "11d branch ahead + equal tag: exit 0" 0
Assert-Status $r "11d branch ahead + equal tag: branch OK-ANCESTRAL" "OK-ANCESTRAL" "refs/heads/main"
Assert-Status $r "11d branch ahead + equal tag: tag OK" "OK" "refs/tags/t"

# 12. Static check: the no-prompt env vars are set BEFORE the first git call (provoking a real credential
# prompt would open a window on the user's machine, so this is checked on the script text).
$text = Get-Content -Raw -LiteralPath $ScriptPath
$iGit = $text.IndexOf("& git")
$iTerm = $text.IndexOf('$env:GIT_TERMINAL_PROMPT = "0"')
$iGcm = $text.IndexOf('$env:GCM_INTERACTIVE = "never"')
Assert-Case "12 GIT_TERMINAL_PROMPT=0 before first git call" ($iTerm -ge 0 -and $iGit -ge 0 -and $iTerm -lt $iGit) "indexes: env=$iTerm git=$iGit"
Assert-Case "12 GCM_INTERACTIVE=never before first git call" ($iGcm -ge 0 -and $iGit -ge 0 -and $iGcm -lt $iGit) "indexes: env=$iGcm git=$iGit"

if ($fail -eq 0) { Write-Output "ALL PASSED" } else { Write-Output "SOME FAILED" }
exit $fail
