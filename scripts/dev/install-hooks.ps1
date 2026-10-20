# Installs the pre-push hook (T22) into the COMMON hooks folder of the repository:
#     <git rev-parse --git-common-dir>/hooks
# That folder is shared by every worktree and every branch of the clone, so the protection does not depend on the
# branch having scripts/dev/git-hooks (a branch cut before the hook existed is protected too).
# Copies the sh launcher "pre-push" and "pre-push.ps1" (both normalized to LF, a marker comment added) from
# scripts/dev/git-hooks. The launcher finds pre-push.ps1 next to itself, so the copies work outside the repository tree.
# The installed files are GENERATED COPIES: never edit them in the hooks folder. Edit the source in scripts/dev/git-hooks and
# run this script again.
#
# Rules (safety first):
#   - Never overwrites or deletes a hook it did not install. A DIFFERENT pre-push / pre-push.ps1 without the marker
#     line ("# claudia-rh-installed-by: ...") stops the install (exit 1), nothing is written, and the message says what to do.
#   - Identical content: "already installed", exit 0. A file with the marker but older content is an older version of
#     this script's own install: it is updated.
#   - core.hooksPath (local, global or per worktree) takes precedence over the common folder: git then ignores it. After
#     installing, the script asks git which hooks folder this repository really uses ("git rev-parse --git-path hooks")
#     and compares it with the common folder. If they differ it says the hook is NOT in effect (and which core.hooksPath
#     applies, with its origin), does not say OK and exits 1. It never changes the git config; that is the user's.
#   - It also runs a REAL check from the repository itself, with its effective config (no -c override): it pushes HEAD by
#     path to a temporary bare repository named claudia-rh.git, as refs/heads/main (must be refused) and as
#     refs/heads/feat/install-check (must pass). No remote, ref or config of the repository changes; the bare lives in
#     the system temp folder and is left behind.
#   - -WhatIf only prints what it would do (no write, no check).
# Usage: powershell -NoProfile -File scripts/dev/install-hooks.ps1 [-WhatIf]
#        [-RepoDir <any folder of the repository or of one of its worktrees>] [-HooksSourceDir <folder with the two files>]
# Exit: 0 installed (or already installed) and checked; 1 conflict or the check failed; 2 environment problem.
param([switch]$WhatIf,
      [string]$RepoDir = "",
      [string]$HooksSourceDir = (Join-Path $PSScriptRoot "git-hooks"))
$ErrorActionPreference = "Stop"
$Marker = "# claudia-rh-installed-by: scripts/dev/install-hooks.ps1"
$HookFiles = @("pre-push", "pre-push.ps1")

function Say([string]$t) { Write-Output $t }
function Stop-With([int]$code, [string]$t) { Write-Output $t; exit $code }

function ConvertTo-Arg([string]$a) {
    if ($a -eq "") { return '""' }
    if ($a -notmatch '[\s"]') { return $a }
    return '"' + ($a -replace '"', '\"') + '"'
}
function Invoke-Git([string]$Dir, [string[]]$GitArgs) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $script:GitExe
    $all = @("-c", "user.name=t", "-c", "user.email=t@t", "-c", "init.defaultBranch=main") + $GitArgs
    $psi.Arguments = (($all | ForEach-Object { ConvertTo-Arg $_ }) -join " ")
    $psi.WorkingDirectory = $Dir
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $p = [System.Diagnostics.Process]::Start($psi)
    $o = $p.StandardOutput.ReadToEndAsync()
    $e = $p.StandardError.ReadToEndAsync()
    $p.WaitForExit()
    return [pscustomobject]@{ Code = $p.ExitCode; Out = $o.Result.Trim(); Err = $e.Result.Trim() }
}

# Expected installed text: source normalized to LF, the marker added (line 2 of the sh launcher, line 1 of the .ps1).
function Get-InstalledText([string]$SourceFile, [string]$Name) {
    $text = [IO.File]::ReadAllText($SourceFile).Replace("`r`n", "`n").Replace("`r", "`n")
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.AddRange([string[]]($text -split "`n"))
    if ($Name -eq "pre-push" -and $lines.Count -gt 0 -and $lines[0].StartsWith("#!")) { $lines.Insert(1, $Marker) }
    else { $lines.Insert(0, $Marker) }
    return ($lines -join "`n")
}

