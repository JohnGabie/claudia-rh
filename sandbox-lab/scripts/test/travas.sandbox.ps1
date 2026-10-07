# SANDBOX ONLY. Creates folders and junctions under Z:\claudia-teste-sandbox and runs app-stage.ps1 / Clear-LabOut against fake
# checkouts there. Refuses to run anywhere else. Nothing is cleaned up: the guest is discarded by test-down.
# Run through: node testes-no-guest.mjs --ps sandbox-lab/scripts/test/travas.sandbox.ps1
$ErrorActionPreference = 'Stop'
$Lib = Join-Path $PSScriptRoot '..\lib'
. (Join-Path $Lib 'travas.ps1')   # only defines functions
. (Join-Path $Lib 'disco.ps1')    # only defines functions
# Two factors, never $env:USERNAME: the WDAGUtilityAccount identity AND the guest-only C:\labsandbox\bootstrap.ps1.
if (-not (Test-LabInSandbox)) { Write-Host 'REFUSED: this test runs only inside Windows Sandbox'; exit 3 }
# Spec A6: everything this test creates lives on the guest-only Z: drive (sandbox\disco-z.ps1), never on C:.
$RootReal = $null
try { New-Item -ItemType Directory -Force -Path $script:LabGuestTestRoot | Out-Null; $RootReal = (Get-Item -LiteralPath $script:LabGuestTestRoot -Force).FullName } catch { }
$why = $(if ($RootReal) { Get-LabGuestTestRootRefusal $RootReal } else { 'the Z: test root could not be created (is Z: mounted? see C:\lab\out\disco-z.json)' })
if (-not $why -and @(Get-LabLinksAboveRoot $RootReal).Count) { $why = 'link (junction/symlink) above the test root' }
if ($why) { Write-Host "REFUSED: $why"; exit 3 }
$Base = Join-Path $RootReal ('travas-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $Base | Out-Null
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) {
  try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" }
}
function New-Sentinel([string]$Dir) { New-Item -ItemType Directory -Force $Dir | Out-Null; $f = Join-Path $Dir 'keep.txt'; [IO.File]::WriteAllText($f, 'keep'); return $f }
function New-FakeRepo([string]$Name) {
  $repo = Join-Path $Base $Name
  $scripts = Join-Path $repo 'sandbox-lab\scripts'
  New-Item -ItemType Directory -Path (Join-Path $scripts 'lib') | Out-Null
  Copy-Item -LiteralPath (Join-Path $PSScriptRoot '..\app-stage.ps1') -Destination $scripts
  Copy-Item -LiteralPath (Join-Path $Lib 'travas.ps1'), (Join-Path $Lib 'disco.ps1') -Destination (Join-Path $scripts 'lib')
  return $repo
}
$App = Join-Path $Base 'app x.exe'
[IO.File]::WriteAllText($App, 'x')
function Invoke-Stage([string]$Repo, [string[]]$Extra = @(), [string]$AppPath = $App) {
  $ErrorActionPreference = 'Continue'   # native stderr must not throw here
  $stage = Join-Path $Repo 'sandbox-lab\scripts\app-stage.ps1'
  $null = & powershell -NoProfile -ExecutionPolicy Bypass -File $stage -App $AppPath @Extra 2>&1
  return $LASTEXITCODE
}
function Should-Refuse([scriptblock]$Body) { $threw = $false; try { & $Body } catch { $threw = $true }; if (-not $threw) { throw 'expected a refusal' } }

