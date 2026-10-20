# pre-push hook logic (T12). Protects the PUBLIC repository (origin = .../claudia-rh) from pushes by mistake
# (git push origin --all / --tags / --mirror, several branches at once, the recovery branch, internal tags).
# Spec: docs/superpowers/specs/2026-10-19-regras-agentes-design.md (P7, P14, 10.9 item 6); agents/versao.md.
#
# Called by the sh launcher "pre-push" in this folder:  pre-push.ps1 <remote-name> <remote-url>
# stdin: one line per ref, "<local ref> <local sha> <remote ref> <remote sha>".
# Exit code: 0 = allow, 10 = refuse (the launcher turns it into exit 1 for git). Any other code (or no code)
# means this logic did not run; the launcher then fails CLOSED for the public repository.
#
# ACTIVATION (once per clone, a decision of the USER; no agent runs this without being asked):
#     powershell -NoProfile -File scripts/dev/install-hooks.ps1
# The installer (T22) copies the launcher and this file into the COMMON hooks folder of the repository
# (git rev-parse --git-common-dir + /hooks), which applies to every worktree and every branch, and checks it with a
# real push to a temporary bare repository. It never overwrites a different hook that was not installed by itself.
# (core.hooksPath, if set, takes precedence over that folder; the installer warns about it and does not touch it.)
# Server side: GitHub branch protection (T13/T14) is the other layer. This hook protects the machine; it does
# not replace the server rules.
#
# PUBLIC remote = URL whose NORMALIZED path ends in claudia-rh (https, ssh git@host:owner/repo, ssh://; case-insensitive).
# Normalization (T19, user decision 2026-10-20) runs before the decision: percent-escapes (%xx) are decoded, credentials
# (user:password@) are dropped, "." and ".." path segments are resolved, trailing slashes and a final ".git" are dropped.
# If the normalization fails, the remote is treated as public (fail closed). The backup (claudia-rh-backup) and any
# other remote pass with no restriction.
# For the public remote every ref must be, as REMOTE ref, one of:
#     refs/heads/feat/*, refs/heads/fix/*, refs/heads/sync/*, refs/tags/v<digits and dots>[-suffix]
# and must be a create/update (never a delete). main and dev change only by PR on GitHub.
# ONE work branch per push (T22, P7/P14/P15): at most ONE remote ref under refs/heads/ per push. Two or more branches in
# one push are refused ("one work branch per push to the public repository; push by named ref"); that is what stops
# "git push origin --all" (it carries every local branch; with a single local branch it equals a named push, which is
# fine). Tags refs/tags/v* count apart: one branch plus new tags passes, subject to the tag rules below. The old limit
# of 20 refs per push stays as a second layer, mainly for tags (--tags).
# The tag format accepted above (v + digits and dots, optional suffix) is a decision of the USER (2026-10-20).
# No force push (T19): for an update (remote sha not all zeros) the old remote commit must be an ancestor of the new
# one (git merge-base --is-ancestor). Otherwise the push is refused as force push / history rewrite. If the old commit
# is missing from the local repository the push is refused too, and "git fetch origin" fixes it. Creating a ref
# (old sha all zeros) stays allowed for the refs in the list.
# Tags are stricter: ANY update of an existing tag on the public remote (moved, or recreated even on the same commit)
# is refused without looking at ancestry, because release.yml fires on v* and would publish something else.
# Creating a new tag stays allowed.
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

# Returns the last path segment of the URL after normalization (lower case, no final ".git"). Throws when nothing is left.
function Get-NormalizedRepoName([string]$Url) {
    $u = $Url.Trim() -replace '[?#].*$', ''
    for ($i = 0; $i -lt 5; $i++) {                       # decode %xx (repeatedly: %252D must not hide a dash)
        $d = [Uri]::UnescapeDataString($u)
        if ($d -ceq $u) { break }
        $u = $d
    }
    $u = $u.Replace('\', '/')
    $u = $u -replace '^[A-Za-z][A-Za-z0-9+.-]*://', ''  # scheme
    $u = $u -replace '^[^/]*@', ''                       # credentials user:password@ (also the git@ of scp form)
    $u = $u.Replace(':', '/')                            # scp form host:owner/repo, or a port
    $stack = New-Object System.Collections.Generic.List[string]
    foreach ($seg in $u.Split('/')) {
        $s = $seg.Trim()
        if ($s -eq '' -or $s -eq '.') { continue }
        if ($s -eq '..') { if ($stack.Count -gt 0) { $stack.RemoveAt($stack.Count - 1) }; continue }
        $stack.Add($s)
    }
    if ($stack.Count -eq 0) { throw "empty path after normalization of '$Url'" }
    return ($stack[$stack.Count - 1] -replace '(?i)\.git$', '').ToLowerInvariant()
}

# True when the URL names the public repository. Unknown URL or failed normalization: public (fail closed).
function Test-PublicRemoteUrl([string]$Url) {
    if (-not $Url) { return $true }
    try { return ((Get-NormalizedRepoName $Url) -ceq 'claudia-rh') } catch { return $true }
}

# Returns the reason an UPDATE (old remote sha not zeros) is refused as force push, or $null when it is a fast-forward.
function Get-ForcePushReason([string]$OldSha, [string]$NewSha) {
    $ErrorActionPreference = "Continue"   # native git stderr must not become a terminating error
    & git cat-file -e "$OldSha^{commit}" 2>$null
    if ($LASTEXITCODE -ne 0) {
        return "the old remote commit $OldSha is not in the local repository, so a fast-forward cannot be proven; run 'git fetch origin' and push again"
    }
    & git merge-base --is-ancestor $OldSha $NewSha 2>$null
    if ($LASTEXITCODE -ne 0) {
        return "force push / history rewrite (the remote commit $OldSha is not an ancestor of $NewSha)"
    }
    return $null
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

    # One work branch per push (T22): count the REMOTE refs under refs/heads/ (--all carries every local branch).
    $branchCount = @($lines | Where-Object {
        $f = @($_.Trim() -split '\s+')
        ($f.Count -ge 3) -and ($f[2] -clike 'refs/heads/*')
    }).Count
    if ($branchCount -gt 1) {
        [void]$problems.Add("$branchCount branches in one push: one work branch per push to the public repository; push by named ref")
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
            if (-not $reason -and $parts[3] -notmatch '^0+$') {
                if ($parts[2] -clike 'refs/tags/*') { $reason = "tag already published; tags are never moved or recreated" }
                else { $reason = Get-ForcePushReason $parts[3] $parts[1] }
            }
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
Write-Err "Push to the public repo only by named ref (feat/*, fix/*, sync/*, tags v*), one work branch per push; never --all, --tags or --mirror."
Write-Err "main and dev change only by PR on GitHub (see agents/versao.md)."
exit $RefuseCode
