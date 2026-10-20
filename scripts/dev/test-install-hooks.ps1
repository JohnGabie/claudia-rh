# Test for install-hooks.ps1 (T22): real git, real hook files, temp repositories only.
# Every repository, worktree and bare used here is created in fresh temp folders by this script and left behind
# (nothing is deleted or reset). Pushes go ONLY to temp bares named claudia-rh.git. The installer is pointed at the temp
# repositories with -RepoDir, so the real repository and its hooks folder are never touched.
# Usage: powershell -NoProfile -File scripts/dev/test-install-hooks.ps1 [-InstallScript <install-hooks.ps1>] [-HooksSourceDir <folder>]
#   -> exit 0 when all cases pass, 1 otherwise. The parameters exist so mutants (copies in a temp folder) can be tested.
param([string]$InstallScript = (Join-Path $PSScriptRoot "install-hooks.ps1"),
      [string]$HooksSourceDir = (Join-Path $PSScriptRoot "git-hooks"))
$ErrorActionPreference = "Stop"
$fail = 0

trap {
    Write-Output "FAILED  unexpected exception (the rest of that step was skipped): $($_.Exception.Message)"
    $script:fail = 1
    continue
}

$GitExe = (Get-Command git).Source
$PsExe = (Get-Command powershell.exe).Source

