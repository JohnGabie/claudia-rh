# Teste em máquina limpa e observação do app — plano de implementação

> **Para agentes executores:** SUB-SKILL OBRIGATÓRIA: use superpowers:subagent-driven-development para executar este plano tarefa a tarefa. Os passos usam checkbox (`- [ ]`).

**Objetivo:** duas skills especializadas — `claudia-sandbox` (nova: Windows Sandbox limpo, cenários de dados, publicar/instalar, CDP no guest) e `claudia-app` (a `claudia-teste` renomeada: observar o app por CDP localmente ou no Sandbox, com prints em escada).

**Arquitetura:** o específico do ambiente vive em `.claude/skills/claudia-sandbox/` (scripts PowerShell, fixtures, referências, evals); a observação do app continua nos scripts Node da skill renomeada `.claude/skills/claudia-app/`, que passam a aceitar `CDP_HOST` e `CLAUDIA_DB`. O laboratório é localizado por `SANDBOX_LAB_DIR` e usado só pela sua interface pública. Cada cenário produz uma pasta com `appdata\io.github.johngabie.claudia-rh\` e `cenario.json`; `publicar.ps1` entrega app + cenário ao laboratório; `cdp-guest.ps1` expõe o CDP do app no guest ao host.

**Stack:** PowerShell 7 (compatível com Windows PowerShell 5.1), Node ≥ 22 (`node:test`, `node:sqlite`), sqlite3 do laboratório, winapp CLI, skill-creator.

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
2. `disparo.json` existente com campos extras ou faltando `limiar_minutos`: o resultado precisa ser JSON completo com `ativo: false`. Teste na Tarefa 1 (`automacao completa a partir de arquivo parcial`).
3. `CDP_HOST` remoto sem resposta: `regras.decidirSubida` nunca devolve `subir` com `remoto: true` (logo `subir.mjs` não sobe `tauri dev` no host); aborta com motivo. Teste na Tarefa 7 (`remoto sem CDP aborta`).
4. `-Saida` já existente com lixo de execução anterior: a pasta é recriada, sem sobras. Teste na Tarefa 1 (`saida recriada`).
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
| `.claude/skills/claudia-sandbox/scripts/cdp-guest.ps1`, `banco-guest.ps1` | CDP do app no guest exposto ao host; snapshot do banco do guest |
| `.claude/skills/claudia-app/` (renomeada de `claudia-teste`) | `SKILL.md`, `evals/`, `scripts/regras.mjs`, `cdp.mjs`, `subir.mjs`, `ver.mjs`, `invocar.mjs`, `observar.mjs`, `banco.mjs` com `CDP_HOST`/`CLAUDIA_DB` |
| `.claude/skills/claudia-app/scripts/print-janela.ps1` (+ teste) | print da janela nativa ou da tela inteira, sem CDP |
| `CLAUDE.md` | nome novo da skill + seção "Teste em máquina limpa" |

## Ondas de execução

- **Onda 1:** Tarefa 1.
- **Onda 2 (paralelo, arquivos distintos):** Tarefas 2, 3, 4, 7, 10.
- **Onda 3 (paralelo):** Tarefa 5 (o teste usa `cenario-novo.ps1` da Tarefa 2), Tarefa 8 (prints, depois da 7) e Tarefa 9 (CDP/banco no guest).
- **Onda 4:** Tarefa 6 (skill `claudia-sandbox`; documenta os scripts prontos).
- **Onda 5:** Tarefa 11 (smoke) e Tarefa 12 (revisão + varredura).
- **Fim:** integração decidida pelo usuário (Tarefa 13).

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
  # [string] turns $null into '': map it back so scenarios without a source write "origem": null.
  $o = [ordered]@{ cenario = $Nome; criadoEm = (Get-Date).ToString('o'); origem = $(if ($Origem) { $Origem } else { $null }); arquivos = $files }
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
Assert-True ($null -eq $j.origem) 'novo com origem null'
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
- Criar: `.claude/skills/claudia-sandbox/evals/evals.json` (e runner, ver Passo 6)

**Interfaces:**
- Consome: scripts das Tarefas 1–5 e 9 (nomes e parâmetros exatos neste plano); skill `claudia-app` (Tarefas 7–8) para observar e tirar prints; laboratório via `SANDBOX_LAB_DIR` (interface na spec, seção "Contexto: interface do laboratório").

- [ ] **Passo 1:** ler a spec, `CLAUDE.md`, `.claude/skills/claudia-app/SKILL.md` (estilo e tamanho de referência) e `%SANDBOX_LAB_DIR%\.claude\skills\sandbox-lab\SKILL.md`.
- [ ] **Passo 2:** escrever `SKILL.md` em pt-BR com frontmatter `name: claudia-sandbox` e `description` ≤ 1.024 caracteres, cobrindo os 8 itens da spec ("Conteúdo da `claudia-sandbox`"). Corpo ≤ 120 linhas; detalhe vai para `references/`. A skill é um mapa: capacidades, cenários e quando usar cada um, regras; não prescreve roteiro de teste. Fluxo típico descrito como exemplo, não como obrigação: cenário → `publicar.ps1` → `test-up.ps1 -Config lab.config.app-winapp.json` → instalar ou `guest-run.ps1` → `cdp-guest.ps1` → observar com `claudia-app` (`CDP_HOST`/`CDP_PORT`, `CLAUDIA_DB` de `banco-guest.ps1`) → prints → `test-down.ps1`.
- [ ] **Passo 3:** `references/pastas-de-dados.md`: o que fica em `%APPDATA%\io.github.johngabie.claudia-rh` (YAMLs do perfil, `strategy.md`, `claudia_rh.db`, `disparo.json`, `modo_autonomo.json`, `notif.json`, `welcome_seen`, `diagnostics\`) e em `%LOCALAPPDATA%\io.github.johngabie.claudia-rh` (browser e Activity log, quando existirem na versão em teste). Conferir cada nome no código (`grep` em `src-tauri/src`) e marcar como "não verificado" o que não for encontrado.
- [ ] **Passo 4:** `references/limites.md`: comportamentos do laboratório que afetam o teste, cada um com o comando que o demonstra (se o comando não reproduzir o comportamento, marcar como "não verificado"): winapp não alcança o guest com `networking: Disable`; app iniciado por `winapp target exec` termina quando o comando termina (iniciar com `wsb exec --id <id> -r ExistingLogin`); `wsb exec` não devolve a saída do processo; target do cargo em pasta mapeada falha com `os error 1392` (usar disco do guest); só um Sandbox por vez; `.exe` de `tauri build --no-bundle` não instala pré-requisitos (WebView2), o instalador sim; o WebView2 só aceita CDP em `127.0.0.1` do guest (daí o `portproxy` do `cdp-guest.ps1`).
- [ ] **Passo 5:** `evals/evals.json` no formato de `.claude/skills/claudia-app/evals/evals.json`: ≥ 8 prompts que devem disparar (máquina limpa, Sandbox, instalador, primeira abertura, desinstalação, PC novo, testar com meus dados sem mexer neles, build do zero) e ≥ 5 que não devem (olhar a tela do app já aberto, tirar print do Perfil, `cargo test`, registrar no Linear, mexer no CSS).
- [ ] **Passo 6:** copiar `.claude/skills/claudia-app/evals/rodar-evals.mjs` para `.claude/skills/claudia-sandbox/evals/rodar-evals.mjs` trocando só a constante `NOME` para `"claudia-sandbox"`; rodar `node .claude/skills/claudia-sandbox/evals/rodar-evals.mjs sonnet` (2 rodadas); registrar resultado em `references/limites.md` (seção "Evals de disparo"): execução inválida não conta. Ajustar a descrição até disparar em todos os "deve" sem disparar nos "não deve".
- [ ] **Passo 7:** varredura de agnosticismo nos arquivos da tarefa (Tarefa 12, Passo 2) e commit — `git commit -m "feat(skill): claudia-sandbox skill for clean-machine testing"`

---

### Tarefa 7: renomear `claudia-teste` → `claudia-app` e alvo remoto

**Arquivos:**
- Mover: `.claude/skills/claudia-teste/` → `.claude/skills/claudia-app/`
- Modificar: `claudia-app/SKILL.md`, `evals/rodar-evals.mjs`, `evals/evals.json`, `scripts/regras.mjs`, `scripts/cdp.mjs`, `scripts/subir.mjs`, `scripts/ver.mjs`, `scripts/banco.mjs`, `scripts/observar.mjs`
- Conferir: `scripts/invocar.mjs` (e `observar.mjs`) falam CDP só por `alvos()`/`ligar()` de `cdp.mjs` e herdam `HOST`; `grep -n "127.0.0.1" .claude/skills/claudia-app/scripts/*.mjs` no fim do Passo 4 só pode mostrar o teste de `:1420` em `subir.mjs`
- Modificar: `scripts/test/regras.test.mjs`
- Criar: `scripts/test/banco.test.mjs`

**Interfaces:**
- Produz: env `CDP_HOST` (padrão `127.0.0.1`), `CDP_PORT` (padrão 9333), `CLAUDIA_DB` (padrão `%APPDATA%\io.github.johngabie.claudia-rh\claudia_rh.db`); `cdp.mjs` exporta `HOST`; `regras.mjs` exporta `hostRemoto(host) -> boolean` e `decidirSubida({ appRodando, appTemCdp, remoto }) -> "subir"|"reutilizar"|"perguntar"|"abortar"`.

- [ ] **Passo 0 (referência dos evals, antes de mudar qualquer coisa):** `node .claude/skills/claudia-teste/evals/rodar-evals.mjs sonnet` (2 rodadas); guardar o resultado por caso.
- [ ] **Passo 1: renomear**

```bash
git mv .claude/skills/claudia-teste .claude/skills/claudia-app
grep -rn "claudia-teste" .claude/skills/claudia-app
```
Trocar cada ocorrência por `claudia-app` (frontmatter `name:`, constante `NOME` e comentários de `evals/rodar-evals.mjs`, comentário de cabeçalho de `scripts/regras.mjs`, texto da `SKILL.md` e de `references/`). Em `references/limites.md`, as medições antigas continuam válidas: só o nome muda.

- [ ] **Passo 2: testes que falham** — acrescentar a `scripts/test/regras.test.mjs` (e `hostRemoto` no import):

```js
test("aceita a página do build de release", () => {
  assert.equal(validarVersaoApp(versaoApp, [{ type: "page", url: "http://tauri.localhost/" }]).ok, true);
});

test("remoto sem CDP aborta, nunca sobe tauri dev", () => {
  assert.equal(decidirSubida({ appRodando: false, appTemCdp: false, remoto: true }), "abortar");
});

test("remoto com CDP reutiliza", () => {
  assert.equal(decidirSubida({ appRodando: true, appTemCdp: true, remoto: true }), "reutilizar");
});

test("hostRemoto distingue local de remoto", () => {
  assert.equal(hostRemoto(undefined), false);
  assert.equal(hostRemoto("127.0.0.1"), false);
  assert.equal(hostRemoto("localhost"), false);
  assert.equal(hostRemoto("172.20.0.5"), true);
});
```

Criar `scripts/test/banco.test.mjs`:

```js
import test from "node:test";
import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

test("banco.mjs lê o banco apontado por CLAUDIA_DB", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "claudia app "));
  const db = path.join(dir, "x.db");
  const d = new DatabaseSync(db); d.exec("CREATE TABLE t(v); INSERT INTO t VALUES (7);"); d.close();
  const out = execFileSync(process.execPath, [path.join(import.meta.dirname, "..", "banco.mjs"), "SELECT v FROM t"],
    { env: { ...process.env, CLAUDIA_DB: db } }).toString();
  assert.equal(JSON.parse(out)[0].v, 7);
  fs.rmSync(dir, { recursive: true, force: true });
});
```

- [ ] **Passo 3: rodar e ver falhar** — `node --test .claude/skills/claudia-app/scripts/test/` → falhas nos testes novos.

- [ ] **Passo 4: implementar**

`scripts/regras.mjs`:

```js
const PAGINA_APP = /localhost:1420|tauri\.localhost/;

/** The app's WebView2 reports an Edge build and serves the page on :1420 (tauri dev) or tauri.localhost (release). */
export function validarVersaoApp(versao, paginas) {
  if (!versao || typeof versao.Browser !== "string") {
    return { ok: false, motivo: "sem campo Browser: a porta não responde como CDP" };
  }
  if (!/^Edg\//.test(versao.Browser)) {
    return { ok: false, motivo: `Browser é ${versao.Browser}, esperado Edg/ (WebView2). A porta é de outro processo` };
  }
  const temApp = Array.isArray(paginas) && paginas.some((p) => p.type === "page" && PAGINA_APP.test(p.url || ""));
  if (!temApp) {
    return { ok: false, motivo: "Edg encontrado, mas sem página do app (localhost:1420 ou tauri.localhost)" };
  }
  return { ok: true, motivo: "porta do app confirmada" };
}