Check 'first publish creates deploy, the marker and copies the app' {
  $r = New-FakeRepo 'a'
  if ((Invoke-Stage $r @('-Clean')) -ne 0) { throw 'app-stage failed' }
  if (-not (Test-Path -LiteralPath (Join-Path $r '.sandbox-work\deploy\.lab-deploy'))) { throw 'marker missing' }
  if (-not (Test-Path -LiteralPath (Join-Path $r '.sandbox-work\deploy\app\app x.exe'))) { throw 'app not copied' }
}
Check 'second publish with the marker wipes the old deploy' {
  $r = New-FakeRepo 'b'
  if ((Invoke-Stage $r @('-Clean')) -ne 0) { throw 'first publish failed' }
  $old = New-Sentinel (Join-Path $r '.sandbox-work\deploy\velho')
  if ((Invoke-Stage $r @('-Clean')) -ne 0) { throw 'second publish failed' }
  if (Test-Path -LiteralPath $old) { throw 'old file survived -Clean' }
}
Check 'deploy without the marker is refused and kept (with -Clean)' {
  $r = New-FakeRepo 'c'
  $keep = New-Sentinel (Join-Path $r '.sandbox-work\deploy')
  if ((Invoke-Stage $r @('-Clean')) -eq 0) { throw 'expected refusal' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'file deleted' }
}
Check 'deploy without the marker is refused and kept (without -Clean: robocopy /MIR also deletes)' {
  $r = New-FakeRepo 'c2'
  $keep = New-Sentinel (Join-Path $r '.sandbox-work\deploy\app')
  if ((Invoke-Stage $r) -eq 0) { throw 'expected refusal' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'file deleted' }
}
Check 'deploy as a junction is refused, target kept' {
  $r = New-FakeRepo 'd'
  $target = Join-Path $Base 'alvo-d'; $keep = New-Sentinel $target
  [IO.File]::WriteAllText((Join-Path $target '.lab-deploy'), '')
  New-Item -ItemType Directory -Path (Join-Path $r '.sandbox-work') | Out-Null
  New-Item -ItemType Junction -Path (Join-Path $r '.sandbox-work\deploy') -Target $target | Out-Null
  if ((Invoke-Stage $r @('-Clean')) -eq 0) { throw 'expected refusal' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'target file deleted' }
}
Check '.sandbox-work as a junction is refused, target kept' {
  $r = New-FakeRepo 'e'
  $target = Join-Path $Base 'alvo-e'; $keep = New-Sentinel (Join-Path $target 'deploy')
  [IO.File]::WriteAllText((Join-Path $target 'deploy\.lab-deploy'), '')
  New-Item -ItemType Junction -Path (Join-Path $r '.sandbox-work') -Target $target | Out-Null
  if ((Invoke-Stage $r @('-Clean')) -eq 0) { throw 'expected refusal' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'target file deleted' }
}
Check 'a junction inside deploy is refused, target kept' {
  $r = New-FakeRepo 'f'
  if ((Invoke-Stage $r @('-Clean')) -ne 0) { throw 'first publish failed' }
  $target = Join-Path $Base 'alvo-f'; $keep = New-Sentinel $target
  New-Item -ItemType Junction -Path (Join-Path $r '.sandbox-work\deploy\app\link') -Target $target | Out-Null
  if ((Invoke-Stage $r @('-Clean')) -eq 0) { throw 'expected refusal' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'target file deleted' }
}
Check 'a different -WorkDir is refused' {
  $r = New-FakeRepo 'g'
  $keep = New-Sentinel (Join-Path $Base 'outro-work\deploy')
  if ((Invoke-Stage $r @('-WorkDir', (Join-Path $Base 'outro-work'), '-Clean')) -eq 0) { throw 'expected refusal' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'file deleted' }
}
Check 'a source that is the repo (parent of the deploy) is refused, nothing copied' {
  $r = New-FakeRepo 'k'
  $ErrorActionPreference = 'Continue'
  $stage = Join-Path $r 'sandbox-lab\scripts\app-stage.ps1'
  $null = & powershell -NoProfile -ExecutionPolicy Bypass -File $stage -App $r 2>&1
  $code = $LASTEXITCODE
  $ErrorActionPreference = 'Stop'
  if ($code -eq 0) { throw 'expected refusal' }
  if (Test-Path -LiteralPath (Join-Path $r '.sandbox-work\deploy\app')) { throw 'deploy was populated' }
}
Check 'robocopy /XJ: a junction inside the app source is not followed' {
  $r = New-FakeRepo 'm'
  $src = Join-Path $Base 'fonte-m'; New-Item -ItemType Directory -Path $src | Out-Null
  [IO.File]::WriteAllText((Join-Path $src 'ok.txt'), 'ok')
  $target = Join-Path $Base 'alvo-m'; $null = New-Sentinel $target
  New-Item -ItemType Junction -Path (Join-Path $src 'link') -Target $target | Out-Null
  $ErrorActionPreference = 'Continue'
  $stage = Join-Path $r 'sandbox-lab\scripts\app-stage.ps1'
  $null = & powershell -NoProfile -ExecutionPolicy Bypass -File $stage -App $src 2>&1
  $ErrorActionPreference = 'Stop'
  if (-not (Test-Path -LiteralPath (Join-Path $r '.sandbox-work\deploy\app\ok.txt'))) { throw 'regular file not copied' }
  if (Test-Path -LiteralPath (Join-Path $r '.sandbox-work\deploy\app\link')) { throw 'junction was followed/copied' }
  if (-not (Test-Path -LiteralPath (Join-Path $target 'keep.txt'))) { throw 'target changed' }
}
Check 'Get-LabReparsePoints: an absent root is fine and a plain tree has no links' {
  if (@(Get-LabReparsePoints (Join-Path $Base 'nao-existe')).Count) { throw 'absent root should give no links' }
  $plain = Join-Path $Base 'plain-n'; $null = New-Sentinel (Join-Path $plain 'a\b')
  if (@(Get-LabReparsePoints $plain).Count) { throw 'plain tree should have no links' }
}
Check 'a source reached through a junction ancestor is refused, nothing is published' {
  $r = New-FakeRepo 'k2'
  $realDir = Join-Path $Base 'real-k2'; New-Item -ItemType Directory -Path $realDir | Out-Null
  [IO.File]::WriteAllText((Join-Path $realDir 'app k2.exe'), 'x')
  New-Item -ItemType Junction -Path (Join-Path $Base 'link-k2') -Target $realDir | Out-Null
  if ((Invoke-Stage $r @('-Clean') (Join-Path $Base 'link-k2\app k2.exe')) -eq 0) { throw 'expected refusal' }
  if (Test-Path -LiteralPath (Join-Path $r '.sandbox-work\deploy\app\app k2.exe')) { throw 'app was published through the link' }
}
Check 'Resolve-LabRealPath and Get-LabLinksAboveRoot see a junction ancestor' {
  $realDir = Join-Path $Base 'real-l'; New-Item -ItemType Directory -Path $realDir | Out-Null
  $link = Join-Path $Base 'link-l'; New-Item -ItemType Junction -Path $link -Target $realDir | Out-Null
  if ((Resolve-LabRealPath $realDir) -ne $realDir) { throw 'real path changed' }
  if (@(Get-LabLinksAboveRoot (Join-Path $link 'x')).Count -ne 1) { throw 'junction not found above a missing child' }
  if (@(Get-LabLinksAboveRoot $realDir).Count -ne 0) { throw 'false positive on a real folder' }
}
Check 'Test-LabInSandbox is true here (identity + guest bootstrap)' {
  if (-not (Test-LabInSandbox)) { throw 'expected true inside the guest' }
}
Check 'Clear-LabOut empties shared\out but keeps .gitkeep' {
  $lab = Join-Path $Base 'lab-h'; $out = Join-Path $lab 'shared\out'
  $f = New-Sentinel (Join-Path $out 'sub'); [IO.File]::WriteAllText((Join-Path $out '.gitkeep'), '')
  Clear-LabOut -OutDir $out -LabDir $lab
  if (Test-Path -LiteralPath $f) { throw 'out not cleaned' }
  if (-not (Test-Path -LiteralPath (Join-Path $out '.gitkeep'))) { throw '.gitkeep removed' }
}
Check 'Clear-LabOut refuses a junction inside out, target kept' {
  $lab = Join-Path $Base 'lab-i'; $out = Join-Path $lab 'shared\out'
  New-Item -ItemType Directory -Force $out | Out-Null
  $target = Join-Path $Base 'alvo-i'; $keep = New-Sentinel $target
  New-Item -ItemType Junction -Path (Join-Path $out 'link') -Target $target | Out-Null
  Should-Refuse { Clear-LabOut -OutDir $out -LabDir $lab }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'target file deleted' }
}
Check 'Clear-LabOut refuses shared\out as a junction, target kept' {
  $lab = Join-Path $Base 'lab-j'
  New-Item -ItemType Directory -Force (Join-Path $lab 'shared') | Out-Null
  $target = Join-Path $Base 'alvo-j'; $keep = New-Sentinel $target
  New-Item -ItemType Junction -Path (Join-Path $lab 'shared\out') -Target $target | Out-Null
  Should-Refuse { Clear-LabOut -OutDir (Join-Path $lab 'shared\out') -LabDir $lab }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'target file deleted' }
}

