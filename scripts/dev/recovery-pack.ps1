# Recovery pack (agents/versao.md "Pacote de recuperacao (P14 e P17)"): writes a NEW commit on the ORPHAN branch
# `recovery` with what is needed to resume work if this PC is lost: project config, global config and memory, and
# work state (STATE.md, patches of uncommitted work, context diaries). Cleaned of secrets.
# Usage: powershell -NoProfile -File scripts/dev/recovery-pack.ps1 [-RepoDir <folder>] [-GlobalClaudeDir <folder>]
#        [-MemoryDir <folder>] [-WhatIf]
#
# NEVER pushes, fetches or runs ls-remote. The versionador pushes it with `git push backup recovery`, and only to
# the backup: the pre-push hook and the push rules refuse `recovery*` on the origin.
#
# Mechanism (no worktree, nothing is deleted): the content is assembled in a NEW temp folder
# (<temp>\recovery-pack-<guid>\tree) and committed by plumbing with a temporary index (GIT_INDEX_FILE inside that
# temp folder): `git --work-tree=<tree> add -A -f`, `git write-tree`, `git commit-tree` (parent = current tip of
# refs/heads/recovery, none the first time, which makes the branch orphan) and
# `git update-ref refs/heads/recovery <new> <expected old or zeros>`. Only refs/heads/recovery is ever written.
# HEAD, the user's index and the user's branches are never touched. The temp folder is left behind (it is never
# deleted by this script); its path is printed.
#
# Allowlist (nothing enters by default):
#   project/  <main checkout>\.claude\skills\**, .claude\commands\**, .claude\settings.json,
#             .claude\settings.local.json, .mcp.json. NEVER .claude\worktrees\ (full copies of the repository).
#   global/   <GlobalClaudeDir>\CLAUDE.md, settings.json, plugins.txt (names and versions only, from
#             <GlobalClaudeDir>\plugins\installed_plugins.json) and memory\ (-MemoryDir).
#   STATE.md, patches/<branch>.patch (git diff HEAD of each worktree with changes), journal/ (diaries of the main
#   checkout .journal\ plus the ones that exist only in some worktree .journal\).
# Always out, wherever it shows up (names compared ignoring case): files .env*, *.pem, *.key, *.pfx, *.p12,
#   id_rsa*, .credentials.json, *credential*, the personal profile files, *.db, *.sqlite*, files over 5 MB, files
#   that cannot be scanned as text (NUL bytes without a UTF-16/UTF-32 BOM: binary or BOM-less UTF-16); folders .git,
#   .env*, *credential*, secrets, .claude/worktrees and sandbox-lab/shared/secrets; and links: a junction/symlink is
#   never followed, neither inside a tree nor as the root of a source (.claude, .claude\skills, .claude\commands,
#   .journal, -MemoryDir, -GlobalClaudeDir), nor at any level in between: a link anywhere from -GlobalClaudeDir
#   down to the memory folder keeps the memory out (reason in STATE.md), and the same from the repository root down
#   to .claude\skills and .claude\commands. Folders ABOVE -GlobalClaudeDir or the repository root are not checked
#   (redirected profile folders are legitimate). Excluded files are never opened (except the NUL-byte probe).
# Every JSON is cleaned structurally before the copy (P17): keys `env` and `headers` and any key whose name matches
#   (?i)(key|token|secret|password|passwd|credential|auth|cookie) are removed recursively; invalid JSON is not copied.
# Second layer: a regex scan of EVERYTHING to be written, content AND relative path names (STATE.md and patches
#   included). Any hit: no commit, no ref change, exit 1, and the output names the file and the pattern, never the
#   value (a path that holds a secret is printed with the secret masked). Every message printed (the script's own
#   and git's error text: add, write-tree, commit-tree, update-ref, diff) passes through Hide-Secrets.
#
# SECRET PATTERNS: the list $SecretPatterns below IS the project's reference list for secret scans (the
#   "patterns of 2026-10-19" cited by docs/superpowers/specs/2026-10-19-regras-agentes-design.md). Change it here.
#
# -MemoryDir default: <GlobalClaudeDir>\projects\<encoded main checkout path>\memory, where the encoding is the one
#   Claude Code uses for its project folders: every character that is not an ASCII letter or digit becomes "-"
#   (e.g. "C:\a\b-c" -> "C--a-b-c"). Pass -MemoryDir to override.
# -TargetRef exists only so the ref guard can be tested: any value other than refs/heads/recovery is refused at
#   the start, before anything is built. -TestLateGuard (tests only) skips that early check so the second guard,
#   inside the only ref writer, can be proven too.
# -TestPretendNoRecovery exists only for tests: behaves as if refs/heads/recovery did not exist yet (simulates a
#   concurrent writer), so update-ref must fail on the expected old value.
# Exit: 0 committed or nothing changed (also -WhatIf); 1 the scan found something, the ref guard refused, the ref
#   moved during the run, or another content error; 2 environment error (git missing, not a repository).
param(
    [string]$RepoDir = (Get-Location).Path,
    [string]$GlobalClaudeDir = "",
    [string]$MemoryDir = "",
    [switch]$WhatIf,
    [string]$TargetRef = "refs/heads/recovery",
    [switch]$TestPretendNoRecovery,
    [switch]$TestLateGuard
)
$ErrorActionPreference = "Stop"

