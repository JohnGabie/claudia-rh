# Pure guards for the lab scripts: string checks only, no disk access, no [IO.Path] (the tests run on any OS).
# Dot-source this file. A guard returns $null when the action is allowed, otherwise the reason.

$script:LabDeployMarker = '.lab-deploy'

function ConvertTo-LabPathKey([string]$Path) {
  # Windows paths compare case-insensitively. No separator rewriting: '/' never matches the exact expected path.
  return $Path.TrimEnd('\').ToLowerInvariant()
}

function Get-LabRootProblem([string]$Root) {
  if ($Root -notmatch '^[A-Za-z]:\\[^\\]') { return 'must be an absolute local path below a drive root' }
  if ($Root -match '(^|\\)\.{1,2}(\\|$)') { return "has '.' or '..' segments" }
  return $null
}

function Get-LabCleanRefusal {
  param([string]$Deploy, [string]$RepoRoot, [bool]$Exists, [bool]$HasMarker, [string[]]$Links = @())
  if ([string]::IsNullOrWhiteSpace($Deploy) -or [string]::IsNullOrWhiteSpace($RepoRoot)) { return 'deploy and repo root are required' }
  $root = $RepoRoot.TrimEnd('\')
  $why = Get-LabRootProblem $root
  if ($why) { return "repo root $why" }
  $expected = "$root\.sandbox-work\deploy"
  if ((ConvertTo-LabPathKey $Deploy) -ne (ConvertTo-LabPathKey $expected)) { return "deploy is not exactly $expected" }
  $l = @($Links | Where-Object { $_ })
  if ($l.Count) { return "link (symlink/junction) in or under the path: $($l[0])" }
  if (-not $Exists) { return $null }
  if (-not $HasMarker) { return "deploy exists without the $script:LabDeployMarker marker (not created by the lab)" }
  return $null
}

function Get-LabOutCleanRefusal {
  param([string]$OutDir, [string]$LabDir, [string[]]$Links = @())
  if ([string]::IsNullOrWhiteSpace($OutDir) -or [string]::IsNullOrWhiteSpace($LabDir)) { return 'out dir and lab dir are required' }
  $lab = $LabDir.TrimEnd('\')
  $why = Get-LabRootProblem $lab
  if ($why) { return "lab dir $why" }
  $expected = "$lab\shared\out"
  if ((ConvertTo-LabPathKey $OutDir) -ne (ConvertTo-LabPathKey $expected)) { return "out dir is not exactly $expected" }
  $l = @($Links | Where-Object { $_ })
  if ($l.Count) { return "link (symlink/junction) in or under the path: $($l[0])" }
  return $null
}

function Get-LabMissingTools {
  # Compares shared\tools.sha256 with the files found (relative path with '/', size). Size only: hashing ~600 MB on
  # every test-up is too slow; validate-config and the copy check use the hash. Extra files are not a problem.
  param([string]$ManifestText, [hashtable]$FoundSizes)
  $problems = New-Object System.Collections.Generic.List[string]
  $entries = 0
  foreach ($line in ($ManifestText -split "`r?`n")) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    if ($line -notmatch '^([0-9a-f]{64}) (\d+) (.+)$') { $problems.Add("malformed manifest line: $line"); continue }
    $entries++
    $size = [long]$Matches[2]; $rel = $Matches[3]
    $key = @($FoundSizes.Keys | Where-Object { $_ -ieq $rel }) | Select-Object -First 1
    if ($null -eq $key) { $problems.Add("missing: $rel") }
    elseif ([long]$FoundSizes[$key] -ne $size) { $problems.Add("size differs: $rel") }
  }
  if ($entries -eq 0 -and $problems.Count -eq 0) { $problems.Add('manifest is empty') }
  return ,$problems.ToArray()
}