function New-FakeLogin([string]$Name) {
  $up = Join-Path $Base $Name
  New-Item -ItemType Directory -Force (Join-Path $up '.claude') | Out-Null
  [IO.File]::WriteAllText((Join-Path $up '.claude\.credentials.json'), 'fake-login-for-test')
  return $up
}
Check 'the test base is on Z: and its real path is under the guest test root' {
  if (Get-LabGuestTestRootRefusal (Get-Item -LiteralPath $Base).FullName) { throw 'base not under Z:\claudia-teste-sandbox' }
}
Check 'credential: copy creates exactly .sandbox-work\credencial\.credentials.json, same bytes, no .tmp left' {
  $r = Join-Path $Base 'cred-a'; $up = New-FakeLogin 'perfil-a'
  Copy-LabCredential -RepoRoot $r -UserProfile $up -ConfigDir ''
  $p = Get-LabCredentialPaths $r
  if ([IO.File]::ReadAllText($p.File) -ne 'fake-login-for-test') { throw 'content differs' }
  if (@(Get-ChildItem -LiteralPath $p.Dir -Force).Count -ne 1) { throw 'extra files in credencial' }
}
Check 'credential: a second copy replaces the first' {
  $r = Join-Path $Base 'cred-b'; $up = New-FakeLogin 'perfil-b'
  Copy-LabCredential -RepoRoot $r -UserProfile $up -ConfigDir ''
  [IO.File]::WriteAllText((Join-Path $up '.claude\.credentials.json'), 'second')
  Copy-LabCredential -RepoRoot $r -UserProfile $up -ConfigDir ''
  if ([IO.File]::ReadAllText((Get-LabCredentialPaths $r).File) -ne 'second') { throw 'not replaced' }
}
Check 'credential: CLAUDE_CONFIG_DIR set refuses and copies nothing' {
  $r = Join-Path $Base 'cred-c'; $up = New-FakeLogin 'perfil-c'
  Should-Refuse { Copy-LabCredential -RepoRoot $r -UserProfile $up -ConfigDir 'Z:\outra' }
  if (Test-Path -LiteralPath (Get-LabCredentialPaths $r).File) { throw 'copied anyway' }
}
Check 'credential: no login on the PC refuses with the /login hint' {
  $r = Join-Path $Base 'cred-d'; $up = Join-Path $Base 'perfil-vazio'; New-Item -ItemType Directory -Force $up | Out-Null
  $msg = $null; try { Copy-LabCredential -RepoRoot $r -UserProfile $up -ConfigDir '' } catch { $msg = $_.Exception.Message }
  if ("$msg" -notmatch '/login') { throw "got: $msg" }
}
Check 'credential: remove deletes only that file; a sentinel in the same folder is kept' {
  $r = Join-Path $Base 'cred-e'; $up = New-FakeLogin 'perfil-e'
  Copy-LabCredential -RepoRoot $r -UserProfile $up -ConfigDir ''
  $p = Get-LabCredentialPaths $r; $keep = Join-Path $p.Dir 'keep.txt'; [IO.File]::WriteAllText($keep, 'keep')
  Remove-LabCredential -RepoRoot $r
  if (Test-Path -LiteralPath $p.File) { throw 'copy survived' }
  if (-not (Test-Path -LiteralPath $keep)) { throw 'sentinel deleted' }
  if (-not (Test-Path -LiteralPath (Join-Path $up '.claude\.credentials.json'))) { throw 'source deleted' }
}
Check 'credential: a stray .tmp is removed, any other file is refused and kept' {
  $r = Join-Path $Base 'cred-f'; $p = Get-LabCredentialPaths $r; New-Item -ItemType Directory -Force $p.Dir | Out-Null
  $tmp = "$($p.File).0123456789abcdef0123456789abcdef.tmp"; [IO.File]::WriteAllText($tmp, 'x')
  Remove-LabCredential -RepoRoot $r
  if (Test-Path -LiteralPath $tmp) { throw '.tmp survived' }
  $other = Join-Path $p.Dir 'outro.json'; [IO.File]::WriteAllText($other, 'x')
  Should-Refuse { Remove-LabCredentialFile -File $other -RepoRoot $r }
  if (-not (Test-Path -LiteralPath $other)) { throw 'other file deleted' }
}
Check 'credential: credencial folder as a junction is refused, target kept' {
  $r = Join-Path $Base 'cred-g'; New-Item -ItemType Directory -Force (Join-Path $r '.sandbox-work') | Out-Null
  $target = Join-Path $Base 'alvo-cred'; $keep = New-Sentinel $target
  [IO.File]::WriteAllText((Join-Path $target '.credentials.json'), 'x')
  New-Item -ItemType Junction -Path (Get-LabCredentialPaths $r).Dir -Target $target | Out-Null
  Should-Refuse { Remove-LabCredential -RepoRoot $r }
  Should-Refuse { Copy-LabCredential -RepoRoot $r -UserProfile (New-FakeLogin 'perfil-g') -ConfigDir '' }
  if (-not (Test-Path -LiteralPath $keep) -or -not (Test-Path -LiteralPath (Join-Path $target '.credentials.json'))) { throw 'target changed' }
}

if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
