# Disk helpers for the lab guards. Reading attributes only, except Clear-LabOut, which deletes inside shared\out after
# the pure guard (travas.ps1) said yes. Dot-source travas.ps1 first.

function Get-LabLinksInPath([string]$Dir, [string]$StopAt) {
  # Reparse points from $Dir up to, not including, $StopAt. Missing folders are not links.
  $links = @()
  $cur = $Dir
  $stop = ConvertTo-LabPathKey $StopAt
  while ((ConvertTo-LabPathKey $cur) -ne $stop) {
    $item = Get-Item -LiteralPath $cur -Force -ErrorAction SilentlyContinue
    if ($item -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { $links += $cur }
    $parent = Split-Path $cur -Parent
    if (-not $parent -or $parent -eq $cur) { throw "$Dir is not inside $StopAt" }
    $cur = $parent
  }
  return ,$links
}

function Get-LabReparsePoints([string]$Dir) {
  # Every reparse point at or under $Dir, without ever descending into one (same behavior on PowerShell 5.1 and 7).
  $found = @()
  $stack = New-Object System.Collections.Stack
  $stack.Push($Dir)
  while ($stack.Count) {
    $d = $stack.Pop()
    $item = Get-Item -LiteralPath $d -Force -ErrorAction SilentlyContinue
    if (-not $item) { continue }
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { $found += $d; continue }
    if (-not $item.PSIsContainer) { continue }
    foreach ($e in [IO.Directory]::EnumerateFileSystemEntries($d)) { $stack.Push($e) }
  }
  return ,$found
}

function Clear-LabOut([string]$OutDir, [string]$LabDir) {
  $links = @(Get-LabLinksInPath $OutDir $LabDir) + @(Get-LabReparsePoints $OutDir)
  $refusal = Get-LabOutCleanRefusal -OutDir $OutDir -LabDir $LabDir -Links $links
  if ($refusal) { throw "Refusing to clean ${OutDir}: $refusal" }
  Get-ChildItem -LiteralPath $OutDir -Force | Where-Object { $_.Name -ne '.gitkeep' } | Remove-Item -Recurse -Force
}

function Test-LabTools([string]$LabDir) {
  # Problems with shared\tools against shared\tools.sha256 (empty array = fine). Reads only.
  $manifest = Join-Path $LabDir 'shared\tools.sha256'
  if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) { return ,@('manifest not found: shared\tools.sha256') }
  $toolsDir = Join-Path $LabDir 'shared\tools'
  $found = @{}
  if (Test-Path -LiteralPath $toolsDir -PathType Container) {
    $base = (Get-Item -LiteralPath $toolsDir).FullName.TrimEnd('\') + '\'
    Get-ChildItem -LiteralPath $toolsDir -Recurse -File -Force | ForEach-Object { $found[$_.FullName.Substring($base.Length).Replace('\', '/')] = $_.Length }
  }
  return ,(Get-LabMissingTools -ManifestText (Get-Content -LiteralPath $manifest -Raw) -FoundSizes $found)
}