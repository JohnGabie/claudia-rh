# Secret scan of a commit range (agents/versao.md "Varredura de segredos": before the first push to the backup and
# before each PR). READ ONLY: never pushes, fetches, writes a ref, the index or any file of the repository.
# Usage: powershell -NoProfile -File scripts/dev/scan-secrets.ps1 -Range <rev-range> [-RepoDir <folder>]
#        e.g. -Range origin/dev..sync/dev-2026-10-20
#
# What is scanned, for every commit of `git rev-list <range>` (merges included):
#   - the ADDED lines of `git log -p --no-color --no-ext-diff --no-textconv --no-renames <range>`; a merge is diffed
#     against its first parent (--diff-merges=first-parent), so a change made in the merge itself is scanned too.
#     Removed lines are not scanned (they leave nothing new in the history being sent). With --no-renames a renamed
#     file shows up as added in full, so its content and its new name are scanned (conservative: a secret that
#     was already in the old path is reported again).
#   - a file git reports as binary ("Binary files ... differ"): the blob of the new version is read raw and scanned as
#     text (a UTF-16 BOM decodes as UTF-16; otherwise byte by byte, and again with the NUL bytes dropped, which
#     catches ASCII stored as UTF-16 without BOM). A blob over 20 MB is not scanned and is listed as
#     "binario nao varrido". Safe side: ANY file not scanned makes the exit 1 with "Nada encontrado no que foi
#     varrido; N arquivo(s) NAO varrido(s): conferir a mao antes de enviar" (never a plain "Nada encontrado.").
#   - the text of -Range and the full names of the refs it resolves to ("nome no intervalo"): a branch or tag
#     name that holds a secret would be published by the push itself.
#   - the commit message.
#   - the NAME (full path) of every file added or changed (a deleted file is not a new name).
#   - forbidden file names, without regex (same "always out" list as recovery-pack.ps1, case-insensitive, on the
#     file name): .env*, *.pem, *.key, *.pfx, *.p12, id_rsa*, .credentials.json, *credential*, candidate_base.yaml,
#     search_variants.yaml, strategy.md, disparo.json, notif.json, modo_autonomo.json, *.db, *.sqlite*.
#   - forbidden FOLDERS (same folder rules as recovery-pack.ps1, case-insensitive, at any level): a file added or
#     changed inside a folder named .env*, secrets or *credential*, or under sandbox-lab/shared/secrets/.
#   Deleting a file (forbidden name or inside a forbidden folder) is not a finding: it adds nothing new.
# WARNINGS (do not block, exit stays 0 when there are only warnings), in the ADDED lines only:
#   - "caminho pessoal": absolute user-folder paths, any slash: X:\Users\<name>, X:/Users/<name>, /home/<name>,
#     /Users/<name>, and "X:Users<name>" (slashes lost). Placeholders such as C:\Users\<nome> are not matched.
#   - "e-mail": e-mail addresses, except noreply@... and ...@users.noreply.github.com.
#   An added line that already holds a finding never gives a warning (the warning would echo part of the value).
#   Printed in the section "Avisos (nao bloqueiam)" with commit, file and kind; the user name and the local part of
#   the e-mail are masked ("C:\Users\[oculto]", "[oculto]@domain"). Reason: the public dev already has known
#   personal paths (CLA-54); blocking would stop the sync PR, a warning lets the user see them.
# Patterns: scripts/dev/lib/secret-patterns.ps1 (the project's single reference list, dot-sourced).
# Output: one line per finding with the short commit, the place (file name masked by Hide-Secrets, or "mensagem do
#   commit") and the pattern name. NEVER the value. Every message printed (own and git's) goes through Hide-Secrets.
# Exit: 0 nothing found and everything scanned (warnings allowed); 1 something found or something not scanned; 2 environment error (git missing, folder missing, not a repository,
#   invalid or empty -Range, or any unhandled exception: printed masked by the script-level trap).
# Git is run through System.Diagnostics.Process with stdout read as raw bytes (decoded as UTF-8) and stderr read
# apart: in Windows PowerShell 5.1 native stderr under "Stop" becomes a terminating error (the same care as
# Get-GitLines in pre-compact.ps1), and the console code page would garble the text.
param(
    [string]$Range = "",
    [string]$RepoDir = (Get-Location).Path
)
$ErrorActionPreference = "Stop"

