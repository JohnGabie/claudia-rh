# Disk helpers for the lab guards. Reading attributes only, except Clear-LabOut, which deletes inside shared\out after
# the pure guard (travas.ps1) said yes. Dot-source travas.ps1 first.
# Defining these functions touches nothing; calling them is what reads the disk.

function Test-LabInSandbox {
  # Reads the OS identity and one guest-only file; the decision itself is the pure Test-LabSandboxDecision.
  $id = [Security.Principal.WindowsIdentity]::GetCurrent().Name
  return Test-LabSandboxDecision -Identity $id -HasBootstrap (Test-Path -LiteralPath $script:LabGuestMarker -PathType Leaf)
}

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

function ConvertTo-LabFullPath([string]$Path, [string]$Base) {
  # Full path with an explicit base (never the CWD). Only 'X:\...' and UNC count as absolute; drive-relative ('C:x') and
  # rooted-without-drive ('\x') are refused because GetFullPath would resolve them against the CWD. Windows PowerShell 5.1
  # has no two-argument GetFullPath, so relative values are joined to the base first. SANDBOX ONLY (touches no disk, but
  # is exercised together with the disk helpers in travas.sandbox.ps1).
  if ([string]::IsNullOrWhiteSpace($Path)) { throw 'empty path' }
  if ($Path -match '^[A-Za-z]:\\' -or $Path -match '^\\\\') { return [IO.Path]::GetFullPath($Path) }
  if ([IO.Path]::IsPathRooted($Path)) { throw "path is neither absolute nor relative (depends on the CWD): $Path" }
  return [IO.Path]::GetFullPath((Join-Path $Base $Path))
}

function Resolve-LabRealPath([string]$FullPath) {
  # Path as the OS reports it for an existing item (SANDBOX ONLY: needs a real disk); a missing item stays as given.
  # Links are not followed here: Get-LabLinksAboveRoot finds them and the pure guards refuse them.
  $item = Get-Item -LiteralPath $FullPath -Force -ErrorAction SilentlyContinue
  if ($item) { return $item.FullName }
  return $FullPath
}

function Get-LabLinksAboveRoot([string]$FullPath) {
  # Reparse points in the path or any ancestor up to the drive root (junction, symlink, subst folder). SANDBOX ONLY.
  return ,@(Get-LabLinksInPath $FullPath ([IO.Path]::GetPathRoot($FullPath)))
}

function Get-LabReparsePoints([string]$Dir) {
  # Every reparse point at or under $Dir, without ever descending into one (same behavior on PowerShell 5.1 and 7).
  $found = @()
  $stack = New-Object System.Collections.Stack
  $stack.Push($Dir)
  while ($stack.Count) {
    $d = $stack.Pop()
    $item = Get-Item -LiteralPath $d -Force -ErrorAction SilentlyContinue
    # Only the starting folder may be absent; a child that cannot be read refuses (it could hide a link).
    if (-not (Test-LabScanEntryOk -IsRoot ($d -eq $Dir) -Found ([bool]$item))) { throw "cannot read $d while looking for links under ${Dir}: refusing" }
    if (-not $item) { continue }
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { $found += $d; continue }
    if (-not $item.PSIsContainer) { continue }
    foreach ($e in [IO.Directory]::EnumerateFileSystemEntries($d)) { $stack.Push($e) }
  }
  return ,$found
}

function Read-LabLinkTarget([string]$LinkPath) {
  # The text a link points to, read WITHOUT following it (Get-Item -Force gives the reparse data; PowerShell 5.1 has .Target,
  # PowerShell 7 also .LinkTarget). $null when unreadable, empty, or more than one target. SANDBOX ONLY.
  try {
    $item = Get-Item -LiteralPath $LinkPath -Force -ErrorAction Stop
    $v = $item.Target
    if ($null -eq $v -and $item.PSObject.Properties['LinkTarget']) { $v = $item.LinkTarget }
    $v = @($v | Where-Object { $null -ne $_ })
    if ($v.Count -ne 1 -or [string]::IsNullOrWhiteSpace([string]$v[0])) { return $null }
    return [string]$v[0]
  } catch { return $null }
}

