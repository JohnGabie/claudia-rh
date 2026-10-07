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