# Any unhandled exception would be printed raw (it may carry a path with a secret): print it masked, exit 2.
trap {
    $m = [string]$_.Exception.Message
    if (Get-Command Hide-Secrets -ErrorAction SilentlyContinue) { $m = Hide-Secrets $m } else { $m = "(mensagem omitida: lista de padroes nao carregada)" }
    [Console]::Error.WriteLine("ERRO: " + $m)
    exit 2
}

. "$PSScriptRoot\lib\secret-patterns.ps1"

# Never let git or the credential manager open a prompt (this script makes no network call, but be safe).
$env:GIT_TERMINAL_PROMPT = "0"
$env:GCM_INTERACTIVE = "never"

$MaxBlobBytes = 20MB
$Utf8 = New-Object System.Text.UTF8Encoding($false)
$Latin1 = [Text.Encoding]::GetEncoding(28591)
$ForbiddenNames = @(".env*", "*.pem", "*.key", "*.pfx", "*.p12", "id_rsa*", ".credentials.json", "*credential*",
    "candidate_base.yaml", "search_variants.yaml", "strategy.md", "disparo.json", "notif.json", "modo_autonomo.json",
    "*.db", "*.sqlite*")
$ForbiddenDirs = @(".env*", "secrets", "*credential*")
# Warning patterns: group "n" is the part that is masked (user name or e-mail local part).
$PersonalPathPatterns = @(
    '(?i)(?<![A-Za-z0-9])[A-Za-z]:[\\/]+Users[\\/]+(?<n>[A-Za-z0-9._-]+)',
    '(?i)(?<![A-Za-z0-9.:/\\])[\\/]+(?:home|Users)[\\/]+(?<n>[A-Za-z0-9._-]+)',
    '(?i)(?<![A-Za-z0-9])[A-Za-z]:Users(?<n>[A-Za-z0-9._-]+)'
)
$EmailPattern = '(?<![A-Za-z0-9._%+-])(?<n>[A-Za-z0-9._%+-]+)@(?<d>[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+)'

function Say([string]$Text) { Write-Output (Hide-Secrets $Text) }
function Say-Err([string]$Text) { [Console]::Error.WriteLine((Hide-Secrets $Text)) }

