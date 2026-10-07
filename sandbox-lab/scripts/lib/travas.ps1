# Pure guards for the lab scripts: string checks only, no disk access, no [IO.Path] (the tests run on any OS).
# Dot-source this file. A guard returns $null when the action is allowed, otherwise the reason.

$script:LabDeployMarker = '.lab-deploy'
# File that exists only inside the guest (sandbox\ is mapped to C:\labsandbox there). The Node side keeps the same literal
# (MARCADOR_GUEST in caminhos-lab.mjs); each side has a pure test pinning the literal, so a change in one fails the tests.
# Why the two guards differ: this PowerShell guard reads the full DOMAIN\user from WindowsIdentity and requires
# '\WDAGUtilityAccount'. The Node guard gets the identity from os.userInfo(), which may come without a domain, so it also
# accepts the bare user name; both still need this marker file as the second factor.
$script:LabGuestMarker = 'C:\labsandbox\bootstrap.ps1'

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

function Test-LabSandboxDecision {
  # True only with BOTH factors: the process identity (DOMAIN\user, from WindowsIdentity) is WDAGUtilityAccount and the
  # bootstrap folder mapped only in the guest exists. Never decided by $env:USERNAME. Inputs are injected by disco.ps1.
  param([string]$Identity, [bool]$HasBootstrap)
  return ([bool]($Identity -match '\\WDAGUtilityAccount$')) -and $HasBootstrap
}

function Test-LabScanEntryOk {
  # Decision for one entry met while searching for reparse points: only the starting folder may be absent.
  # Any child that cannot be read makes the guard refuse (it could be a link we cannot see).
  param([bool]$IsRoot, [bool]$Found)
  return $Found -or $IsRoot
}

function Test-LabShortNameSegment([string]$Path) {
  # 8.3 short names (PROGRA~1, JOAOGA~1) hide the real folder from a text comparison.
  return [bool]($Path -match '(^|\\)[^\\]*~\d[^\\]*(\\|$)')
}