$SecretPatterns = @(
    @("anthropic-key", 'sk-ant-[A-Za-z0-9_\-]{20,}'),
    @("github-token", 'gh[opsu]_[A-Za-z0-9]{20,}'),
    @("github-pat", 'github_pat_[A-Za-z0-9_]{20,}'),
    @("aws-access-key", 'AKIA[0-9A-Z]{16}'),
    @("private-key", '-----BEGIN [A-Z ]*PRIVATE KEY-----'),
    @("slack-token", 'xox[baprs]-[A-Za-z0-9\-]{10,}'),
    @("google-api-key", 'AIza[0-9A-Za-z_\-]{35}'),
    @("linear-key", 'lin_api_[A-Za-z0-9]{20,}'),
    @("jwt", 'eyJ[A-Za-z0-9_\-]{10,}\.eyJ[A-Za-z0-9_\-]{10,}'),
    @("openai-project-key", 'sk-proj-[A-Za-z0-9_\-]{20,}'),
    @("sk-key", '(?<![A-Za-z0-9])sk-[A-Za-z0-9_\-]{20,}'),
    # name = value, quoted or not; no word boundary on the left so ANTHROPIC_AUTH_TOKEN / OPENAI_API_KEY match.
    @("generic-assignment", '(?i)(?<![A-Za-z0-9])(api[_-]?key|secret|passwd|password|token)(?:[_-][A-Za-z0-9]+)*["'']?\s*[:=]\s*["'']?[^"''\s]{16,}'),
    # camelCase / PascalCase names in plain text (accessToken, clientSecret, apiKey...): the generic pattern above
    # needs a non-alphanumeric char before the word, so "accessToken" or "clientSecret" escape it.
    @("camel-assignment", '(?i)(?<![A-Za-z0-9])(?:(?:access|refresh|auth|bearer|id|session|api|client)[_-]?token|(?:api|private|secret|client|access|signing|encryption)[_-]?key|(?:api|client|app)[_-]?secret)["'']?\s*[:=]\s*["'']?[^"''\s]{16,}')
)
# Masks every secret-looking substring. EVERY message this script prints (its own and git's) goes through it.
function Hide-Secrets([string]$Text) {
    foreach ($p in $script:SecretPatterns) { $Text = [regex]::Replace($Text, $p[1], "[oculto]") }
    return $Text
}
function Say([string]$Text) { Write-Output (Hide-Secrets $Text) }
function Say-Err([string]$Text) { [Console]::Error.WriteLine((Hide-Secrets $Text)) }
# Text of a git error (stderr lines joined), masked. Used for every git failure that is printed or recorded.
function Format-GitError($Result) { return (Hide-Secrets (($Result.Err -join ' ').Trim())) }

# Early ref guard: refuse before building anything.
if ($TargetRef -cne "refs/heads/recovery" -and -not $TestLateGuard) {
    Say "ERRO: recusado: este script so escreve refs/heads/recovery (pedido: $TargetRef). Nada foi montado."
    exit 1
}

# Never let git or the credential manager open a prompt (this script makes no network call, but be safe).
$env:GIT_TERMINAL_PROMPT = "0"
$env:GCM_INTERACTIVE = "never"

