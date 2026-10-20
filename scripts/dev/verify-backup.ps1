# Read-only report for the P12 backup invariant (agents/versao.md, "Backup (P12)"):
# every branch tip and every tag of <Origin> is equal to, or an ANCESTOR of, the same ref on <Backup>.
# Usage: powershell -NoProfile -File scripts/dev/verify-backup.ps1 [-Origin origin] [-Backup backup] [-RepoDir <folder>]
#
# Reads only: `git ls-remote --heads --tags` on both remotes, `git cat-file -e` and `git merge-base --is-ancestor`
# locally. It never fetches, pushes or writes to disk (stdout/stderr only).
#
# Annotated tags: ls-remote lists the tag object (refs/tags/x) and the peeled commit (refs/tags/x^{}). The peeled
# commit is the one compared, so two different tag objects that point to the same commit count as equal.
#
# Statuses per origin ref:
#   OK              same commit on the backup
#   OK-ANCESTRAL    origin commit is an ancestor of the backup commit (the backup is ahead)
#   ATRASADO        backup commit is an ancestor of the origin commit (the backup is behind)
#   DIVERGE         neither is an ancestor of the other
#   FALTA           the ref does not exist on the backup
#   NAO-VERIFICAVEL a commit object is missing locally (a `git fetch` done by the user would resolve it)
# Exit: 0 all refs OK or OK-ANCESTRAL; 1 any other status; 2 an ls-remote failed (message on stderr).
param(
    [string]$Origin = "origin",
    [string]$Backup = "backup",
    [string]$RepoDir = (Get-Location).Path
)
$ErrorActionPreference = "Stop"

# Windows PowerShell 5.1 turns native stderr into a terminating error under "Stop", so native calls run
# with "Continue" inside these helpers (same care as Get-GitLines in pre-compact.ps1).
function Invoke-Git([string[]]$GitArgs) {
    $ErrorActionPreference = "Continue"
    $out = @(); $err = @()
    try {
        foreach ($item in @(& git -C $RepoDir @GitArgs 2>&1)) {
            if ($item -is [System.Management.Automation.ErrorRecord]) { $err += [string]$item } else { $out += [string]$item }
        }
        return [pscustomobject]@{ Code = $LASTEXITCODE; Out = $out; Err = $err }
    } catch {
        return [pscustomobject]@{ Code = 128; Out = @(); Err = @([string]$_) }
    }
}

# ref -> commit sha (ordinal comparer: ref names are case-sensitive). $null when ls-remote failed.
function Get-RemoteRefs([string]$Remote) {
    $r = Invoke-Git @("ls-remote", "--heads", "--tags", "--", $Remote)
    if ($r.Code -ne 0) {
        $msg = ($r.Err -join " ").Trim()
        [Console]::Error.WriteLine("ERRO: git ls-remote '$Remote' falhou (exit $($r.Code)): $msg")
        return $null
    }
    $direct = New-Object 'System.Collections.Generic.Dictionary[string,string]'
    $peeled = New-Object 'System.Collections.Generic.Dictionary[string,string]'
    foreach ($line in $r.Out) {
        $parts = $line -split "`t", 2
        if ($parts.Count -ne 2) { continue }
        $sha = $parts[0].Trim(); $ref = $parts[1].Trim()
        if ($ref.EndsWith("^{}")) { $peeled[$ref.Substring(0, $ref.Length - 3)] = $sha } else { $direct[$ref] = $sha }
    }
    $map = New-Object 'System.Collections.Generic.Dictionary[string,string]'
    foreach ($k in $direct.Keys) {
        if ($peeled.ContainsKey($k)) { $map[$k] = $peeled[$k] } else { $map[$k] = $direct[$k] }
    }
    return $map
}

function Test-HasCommit([string]$Sha) {
    return ((Invoke-Git @("cat-file", "-e", "$Sha^{commit}")).Code -eq 0)
}
# $true / $false, or $null when git could not answer.
function Test-Ancestor([string]$Maybe, [string]$Of) {
    $c = (Invoke-Git @("merge-base", "--is-ancestor", $Maybe, $Of)).Code
    if ($c -eq 0) { return $true }
    if ($c -eq 1) { return $false }
    return $null
}
function Short([string]$Sha) { if ($Sha.Length -gt 7) { return $Sha.Substring(0, 7) } return $Sha }

if (-not (Test-Path -LiteralPath $RepoDir -PathType Container)) {
    [Console]::Error.WriteLine("ERRO: pasta do repositorio nao existe: $RepoDir")
    exit 2
}

$originRefs = Get-RemoteRefs $Origin
if ($null -eq $originRefs) { exit 2 }
$backupRefs = Get-RemoteRefs $Backup
if ($null -eq $backupRefs) { exit 2 }

$results = @()
foreach ($ref in ($originRefs.Keys | Sort-Object { $_ } -CaseSensitive)) {
    $o = $originRefs[$ref]
    $status = ""; $detail = Short $o
    if (-not $backupRefs.ContainsKey($ref)) {
        $status = "FALTA"; $detail = "$(Short $o) -> (ausente no $Backup)"
    } else {
        $b = $backupRefs[$ref]
        $detail = "$(Short $o) / $(Short $b)"
        if ($o -eq $b) {
            $status = "OK"
        } elseif (-not (Test-HasCommit $o) -or -not (Test-HasCommit $b)) {
            $status = "NAO-VERIFICAVEL"
            $detail += " (objeto ausente localmente; um 'git fetch' feito por voce resolveria)"
        } else {
            $up = Test-Ancestor $o $b
            $down = Test-Ancestor $b $o
            if ($null -eq $up -or $null -eq $down) { $status = "NAO-VERIFICAVEL"; $detail += " (git nao conseguiu comparar)" }
            elseif ($up) { $status = "OK-ANCESTRAL" }
            elseif ($down) { $status = "ATRASADO" }
            else { $status = "DIVERGE" }
        }
    }
    $results += [pscustomobject]@{ Status = $status; Ref = $ref; Detail = $detail }
}

$onlyBackup = @($backupRefs.Keys | Where-Object { -not $originRefs.ContainsKey($_) } | Sort-Object { $_ } -CaseSensitive)

Write-Output "Invariante do backup (P12): $Origin -> $Backup"
Write-Output ("{0,-16} {1,-40} {2}" -f "STATUS", "REF", "ORIGIN / BACKUP")
foreach ($r in $results) { Write-Output ("{0,-16} {1,-40} {2}" -f $r.Status, $r.Ref, $r.Detail) }
if ($onlyBackup.Count -gt 0) {
    Write-Output ""
    Write-Output "So no ${Backup} (informativo):"
    foreach ($ref in $onlyBackup) { Write-Output ("{0,-16} {1,-40} {2}" -f "SO-BACKUP", $ref, (Short $backupRefs[$ref])) }
}

$okStatuses = @("OK", "OK-ANCESTRAL")
$bad = @($results | Where-Object { $okStatuses -notcontains $_.Status })
$exitCode = 0
if ($bad.Count -gt 0) { $exitCode = 1 }
Write-Output ""
Write-Output ("Resumo: {0} refs do {1}, {2} fora da invariante, {3} so no {4}. {5}" -f $results.Count, $Origin, $bad.Count, $onlyBackup.Count, $Backup, $(if ($exitCode -eq 0) { "INVARIANTE VALE" } else { "INVARIANTE NAO VALE" }))
exit $exitCode
