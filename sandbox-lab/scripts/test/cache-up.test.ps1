# Pure test: reads the text of cache-up.ps1 and layout-msvc.ps1 (this repository). cache-up must never delete, move over,
# overwrite, or run anything of Visual Studio on the PC (spec A2 and addendum 8.2).
#   pwsh -NoProfile -File sandbox-lab/scripts/test/cache-up.test.ps1
$ErrorActionPreference = 'Stop'
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) { try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" } }
$text = Get-Content -LiteralPath "$PSScriptRoot/../cache-up.ps1" -Raw
$guest = Get-Content -LiteralPath "$PSScriptRoot/../../sandbox/layout-msvc.ps1" -Raw

Check 'cache-up has no delete or overwrite command' {
  $bad = '(?im)\b(Remove-Item|rmdir|rd|del|erase|Clear-Content|Clear-Item|Move-Item|Copy-Item|robocopy|Set-Content|Out-File)\b|\.Delete\(|\]::Replace\('
  $m = [regex]::Matches($text, $bad)
  if ($m.Count) { throw "found: $(@($m | ForEach-Object { $_.Value }) -join ', ')" }
  # one level of nested parentheses is allowed in the arguments, e.g. Copy((Join-Path $a 'x'), (Join-Path $b 'x'), $false)
  $copies = [regex]::Matches($text, '\[IO\.File\]::Copy\((?:[^()]|\([^()]*\))*\)')
  if (-not $copies.Count) { throw 'no [IO.File]::Copy call found: the check below would prove nothing' }
  foreach ($c in $copies) { if ($c.Value -notmatch ',\s*\$false\)$') { throw "File.Copy without overwrite=`$false: $($c.Value)" } }
}
Check 'cache-up never runs the Visual Studio bootstrapper on the PC' {
  $m = [regex]::Matches($text, '(?im)Start-Process|Invoke-Item|--layout|--add\b|--noWeb')
  if ($m.Count) { throw "found: $(@($m | ForEach-Object { $_.Value }) -join ', ')" }
}
Check 'cache-up checks with the pure guards and renames only new files and folders' {
  foreach ($must in @('Get-LabCacheDirRefusal', 'Get-LabCachePlan', 'Get-LabLayoutImportRefusal', 'Get-LabLayoutManifestProblems', 'Get-LabReparsePoints', '.tmp', '[IO.File]::Move(', '[IO.Directory]::Move(')) {
    if (-not $text.Contains($must)) { throw "missing: $must" }
  }
}
Check 'layout-msvc copies into a new folder and never mirrors' {
  if ($guest -match '(?i)/MIR|/PURGE|/MOV|Remove-Item') { throw 'mirror or delete in layout-msvc.ps1' }
  foreach ($must in @('C:\labcachesaida', 'layout.sha256', '--layout')) { if (-not $guest.Contains($must)) { throw "missing: $must" } }
}
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
