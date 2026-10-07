# disco-z.ps1 - runs INSIDE the Sandbox as an administrator (bootstrap.ps1 when the logon is elevated, otherwise
# test-up.ps1 through 'wsb exec -r System'). Creates C:\labdata\z.vhdx, mounts it as Z: and creates Z:\tmp and
# Z:\claudia-teste-sandbox (spec A6). Formats only the partition of the disk attached from that exact image file.
# Output: C:\lab\out\disco-z.json
param([int]$MaxMB = 20480)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'guest-z.lib.ps1')
. (Join-Path $PSScriptRoot 'guest-install.lib.ps1')
# Guest only, two factors: the guest identity (WDAGUtilityAccount, or SYSTEM through wsb exec plus the WDAG profile)
# and the bootstrap marker that is mapped only in the guest. Outside the guest: print and leave, BEFORE anything else runs
# and outside the try, so the refusal never reaches Save-Result and nothing is written to the disk.
if (-not (Test-LabInGuest)) { Write-Host 'not inside the lab guest (needs the guest identity and the bootstrap marker)'; exit 1 }
$Image = $script:LabZImage
$OutFile = 'C:\lab\out\disco-z.json'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$sw = [Diagnostics.Stopwatch]::StartNew()
$res = [ordered]@{ ok = $false; letra = $script:LabZLetter; imagem = $Image; segundos = $null; erro = $null }

function Save-Result { $res.segundos = [math]::Round($sw.Elapsed.TotalSeconds, 1); [IO.File]::WriteAllText($OutFile, ($res | ConvertTo-Json), $utf8) }
function Invoke-DiskPart([string[]]$Lines) {
  $f = 'C:\labdata\disco-z-' + [guid]::NewGuid().ToString('N') + '.txt'
  [IO.File]::WriteAllLines($f, $Lines)
  $ErrorActionPreference = 'Continue'   # PS 5.1: native stderr must not become a terminating error here
  $out = & diskpart.exe /s $f 2>&1
  if ($LASTEXITCODE -ne 0) { throw "diskpart failed ($LASTEXITCODE): $($out -join ' | ')" }
}
function Get-State {
  $img = Get-DiskImage -ImagePath $Image -ErrorAction SilentlyContinue
  $part = Get-Partition -DriveLetter $script:LabZLetter -ErrorAction SilentlyContinue
  $imgDisk = $(if ($img -and $img.Attached) { $img.Number } else { $null })
  $partDisk = $(if ($part) { $part.DiskNumber } else { $null })
  return @{ img = $img; part = $part; refusal = (Get-LabZFormatRefusal -ImageAttached ([bool]($img -and $img.Attached)) -ImageDiskNumber $imgDisk -PartitionDiskNumber $partDisk) }
}

try {
  $zExists = Test-Path -LiteralPath 'Z:\'
  $s = $(if ($zExists) { Get-State } else { @{ refusal = $null } })
  $action = Get-LabZStartAction -ZExists $zExists -ImageExists (Test-Path -LiteralPath $Image) -FormatRefusal $s.refusal
  if ($action -like 'refuse*') { throw $action }
  if ($action -eq 'create') {
    New-Item -ItemType Directory -Force 'C:\labdata' | Out-Null
    Invoke-DiskPart @(
      "create vdisk file=`"$Image`" maximum=$MaxMB type=expandable",
      "select vdisk file=`"$Image`"",
      'attach vdisk',
      'create partition primary',
      "assign letter=$($script:LabZLetter)"
    )
    $s = Get-State
    if ($s.refusal) { throw "nothing formatted: $($s.refusal)" }
    $s.part | Format-Volume -FileSystem NTFS -NewFileSystemLabel 'LABZ' -Confirm:$false | Out-Null
  }
  New-Item -ItemType Directory -Force 'Z:\tmp', 'Z:\claudia-teste-sandbox' | Out-Null
  $item = Get-Item -LiteralPath 'Z:\claudia-teste-sandbox' -Force
  if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Z:\claudia-teste-sandbox is a link' }
  $res.ok = $true
} catch { $res.erro = $_.Exception.Message }
Save-Result
if ($res.ok) { exit 0 } else { exit 1 }
