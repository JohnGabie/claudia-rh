# Pure tests for sandbox/guest-z.lib.ps1 (the decisions disco-z.ps1 makes before touching a disk). No disk.
#   pwsh -NoProfile -File sandbox-lab/scripts/test/guest-z.test.ps1
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../../sandbox/guest-z.lib.ps1"
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) { try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" } }
function Should-BeNull($v) { if ($null -ne $v) { throw "expected null, got '$v'" } }
function Should-Match($v, [string]$re) { if ($null -eq $v -or "$v" -notmatch $re) { throw "expected /$re/, got '$v'" } }

Check 'format: allowed only when Z: is on the disk attached from our image' {
  Should-BeNull (Get-LabZFormatRefusal -ImageAttached $true -ImageDiskNumber 3 -PartitionDiskNumber 3)
  Should-Match (Get-LabZFormatRefusal -ImageAttached $true -ImageDiskNumber 3 -PartitionDiskNumber 0) 'disk 0'
  Should-Match (Get-LabZFormatRefusal -ImageAttached $false -ImageDiskNumber 3 -PartitionDiskNumber 3) 'not attached'
  Should-Match (Get-LabZFormatRefusal -ImageAttached $true -ImageDiskNumber $null -PartitionDiskNumber 3) 'unknown'
  Should-Match (Get-LabZFormatRefusal -ImageAttached $true -ImageDiskNumber 3 -PartitionDiskNumber $null) 'unknown'
}
Check 'start: create only when neither Z: nor the image exist; reuse only our own Z:' {
  if ((Get-LabZStartAction -ZExists $false -ImageExists $false -FormatRefusal $null) -ne 'create') { throw 'create' }
  if ((Get-LabZStartAction -ZExists $true -ImageExists $true -FormatRefusal $null) -ne 'reuse') { throw 'reuse' }
  Should-Match (Get-LabZStartAction -ZExists $true -ImageExists $true -FormatRefusal 'Z: is on disk 0, the image is disk 3') '^refuse'
  Should-Match (Get-LabZStartAction -ZExists $true -ImageExists $false -FormatRefusal 'the image is not attached') '^refuse'
  Should-Match (Get-LabZStartAction -ZExists $false -ImageExists $true -FormatRefusal $null) '^refuse'
}
Check 'letter and image are pinned' {
  if ($script:LabZLetter -cne 'Z' -or $script:LabZImage -cne 'C:\labdata\z.vhdx') { throw 'changed' }
}
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