function Get-LabSourceRefusal {
  # Source of a copy into the deploy (-App): full path, below a drive root, and unrelated to
  # the deploy (not the deploy, not inside it, not a parent of it: robocopy /MIR would copy the deploy into itself).
  # Callers pass the REAL path (disco.ps1: Resolve-LabRealPath) and the reparse points found in it (Get-LabLinksAboveRoot).
  param([string]$Source, [string]$Deploy, [string[]]$Links = @())
  if ([string]::IsNullOrWhiteSpace($Source) -or [string]::IsNullOrWhiteSpace($Deploy)) { return 'source and deploy are required' }
  $s = $Source.TrimEnd('\')
  $why = Get-LabRootProblem $s
  if ($why) { return "source $why" }
  if (Test-LabShortNameSegment $s) { return 'source has an 8.3 short name segment (use the long path)' }
  $l = @($Links | Where-Object { $_ })
  if ($l.Count) { return "link (symlink/junction/subst) in or under the path: $($l[0])" }
  if ([string]::IsNullOrWhiteSpace($s.Substring($s.LastIndexOf('\') + 1))) { return 'source has an empty final name' }
  $sk = ConvertTo-LabPathKey $s
  $dk = ConvertTo-LabPathKey $Deploy
  if ($sk -eq $dk -or $sk.StartsWith("$dk\", [StringComparison]::Ordinal)) { return 'source is the deploy or inside it' }
  if ($dk.StartsWith("$sk\", [StringComparison]::Ordinal)) { return 'source contains the deploy' }
  return $null
}

function Get-LabProjectRefusal {
  # Project folder mapped read-only into the guest: never a drive root, the user folder, or a folder holding it.
  # Callers pass the REAL path and the reparse points found in it, same as Get-LabSourceRefusal.
  param([string]$Project, [string]$UserProfile, [string[]]$Links = @())
  if ([string]::IsNullOrWhiteSpace($Project)) { return 'project is required' }
  $p = $Project.TrimEnd('\')
  $why = Get-LabRootProblem $p
  if ($why) { return "project $why" }
  if (Test-LabShortNameSegment $p) { return 'project has an 8.3 short name segment (use the long path)' }
  $l = @($Links | Where-Object { $_ })
  if ($l.Count) { return "link (symlink/junction/subst) in or under the path: $($l[0])" }
  if ([string]::IsNullOrWhiteSpace($UserProfile)) { return 'user folder is unknown' }
  $pk = ConvertTo-LabPathKey $p
  $uk = ConvertTo-LabPathKey $UserProfile
  if ($pk -eq $uk) { return 'project is the user folder' }
  if ($uk.StartsWith("$pk\", [StringComparison]::Ordinal)) { return 'project contains the user folder' }
  return $null
}

function Get-LabDownRefusal {
  # test-down.ps1 must report success before shared\out is cleaned.
  param($ExitCode)
  if ($null -eq $ExitCode -or "$ExitCode" -ne '0') { return "test-down did not finish cleanly (exit code '$ExitCode'); shared\out was not cleaned" }
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


# Root of every folder the Sandbox tests create in the guest (spec A6): a VHD mounted as Z: by sandbox\disco-z.ps1.
# Node keeps the same literal (RAIZ_TESTE_GUEST in caminhos-lab.mjs); a pure test on each side pins it.
$script:LabGuestTestRoot = 'Z:\claudia-teste-sandbox'

function Get-LabHostDriveRefusal {
  # The Z: drive exists only in the guest. If the host has one, a test that slipped out of the guest would hit it.
  param([string[]]$DriveNames = @())
  foreach ($d in @($DriveNames)) {
    if ("$d" -match '^[Zz]:') { return 'the host has a Z: drive; the lab uses Z: only inside the guest, so test-up refuses to start (remove or remap that drive first)' }
  }
  return $null
}

function Get-LabGuestTestRootRefusal([string]$RealPath) {
  # Callers pass the REAL path (Get-Item FullName / realpath) of a folder the Sandbox tests are about to use.
  if ([string]::IsNullOrWhiteSpace($RealPath)) { return 'path is required' }
  if ($RealPath -match '(^|\\)\.{1,2}(\\|$)') { return "has '.' or '..' segments" }
  $k = ConvertTo-LabPathKey $RealPath
  $r = ConvertTo-LabPathKey $script:LabGuestTestRoot
  if ($k -eq $r -or $k.StartsWith("$r\", [StringComparison]::Ordinal)) { return $null }
  return "real path is not under $script:LabGuestTestRoot"
}

function Get-LabCredentialPaths([string]$RepoRoot) {
  # Where test-up puts the copy of the Claude Code login (ignored by git: .sandbox-work/).
  $dir = $RepoRoot.TrimEnd('\') + '\.sandbox-work\credencial'
  return [ordered]@{ Dir = $dir; File = "$dir\.credentials.json" }
}

function Get-LabCredentialSourceRefusal {
  # Only the default login file of the user running test-up. Callers pass [Environment]::GetFolderPath('UserProfile'),
  # $env:CLAUDE_CONFIG_DIR and the reparse points above the source. Messages never print the user folder.
  param([string]$Source, [string]$UserProfile, [string]$ConfigDir, [string[]]$Links = @())
  if (-not [string]::IsNullOrWhiteSpace($ConfigDir)) { return 'CLAUDE_CONFIG_DIR is set: the lab only copies the default login (%USERPROFILE%\.claude\.credentials.json); unset it for test-up' }
  if ([string]::IsNullOrWhiteSpace($Source) -or [string]::IsNullOrWhiteSpace($UserProfile)) { return 'source and user folder are required' }
  $up = $UserProfile.TrimEnd('\')
  $why = Get-LabRootProblem $up
  if ($why) { return "user folder $why" }
  if (Test-LabShortNameSegment $Source) { return 'source has an 8.3 short name segment' }
  if ((ConvertTo-LabPathKey $Source) -ne (ConvertTo-LabPathKey "$up\.claude\.credentials.json")) { return 'source is not exactly %USERPROFILE%\.claude\.credentials.json' }
  $l = @($Links | Where-Object { $_ })
  if ($l.Count) { return 'link (symlink/junction) in the path of the login file' }
  return $null
}

function Get-LabCredentialCleanRefusal {
  # The only files test-up/test-down may delete for the credential: the copy itself, or a .tmp left by Copy-LabCredential.
  param([string]$File, [string]$RepoRoot, [string[]]$Links = @())
  if ([string]::IsNullOrWhiteSpace($File) -or [string]::IsNullOrWhiteSpace($RepoRoot)) { return 'file and repo root are required' }
  $root = $RepoRoot.TrimEnd('\')
  $why = Get-LabRootProblem $root
  if ($why) { return "repo root $why" }
  $p = Get-LabCredentialPaths $root
  $fk = ConvertTo-LabPathKey $File
  $tmpPattern = '^' + [regex]::Escape((ConvertTo-LabPathKey $p.Dir)) + '\\\.credentials\.json\.[0-9a-f]{32}\.tmp$'
  if ($fk -ne (ConvertTo-LabPathKey $p.File) -and $fk -notmatch $tmpPattern) { return "file is not exactly $($p.File) (or its .tmp copy)" }
  $l = @($Links | Where-Object { $_ })
  if ($l.Count) { return "link (symlink/junction) in or under the path: $($l[0])" }
  return $null
}

function Get-LabCacheDirRefusal {
  # cache-up.ps1 writes only inside exactly <lab>\shared\cache, with no link on the way.
  param([string]$CacheDir, [string]$LabDir, [string[]]$Links = @())
  if ([string]::IsNullOrWhiteSpace($CacheDir) -or [string]::IsNullOrWhiteSpace($LabDir)) { return 'cache dir and lab dir are required' }
  $lab = $LabDir.TrimEnd('\')
  $why = Get-LabRootProblem $lab
  if ($why) { return "lab dir $why" }
  $expected = "$lab\shared\cache"
  if ((ConvertTo-LabPathKey $CacheDir) -ne (ConvertTo-LabPathKey $expected)) { return "cache dir is not exactly $expected" }
  $l = @($Links | Where-Object { $_ })
  if ($l.Count) { return "link (symlink/junction) in or under the path: $($l[0])" }
  return $null
}

function Get-LabCachePlan {
  # $Pacotes: objects with nome, arquivo, sha256 ('' for a vs-layout folder). $Existentes: arquivo -> lowercase sha256,
  # or 'dir' for a folder. A file that exists with another hash is a conflict: cache-up never overwrites nor deletes.
  param([object[]]$Pacotes, [hashtable]$Existentes)
  $plan = @()
  foreach ($p in @($Pacotes)) {
    $key = @($Existentes.Keys | Where-Object { $_ -ieq $p.arquivo }) | Select-Object -First 1
    $acao = 'baixar'; $motivo = 'not in the cache'
    if ($null -ne $key) {
      $found = [string]$Existentes[$key]
      if ([string]::IsNullOrEmpty($p.sha256)) {
        if ($found -eq 'dir') { $acao = 'ok'; $motivo = 'folder present' } else { $acao = 'conflito'; $motivo = 'a file has the name of the layout folder' }
      } elseif ($found -eq 'dir') { $acao = 'conflito'; $motivo = 'a folder has the name of the file' }
      elseif ($found -ieq $p.sha256) { $acao = 'ok'; $motivo = 'sha256 matches' }
      else { $acao = 'conflito'; $motivo = 'sha256 differs (file kept; remove it by hand if it should be replaced)' }
    }
    $plan += [ordered]@{ nome = $p.nome; arquivo = $p.arquivo; acao = $acao; motivo = $motivo }
  }
  # No unary comma: callers wrap the call in @(), and a leading comma would make that a one-element array.
  return $plan
}

$script:LabAppId = 'io.github.johngabie.claudia-rh'

function Get-LabLinkTargetOf($LinkTargets, [string]$LinkPath) {
  # Looks the target of one link up in the table made by Get-LabLinkTargets (case and slash insensitive keys). $null if absent.
  if ($null -eq $LinkTargets -or $LinkTargets -isnot [System.Collections.IDictionary]) { return $null }
  $want = ConvertTo-LabPathKey ($LinkPath -replace '/', '\')
  foreach ($key in @($LinkTargets.Keys)) {
    if ((ConvertTo-LabPathKey (([string]$key) -replace '/', '\')) -eq $want) { return $LinkTargets[$key] }
  }
  return $null
}

function Test-LabLatestLinkTargetOk {
  # The target of diagnostics\latest read without following it: exactly 'claude-startup.log' (relative) or exactly
  # <Source>\diagnostics\claude-startup.log, ignoring case and with '/' read as '\'. Nothing else: no '..', no 8.3 '~', no UNC,
  # no other drive, no extra '.', no trailing space or dot, nothing empty or unreadable (an array is not a single target).
  param($Target, [string]$Source)
  if ($null -eq $Target -or $Target -isnot [string]) { return $false }
  if ([string]::IsNullOrWhiteSpace($Target) -or [string]::IsNullOrWhiteSpace($Source)) { return $false }
  $t = ($Target -replace '/', '\').ToLowerInvariant()   # no TrimEnd: a trailing backslash is not the exact target
  if ($t -eq 'claude-startup.log') { return $true }
  return ($t -eq (ConvertTo-LabPathKey ($Source.TrimEnd('\') + '\diagnostics\claude-startup.log')))
}

function Get-LabRealDataSourceRefusal {
  # test-up -DadosReais (spec addendum 8.1): only the app's folder under Roaming, read-only, never %LOCALAPPDATA%.
  # Callers pass [Environment]::GetFolderPath('ApplicationData'), the reparse points above the source ($Links) and the ones
  # found INSIDE it ($LinksInside): any link inside refuses too (the copy scenario refuses links, it never follows them).
  # One exception, nothing else: the FILE link <Source>\diagnostics\latest (made by the Claude Code debug logger) is skipped,
  # never followed. $FileLinksInside lists which of the $LinksInside are links to a FILE (not a folder or junction); the same
  # name as a folder link or junction, at another level, or any other link still refuses.
  # $LinkTargets (hashtable: link path -> the target text read WITHOUT following the link, Get-LabLinkTargets) is required for
  # the exception: the target must be exactly claude-startup.log or <Source>\diagnostics\claude-startup.log (Test-LabLatestLinkTargetOk).
  param([string]$Source, [string]$RoamingAppData, [string[]]$Links = @(), [string[]]$LinksInside = @(), [string[]]$FileLinksInside = @(), $LinkTargets = $null)
  if ([string]::IsNullOrWhiteSpace($Source) -or [string]::IsNullOrWhiteSpace($RoamingAppData)) { return 'source and roaming folder are required' }
  $r = $RoamingAppData.TrimEnd('\')
  $why = Get-LabRootProblem $r
  if ($why) { return "roaming folder $why" }
  if ($r -match '\\Local$' -or $Source -match '\\AppData\\Local(\\|$)') { return 'never %LOCALAPPDATA% (browser profile, cookies)' }
  if (Test-LabShortNameSegment $Source) { return 'source has an 8.3 short name segment' }
  if ((ConvertTo-LabPathKey $Source) -ne (ConvertTo-LabPathKey "$r\$script:LabAppId")) { return "source is not exactly %APPDATA%\$script:LabAppId" }
  $l = @($Links | Where-Object { $_ })
  if ($l.Count) { return 'link (symlink/junction) in the path of the app data folder' }
  $skipKey = ConvertTo-LabPathKey "$($Source.TrimEnd('\'))\diagnostics\latest"
  $fileKeys = @(@($FileLinksInside) | Where-Object { $_ } | ForEach-Object { ConvertTo-LabPathKey ($_ -replace '/', '\') })
  $blocking = @(@($LinksInside) | Where-Object { $_ } | Where-Object {
    $k = ConvertTo-LabPathKey ($_ -replace '/', '\')
    -not ($k -eq $skipKey -and $fileKeys -contains $k -and (Test-LabLatestLinkTargetOk -Target (Get-LabLinkTargetOf $LinkTargets $_) -Source $Source))
  })
  if ($blocking.Count) { return 'link (symlink/junction) inside the app data folder: the copy scenario refuses links' }
  return $null
}

function Get-LabLayoutImportRefusal {
  # cache-up.ps1 -ImportarMsvc reads only the folder layout-msvc.ps1 wrote: <repo>\.sandbox-work\cache-saida\<8 hex>\msvc-<versao>.
  param([string]$Source, [string]$RepoRoot, [string]$Versao, [string[]]$Links = @())
  if ([string]::IsNullOrWhiteSpace($Source) -or [string]::IsNullOrWhiteSpace($RepoRoot)) { return 'source and repo root are required' }
  $root = $RepoRoot.TrimEnd('\')
  $why = Get-LabRootProblem $root
  if ($why) { return "repo root $why" }
  if ("$Versao" -notmatch '^\d+(\.\d+){1,3}$') { return 'versao must be digits and dots' }
  $pattern = '^' + [regex]::Escape((ConvertTo-LabPathKey "$root\.sandbox-work\cache-saida")) + '\\[0-9a-f]{8}\\' + [regex]::Escape("msvc-$Versao".ToLowerInvariant()) + '$'
  if ((ConvertTo-LabPathKey $Source) -cnotmatch $pattern) { return "source is not <repo>\.sandbox-work\cache-saida\<id>\msvc-$Versao" }
  $l = @($Links | Where-Object { $_ })
  if ($l.Count) { return "link (symlink/junction) in or under the path: $($l[0])" }
  return $null
}

function Get-LabLayoutManifestProblems {
  # layout.sha256 (written in the guest) against files on disk: every listed file present with the same size and SHA-256,
  # and nothing extra. $Found: relative path with '/' -> "<sha256> <bytes>". Empty array = identical.
  param([string]$ManifestText, [hashtable]$Found)
  $problems = New-Object System.Collections.Generic.List[string]
  $foundLc = @{}
  foreach ($k in $Found.Keys) { $foundLc[$k.ToLowerInvariant()] = @($k, [string]$Found[$k]) }
  $listed = @{}
  foreach ($line in ($ManifestText -split "`r?`n")) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    if ($line -notmatch '^([0-9a-f]{64}) (\d+) (.+)$') { $problems.Add("malformed manifest line: $line"); continue }
    $rel = $Matches[3]; $want = "$($Matches[1]) $($Matches[2])"
    $listed[$rel.ToLowerInvariant()] = $true
    $hit = $foundLc[$rel.ToLowerInvariant()]
    if ($null -eq $hit) { $problems.Add("missing: $rel") } elseif ($hit[1] -ne $want) { $problems.Add("differs: $rel") }
  }
  if ($listed.Count -eq 0 -and $problems.Count -eq 0) { $problems.Add('manifest is empty') }
  foreach ($k in $foundLc.Keys) { if (-not $listed.ContainsKey($k)) { $problems.Add("not in the manifest: $($foundLc[$k][0])") } }
  # No unary comma: callers wrap the call in @() (an empty result must count 0).
  return $problems.ToArray()
}