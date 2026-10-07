# Pure decisions for disco-z.ps1. Dot-sourced in the guest (C:\labsandbox) and by the pure test on the host. No disk.
$script:LabZLetter = 'Z'
$script:LabZImage = 'C:\labdata\z.vhdx'

function Get-LabZFormatRefusal {
  # Format only the partition that sits on the disk attached from our own image file.
  param([bool]$ImageAttached, $ImageDiskNumber, $PartitionDiskNumber)
  if (-not $ImageAttached) { return 'the image is not attached' }
  if ($null -eq $ImageDiskNumber -or $null -eq $PartitionDiskNumber) { return 'disk number unknown' }
  if ([int]$ImageDiskNumber -ne [int]$PartitionDiskNumber) { return "Z: is on disk $PartitionDiskNumber, the image is disk $ImageDiskNumber" }
  return $null
}

function Get-LabZStartAction {
  # 'create' (nothing there yet), 'reuse' (Z: already on our image), or 'refuse: <reason>'.
  param([bool]$ZExists, [bool]$ImageExists, $FormatRefusal)
  if (-not $ZExists -and -not $ImageExists) { return 'create' }
  if (-not $ZExists) { return 'refuse: the image exists but Z: is not mounted' }
  if ($FormatRefusal) { return "refuse: Z: is already used by another disk ($FormatRefusal)" }
  return 'reuse'
}