function Get-LabLinkTargets([string[]]$Paths) {
  # Hashtable link path -> target text (Read-LabLinkTarget); links whose target cannot be read are left out. Never follows a link.
  $r = @{}
  foreach ($p in @($Paths | Where-Object { $_ })) { $t = Read-LabLinkTarget $p; if ($null -ne $t) { $r[$p] = $t } }
  return $r
}

function Get-LabFileLinks([string[]]$Paths) {
  # Of reparse points already found, the ones that are links to a FILE (no Directory attribute). Reads attributes only and
  # never follows the link; anything unreadable is NOT listed, so it stays a refusing link. SANDBOX ONLY.
  $r = @()
  foreach ($p in @($Paths | Where-Object { $_ })) {
    $item = Get-Item -LiteralPath $p -Force -ErrorAction SilentlyContinue
    # A target that cannot be read leaves the link out of the list, so it stays a refusing link.
    if ($item -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -and -not ($item.Attributes -band [IO.FileAttributes]::Directory) -and ($null -ne (Read-LabLinkTarget $p))) { $r += $p }
  }
  return ,$r
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

function Remove-LabCredentialFile {
  # Deletes ONE file, only after Get-LabCredentialCleanRefusal says yes. Never -Recurse.
  param([string]$File, [string]$RepoRoot)
  $p = Get-LabCredentialPaths $RepoRoot
  $links = @(Get-LabLinksInPath $p.Dir $RepoRoot)
  $item = Get-Item -LiteralPath $File -Force -ErrorAction SilentlyContinue
  if ($item -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { $links += $File }
  if ($item -and $item.PSIsContainer) { throw "[credential] refusing to delete a folder: $File" }
  $refusal = Get-LabCredentialCleanRefusal -File $File -RepoRoot $RepoRoot -Links $links
  if ($refusal) { throw "[credential] refusing to delete ${File}: $refusal" }
  if ($item) { Remove-Item -LiteralPath $File -Force }
}

function Remove-LabCredential {
  # test-down / failed test-up: the copy and any .tmp left by an interrupted copy. Nothing else in the folder.
  param([string]$RepoRoot)
  $p = Get-LabCredentialPaths $RepoRoot
  if (-not (Test-Path -LiteralPath $p.Dir -PathType Container)) { return }
  Remove-LabCredentialFile -File $p.File -RepoRoot $RepoRoot
  foreach ($f in @(Get-ChildItem -LiteralPath $p.Dir -Force -File -Filter '.credentials.json.*.tmp')) {
    Remove-LabCredentialFile -File $f.FullName -RepoRoot $RepoRoot
  }
}

function Copy-LabCredential {
  # Copies the login of the account signed in on this PC (spec A4) into .sandbox-work\credencial. The bytes are never
  # read into a variable, printed or logged; errors never carry the user folder.
  param([string]$RepoRoot, [string]$UserProfile, [string]$ConfigDir)
  $src = Join-Path $UserProfile '.claude\.credentials.json'
  $refusal = Get-LabCredentialSourceRefusal -Source $src -UserProfile $UserProfile -ConfigDir $ConfigDir -Links (Get-LabLinksAboveRoot $src)
  if ($refusal) { throw "[credential] refused: $refusal" }
  if (-not (Test-Path -LiteralPath $src -PathType Leaf)) { throw '[credential] no account is signed in on this PC (%USERPROFILE%\.claude\.credentials.json is missing): run claude and /login, then test-up again' }
  $p = Get-LabCredentialPaths $RepoRoot
  New-Item -ItemType Directory -Force -Path $p.Dir | Out-Null
  $links = @(Get-LabLinksInPath $p.Dir $RepoRoot)
  if ($links.Count) { throw "[credential] refused: link (symlink/junction) in $($links[0])" }
  $tmp = "$($p.Dir)\.credentials.json.$([guid]::NewGuid().ToString('N')).tmp"
  try {
    try {
      [IO.File]::Copy($src, $tmp)
      if (Test-Path -LiteralPath $p.File) { [IO.File]::Replace($tmp, $p.File, $null) } else { [IO.File]::Move($tmp, $p.File) }
    } catch { throw '[credential] copy failed (the login file could not be read or written)' }
  } finally {
    if (Test-Path -LiteralPath $tmp) { Remove-LabCredentialFile -File $tmp -RepoRoot $RepoRoot }
  }
}