# Test for scan-secrets.ps1 (secret scan of a commit range).
# Everything is FAKE and lives in fresh temp folders created here ([IO.Path]::GetTempPath() + GUID): small git
# repositories with commits made by this test. Every secret is synthetic, generated here at run time.
# Never reads or writes the user's home, AppData or any real repository. Nothing is deleted: every scenario gets its
# own new repository. Temp folders are left behind.
# Usage: powershell -NoProfile -File scripts/dev/test-scan-secrets.ps1 [-ScriptPath <scan-secrets.ps1 copy>]
#   -> exit 0 when all cases pass, 1 otherwise. -ScriptPath exists so mutants can be tested (the copy needs
#   lib\secret-patterns.ps1 next to it).
param([string]$ScriptPath = (Join-Path $PSScriptRoot "scan-secrets.ps1"))
$ErrorActionPreference = "Stop"
$fail = 0

trap {
    Write-Output "FAILED  unexpected exception (the rest of that step was skipped): $($_.Exception.Message)"
    $script:fail = 1
    continue
}

# git must never walk up out of the temp folders into some real repository.
$env:GIT_CEILING_DIRECTORIES = ([IO.Path]::GetTempPath()).TrimEnd('\')

$Utf8 = New-Object System.Text.UTF8Encoding($false)
$root = Join-Path ([IO.Path]::GetTempPath()) ("scan-test-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $root | Out-Null

function Write-Text([string]$path, [string]$text) {
    $parent = Split-Path -Parent $path
    if (-not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Path $parent | Out-Null }
    [IO.File]::WriteAllText($path, $text, $script:Utf8)
}
function Write-Bytes([string]$path, [byte[]]$bytes) {
    $parent = Split-Path -Parent $path
    if (-not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Path $parent | Out-Null }
    [IO.File]::WriteAllBytes($path, $bytes)
}
function Get-Letters([int]$n) { return (-join (1..$n | ForEach-Object { [char](Get-Random -Minimum 97 -Maximum 123) })) }
# Runs git; returns stdout lines. stderr is dropped (PS 5.1 would turn it into a terminating error).
function Invoke-G {
    $ErrorActionPreference = "Continue"
    $out = @(& git -c user.name=t -c user.email=t@t -c init.defaultBranch=main -c core.autocrlf=false -c commit.gpgsign=false @args 2>$null)
    return ,@($out | ForEach-Object { [string]$_ })
}
function Invoke-Scan {
    $ErrorActionPreference = "Continue"
    $text = (& powershell -NoProfile -ExecutionPolicy Bypass -File $ScriptPath @args 2>&1 | ForEach-Object { [string]$_ }) -join "`n"
    return [pscustomobject]@{ Code = $LASTEXITCODE; Text = $text }
}
function Assert-Case([string]$name, [bool]$ok, [string]$why) {
    if ($ok) { Write-Output "PASS    $name" }
    else { Write-Output "FAILED  $name :: $why"; $script:fail = 1 }
}
function Assert-Exit($res, [string]$name, [int]$code) {
    Assert-Case $name ($res.Code -eq $code) "expected exit $code, got $($res.Code). Output:`n$($res.Text)"
}
# A new fake repository with one clean base commit, tagged `base`. Returns its path.
function New-Repo([string]$Tag) {
    $repo = Join-Path $script:root $Tag
    New-Item -ItemType Directory -Path $repo | Out-Null
    Invoke-G init -q $repo | Out-Null
    Write-Text (Join-Path $repo "readme.txt") "base`n"
    Invoke-G -C $repo add -A | Out-Null
    Invoke-G -C $repo commit -q -m "base" | Out-Null
    Invoke-G -C $repo tag base | Out-Null
    return $repo
}
function Commit-All([string]$repo, [string]$msg) {
    Invoke-G -C $repo add -A | Out-Null
    Invoke-G -C $repo commit -q -m $msg | Out-Null
    return (Invoke-G -C $repo rev-parse --short=7 HEAD)[0]
}

# ------------------------------------------------------------------ 1. secret in an added line
$R1 = New-Repo "added"
$v = "sk-ant-" + (Get-Letters 30)
Write-Text (Join-Path $R1 "src\config.txt") "harmless`nthe key is $v here`n"
$c1 = Commit-All $R1 "add config"
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $R1
Assert-Exit $res "1 secret in an added line: exit 1" 1
Assert-Case "1 secret in an added line: commit, file and pattern named" ($res.Text.Contains("$c1 src/config.txt :: anthropic-key")) "output:`n$($res.Text)"
Assert-Case "1 secret in an added line: value never shown" (-not $res.Text.Contains($v)) "value leaked"

# ------------------------------------------------------------------ 2. secret only in a REMOVED line (added outside the range)
$R2 = New-Repo "removed"
$v = "ghp_" + (Get-Letters 30)
Write-Text (Join-Path $R2 "notes.txt") "token line $v`nkeep`n"
Commit-All $R2 "old secret (outside the range)" | Out-Null
Invoke-G -C $R2 tag before | Out-Null
Write-Text (Join-Path $R2 "notes.txt") "keep`n"
Commit-All $R2 "remove it" | Out-Null
$res = Invoke-Scan -Range "before..HEAD" -RepoDir $R2
Assert-Exit $res "2 secret only in a removed line: exit 0" 0
Assert-Case "2 secret only in a removed line: says nothing found" ($res.Text.Contains("Nada encontrado")) "output:`n$($res.Text)"
# 2b. the commit outside the range is not scanned either.
$res = Invoke-Scan -Range "HEAD~1..HEAD" -RepoDir $R2
Assert-Exit $res "2b secret only before the range: exit 0" 0

# ------------------------------------------------------------------ 3. secret in the commit message
$R3 = New-Repo "message"
$v = "github_pat_" + (Get-Letters 30)
Write-Text (Join-Path $R3 "a.txt") "harmless`n"
$c3 = Commit-All $R3 "fix: use $v for the call"
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $R3
Assert-Exit $res "3 secret in the message: exit 1" 1
Assert-Case "3 secret in the message: commit and pattern named" ($res.Text.Contains("$c3 mensagem do commit :: github-pat")) "output:`n$($res.Text)"
Assert-Case "3 secret in the message: value never shown" (-not $res.Text.Contains($v)) "value leaked"

# ------------------------------------------------------------------ 4. secret in a file NAME (content harmless)
$R4 = New-Repo "name"
$v = "ghp_" + (Get-Letters 30)
Write-Text (Join-Path $R4 "docs\note-$v.md") "harmless`n"
$c4 = Commit-All $R4 "add note"
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $R4
Assert-Exit $res "4 secret in a file name: exit 1" 1
Assert-Case "4 secret in a file name: name masked, commit and pattern named" ($res.Text.Contains("$c4 docs/note-[oculto].md (no NOME do arquivo) :: github-token")) "output:`n$($res.Text)"
Assert-Case "4 secret in a file name: value never shown" (-not $res.Text.Contains($v)) "value leaked"

# ------------------------------------------------------------------ 5. forbidden file names (mixed case), no secret inside
$R5 = New-Repo "forbidden"
Write-Text (Join-Path $R5 "perfil\Candidate_Base.yaml") "name: someone`n"
$c5 = Commit-All $R5 "add profile"
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $R5
Assert-Exit $res "5a Candidate_Base.yaml: exit 1" 1
Assert-Case "5a Candidate_Base.yaml: named as forbidden" ($res.Text.Contains("$c5 perfil/Candidate_Base.yaml (arquivo proibido pelo nome) :: nome-proibido candidate_base.yaml")) "output:`n$($res.Text)"
Write-Text (Join-Path $R5 ".ENV") "X=1`n"
$c5b = Commit-All $R5 "add env"
$res = Invoke-Scan -Range "HEAD~1..HEAD" -RepoDir $R5
Assert-Exit $res "5b .ENV: exit 1" 1
Assert-Case "5b .ENV: named as forbidden" ($res.Text.Contains("$c5b .ENV (arquivo proibido pelo nome) :: nome-proibido .env*")) "output:`n$($res.Text)"
# 5c. an EMPTY forbidden file (no hunk, no +++ line) is still caught by name.
Write-Text (Join-Path $R5 "data\app.DB") ""
$c5c = Commit-All $R5 "add empty db"
$res = Invoke-Scan -Range "HEAD~1..HEAD" -RepoDir $R5
Assert-Exit $res "5c empty app.DB: exit 1" 1
Assert-Case "5c empty app.DB: named as forbidden" ($res.Text.Contains("$c5c data/app.DB (arquivo proibido pelo nome) :: nome-proibido *.db")) "output:`n$($res.Text)"
# 5d. A commit that DELETES a forbidden file adds nothing: exit 0. The file is only untracked (`git rm --cached`):
# it stays on disk, nothing is deleted.
Invoke-G -C $R5 rm -q --cached -- ".ENV" | Out-Null
Invoke-G -C $R5 commit -q -m "untrack env" | Out-Null
$st5 = (Invoke-G -C $R5 show --name-status --format= HEAD) -join "|"
Assert-Case "5d setup: the last commit deletes .ENV" ($st5 -match '^D\s+\.ENV$') "name-status: $st5"
$res = Invoke-Scan -Range "HEAD~1..HEAD" -RepoDir $R5
Assert-Exit $res "5d deleting a forbidden file: exit 0" 0

# ------------------------------------------------------------------ 6. clean range
$R6 = New-Repo "clean"
Write-Text (Join-Path $R6 "a.txt") "just text, the token is described in the docs`napiKey: <your key here>`n"
Commit-All $R6 "plain change" | Out-Null
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $R6
Assert-Exit $res "6 clean range: exit 0" 0
Assert-Case "6 clean range: says nothing found" ($res.Text.Contains("Nada encontrado")) "output:`n$($res.Text)"
# 6b. empty range (no commits).
$res = Invoke-Scan -Range "HEAD..HEAD" -RepoDir $R6
Assert-Exit $res "6b empty range: exit 0" 0

# ------------------------------------------------------------------ 7. environment errors -> exit 2
$res = Invoke-Scan -Range "base..no-such-branch" -RepoDir $R6
Assert-Exit $res "7a invalid range: exit 2" 2
$v = "sk-ant-" + (Get-Letters 30)
$res = Invoke-Scan -Range "$v..HEAD" -RepoDir $R6
Assert-Exit $res "7b invalid range holding a secret: exit 2" 2
Assert-Case "7b invalid range holding a secret: message masked" ($res.Text.Contains("[oculto]") -and -not $res.Text.Contains($v)) "output:`n$($res.Text)"
$res = Invoke-Scan -RepoDir $R6
Assert-Exit $res "7c missing -Range: exit 2" 2
$res = Invoke-Scan -Range "--all" -RepoDir $R6
Assert-Exit $res "7d option-like range refused: exit 2" 2
$NR = Join-Path $root "not-a-repo"
New-Item -ItemType Directory -Path $NR | Out-Null
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $NR
Assert-Exit $res "8 outside a repository: exit 2" 2
$v = "sk-ant-" + (Get-Letters 30)
$NR2 = Join-Path $root "not-a-repo-$v"
New-Item -ItemType Directory -Path $NR2 | Out-Null
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $NR2
Assert-Exit $res "8b outside a repository, secret in the folder name: exit 2" 2
Assert-Case "8b outside a repository: message masked" ($res.Text.Contains("[oculto]") -and -not $res.Text.Contains($v)) "output:`n$($res.Text)"

# ------------------------------------------------------------------ 9. secret in a commit OUTSIDE the range
$R9 = New-Repo "outside"
$v = "AKIA" + ((Get-Letters 16).ToUpperInvariant())
Write-Text (Join-Path $R9 "old.txt") "aws $v`n"
Commit-All $R9 "old" | Out-Null
Invoke-G -C $R9 tag sent | Out-Null
Write-Text (Join-Path $R9 "new.txt") "clean`n"
Commit-All $R9 "new" | Out-Null
$res = Invoke-Scan -Range "sent..HEAD" -RepoDir $R9
Assert-Exit $res "9 secret only outside the range: exit 0" 0
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $R9
Assert-Exit $res "9 control: the same commit inside the range: exit 1" 1
Assert-Case "9 control: value never shown" (-not $res.Text.Contains($v)) "value leaked"

# ------------------------------------------------------------------ 10. secret added and removed INSIDE the range
$R10 = New-Repo "transient"
$v = "xoxb-" + (Get-Letters 20)
Write-Text (Join-Path $R10 "a.txt") "slack $v`n"
$c10 = Commit-All $R10 "add"
Write-Text (Join-Path $R10 "a.txt") "gone`n"
Commit-All $R10 "remove" | Out-Null
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $R10
Assert-Exit $res "10 secret added then removed inside the range: exit 1" 1
Assert-Case "10 secret added then removed: names the commit that added it" ($res.Text.Contains("$c10 a.txt :: slack-token")) "output:`n$($res.Text)"

# ------------------------------------------------------------------ 11. binary files
$R11 = New-Repo "binary"
$v = "sk-ant-" + (Get-Letters 30)
$bytes = [byte[]](@(0, 1, 2, 0, 255) + $Utf8.GetBytes(" key $v ") + @(0, 0, 7))
Write-Bytes (Join-Path $R11 "img\blob.bin") $bytes
$c11 = Commit-All $R11 "add binary"
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $R11
Assert-Exit $res "11a secret inside a binary file: exit 1" 1
Assert-Case "11a secret inside a binary file: file and pattern named" ($res.Text.Contains("$c11 img/blob.bin (binario) :: anthropic-key")) "output:`n$($res.Text)"
Assert-Case "11a secret inside a binary file: value never shown" (-not $res.Text.Contains($v)) "value leaked"
$v = "ghp_" + (Get-Letters 30)
Write-Bytes (Join-Path $R11 "wide.txt") ([Text.Encoding]::Unicode.GetBytes("value $v`n"))
$c11b = Commit-All $R11 "add utf16 without bom"
$res = Invoke-Scan -Range "HEAD~1..HEAD" -RepoDir $R11
Assert-Exit $res "11b UTF-16 without BOM (seen as binary): exit 1" 1
Assert-Case "11b UTF-16 without BOM: file and pattern named" ($res.Text.Contains("$c11b wide.txt (binario) :: github-token")) "output:`n$($res.Text)"
# 11c. a binary over 20 MB is listed as not scanned, without failing.
$big = New-Object byte[] (21MB)
$big[10] = 1
Write-Bytes (Join-Path $R11 "big.bin") $big
$c11c = Commit-All $R11 "add big binary"
$res = Invoke-Scan -Range "HEAD~1..HEAD" -RepoDir $R11
Assert-Exit $res "11c binary over 20 MB: exit 0" 0
Assert-Case "11c binary over 20 MB: listed as not scanned" ($res.Text.Contains("$c11c big.bin: binario nao varrido")) "output:`n$($res.Text)"

# ------------------------------------------------------------------ 12. a change made in a MERGE commit itself
$R12 = New-Repo "merge"
Invoke-G -C $R12 checkout -q -b side | Out-Null
Write-Text (Join-Path $R12 "side.txt") "side`n"
Commit-All $R12 "side" | Out-Null
Invoke-G -C $R12 checkout -q main | Out-Null
Write-Text (Join-Path $R12 "main.txt") "main`n"
Commit-All $R12 "main" | Out-Null
Invoke-G -C $R12 merge -q --no-ff --no-commit side | Out-Null
$v = "lin_api_" + (Get-Letters 30)
Write-Text (Join-Path $R12 "merge-fix.txt") "fixed with $v`n"
$c12 = Commit-All $R12 "merge side"
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $R12
Assert-Exit $res "12 secret added in the merge commit: exit 1" 1
Assert-Case "12 secret added in the merge commit: merge commit and pattern named" ($res.Text.Contains("$c12 merge-fix.txt :: linear-key")) "output:`n$($res.Text)"

# ------------------------------------------------------------------ 12b. path with spaces and non-ASCII letters
$R12b = New-Repo "spaces"
$v = "sk-proj-" + (Get-Letters 30)
$odd = "pasta com espaco/relat" + [char]0x00F3 + "rio final.txt"
Write-Text (Join-Path $R12b ($odd -replace '/', '\')) "OPENAI_API_KEY=$v`n"
$c12b = Commit-All $R12b "odd path"
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $R12b
Assert-Exit $res "12b path with spaces: exit 1" 1
Assert-Case "12b path with spaces: full path named" ($res.Text.Contains("$c12b pasta com espaco/relat") -and $res.Text.Contains("rio final.txt :: openai-project-key")) "output:`n$($res.Text)"

# ------------------------------------------------------------------ 14. forbidden FOLDERS (any level, any case) -> exit 1
$R14 = New-Repo "folders"
foreach ($case in @(
        @("config/.Env.d/inner.txt", ".env*/"),
        @("deep/a/Secrets/s.txt", "secrets/"),
        @("x/My-Credentials/c.txt", "*credential*/"),
        @("sandbox-lab/shared/secrets/k.txt", "sandbox-lab/shared/secrets/"))) {
    Write-Text (Join-Path $R14 ($case[0] -replace '/', '\')) "harmless`n"
    $c14 = Commit-All $R14 "add $($case[0])"
    $res = Invoke-Scan -Range "HEAD~1..HEAD" -RepoDir $R14
    Assert-Exit $res "14 file inside forbidden folder $($case[0]): exit 1" 1
    Assert-Case "14 file inside forbidden folder $($case[0]): named" ($res.Text.Contains("$c14 $($case[0]) (dentro de pasta proibida) :: pasta-proibida $($case[1])")) "output:`n$($res.Text)"
}
# 14 control: similar names that are not forbidden folders.
Write-Text (Join-Path $R14 "docs\secretsauce\a.txt") "harmless`n"
Write-Text (Join-Path $R14 "docs\secrets.md") "harmless`n"
Commit-All $R14 "control" | Out-Null
$res = Invoke-Scan -Range "HEAD~1..HEAD" -RepoDir $R14
Assert-Exit $res "14 control: secretsauce/ and secrets.md are not forbidden: exit 0" 0

# ------------------------------------------------------------------ 15. WARNINGS: personal paths and e-mails (exit 0)
$R15 = New-Repo "warnings"
$n = Get-Letters 9
Write-Text (Join-Path $R15 "notes.md") ("a C:\Users\$n\proj`nb C:/Users/$n/proj`nc D:\\Users\\$n\\x`nd /home/$n/src`ne /Users/$n/src`n" +
    "f C:Users$n" + "PycharmProjects`ng placeholder C:\Users\<nome>\x stays quiet`n")
$c15 = Commit-All $R15 "paths"
$res = Invoke-Scan -Range "HEAD~1..HEAD" -RepoDir $R15
Assert-Exit $res "15a personal paths only: exit 0" 0
Assert-Case "15a personal paths: warning section with count" ($res.Text.Contains("Avisos (nao bloqueiam): 6")) "output:`n$($res.Text)"
foreach ($shown in @("C:\Users\[oculto]", "C:/Users/[oculto]", "D:\\Users\\[oculto]", "/home/[oculto]", "/Users/[oculto]", "C:Users[oculto]")) {
    Assert-Case "15a personal path warned and masked: $shown" ($res.Text.Contains("$c15 notes.md :: caminho pessoal $shown")) "output:`n$($res.Text)"
}
Assert-Case "15a personal paths: user name never shown" (-not $res.Text.Contains($n)) "name leaked. Output:`n$($res.Text)"
Assert-Case "15a personal paths: says nothing found (only warnings)" ($res.Text.Contains("Nada encontrado")) "output:`n$($res.Text)"
$local = Get-Letters 8
Write-Text (Join-Path $R15 "contact.md") "write to $($local).x@exemplo.com.br please`n"
$c15b = Commit-All $R15 "email"
$res = Invoke-Scan -Range "HEAD~1..HEAD" -RepoDir $R15
Assert-Exit $res "15b ordinary e-mail: exit 0" 0
Assert-Case "15b ordinary e-mail: warned with the local part masked" ($res.Text.Contains("$c15b contact.md :: e-mail [oculto]@exemplo.com.br") -and $res.Text.Contains("Avisos (nao bloqueiam): 1")) "output:`n$($res.Text)"
Assert-Case "15b ordinary e-mail: local part never shown" (-not $res.Text.Contains($local)) "local part leaked"
Write-Text (Join-Path $R15 "bot.md") "Co-Authored-By: Bot <noreply@anthropic.com>`nby 12345+someone@users.noreply.github.com`n"
Commit-All $R15 "noreply" | Out-Null
$res = Invoke-Scan -Range "HEAD~1..HEAD" -RepoDir $R15
Assert-Exit $res "15c noreply addresses: exit 0" 0
Assert-Case "15c noreply addresses: no warning" (-not $res.Text.Contains("Avisos")) "output:`n$($res.Text)"
$n2 = Get-Letters 9
$v = "sk-ant-" + (Get-Letters 30)
Write-Text (Join-Path $R15 "both.md") "path C:\Users\$n2\x`nkey $v`n"
$c15d = Commit-All $R15 "both"
$res = Invoke-Scan -Range "HEAD~1..HEAD" -RepoDir $R15
Assert-Exit $res "15d personal path plus secret: exit 1" 1
Assert-Case "15d personal path plus secret: finding and warning both listed" ($res.Text.Contains("$c15d both.md :: anthropic-key") -and $res.Text.Contains("$c15d both.md :: caminho pessoal C:\Users\[oculto]")) "output:`n$($res.Text)"
Assert-Case "15d personal path plus secret: neither value shown" (-not $res.Text.Contains($v) -and -not $res.Text.Contains($n2)) "value leaked"

# ------------------------------------------------------------------ 13. read only: refs, HEAD and index unchanged
$snap = { param($repo) ((Invoke-G -C $repo for-each-ref "--format=%(refname) %(objectname)") -join "|") + "#" + ((Invoke-G -C $repo rev-parse HEAD) -join "") + "#" + ((Invoke-G -C $repo status --porcelain) -join "|") }
$before = & $snap $R12
$res = Invoke-Scan -Range "base..HEAD" -RepoDir $R12
Assert-Case "13 refs, HEAD and status unchanged by a scan" ((& $snap $R12) -ceq $before) "repository changed"

Write-Output "Temp folder (left behind): $root"
if ($fail -eq 0) { Write-Output "ALL PASSED" } else { Write-Output "SOME FAILED" }
exit $fail
