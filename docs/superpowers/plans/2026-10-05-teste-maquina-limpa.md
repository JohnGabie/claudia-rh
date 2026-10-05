# Teste em máquina limpa — plano de implementação

> **Para agentes executores:** SUB-SKILL OBRIGATÓRIA: use superpowers:subagent-driven-development para executar este plano tarefa a tarefa. Os passos usam checkbox (`- [ ]`).

**Objetivo:** dar ao agente uma skill (`claudia-sandbox`) e scripts de cenário de dados para testar o Claudia RH num Windows Sandbox limpo, usando um laboratório mantido fora do repositório.

**Arquitetura:** tudo o que é específico do app vive em `.claude/skills/claudia-sandbox/` (skill, scripts PowerShell, fixtures, referências, evals). O laboratório é localizado por `SANDBOX_LAB_DIR` e usado só pela sua interface pública (`scripts\app-stage.ps1`, perfis `lab.config.app*.json`, scripts do guest). Cada cenário produz uma pasta com `appdata\io.github.johngabie.claudia-rh\` e `cenario.json`; `publicar.ps1` entrega app + cenário ao laboratório.

**Stack:** PowerShell 7 (compatível com Windows PowerShell 5.1), sqlite3 do laboratório, Node ≥ 22 (evals existentes), skill-creator.

**Spec:** `docs/superpowers/specs/2026-10-05-teste-maquina-limpa-design.md`

## Restrições globais

- Identifier do app: `io.github.johngabie.claudia-rh` (nunca alterar).
- Comentários de código e mensagens de commit em inglês; textos da skill, referências e docs em pt-BR.
- Sem dependências novas para o usuário final; nada instalado globalmente na máquina do dev.
- Scripts não escrevem fora da pasta `-Saida`/`-WorkDir` informada; a origem do cenário `copia` só é lida.
- `copia` sem `-Autorizado` sai com exit 2 e não lê a origem.
- `disparo.json` gravado pelos scripts tem sempre todos os campos (`ativo`, `limiar_minutos`, `limite_diario`, `limite_tempo_minutos`, `janelas`): sem `limiar_minutos` o app ignora o arquivo e usa `ativo: true` (`src-tauri/src/lib.rs`, `load_idle_config`).
- Arquivos agnósticos: nenhum arquivo novo ou alterado menciona sessões de trabalho, pessoas, modelos, nomes de máquina, IPs, caminhos pessoais ou pastas fora do repositório além de `SANDBOX_LAB_DIR`/`LAB_WORK_DIR`/`%TEMP%`/`%APPDATA%`/`%LOCALAPPDATA%`.
- Cada agente executor trabalha em worktree própria e usa `CARGO_TARGET_DIR` fora do `target/` do repositório quando compilar.

## Foco de revisão

1. Origem do `copia` com o app aberto (banco em uso, `-wal` com dados não consolidados): a cópia precisa abrir e conter as linhas do `-wal`. Teste na Tarefa 4 (`copia consolida wal`).
2. `disparo.json` existente com campos extras ou faltando `limiar_minutos`: o resultado precisa ser JSON completo com `ativo: false`. Teste na Tarefa 2 (`automacao completa a partir de arquivo parcial`).
3. `SANDBOX_LAB_DIR` apontando para pasta sem `scripts\app-stage.ps1`: erro que nomeia o arquivo, sem copiar nada. Teste na Tarefa 5.
4. `-Saida` já existente com lixo de execução anterior: a pasta é recriada, sem sobras. Teste na Tarefa 2 (`saida recriada`).
5. Caminhos com espaços (`-Saida`, `-Origem`, `-App`): todos os scripts funcionam. Coberto usando pastas temporárias com espaço no nome em todos os testes.

---

## Estrutura de arquivos

| Arquivo | Responsabilidade |
|---|---|
| `.claude/skills/claudia-sandbox/scripts/comum.ps1` | constantes e funções: `$AppId`, `$RepoRoot`, `Get-LabDir`, `Get-SqliteExe`, `Invoke-Sqlite`, `New-CenarioDir`, `Write-Cenario`, `Set-AutomacaoDesligada`, `Write-Utf8` |
| `.../scripts/cenario-novo.ps1` | cenário sem dados |
| `.../scripts/cenario-mock.ps1` | cenário com fixtures |
| `.../scripts/cenario-copia.ps1` | cenário com cópia dos dados do usuário |
| `.../scripts/publicar.ps1` | app + cenário → `app-stage.ps1` |
| `.../scripts/test/assert.ps1` | `Assert-True`, `Complete-Tests`, `New-TempDir`, `Invoke-Script` |
| `.../scripts/test/*.test.ps1` | um arquivo de teste por script |
| `.../scripts/test/run-all.ps1` | roda todos os testes; exit ≠ 0 se algum falhar |
| `.../fixtures/*` | dados fictícios |
| `.../references/limites.md`, `pastas-de-dados.md` | referências da skill |
| `.../SKILL.md`, `.../evals/evals.json` | skill e evals de disparo |
| `.claude/skills/claudia-teste/SKILL.md`, `evals/evals.json` | ponteiro para a skill nova |
| `CLAUDE.md` | seção "Teste em máquina limpa" |

## Ondas de execução

- **Onda 1:** Tarefa 1.
- **Onda 2 (paralelo, arquivos distintos):** Tarefas 2, 3, 4, 5, 7, 8.
- **Onda 3:** Tarefa 6 (skill; documenta os scripts prontos).
- **Onda 4:** Tarefa 9 (smoke no Sandbox) e Tarefa 10 (revisão + varredura).
- **Fim:** integração decidida pelo usuário (Tarefa 11).

---

### Tarefa 1: base comum e harness de teste

**Arquivos:**
- Criar: `.claude/skills/claudia-sandbox/scripts/comum.ps1`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/assert.ps1`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/comum.test.ps1`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/run-all.ps1`

**Interfaces:**
- Produz (`comum.ps1`, carregado com `. (Join-Path $PSScriptRoot 'comum.ps1')`):
  - `$AppId = 'io.github.johngabie.claudia-rh'`
  - `$RepoRoot` — raiz do repositório (4 níveis acima de `scripts/`)
  - `Get-LabDir() -> string` — valida `SANDBOX_LAB_DIR` e `scripts\app-stage.ps1`; lança exceção com o nome do que falta
  - `Get-SqliteExe([string]$Sqlite) -> string` — `-Sqlite` explícito, senão `<lab>\shared\tools\sqlite\sqlite3.exe`, senão `sqlite3` no PATH; lança exceção listando os caminhos tentados
  - `Invoke-Sqlite([string]$Exe, [string]$Db, [string]$Sql) -> string[]` — envia `$Sql` por stdin; lança exceção se exit ≠ 0
  - `New-CenarioDir([string]$Saida, [string]$Nome) -> string` — recria a pasta; cria `appdata\$AppId` e `localappdata`; devolve caminho absoluto
  - `Write-Cenario([string]$Dir, [string]$Nome, [string]$Origem) -> string` — grava e devolve o JSON compacto de `cenario.json`
  - `Set-AutomacaoDesligada([string]$DataDir)` — grava `disparo.json` completo com `ativo:false` (preserva campos existentes) e `modo_autonomo.json` = `{"skip_permissions":false}`
  - `Write-Utf8([string]$Path, [string]$Text)` — UTF-8 sem BOM
- Produz (`assert.ps1`): `Assert-True([bool]$Cond, [string]$Nome)`, `Complete-Tests()`, `New-TempDir([string]$Prefixo) -> string` (nome com espaço), `Invoke-Script([string]$Path, [string[]]$ScriptArgs) -> @{ Code; Out; Err }`

- [ ] **Passo 1: escrever `assert.ps1`**

```powershell
# Minimal assertion helpers shared by the scenario tests.
$script:Falhas = 0

function Assert-True([bool]$Cond, [string]$Nome) {
  if ($Cond) { Write-Host "PASS $Nome" } else { Write-Host "FAILED $Nome" -ForegroundColor Red; $script:Falhas++ }
}

function Complete-Tests {
  if ($script:Falhas) { Write-Host "$($script:Falhas) failure(s)"; exit 1 }
  Write-Host 'OK'; exit 0
}

function New-TempDir([string]$Prefixo) {
  $d = Join-Path ([IO.Path]::GetTempPath()) ("cs test $Prefixo " + [guid]::NewGuid().ToString('N').Substring(0, 8))
  New-Item -ItemType Directory -Force $d | Out-Null
  return $d
}

function Invoke-Script([string]$Path, [string[]]$ScriptArgs) {
  $errFile = [IO.Path]::GetTempFileName()
  $out = & pwsh -NoProfile -File $Path @ScriptArgs 2>$errFile
  $code = $LASTEXITCODE
  $err = Get-Content $errFile -Raw; Remove-Item $errFile
  return @{ Code = $code; Out = @($out); Err = $err }
}
```

- [ ] **Passo 2: escrever o teste `comum.test.ps1` (falha: `comum.ps1` não existe)**

```powershell
. (Join-Path $PSScriptRoot 'assert.ps1')
. (Join-Path $PSScriptRoot '..\comum.ps1')

# New-CenarioDir recreates the folder
$base = New-TempDir 'comum'
$saida = Join-Path $base 'saida x'
New-Item -ItemType Directory -Force (Join-Path $saida 'lixo') | Out-Null
$dir = New-CenarioDir $saida 'teste'
Assert-True (-not (Test-Path (Join-Path $dir 'lixo'))) 'saida recriada'
Assert-True (Test-Path (Join-Path $dir "appdata\$AppId")) 'appdata do app criado'
Assert-True (Test-Path (Join-Path $dir 'localappdata')) 'localappdata criado'

# Write-Cenario contract
$json = Write-Cenario $dir 'teste' 'origem-x' | ConvertFrom-Json
$file = Get-Content (Join-Path $dir 'cenario.json') -Raw | ConvertFrom-Json
Assert-True ($json.cenario -eq 'teste' -and $file.cenario -eq 'teste') 'cenario.json gravado e devolvido'
Assert-True ($null -ne $file.criadoEm -and $file.origem -eq 'origem-x') 'cenario.json com criadoEm e origem'

# Set-AutomacaoDesligada from a partial file
$data = Join-Path $dir "appdata\$AppId"
Write-Utf8 (Join-Path $data 'disparo.json') '{"ativo":true,"limite_diario":3}'
Set-AutomacaoDesligada $data
$d = Get-Content (Join-Path $data 'disparo.json') -Raw | ConvertFrom-Json
Assert-True ($d.ativo -eq $false -and $d.limiar_minutos -eq 15 -and $d.limite_diario -eq 3 -and $null -ne $d.janelas) 'automacao completa a partir de arquivo parcial'
$m = Get-Content (Join-Path $data 'modo_autonomo.json') -Raw | ConvertFrom-Json
Assert-True ($m.skip_permissions -eq $false) 'modo autonomo desligado'

# Get-LabDir validation
$old = $env:SANDBOX_LAB_DIR
$env:SANDBOX_LAB_DIR = $null
$msg = try { Get-LabDir; '' } catch { $_.Exception.Message }
Assert-True ($msg -match 'SANDBOX_LAB_DIR') 'lab ausente nomeia a variavel'
$env:SANDBOX_LAB_DIR = $base
$msg = try { Get-LabDir; '' } catch { $_.Exception.Message }
Assert-True ($msg -match 'app-stage\.ps1') 'lab incompleto nomeia o arquivo'
$env:SANDBOX_LAB_DIR = $old

Remove-Item $base -Recurse -Force
Complete-Tests
```

- [ ] **Passo 3: rodar e ver falhar**

Rodar: `pwsh -NoProfile -File .claude/skills/claudia-sandbox/scripts/test/comum.test.ps1`
Esperado: erro ao carregar `comum.ps1` (arquivo inexistente), exit ≠ 0.

- [ ] **Passo 4: escrever `comum.ps1`**

```powershell
# Shared constants and helpers for the clean-machine scenario scripts.
$AppId = 'io.github.johngabie.claudia-rh'
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path

function Write-Utf8([string]$Path, [string]$Text) {
  [IO.File]::WriteAllText($Path, $Text, (New-Object Text.UTF8Encoding($false)))
}

function Get-LabDir {
  $d = $env:SANDBOX_LAB_DIR
  if (-not $d) { throw 'SANDBOX_LAB_DIR is not set: point it to the Windows Sandbox lab folder.' }
  $stage = Join-Path $d 'scripts\app-stage.ps1'
  if (-not (Test-Path $stage)) { throw "Incomplete lab: $stage not found." }
  return (Resolve-Path $d).Path
}

function Get-SqliteExe([string]$Sqlite) {
  $tried = @()
  if ($Sqlite) { if (Test-Path $Sqlite) { return $Sqlite }; $tried += $Sqlite }
  if ($env:SANDBOX_LAB_DIR) {
    $p = Join-Path $env:SANDBOX_LAB_DIR 'shared\tools\sqlite\sqlite3.exe'
    if (Test-Path $p) { return $p }; $tried += $p
  }
  $c = Get-Command sqlite3 -ErrorAction SilentlyContinue
  if ($c) { return $c.Source }; $tried += 'sqlite3 (PATH)'
  throw "sqlite3 not found. Tried: $($tried -join '; ')"
}

function Invoke-Sqlite([string]$Exe, [string]$Db, [string]$Sql) {
  $out = $Sql | & $Exe $Db 2>&1
  if ($LASTEXITCODE -ne 0) { throw "sqlite3 failed ($LASTEXITCODE) on ${Db}: $out" }
  return $out
}

function New-CenarioDir([string]$Saida, [string]$Nome) {
  if (-not $Saida) { $Saida = Join-Path $env:TEMP "claudia-sandbox\$Nome" }
  if (Test-Path $Saida) { Remove-Item $Saida -Recurse -Force }
  New-Item -ItemType Directory -Force (Join-Path $Saida "appdata\$AppId"), (Join-Path $Saida 'localappdata') | Out-Null
  return (Resolve-Path $Saida).Path
}

function Write-Cenario([string]$Dir, [string]$Nome, [string]$Origem) {
  $root = Join-Path $Dir 'appdata'
  $files = @(Get-ChildItem $root -Recurse -File | ForEach-Object { $_.FullName.Substring($Dir.Length + 1) })
  $o = [ordered]@{ cenario = $Nome; criadoEm = (Get-Date).ToString('o'); origem = $Origem; arquivos = $files }
  $json = $o | ConvertTo-Json -Depth 4 -Compress
  Write-Utf8 (Join-Path $Dir 'cenario.json') $json
  return $json
}

function Set-AutomacaoDesligada([string]$DataDir) {
  # The app falls back to ativo=true when disparo.json misses required fields, so always write the full shape.
  $cfg = [ordered]@{ ativo = $false; limiar_minutos = 15; limite_diario = 10; limite_tempo_minutos = 0; janelas = @() }
  $p = Join-Path $DataDir 'disparo.json'
  if (Test-Path $p) {
    try {
      $old = Get-Content $p -Raw | ConvertFrom-Json
      foreach ($k in 'limiar_minutos', 'limite_diario', 'limite_tempo_minutos') { if ($null -ne $old.$k) { $cfg[$k] = $old.$k } }
      if ($null -ne $old.janelas) { $cfg.janelas = @($old.janelas) }
    } catch { }
  }
  $cfg.ativo = $false
  Write-Utf8 $p ($cfg | ConvertTo-Json -Depth 6 -Compress)
  Write-Utf8 (Join-Path $DataDir 'modo_autonomo.json') '{"skip_permissions":false}'
}
```

- [ ] **Passo 5: escrever `run-all.ps1`**

```powershell
# Runs every *.test.ps1 in this folder; exits non-zero if any fails.
$fail = 0
foreach ($t in Get-ChildItem $PSScriptRoot -Filter *.test.ps1 | Sort-Object Name) {
  Write-Host "== $($t.Name)"
  & pwsh -NoProfile -File $t.FullName
  if ($LASTEXITCODE -ne 0) { $fail++ }
}
if ($fail) { Write-Host "$fail test file(s) failed"; exit 1 }
Write-Host 'ALL OK'; exit 0
```

- [ ] **Passo 6: rodar e ver passar**

Rodar: `pwsh -NoProfile -File .claude/skills/claudia-sandbox/scripts/test/run-all.ps1; echo "exit=$LASTEXITCODE"`
Esperado: todas as linhas `PASS`, `ALL OK`, `exit=0`.

- [ ] **Passo 7: prova de mutação**

Em `Set-AutomacaoDesligada`, trocar `$cfg.ativo = $false` por `$cfg.ativo = $true`; rodar `comum.test.ps1`; confirmar a linha `FAILED automacao completa a partir de arquivo parcial`; restaurar e rodar de novo (verde).

- [ ] **Passo 8: commit**

```bash
git add .claude/skills/claudia-sandbox/scripts
git commit -m "feat(skill): shared helpers and test harness for clean-machine scenarios"
```

---

### Tarefa 2: cenário `novo`

**Arquivos:**
- Criar: `.claude/skills/claudia-sandbox/scripts/cenario-novo.ps1`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/novo.test.ps1`

**Interfaces:**
- Consome: `comum.ps1` (Tarefa 1), `assert.ps1`.
- Produz: `cenario-novo.ps1 [-Saida <pasta>]` → exit 0; última linha do stdout = JSON de `cenario.json` com `cenario: "novo"`, `origem: null`, `arquivos: []`.

- [ ] **Passo 1: teste (falha: script inexistente)**

```powershell
. (Join-Path $PSScriptRoot 'assert.ps1')
$script = Join-Path $PSScriptRoot '..\cenario-novo.ps1'
$base = New-TempDir 'novo'
$saida = Join-Path $base 'out dir'
$r = Invoke-Script $script @('-Saida', $saida)
Assert-True ($r.Code -eq 0) 'novo exit 0'
$j = $r.Out[-1] | ConvertFrom-Json
Assert-True ($j.cenario -eq 'novo' -and @($j.arquivos).Count -eq 0) 'novo sem arquivos'
Assert-True (Test-Path (Join-Path $saida 'appdata\io.github.johngabie.claudia-rh')) 'novo cria pasta de dados vazia'
Remove-Item $base -Recurse -Force
Complete-Tests
```

- [ ] **Passo 2: rodar e ver falhar** — `pwsh -NoProfile -File .claude/skills/claudia-sandbox/scripts/test/novo.test.ps1` → `FAILED novo exit 0`.

- [ ] **Passo 3: implementar**

```powershell
<#
.SYNOPSIS
  Clean-machine scenario "novo": empty app data folder, so the app starts from scratch.
.PARAMETER Saida  Output folder (default: %TEMP%\claudia-sandbox\novo). Recreated on every run.
#>
param([string]$Saida = '')
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'comum.ps1')
try {
  $dir = New-CenarioDir $Saida 'novo'
  Write-Output (Write-Cenario $dir 'novo' $null)
  exit 0
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
```

- [ ] **Passo 4: rodar e ver passar** — mesmo comando, exit 0.
- [ ] **Passo 5: commit** — `git commit -m "feat(skill): empty-data clean-machine scenario"`

---

### Tarefa 3: cenário `mock` e fixtures

**Arquivos:**
- Criar: `.claude/skills/claudia-sandbox/fixtures/candidate_base.yaml`, `search_variants.yaml`, `strategy.md`, `seed.sql`
- Criar: `.claude/skills/claudia-sandbox/scripts/cenario-mock.ps1`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/mock.test.ps1`

**Interfaces:**
- Consome: `comum.ps1`; `src-tauri/src/db/schema.sql`; sqlite3 via `Get-SqliteExe`.
- Produz: `cenario-mock.ps1 [-Saida <pasta>] [-Sqlite <exe>]` → pasta com `candidate_base.yaml`, `search_variants.yaml`, `strategy.md`, `claudia_rh.db`, `welcome_seen`, `disparo.json` (ativo false), `modo_autonomo.json`.

- [ ] **Passo 1: fixtures**

`candidate_base.yaml`: pessoa fictícia ("Ana Exemplo", e-mail `ana@example.com`, links `example.com`), seguindo **exatamente** o schema `PROFILE_SCHEMA` em `src-tauri/src/commands/prompts.rs` (chaves de topo: `dados_pessoais`, `experiencia`, `projetos`, `formacao`, `competencias`, `idiomas`, `fontes_usadas`, `gaps_conhecidos`, `respostas_modelo`, `ultima_atualizacao`; `links` e `fontes_usadas` como listas de objetos; `fim` sempre string; `conquistas`/`tecnologias` sempre listas). Duas experiências, um projeto, uma formação.

`search_variants.yaml`: duas variantes no formato de `prompts.rs` (seção "Mandatory schema — search_variants.yaml"), pesos somando 100, `ativa: true`.

`strategy.md`: três linhas de estratégia fictícia.

`seed.sql`:

```sql
INSERT INTO vagas (titulo, empresa, plataforma, url, localizacao, modelo_trabalho, idioma, descoberta_em, status, match_score) VALUES
 ('Backend Developer', 'Exemplo Ltda', 'linkedin', 'https://example.com/vagas/1', 'Remoto', 'remoto', 'pt', '2026-01-10T09:00:00Z', 'descoberta', 'alto'),
 ('Fullstack Engineer', 'Demo SA', 'linkedin', 'https://example.com/vagas/2', 'Lisboa', 'hibrido', 'en', '2026-01-11T09:00:00Z', 'candidatada', 'medio'),
 ('Data Analyst', 'Teste Corp', 'outra', 'https://example.com/vagas/3', 'Porto', 'presencial', 'pt', '2026-01-12T09:00:00Z', 'descartada', 'baixo');
INSERT INTO candidaturas (vaga_id, enviada_em, pasta_arquivos, metodo) VALUES
 (2, '2026-01-11T10:00:00Z', 'candidaturas/2', 'formulario');
INSERT INTO pendencias (vaga_id, criada_em, categoria, descricao) VALUES
 (1, '2026-01-10T09:30:00Z', 'pergunta', 'Pretensao salarial nao definida no perfil');
INSERT INTO propostas_perfil (vaga_id, criada_em, pergunta, contexto) VALUES
 (1, '2026-01-10T09:31:00Z', 'Tem experiencia com Kubernetes?', 'Requisito da vaga 1');
INSERT INTO sessoes (iniciada_em, terminada_em, motivo_disparo, motivo_termino, vagas_processadas) VALUES
 ('2026-01-11T09:00:00Z', '2026-01-11T09:40:00Z', 'manual', 'concluida', 3);
```

- [ ] **Passo 2: teste (falha: script inexistente)**

```powershell
. (Join-Path $PSScriptRoot 'assert.ps1')
. (Join-Path $PSScriptRoot '..\comum.ps1')
$script = Join-Path $PSScriptRoot '..\cenario-mock.ps1'
$base = New-TempDir 'mock'
$saida = Join-Path $base 'out dir'
$r = Invoke-Script $script @('-Saida', $saida)
Assert-True ($r.Code -eq 0) "mock exit 0 $($r.Err)"
$data = Join-Path $saida "appdata\$AppId"
foreach ($f in 'candidate_base.yaml', 'search_variants.yaml', 'strategy.md', 'claudia_rh.db', 'welcome_seen', 'disparo.json', 'modo_autonomo.json') {
  Assert-True (Test-Path (Join-Path $data $f)) "mock tem $f"
}
$exe = Get-SqliteExe ''
$n = Invoke-Sqlite $exe (Join-Path $data 'claudia_rh.db') 'SELECT count(*) FROM vagas;'
Assert-True ("$n".Trim() -eq '3') 'mock banco com as vagas do seed'
$d = Get-Content (Join-Path $data 'disparo.json') -Raw | ConvertFrom-Json
Assert-True ($d.ativo -eq $false -and $null -ne $d.limiar_minutos) 'mock disparo desligado e completo'
$y = Get-Content (Join-Path $data 'candidate_base.yaml') -Raw
foreach ($k in 'dados_pessoais:', 'experiencia:', 'projetos:', 'formacao:', 'competencias:', 'idiomas:', 'fontes_usadas:', 'gaps_conhecidos:', 'respostas_modelo:', 'ultima_atualizacao:') {
  Assert-True ($y -match "(?m)^$k") "mock yaml tem $k"
}
$j = $r.Out[-1] | ConvertFrom-Json
Assert-True ($j.cenario -eq 'mock') 'mock cenario.json'
Remove-Item $base -Recurse -Force
Complete-Tests
```

Pré-requisito do teste: `SANDBOX_LAB_DIR` definido (sqlite3 do laboratório) ou `sqlite3` no PATH.

- [ ] **Passo 3: rodar e ver falhar** — `FAILED mock exit 0`.

- [ ] **Passo 4: implementar**

```powershell
<#
.SYNOPSIS
  Clean-machine scenario "mock": fictional profile and database built from the repo schema.
.PARAMETER Saida   Output folder (default: %TEMP%\claudia-sandbox\mock). Recreated on every run.
.PARAMETER Sqlite  sqlite3.exe to use (default: the lab's, then PATH).
#>
param([string]$Saida = '', [string]$Sqlite = '')
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'comum.ps1')
try {
  $exe = Get-SqliteExe $Sqlite
  $dir = New-CenarioDir $Saida 'mock'
  $data = Join-Path $dir "appdata\$AppId"
  $fx = Join-Path $PSScriptRoot '..\fixtures'
  foreach ($f in 'candidate_base.yaml', 'search_variants.yaml', 'strategy.md') { Copy-Item (Join-Path $fx $f) $data }
  $schema = Get-Content (Join-Path $RepoRoot 'src-tauri\src\db\schema.sql') -Raw
  $seed = Get-Content (Join-Path $fx 'seed.sql') -Raw
  Invoke-Sqlite $exe (Join-Path $data 'claudia_rh.db') ($schema + "`n" + $seed) | Out-Null
  Write-Utf8 (Join-Path $data 'welcome_seen') '1'
  Set-AutomacaoDesligada $data
  Write-Output (Write-Cenario $dir 'mock' 'fixtures')
  exit 0
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
```

- [ ] **Passo 5: rodar e ver passar.**
- [ ] **Passo 6: prova de mutação** — remover a linha `Set-AutomacaoDesligada $data`; confirmar `FAILED mock disparo desligado e completo`; restaurar.
- [ ] **Passo 7: commit** — `git commit -m "feat(skill): mock-data clean-machine scenario with fictional fixtures"`

---

### Tarefa 4: cenário `copia`

**Arquivos:**
- Criar: `.claude/skills/claudia-sandbox/scripts/cenario-copia.ps1`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/copia.test.ps1`

**Interfaces:**
- Consome: `comum.ps1`.
- Produz: `cenario-copia.ps1 -Autorizado [-Origem <pasta>] [-Saida <pasta>] [-ManterAutomacao] [-Sqlite <exe>]`. Sem `-Autorizado`: exit 2, mensagem pedindo autorização, nada lido. `-Origem` padrão: `%APPDATA%\io.github.johngabie.claudia-rh`. `%LOCALAPPDATA%` nunca é copiado.

- [ ] **Passo 1: teste (falha: script inexistente)**

```powershell
. (Join-Path $PSScriptRoot 'assert.ps1')
. (Join-Path $PSScriptRoot '..\comum.ps1')
$script = Join-Path $PSScriptRoot '..\cenario-copia.ps1'
$exe = Get-SqliteExe ''
$base = New-TempDir 'copia'
$origem = Join-Path $base 'origem dados'
New-Item -ItemType Directory -Force (Join-Path $origem 'sub') | Out-Null
Write-Utf8 (Join-Path $origem 'candidate_base.yaml') 'dados_pessoais: {}'
Write-Utf8 (Join-Path $origem 'sub\nota.txt') 'x'
Write-Utf8 (Join-Path $origem 'disparo.json') '{"ativo":true,"limiar_minutos":5,"limite_diario":7,"limite_tempo_minutos":0,"janelas":[]}'
# Database whose last row lives only in the -wal file: sqlite3 folds the WAL on close,
# so copy db + wal while a connection is still open.
$vivo = Join-Path $base 'vivo'; New-Item -ItemType Directory $vivo | Out-Null
$psi = New-Object Diagnostics.ProcessStartInfo $exe, ('"' + (Join-Path $vivo 'claudia_rh.db') + '"')
$psi.RedirectStandardInput = $true; $psi.UseShellExecute = $false
$p = [Diagnostics.Process]::Start($psi)
$p.StandardInput.WriteLine('PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE t(x); INSERT INTO t VALUES (42);')
$p.StandardInput.Flush(); Start-Sleep -Seconds 1
Copy-Item (Join-Path $vivo 'claudia_rh.db'), (Join-Path $vivo 'claudia_rh.db-wal') $origem
$p.StandardInput.WriteLine('.quit'); $p.WaitForExit()
Assert-True (Test-Path (Join-Path $origem 'claudia_rh.db-wal')) 'fixture tem wal com dados'
function Get-TreeHash($p) { (Get-ChildItem $p -Recurse -File | Sort-Object FullName | ForEach-Object { (Get-FileHash $_.FullName).Hash }) -join ',' }
$antes = Get-TreeHash $origem

$r = Invoke-Script $script @('-Origem', $origem, '-Saida', (Join-Path $base 'sem'))
Assert-True ($r.Code -eq 2) 'copia sem autorizacao sai com 2'
Assert-True (-not (Test-Path (Join-Path $base 'sem\appdata'))) 'copia sem autorizacao nao copia nada'

$saida = Join-Path $base 'out dir'
$r = Invoke-Script $script @('-Autorizado', '-Origem', $origem, '-Saida', $saida)
Assert-True ($r.Code -eq 0) "copia exit 0 $($r.Err)"
$data = Join-Path $saida "appdata\$AppId"
Assert-True (Test-Path (Join-Path $data 'sub\nota.txt')) 'copia recursiva'
$v = Invoke-Sqlite $exe (Join-Path $data 'claudia_rh.db') 'SELECT x FROM t;'
Assert-True ("$v".Trim() -eq '42') 'copia consolida wal'
Assert-True (-not (Test-Path (Join-Path $data 'claudia_rh.db-wal'))) 'copia sem arquivo wal'
$d = Get-Content (Join-Path $data 'disparo.json') -Raw | ConvertFrom-Json
Assert-True ($d.ativo -eq $false -and $d.limiar_minutos -eq 5) 'copia desliga automacao e preserva campos'
Assert-True ((Get-TreeHash $origem) -eq $antes) 'origem intacta'
Assert-True (@(Get-ChildItem (Join-Path $saida 'localappdata') -Force).Count -eq 0) 'localappdata nao copiado'

$r = Invoke-Script $script @('-Autorizado', '-ManterAutomacao', '-Origem', $origem, '-Saida', $saida)
$d = Get-Content (Join-Path $data 'disparo.json') -Raw | ConvertFrom-Json
Assert-True ($d.ativo -eq $true) 'ManterAutomacao preserva disparo'

$r = Invoke-Script $script @('-Autorizado', '-Origem', (Join-Path $base 'nao existe'), '-Saida', $saida)
Assert-True ($r.Code -eq 1 -and $r.Err -match 'nao existe') 'origem ausente sai com 1 e nomeia o caminho'

Remove-Item $base -Recurse -Force
Complete-Tests
```

- [ ] **Passo 2: rodar e ver falhar.**

- [ ] **Passo 3: implementar**

```powershell
<#
.SYNOPSIS
  Clean-machine scenario "copia": copy of the user's real app data. Reads the source only.
  Requires -Autorizado: ask the user for permission for this round before running it.
.PARAMETER Origem           Source data folder (default: %APPDATA%\io.github.johngabie.claudia-rh).
.PARAMETER Saida            Output folder (default: %TEMP%\claudia-sandbox\copia). Recreated on every run.
.PARAMETER ManterAutomacao  Keep disparo.json / modo_autonomo.json as copied (default: turned off in the copy).
.PARAMETER Sqlite           sqlite3.exe to use (default: the lab's, then PATH).
#>
param([switch]$Autorizado, [string]$Origem = '', [string]$Saida = '', [switch]$ManterAutomacao, [string]$Sqlite = '')
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'comum.ps1')
if (-not $Autorizado) {
  [Console]::Error.WriteLine("Scenario 'copia' reads the user's real data: ask the user for permission for this round, then run again with -Autorizado.")
  exit 2
}
try {
  if (-not $Origem) { $Origem = Join-Path $env:APPDATA $AppId }
  if (-not (Test-Path $Origem -PathType Container)) { throw "Source data not found: $Origem" }
  $exe = Get-SqliteExe $Sqlite
  $dir = New-CenarioDir $Saida 'copia'
  $data = Join-Path $dir "appdata\$AppId"
  robocopy $Origem $data /E /R:1 /W:1 /NFL /NDL /NP /NJH /NJS | Out-Null
  if ($LASTEXITCODE -ge 8) { throw "robocopy failed ($LASTEXITCODE): $Origem" }
  # Fold each copied WAL into its database so the copy is a single consistent file.
  foreach ($db in Get-ChildItem $data -Recurse -Filter *.db -File) {
    if (Test-Path "$($db.FullName)-wal") { Invoke-Sqlite $exe $db.FullName 'PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;' | Out-Null }
    Remove-Item "$($db.FullName)-wal", "$($db.FullName)-shm" -ErrorAction SilentlyContinue
  }
  if (-not $ManterAutomacao) { Set-AutomacaoDesligada $data }
  Write-Output (Write-Cenario $dir 'copia' $Origem)
  exit 0
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
```

- [ ] **Passo 4: rodar e ver passar.**
- [ ] **Passo 5: prova de mutação** — remover o bloco do `wal_checkpoint`; confirmar `FAILED copia consolida wal` ou `FAILED copia sem arquivo wal`; restaurar.
- [ ] **Passo 6: commit** — `git commit -m "feat(skill): authorized copy-of-user-data clean-machine scenario"`

---

### Tarefa 5: `publicar.ps1`

**Arquivos:**
- Criar: `.claude/skills/claudia-sandbox/scripts/publicar.ps1`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/publicar.test.ps1`

**Interfaces:**
- Consome: `comum.ps1` (`Get-LabDir`, `$AppId`); interface do laboratório `scripts\app-stage.ps1 -App -AppData -WorkDir -Clean`.
- Produz: `publicar.ps1 -App <exe|instalador|pasta> -Cenario <pasta> [-WorkDir <pasta>]` → exit 0 e JSON `{workDir, deploy, cenario}` na última linha.

- [ ] **Passo 1: teste com laboratório falso (falha: script inexistente)**

```powershell
. (Join-Path $PSScriptRoot 'assert.ps1')
$script = Join-Path $PSScriptRoot '..\publicar.ps1'
$novo = Join-Path $PSScriptRoot '..\cenario-novo.ps1'
$base = New-TempDir 'publicar'
$lab = Join-Path $base 'lab x'
New-Item -ItemType Directory -Force (Join-Path $lab 'scripts') | Out-Null
# Fake app-stage that records its parameters
@'
param([string]$App, [string[]]$AppData, [string[]]$LocalAppData, [string]$WorkDir, [switch]$Clean)
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'called.json'), (@{ App = $App; AppData = $AppData; WorkDir = $WorkDir; Clean = [bool]$Clean } | ConvertTo-Json))
exit 0
'@ | Set-Content (Join-Path $lab 'scripts\app-stage.ps1')
$app = Join-Path $base 'app x.exe'; Set-Content $app 'x'
$cen = Join-Path $base 'cen x'
Invoke-Script $novo @('-Saida', $cen) | Out-Null
$work = Join-Path $base 'work x'

$env:SANDBOX_LAB_DIR = $lab
$r = Invoke-Script $script @('-App', $app, '-Cenario', $cen, '-WorkDir', $work)
Assert-True ($r.Code -eq 0) "publicar exit 0 $($r.Err)"
$c = Get-Content (Join-Path $lab 'scripts\called.json') -Raw | ConvertFrom-Json
Assert-True ($c.App -eq $app -and $c.WorkDir -eq $work -and $c.Clean) 'publicar repassa App, WorkDir e Clean'
Assert-True (@($c.AppData)[0] -like '*appdata\io.github.johngabie.claudia-rh') 'publicar repassa a pasta de dados do app'

$r = Invoke-Script $script @('-App', $app, '-Cenario', $base, '-WorkDir', $work)
Assert-True ($r.Code -eq 1 -and $r.Err -match 'cenario\.json') 'publicar recusa pasta sem cenario.json'

$env:SANDBOX_LAB_DIR = $base
$r = Invoke-Script $script @('-App', $app, '-Cenario', $cen, '-WorkDir', $work)
Assert-True ($r.Code -eq 1 -and $r.Err -match 'app-stage\.ps1') 'publicar com lab incompleto nomeia o arquivo'

Remove-Item $base -Recurse -Force
Complete-Tests
```

- [ ] **Passo 2: rodar e ver falhar.**

- [ ] **Passo 3: implementar**

```powershell
<#
.SYNOPSIS
  Publishes the app plus a scenario folder to the Windows Sandbox lab (lab's scripts\app-stage.ps1).
.PARAMETER App      App exe, installer, or build folder.
.PARAMETER Cenario  Folder produced by one of the cenario-*.ps1 scripts.
.PARAMETER WorkDir  Lab work folder (default: $env:LAB_WORK_DIR, else %TEMP%\claudia-sandbox\work).
#>
param([Parameter(Mandatory)][string]$App, [Parameter(Mandatory)][string]$Cenario, [string]$WorkDir = '')
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'comum.ps1')
try {
  $lab = Get-LabDir
  $meta = Join-Path $Cenario 'cenario.json'
  if (-not (Test-Path $meta)) { throw "Not a scenario folder (cenario.json missing): $Cenario" }
  if (-not $WorkDir) { $WorkDir = $(if ($env:LAB_WORK_DIR) { $env:LAB_WORK_DIR } else { Join-Path $env:TEMP 'claudia-sandbox\work' }) }
  $stage = @{ App = $App; AppData = @(Join-Path $Cenario "appdata\$AppId"); WorkDir = $WorkDir; Clean = $true }
  & (Join-Path $lab 'scripts\app-stage.ps1') @stage | Out-Host
  if ($LASTEXITCODE) { throw "app-stage.ps1 failed ($LASTEXITCODE)" }
  $nome = (Get-Content $meta -Raw | ConvertFrom-Json).cenario
  Write-Output ([ordered]@{ workDir = $WorkDir; deploy = (Join-Path $WorkDir 'deploy'); cenario = $nome } | ConvertTo-Json -Compress)
  exit 0
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
```

- [ ] **Passo 4: rodar e ver passar.**
- [ ] **Passo 5: rodar a suíte inteira** — `pwsh -NoProfile -File .claude/skills/claudia-sandbox/scripts/test/run-all.ps1; echo "exit=$LASTEXITCODE"` → `ALL OK`, `exit=0`.
- [ ] **Passo 6: commit** — `git commit -m "feat(skill): publish app and scenario to the sandbox lab"`

---

### Tarefa 6: skill `claudia-sandbox` (via skill-creator)

Executor: agente que carrega a skill `skill-creator` e segue o processo dela (rascunho → evals de disparo → ajuste da descrição).

**Arquivos:**
- Criar: `.claude/skills/claudia-sandbox/SKILL.md`
- Criar: `.claude/skills/claudia-sandbox/references/limites.md`
- Criar: `.claude/skills/claudia-sandbox/references/pastas-de-dados.md`
- Criar: `.claude/skills/claudia-sandbox/evals/evals.json`

**Interfaces:**
- Consome: scripts das Tarefas 1–5 (nomes e parâmetros exatos acima); laboratório via `SANDBOX_LAB_DIR` (interface na spec, seção "Contexto: interface do laboratório"); runner de evals existente `node .claude/skills/claudia-teste/evals/rodar-evals.mjs` (ler o código para adaptar o caminho do `evals.json`, sem alterar o runner da `claudia-teste`, ou copiar o runner para `claudia-sandbox/evals/`).

- [ ] **Passo 1:** ler a spec, `CLAUDE.md`, `.claude/skills/claudia-teste/SKILL.md` (estilo e tamanho de referência) e `%SANDBOX_LAB_DIR%\.claude\skills\sandbox-lab\SKILL.md`.
- [ ] **Passo 2:** escrever `SKILL.md` em pt-BR com frontmatter `name: claudia-sandbox` e `description` ≤ 1.024 caracteres, cobrindo os 8 itens da spec ("Conteúdo da skill"). Corpo ≤ 120 linhas; detalhe vai para `references/`. A skill é um mapa: capacidades, cenários e quando usar cada um, regras; não prescreve roteiro de teste.
- [ ] **Passo 3:** `references/pastas-de-dados.md`: o que fica em `%APPDATA%\io.github.johngabie.claudia-rh` (YAMLs do perfil, `strategy.md`, `claudia_rh.db`, `disparo.json`, `modo_autonomo.json`, `notif.json`, `welcome_seen`, `diagnostics\`) e em `%LOCALAPPDATA%\io.github.johngabie.claudia-rh` (browser e Activity log, quando existirem na versão em teste). Conferir cada nome no código (`grep` em `src-tauri/src`) e marcar como "não verificado" o que não for encontrado.
- [ ] **Passo 4:** `references/limites.md`: comportamentos do laboratório que afetam o teste, cada um com o comando que o demonstra: winapp não alcança o guest com `networking: Disable`; app iniciado por `winapp target exec` termina quando o comando termina (iniciar com `wsb exec --id <id> -r ExistingLogin`); target do cargo em pasta mapeada falha com `os error 1392` (usar disco do guest); só um Sandbox por vez; `.exe` de `tauri build --no-bundle` não instala pré-requisitos (WebView2), o instalador sim.
- [ ] **Passo 5:** `evals/evals.json` no formato do `claudia-teste/evals/evals.json`: ≥ 8 prompts que devem disparar (máquina limpa, Sandbox, instalador, primeira abertura, desinstalação, PC novo, testar com meus dados sem mexer neles, build do zero) e ≥ 5 que não devem (olhar tela no `tauri dev`, `cargo test`, registrar no Linear, mexer no CSS, bug na tela de Perfil).
- [ ] **Passo 6:** rodar os evals de disparo (Sonnet, 2 rodadas); registrar resultado em `references/limites.md` (seção "Evals de disparo"), com a mesma regra da `claudia-teste`: execução inválida não conta. Ajustar a descrição até disparar em todos os "deve" sem disparar nos "não deve".
- [ ] **Passo 7:** varredura de agnosticismo nos arquivos da tarefa (ver Tarefa 10, passo 2) e commit — `git commit -m "feat(skill): claudia-sandbox skill for clean-machine testing"`

---

### Tarefa 7: ajuste da `claudia-teste`

**Arquivos:**
- Modificar: `.claude/skills/claudia-teste/SKILL.md` (frontmatter `description` e seção "Mais detalhe")
- Modificar: `.claude/skills/claudia-teste/evals/evals.json`

- [ ] **Passo 1:** na `description`, acrescentar ao trecho "NÃO use para …": `teste em Windows limpo ou Sandbox (use claudia-sandbox)`. Manter ≤ 1.024 caracteres (medir com `(Get-Content SKILL.md -Raw)` e contar o bloco).
- [ ] **Passo 2:** em "Mais detalhe", acrescentar: `- Teste em Windows limpo (Sandbox): skill claudia-sandbox`.
- [ ] **Passo 3:** em `evals.json`, acrescentar 3 casos "não deve disparar": "testa o instalador num Windows limpo", "roda o app no Sandbox com dados mockados", "funciona num PC novo sem nada instalado?".
- [ ] **Passo 4:** rodar `node .claude/skills/claudia-teste/evals/rodar-evals.mjs sonnet` (2 rodadas). Esperado: todos os casos antigos com o mesmo resultado de antes e os 3 novos sem disparar. Se algum antigo piorar, reverter a frase e reportar.
- [ ] **Passo 5:** commit — `git commit -m "docs(skill): point clean-machine testing to claudia-sandbox"`

---

### Tarefa 8: `CLAUDE.md`

**Arquivos:**
- Modificar: `CLAUDE.md` (nova seção logo depois de "## Build")

- [ ] **Passo 1:** inserir:

```markdown
## Teste em máquina limpa

Para testar o app num Windows limpo e isolado (instalador, primeira abertura, desinstalação, comportamento sem nada pré-instalado, ou com cópia dos dados do usuário sem tocar neles), use a skill `claudia-sandbox`. Ela usa um laboratório Windows Sandbox mantido fora do repositório, localizado pela variável `SANDBOX_LAB_DIR`. Cenários de dados: `novo`, `mock` e `copia` (este só com autorização do usuário na rodada). Falha do app num Windows limpo é resultado do teste: registre no Linear, não contorne no laboratório.
```

- [ ] **Passo 2:** conferir que nenhuma outra seção mudou (`git diff CLAUDE.md` mostra só a inserção).
- [ ] **Passo 3:** commit — `git commit -m "docs: document clean-machine testing entry point"`

---

### Tarefa 9: smoke no Sandbox

Executor: agente Opus. Pré-requisitos: `SANDBOX_LAB_DIR` definido; nenhum Sandbox rodando (`wsb list` vazio).

- [ ] **Passo 1:** build de release sem bundle com target fora do repo: `$env:CARGO_TARGET_DIR="$env:TEMP\claudia-sandbox\target"; npm ci; npx tauri build --no-bundle`. Registrar tempo e caminho do exe.
- [ ] **Passo 2:** para cada cenário em `novo`, `mock`:
  1. `pwsh -File .claude/skills/claudia-sandbox/scripts/cenario-<c>.ps1`
  2. `pwsh -File .claude/skills/claudia-sandbox/scripts/publicar.ps1 -App <exe> -Cenario %TEMP%\claudia-sandbox\<c>`
  3. `$env:LAB_APP_PROJECT=<raiz do repo>; $env:LAB_WORK_DIR=<workDir do passo 2>; & "$env:SANDBOX_LAB_DIR\scripts\test-up.ps1" -Config lab.config.app-winapp.json`
  4. `wsb exec --id <id> -r System -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-firewall.ps1"`
  5. `wsb exec --id <id> -r ExistingLogin -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1"`
  6. `winapp ui list-windows --on sandbox` e `winapp target screenshot sandbox -o <evidencias>\<c>.png`; ler o PNG.
  7. `& "$env:SANDBOX_LAB_DIR\scripts\test-down.ps1"`
- [ ] **Passo 3:** relatório (fora do repositório, em `%TEMP%\claudia-sandbox\evidencias\relatorio.md`): por cenário, comandos, exit codes, o que a tela mostrou. Falha do app é registrada como achado (nome, passo, print) para virar task no Linear; não é contornada.
- [ ] **Passo 4:** cenário `copia` **só** se o usuário autorizar nesta rodada; sem autorização, registrar "não verificado: aguarda autorização".

---

### Tarefa 10: revisão e varredura

Executor: agente Opus, independente dos implementadores.

- [ ] **Passo 1:** revisão de código da branch inteira contra a spec (`requesting-code-review`): contrato dos cenários, exit codes, origem só lida, `disparo.json` completo, nenhuma escrita fora de `-Saida`/`-WorkDir`.
- [ ] **Passo 2:** varredura de agnosticismo em `git diff dev --name-only`: procurar referências a sessões de trabalho, pessoas, modelos de IA usados na construção, nomes de máquina, IPs, usuários do Windows, caminhos absolutos pessoais ou pastas externas além das variáveis permitidas nas restrições globais. Cada ocorrência: arquivo:linha e correção.
- [ ] **Passo 3:** rodar `run-all.ps1` de novo e conferir `exit=0`.

---

### Tarefa 11: integração (decisão do usuário)

- [ ] **Passo 1:** apresentar ao usuário o resumo: commits, resultado dos testes, evals, smoke e revisão. Aguardar a decisão.
- [ ] **Passo 2 (com aprovação):** `git checkout dev && git merge --no-ff feat/clean-machine-testing`.
- [ ] **Passo 3 (com aprovação):** `git checkout feat/browser-cdp-mcp && git rebase dev`. Conflito esperado em `CLAUDE.md` (a branch reescreve o arquivo): manter o conteúdo da branch e reinserir a seção "Teste em máquina limpa" depois de "## Build"; em `claudia-teste/SKILL.md`, manter as duas mudanças. Depois do rebase, rodar as rodadas de teste da branch e conferir exit codes.
