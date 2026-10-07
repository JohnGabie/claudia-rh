# Pure tests for lib/travas.ps1: invented strings only, no disk. Runs on any OS:
#   pwsh -NoProfile -File sandbox-lab/scripts/test/travas.test.ps1
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../lib/travas.ps1"
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) {
  try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" }
}
function Should-BeNull($v) { if ($null -ne $v) { throw "expected null, got '$v'" } }
function Should-Match($v, [string]$re) { if ($null -eq $v -or "$v" -notmatch $re) { throw "expected /$re/, got '$v'" } }

$R = 'Z:\proj x\claudia-rh'
$D = 'Z:\proj x\claudia-rh\.sandbox-work\deploy'
$L = 'Z:\proj x\claudia-rh\sandbox-lab'
$O = 'Z:\proj x\claudia-rh\sandbox-lab\shared\out'

Check 'clean: absent deploy at the exact path is allowed' { Should-BeNull (Get-LabCleanRefusal -Deploy $D -RepoRoot $R -Exists $false -HasMarker $false) }
Check 'clean: existing deploy with the marker is allowed' { Should-BeNull (Get-LabCleanRefusal -Deploy $D -RepoRoot $R -Exists $true -HasMarker $true) }
Check 'clean: letter case does not matter' { Should-BeNull (Get-LabCleanRefusal -Deploy $D.ToUpper() -RepoRoot $R -Exists $true -HasMarker $true) }
Check 'clean: existing deploy without the marker is refused' { Should-Match (Get-LabCleanRefusal -Deploy $D -RepoRoot $R -Exists $true -HasMarker $false) 'marker' }
Check 'clean: any other path is refused' {
  foreach ($p in @("$R\.sandbox-work", "$R\.sandbox-work\deploy2", "$D\app", "$D\..\..", "$D.", ($D -replace '\\', '/'), 'Z:\', 'Z:\outro\.sandbox-work\deploy', 'deploy')) {
    Should-Match (Get-LabCleanRefusal -Deploy $p -RepoRoot $R -Exists $true -HasMarker $true) 'not exactly'
  }
}
Check 'clean: a link wins over the marker' { Should-Match (Get-LabCleanRefusal -Deploy $D -RepoRoot $R -Exists $true -HasMarker $true -Links @("$R\.sandbox-work")) 'link' }
Check 'clean: empty, relative, UNC, drive-root and dotted roots are refused' {
  Should-Match (Get-LabCleanRefusal -Deploy $D -RepoRoot '' -Exists $false -HasMarker $false) 'required'
  Should-Match (Get-LabCleanRefusal -Deploy 'claudia-rh\.sandbox-work\deploy' -RepoRoot 'claudia-rh' -Exists $false -HasMarker $false) 'repo root'
  Should-Match (Get-LabCleanRefusal -Deploy '\\srv\s\r\.sandbox-work\deploy' -RepoRoot '\\srv\s\r' -Exists $false -HasMarker $false) 'repo root'
  Should-Match (Get-LabCleanRefusal -Deploy 'Z:\.sandbox-work\deploy' -RepoRoot 'Z:\' -Exists $false -HasMarker $false) 'repo root'
  Should-Match (Get-LabCleanRefusal -Deploy 'Z:\a\..\b\.sandbox-work\deploy' -RepoRoot 'Z:\a\..\b' -Exists $false -HasMarker $false) 'repo root'
}
Check 'out: the exact shared\out is allowed' { Should-BeNull (Get-LabOutCleanRefusal -OutDir $O -LabDir $L) }
Check 'out: any other path is refused' {
  foreach ($p in @("$L\shared", "$O\sub", $L, 'Z:\', ($O -replace '\\', '/'))) { Should-Match (Get-LabOutCleanRefusal -OutDir $p -LabDir $L) 'not exactly' }
}
Check 'out: a link is refused' { Should-Match (Get-LabOutCleanRefusal -OutDir $O -LabDir $L -Links @("$O\x")) 'link' }
Check 'out: bad lab dirs are refused' {
  Should-Match (Get-LabOutCleanRefusal -OutDir 'Z:\shared\out' -LabDir 'Z:\') 'lab dir'
  Should-Match (Get-LabOutCleanRefusal -OutDir $O -LabDir '') 'required'
}

if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