/** A CDP host other than this machine means the app runs elsewhere (e.g. a sandbox guest). */
export function hostRemoto(host) {
  return !!host && !/^(127\.0\.0\.1|localhost|::1)$/i.test(host);
}

/** Never restart the user's app on our own: running without CDP means ask. Remote targets are never started from here. */
export function decidirSubida({ appRodando, appTemCdp, remoto = false }) {
  if (remoto) return appTemCdp ? "reutilizar" : "abortar";
  if (!appRodando) return "subir";
  return appTemCdp ? "reutilizar" : "perguntar";
}
```

`scripts/cdp.mjs`: acrescentar `export const HOST = process.env.CDP_HOST || "127.0.0.1";` e trocar `127.0.0.1` por `${HOST}` em `alvos` e `novaAba`.

`scripts/ver.mjs`: importar `HOST` de `./cdp.mjs` e trocar `127.0.0.1` por `${HOST}` na linha do `/json/activate/`.

`scripts/subir.mjs`: importar `hostRemoto` de `./regras.mjs` e `HOST` de `./cdp.mjs`; trocar todo `127.0.0.1` dos `fetch` de CDP por `${HOST}` (o teste de `:1420` continua em `127.0.0.1`); logo depois dos imports, inserir:

```js
if (hostRemoto(HOST)) {
  const porta = Number(process.env.CDP_PORT ?? 9333);
  const v = await ler(`http://${HOST}:${porta}/json/version`);
  const ok = v && validarVersaoApp(v, await ler(`http://${HOST}:${porta}/json/list`)).ok;
  const decisao = decidirSubida({ appRodando: ok, appTemCdp: ok, remoto: true });
  if (decisao === "reutilizar") { console.log(JSON.stringify({ acao: "reutilizar", host: HOST, porta })); process.exit(0); }
  console.log(JSON.stringify({ acao: "abortar", motivo: `sem CDP do app em ${HOST}:${porta}; no Sandbox, inicie o app com claudia-sandbox/scripts/cdp-guest.ps1 -Iniciar` }));
  process.exit(3);
}
```
(mover a função `ler` para antes desse bloco, se ela estiver declarada depois.)

`scripts/banco.mjs` e `scripts/observar.mjs`: `const dados = process.env.CLAUDIA_DB || path.join(process.env.APPDATA, "io.github.johngabie.claudia-rh", "claudia_rh.db");`

- [ ] **Passo 5: rodar e ver passar** — `node --test .claude/skills/claudia-app/scripts/test/; echo "exit=$LASTEXITCODE"` → `exit=0`.
- [ ] **Passo 6: prova de mutação** — em `decidirSubida`, trocar `"abortar"` por `"subir"`; confirmar `FAILED`/`not ok` em `remoto sem CDP aborta, nunca sobe tauri dev`; restaurar.
- [ ] **Passo 7: `SKILL.md`** — `description` (≤ 1.024 caracteres) passa a dizer que a skill olha o app rodando **localmente ou no Sandbox** (com `CDP_HOST`), mantém todos os gatilhos atuais (telas, layout, UI vs banco, agentes, comandos Tauri, QUALQUER bug → reproduzir primeiro) e acrescenta "NÃO use para montar Windows limpo, escolher dados ou instalar o app no Sandbox (claudia-sandbox)". No corpo, nova seção "Alvo" com a tabela:

| Alvo | Como |
|---|---|
| Local (`tauri dev` no host) | padrão; `node scripts/subir.mjs` |
| Sandbox | `CDP_HOST`/`CDP_PORT` impressos por `claudia-sandbox/scripts/cdp-guest.ps1`; banco: `CLAUDIA_DB` impresso por `claudia-sandbox/scripts/banco-guest.ps1` (snapshot; repetir após cada ação) |

- [ ] **Passo 8: evals** — em `evals/evals.json`, acrescentar "deve disparar": "tira um print da tela de Perfil", "olha o app que tá rodando no Sandbox"; "não deve disparar": "monta um Windows limpo com dados mockados", "testa o instalador num PC novo", "prepara o Sandbox com cópia dos meus dados". Rodar `node .claude/skills/claudia-app/evals/rodar-evals.mjs sonnet` (2 rodadas). Esperado: casos antigos repetem a referência do Passo 0; casos novos conforme o esperado. Se algum antigo piorar, ajustar a descrição e repetir; se não convergir, reportar.
- [ ] **Passo 9: commit** — `git commit -m "refactor(skill): rename claudia-teste to claudia-app and support a remote CDP target"`

---

### Tarefa 8: prints em escada (`claudia-app`)

**Arquivos:**
- Criar: `.claude/skills/claudia-app/scripts/print-janela.ps1`
- Criar: `.claude/skills/claudia-app/scripts/test/print-janela.test.ps1`
- Modificar: `.claude/skills/claudia-app/SKILL.md` (seção "Prints")

**Interfaces:**
- Produz: `print-janela.ps1 [-Processo claudia-rh] [-TelaInteira] [-Saida <png>]` → exit 0 e JSON `{arquivo, largura, altura}`; exit 1 com motivo (sem janela visível, janela minimizada). Padrão de `-Saida`: `debug-logs\prints\<yyyyMMdd-HHmmss>.png` na raiz do repositório.

- [ ] **Passo 1: teste (falha: script inexistente)**

```powershell
$script = Join-Path $PSScriptRoot '..\print-janela.ps1'
$fail = 0
function Check([bool]$c, [string]$n) { if ($c) { "PASS $n" } else { "FAILED $n"; $script:fail++ } }
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("print test " + [guid]::NewGuid().ToString('N').Substring(0, 8) + '.png')