$MaxBytes = 5MB
$Utf8 = New-Object System.Text.UTF8Encoding($false)
$SecretKeyRegex = '(?i)(key|token|secret|password|passwd|credential|auth|cookie)'
# True when the path exists and is a junction or symlink.
function Test-IsLink([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return $false }
    $i = Get-Item -LiteralPath $Path -Force
    return [bool]($i.Attributes -band [IO.FileAttributes]::ReparsePoint)
}
# First junction/symlink on the way from $Root down to $Target, as the path relative to $Root ("." for $Root
# itself, checked only with -IncludeRoot), or $null when there is none or $Target is not under $Root. Folders
# ABOVE $Root are never checked (redirected profile folders are legitimate).
function Get-LinkBetween([string]$Root, [string]$Target, [switch]$IncludeRoot) {
    $rootN = [IO.Path]::GetFullPath($Root).TrimEnd('\')
    $targetN = [IO.Path]::GetFullPath($Target).TrimEnd('\')
    if ($IncludeRoot -and (Test-IsLink $rootN)) { return "." }
    if ($targetN -ieq $rootN) { return $null }
    if (-not $targetN.StartsWith($rootN + "\", [StringComparison]::OrdinalIgnoreCase)) { return $null }
    $cur = $rootN
    $rel = ""
    foreach ($seg in ($targetN.Substring($rootN.Length + 1) -split '\\')) {
        if (-not $seg) { continue }
        $cur = Join-Path $cur $seg
        $rel = if ($rel) { "$rel/$seg" } else { $seg }
        if (Test-IsLink $cur) { return $rel }
    }
    return $null
}
# False when the file holds NUL bytes without a UTF-16/UTF-32 BOM (binary or BOM-less UTF-16): it cannot be
# scanned as text safely, so it never enters.
function Test-Scannable([string]$Path) {
    $b = [IO.File]::ReadAllBytes($Path)
    if ($b.Length -ge 2 -and (($b[0] -eq 0xFF -and $b[1] -eq 0xFE) -or ($b[0] -eq 0xFE -and $b[1] -eq 0xFF))) { return $true }
    if ($b.Length -ge 4 -and $b[0] -eq 0 -and $b[1] -eq 0 -and $b[2] -eq 0xFE -and $b[3] -eq 0xFF) { return $true }
    return ([Array]::IndexOf($b, [byte]0) -lt 0)
}

# --- Shared diary helpers (copied from journal-skeleton.ps1 / pre-compact.ps1; keep identical) ---
# Output lines of a git command run in $Dir; empty when git fails or is missing. Windows PowerShell 5.1
# turns native stderr into a terminating error under "Stop", so stderr is dropped with "Continue" here.
function Get-GitLines([string]$Dir, [string[]]$GitArgs) {
    $ErrorActionPreference = "Continue"
    try {
        $out = @(& git -C $Dir @GitArgs 2>$null)
        if ($LASTEXITCODE -ne 0) { return @() }
        return @($out | ForEach-Object { [string]$_ })
    } catch { return @() }
}
# First non-empty output line of a git command, or $null.
function Get-GitLine([string]$Dir, [string[]]$GitArgs) {
    $l = @(Get-GitLines $Dir $GitArgs | Where-Object { $_.Trim() } | Select-Object -First 1)
    if ($l.Count -eq 0) { return $null }
    return $l[0].Trim()
}
function ConvertTo-NormPath([string]$P) { return [IO.Path]::GetFullPath($P.Trim()).TrimEnd('\') }
function Test-MainCheckout([string]$Parent, [string]$CommonDir) {
    $pgd = Get-GitLine $Parent @("rev-parse", "--path-format=absolute", "--git-dir")
    if (-not $pgd -or ((ConvertTo-NormPath $pgd) -ine $CommonDir)) { return $false }
    if ((Get-GitLine $Parent @("rev-parse", "--is-inside-work-tree")) -ne "true") { return $false }
    $ptop = Get-GitLine $Parent @("rev-parse", "--show-toplevel")
    if (-not $ptop -or ((ConvertTo-NormPath $ptop) -ine $Parent)) { return $false }
    $other = @(Get-ChildItem -LiteralPath $Parent -Force | Where-Object { $_.Name -ne ".git" -and $_.Name -ne ".journal" })
    if ($other.Count -gt 0) { return $true }
    return (-not (Get-GitLine $Parent @("ls-tree", "HEAD")))
}
# Root of the MAIN checkout. Falls back to the given folder.
function Get-MainRoot([string]$Dir) {
    try {
        $gd = Get-GitLine $Dir @("rev-parse", "--path-format=absolute", "--git-dir")
        $cd = Get-GitLine $Dir @("rev-parse", "--path-format=absolute", "--git-common-dir")
        if (-not $gd -or -not $cd) { return $Dir }
        $gd = ConvertTo-NormPath $gd
        $cd = ConvertTo-NormPath $cd
        if ($gd -ieq $cd) {
            $top = Get-GitLine $Dir @("rev-parse", "--show-toplevel")
            if (-not $top) { return $Dir }
            return ConvertTo-NormPath $top
        }
        if ((Split-Path -Leaf $cd) -ne ".git") { return $Dir }
        $parent = Split-Path -Parent $cd
        if (Test-MainCheckout $parent $cd) { return $parent }
        return $Dir
    } catch { return $Dir }
}
# --- end of shared helpers ---

# Git call with exit code, stdout and stderr kept apart (same care as verify-backup.ps1).
function Invoke-Git([string]$Dir, [string[]]$GitArgs) {
    $ErrorActionPreference = "Continue"
    $out = @(); $err = @()
    try {
        foreach ($item in @(& git -C $Dir -c core.fsmonitor=false -c core.autocrlf=false -c core.safecrlf=false @GitArgs 2>&1)) {
            if ($item -is [System.Management.Automation.ErrorRecord]) {
                $s = ([string]$item).Trim()
                if ($s -and $s -ne "System.Management.Automation.RemoteException") { $err += $s }
            } else { $out += [string]$item }
        }
        return [pscustomobject]@{ Code = $LASTEXITCODE; Out = $out; Err = $err }
    } catch {
        return [pscustomobject]@{ Code = 128; Out = @(); Err = @([string]$_) }
    }
}
function Short([string]$Sha) { if ($Sha -and $Sha.Length -gt 7) { return $Sha.Substring(0, 7) } return $Sha }
function ConvertTo-Safe([string]$Name) {
    $s = ($Name -replace '[^A-Za-z0-9_-]', '-').Trim('-')
    if (-not $s) { $s = "x" }
    return $s
}
# 8 hex chars of a normalized path (same idea as Get-WorktreeTag in journal-skeleton.ps1).
function Get-PathTag([string]$Path) {
    $sha = [Security.Cryptography.SHA1]::Create()
    $bytes = $sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($Path.ToLowerInvariant()))
    return (-join ($bytes[0..3] | ForEach-Object { $_.ToString("x2") }))
}
# Path shown in STATE.md: relative to the main checkout when possible, never the machine path.
function Get-ShownPath([string]$Path) {
    if ($Path -ieq $script:MainRoot) { return "." }
    if ($Path.StartsWith($script:MainRoot + "\", [StringComparison]::OrdinalIgnoreCase)) {
        return ($Path.Substring($script:MainRoot.Length + 1) -replace '\\', '/')
    }
    return "(fora do repositorio) " + (Split-Path -Leaf $Path)
}

# Reason a file or folder never enters the pack, or $null. Decided from the name, the relative path and the size
# only: an excluded file is never opened.
function Get-ExcludeReason([string]$Rel, [string]$Name, [bool]$IsDir, [long]$Length) {
    if ($Rel -match '(?i)(^|/)\.claude/worktrees(/|$)') { return ".claude/worktrees (copias completas do repositorio)" }
    if ($Rel -match '(?i)(^|/)sandbox-lab/shared/secrets(/|$)') { return "segredos do laboratorio" }
    if ($IsDir) {
        if ($Name -ieq ".git") { return "repositorio git aninhado" }
        if ($Name -like ".env*") { return "pasta de ambiente (.env*)" }
        if ($Name -like "*credential*") { return "pasta de credenciais" }
        if ($Name -ieq "secrets") { return "pasta de segredos" }
        return $null
    }
    if ($Name -like ".env*") { return "arquivo de ambiente (.env*)" }
    foreach ($p in @("*.pem", "*.key", "*.pfx", "*.p12", "id_rsa*")) { if ($Name -like $p) { return "chave ou certificado ($p)" } }
    if ($Name -ieq ".credentials.json" -or $Name -like "*credential*") { return "credencial" }
    foreach ($p in @("candidate_base.yaml", "search_variants.yaml", "strategy.md", "disparo.json", "notif.json", "modo_autonomo.json")) {
        if ($Name -ieq $p) { return "dado pessoal do perfil" }
    }
    if ($Name -like "*.db" -or $Name -like "*.sqlite*") { return "banco de dados" }
    if ($Length -gt $script:MaxBytes) { return "maior que 5 MB" }
    return $null
}

$Plan = New-Object System.Collections.ArrayList
$Excluded = New-Object System.Collections.ArrayList
$Notes = New-Object System.Collections.ArrayList
function Add-Excluded([string]$Rel, [string]$Reason) { [void]$script:Excluded.Add([pscustomobject]@{ Rel = $Rel; Reason = $Reason }) }

# Plans one file. $Rel is the path used for the exclusion rules and shown in reports.
function Add-PlanFile([string]$Group, [string]$Source, [string]$Dest, [string]$Rel) {
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { return }
    $item = Get-Item -LiteralPath $Source -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { Add-Excluded $Rel "link (nao seguido)"; return }
    $reason = Get-ExcludeReason $Rel $item.Name $false $item.Length
    if ($reason) { Add-Excluded $Rel $reason; return }
    if (-not (Test-Scannable $item.FullName)) { Add-Excluded $Rel "nao varrivel como texto (binario ou UTF-16 sem BOM)"; return }
    $kind = "copy"
    if ($item.Extension -ieq ".json") {
        $kind = "json"
        if ($null -eq (Get-CleanJson $item.FullName)) { Add-Excluded $Rel "JSON invalido (nao copiado)"; return }
    }
    [void]$script:Plan.Add([pscustomobject]@{ Group = $Group; Source = $item.FullName; Dest = $Dest; Rel = $Rel; Kind = $kind })
}
# Plans a folder recursively, never following links and never descending into an excluded folder.
function Add-PlanTree([string]$Group, [string]$SourceDir, [string]$DestPrefix, [string]$RelPrefix) {
    if (-not (Test-Path -LiteralPath $SourceDir -PathType Container)) { return }
    # The root of a source is checked too: a junction/symlink root is never followed.
    if (Test-IsLink $SourceDir) { Add-Excluded "$RelPrefix/" "link (nao seguido)"; return }
    foreach ($child in @(Get-ChildItem -LiteralPath $SourceDir -Force)) {
        $rel = "$RelPrefix/$($child.Name)"
        $dest = "$DestPrefix/$($child.Name)"
        if ($child.PSIsContainer) {
            if ($child.Attributes -band [IO.FileAttributes]::ReparsePoint) { Add-Excluded "$rel/" "link (nao seguido)"; continue }
            $reason = Get-ExcludeReason $rel $child.Name $true 0
            if ($reason) { Add-Excluded "$rel/" $reason; continue }
            Add-PlanTree $Group $child.FullName $dest $rel
        } else {
            Add-PlanFile $Group $child.FullName $dest $rel
        }
    }
}

# Structural cleanup (P17): removes env, headers and secret-looking keys, recursively. Returns the count removed.
function Remove-SecretKeys($Node) {
    $n = 0
    if ($null -eq $Node) { return 0 }
    if ($Node -is [System.Management.Automation.PSCustomObject]) {
        foreach ($name in @($Node.PSObject.Properties | ForEach-Object { $_.Name })) {
            if ($name -ieq "env" -or $name -ieq "headers" -or $name -match $script:SecretKeyRegex) {
                $Node.PSObject.Properties.Remove($name); $n++
            } else {
                $n += Remove-SecretKeys $Node.PSObject.Properties[$name].Value
            }
        }
    } elseif (($Node -is [System.Collections.IEnumerable]) -and -not ($Node -is [string])) {
        foreach ($el in $Node) { $n += Remove-SecretKeys $el }
    }
    return $n
}
# Cleaned JSON text, or $null when the file is not valid JSON.
function Get-CleanJson([string]$Path) {
    try {
        $text = [IO.File]::ReadAllText($Path)
        if (-not $text.Trim()) { return $null }
        $obj = ConvertFrom-Json -InputObject $text
        [void](Remove-SecretKeys $obj)
        return (ConvertTo-Json -InputObject $obj -Depth 100)
    } catch { return $null }
}

# Names and versions of the installed plugins, or $null when the list is not found or unreadable.
function Get-PluginLines([string]$GlobalDir) {
    $f = Join-Path $GlobalDir "plugins\installed_plugins.json"
    if (-not (Test-Path -LiteralPath $f -PathType Leaf)) { return $null }
    try {
        $obj = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText($f))
        $lines = @()
        if ($null -ne $obj.plugins) {
            foreach ($p in $obj.plugins.PSObject.Properties) {
                $versions = @(@($p.Value) | ForEach-Object { if ($null -ne $_.version) { [string]$_.version } else { "?" } })
                if ($versions.Count -eq 0) { $versions = @("?") }
                foreach ($v in ($versions | Select-Object -Unique)) { $lines += "$($p.Name) $v" }
            }
        }
        return ,@($lines | Sort-Object)
    } catch { return $null }
}

# The ONLY ref writer. Refuses any ref other than refs/heads/recovery and always passes the expected old value,
# so a concurrent change of the ref makes the update fail instead of being overwritten.
function Update-RecoveryRef([string]$Ref, [string]$New, [string]$ExpectedOld) {
    if ($Ref -cne "refs/heads/recovery") {
        return [pscustomobject]@{ Ok = $false; Msg = "recusado: este script so escreve refs/heads/recovery (pedido: $Ref)" }
    }
    if (-not $ExpectedOld) { return [pscustomobject]@{ Ok = $false; Msg = "recusado: falta o valor antigo esperado" } }
    $r = Invoke-Git $script:RepoDir @("update-ref", "-m", "recovery-pack", $Ref, $New, $ExpectedOld)
    if ($r.Code -ne 0) {
        return [pscustomobject]@{ Ok = $false; Msg = "a ref mudou durante a execucao ou nao pode ser gravada: $(Format-GitError $r)" }
    }
    return [pscustomobject]@{ Ok = $true; Msg = "" }
}

# ---------------------------------------------------------------- environment
if (-not (Get-Command git -ErrorAction SilentlyContinue)) { Say-Err "ERRO: git nao encontrado no PATH"; exit 2 }
if (-not (Test-Path -LiteralPath $RepoDir -PathType Container)) { Say-Err "ERRO: pasta nao existe: $RepoDir"; exit 2 }
$RepoDir = ConvertTo-NormPath $RepoDir
$gitDir = Get-GitLine $RepoDir @("rev-parse", "--path-format=absolute", "--git-dir")
$top = Get-GitLine $RepoDir @("rev-parse", "--show-toplevel")
if (-not $gitDir -or -not $top) { Say-Err "ERRO: nao e um repositorio git (com work tree): $RepoDir"; exit 2 }
$gitDir = ConvertTo-NormPath $gitDir
$MainRoot = Get-MainRoot (ConvertTo-NormPath $top)

if (-not $GlobalClaudeDir) {
    if ($env:USERPROFILE) { $GlobalClaudeDir = Join-Path $env:USERPROFILE ".claude" }
}
if (-not $MemoryDir -and $GlobalClaudeDir) {
    $encoded = $MainRoot -replace '[^A-Za-z0-9]', '-'
    $MemoryDir = Join-Path $GlobalClaudeDir ("projects\" + $encoded + "\memory")
}

# ---------------------------------------------------------------- plan: project config
if (Test-IsLink (Join-Path $MainRoot ".claude")) {
    Add-Excluded ".claude/" "link (nao seguido)"
} else {
    # Every level between the repository root (not included, nor anything above it) and the folder is checked; a
    # link at the folder itself is reported by Add-PlanTree as before.
    foreach ($sub in @("skills", "commands")) {
        $subDir = Join-Path $MainRoot ".claude\$sub"
        $lk = Get-LinkBetween $MainRoot $subDir
        if ($lk -and $lk -ine ".claude/$sub") { Add-Excluded ".claude/$sub/" "link num nivel entre a raiz do repositorio e a pasta ($lk) (nao seguido)"; continue }
        Add-PlanTree "projeto" $subDir "project/.claude/$sub" ".claude/$sub"
    }
    Add-PlanFile "projeto" (Join-Path $MainRoot ".claude\settings.json") "project/.claude/settings.json" ".claude/settings.json"
    Add-PlanFile "projeto" (Join-Path $MainRoot ".claude\settings.local.json") "project/.claude/settings.local.json" ".claude/settings.local.json"
    if (Test-Path -LiteralPath (Join-Path $MainRoot ".claude\worktrees")) { Add-Excluded ".claude/worktrees/" ".claude/worktrees (copias completas do repositorio)" }
}
Add-PlanFile "projeto" (Join-Path $MainRoot ".mcp.json") "project/.mcp.json" ".mcp.json"

# ---------------------------------------------------------------- plan: global config and memory
$pluginLines = $null
if ($GlobalClaudeDir -and (Test-IsLink $GlobalClaudeDir)) {
    Add-Excluded "global/" "link (nao seguido)"
    [void]$Notes.Add("pasta global do Claude e um link: nao seguida")
} elseif ($GlobalClaudeDir -and (Test-Path -LiteralPath $GlobalClaudeDir -PathType Container)) {
    Add-PlanFile "global" (Join-Path $GlobalClaudeDir "CLAUDE.md") "global/CLAUDE.md" "global/CLAUDE.md"
    Add-PlanFile "global" (Join-Path $GlobalClaudeDir "settings.json") "global/settings.json" "global/settings.json"
    $pluginLines = Get-PluginLines $GlobalClaudeDir
} else {
    [void]$Notes.Add("pasta global do Claude nao encontrada")
}
if ($null -eq $pluginLines) { [void]$Notes.Add("lista de plugins: nao encontrado") }
if ($MemoryDir -and (Test-Path -LiteralPath $MemoryDir -PathType Container)) {
    # A junction/symlink at ANY level from -GlobalClaudeDir (included) down to the memory folder keeps the memory
    # out. Levels above -GlobalClaudeDir are not checked. The memory folder itself is checked by Add-PlanTree.
    $memLink = $null
    $memRel = $null
    if ($GlobalClaudeDir) {
        $gN = [IO.Path]::GetFullPath($GlobalClaudeDir).TrimEnd('\')
        $mN = [IO.Path]::GetFullPath($MemoryDir).TrimEnd('\')
        if ($mN.StartsWith($gN + "\", [StringComparison]::OrdinalIgnoreCase)) {
            $memRel = $mN.Substring($gN.Length + 1) -replace '\\', '/'
            $memLink = Get-LinkBetween $gN $mN -IncludeRoot
        }
    }
    if ($memLink -and $memLink -ine $memRel) {
        # Shown without the encoded project folder name (it is derived from a machine path).
        $segs = @($memLink -split '/')
        $shownLink = if ($memLink -eq ".") { "a propria pasta global" } elseif ($segs.Count -gt 1) { "global/$($segs[0])/..." } else { "global/$($segs[0])" }
        Add-Excluded "memory/" "link num nivel entre a pasta global e a memoria ($shownLink) (nao seguido)"
        [void]$Notes.Add("memoria do projeto: nao entrou, ha um link entre a pasta global e a memoria ($shownLink)")
    } else {
        Add-PlanTree "global" $MemoryDir "global/memory" "memory"
    }
} else {
    [void]$Notes.Add("memoria do projeto: nao encontrado")
}

# ---------------------------------------------------------------- plan: work state
$worktrees = @()
$cur = $null
foreach ($line in (Get-GitLines $RepoDir @("worktree", "list", "--porcelain"))) {
    if ($line.StartsWith("worktree ")) {
        if ($cur) { $worktrees += $cur }
        $cur = [pscustomobject]@{ Path = (ConvertTo-NormPath $line.Substring(9)); Head = $null; Branch = $null; Bare = $false }
    } elseif ($cur -and $line.StartsWith("HEAD ")) { $cur.Head = $line.Substring(5).Trim() }
    elseif ($cur -and $line.StartsWith("branch ")) { $cur.Branch = $line.Substring(7).Trim() -replace '^refs/heads/', '' }
    elseif ($cur -and $line -eq "bare") { $cur.Bare = $true }
}
if ($cur) { $worktrees += $cur }
$worktrees = @($worktrees | Where-Object { -not $_.Bare -and (Test-Path -LiteralPath $_.Path -PathType Container) })

# Diaries: all of the main checkout, plus the ones that only exist in some worktree .journal\.
$journalNames = @{}
$mainJournal = Join-Path $MainRoot ".journal"
if (Test-IsLink $mainJournal) {
    Add-Excluded ".journal/" "link (nao seguido)"
} elseif (Test-Path -LiteralPath $mainJournal -PathType Container) {
    foreach ($f in @(Get-ChildItem -LiteralPath $mainJournal -Filter "*.md" -File -Force)) {
        $journalNames[$f.Name.ToLowerInvariant()] = $true
        Add-PlanFile "estado" $f.FullName "journal/$($f.Name)" "journal/$($f.Name)"
    }
}
foreach ($wt in $worktrees) {
    if ($wt.Path -ieq $MainRoot) { continue }
    $wj = Join-Path $wt.Path ".journal"
    if (-not (Test-Path -LiteralPath $wj -PathType Container)) { continue }
    if (Test-IsLink $wj) { Add-Excluded ((Get-ShownPath $wt.Path) + "/.journal/") "link (nao seguido)"; continue }
    foreach ($f in @(Get-ChildItem -LiteralPath $wj -Filter "*.md" -File -Force)) {
        $key = $f.Name.ToLowerInvariant()
        if ($journalNames.ContainsKey($key)) { continue }
        $journalNames[$key] = $true
        Add-PlanFile "estado" $f.FullName "journal/$($f.Name)" "journal/$($f.Name)"
    }
}

# Pathspecs that keep forbidden files out of the patches too.
$patchExcludes = @(".")
foreach ($g in @("**/.env*", "**/*.pem", "**/*.key", "**/*.pfx", "**/*.p12", "**/id_rsa*", "**/.credentials.json",
        "**/*credential*", "sandbox-lab/shared/secrets/**", "**/candidate_base.yaml", "**/search_variants.yaml",
        "**/strategy.md", "**/disparo.json", "**/notif.json", "**/modo_autonomo.json", "**/*.db", "**/*.sqlite*",
        ".claude/worktrees/**", "**/.env*/**", "**/*credential*/**", "**/secrets/**")) {
    $patchExcludes += ":(exclude,glob,icase)$g"
}

# Per-worktree state (read only).
$devExists = [bool](Get-GitLine $RepoDir @("rev-parse", "-q", "--verify", "refs/heads/dev^{commit}"))
$wtInfo = @()
foreach ($wt in $worktrees) {
    $hasHead = [bool](Get-GitLine $wt.Path @("rev-parse", "-q", "--verify", "HEAD^{commit}"))
    $changed = @(Get-GitLines $wt.Path @("status", "--porcelain", "--untracked-files=no") | Where-Object { $_.Trim() })
    $untracked = @(Get-GitLines $wt.Path @("ls-files", "--others", "--exclude-standard") | Where-Object { $_.Trim() })
    $label = $wt.Branch
    if (-not $label) { $label = "HEAD-" + (Get-PathTag $wt.Path) }
    $patch = $null
    if ($hasHead -and $changed.Count -gt 0) { $patch = "patches/" + (ConvertTo-Safe $label) + ".patch" }
    $ab = "dev nao existe localmente"
    if ($devExists -and $hasHead) {
        $c = Get-GitLine $wt.Path @("rev-list", "--left-right", "--count", "refs/heads/dev...HEAD")
        if ($c) { $parts = $c -split '\s+'; $ab = "$($parts[1]) a frente, $($parts[0]) atras" }
    }
    $inOrigin = "-"; $inBackup = "-"
    if ($wt.Branch) {
        $inOrigin = if (Get-GitLine $RepoDir @("rev-parse", "-q", "--verify", "refs/remotes/origin/$($wt.Branch)")) { "sim" } else { "nao" }
        $inBackup = if (Get-GitLine $RepoDir @("rev-parse", "-q", "--verify", "refs/remotes/backup/$($wt.Branch)")) { "sim" } else { "nao" }
    }
    $wtInfo += [pscustomobject]@{ Wt = $wt; HasHead = $hasHead; Changed = $changed; Untracked = $untracked; Patch = $patch; AheadBehind = $ab; InOrigin = $inOrigin; InBackup = $inBackup }
}

# ---------------------------------------------------------------- -WhatIf: list only, write nothing
$groups = @("projeto", "global", "estado")
if ($WhatIf) {
    Say "Pacote de recuperacao, SIMULACAO (-WhatIf): nada sera escrito."
    foreach ($g in $groups) {
        $items = @($Plan | Where-Object { $_.Group -eq $g })
        Say "[$g] $($items.Count) arquivo(s)"
        foreach ($i in $items) { Say ("  $($i.Dest)") }
    }
    if ($null -ne $pluginLines) { Say "  global/plugins.txt (gerado)" }
    Say "  STATE.md (gerado)"
    foreach ($w in $wtInfo) { if ($w.Patch) { Say ("  $($w.Patch) (gerado)") } }
    Say "Fora do pacote:"
    foreach ($e in $Excluded) { Say ("  $($e.Rel): $($e.Reason)") }
    foreach ($n in $Notes) { Say ("Nota: $n") }
    Say "A varredura de segredos roda so na execucao real."
    exit 0
}

# ---------------------------------------------------------------- materialize in a new temp folder
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("recovery-pack-" + [guid]::NewGuid().ToString("N"))
$tree = Join-Path $tmp "tree"
$work = Join-Path $tmp "work"
New-Item -ItemType Directory -Path $tree | Out-Null
New-Item -ItemType Directory -Path $work | Out-Null

function Get-DestPath([string]$Dest) {
    $p = Join-Path $script:tree ($Dest -replace '/', '\')
    $parent = Split-Path -Parent $p
    if (-not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Path $parent | Out-Null }
    return $p
}

$written = @{ "projeto" = 0; "global" = 0; "estado" = 0 }
foreach ($i in $Plan) {
    $dst = Get-DestPath $i.Dest
    if ($i.Kind -eq "json") {
        $clean = Get-CleanJson $i.Source
        if ($null -eq $clean) { Add-Excluded $i.Rel "JSON invalido (nao copiado)"; continue }
        [IO.File]::WriteAllText($dst, $clean, $Utf8)
    } else {
        [IO.File]::Copy($i.Source, $dst, $false)
    }
    $written[$i.Group]++
}
if ($null -ne $pluginLines) {
    [IO.File]::WriteAllText((Get-DestPath "global/plugins.txt"), (($pluginLines -join "`n") + "`n"), $Utf8)
    $written["global"]++
}

# Patches: written outside the tree first; copied in only when within the size limit.
foreach ($w in $wtInfo) {
    if (-not $w.Patch) { continue }
    $tmpPatch = Join-Path $work ([IO.Path]::GetFileName($w.Patch))
    $r = Invoke-Git $w.Wt.Path (@("diff", "--no-color", "--no-ext-diff", "--no-textconv", "HEAD", "--output=$tmpPatch", "--") + $patchExcludes)
    if ($r.Code -ne 0 -or -not (Test-Path -LiteralPath $tmpPatch)) { [void]$Notes.Add((Hide-Secrets "patch de $($w.Patch) falhou: $(Format-GitError $r)")); $w.Patch = $null; continue }
    $len = (Get-Item -LiteralPath $tmpPatch).Length
    if ($len -eq 0) { $w.Patch = $null; continue }
    if ($len -gt $MaxBytes) { Add-Excluded $w.Patch "maior que 5 MB"; $w.Patch = $null; continue }
    [IO.File]::Copy($tmpPatch, (Get-DestPath $w.Patch), $false)
    $written["estado"]++
}

# STATE.md
$sb = New-Object System.Text.StringBuilder
function Add-Line([string]$s) { [void]$script:sb.Append($s + "`n") }
Add-Line "# Estado de trabalho (pacote de recuperacao)"
Add-Line ""
Add-Line ("- Gerado em: " + (Get-Date -Format "yyyy-MM-dd HH:mm"))
Add-Line ("- Repositorio: " + (Split-Path -Leaf $MainRoot))
Add-Line ""
Add-Line "## Worktrees"
foreach ($w in $wtInfo) {
    Add-Line ""
    Add-Line ("### " + (Get-ShownPath $w.Wt.Path))
    $b = if ($w.Wt.Branch) { $w.Wt.Branch } else { "(HEAD destacado)" }
    $h = if ($w.Wt.Head) { Short $w.Wt.Head } else { "sem commit" }
    Add-Line "- Branch: $b @ $h"
    Add-Line "- Em relacao a dev: $($w.AheadBehind)"
    Add-Line "- No origin (refs remotas ja conhecidas localmente): $($w.InOrigin)"
    Add-Line "- No backup (refs remotas ja conhecidas localmente): $($w.InBackup)"
    $p = if ($w.Patch) { $w.Patch } else { "-" }
    Add-Line "- Patch do trabalho nao commitado: $p"
    Add-Line "- Alterados (rastreados): $($w.Changed.Count)"
    foreach ($c in ($w.Changed | Select-Object -First 200)) { Add-Line "  - $($c.Trim())" }
    Add-Line "- Nao rastreados (so o nome; o conteudo nao entra): $($w.Untracked.Count)"
    foreach ($u in ($w.Untracked | Select-Object -First 200)) { Add-Line "  - $u" }
    if ($w.Untracked.Count -gt 200) { Add-Line "  - ... e mais $($w.Untracked.Count - 200)" }
}
Add-Line ""
Add-Line "## Proximos passos"
$journals = @($Plan | Where-Object { $_.Dest -like "journal/*" } | Sort-Object { $_.Dest } -Descending)
if ($journals.Count -gt 0) { Add-Line "- Ver o diario mais recente: $($journals[0].Dest)" } else { Add-Line "- Ver o diario mais recente (nenhum diario encontrado)" }
Add-Line ""
Add-Line "## Notas"
if ($Notes.Count -eq 0) { Add-Line "- nenhuma" }
foreach ($n in $Notes) { Add-Line "- $n" }
Add-Line ""
Add-Line "## Fora do pacote"
if ($Excluded.Count -eq 0) { Add-Line "- nada" }
foreach ($e in $Excluded) { Add-Line "- $($e.Rel): $($e.Reason)" }
$stateText = $sb.ToString()
[IO.File]::WriteAllText((Get-DestPath "STATE.md"), $stateText, $Utf8)
$written["estado"]++

Say "Pacote de recuperacao ($TargetRef)"
foreach ($g in $groups) { Say ("[{0}] {1} arquivo(s)" -f $g, $written[$g]) }
Say "Fora do pacote:"
if ($Excluded.Count -eq 0) { Say "  nada" }
foreach ($e in $Excluded) { Say ("  $($e.Rel): $($e.Reason)") }
foreach ($n in $Notes) { Say ("Nota: $n") }
Say "Pasta temporaria (nao e apagada por este script): $tmp"

# ---------------------------------------------------------------- second layer: secret scan of everything
# Content of every file (STATE.md and patches included) and the relative path name of every file.
$findings = @()
foreach ($f in @(Get-ChildItem -LiteralPath $tree -Recurse -File -Force)) {
    $text = [IO.File]::ReadAllText($f.FullName)
    $rel = $f.FullName.Substring($tree.Length + 1) -replace '\\', '/'
    $shown = Hide-Secrets $rel
    foreach ($p in $SecretPatterns) {
        if ([regex]::IsMatch($text, $p[1])) { $findings += "$shown :: $($p[0])" }
        if ([regex]::IsMatch($rel, $p[1])) { $findings += "$shown (no NOME do arquivo) :: $($p[0])" }
    }
}
if ($findings.Count -gt 0) {
    Say "VARREDURA ACHOU POSSIVEL SEGREDO: nada foi gravado e a ref nao mudou."
    foreach ($x in $findings) { Say "  $x" }
    Say "Remova o segredo da origem (e rotacione a credencial) antes de gerar o pacote de novo."
    Say "ATENCAO: a pasta temporaria $tmp contem a copia com o possivel segredo. Apague-a a mao: este script nao apaga nada, por regra."
    exit 1
}

# ---------------------------------------------------------------- commit by plumbing
$index = Join-Path $tmp "index"
[Environment]::SetEnvironmentVariable("GIT_INDEX_FILE", $index, "Process")
try {
    $r = Invoke-Git $tree @("--git-dir=$gitDir", "--work-tree=$tree", "-c", "core.untrackedCache=false", "add", "-A", "-f", "--", ".")
    if ($r.Code -ne 0) { Say-Err "ERRO: git add falhou: $(Format-GitError $r)"; exit 1 }
    $r = Invoke-Git $tree @("--git-dir=$gitDir", "--work-tree=$tree", "write-tree")
    if ($r.Code -ne 0 -or $r.Out.Count -eq 0) { Say-Err "ERRO: git write-tree falhou: $(Format-GitError $r)"; exit 1 }
    $newTree = $r.Out[0].Trim()
} finally {
    [Environment]::SetEnvironmentVariable("GIT_INDEX_FILE", $null, "Process")
}

$old = $null
if (-not $TestPretendNoRecovery) { $old = Get-GitLine $RepoDir @("rev-parse", "-q", "--verify", "$TargetRef^{commit}") }

if ($old) {
    $oldTree = Get-GitLine $RepoDir @("rev-parse", "$old^{tree}")
    $same = ($oldTree -eq $newTree)
    if (-not $same) {
        # Only the generation date of STATE.md changed? Then nothing changed.
        $diff = @(Get-GitLines $RepoDir @("diff-tree", "-r", "--name-only", $oldTree, $newTree) | Where-Object { $_.Trim() })
        if ($diff.Count -eq 1 -and $diff[0] -ceq "STATE.md") {
            $oldState = (Get-GitLines $RepoDir @("cat-file", "blob", "${old}:STATE.md")) -join "`n"
            $strip = { param($t) (($t -replace "`r", "") -replace '(?m)^- Gerado em: .*$', "").Trim() }
            $same = ((& $strip $oldState) -ceq (& $strip $stateText))
        }
    }
    if ($same) {
        Say "Resultado: nada mudou ($(Short $old))."
        exit 0
    }
}

$idArgs = @()
if (-not (Get-GitLine $RepoDir @("config", "user.name"))) { $idArgs += @("-c", "user.name=recovery-pack") }
if (-not (Get-GitLine $RepoDir @("config", "user.email"))) { $idArgs += @("-c", "user.email=recovery-pack@localhost") }
$msg = "recovery: pack " + (Get-Date -Format "yyyy-MM-dd HH:mm")
$ctArgs = $idArgs + @("commit-tree", $newTree, "-m", $msg)
if ($old) { $ctArgs += @("-p", $old) }
$r = Invoke-Git $RepoDir $ctArgs
if ($r.Code -ne 0 -or $r.Out.Count -eq 0) { Say-Err "ERRO: git commit-tree falhou: $(Format-GitError $r)"; exit 1 }
$newCommit = $r.Out[0].Trim()

$expected = $old
if (-not $expected) { $expected = "0" * $newCommit.Length }
$u = Update-RecoveryRef $TargetRef $newCommit $expected
if (-not $u.Ok) {
    Say "ERRO: $($u.Msg). Nenhuma ref foi alterada."
    exit 1
}
$parentText = if ($old) { "pai $(Short $old)" } else { "sem pai: branch orfa" }
Say "Resultado: commit novo $(Short $newCommit) em $TargetRef ($parentText). Push so ao backup: git push backup recovery"
exit 0
