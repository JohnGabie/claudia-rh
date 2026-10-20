# pre-push hook logic (T12). Protects the PUBLIC repository (origin = .../claudia-rh) from pushes by mistake
# (git push origin --all / --tags / --mirror, work branches, the recovery branch, internal tags).
# Spec: docs/superpowers/specs/2026-10-19-regras-agentes-design.md (P7, P14, 10.9 item 6); agents/versao.md.
#
# Called by the sh launcher "pre-push" in this folder:  pre-push.ps1 <remote-name> <remote-url>
# stdin: one line per ref, "<local ref> <local sha> <remote ref> <remote sha>".
# Exit code: 0 = allow, 10 = refuse (the launcher turns it into exit 1 for git). Any other code (or no code)
# means this logic did not run; the launcher then fails CLOSED for the public repository.
#
# ACTIVATION (once per clone, a decision of the USER; no agent runs this without being asked):
#     git config core.hooksPath scripts/dev/git-hooks
# Server side: GitHub branch protection (T13/T14) is the other layer. This hook protects the machine; it does
# not replace the server rules.
#
# PUBLIC remote = URL ending in /claudia-rh or /claudia-rh.git (https or ssh, case-insensitive). The backup
# (claudia-rh-backup) and any other remote pass with no restriction.
# For the public remote every ref must be, as REMOTE ref, one of:
#     refs/heads/feat/*, refs/heads/fix/*, refs/heads/sync/*, refs/tags/v<digits and dots>[-suffix]
# and must be a create/update (never a delete). main and dev change only by PR on GitHub. At most 20 refs per push.
#
# ESCAPE HATCH (user only): CLAUDIA_ALLOW_PUBLIC_PUSH=1 lets the push through with a warning on stderr.
# Decision: it frees EVERYTHING, deletes included, because it is the conscious decision of the user.
# No agent sets this variable without an explicit request from the user.
#
# Fail closed: if this logic throws, the public push is refused; other remotes pass.
# Test hook: CLAUDIA_PREPUSH_TEST_THROW=1 makes the logic throw (used only by test-pre-push.ps1).
# Only reads; never deletes or writes anything.
$ErrorActionPreference = "Stop"
$RefuseCode = 10
$MaxRefs = 20

function Write-Err([string]$Text) { [Console]::Error.WriteLine($Text) }

# True when the URL names the public repository (claudia-rh or claudia-rh.git as the last path part).
function Test-PublicRemoteUrl([string]$Url) {
    if (-not $Url) { return $true }   # unknown URL: treat as public (fail closed)
    return ($Url.Trim() -imatch '[/\\:]claudia-rh(\.git)?[/\\]*$')
}

# Returns the reason a remote ref is refused, or $null when it is allowed (create/update only).
function Get-RefusalReason([string]$LocalSha, [string]$RemoteRef) {
    if ($LocalSha -match '^0+$') {
        return "deleting a remote ref is not allowed"
    }
    if ($RemoteRef -ceq "refs/heads/main" -or $RemoteRef -ceq "refs/heads/dev") {
        return "main and dev change only by PR on GitHub"
    }
    if ($RemoteRef -cmatch '^refs/heads/(feat|fix|sync)/.+') { return $null }
    if ($RemoteRef -cmatch '^refs/tags/v[0-9]+(\.[0-9]+)*(-[0-9A-Za-z][0-9A-Za-z.-]*)?$') { return $null }
    if ($RemoteRef -match '^refs/(heads|tags)/(recovery|wip/|archive/|backup/|pre-rebase/)') {
        return "internal ref (recovery, wip, archive, backup, pre-rebase) never goes to the public repository"
    }
    return "not in the allowed list (feat/*, fix/*, sync/*, tags v*)"
}

$remoteName = if ($args.Count -ge 1) { [string]$args[0] } else { "" }
$remoteUrl = if ($args.Count -ge 2) { [string]$args[1] } else { "" }

$public = $true
try { $public = Test-PublicRemoteUrl $remoteUrl } catch { $public = $true }
if (-not $public) { exit 0 }

$problems = New-Object System.Collections.ArrayList
try {
    if ($env:CLAUDIA_PREPUSH_TEST_THROW -eq "1") { throw "simulated failure (test)" }

    $raw = [Console]::In.ReadToEnd()
    $lines = @($raw -split "`r?`n" | Where-Object { $_.Trim() })

    if ($env:CLAUDIA_ALLOW_PUBLIC_PUSH -eq "1") {
        Write-Err "pre-push: WARNING: CLAUDIA_ALLOW_PUBLIC_PUSH=1 - push to the PUBLIC repository allowed without checks ($($lines.Count) ref(s))."
        exit 0
    }

    if ($lines.Count -gt $MaxRefs) {
        [void]$problems.Add("$($lines.Count) refs in one push (limit $MaxRefs): looks like --all, --tags or --mirror")
    } else {
        foreach ($line in $lines) {
            $parts = @($line.Trim() -split '\s+')
            if ($parts.Count -lt 4) {
                [void]$problems.Add("unreadable ref line '$line'")
                continue
            }
            $reason = Get-RefusalReason $parts[1] $parts[2]
            if ($reason) { [void]$problems.Add("$($parts[2]): $reason") }
        }
    }
} catch {
    Write-Err "pre-push: REFUSED. The check failed unexpectedly ($($_.Exception.Message)); failing closed for the public repository."
    Write-Err "pre-push: push to the public repo only by named ref; main/dev change only by PR on GitHub (see agents/versao.md)."
    exit $RefuseCode
}

if ($problems.Count -eq 0) { exit 0 }

Write-Err "pre-push: REFUSED push to the PUBLIC repository '$remoteName' ($remoteUrl)."
foreach ($p in $problems) { Write-Err "  refused ref -> $p" }
Write-Err "Push to the public repo only by named ref (feat/*, fix/*, sync/*, tags v*), one at a time; never --all, --tags or --mirror."
Write-Err "main and dev change only by PR on GitHub (see agents/versao.md)."
exit $RefuseCode