$out = & pwsh -NoProfile -File $script -TelaInteira -Saida $tmp
Check ($LASTEXITCODE -eq 0) 'tela inteira exit 0'
$j = $out[-1] | ConvertFrom-Json
$bytes = [IO.File]::ReadAllBytes($tmp)
Check ($bytes[0] -eq 0x89 -and $bytes[1] -eq 0x50 -and $bytes[2] -eq 0x4E -and $bytes[3] -eq 0x47) 'arquivo e PNG'
Check ($j.largura -gt 0 -and $j.altura -gt 0) 'dimensoes positivas'
Remove-Item $tmp

$errFile = [IO.Path]::GetTempFileName()
& pwsh -NoProfile -File $script -Processo 'processo-que-nao-existe-xyz' -Saida $tmp 2>$errFile | Out-Null
Check ($LASTEXITCODE -eq 1 -and (Get-Content $errFile -Raw) -match 'TelaInteira') 'sem janela sai com 1 e sugere -TelaInteira'
Remove-Item $errFile

if ($fail) { exit 1 } else { 'OK'; exit 0 }
```

- [ ] **Passo 2: rodar e ver falhar.**
- [ ] **Passo 3: implementar**

```powershell
<#
.SYNOPSIS
  Read-only screenshot of the app's main window, or of the whole screen, without CDP.
  Captures what is on screen: a window covered by another shows the one on top.
