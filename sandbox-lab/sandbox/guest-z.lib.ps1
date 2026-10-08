# Pure decisions for disco-z.ps1. Dot-sourced in the guest (C:\labsandbox) and by the pure test on the host. No disk.
$script:LabZLetter = 'Z'
$script:LabZImage = 'C:\labdata\z.vhdx'

# Inside Windows Sandbox the Storage provider (Get-Disk, Get-Partition, Get-Volume, MSFT_Disk) lists nothing, not even
# disk 0 (verified 2026-10-07). disco-z.ps1 therefore reads the facts from MSFT_DiskImage (Get-DiskImage, which works)
# and from CIMv2 (Win32_LogicalDisk -> Win32_DiskPartition -> Win32_DiskDrive), and formats through diskpart with the
# vdisk selected BY ITS FILE. These functions decide from those facts only.

function Get-LabZOwnerRefusal {
  # Z: must be exactly one partition, on the disk attached from our own image file, and that disk is never disk 0.
  #   ImageAttached/ImageDiskNumber/ImageDevicePath: Get-DiskImage -ImagePath C:\labdata\z.vhdx
  #   ZDiskNumbers: DiskIndex of every Win32_DiskPartition behind Z:   ZDiskDevicePaths: DeviceID of their Win32_DiskDrive
  param([bool]$ImageAttached, $ImageDiskNumber, [string]$ImageDevicePath, [object[]]$ZDiskNumbers = @(), [string[]]$ZDiskDevicePaths = @())
  if (-not $ImageAttached) { return 'the image is not attached' }
  if ($null -eq $ImageDiskNumber -or "$ImageDiskNumber" -notmatch '^\d+$') { return 'disk number unknown' }
  if ([int]$ImageDiskNumber -eq 0) { return 'the image reports disk 0 (the system disk)' }
  if (-not $ImageDevicePath) { return 'device path of the image unknown' }
  $nums = @(@($ZDiskNumbers) | Where-Object { $null -ne $_ -and "$_" -ne '' })
  if ($nums.Count -eq 0) { return 'disk number unknown' }
  if ($nums.Count -ne 1) { return "Z: maps to $($nums.Count) partitions" }
  if ("$($nums[0])" -notmatch '^\d+$') { return 'disk number unknown' }
  if ([int]$nums[0] -ne [int]$ImageDiskNumber) { return "Z: is on disk $($nums[0]), the image is disk $ImageDiskNumber" }
  $devs = @(@($ZDiskDevicePaths) | Where-Object { $_ })
  if ($devs.Count -ne 1) { return 'device path of Z: unknown' }
  if ($devs[0] -ine $ImageDevicePath) { return "Z: is on $($devs[0]), the image is $ImageDevicePath" }
  return $null
}

function Get-LabZFormatRefusal {
  # Format only when Z: is ours (Get-LabZOwnerRefusal), our disk holds exactly one partition (the one diskpart selects
  # as 'partition 1' of the vdisk) and Z: has no file system yet: a formatted volume is never formatted again.
  param([bool]$ImageAttached, $ImageDiskNumber, [string]$ImageDevicePath, [object[]]$ZDiskNumbers = @(), [string[]]$ZDiskDevicePaths = @(),
        $PartitionsOnImageDisk, [string]$ZFileSystem)
  $owner = Get-LabZOwnerRefusal -ImageAttached $ImageAttached -ImageDiskNumber $ImageDiskNumber -ImageDevicePath $ImageDevicePath -ZDiskNumbers $ZDiskNumbers -ZDiskDevicePaths $ZDiskDevicePaths
  if ($owner) { return $owner }
  if ($null -eq $PartitionsOnImageDisk -or [int]$PartitionsOnImageDisk -ne 1) { return "the image disk has $PartitionsOnImageDisk partitions, expected 1" }
  if ($ZFileSystem) { return "Z: already has a file system ($ZFileSystem)" }
  return $null
}

function Get-LabZReadyRefusal {
  # After formatting (or to reuse): Z: is ours and NTFS.
  param([bool]$ImageAttached, $ImageDiskNumber, [string]$ImageDevicePath, [object[]]$ZDiskNumbers = @(), [string[]]$ZDiskDevicePaths = @(), [string]$ZFileSystem)
  $owner = Get-LabZOwnerRefusal -ImageAttached $ImageAttached -ImageDiskNumber $ImageDiskNumber -ImageDevicePath $ImageDevicePath -ZDiskNumbers $ZDiskNumbers -ZDiskDevicePaths $ZDiskDevicePaths
  if ($owner) { return $owner }
  if ($ZFileSystem -ne 'NTFS') { return "Z: file system is '$ZFileSystem', expected NTFS" }
  return $null
}

function Get-LabZFormatDiskPart {
  # The diskpart lines that format Z:. The disk is selected by OUR image file, never by number; partition 1 is the only
  # partition on it (checked by Get-LabZFormatRefusal right before).
  return @("select vdisk file=`"$($script:LabZImage)`"", 'select partition 1', 'format fs=ntfs quick label=LABZ')
}

function Get-LabZStartAction {
  # 'create' (nothing there yet), 'reuse' (Z: already on our image, NTFS), or 'refuse: <reason>'.
  param([bool]$ZExists, [bool]$ImageExists, $ReadyRefusal)
  if (-not $ZExists -and -not $ImageExists) { return 'create' }
  if (-not $ZExists) { return 'refuse: the image exists but Z: is not mounted' }
  if ($ReadyRefusal) { return "refuse: Z: is not ours or not ready ($ReadyRefusal)" }
  return 'reuse'
}