# --- environment ---
$gitCmd = Get-Command git -ErrorAction SilentlyContinue
if (-not $gitCmd) { Stop-With 2 "install-hooks: git not found on PATH." }
$GitExe = $gitCmd.Source
if (-not $RepoDir) { $RepoDir = $PSScriptRoot }
if (-not (Test-Path -LiteralPath $RepoDir -PathType Container)) { Stop-With 2 "install-hooks: folder not found: $RepoDir" }
foreach ($f in $HookFiles) {
    if (-not (Test-Path -LiteralPath (Join-Path $HooksSourceDir $f) -PathType Leaf)) { Stop-With 2 "install-hooks: source hook not found: $(Join-Path $HooksSourceDir $f)" }
}
$cd = Invoke-Git $RepoDir @("rev-parse", "--path-format=absolute", "--git-common-dir")
if ($cd.Code -ne 0 -or -not $cd.Out) { Stop-With 2 "install-hooks: not a git repository ($RepoDir): $($cd.Err)" }
$commonDir = $cd.Out.Replace("/", "\")
$hooksDir = Join-Path $commonDir "hooks"
# The real check needs a commit to push: refuse BEFORE writing anything into the hooks folder.
if ((Invoke-Git $RepoDir @("rev-parse", "--verify", "-q", "HEAD^{commit}")).Code -ne 0) {
    Stop-With 2 "install-hooks: the repository has no commit at HEAD, so the real check cannot run and nothing was written. Make a commit and run this script again."
}
Say "install-hooks: common hooks folder (all worktrees and branches): $hooksDir"

$hp = Invoke-Git $RepoDir @("config", "--get", "core.hooksPath")
$hooksPathSet = ($hp.Code -eq 0 -and $hp.Out)
if ($hooksPathSet) {
    Say "install-hooks: WARNING: core.hooksPath is set to '$($hp.Out)'. It takes precedence: git IGNORES the common hooks folder while it is set, so this install will NOT take effect in this repository. This script does not change the config; unset it yourself ('git config --unset core.hooksPath') if you want the common folder to apply."
}

# --- plan: decide every file before writing any ---
$plan = @()
$conflicts = 0
foreach ($f in $HookFiles) {
    $target = Join-Path $hooksDir $f
    $want = Get-InstalledText (Join-Path $HooksSourceDir $f) $f
    $action = "install"
    if (Test-Path -LiteralPath $target -PathType Leaf) {
        $have = [IO.File]::ReadAllText($target).Replace("`r`n", "`n")
        if ($have -ceq $want) { $action = "same" }
        elseif ($have.Contains($Marker)) { $action = "update" }
        else { $action = "conflict"; $conflicts++ }
    } elseif (Test-Path -LiteralPath $target) {
        $action = "conflict"; $conflicts++
    }
    $plan += [pscustomobject]@{ Name = $f; Target = $target; Want = $want; Action = $action }
}
foreach ($p in $plan) { Say ("install-hooks: {0,-13} {1}" -f $p.Action, $p.Target) }

if ($conflicts -gt 0) {
    Say "install-hooks: REFUSED. A different hook already exists in the common hooks folder and it was not installed by this script, so nothing was written."
    Say "install-hooks: what to do: look at the existing file(s) above, merge their logic into scripts/dev/git-hooks by hand (or move them aside yourself), then run this script again."
    exit 1
}

if ($WhatIf) {
    foreach ($p in $plan) {
        if ($p.Action -eq "same") { Say "install-hooks: -WhatIf: $($p.Name) is already installed (ja instalado); nothing to do." }
        else { Say "install-hooks: -WhatIf: would $($p.Action) $($p.Target)" }
    }
    Say "install-hooks: -WhatIf: no file written, no check run."
    exit 0
}

# --- write ---
$utf8 = New-Object System.Text.UTF8Encoding $false
try {
    if (-not (Test-Path -LiteralPath $hooksDir -PathType Container)) { New-Item -ItemType Directory -Path $hooksDir -Force | Out-Null }
    foreach ($p in $plan) {
        if ($p.Action -eq "same") { continue }
        [IO.File]::WriteAllText($p.Target, $p.Want, $utf8)
        Say "install-hooks: wrote $($p.Target)"
    }
} catch {
    Stop-With 2 "install-hooks: could not write to the hooks folder: $($_.Exception.Message)"
}
if (@($plan | Where-Object { $_.Action -ne "same" }).Count -eq 0) { Say "install-hooks: already installed (ja instalado), identical content." }

# --- 1. is the installed folder the one git really uses in THIS repository? ---
# "git rev-parse --git-path hooks" honors core.hooksPath from the local, global and per-worktree config.
function Format-PathKey([string]$p) { return $p.Replace("\", "/").TrimEnd("/").ToLowerInvariant() }
$inEffect = $false
$eff = Invoke-Git $RepoDir @("rev-parse", "--path-format=absolute", "--git-path", "hooks")
if ($eff.Code -eq 0 -and $eff.Out) { $inEffect = ((Format-PathKey $eff.Out) -ceq (Format-PathKey $hooksDir)) }
$effectiveText = if ($eff.Out) { $eff.Out } else { "(unknown)" }
Say "install-hooks: hooks folder git uses in this repository: $effectiveText -> $(if ($inEffect) { 'same as the installed folder' } else { 'DIFFERENT from the installed folder' })"

# --- 2. real check, run FROM the repository itself so its effective config decides (no -c override) ---
# Pushes HEAD by path to a temporary bare repository named claudia-rh.git (no remote is created, no ref or config of
# the repository changes): HEAD:refs/heads/main must be refused, HEAD:refs/heads/feat/install-check must pass.
$root = Join-Path ([IO.Path]::GetTempPath()) ("install-hooks-check-" + [guid]::NewGuid().ToString("N"))
$bare = Join-Path $root "claudia-rh.git"
$checkOk = $false
try {
    if ((Invoke-Git $RepoDir @("rev-parse", "--verify", "-q", "HEAD^{commit}")).Code -ne 0) {
        Stop-With 2 "install-hooks: the repository has no commit at HEAD, so the real check cannot run. Make a commit and run this script again."
    }
    New-Item -ItemType Directory -Path $bare -Force | Out-Null
    Invoke-Git $bare @("init", "-q", "--bare") | Out-Null
    $url = $bare.Replace("\", "/")
    $pushMain = Invoke-Git $RepoDir @("push", "--no-follow-tags", "--recurse-submodules=no", $url, "HEAD:refs/heads/main")
    $mainThere = (Invoke-Git $bare @("rev-parse", "--verify", "-q", "refs/heads/main")).Code -eq 0
    $mainRefused = ($pushMain.Code -ne 0) -and (-not $mainThere) -and ($pushMain.Err -match "REFUSED")
    $pushFeat = Invoke-Git $RepoDir @("push", "--no-follow-tags", "--recurse-submodules=no", $url, "HEAD:refs/heads/feat/install-check")
    $featThere = (Invoke-Git $bare @("rev-parse", "--verify", "-q", "refs/heads/feat/install-check")).Code -eq 0
    $featPassed = ($pushFeat.Code -eq 0) -and $featThere
    Say "install-hooks: check (temp bare in $root): push HEAD:refs/heads/main -> $(if ($mainRefused) { 'refused, as expected' } else { 'NOT refused (exit ' + $pushMain.Code + ')' })"
    Say "install-hooks: check: push HEAD:refs/heads/feat/install-check -> $(if ($featPassed) { 'passed, as expected' } else { 'NOT passed (exit ' + $pushFeat.Code + ')' })"
    if (-not $featPassed) { Say $pushFeat.Err }
    $checkOk = $mainRefused -and $featPassed
} catch {
    Say "install-hooks: check failed to run: $($_.Exception.Message)"
}

if (-not $inEffect) {
    $origin = Invoke-Git $RepoDir @("config", "--show-origin", "--get", "core.hooksPath")
    $from = if ($origin.Code -eq 0 -and $origin.Out) { "core.hooksPath is in effect, from: $($origin.Out)" } else { "git uses '$effectiveText' (core.hooksPath not found by 'git config --show-origin')" }
    $optB = "git config core.hooksPath `"" + $hooksDir.Replace("\", "/") + "`""
    Stop-With 1 "install-hooks: the hook is NOT in effect in this repository (not in effect): $from. The files were copied to $hooksDir but git does not read that folder here, because core.hooksPath points at another folder. This script does not change your git config. To activate the hook you decide between two options (commands shown as text, not run). Option 1, remove that core.hooksPath: git config --unset core.hooksPath   Option 2, point it at the common folder: $optB   Then run this script again."
}
if (-not $checkOk) { Stop-With 1 "install-hooks: CHECK FAILED. The installed hook did not behave as expected in this repository; do not rely on it." }
Say "install-hooks: OK. Installed, in effect in this repository and checked for every worktree and branch that uses $hooksDir (temp check folder left behind: $root)."
exit 0