function New-TempPath([string]$prefix) {
    return (Join-Path ([IO.Path]::GetTempPath()) ($prefix + [guid]::NewGuid().ToString("N")))
}
function ConvertTo-Arg([string]$a) {
    if ($a -eq "") { return '""' }
    if ($a -notmatch '[\s"]') { return $a }
    return '"' + ($a -replace '"', '\"') + '"'
}
function Invoke-Native([string]$Exe, [string]$Dir, [string[]]$ArgList, [hashtable]$Env = @{}) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $Exe
    $psi.Arguments = (($ArgList | ForEach-Object { ConvertTo-Arg $_ }) -join " ")
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
function Invoke-Git([string]$Dir, [string[]]$GitArgs) {
    return (Invoke-Native $GitExe $Dir (@("-c", "user.name=t", "-c", "user.email=t@t", "-c", "init.defaultBranch=main") + $GitArgs))
}
function Invoke-Installer([string]$Repo, [string[]]$Extra = @(), [hashtable]$Env = @{}) {
    return (Invoke-Native $PsExe $Repo (@("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $InstallScript, "-RepoDir", $Repo, "-HooksSourceDir", $HooksSourceDir) + $Extra) $Env)
}
function Assert-Case([string]$name, [bool]$ok, [string]$why) {
    if ($ok) { Write-Host "PASS    $name" }
    else { Write-Host "FAILED  $name :: $why"; $script:fail = 1 }
}
function Test-BareRef([string]$bare, [string]$ref) { return ((Invoke-Git $bare @("rev-parse", "--verify", "-q", $ref)).Code -eq 0) }
# .NET SHA256 (Get-FileHash fails to load under Git Bash, where PSModulePath carries PowerShell 7 modules)
function Get-Hash([string]$path) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    return [BitConverter]::ToString($sha.ComputeHash([IO.File]::ReadAllBytes($path)))
}
function New-WorkRepo([string]$root, [string]$name) {
    $r = Join-Path $root $name
    New-Item -ItemType Directory -Path $r -Force | Out-Null
    Invoke-Git $r @("init", "-q") | Out-Null
    Set-Content -LiteralPath (Join-Path $r "a.txt") -Value "one"
    Invoke-Git $r @("add", "a.txt") | Out-Null
    Invoke-Git $r @("commit", "-q", "-m", "c1") | Out-Null
    return $r
}
# Everything the installer must NOT change in the repository it is pointed at: refs, config, remotes.
function Get-RepoState([string]$repo) {
    return ((Invoke-Git $repo @("for-each-ref")).Out + "|" + (Invoke-Git $repo @("config", "--local", "--list")).Out + "|" + (Invoke-Git $repo @("remote", "-v")).Out + "|" + (Invoke-Git $repo @("rev-parse", "HEAD")).Out)
}
function Get-CommonHooks([string]$repo) { return (Join-Path (Join-Path $repo ".git") "hooks") }

$root = New-TempPath "installhooks-"
New-Item -ItemType Directory -Path $root -Force | Out-Null
Write-Output "fixture: $root"

# --- 1. Install into the common folder; push from a linked worktree on a branch WITHOUT scripts/dev/git-hooks ---
$R = New-WorkRepo $root "repo"
$bare = Join-Path (Join-Path $root "pub") "claudia-rh.git"
New-Item -ItemType Directory -Path $bare -Force | Out-Null
Invoke-Git $bare @("init", "-q", "--bare") | Out-Null
Invoke-Git $R @("remote", "add", "origin", $bare.Replace("\", "/")) | Out-Null
$WT = Join-Path $root "wt"
$wtAdd = Invoke-Git $R @("worktree", "add", "-q", "-b", "feat/wt", $WT)
Assert-Case "fixture: linked worktree created" ((Test-Path -LiteralPath (Join-Path $WT ".git")) -and (-not (Test-Path -LiteralPath (Join-Path $WT "scripts")))) "exit $($wtAdd.Code) $($wtAdd.Err)"
$common = Get-CommonHooks $R
# Installed FROM the linked worktree on purpose: the files must still land in the common folder of the main repository.
$stateBefore = Get-RepoState $WT
$i1 = Invoke-Installer $WT
Assert-Case "install: exit 0" ($i1.Code -eq 0) "exit $($i1.Code) out: $($i1.Out) err: $($i1.Err)"
Assert-Case "install: both files in the common hooks folder" ((Test-Path -LiteralPath (Join-Path $common "pre-push")) -and (Test-Path -LiteralPath (Join-Path $common "pre-push.ps1"))) "missing in $common"
Assert-Case "install: own check, run from the repository, reports main refused and feat/install-check passed" (($i1.Out -match "refs/heads/main -> refused") -and ($i1.Out -match "refs/heads/feat/install-check -> passed") -and ($i1.Out -match "OK\.") -and ($i1.Out -match "same as the installed folder")) "out: $($i1.Out)"
Assert-Case "install: the repository is unchanged (refs, config, remotes, HEAD)" ((Get-RepoState $WT) -ceq $stateBefore) "state changed"
$launcherBytes = [IO.File]::ReadAllBytes((Join-Path $common "pre-push"))
Assert-Case "install: installed launcher has 0 CR bytes and the marker" ((@($launcherBytes | Where-Object { $_ -eq 13 }).Count -eq 0) -and ([Text.Encoding]::ASCII.GetString($launcherBytes) -match "claudia-rh-installed-by")) "CR or marker problem"
$wtGitDir = (Invoke-Git $WT @("rev-parse", "--path-format=absolute", "--git-dir")).Out.Trim().Replace("/", "\")
Assert-Case "worktree: its own git dir has no hooks of its own" (-not (Test-Path -LiteralPath (Join-Path $wtGitDir "hooks\pre-push"))) "found hooks in $wtGitDir"
Invoke-Git $WT @("commit", "-q", "--allow-empty", "-m", "wt work") | Out-Null
$pm = Invoke-Git $WT @("push", "origin", "main")
Assert-Case "worktree (branch without the hook files): push main refused" (($pm.Code -ne 0) -and ($pm.Err -match "REFUSED") -and (-not (Test-BareRef $bare "refs/heads/main"))) "exit $($pm.Code) stderr: $($pm.Err)"
$pf = Invoke-Git $WT @("push", "origin", "feat/wt")
Assert-Case "worktree: push feat/wt passes" (($pf.Code -eq 0) -and (Test-BareRef $bare "refs/heads/feat/wt")) "exit $($pf.Code) stderr: $($pf.Err)"
Invoke-Git $R @("branch", "feat/two") | Out-Null
Invoke-Git $R @("branch", "feat/three") | Out-Null
$p2 = Invoke-Git $WT @("push", "origin", "feat/three", "feat/two")
Assert-Case "worktree: two branches in one push refused" (($p2.Code -ne 0) -and ($p2.Err -match "one work branch per push") -and (-not (Test-BareRef $bare "refs/heads/feat/two")) -and (-not (Test-BareRef $bare "refs/heads/feat/three"))) "exit $($p2.Code) stderr: $($p2.Err)"

# --- 2. Reinstall identical ---
$h1 = Get-Hash (Join-Path $common "pre-push")
$h2 = Get-Hash (Join-Path $common "pre-push.ps1")
$i2 = Invoke-Installer $R
Assert-Case "reinstall identical: exit 0 and 'already installed'" (($i2.Code -eq 0) -and ($i2.Out -match "already installed")) "exit $($i2.Code) out: $($i2.Out)"
Assert-Case "reinstall identical: files unchanged" (((Get-Hash (Join-Path $common "pre-push")) -eq $h1) -and ((Get-Hash (Join-Path $common "pre-push.ps1")) -eq $h2)) "hash changed"

# --- 3. Previous version installed by this script (marker kept, content old) is updated ---
Add-Content -LiteralPath (Join-Path $common "pre-push.ps1") -Value "# an older version"
Assert-Case "fixture: older marked version differs" ((Get-Hash (Join-Path $common "pre-push.ps1")) -ne $h2) "no change"
$i3 = Invoke-Installer $R
Assert-Case "older marked version: updated, exit 0" (($i3.Code -eq 0) -and ((Get-Hash (Join-Path $common "pre-push.ps1")) -eq $h2)) "exit $($i3.Code) out: $($i3.Out)"

# --- 4. A different pre-existing hook is never overwritten ---
foreach ($which in @("pre-push", "pre-push.ps1")) {
    $R4 = New-WorkRepo $root ("conflict-" + $which.Replace(".", "-"))
    $c4 = Get-CommonHooks $R4
    New-Item -ItemType Directory -Path $c4 -Force | Out-Null
    $mine = Join-Path $c4 $which
    [IO.File]::WriteAllText($mine, "#!/bin/sh`necho my own hook`n")
    $before = Get-Hash $mine
    $i4 = Invoke-Installer $R4
    $other = Join-Path $c4 $(if ($which -eq "pre-push") { "pre-push.ps1" } else { "pre-push" })
    Assert-Case "different existing $which : exit 1, file intact, nothing else written" (($i4.Code -eq 1) -and ((Get-Hash $mine) -eq $before) -and (-not (Test-Path -LiteralPath $other)) -and ($i4.Out -match "what to do")) "exit $($i4.Code) out: $($i4.Out)"
}

# --- 5. core.hooksPath set (local, then global): git ignores the common folder -> warn, exit 1, say "not in effect", config untouched ---
$R5 = New-WorkRepo $root "hookspath"
Invoke-Git $R5 @("config", "core.hooksPath", "elsewhere") | Out-Null
$i5 = Invoke-Installer $R5
$cfg = (Invoke-Git $R5 @("config", "--get", "core.hooksPath")).Out.Trim()
Assert-Case "local core.hooksPath: exit 1, 'not in effect', no OK, warns about precedence, config untouched" (($i5.Code -eq 1) -and ($i5.Out -match "not in effect") -and ($i5.Out -notmatch "OK\.") -and ($i5.Out -match "precedence") -and ($i5.Out -match "refs/heads/main -> NOT refused") -and ($cfg -eq "elsewhere")) "exit $($i5.Code) cfg=$cfg out: $($i5.Out)"
$R5g = New-WorkRepo $root "hookspath-global"
$gcfg = Join-Path $root "global-gitconfig"
[IO.File]::WriteAllText($gcfg, "[core]`n`thooksPath = C:/nowhere/hooks`n")
$stateG = Get-RepoState $R5g
$i5g = Invoke-Installer $R5g @() @{ GIT_CONFIG_GLOBAL = $gcfg }
Assert-Case "global core.hooksPath: exit 1, 'not in effect' with its origin, no OK, main NOT refused" (($i5g.Code -eq 1) -and ($i5g.Out -match "not in effect") -and ($i5g.Out -match "global-gitconfig") -and ($i5g.Out -notmatch "OK\.") -and ($i5g.Out -match "refs/heads/main -> NOT refused")) "exit $($i5g.Code) out: $($i5g.Out)"
Assert-Case "global core.hooksPath: repository unchanged" ((Get-RepoState $R5g) -ceq $stateG) "state changed"
# --- 6. -WhatIf writes nothing ---
$R6 = New-WorkRepo $root "whatif"
$i6 = Invoke-Installer $R6 @("-WhatIf")
$c6 = Get-CommonHooks $R6
Assert-Case "-WhatIf: exit 0, nothing written" (($i6.Code -eq 0) -and ($i6.Out -match "WhatIf") -and (-not (Test-Path -LiteralPath (Join-Path $c6 "pre-push"))) -and (-not (Test-Path -LiteralPath (Join-Path $c6 "pre-push.ps1")))) "exit $($i6.Code) out: $($i6.Out)"

# --- 7. Not a repository -> exit 2 ---
$nr = Join-Path $root "plain"
New-Item -ItemType Directory -Path $nr | Out-Null
$i7 = Invoke-Installer $nr
Assert-Case "folder that is not a repository: exit 2" ($i7.Code -eq 2) "exit $($i7.Code) out: $($i7.Out)"

if ($fail -eq 0) { Write-Output "ALL PASS" } else { Write-Output "SOME CASES FAILED" }
exit $fail