# One command-line argument quoted for CreateProcess (Windows PowerShell 5.1 has no ProcessStartInfo.ArgumentList).
function ConvertTo-Arg([string]$A) {
    if ($A.Length -gt 0 -and $A -notmatch '[\s"]') { return $A }
    $sb = New-Object System.Text.StringBuilder
    [void]$sb.Append('"')
    $bs = 0
    foreach ($ch in $A.ToCharArray()) {
        if ($ch -eq [char]'\') { $bs++; continue }
        if ($ch -eq [char]'"') { [void]$sb.Append([char]'\', 2 * $bs + 1); [void]$sb.Append([char]'"'); $bs = 0; continue }
        if ($bs) { [void]$sb.Append([char]'\', $bs); $bs = 0 }
        [void]$sb.Append($ch)
    }
    if ($bs) { [void]$sb.Append([char]'\', 2 * $bs) }
    [void]$sb.Append('"')
    return $sb.ToString()
}
# Runs git in the repository; returns exit code, stdout as raw bytes, stderr as text. Never throws on a git error.
function Invoke-GitRaw([string[]]$GitArgs) {
    $all = @("-C", $script:RepoDir, "--no-pager", "-c", "core.quotePath=false", "-c", "color.ui=never",
        "-c", "core.fsmonitor=false", "-c", "log.showSignature=false") + $GitArgs
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $script:GitExe
    $psi.Arguments = (($all | ForEach-Object { ConvertTo-Arg $_ }) -join ' ')
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $p = [System.Diagnostics.Process]::Start($psi)
    $p.StandardInput.Close()
    $errTask = $p.StandardError.ReadToEndAsync()
    $ms = New-Object System.IO.MemoryStream
    $p.StandardOutput.BaseStream.CopyTo($ms)
    $p.WaitForExit()
    $err = $errTask.Result
    return [pscustomobject]@{ Code = $p.ExitCode; Bytes = $ms.ToArray(); Err = ([string]$err).Trim() }
}
function Get-GitText([string[]]$GitArgs) {
    $r = Invoke-GitRaw $GitArgs
    return [pscustomobject]@{ Code = $r.Code; Text = $script:Utf8.GetString($r.Bytes); Err = $r.Err }
}
function Short([string]$Sha) { if ($Sha -and $Sha.Length -gt 7) { return $Sha.Substring(0, 7) } return $Sha }
# Undoes git's C-style quoting of a path ("a\tb", "\303\251"); used only for paths with special characters.
function ConvertFrom-GitQuoted([string]$S) {
    if (-not ($S.Length -ge 2 -and $S.StartsWith('"') -and $S.EndsWith('"'))) { return $S }
    $s = $S.Substring(1, $S.Length - 2)
    $bytes = New-Object System.Collections.Generic.List[byte]
    $i = 0
    while ($i -lt $s.Length) {
        $c = $s[$i]
        if ($c -eq '\' -and $i + 1 -lt $s.Length) {
            $n = $s[$i + 1]
            if (($s.Length - $i) -ge 4 -and $s.Substring($i + 1, 3) -match '^[0-7]{3}$') {
                $bytes.Add([Convert]::ToByte($s.Substring($i + 1, 3), 8)); $i += 4; continue
            }
            $map = @{ 'a' = 7; 'b' = 8; 't' = 9; 'n' = 10; 'v' = 11; 'f' = 12; 'r' = 13; '"' = 34; '\' = 92 }
            $k = [string]$n
            if ($map.ContainsKey($k)) { $bytes.Add([byte]$map[$k]); $i += 2; continue }
        }
        foreach ($b in $script:Utf8.GetBytes([string]$c)) { $bytes.Add($b) }
        $i++
    }
    return $script:Utf8.GetString($bytes.ToArray())
}
# Path of a "diff --git a/<p> b/<p>" header. With --no-renames both sides are the same path.
function Get-HeaderPath([string]$Line) {
    $rest = $Line.Substring("diff --git ".Length)
    if ($rest.StartsWith('"')) {
        # Quoted: "a/<q>" "b/<q>" -> two halves of the same length around one space.
        if ((($rest.Length - 1) % 2) -eq 0) { return ((ConvertFrom-GitQuoted $rest.Substring(($rest.Length + 1) / 2)) -replace '^b/', '') }
    }
    # Unquoted: a/<p> b/<p> -> length 2*len(p)+5, the path starts at len(p)+5.
    if ($rest.Length -ge 7 -and (($rest.Length - 5) % 2) -eq 0) { return $rest.Substring(($rest.Length - 5) / 2 + 5) }
    return $rest
}

$Findings = New-Object System.Collections.ArrayList
$Notes = New-Object System.Collections.ArrayList
$Seen = @{}
function Add-Finding([string]$Sha, [string]$Where, [string]$Pattern) {
    # $Sha is a commit id, or a label such as "intervalo" (shown whole).
    $label = if ($Sha -match '^[0-9a-f]{40}$') { Short $Sha } else { $Sha }
    $line = "$label $(Hide-Secrets $Where) :: $Pattern"
    if ($script:Seen.ContainsKey($line)) { return }
    $script:Seen[$line] = $true
    [void]$script:Findings.Add($line)
}
# Every pattern that matches $Text (names only).
function Get-Matches([string]$Text) {
    $hits = @()
    foreach ($p in $script:SecretPatterns) { if ([regex]::IsMatch($Text, $p[1])) { $hits += $p[0] } }
    return $hits
}
function Test-Text([string]$Sha, [string]$Where, [string]$Text) {
    foreach ($h in (Get-Matches $Text)) { Add-Finding $Sha $Where $h }
}
# A touched file (added or changed): its name against the patterns and against the forbidden-name list.
function Test-FileName([string]$Sha, [string]$Path) {
    foreach ($h in (Get-Matches $Path)) { Add-Finding $Sha "$Path (no NOME do arquivo)" $h }
    $segs = @($Path -split '/')
    $leaf = $segs[-1]
    foreach ($f in $script:ForbiddenNames) {
        if ($leaf -like $f) { Add-Finding $Sha "$Path (arquivo proibido pelo nome)" "nome-proibido $f"; break }
    }
    Test-FolderRules $Sha $Path $segs
}
# A file inside a forbidden folder (any level), or under sandbox-lab/shared/secrets/.
function Test-FolderRules([string]$Sha, [string]$Path, [string[]]$Segs) {
    if ($Path -match '(?i)(^|/)sandbox-lab/shared/secrets/') {
        Add-Finding $Sha "$Path (dentro de pasta proibida)" "pasta-proibida sandbox-lab/shared/secrets/"; return
    }
    for ($i = 0; $i -lt $Segs.Count - 1; $i++) {
        foreach ($f in $script:ForbiddenDirs) {
            if ($Segs[$i] -like $f) { Add-Finding $Sha "$Path (dentro de pasta proibida)" "pasta-proibida $f/"; return }
        }
    }
}

# Warnings (personal paths and e-mails) in an added line. Never block; the masked part is replaced by [oculto].
$Warnings = New-Object System.Collections.ArrayList
$WarnSeen = @{}
function Add-Warning([string]$Sha, [string]$Path, [string]$Kind, [string]$Shown) {
    $line = "$(Short $Sha) $(Hide-Secrets $Path) :: $Kind $Shown"
    if ($script:WarnSeen.ContainsKey($line)) { return }
    $script:WarnSeen[$line] = $true
    [void]$script:Warnings.Add($line)
}
function Test-Warnings([string]$Sha, [string]$Path, [string]$Text) {
    foreach ($p in $script:PersonalPathPatterns) {
        foreach ($m in [regex]::Matches($Text, $p)) {
            $g = $m.Groups["n"]
            Add-Warning $Sha $Path "caminho pessoal" ($m.Value.Substring(0, $g.Index - $m.Index) + "[oculto]")
        }
    }
    foreach ($m in [regex]::Matches($Text, $script:EmailPattern)) {
        $local = $m.Groups["n"].Value
        $domain = $m.Groups["d"].Value
        if ($local -ieq "noreply" -or $domain -match '(?i)(^|\.)users\.noreply\.github\.com$') { continue }
        Add-Warning $Sha $Path "e-mail" ("[oculto]@" + $domain)
    }
}
# Binary file: scan the raw blob of the new version as text when possible.
function Test-Blob([string]$Sha, [string]$Path) {
    $size = Get-GitText @("cat-file", "-s", "${Sha}:$Path")
    if ($size.Code -ne 0) { [void]$script:Notes.Add("$(Short $Sha) $(Hide-Secrets $Path): binario nao varrido (blob nao encontrado)"); return }
    if ([long]($size.Text.Trim()) -gt $script:MaxBlobBytes) { [void]$script:Notes.Add("$(Short $Sha) $(Hide-Secrets $Path): binario nao varrido (maior que 20 MB)"); return }
    $r = Invoke-GitRaw @("cat-file", "blob", "${Sha}:$Path")
    if ($r.Code -ne 0) { [void]$script:Notes.Add("$(Short $Sha) $(Hide-Secrets $Path): binario nao varrido (leitura falhou)"); return }
    $b = $r.Bytes
    $views = @()
    if ($b.Length -ge 2 -and $b[0] -eq 0xFF -and $b[1] -eq 0xFE) { $views += [Text.Encoding]::Unicode.GetString($b) }
    elseif ($b.Length -ge 2 -and $b[0] -eq 0xFE -and $b[1] -eq 0xFF) { $views += [Text.Encoding]::BigEndianUnicode.GetString($b) }
    $byteText = $script:Latin1.GetString($b)
    $views += $byteText
    $views += $byteText.Replace([string][char]0, "")
    foreach ($v in $views) { Test-Text $Sha "$Path (binario)" $v }
}

# ---------------------------------------------------------------- environment
$gitCmd = Get-Command git -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $gitCmd) { Say-Err "ERRO: git nao encontrado no PATH"; exit 2 }
$GitExe = $gitCmd.Source
if (-not $Range.Trim()) { Say-Err "ERRO: falta -Range <intervalo> (ex.: origin/dev..minha-branch)"; exit 2 }
if ($Range.StartsWith("-")) { Say-Err "ERRO: intervalo invalido: $Range"; exit 2 }
if (-not (Test-Path -LiteralPath $RepoDir -PathType Container)) { Say-Err "ERRO: pasta nao existe: $RepoDir"; exit 2 }
$RepoDir = [IO.Path]::GetFullPath($RepoDir)
$r = Get-GitText @("rev-parse", "--git-dir")
if ($r.Code -ne 0) { Say-Err "ERRO: nao e um repositorio git: $RepoDir ($($r.Err))"; exit 2 }
$r = Get-GitText @("rev-list", $Range, "--")
if ($r.Code -ne 0) { Say-Err "ERRO: intervalo invalido: $Range ($($r.Err))"; exit 2 }
$commits = @($r.Text -split "`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ })

Say "Varredura de segredos: $Range ($($commits.Count) commit(s))"

# ---------------------------------------------------------------- the range text and the refs it names
# A branch or tag name that holds a secret would be published by the push itself.
Test-Text "intervalo" "nome no intervalo" $Range
$sym = Get-GitText @("rev-parse", "--symbolic-full-name", $Range, "--")
if ($sym.Code -eq 0) {
    foreach ($ref in ($sym.Text -split "`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ -and $_ -ne "--" })) {
        Test-Text "intervalo" "nome no intervalo" $ref
    }
}

# ---------------------------------------------------------------- commit messages
foreach ($c in $commits) {
    $m = Get-GitText @("log", "-1", "--format=%B", $c, "--")
    if ($m.Code -ne 0) { Say-Err "ERRO: git log falhou em $(Short $c): $($m.Err)"; exit 2 }
    Test-Text $c "mensagem do commit" $m.Text
}

# ---------------------------------------------------------------- added lines, file names, binaries
if ($commits.Count -gt 0) {
    $d = Get-GitText @("log", "-p", "--no-color", "--no-ext-diff", "--no-textconv", "--no-renames",
        "--diff-merges=first-parent", "--src-prefix=a/", "--dst-prefix=b/", "--format=%x01%H", $Range, "--")
    if ($d.Code -ne 0) { Say-Err "ERRO: git log -p falhou: $($d.Err)"; exit 2 }
    # Diff lines: "\x01<sha>" (our --format marker; a diff line never starts with \x01), then per file
    # "diff --git a/<p> b/<p>", extended headers ("deleted file mode" comes right after "diff --git"), and hunks.
    # Inside a hunk a line starting with "+" is an added line; "+++ b/<p>" exists only before the first "@@".
    $sha = $null; $path = $null; $inHunk = $false; $deleted = $false; $namePending = $false
    foreach ($raw in ($d.Text -split "`n")) {
        $line = $raw.TrimEnd("`r")
        if ($namePending -and -not $line.StartsWith("deleted file mode")) { Test-FileName $sha $path }
        $namePending = $false
        if ($line.StartsWith([string][char]1)) { $sha = $line.Substring(1).Trim(); $path = $null; $inHunk = $false; continue }
        if ($line.StartsWith("diff --git ")) {
            $path = Get-HeaderPath $line; $inHunk = $false; $deleted = $false; $namePending = $true
            continue
        }
        if (-not $path) { continue }
        if ($inHunk) {
            if ($line.StartsWith("+")) {
                $added = $line.Substring(1)
                $hits = @(Get-Matches $added)
                foreach ($h in $hits) { Add-Finding $sha $path $h }
                # A line that already holds a finding never gives a warning: the warning would echo part of the
                # value (e.g. "password=abc@x.y" would show "[oculto]@x.y").
                if ($hits.Count -eq 0) { Test-Warnings $sha $path $added }
            }
            continue
        }
        if ($line.StartsWith("deleted file mode")) { $deleted = $true }
        elseif ($line.StartsWith("Binary files ") -and -not $deleted) { Test-Blob $sha $path }
        elseif ($line.StartsWith("@@")) { $inHunk = $true }
    }
    if ($namePending) { Test-FileName $sha $path }
}

# ---------------------------------------------------------------- report
foreach ($n in $Notes) { Say "Nota: $n" }
if ($Warnings.Count -gt 0) {
    Say "Avisos (nao bloqueiam): $($Warnings.Count)"
    foreach ($w in $Warnings) { Say "  $w" }
}
if ($Findings.Count -gt 0) {
    Say "ACHOU POSSIVEL SEGREDO ($($Findings.Count)): commit, lugar e padrao (o valor nunca e mostrado)."
    foreach ($f in $Findings) { Say "  $f" }
    Say "Pare, avise o usuario e rotacione a credencial antes de enviar (remover do historico nao basta)."
    if ($Notes.Count -gt 0) { Say "Alem disso, $($Notes.Count) arquivo(s) NAO varrido(s): conferir a mao antes de enviar." }
    exit 1
}
# Safe side: anything that could not be scanned keeps the result from being "nothing found".
if ($Notes.Count -gt 0) {
    Say "Nada encontrado no que foi varrido; $($Notes.Count) arquivo(s) NAO varrido(s): conferir a mao antes de enviar"
    exit 1
}
Say "Nada encontrado."
exit 0
