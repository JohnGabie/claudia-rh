# Pure tests for sandbox/guest-z.lib.ps1 (the decisions disco-z.ps1 makes before touching a disk). No disk.
#   pwsh -NoProfile -File sandbox-lab/scripts/test/guest-z.test.ps1
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../../sandbox/guest-z.lib.ps1"
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) { try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" } }
function Should-BeNull($v) { if ($null -ne $v) { throw "expected null, got '$v'" } }
function Should-Match($v, [string]$re) { if ($null -eq $v -or "$v" -notmatch $re) { throw "expected /$re/, got '$v'" } }

# Facts as seen live in the Sandbox (2026-10-07): image = disk 1, \\.\PhysicalDrive1; Z: = Disk #1 on \\.\PHYSICALDRIVE1.
$ours = @{ ImageAttached = $true; ImageDiskNumber = 1; ImageDevicePath = '\\.\PhysicalDrive1'; ZDiskNumbers = @(1); ZDiskDevicePaths = @('\\.\PHYSICALDRIVE1') }
function With([hashtable]$Change) { $h = @{} + $ours; foreach ($k in $Change.Keys) { $h[$k] = $Change[$k] }; return $h }
function Invoke-Z([string]$Which, [hashtable]$Facts, [hashtable]$More = @{}) { $all = @{} + $Facts + $More; & "Get-LabZ${Which}Refusal" @all }

Check 'owner: Z: is ours only when it is one partition on the disk attached from our image' {
  Should-BeNull (Get-LabZOwnerRefusal @ours)
  Should-Match (Invoke-Z Owner (With @{ ImageAttached = $false })) 'not attached'
  Should-Match (Invoke-Z Owner (With @{ ImageDiskNumber = $null })) 'unknown'
  Should-Match (Invoke-Z Owner (With @{ ImageDiskNumber = 'x' })) 'unknown'
  Should-Match (Invoke-Z Owner (With @{ ZDiskNumbers = @() })) 'unknown'
  Should-Match (Invoke-Z Owner (With @{ ZDiskNumbers = @($null) })) 'unknown'
  Should-Match (Invoke-Z Owner (With @{ ZDiskNumbers = @(0) })) 'Z: is on disk 0'
  Should-Match (Invoke-Z Owner (With @{ ZDiskNumbers = @(1, 1) })) '2 partitions'
  Should-Match (Invoke-Z Owner (With @{ ImageDevicePath = '' })) 'device path'
  Should-Match (Invoke-Z Owner (With @{ ZDiskDevicePaths = @() })) 'device path of Z'
  Should-Match (Invoke-Z Owner (With @{ ZDiskDevicePaths = @('\\.\PHYSICALDRIVE0') })) 'PHYSICALDRIVE0'
}
Check 'owner: disk 0 (the system disk) is never ours, even when every number agrees' {
  Should-Match (Invoke-Z Owner (With @{ ImageDiskNumber = 0; ZDiskNumbers = @(0); ImageDevicePath = '\\.\PhysicalDrive0'; ZDiskDevicePaths = @('\\.\PHYSICALDRIVE0') })) 'disk 0'
}
Check 'format: only a RAW Z: that is ours, on a disk with exactly one partition' {
  Should-BeNull (Get-LabZFormatRefusal @ours -PartitionsOnImageDisk 1 -ZFileSystem '')
  Should-Match (Invoke-Z Format (With @{ ZDiskNumbers = @(0) }) @{ PartitionsOnImageDisk = 1; ZFileSystem = '' }) 'disk 0'
  Should-Match (Get-LabZFormatRefusal @ours -PartitionsOnImageDisk 1 -ZFileSystem 'NTFS') 'already has a file system'
  Should-Match (Get-LabZFormatRefusal @ours -PartitionsOnImageDisk 2 -ZFileSystem '') 'expected 1'
  Should-Match (Get-LabZFormatRefusal @ours -PartitionsOnImageDisk $null -ZFileSystem '') 'expected 1'
}
Check 'ready: ours and NTFS' {
  Should-BeNull (Get-LabZReadyRefusal @ours -ZFileSystem 'NTFS')
  Should-Match (Get-LabZReadyRefusal @ours -ZFileSystem '') 'expected NTFS'
  Should-Match (Invoke-Z Ready (With @{ ZDiskNumbers = @(0) }) @{ ZFileSystem = 'NTFS' }) 'disk 0'
}
Check 'format diskpart: selects the vdisk by our file, never a disk by number' {
  $l = @(Get-LabZFormatDiskPart)
  if ($l.Count -ne 3) { throw "lines: $($l.Count)" }
  if ($l[0] -cne 'select vdisk file="C:\labdata\z.vhdx"') { throw "first line: $($l[0])" }
  if ($l[1] -cne 'select partition 1') { throw "second line: $($l[1])" }
  if ($l[2] -notmatch '^format fs=ntfs quick label=LABZ$') { throw "third line: $($l[2])" }
  if (@($l | Where-Object { $_ -match '(?i)select\s+(disk|volume)|clean|delete' }).Count) { throw 'selects a disk/volume by number or wipes' }
}
Check 'start: create only when neither Z: nor the image exist; reuse only our own ready Z:' {
  if ((Get-LabZStartAction -ZExists $false -ImageExists $false -ReadyRefusal $null) -ne 'create') { throw 'create' }
  if ((Get-LabZStartAction -ZExists $true -ImageExists $true -ReadyRefusal $null) -ne 'reuse') { throw 'reuse' }
  Should-Match (Get-LabZStartAction -ZExists $true -ImageExists $true -ReadyRefusal 'Z: is on disk 0, the image is disk 3') '^refuse'
  Should-Match (Get-LabZStartAction -ZExists $true -ImageExists $false -ReadyRefusal 'the image is not attached') '^refuse'
  Should-Match (Get-LabZStartAction -ZExists $false -ImageExists $true -ReadyRefusal $null) '^refuse'
}
Check 'letter and image are pinned' {
  if ($script:LabZLetter -cne 'Z' -or $script:LabZImage -cne 'C:\labdata\z.vhdx') { throw 'changed' }
}
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