.PARAMETER Processo     Process name (default: claudia-rh).
.PARAMETER TelaInteira  Capture the whole virtual screen (use it for native dialogs or when the window is not found).
.PARAMETER Saida        PNG path (default: debug-logs\prints\<timestamp>.png under the repo root).
#>
param([string]$Processo = 'claudia-rh', [switch]$TelaInteira, [string]$Saida = '')
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type -Namespace W -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")] public static extern bool GetWindowRect(System.IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool IsIconic(System.IntPtr h);
public struct RECT { public int Left, Top, Right, Bottom; }
'@
[void][W.U]::SetProcessDPIAware()
try {
  if (-not $Saida) {
    $root = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path
    $Saida = Join-Path $root ('debug-logs\prints\' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.png')
  }
  New-Item -ItemType Directory -Force (Split-Path $Saida) | Out-Null
  if ($TelaInteira) {
    $b = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $x = $b.Left; $y = $b.Top; $w = $b.Width; $h = $b.Height
  } else {
    $p = Get-Process -Name $Processo -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    if (-not $p) { throw "No visible window for process '$Processo'. Use -TelaInteira." }
    if ([W.U]::IsIconic($p.MainWindowHandle)) { throw "Window of '$Processo' is minimized: restore it or use -TelaInteira." }
    $r = New-Object W.U+RECT
    [void][W.U]::GetWindowRect($p.MainWindowHandle, [ref]$r)
    $x = $r.Left; $y = $r.Top; $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
  }
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($x, $y, 0, 0, $bmp.Size)
  $bmp.Save($Saida, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  Write-Output ([ordered]@{ arquivo = $Saida; largura = $w; altura = $h } | ConvertTo-Json -Compress)
  exit 0
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
```

- [ ] **Passo 4: rodar e ver passar** — `pwsh -NoProfile -File .claude/skills/claudia-app/scripts/test/print-janela.test.ps1; echo "exit=$LASTEXITCODE"` → `exit=0`. Abrir o PNG gerado num passo manual (Read) para confirmar que mostra a tela.
- [ ] **Passo 5: prova de mutação** — remover o `throw` de "No visible window"; confirmar `FAILED sem janela sai com 1 e sugere -TelaInteira`; restaurar.
- [ ] **Passo 6: `SKILL.md`, seção "Prints"** — a tabela de escada da spec (seção "Prints (`claudia-app`)"), a regra "toda afirmação sobre a UI vem com print; se nenhum degrau funcionar, dizer quais falharam e por quê" e o aviso de dados pessoais. Pasta padrão: `debug-logs/prints/` (já ignorada pelo git).
- [ ] **Passo 7: commit** — `git commit -m "feat(skill): screenshot ladder with a native-window fallback"`

---

### Tarefa 9: CDP e banco do guest (`claudia-sandbox`)

**Arquivos:**
- Modificar: `.claude/skills/claudia-sandbox/scripts/comum.ps1` (funções de Sandbox)
- Criar: `.claude/skills/claudia-sandbox/scripts/cdp-guest.ps1`
- Criar: `.claude/skills/claudia-sandbox/scripts/banco-guest.ps1`
- Criar: `.claude/skills/claudia-sandbox/scripts/test/sandbox-id.test.ps1`

**Interfaces:**
- Consome: `comum.ps1`; laboratório (`C:\labsandbox\app\guest-run.ps1`, `C:\lab\tools\sqlite\sqlite3.exe`, `C:\labwork`); `wsb`.
- Produz em `comum.ps1`: `Get-IdsFromWsbList([string[]]$Lines) -> string[]`, `Get-SandboxId([string]$Id) -> string`, `Get-SandboxIp([string]$Id) -> string`, `Invoke-Guest([string]$Id, [ValidateSet('System','ExistingLogin')][string]$Como, [string]$PowerShell)` (envia com `-EncodedCommand`, sem problemas de aspas).
- Produz: `cdp-guest.ps1 [-Id] [-Porta 9333] [-PortaHost 19333] [-Iniciar] [-ExePath <exe no guest>] [-TimeoutSec 90]` → JSON `{host, porta, browser}`; `banco-guest.ps1 [-Id] [-WorkDir]` → JSON `{db}` (caminho no host).

- [ ] **Passo 1: teste do parser (falha)**

```powershell
. (Join-Path $PSScriptRoot 'assert.ps1')
. (Join-Path $PSScriptRoot '..\comum.ps1')
$linhas = @('Id                                   Status   Uptime', '6f446955-fba4-455f-af60-4da1c64f8b3b Running  00:03:10')
Assert-True ((Get-IdsFromWsbList $linhas) -join ',' -eq '6f446955-fba4-455f-af60-4da1c64f8b3b') 'extrai um id'
Assert-True (@(Get-IdsFromWsbList @('Nenhum ambiente')).Count -eq 0) 'sem sandbox, nenhum id'
Complete-Tests
```

- [ ] **Passo 2: rodar e ver falhar.**
- [ ] **Passo 3: implementar em `comum.ps1`**

```powershell
function Get-IdsFromWsbList([string[]]$Lines) {
  return @($Lines | Select-String -Pattern '[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}' -AllMatches |
    ForEach-Object { $_.Matches.Value } | Select-Object -Unique)
}

function Get-SandboxId([string]$Id) {
  if ($Id) { return $Id }
  $ids = Get-IdsFromWsbList @(wsb list 2>$null)
  if ($ids.Count -ne 1) { throw "Expected exactly one running sandbox, found $($ids.Count). Start one with the lab's test-up.ps1 or pass -Id." }
  return $ids[0]
}

function Get-SandboxIp([string]$Id) {
  $ip = ((wsb ip --id $Id 2>$null) | Select-String -Pattern '\d{1,3}(\.\d{1,3}){3}' | Select-Object -First 1).Matches.Value
  if (-not $ip) { throw "Could not read the sandbox IP (wsb ip --id $Id). The profile needs networking enabled." }
  return $ip
}

function Invoke-Guest([string]$Id, [ValidateSet('System', 'ExistingLogin')][string]$Como, [string]$PowerShell) {
  $enc = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($PowerShell))
  wsb exec --id $Id -r $Como -c "powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand $enc" | Out-Null
}
```

- [ ] **Passo 4: `cdp-guest.ps1`**

```powershell
<#
.SYNOPSIS
  Exposes the app's WebView2 DevTools port (inside the sandbox guest) to the host, optionally starting the app with CDP on.
  Prints {host, porta, browser}: use them as CDP_HOST / CDP_PORT with the claudia-app scripts.
.PARAMETER Iniciar   Start the app with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<Porta>.
.PARAMETER ExePath   App exe inside the guest (e.g. the installed copy). Without it, the lab's guest-run.ps1 starts the published exe.
#>
param([string]$Id = '', [int]$Porta = 9333, [int]$PortaHost = 19333, [switch]$Iniciar, [string]$ExePath = '', [int]$TimeoutSec = 90)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'comum.ps1')
try {
  $Id = Get-SandboxId $Id
  $ip = Get-SandboxIp $Id
  Invoke-Guest $Id System @"
netsh interface portproxy delete v4tov4 listenaddress=0.0.0.0 listenport=$PortaHost | Out-Null
netsh interface portproxy add v4tov4 listenaddress=0.0.0.0 listenport=$PortaHost connectaddress=127.0.0.1 connectport=$Porta | Out-Null
Remove-NetFirewallRule -DisplayName 'claudia-cdp' -ErrorAction SilentlyContinue
New-NetFirewallRule -DisplayName 'claudia-cdp' -Direction Inbound -Action Allow -Protocol TCP -LocalPort $PortaHost | Out-Null
"@
  if ($Iniciar) {
    $start = $(if ($ExePath) { "Start-Process '$ExePath'" } else { '& C:\labsandbox\app\guest-run.ps1' })
    Invoke-Guest $Id ExistingLogin "`$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=$Porta'; $start"
  }
  $url = "http://${ip}:$PortaHost/json/version"
  $v = $null; $deadline = (Get-Date).AddSeconds($TimeoutSec)
  while (-not $v -and (Get-Date) -lt $deadline) { try { $v = Invoke-RestMethod $url -TimeoutSec 3 } catch { Start-Sleep -Seconds 2 } }
  if (-not $v) { throw "CDP did not answer at $url within ${TimeoutSec}s. Check that WebView2 exists in the guest and the app is running (claudia-app print ladder shows why)." }
  Write-Output ([ordered]@{ host = $ip; porta = $PortaHost; browser = $v.Browser } | ConvertTo-Json -Compress)
  exit 0
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
```

- [ ] **Passo 5: `banco-guest.ps1`**

```powershell
<#
.SYNOPSIS
  Copies a consistent snapshot of the app database from the sandbox guest to the host.
  Prints {db}: use it as CLAUDIA_DB with the claudia-app scripts. Repeat after each action that changes data.
.PARAMETER WorkDir  Lab work folder used by publicar.ps1 (default: $env:LAB_WORK_DIR).
#>
param([string]$Id = '', [string]$WorkDir = '', [int]$TimeoutSec = 30)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'comum.ps1')
try {
  $Id = Get-SandboxId $Id
  if (-not $WorkDir) { $WorkDir = $env:LAB_WORK_DIR }
  if (-not $WorkDir) { throw 'LAB_WORK_DIR is not set: pass -WorkDir (the folder used by publicar.ps1).' }
  $hostDb = Join-Path $WorkDir 'guest-rw\db\claudia_rh.db'
  Remove-Item $hostDb -ErrorAction SilentlyContinue
  Invoke-Guest $Id ExistingLogin @"
New-Item -ItemType Directory -Force C:\labwork\db | Out-Null
& C:\lab\tools\sqlite\sqlite3.exe (Join-Path `$env:APPDATA '$AppId\claudia_rh.db') '.backup C:/labwork/db/claudia_rh.db'
"@
  $deadline = (Get-Date).AddSeconds($TimeoutSec)
  while (-not (Test-Path $hostDb) -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 1 }
  if (-not (Test-Path $hostDb)) { throw "Snapshot not found at $hostDb after ${TimeoutSec}s (does the app database exist in the guest yet?)." }
  Write-Output ([ordered]@{ db = $hostDb } | ConvertTo-Json -Compress)
  exit 0
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
```

- [ ] **Passo 6: rodar a suíte** — `pwsh -NoProfile -File .claude/skills/claudia-sandbox/scripts/test/run-all.ps1` → `ALL OK`. `cdp-guest.ps1` e `banco-guest.ps1` são verificados no smoke (Tarefa 11), porque dependem de um Sandbox rodando.
- [ ] **Passo 7: commit** — `git commit -m "feat(skill): expose the guest app's CDP and database to the host"`

---

### Tarefa 10: `CLAUDE.md`

**Arquivos:**
- Modificar: `CLAUDE.md`

- [ ] **Passo 1:** na seção "## Superpowers", trocar `claudia-teste` por `claudia-app`.
- [ ] **Passo 2:** inserir logo depois de "## Build":

```markdown
## Teste em máquina limpa

| Quero… | Skill |
|---|---|
| ver, clicar ou conferir o app rodando, tirar print, reproduzir bug | `claudia-app` (local ou no Sandbox) |
| montar um Windows limpo, escolher os dados, publicar ou instalar o app, ligar o CDP no guest | `claudia-sandbox` |

A `claudia-sandbox` usa um laboratório Windows Sandbox mantido fora do repositório, localizado pela variável `SANDBOX_LAB_DIR`. Cenários de dados: `novo`, `mock` e `copia` (este só com autorização do usuário na rodada). Falha do app num Windows limpo é resultado do teste: registre no Linear, não contorne no laboratório.
```

- [ ] **Passo 3:** `git diff CLAUDE.md` mostra só essas duas mudanças; `grep -n "claudia-teste" CLAUDE.md` não encontra nada.
- [ ] **Passo 4:** commit — `git commit -m "docs: route app observation and clean-machine testing to their skills"`

---

### Tarefa 11: smoke

Executor: agente Opus. Pré-requisitos: `SANDBOX_LAB_DIR` definido; nenhum Sandbox rodando (`wsb list` sem ids). Evidências em `%TEMP%\claudia-sandbox\evidencias\` (fora do repositório).

- [ ] **Passo 1: build** com target fora do repo: `$env:CARGO_TARGET_DIR="$env:TEMP\claudia-sandbox\target"; npm ci; npx tauri build --bundles nsis`. Registrar tempo, o `.exe` do app (`$env:CARGO_TARGET_DIR\release\claudia-rh.exe`) e o instalador (`$env:CARGO_TARGET_DIR\release\bundle\nsis\*.exe`).
- [ ] **Passo 2: cenários `novo` e `mock` com o exe solto.** Para cada `<c>`:
  1. `pwsh -File .claude/skills/claudia-sandbox/scripts/cenario-<c>.ps1`
  2. `pwsh -File .claude/skills/claudia-sandbox/scripts/publicar.ps1 -App <exe> -Cenario %TEMP%\claudia-sandbox\<c>`
  3. `$env:LAB_APP_PROJECT=<raiz do repo>; $env:LAB_WORK_DIR=<workDir impresso no passo 2>; & "$env:SANDBOX_LAB_DIR\scripts\test-up.ps1" -Config lab.config.app-winapp.json`
  4. (`<id>` = id impresso pelo `test-up.ps1`, também em `wsb list`) `wsb exec --id <id> -r System -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-firewall.ps1"`
  5. `wsb exec --id <id> -r ExistingLogin -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1"`
  6. Prints pela escada do Sandbox (degraus 2 e 3; o 1 depende de CDP): `winapp ui screenshot --on sandbox -a claudia-rh -o …\<c>-janela.png`, `winapp target screenshot sandbox -o …\<c>-tela.png`; ler os PNGs e descrever o que mostram.
  7. `& "$env:SANDBOX_LAB_DIR\scripts\test-down.ps1"`
- [ ] **Passo 3: instalador + CDP (cenário `mock`).**
  1. Cenário `mock`, `publicar.ps1 -App <instalador>`, `test-up.ps1 -Config lab.config.app-winapp.json` (sem firewall ainda: o instalador pode baixar o WebView2).
  2. `wsb exec --id <id> -r ExistingLogin -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1 -NoStart"` (copia os dados) e `wsb exec --id <id> -r ExistingLogin -c "C:\appdeploy\app\<instalador>.exe /S"`; aguardar; localizar o exe instalado (`winapp target exec sandbox -- powershell -c "Get-ChildItem $env:LOCALAPPDATA,$env:ProgramFiles -Recurse -Filter claudia-rh.exe -ErrorAction SilentlyContinue | Select -Expand FullName"`).
  3. Firewall (`guest-firewall.ps1` como System).
  4. `pwsh -File .claude/skills/claudia-sandbox/scripts/cdp-guest.ps1 -Iniciar -ExePath <exe instalado>` → `{host, porta}`.
  5. `$env:CDP_HOST=<host>; $env:CDP_PORT=<porta>; node .claude/skills/claudia-app/scripts/subir.mjs` (esperado `reutilizar`), `ver.mjs arvore`, `ver.mjs tela …\mock-cdp.png` (degrau 1).
  6. `banco-guest.ps1` → `$env:CLAUDIA_DB=<db>; node .claude/skills/claudia-app/scripts/banco.mjs "SELECT count(*) AS n FROM vagas"` (esperado `3`).
  7. Registrar a origem real da página do app (`/json/list`) e confrontar com a regra `tauri.localhost`; divergência → corrigir a regra e o teste (Tarefa 7) antes de seguir.
  8. `test-down.ps1`.
- [ ] **Passo 4: local** — com o app do usuário fechado ou com autorização para subir um `tauri dev`: `node .claude/skills/claudia-app/scripts/subir.mjs`, `ver.mjs arvore`, `pwsh .claude/skills/claudia-app/scripts/print-janela.ps1`. Se o app do usuário estiver aberto sem CDP, `subir.mjs` devolve `perguntar`: perguntar antes.
- [ ] **Passo 5:** cenário `copia` **só** se o usuário autorizar nesta rodada; sem autorização, registrar "não verificado: aguarda autorização".
- [ ] **Passo 6: relatório** em `%TEMP%\claudia-sandbox\evidencias\relatorio.md`: por passo, comandos, exit codes, o que cada print mostrou, qual degrau da escada funcionou. Falha do app é achado (nome, passo, print) para virar task no Linear; não é contornada.

---

### Tarefa 12: revisão e varredura

Executor: agente Opus, independente dos implementadores.

- [ ] **Passo 1:** revisão de código da branch inteira contra a spec (`requesting-code-review`): contrato dos cenários, exit codes, origem só lida, `disparo.json` completo, nenhuma escrita fora de `-Saida`/`-WorkDir`, `subir.mjs` nunca sobe `tauri dev` com alvo remoto, regras de segurança da `claudia-app` intactas.
- [ ] **Passo 2:** varredura de agnosticismo em `git diff dev --name-only`: procurar referências a sessões de trabalho, pessoas, modelos de IA usados na construção, nomes de máquina, IPs, usuários do Windows, caminhos absolutos pessoais ou pastas externas além das variáveis permitidas nas restrições globais. Cada ocorrência: arquivo:linha e correção.
- [ ] **Passo 3:** rodar de novo `pwsh -NoProfile -File .claude/skills/claudia-sandbox/scripts/test/run-all.ps1`, `node --test .claude/skills/claudia-app/scripts/test/` e `pwsh -NoProfile -File .claude/skills/claudia-app/scripts/test/print-janela.test.ps1`; conferir `exit=0` em cada um.

---

### Tarefa 13: integração (decisão do usuário)

- [ ] **Passo 1:** apresentar ao usuário o resumo: commits, resultado dos testes, evals, smoke e revisão. Aguardar a decisão.
- [ ] **Passo 2 (com aprovação):** `git checkout dev && git merge --no-ff feat/clean-machine-testing`.
- [ ] **Passo 3 (com aprovação):** `git checkout feat/browser-cdp-mcp && git rebase dev`. Conflito esperado em `CLAUDE.md` (conferir antes com `git diff dev feat/browser-cdp-mcp -- CLAUDE.md`): manter o conteúdo da branch, reinserir a seção "Teste em máquina limpa" depois de "## Build" e trocar toda referência `claudia-teste`/`/claudia-teste` por `claudia-app`/`/claudia-app` (`grep -n "claudia-teste" CLAUDE.md` vazio no fim). Arquivos da branch que citem `.claude/skills/claudia-teste/` fora de `docs/superpowers/` também passam ao caminho novo. Depois do rebase, rodar as rodadas de teste definidas no `CLAUDE.md` resultante (unitários: `cargo test --lib` em `src-tauri/` e `npx tsc --noEmit`) e as três suítes da Tarefa 12, Passo 3; conferir exit codes.
