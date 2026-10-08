# Ambiente do Sandbox declarado num arquivo — Plano de implementação

## ⛔ REGRAS DURAS PARA TODO AGENTE EXECUTOR (ler antes de qualquer comando)

Origem: em 2026-10-05 um teste passou `C:\` para uma função de apagar e destruiu o disco do usuário. Estas regras valem para todas as tarefas abaixo, sem exceção, e vencem qualquer passo que pareça dizer outra coisa.

1. **Nunca rodar no PC do usuário nada que apague.** No PC rodam só: testes puros (sem disco), parser de PowerShell, `node --test` com lista explícita de testes puros, `conferir-repo.mjs`, leitura de arquivos e `git`.
2. **Nunca passar caminho real a função que apaga, nem para testar a recusa.** Nem raiz de disco, nem `C:\`, nem o repositório, nem a pasta acima dele, nem `AppData`, nem `%USERPROFILE%`, nem a pasta do laboratório. A recusa se testa **só na função de validação pura**, com caminhos inventados (`Y:\inventado\...`).
3. **Todo teste que cria ou apaga pasta ou arquivo roda só no Windows Sandbox**, por `testes-no-guest.mjs` (Tarefa 12). Fora do Sandbox, `tempDir()` e o cabeçalho do `.sandbox.ps1` recusam antes de criar qualquer coisa.
4. **Script que apaga só aceita caminho validado:** absoluto, exatamente o esperado dentro da raiz permitida, não é raiz de disco, não é a pasta do usuário, sem junction/symlink no caminho nem dentro dele. A validação é uma função pura com teste.
5. **Nenhum caminho absoluto de máquina** em código, teste, script, skill, plano, doc ou mensagem de commit. Caminhos do guest (`C:\lab...`, `Z:\claudia-teste-sandbox`) são permitidos: são do Windows descartável.
6. **Nunca o Chrome pessoal do usuário** (nem perfil, nem cópia).
7. **Nunca abrir, imprimir, copiar para log ou comparar o conteúdo** de `sandbox-lab/shared/secrets/` nem da credencial do Claude (`.credentials.json`). Só existência, tamanho e data. O único código que lê a credencial é `Copy-LabCredential` (Tarefa 2), que copia bytes sem exibi-los.
8. **Não subir nem derrubar o Sandbox** fora da Tarefa 12, e lá só com autorização do usuário naquela rodada. Pode haver um Sandbox de outro trabalho rodando: não mexa nele.
9. **`git add` só com lista explícita de arquivos.** Nunca `git add -A`, `git add .` nem `git add <pasta>`. Sem push, sem merge.
10. **Na dúvida, não rode.** Pare e relate ao orquestrador.

---

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** o guest do Windows Sandbox deixa de emprestar toolchains do host: um `sandbox-lab/compose.json` declara ambientes nomeados, um cache local guarda instaladores com versão e SHA-256 fixos, o guest instala MSVC, Rust, Node e Claude Code sozinho, usa a conta Claude logada no PC na hora do `test-up` sem vazar caminho nem segredo, e todo arquivo que os testes criam ou apagam no guest fica num disco `Z:` próprio.

**Architecture:** decisões viram funções puras em PowerShell (`scripts/lib/compose.ps1`, `scripts/lib/travas.ps1`, `sandbox/guest-*.lib.ps1`) e Node (`caminhos-lab.mjs`, `proibidos.mjs`), testadas sem disco no PC e no CI. Os scripts com efeito (`test-up.ps1`, `test-down.ps1`, `cache-up.ps1`, `bootstrap.ps1`, `disco-z.ps1`, `instalar.ps1`) só chamam essas decisões e fazem o mínimo de disco. O que cria ou apaga é testado só no Sandbox.

**Tech Stack:** PowerShell 7 no host e Windows PowerShell 5.1 no guest, Node ≥ 22.13 (`node:test`, sem npm), Windows Sandbox (`wsb` CLI), `diskpart`/Storage cmdlets no guest, `curl.exe`, Visual Studio Build Tools 2022 (layout offline), MSI do Rust, zip do Node, binário nativo do Claude Code, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-07-sandbox-ambiente-declarativo-design.md` (APROVADA em 2026-10-07 com A1, A2, A4 na versão "conta logada", A6 e A7). Leia a spec inteira antes de qualquer tarefa.

## Global Constraints

- A1: arquivo único `sandbox-lab/compose.json` com `ambientes` nomeados (estilo docker-compose) e `pacotes`; os `lab.config*.json` saem. JSON, porque o guest roda PowerShell 5.1.
- A1: uso `scripts\test-up.ps1 -Ambiente <nome> -Project <projeto do onde.mjs>`. O arquivo nunca tem caminho do host: só nomes (`projeto`, `trabalho`, `deploy`), caminhos relativos ao laboratório e caminhos do guest (`C:\lab...`).
- A2: `scripts\cache-up.ps1` baixa as versões fixadas para `sandbox-lab/shared/cache/` (fora do git), confere SHA-256, **só cria**: arquivo novo vai para `.tmp` e depois é renomeado; **nunca apaga**; versão antiga fica até o usuário apagar à mão.
- A3: `sandbox\instalar.ps1` instala no guest, nesta ordem, MSVC → Rust → Node → Claude Code a partir do cache, grava `C:\lab\out\instalar.json` (versão, tempo, código de saída de cada pacote); `test-up.ps1` espera esse arquivo. Target do cargo continua no disco do guest. `guest-build.ps1` sem `C:\hosttools`. Mapeamentos de toolchain do host saem.
- A4: conta = **a logada no PC na hora do `test-up`** (sem `-Conta`, sem `contas.json`). Copia **só** o arquivo de credencial, achado em tempo de execução pela API (`[Environment]::GetFolderPath('UserProfile')`), para `.sandbox-work/`, montado só leitura; apagado no `test-down` (só aquele arquivo) por função de recusa pura. Nunca em log, `status.json` nem `guest.config.json`. O risco da renovação do token é **não verificado** e é o primeiro teste no Sandbox.
- A5: `Get-LabComposeRefusal` pura recusa caminho do host, variável de ambiente e nome desconhecido; `conferir-repo.mjs` detecta padrão de token; `shared/cache/` no `.gitignore` e no `proibidos.mjs`.
- A6: VHD montado como `Z:` no guest (diskpart como admin, seleciona o vdisk pelo arquivo e confere a letra antes de formatar; ~7 s); raiz fixa `Z:\claudia-teste-sandbox` em `util.mjs tempDir()`, `comum.mjs novoCenarioDir()` e `travas.sandbox.ps1`; recusa se o `realpathSync.native` não começa com `Z:\`; `testes-no-guest.mjs` passa `TEMP=Z:\tmp`; `cenario-copia.mjs:87` revisado; `test-up.ps1` recusa se o host tiver `Z:`. A trava de dois fatores (`exigirSandbox()`) continua.
- **Adendo 2026-10-07 (spec §8), APROVADO pelo usuário em 2026-10-07:** D1 = cenários **só no guest** (`novoCenarioDir` recusa fora do Sandbox; host dispara por `cenario-no-guest.mjs`; dados reais do `copia` só com `test-up -DadosReais`, pasta Roaming do app achada pela API, montada só leitura em `C:\labdadosreais`, nunca `%LOCALAPPDATA%`; `publicar.mjs` publica só o app; `guest-run.ps1 -Cenario`). MSVC = layout montado **dentro de um Sandbox** (ambiente `cache-msvc`, saída gravável `.sandbox-work\cache-saida` → `C:\labcachesaida`), conferido por SHA-256 no host e importado por `cache-up.ps1 -ImportarMsvc <id>` só criando. O PC não executa nada do Visual Studio. D2: todas aprovadas.
- A7: sai o caso `Q:\` do `travas.sandbox.ps1`; raiz de disco coberta só no `travas.test.ps1` puro.
- Versões fixadas (pesquisa de 2026-10-07, fontes oficiais, hashes conferidos por download dos arquivos oficiais de checksum):
  - Rust `1.95.0`: `https://static.rust-lang.org/dist/rust-1.95.0-x86_64-pc-windows-msvc.msi`, SHA-256 `08d228850565cfe421da6faeac029fb1f0c8341219d0dd9490d42a8355aa0703` (do `.msi.sha256` oficial), 303775340 bytes.
  - Node `22.23.3` (último 22.x LTS em `latest-v22.x`): `https://nodejs.org/dist/v22.23.3/node-v22.23.3-win-x64.zip`, SHA-256 `2b0ff57b049cda1bbcea2240eec20467018713c1efe1f7360c2681859b90ed71` (do `SHASUMS256.txt`), 35574076 bytes.
  - Claude Code `2.1.285` (canal `stable` em 2026-10-07): `https://downloads.claude.ai/claude-code-releases/2.1.285/win32-x64/claude.exe`, SHA-256 `121fc8151ed40bd9c144d68aa1cea23427803628ffab65e23da1cceda155697e` (do `manifest.json` da versão), 243751072 bytes.
  - MSVC: Visual Studio 2022 Build Tools, componentes `Microsoft.VisualStudio.Workload.VCTools`, `Microsoft.VisualStudio.Component.VC.Tools.x86.x64`, `Microsoft.VisualStudio.Component.Windows11SDK.26100`, idioma `en-US`. Versão exata e bootstrapper de versão fixa: a fixar na Tarefa 1 (Passo 1). O bootstrapper **nunca é executado no PC**.
- Docs em pt-BR; código, comentários e commits em inglês. Mensagens de commit terminam com:
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_019iZMxAmFGsgenP4pjSBmb6
  ```
- `node --test` sempre com lista explícita de arquivos (nunca glob nem pasta).
- Cada regra nova tem teste puro com **prova de mutação**: quebrar a linha indicada, rodar, colar a linha `FAILED` com o nome do teste, restaurar, rodar de novo e ver `all passed`.

## Review Focus

1. **Cópia da credencial fica no PC quando o `test-up` falha depois de copiar** (o `wsb start` erra, o id não aparece): espera-se que o `test-up` apague a cópia antes de sair. Teste: `Invoke-LabStartWithCredential` com `Start` que lança (Tarefa 1, teste "start with credential").
2. **`CLAUDE_CONFIG_DIR` definido no PC** (o usuário usa outra pasta de configuração): espera-se recusa clara antes de copiar qualquer coisa, sem adivinhar a outra pasta. Teste: `Get-LabCredentialSourceRefusal` com `-ConfigDir` (Tarefa 2).
3. **O PC tem uma unidade `Z:`** (pendrive, unidade de rede): espera-se que o `test-up` recuse antes de gerar arquivos ou subir o Sandbox, porque `Z:` passaria a existir nos dois lados. Teste: `Get-LabHostDriveRefusal` (Tarefa 2).
4. **Cache com conteúdo diferente do esperado**: arquivo com hash diferente (download antigo ou corrompido) ou layout do MSVC vindo do guest com arquivo faltando, sobrando ou alterado na cópia. Espera-se que o `cache-up.ps1` pare sem baixar, sem importar, sem sobrescrever e sem apagar. Testes: `Get-LabCachePlan` → `conflito` e `Get-LabLayoutManifestProblems` (Tarefa 2), teste estático "cache-up has no delete or overwrite command" (Tarefa 9).
5. **Dados reais do app no `copia`** (D1): espera-se que só a pasta Roaming do app (`io.github.johngabie.claudia-rh`) seja montada, só leitura, só com `-DadosReais`; nunca `%LOCALAPPDATA%`, nunca outra pasta, nunca com link no caminho; sem `-DadosReais`, o `copia` no guest para com mensagem clara. Testes: `Get-LabRealDataSourceRefusal` (Tarefa 2) e `comandoDeCenarioNoGuest` (Tarefa 5).

LogonCommand sem privilégio de administrador (não verificado) e `Z:` que não monta continuam cobertos por `Get-LabUpVerdict` (Tarefa 1) e `Get-LabZStartAction`/`Get-LabZFormatRefusal` (Tarefa 6).

## Contrato entre as tarefas (nomes que não mudam)

| Onde | Nome | Forma |
|---|---|---|
| `sandbox-lab/scripts/lib/compose.ps1` | `$script:LabInstallOrder` | `@('msvc','rust','node','claude-code')` |
| idem | `$script:LabMountTargets` | `projeto`→`C:\labproject` ro, `trabalho`→`C:\labwork` rw, `deploy`→`C:\appdeploy` ro, `cache-saida`→`C:\labcachesaida` rw |
| idem | `ConvertTo-LabGuestMsvcLayout -Compose` | `[ordered]@{versao;bootstrapper;sha256;componentes;saida}` (caminhos do guest) para o `layout-msvc.ps1` |
| idem | `$script:LabPackageTypes` | `msvc`=`vs-layout`, `rust`=`msi`, `node`=`zip`, `claude-code`=`exe` |
| idem | `$script:LabCacheGuestDir` | `C:\lab\cache` (o `shared\` já é montado em `C:\lab`) |
| idem | `Get-LabComposeRefusal -Compose <obj> -Ambiente <string>` | `$null` ou motivo |
| idem | `Get-LabPackageRefusal -Nome <string> -Pacote <obj>` | `$null` ou motivo |
| idem | `ConvertTo-LabGuestInstall -Compose -Ambiente` | array ordenado de `[ordered]@{nome;versao;tipo;arquivo;sha256;bytes;componentes;bootstrapper}` (`arquivo` e `bootstrapper` = caminhos no guest; `bootstrapper` vazio fora do `msvc`) |
| idem | `Get-LabComposeMounts -Compose -Ambiente` | array de `[ordered]@{nome;sandbox;readOnly}` sem `projeto` |
| idem | `Get-LabUpVerdict -Status -DiscoZ -Instalar -PrecisaInstalar` | `[ordered]@{exit;motivo}` |
| idem | `Invoke-LabStartWithCredential -Conta -Copy -Start -Cleanup` | devolve a saída de `Start` |
| `sandbox-lab/scripts/lib/travas.ps1` | `$script:LabGuestTestRoot` | `'Z:\claudia-teste-sandbox'` |
| idem | `Get-LabHostDriveRefusal -DriveNames <string[]>` | `$null` ou motivo |
| idem | `Get-LabGuestTestRootRefusal <string>` | `$null` ou motivo |
| idem | `Get-LabCredentialPaths <RepoRoot>` | `[ordered]@{Dir;File}` = `<repo>\.sandbox-work\credencial` e `...\.credentials.json` |
| idem | `Get-LabCredentialSourceRefusal -Source -UserProfile -ConfigDir -Links` | `$null` ou motivo |
| idem | `Get-LabCredentialCleanRefusal -File -RepoRoot -Links` | `$null` ou motivo |
| idem | `Get-LabCacheDirRefusal -CacheDir -LabDir -Links` | `$null` ou motivo |
| idem | `Get-LabCachePlan -Pacotes -Existentes` | array de `[ordered]@{nome;arquivo;acao;motivo}`, `acao` ∈ `baixar`,`ok`,`conflito` |
| idem | `Get-LabRealDataSourceRefusal -Source -RoamingAppData -Links` | `$null` ou motivo (só `<Roaming>\io.github.johngabie.claudia-rh`) |
| idem | `Get-LabLayoutImportRefusal -Source -RepoRoot -Versao -Links` | `$null` ou motivo (só `<repo>\.sandbox-work\cache-saida\<8 hex>\msvc-<versao>`) |
| idem | `Get-LabLayoutManifestProblems -ManifestText -Found` | array de problemas (vazio = igual); `Found`: caminho relativo com `/` → `"<sha256> <bytes>"` |
| `sandbox-lab/scripts/cache-up.ps1` | `-SoPlano`, `-ImportarMsvc <id>` | só cria |
| `sandbox-lab/sandbox/layout-msvc.ps1` | guest, System | grava `C:\labcachesaida\<id>\msvc-<v>\{layout\,layout.sha256}` e `C:\lab\out\layout-msvc.json` `{ok,id,versao,arquivos,bytes,segundos,erro}` |
| `test-up.ps1` | `-DadosReais` | monta só leitura a pasta Roaming do app em `C:\labdadosreais`; `guest.config.json` só ganha `dadosReais: true` |
| `cenario-no-guest.mjs` | `comandoDeCenarioNoGuest({cenario, autorizado, manterAutomacao, nonce})` (pura) | linha PowerShell para `C:\labsandbox\app\guest-cenario.ps1`; resultado em `C:\labwork\run\cenario.json` `{nonce,cenario,exit,saida,erro}` |
| `guest-run.ps1` | `-Cenario Z:\claudia-teste-sandbox\<nome>` | copia `appdata`/`localappdata` do cenário; recusa fora dessa raiz |
| `sandbox-lab/scripts/lib/disco.ps1` | `Copy-LabCredential -RepoRoot -UserProfile -ConfigDir`, `Remove-LabCredentialFile -File -RepoRoot`, `Remove-LabCredential -RepoRoot` | efeito em disco |
| `sandbox-lab/sandbox/guest-z.lib.ps1` | `Get-LabZFormatRefusal`, `Get-LabZStartAction` | puras |
| `sandbox-lab/sandbox/guest-install.lib.ps1` | `ConvertTo-LabInstallExit`, `Test-LabInstallOk`, `Get-LabInstallDone`, `Test-LabInstallNeed`, `Get-LabGuestToolEnv -Precisa todos\|node\|cargo`, `Test-LabGuestScriptDecision` (+ `Test-LabInGuest`, o leitor fino) | puras (adendo 2026-10-07) |
| `caminhos-lab.mjs` | `RAIZ_TESTE_GUEST`, `motivoRecusaRaizTeste(real)`, `raizDeCenarios({noSandbox})` → `"Z:\\"` ou `null` | puras |
| `proibidos.mjs` | `achaToken(texto)` → `[{linha}]` (nunca o texto) | pura |
| `comum.mjs` (D1) | `raizCenariosAtual()` (lança fora do guest), `saidaPadraoCenario(nome, tempRoot)` | |
| `guest.config.json` | novos campos `ambiente`, `conta` (bool), `dadosReais` (bool), `instalar` (array acima), `msvc` (de `ConvertTo-LabGuestMsvcLayout`, só quando o ambiente monta `cache-saida`) | sem caminho do host, sem segredo |
| `C:\lab\out\status.json` | novos campos `elevated` (bool), `discoZ` (`ok`/`failed`/`needs-system`), `conta` (`presente`/`ausente`/`falhou`/`nao-pedida`), `instalar` (`nada`/`iniciado`/`needs-system`) | |
| `C:\lab\out\disco-z.json` | `{ok, letra, imagem, segundos, erro}` | |
| `C:\lab\out\instalar.json` | `{inicio, fim, ok, elevado, erro, pacotes:[{nome,versao,exit,segundos,binDir,vcvars,versaoVista,erro}]}` | |
| Guest | `C:\labcred` (credencial, RO), `C:\labdadosreais` (dados reais do app, RO, só com `-DadosReais`), `C:\labcachesaida` (saída do layout, RW, só no `cache-msvc`), `C:\labtools\{msvc,node,claude}`, `C:\labdata\z.vhdx`, `Z:\tmp`, `Z:\claudia-teste-sandbox` | |

## Ondas de execução

| Onda | Tarefas (paralelas só em arquivos diferentes) |
|---|---|
| 0 | Tarefa 0 (decisões registradas; adendo da spec aguardando aprovação do trecho) |
| 1 | 1, 2, 3, 4, 6, 7 em paralelo |
| 2 | 5 (depende de 4), 5b (depende de 7), 8 (depende de 1, 2, 6, 7), 9 (depende de 1, 2, 7) |
| 3 | 10 (docs e CI), depois 11 (revisão Opus), depois 12 (Sandbox, com autorização) |

O adendo (§8 da spec) foi aprovado em 2026-10-07: as tarefas 5, 5b, 9 e as partes `-DadosReais`/`cache-msvc` da 1, 2 e 8 estão liberadas. Execução escolhida pelo usuário: agentes em worktree + revisão Opus.

Cada agente: worktree própria a partir de `feat/sandbox-ambiente`; antes de começar, `git log -1 --oneline` e alinhar com a ponta da branch da feature. Sem `cargo` nestas tarefas (nenhuma mexe em Rust); se precisar, `CARGO_TARGET_DIR` próprio fora do `target/` do repositório.

Comandos de teste puro usados abaixo (rodam no PC, não tocam o disco além de ler o próprio repositório):

```
pwsh -NoProfile -File sandbox-lab/scripts/test/<arquivo>.test.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File sandbox-lab/scripts/test/<arquivo>.test.ps1
node --test <arquivo1>.test.mjs <arquivo2>.test.mjs
```

Os dois PowerShell (7 e 5.1) porque `scripts/lib` roda no host em qualquer um deles e `sandbox/*.lib.ps1` roda no guest em 5.1.

---

### Tarefa 0: Decisões do usuário e adendo da spec (orquestrador, sem código)

**Modelo:** Opus (orquestrador). **Arquivos:** `docs/superpowers/specs/2026-10-07-sandbox-ambiente-declarativo-design.md` (§8, já escrito no mesmo commit deste plano revisado).

Decisões do usuário em 2026-10-07 (AskUserQuestion), registradas aqui e na spec §8:

- **D1 = "Cenários só no Sandbox".** `cenario-novo/mock/copia.mjs` rodam só dentro do guest, em `Z:\claudia-teste-sandbox\<cenario>`; nada de cenário no PC. O host dispara por `cenario-no-guest.mjs`. Os dados reais do `copia` vêm de uma montagem só leitura da pasta Roaming do app (`test-up -DadosReais`, só com autorização naquela rodada, pasta achada pela API, nunca `%LOCALAPPDATA%`). `publicar.mjs` publica só o app; `guest-run.ps1 -Cenario` instala os dados do cenário no perfil do guest. Tarefas 4, 5, 5b, 8.
- **MSVC = layout montado dentro de um Sandbox.** O PC só baixa o bootstrapper (hash conferido) e nunca o executa. O ambiente `cache-msvc` roda `sandbox\layout-msvc.ps1` como System e grava o layout e o `layout.sha256` numa pasta nova de `.sandbox-work\cache-saida` (montada gravável em `C:\labcachesaida`). O host confere SHA-256 e tamanho de cada arquivo e importa com `cache-up.ps1 -ImportarMsvc <id>`, só criando. Tarefas 1, 2, 8, 9.
- **D2 (a–g): todas aprovadas** como estão na lista abaixo.
  - D2a: ambiente `testes` com internet aberta; `app-winapp` continua com `guest-firewall.ps1` manual.
  - D2b: `cache` não é montagem (o cache aparece em `C:\lab\cache`).
  - D2c: ambientes `exemplo`, `app`, `app-winapp`, `testes` (e agora `cache-msvc`); `conta: true` só no `testes`.
  - D2d: `CLAUDE_CONFIG_DIR` definido no PC → `test-up` recusa.
  - D2e: Claude Code instalado copiando o `claude.exe` verificado, com `DISABLE_AUTOUPDATER=1`.
  - D2f: `test-up` exige o `wsb` CLI.
  - D2g: `test-up` confere o cache antes de subir.

- [ ] **Passo 1:** Mostrar ao usuário o §8 da spec ("Adendo de 2026-10-07 (plano) — AGUARDANDO APROVAÇÃO DO USUÁRIO DO TRECHO ALTERADO") e pedir a aprovação **desse trecho** (regra do CLAUDE.md: spec mudou → reaprovação do trecho alterado).
- [ ] **Passo 2:** Com a aprovação, trocar o título do §8 para "Adendo de 2026-10-07 (plano) — APROVADO em <data>" e fazer commit só da spec: `docs(spec): addendum 8 approved by the user` + rodapé. Sem aprovação, as Tarefas 5, 5b e 9 e as partes `-DadosReais`/`cache-msvc` das Tarefas 1, 2 e 8 não começam; o resto segue.

---

### Tarefa 1: `compose.json` e a biblioteca pura `compose.ps1`

**Modelo:** Sonnet.

**Files:**
- Create: `sandbox-lab/compose.json`
- Create: `sandbox-lab/scripts/lib/compose.ps1`
- Test: `sandbox-lab/scripts/test/compose.test.ps1`

**Interfaces:**
- Consumes: nada.
- Produces: tudo o que a tabela do contrato lista para `compose.ps1`. `compose.ps1` não depende de `travas.ps1`.

- [ ] **Passo 1: Fixar a versão do MSVC (pesquisa, sem instalar nada).** Na página oficial "Visual Studio 2022 Release History" (`https://learn.microsoft.com/en-us/visualstudio/releases/2022/release-history`), seção de bootstrappers de versão fixa, pegar a linha 17.14 mais nova e o link do **Build Tools**. Baixar só o bootstrapper para o scratchpad do agente (não para o repositório) e calcular o hash:
  ```powershell
  $f = Join-Path $env:TEMP ('vsbt-' + [guid]::NewGuid().ToString('N') + '.exe')
  curl.exe -fL -o $f '<link do bootstrapper de versão fixa>'
  (Get-FileHash -LiteralPath $f -Algorithm SHA256).Hash.ToLowerInvariant(); (Get-Item -LiteralPath $f).Length
  ```
  Anotar versão (`17.14.N`), URL, SHA-256 e bytes. **Não executar** o bootstrapper. Se a página não tiver link de versão fixa, parar e relatar (não usar `aka.ms`, que muda de versão).

- [ ] **Passo 2: Escrever o teste que falha** em `sandbox-lab/scripts/test/compose.test.ps1`:

```powershell
# Pure tests for lib/compose.ps1 and the versioned compose.json: invented values and the repo file only, no other disk.
#   pwsh -NoProfile -File sandbox-lab/scripts/test/compose.test.ps1
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../lib/compose.ps1"
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) {
  try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" }
}
function Should-BeNull($v) { if ($null -ne $v) { throw "expected null, got '$v'" } }
function Should-Match($v, [string]$re) { if ($null -eq $v -or "$v" -notmatch $re) { throw "expected /$re/, got '$v'" } }

$H = 'a' * 64
function New-Compose {
  $json = @"
{
  "comum": { "dbBackup": "shared/db/backup.sqlite", "vGPU": "Enable" },
  "ambientes": {
    "testes": { "rede": true, "memoriaMB": 8192, "conta": true, "instalar": ["claude-code", "node", "rust", "msvc"], "montar": ["projeto:ro", "trabalho:rw"] },
    "app":    { "rede": false, "memoriaMB": 6144, "conta": false, "instalar": [], "montar": ["projeto:ro", "deploy:ro", "trabalho:rw"] }
  },
  "pacotes": {
    "msvc": { "tipo": "vs-layout", "versao": "17.14.1", "arquivo": "msvc-17.14.1", "componentes": ["Microsoft.VisualStudio.Workload.VCTools"], "bootstrapper": { "arquivo": "vs_buildtools-17.14.1.exe", "url": "https://example.invalid/vs_buildtools.exe", "sha256": "$H", "bytes": 10 } },
    "rust": { "tipo": "msi", "versao": "1.95.0", "arquivo": "rust.msi", "url": "https://example.invalid/rust.msi", "sha256": "$H", "bytes": 10 },
    "node": { "tipo": "zip", "versao": "22.23.3", "arquivo": "node.zip", "url": "https://example.invalid/node.zip", "sha256": "$H", "bytes": 10 },
    "claude-code": { "tipo": "exe", "versao": "2.1.285", "arquivo": "claude.exe", "url": "https://example.invalid/claude.exe", "sha256": "$H", "bytes": 10 }
  }
}
"@
  return ($json | ConvertFrom-Json)
}

Check 'valid: every ambiente of the sample passes' {
  $c = New-Compose
  Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')
  Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'app')
}
Check 'ambiente: missing or unknown name is refused' {
  $c = New-Compose
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente '') 'ambiente'
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'nope') 'unknown ambiente'
  Should-Match (Get-LabComposeRefusal -Compose $null -Ambiente 'testes') 'empty'
}
Check 'strings: host paths, variables, UNC, home and .. are refused anywhere' {
  foreach ($v in @('D:\dados\x', 'c:/dados/x', 'C:\Windows\x', '%USERPROFILE%\x', '%ProgramFiles(x86)%\y', '$env:APPDATA', '${HOME}', '\\srv\share\x', '~\x', 'shared\..\..\x', 'C:x')) {
    $c = New-Compose
    $c.comum | Add-Member -NotePropertyName 'extra' -NotePropertyValue $v
    Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'testes') 'comum'
  }
}
Check 'strings: a host path hidden in an array or a property name is refused too' {
  $c = New-Compose; $c.ambientes.testes.instalar = @('node', 'D:\x')
  if (-not (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')) { throw 'array value' }
  $c = New-Compose; $c.comum | Add-Member -NotePropertyName 'D:\x' -NotePropertyValue 'y'
  if (-not (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')) { throw 'property name' }
}
Check 'strings: guest paths under C:\lab, lab-relative paths and https URLs are allowed' {
  foreach ($v in @('C:\labwork\x', 'shared/db/backup.sqlite', 'https://example.invalid/a')) {
    $c = New-Compose; $c.comum | Add-Member -NotePropertyName 'extra' -NotePropertyValue $v
    Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')
  }
}
Check 'montar: unknown name, wrong mode, duplicates, bad shape and missing projeto are refused' {
  foreach ($m in @(@('projeto:ro', 'casa:rw'), @('projeto:rw'), @('projeto:ro', 'trabalho:ro'), @('projeto:ro', 'deploy:rw'), @('projeto:ro', 'projeto:ro'), @('projeto'), @('trabalho:rw'))) {
    $c = New-Compose; $c.ambientes.testes.montar = $m
    if (-not (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')) { throw "must refuse $($m -join ',')" }
  }
}
Check 'ambiente fields: rede bool, memoriaMB 2048..32768, conta bool' {
  foreach ($set in @(@('rede', 'sim'), @('memoriaMB', 1024), @('memoriaMB', 65536), @('memoriaMB', '8192'), @('conta', 'yes'))) {
    $c = New-Compose; $c.ambientes.testes.($set[0]) = $set[1]
    if (-not (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')) { throw "must refuse $($set[0])=$($set[1])" }
  }
}
Check 'instalar: unknown or undeclared package is refused' {
  $c = New-Compose; $c.ambientes.testes.instalar = @('python')
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'testes') 'unknown package'
  $c = New-Compose; $c.pacotes.PSObject.Properties.Remove('node')
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'testes') 'not declared'
  Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'app')
}
Check 'pacotes: bad tipo, versao, arquivo, url, sha256 and bytes are refused' {
  $bad = @(
    @('tipo', 'zip'), @('versao', 'latest'), @('versao', '1.x'), @('arquivo', 'a\b.msi'), @('arquivo', 'a/b.msi'), @('arquivo', ' x'),
    @('url', 'http://example.invalid/r.msi'), @('url', 'ftp://example.invalid/r'), @('url', ''), @('sha256', ('A' * 64)), @('sha256', ('a' * 63)),
    @('bytes', 0), @('bytes', -1), @('bytes', '10')
  )
  foreach ($b in $bad) {
    $c = New-Compose; $c.pacotes.rust.($b[0]) = $b[1]
    Should-Match (Get-LabPackageRefusal -Nome 'rust' -Pacote $c.pacotes.rust) "rust"
  }
}
Check 'msvc: bootstrapper and componentes are required and checked' {
  $c = New-Compose; $c.pacotes.msvc.PSObject.Properties.Remove('bootstrapper')
  Should-Match (Get-LabPackageRefusal -Nome 'msvc' -Pacote $c.pacotes.msvc) 'bootstrapper'
  $c = New-Compose; $c.pacotes.msvc.componentes = @()
  Should-Match (Get-LabPackageRefusal -Nome 'msvc' -Pacote $c.pacotes.msvc) 'componentes'
  $c = New-Compose; $c.pacotes.msvc.componentes = @('Foo.Bar')
  Should-Match (Get-LabPackageRefusal -Nome 'msvc' -Pacote $c.pacotes.msvc) 'componentes'
  $c = New-Compose; $c.pacotes.msvc.bootstrapper.sha256 = 'x'
  Should-Match (Get-LabPackageRefusal -Nome 'msvc' -Pacote $c.pacotes.msvc) 'sha256'
  foreach ($a in @('vs\b.exe', 'vs_buildtools', '')) {
    $c = New-Compose; $c.pacotes.msvc.bootstrapper.arquivo = $a
    Should-Match (Get-LabPackageRefusal -Nome 'msvc' -Pacote $c.pacotes.msvc) 'bootstrapper.arquivo'
  }
}
Check 'install order is fixed (msvc, rust, node, claude-code) whatever the compose order, with guest cache paths' {
  $c = New-Compose
  $l = @(ConvertTo-LabGuestInstall -Compose $c -Ambiente 'testes')
  if ((@($l | ForEach-Object { $_.nome }) -join ',') -ne 'msvc,rust,node,claude-code') { throw "order: $(@($l | ForEach-Object { $_.nome }) -join ',')" }
  if ($l[1].arquivo -cne 'C:\lab\cache\rust.msi') { throw "rust path: $($l[1].arquivo)" }
  if ($l[0].arquivo -cne 'C:\lab\cache\msvc-17.14.1') { throw "msvc path: $($l[0].arquivo)" }
  if ($l[0].componentes[0] -ne 'Microsoft.VisualStudio.Workload.VCTools') { throw 'componentes not carried' }
  if ($l[0].sha256 -ne '' -or $l[1].sha256 -ne $H -or [long]$l[1].bytes -ne 10) { throw 'sha256/bytes not carried' }
  if (@(ConvertTo-LabGuestInstall -Compose $c -Ambiente 'app').Count -ne 0) { throw 'app installs nothing' }
}
Check 'mounts: extras only (projeto is in the template), compose order, guest folder and read-only flag' {
  $m = @(Get-LabComposeMounts -Compose (New-Compose) -Ambiente 'app')
  if ($m.Count -ne 2) { throw "count $($m.Count)" }
  if ($m[0].nome -ne 'deploy' -or $m[0].sandbox -cne 'C:\appdeploy' -or $m[0].readOnly -ne $true) { throw 'deploy' }
  if ($m[1].nome -ne 'trabalho' -or $m[1].sandbox -cne 'C:\labwork' -or $m[1].readOnly -ne $false) { throw 'trabalho' }
}
Check 'cache-saida: rw mount for the MSVC layout ambiente; ro is refused' {
  $c = New-Compose; $c.ambientes.app.montar = @('projeto:ro', 'cache-saida:rw')
  Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'app')
  $m = @(Get-LabComposeMounts -Compose $c -Ambiente 'app')
  if ($m[0].nome -ne 'cache-saida' -or $m[0].sandbox -cne 'C:\labcachesaida' -or $m[0].readOnly -ne $false) { throw 'cache-saida mount' }
  $c.ambientes.app.montar = @('projeto:ro', 'cache-saida:ro')
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'app') 'cache-saida'
}
Check 'msvc layout entry for the guest: version, bootstrapper guest path, hash, components, output folder' {
  $l = ConvertTo-LabGuestMsvcLayout -Compose (New-Compose)
  if ($l.versao -ne '17.14.1' -or $l.bootstrapper -cne 'C:\lab\cache\vs_buildtools-17.14.1.exe' -or $l.sha256 -ne $H -or $l.saida -cne 'C:\labcachesaida') { throw "got $($l | ConvertTo-Json -Compress)" }
  if ($l.componentes[0] -ne 'Microsoft.VisualStudio.Workload.VCTools') { throw 'componentes' }
  $i = @(ConvertTo-LabGuestInstall -Compose (New-Compose) -Ambiente 'testes')
  if ($i[0].bootstrapper -cne 'C:\lab\cache\vs_buildtools-17.14.1.exe' -or $i[1].bootstrapper -ne '') { throw 'install bootstrapper field' }
}
Check 'up verdict: status, Z: drive and install decide the exit code' {
  $ok = [pscustomobject]@{ bootstrap = 'ok' }
  $z = [pscustomobject]@{ ok = $true }
  $inst = [pscustomobject]@{ fim = 'x'; ok = $true; pacotes = @() }
  if ((Get-LabUpVerdict -Status $null -DiscoZ $z -Instalar $inst -PrecisaInstalar $true).exit -ne 2) { throw 'no status -> 2' }
  Should-Match (Get-LabUpVerdict -Status ([pscustomobject]@{ bootstrap = 'failed' }) -DiscoZ $z -Instalar $inst -PrecisaInstalar $true).motivo 'bootstrap'
  Should-Match (Get-LabUpVerdict -Status $ok -DiscoZ $null -Instalar $inst -PrecisaInstalar $true).motivo 'Z:'
  Should-Match (Get-LabUpVerdict -Status $ok -DiscoZ ([pscustomobject]@{ ok = $false; erro = 'diskpart x' }) -Instalar $inst -PrecisaInstalar $true).motivo 'diskpart x'
  Should-Match (Get-LabUpVerdict -Status $ok -DiscoZ $z -Instalar $null -PrecisaInstalar $true).motivo 'instalar'
  $bad = [pscustomobject]@{ fim = 'x'; ok = $false; pacotes = @([pscustomobject]@{ nome = 'rust'; exit = 1603 }) }
  Should-Match (Get-LabUpVerdict -Status $ok -DiscoZ $z -Instalar $bad -PrecisaInstalar $true).motivo 'rust'
  if ((Get-LabUpVerdict -Status $ok -DiscoZ $z -Instalar $inst -PrecisaInstalar $true).exit -ne 0) { throw 'all ok -> 0' }
  if ((Get-LabUpVerdict -Status $ok -DiscoZ $z -Instalar $null -PrecisaInstalar $false).exit -ne 0) { throw 'nothing to install -> 0' }
  if ((Get-LabUpVerdict -Status $ok -DiscoZ $null -Instalar $null -PrecisaInstalar $false).exit -ne 1) { throw 'Z: is always required' }
}
Check 'start with credential: cleanup runs when the start throws, never when conta is false; a failed copy stops before start' {
  $log = New-Object System.Collections.Generic.List[string]
  $threw = $false
  try { Invoke-LabStartWithCredential -Conta $true -Copy { $log.Add('copy') } -Start { $log.Add('start'); throw 'boom' } -Cleanup { $log.Add('cleanup') } } catch { $threw = $true }
  if (-not $threw -or ($log -join ',') -ne 'copy,start,cleanup') { throw "throwing start: $($log -join ',')" }
  $log.Clear()
  $id = Invoke-LabStartWithCredential -Conta $true -Copy { $log.Add('copy') } -Start { 'id-1' } -Cleanup { $log.Add('cleanup') }
  if ($id -ne 'id-1' -or ($log -join ',') -ne 'copy') { throw "success: id=$id log=$($log -join ',')" }
  $log.Clear()
  try { Invoke-LabStartWithCredential -Conta $false -Copy { $log.Add('copy') } -Start { throw 'boom' } -Cleanup { $log.Add('cleanup') } } catch { }
  if ($log.Count) { throw "conta false: $($log -join ',')" }
  $log.Clear()
  try { Invoke-LabStartWithCredential -Conta $true -Copy { throw 'no login' } -Start { $log.Add('start') } -Cleanup { $log.Add('cleanup') } } catch { }
  if ($log -contains 'start') { throw 'start ran after a failed copy' }
}
Check 'the versioned compose.json passes for every ambiente and pins the researched versions' {
  $c = Get-Content -LiteralPath "$PSScriptRoot/../../compose.json" -Raw | ConvertFrom-Json
  $names = @($c.ambientes.PSObject.Properties | ForEach-Object { $_.Name })
  foreach ($n in @('exemplo', 'app', 'app-winapp', 'testes', 'cache-msvc')) { if ($names -notcontains $n) { throw "missing ambiente $n" } }
  foreach ($n in @('app', 'app-winapp')) { if ((@($c.ambientes.$n.instalar) -join ',') -ne 'node') { throw "$n must install node (scenarios run in the guest)" } }
  if ((@($c.ambientes.'cache-msvc'.montar) -join ',') -ne 'projeto:ro,cache-saida:rw' -or @($c.ambientes.'cache-msvc'.instalar).Count) { throw 'cache-msvc ambiente' }
  foreach ($n in $names) { Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente $n) }
  if ($c.pacotes.rust.versao -ne '1.95.0' -or $c.pacotes.rust.sha256 -ne '08d228850565cfe421da6faeac029fb1f0c8341219d0dd9490d42a8355aa0703') { throw 'rust pin' }
  if ($c.pacotes.node.versao -ne '22.23.3' -or $c.pacotes.node.sha256 -ne '2b0ff57b049cda1bbcea2240eec20467018713c1efe1f7360c2681859b90ed71') { throw 'node pin' }
  if ($c.pacotes.'claude-code'.versao -ne '2.1.285' -or $c.pacotes.'claude-code'.sha256 -ne '121fc8151ed40bd9c144d68aa1cea23427803628ffab65e23da1cceda155697e') { throw 'claude-code pin' }
  if ((@($c.ambientes.testes.instalar) -join ',') -ne 'msvc,rust,node,claude-code' -or $c.ambientes.testes.conta -ne $true) { throw 'testes ambiente' }
}

if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
```

- [ ] **Passo 3: Rodar e ver falhar.** `pwsh -NoProfile -File sandbox-lab/scripts/test/compose.test.ps1` → erro ao carregar `compose.ps1` (arquivo não existe), exit ≠ 0.

- [ ] **Passo 4: Implementar `sandbox-lab/scripts/lib/compose.ps1`:**

```powershell
# Pure helpers for sandbox-lab/compose.json: object and string checks only. No disk, no [IO.Path], no environment
# variables (the tests run on any OS). Objects come from ConvertFrom-Json (PSCustomObject), never hashtables.
# A refusal returns $null when allowed, otherwise the reason.

$script:LabInstallOrder = @('msvc', 'rust', 'node', 'claude-code')
$script:LabMountTargets = [ordered]@{
  projeto  = @{ sandbox = 'C:\labproject'; modo = 'ro' }
  trabalho = @{ sandbox = 'C:\labwork'; modo = 'rw' }
  deploy   = @{ sandbox = 'C:\appdeploy'; modo = 'ro' }
  # Output of the MSVC layout built in the guest (ambiente cache-msvc, spec addendum 8.2).
  'cache-saida' = @{ sandbox = 'C:\labcachesaida'; modo = 'rw' }
}
$script:LabPackageTypes = @{ 'msvc' = 'vs-layout'; 'rust' = 'msi'; 'node' = 'zip'; 'claude-code' = 'exe' }
# shared\ is mapped to C:\lab in the guest, so the cache needs no mount of its own.
$script:LabCacheGuestDir = 'C:\lab\cache'

function Get-LabProp($Object, [string]$Name) {
  if ($null -eq $Object -or $Object -isnot [System.Management.Automation.PSCustomObject]) { return $null }
  $p = $Object.PSObject.Properties[$Name]
  if ($p) { return $p.Value }
  return $null
}

function Get-LabPropNames($Object) {
  if ($null -eq $Object -or $Object -isnot [System.Management.Automation.PSCustomObject]) { return @() }
  return @($Object.PSObject.Properties | ForEach-Object { $_.Name })
}

function Get-LabList($Value) {
  # JSON arrays come as object[]; a missing key is $null and must become an empty list, not @($null).
  return @(@($Value) | Where-Object { $null -ne $_ })
}

function Get-LabComposeStrings($Node, [string]$Where = '') {
  # Every string value and property name under $Node, with where it was found.
  $out = New-Object System.Collections.Generic.List[object]
  if ($null -eq $Node) { return ,$out.ToArray() }
  if ($Node -is [string]) { $out.Add([pscustomobject]@{ Where = $Where; Value = $Node }); return ,$out.ToArray() }
  if ($Node -is [System.Management.Automation.PSCustomObject]) {
    foreach ($p in $Node.PSObject.Properties) {
      $out.Add([pscustomobject]@{ Where = "$Where.$($p.Name)"; Value = $p.Name })
      foreach ($s in (Get-LabComposeStrings $p.Value "$Where.$($p.Name)")) { $out.Add($s) }
    }
    return ,$out.ToArray()
  }
  if ($Node -is [System.Collections.IEnumerable]) {
    $i = 0
    foreach ($e in $Node) { foreach ($s in (Get-LabComposeStrings $e "$Where[$i]")) { $out.Add($s) }; $i++ }
  }
  return ,$out.ToArray()
}

function Get-LabComposeStringProblem([string]$Value) {
  # compose.json holds names, lab-relative paths, guest paths (C:\lab...) and https URLs. Nothing that points at the host.
  if ([string]::IsNullOrEmpty($Value)) { return $null }
  if ($Value -match '%[A-Za-z_][A-Za-z0-9_()]*%') { return "environment variable in '$Value'" }
  if ($Value -match '\$env:|\$\{|\$\(') { return "PowerShell variable in '$Value'" }
  if ($Value -match '^\s*~') { return "home shortcut in '$Value'" }
  if ($Value -match '^(\\\\|//)') { return "UNC path '$Value'" }
  if ($Value -match '(^|[\\/])\.\.([\\/]|$)') { return "'..' in '$Value'" }
  if ($Value -match '^[A-Za-z]:[\\/]' -and $Value -notmatch '^[Cc]:\\[Ll][Aa][Bb]') { return "host absolute path '$Value'" }
  if ($Value -match '^[A-Za-z]:[^\\/]') { return "drive-relative path '$Value'" }
  return $null
}

function Get-LabPackageRefusal {
  param([string]$Nome, $Pacote)
  if ($null -eq $Pacote) { return "package '$Nome' is not declared in pacotes" }
  $tipo = Get-LabProp $Pacote 'tipo'
  if ("$tipo" -ne $script:LabPackageTypes[$Nome]) { return "package '$Nome': tipo must be '$($script:LabPackageTypes[$Nome])'" }
  if ("$(Get-LabProp $Pacote 'versao')" -notmatch '^\d+(\.\d+){1,3}$') { return "package '$Nome': versao must be digits and dots" }
  if ("$(Get-LabProp $Pacote 'arquivo')" -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]*$') { return "package '$Nome': arquivo must be a plain file or folder name" }
  $src = $Pacote
  if ($tipo -eq 'vs-layout') {
    $src = Get-LabProp $Pacote 'bootstrapper'
    if ($null -eq $src) { return "package '$Nome': bootstrapper is required" }
    if ("$(Get-LabProp $src 'arquivo')" -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]*\.exe$') { return "package '$Nome': bootstrapper.arquivo must be a plain .exe file name" }
    $comp = Get-LabList (Get-LabProp $Pacote 'componentes')
    if (-not $comp.Count) { return "package '$Nome': componentes is empty" }
    foreach ($c in $comp) { if ("$c" -notmatch '^Microsoft\.VisualStudio\.[A-Za-z0-9.]+$') { return "package '$Nome': bad componentes entry '$c'" } }
  }
  if ("$(Get-LabProp $src 'url')" -notmatch '^https://\S+$') { return "package '$Nome': url must be https" }
  if ("$(Get-LabProp $src 'sha256')" -cnotmatch '^[0-9a-f]{64}$') { return "package '$Nome': sha256 must be 64 lowercase hex characters" }
  $bytes = Get-LabProp $src 'bytes'
  if (-not ($bytes -is [int] -or $bytes -is [long]) -or $bytes -le 0) { return "package '$Nome': bytes must be a positive integer" }
  return $null
}

function Get-LabComposeRefusal {
  param($Compose, [string]$Ambiente)
  if ($null -eq $Compose) { return 'compose is empty' }
  foreach ($s in (Get-LabComposeStrings $Compose)) {
    $why = Get-LabComposeStringProblem $s.Value
    if ($why) { return "$($s.Where): $why" }
  }
  $ambientes = Get-LabProp $Compose 'ambientes'
  $names = Get-LabPropNames $ambientes
  if ([string]::IsNullOrWhiteSpace($Ambiente)) { return "ambiente is required (one of: $($names -join ', '))" }
  if ($names -notcontains $Ambiente) { return "unknown ambiente '$Ambiente' (one of: $($names -join ', '))" }
  $a = Get-LabProp $ambientes $Ambiente
  if ((Get-LabProp $a 'rede') -isnot [bool]) { return "ambiente ${Ambiente}: rede must be true or false" }
  $mem = Get-LabProp $a 'memoriaMB'
  if (-not ($mem -is [int] -or $mem -is [long]) -or $mem -lt 2048 -or $mem -gt 32768) { return "ambiente ${Ambiente}: memoriaMB must be an integer between 2048 and 32768" }
  $conta = Get-LabProp $a 'conta'
  if ($null -ne $conta -and $conta -isnot [bool]) { return "ambiente ${Ambiente}: conta must be true or false" }
  $seen = @{}
  foreach ($m in (Get-LabList (Get-LabProp $a 'montar'))) {
    if ($m -isnot [string] -or $m -notmatch '^([a-z][a-z-]*):(ro|rw)$') { return "ambiente ${Ambiente}: montar entry '$m' must be <name>:ro or <name>:rw" }
    $n = $Matches[1].ToLowerInvariant(); $modo = $Matches[2].ToLowerInvariant()
    if (-not $script:LabMountTargets.Contains($n)) { return "ambiente ${Ambiente}: unknown mount name '$n' (one of: $(@($script:LabMountTargets.Keys) -join ', '))" }
    if ($script:LabMountTargets[$n].modo -ne $modo) { return "ambiente ${Ambiente}: mount '$n' must be :$($script:LabMountTargets[$n].modo)" }
    if ($seen.ContainsKey($n)) { return "ambiente ${Ambiente}: mount '$n' listed twice" }
    $seen[$n] = $true
  }
  if (-not $seen.ContainsKey('projeto')) { return "ambiente ${Ambiente}: montar must include projeto:ro" }
  $pacotes = Get-LabProp $Compose 'pacotes'
  foreach ($nome in (Get-LabList (Get-LabProp $a 'instalar'))) {
    if ($script:LabInstallOrder -notcontains $nome) { return "ambiente ${Ambiente}: unknown package '$nome'" }
    $why = Get-LabPackageRefusal -Nome $nome -Pacote (Get-LabProp $pacotes $nome)
    if ($why) { return $why }
  }
  return $null
}

function ConvertTo-LabGuestInstall {
  # The guest.config.json "instalar" list: fixed order (MSVC first: Rust links with it), guest cache paths, no host path.
  param($Compose, [string]$Ambiente)
  $a = Get-LabProp (Get-LabProp $Compose 'ambientes') $Ambiente
  $wanted = Get-LabList (Get-LabProp $a 'instalar')
  $pacotes = Get-LabProp $Compose 'pacotes'
  $list = @()
  foreach ($nome in $script:LabInstallOrder) {
    if ($wanted -notcontains $nome) { continue }
    $p = Get-LabProp $pacotes $nome
    $tipo = [string](Get-LabProp $p 'tipo')
    $list += [ordered]@{
      nome        = $nome
      versao      = [string](Get-LabProp $p 'versao')
      tipo        = $tipo
      arquivo     = $script:LabCacheGuestDir + '\' + (Get-LabProp $p 'arquivo')
      sha256      = $(if ($tipo -eq 'vs-layout') { '' } else { [string](Get-LabProp $p 'sha256') })
      bytes       = $(if ($tipo -eq 'vs-layout') { 0 } else { [long](Get-LabProp $p 'bytes') })
      componentes = @(Get-LabList (Get-LabProp $p 'componentes'))
      bootstrapper = $(if ($tipo -eq 'vs-layout') { $script:LabCacheGuestDir + '\' + (Get-LabProp (Get-LabProp $p 'bootstrapper') 'arquivo') } else { '' })
    }
  }
  return ,$list
}

function ConvertTo-LabGuestMsvcLayout {
  # guest.config.json "msvc" entry for sandbox\layout-msvc.ps1 (ambiente cache-msvc, spec addendum 8.2). Guest paths only.
  param($Compose)
  $p = Get-LabProp (Get-LabProp $Compose 'pacotes') 'msvc'
  if ($null -eq $p) { return $null }
  $b = Get-LabProp $p 'bootstrapper'
  return [ordered]@{
    versao       = [string](Get-LabProp $p 'versao')
    bootstrapper = $script:LabCacheGuestDir + '\' + (Get-LabProp $b 'arquivo')
    sha256       = [string](Get-LabProp $b 'sha256')
    componentes  = @(Get-LabList (Get-LabProp $p 'componentes'))
    saida        = $script:LabMountTargets['cache-saida'].sandbox
  }
}

function Get-LabComposeMounts {
  # Mounts besides projeto (always in sandbox.template.wsb), in compose order.
  param($Compose, [string]$Ambiente)
  $a = Get-LabProp (Get-LabProp $Compose 'ambientes') $Ambiente
  $out = @()
  foreach ($m in (Get-LabList (Get-LabProp $a 'montar'))) {
    $n = ($m -split ':')[0].ToLowerInvariant()
    if ($n -eq 'projeto') { continue }
    $out += [ordered]@{ nome = $n; sandbox = $script:LabMountTargets[$n].sandbox; readOnly = ($script:LabMountTargets[$n].modo -eq 'ro') }
  }
  return ,$out
}

function Get-LabUpVerdict {
  # Final exit code of test-up.ps1: 0 ok, 1 something in the guest failed, 2 the guest never reported.
  param($Status, $DiscoZ, $Instalar, [bool]$PrecisaInstalar)
  if ($null -eq $Status) { return [ordered]@{ exit = 2; motivo = 'status.json did not appear' } }
  if ($Status.bootstrap -ne 'ok') { return [ordered]@{ exit = 1; motivo = "bootstrap=$($Status.bootstrap)" } }
  if ($null -eq $DiscoZ) { return [ordered]@{ exit = 1; motivo = 'Z: drive not ready in the guest: disco-z.json missing' } }
  if (-not $DiscoZ.ok) { return [ordered]@{ exit = 1; motivo = "Z: drive not ready in the guest: $($DiscoZ.erro)" } }
  if ($PrecisaInstalar) {
    if ($null -eq $Instalar -or -not $Instalar.fim) { return [ordered]@{ exit = 1; motivo = 'instalar.json did not finish in time' } }
    if (-not $Instalar.ok) {
      $bad = @(@($Instalar.pacotes) | Where-Object { $_ -and $_.exit -ne 0 } | ForEach-Object { "$($_.nome) (exit $($_.exit))" })
      return [ordered]@{ exit = 1; motivo = "instalar failed: $(if ($bad.Count) { $bad -join ', ' } else { $Instalar.erro })" }
    }
  }
  return [ordered]@{ exit = 0; motivo = 'bootstrap ok, Z: ready' + $(if ($PrecisaInstalar) { ', packages installed' } else { '' }) }
}

function Invoke-LabStartWithCredential {
  # Copy the credential (when the ambiente asks), start the guest, and remove the copy if the start throws.
  # The three steps are injected, so the ordering is tested without a disk.
  param([bool]$Conta, [scriptblock]$Copy, [scriptblock]$Start, [scriptblock]$Cleanup)
  if ($Conta) { & $Copy | Out-Null }
  try { return (& $Start) }
  catch { if ($Conta) { & $Cleanup | Out-Null }; throw }
}
```

- [ ] **Passo 5: Escrever `sandbox-lab/compose.json`** (valores de `msvc` do Passo 1; o resto exatamente como abaixo; `comum` copiado dos `lab.config*.json` atuais):

```json
{
  "$comment": "Ambientes do Windows Sandbox (spec 2026-10-07-sandbox-ambiente-declarativo). Uso: scripts\\test-up.ps1 -Ambiente <nome> -Project <projeto do onde.mjs>. Sem caminho do host: so nomes de montagem (projeto, trabalho, deploy), caminhos relativos a sandbox-lab/ e caminhos do guest. Instaladores: scripts\\cache-up.ps1 baixa os pacotes para shared/cache (fora do git).",
  "comum": {
    "extensionsDir": "shared/extensions",
    "extensions": ["lab-ext"],
    "dbBackup": "shared/db/backup.sqlite",
    "dbHashFile": "shared/db/backup.sha256",
    "dbVerifySql": "shared/db/verify.sql",
    "dbVerifyExpected": "shared/db/verify.expected.txt",
    "chrome": {
      "exe": "shared/tools/chrome/chrome-win64/chrome.exe",
      "startUrl": "http://localhost:8080/test-page.html",
      "testServerPort": 8080,
      "remoteDebuggingPort": 9222
    },
    "mcp": {
      "$comment": "Detalhes do Windows-MCP ficam em shared/mcp.json; port deve bater com ele.",
      "port": 8000,
      "configFile": "shared/mcp.json",
      "authKeyFile": "shared/secrets/mcp-auth.key",
      "authKeyEnv": "LAB_MCP_AUTH_KEY"
    },
    "vGPU": "Enable",
    "clipboardRedirection": "Enable"
  },
  "ambientes": {
    "exemplo": {
      "$comment": "Lab de exemplo (O1-O10). Sem -Project, monta sample-project.",
      "rede": true, "memoriaMB": 4096, "bootTimeoutSec": 240, "conta": false,
      "instalar": [], "montar": ["projeto:ro"]
    },
    "app": {
      "$comment": "App desktop SEM rede; controle so por wsb exec. Antes: publicar.mjs (so o app). Dados: cenario-no-guest.mjs (Node do guest) + guest-run.ps1 -Cenario.",
      "rede": false, "memoriaMB": 6144, "bootTimeoutSec": 300, "instalarTimeoutSec": 600, "conta": false,
      "instalar": ["node"], "montar": ["projeto:ro", "deploy:ro", "trabalho:rw"]
    },
    "app-winapp": {
      "$comment": "App desktop controlado pelo winapp; rede ligada, internet bloqueada por sandbox\\app\\guest-firewall.ps1 (wsb exec -r System). Antes: publicar.mjs (so o app). Dados: cenario-no-guest.mjs + guest-run.ps1 -Cenario.",
      "rede": true, "memoriaMB": 6144, "bootTimeoutSec": 300, "instalarTimeoutSec": 600, "conta": false,
      "instalar": ["node"], "montar": ["projeto:ro", "deploy:ro", "trabalho:rw"]
    },
    "cache-msvc": {
      "$comment": "So para montar o layout offline do MSVC (adendo 8.2): wsb exec -r System C:\\labsandbox\\layout-msvc.ps1; depois scripts\\cache-up.ps1 -ImportarMsvc <id>. Rede ligada (baixa do servidor da Microsoft).",
      "rede": true, "memoriaMB": 6144, "bootTimeoutSec": 300, "conta": false,
      "instalar": [], "montar": ["projeto:ro", "cache-saida:rw"]
    },
    "testes": {
      "$comment": "Testes de Sandbox (testes-no-guest.mjs), cargo test e Claude Code com a conta logada no PC. Internet aberta (cargo e npm).",
      "rede": true, "memoriaMB": 8192, "bootTimeoutSec": 300, "instalarTimeoutSec": 3600, "conta": true,
      "instalar": ["msvc", "rust", "node", "claude-code"], "montar": ["projeto:ro", "trabalho:rw"]
    }
  },
  "pacotes": {
    "msvc": {
      "tipo": "vs-layout",
      "versao": "<17.14.N do Passo 1>",
      "arquivo": "msvc-<17.14.N do Passo 1>",
      "componentes": [
        "Microsoft.VisualStudio.Workload.VCTools",
        "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        "Microsoft.VisualStudio.Component.Windows11SDK.26100"
      ],
      "bootstrapper": { "arquivo": "vs_buildtools-<17.14.N do Passo 1>.exe", "url": "<link de versão fixa do Passo 1>", "sha256": "<sha256 do Passo 1>", "bytes": 0 }
    },
    "rust": {
      "tipo": "msi", "versao": "1.95.0",
      "arquivo": "rust-1.95.0-x86_64-pc-windows-msvc.msi",
      "url": "https://static.rust-lang.org/dist/rust-1.95.0-x86_64-pc-windows-msvc.msi",
      "sha256": "08d228850565cfe421da6faeac029fb1f0c8341219d0dd9490d42a8355aa0703",
      "bytes": 303775340
    },
    "node": {
      "tipo": "zip", "versao": "22.23.3",
      "arquivo": "node-v22.23.3-win-x64.zip",
      "url": "https://nodejs.org/dist/v22.23.3/node-v22.23.3-win-x64.zip",
      "sha256": "2b0ff57b049cda1bbcea2240eec20467018713c1efe1f7360c2681859b90ed71",
      "bytes": 35574076
    },
    "claude-code": {
      "tipo": "exe", "versao": "2.1.285",
      "arquivo": "claude-2.1.285-win32-x64.exe",
      "url": "https://downloads.claude.ai/claude-code-releases/2.1.285/win32-x64/claude.exe",
      "sha256": "121fc8151ed40bd9c144d68aa1cea23427803628ffab65e23da1cceda155697e",
      "bytes": 243751072
    }
  }
}
```

Os três `<...>` do `msvc` são substituídos pelos valores reais do Passo 1 (e `bytes` pelo tamanho medido) **antes** do commit; o teste "versioned compose.json passes" falha enquanto houver `<` no arquivo (o `versao` não casa com dígitos e pontos).

- [ ] **Passo 6: Rodar e ver passar** com os dois PowerShell:
  `pwsh -NoProfile -File sandbox-lab/scripts/test/compose.test.ps1` e `powershell -NoProfile -ExecutionPolicy Bypass -File sandbox-lab/scripts/test/compose.test.ps1` → `all passed`, exit 0 nos dois.

- [ ] **Passo 7: Provas de mutação** (uma por regra; colar a linha `FAILED` de cada uma no relatório, restaurar, rodar de novo):
  1. Em `Get-LabComposeStringProblem`, trocar a linha do `host absolute path` por `if ($false) { ... }` → `FAILED - strings: host paths, variables, UNC, home and .. are refused anywhere`.
  2. Remover a linha do `%...%` → mesmo teste falha.
  3. Em `ConvertTo-LabGuestInstall`, trocar `foreach ($nome in $script:LabInstallOrder)` por `foreach ($nome in $wanted)` → `FAILED - install order is fixed ...`.
  4. Em `Get-LabComposeRefusal`, apagar a verificação `must include projeto:ro` → `FAILED - montar: ...`.
  5. Em `Invoke-LabStartWithCredential`, trocar o `catch` por `catch { throw }` → `FAILED - start with credential ...`.
  6. Em `Get-LabUpVerdict`, apagar a checagem de `$DiscoZ` nulo → `FAILED - up verdict ...`.

- [ ] **Passo 8: Commit.**
```bash
git add sandbox-lab/compose.json sandbox-lab/scripts/lib/compose.ps1 sandbox-lab/scripts/test/compose.test.ps1
git commit -m "feat(lab): compose.json with named environments and pure compose checks

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019iZMxAmFGsgenP4pjSBmb6"
```

---

### Tarefa 2: Travas puras novas (Z:, credencial, cache), funções de disco da credencial e A7

**Modelo:** Sonnet.

**Files:**
- Modify: `sandbox-lab/scripts/lib/travas.ps1` (acrescentar no fim)
- Modify: `sandbox-lab/scripts/lib/disco.ps1` (acrescentar no fim)
- Test: `sandbox-lab/scripts/test/travas.test.ps1` (acrescentar antes do resumo final)
- Modify: `sandbox-lab/scripts/test/travas.sandbox.ps1` (cabeçalho, remover caso `Q:\`, testes da credencial)

**Interfaces:**
- Consumes: funções existentes `ConvertTo-LabPathKey`, `Get-LabRootProblem`, `Test-LabShortNameSegment`, `Get-LabLinksInPath`, `Get-LabLinksAboveRoot`, `Test-LabInSandbox`.
- Produces: as funções de `travas.ps1` e `disco.ps1` da tabela do contrato.

- [ ] **Passo 1: Escrever os testes puros que falham** — acrescentar a `travas.test.ps1`, antes de `if ($script:failed)`:

```powershell
$UP = 'Y:\Perfis\fulano'
$RR = 'Y:\proj x\claudia-rh'
$CD = 'Y:\proj x\claudia-rh\.sandbox-work\credencial'
$CF = "$CD\.credentials.json"
$G32 = '0123456789abcdef0123456789abcdef'

Check 'guest test root literal is pinned (the Node side pins the same literal)' {
  if ($script:LabGuestTestRoot -cne 'Z:\claudia-teste-sandbox') { throw "root changed: '$script:LabGuestTestRoot'" }
}
Check 'guest test root: the root and anything under it are allowed' {
  foreach ($p in @('Z:\claudia-teste-sandbox', 'z:\CLAUDIA-TESTE-SANDBOX\cs test a 1', 'Z:\claudia-teste-sandbox\a\b', 'Z:\claudia-teste-sandbox\')) { Should-BeNull (Get-LabGuestTestRootRefusal $p) }
}
Check 'guest test root: other drives, siblings, dotted, relative and empty are refused' {
  foreach ($p in @('C:\labdata\x', 'Z:\', 'Z:\tmp\x', 'Z:\claudia-teste-sandbox-irmao\x', 'Z:\claudia-teste-sandbox\..\x', 'Z:\claudia-teste-sandbox\.\x', '', 'claudia-teste-sandbox\x', '\\srv\s\claudia-teste-sandbox')) {
    if (-not (Get-LabGuestTestRootRefusal $p)) { throw "must refuse '$p'" }
  }
}
Check 'host drives: a Z: drive on the host refuses test-up' {
  Should-BeNull (Get-LabHostDriveRefusal -DriveNames @('C:\', 'D:\', 'G:\'))
  Should-BeNull (Get-LabHostDriveRefusal -DriveNames @())
  Should-Match (Get-LabHostDriveRefusal -DriveNames @('C:\', 'Z:\')) 'Z:'
  Should-Match (Get-LabHostDriveRefusal -DriveNames @('z:\')) 'Z:'
}
Check 'credential source: only the default login file of the user folder' {
  Should-BeNull (Get-LabCredentialSourceRefusal -Source "$UP\.claude\.credentials.json" -UserProfile $UP -ConfigDir '')
  Should-BeNull (Get-LabCredentialSourceRefusal -Source "Y:\PERFIS\FULANO\.claude\.credentials.json" -UserProfile "$UP\" -ConfigDir $null)
}
Check 'credential source: CLAUDE_CONFIG_DIR set is refused' {
  Should-Match (Get-LabCredentialSourceRefusal -Source "$UP\.claude\.credentials.json" -UserProfile $UP -ConfigDir 'Y:\outra') 'CLAUDE_CONFIG_DIR'
}
Check 'credential source: other files, empty values, drive-root profile, 8.3 names and links are refused' {
  foreach ($s in @("$UP\.claude\settings.json", "$UP\.claude", "$UP\.claude-work\.credentials.json", 'Y:\outro\.claude\.credentials.json', '')) {
    if (-not (Get-LabCredentialSourceRefusal -Source $s -UserProfile $UP -ConfigDir '')) { throw "must refuse '$s'" }
  }
  Should-Match (Get-LabCredentialSourceRefusal -Source 'Y:\.claude\.credentials.json' -UserProfile 'Y:\' -ConfigDir '') 'user folder'
  Should-Match (Get-LabCredentialSourceRefusal -Source 'Y:\Perfis\FULANO~1\.claude\.credentials.json' -UserProfile 'Y:\Perfis\FULANO~1' -ConfigDir '') '8\.3'
  Should-Match (Get-LabCredentialSourceRefusal -Source "$UP\.claude\.credentials.json" -UserProfile $UP -ConfigDir '' -Links @("$UP\.claude")) 'link'
}
Check 'credential paths: derived only from the repo root' {
  $p = Get-LabCredentialPaths "$RR\"
  if ($p.Dir -cne $CD -or $p.File -cne $CF) { throw "got $($p.Dir) / $($p.File)" }
}
Check 'credential clean: only the exact copy or its .tmp sibling' {
  Should-BeNull (Get-LabCredentialCleanRefusal -File $CF -RepoRoot $RR)
  Should-BeNull (Get-LabCredentialCleanRefusal -File $CF.ToUpper() -RepoRoot $RR)
  Should-BeNull (Get-LabCredentialCleanRefusal -File "$CF.$G32.tmp" -RepoRoot $RR)
  foreach ($f in @($CD, "$CD\x.json", "$CF.abc.tmp", "$CF.$G32.tmp.x", "$RR\.sandbox-work\deploy\.credentials.json", 'Y:\', $RR, "$CF\..", ($CF -replace '\\', '/'), "$UP\.claude\.credentials.json")) {
    if (-not (Get-LabCredentialCleanRefusal -File $f -RepoRoot $RR)) { throw "must refuse '$f'" }
  }
  Should-Match (Get-LabCredentialCleanRefusal -File $CF -RepoRoot '') 'required'
  Should-Match (Get-LabCredentialCleanRefusal -File 'Y:\.sandbox-work\credencial\.credentials.json' -RepoRoot 'Y:\') 'repo root'
  Should-Match (Get-LabCredentialCleanRefusal -File $CF -RepoRoot $RR -Links @("$RR\.sandbox-work")) 'link'
}
Check 'cache dir: only the exact shared\cache, no links' {
  $Lab = "$RR\sandbox-lab"
  Should-BeNull (Get-LabCacheDirRefusal -CacheDir "$Lab\shared\cache" -LabDir $Lab)
  foreach ($c in @("$Lab\shared", "$Lab\shared\cache\x", 'Y:\', "$Lab\shared\out", ("$Lab\shared\cache" -replace '\\', '/'))) {
    Should-Match (Get-LabCacheDirRefusal -CacheDir $c -LabDir $Lab) 'not exactly'
  }
  Should-Match (Get-LabCacheDirRefusal -CacheDir "$Lab\shared\cache" -LabDir $Lab -Links @("$Lab\shared")) 'link'
  Should-Match (Get-LabCacheDirRefusal -CacheDir 'Y:\shared\cache' -LabDir 'Y:\') 'lab dir'
}
Check 'cache plan: missing -> baixar, same hash -> ok, other hash or wrong kind -> conflito' {
  $h1 = 'a' * 64; $h2 = 'b' * 64
  $pk = @(
    [pscustomobject]@{ nome = 'rust'; arquivo = 'rust.msi'; sha256 = $h1 },
    [pscustomobject]@{ nome = 'node'; arquivo = 'node.zip'; sha256 = $h1 },
    [pscustomobject]@{ nome = 'claude-code'; arquivo = 'claude.exe'; sha256 = $h1 },
    [pscustomobject]@{ nome = 'msvc'; arquivo = 'msvc-17'; sha256 = '' }
  )
  $plan = @(Get-LabCachePlan -Pacotes $pk -Existentes @{ 'RUST.msi' = $h1; 'node.zip' = $h2; 'msvc-17' = 'dir' })
  $acts = @($plan | ForEach-Object { "$($_.nome)=$($_.acao)" }) -join ','
  if ($acts -ne 'rust=ok,node=conflito,claude-code=baixar,msvc=ok') { throw "got $acts" }
  $plan2 = @(Get-LabCachePlan -Pacotes @($pk[3], $pk[0]) -Existentes @{ 'msvc-17' = $h1; 'rust.msi' = 'dir' })
  if ((@($plan2 | ForEach-Object { $_.acao }) -join ',') -ne 'conflito,conflito') { throw 'wrong kind must be conflito' }
}
$RO = 'Y:\Perfis\fulano\AppData\Roaming'
Check 'real data: only the app folder under Roaming (spec addendum 8.1)' {
  Should-BeNull (Get-LabRealDataSourceRefusal -Source "$RO\io.github.johngabie.claudia-rh" -RoamingAppData $RO)
  Should-BeNull (Get-LabRealDataSourceRefusal -Source "$RO\IO.GITHUB.JOHNGABIE.CLAUDIA-RH" -RoamingAppData "$RO\")
}
Check 'real data: LOCALAPPDATA, other folders, the Roaming root, empty, 8.3 and links are refused' {
  foreach ($s in @('Y:\Perfis\fulano\AppData\Local\io.github.johngabie.claudia-rh', $RO, "$RO\outro", "$RO\io.github.johngabie.claudia-rh\x", "$RO\io.github.johngabie.claudia-rh2", '')) {
    if (-not (Get-LabRealDataSourceRefusal -Source $s -RoamingAppData $RO)) { throw "must refuse '$s'" }
  }
  Should-Match (Get-LabRealDataSourceRefusal -Source 'Y:\P\AppData\Local\io.github.johngabie.claudia-rh' -RoamingAppData 'Y:\P\AppData\Local') 'LOCALAPPDATA'
  Should-Match (Get-LabRealDataSourceRefusal -Source 'Y:\io.github.johngabie.claudia-rh' -RoamingAppData 'Y:\') 'roaming'
  Should-Match (Get-LabRealDataSourceRefusal -Source 'Y:\PERFIS~1\io.github.johngabie.claudia-rh' -RoamingAppData 'Y:\PERFIS~1') '8\.3'
  Should-Match (Get-LabRealDataSourceRefusal -Source "$RO\io.github.johngabie.claudia-rh" -RoamingAppData $RO -Links @($RO)) 'link'
}
Check 'layout import: only <repo>\.sandbox-work\cache-saida\<8 hex>\msvc-<versao>' {
  $ok = "$RR\.sandbox-work\cache-saida\0a1b2c3d\msvc-17.14.1"
  Should-BeNull (Get-LabLayoutImportRefusal -Source $ok -RepoRoot $RR -Versao '17.14.1')
  foreach ($s in @("$RR\.sandbox-work\cache-saida\0a1b2c3d", "$RR\.sandbox-work\cache-saida\0A1B2C3X\msvc-17.14.1", "$RR\.sandbox-work\cache-saida\0a1b2c3d\msvc-17.14.2",
                   "$RR\.sandbox-work\cache-saida\0a1b2c3d\msvc-17.14.1\layout", "$RR\sandbox-lab\shared\cache\msvc-17.14.1", 'Y:\', '')) {
    if (-not (Get-LabLayoutImportRefusal -Source $s -RepoRoot $RR -Versao '17.14.1')) { throw "must refuse '$s'" }
  }
  Should-Match (Get-LabLayoutImportRefusal -Source $ok -RepoRoot $RR -Versao 'latest') 'versao'
  Should-Match (Get-LabLayoutImportRefusal -Source $ok -RepoRoot $RR -Versao '17.14.1' -Links @("$RR\.sandbox-work")) 'link'
}
Check 'layout manifest: same files, sizes and hashes pass; missing, extra, different or malformed fail' {
  $a = 'a' * 64; $b = 'b' * 64
  $man = "$a 10 vs_setup.exe`n$b 20 Archives/x.cab`n"
  if (@(Get-LabLayoutManifestProblems -ManifestText $man -Found @{ 'vs_setup.exe' = "$a 10"; 'archives/X.cab' = "$b 20" }).Count) { throw 'identical must pass' }
  $p = @(Get-LabLayoutManifestProblems -ManifestText $man -Found @{ 'vs_setup.exe' = "$a 11"; 'extra.txt' = "$a 1" })
  $txt = $p -join ';'
  if ($txt -notmatch 'differs: vs_setup\.exe' -or $txt -notmatch 'missing: Archives/x\.cab' -or $txt -notmatch 'not in the manifest: extra\.txt') { throw "got $txt" }
  Should-Match ((Get-LabLayoutManifestProblems -ManifestText '' -Found @{}) -join ';') 'empty'
  Should-Match ((Get-LabLayoutManifestProblems -ManifestText "lixo`n" -Found @{}) -join ';') 'malformed'
}
```

E, no teste existente `'source: drive roots, empty and relative values are refused'`, acrescentar `'Q:\'` à lista (A7: a raiz de disco fica coberta aqui, na função pura).

- [ ] **Passo 2: Rodar e ver falhar.** `pwsh -NoProfile -File sandbox-lab/scripts/test/travas.test.ps1` → linhas `FAILED - guest test root ...` etc. (funções inexistentes), exit 1.

- [ ] **Passo 3: Implementar em `travas.ps1`** (acrescentar no fim; puro, sem `[IO.Path]`):

```powershell
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
  return ,$plan
}

$script:LabAppId = 'io.github.johngabie.claudia-rh'

function Get-LabRealDataSourceRefusal {
  # test-up -DadosReais (spec addendum 8.1): only the app's folder under Roaming, read-only, never %LOCALAPPDATA%.
  # Callers pass [Environment]::GetFolderPath('ApplicationData') and the reparse points above the source.
  param([string]$Source, [string]$RoamingAppData, [string[]]$Links = @())
  if ([string]::IsNullOrWhiteSpace($Source) -or [string]::IsNullOrWhiteSpace($RoamingAppData)) { return 'source and roaming folder are required' }
  $r = $RoamingAppData.TrimEnd('\')
  $why = Get-LabRootProblem $r
  if ($why) { return "roaming folder $why" }
  if ($r -match '\\Local$' -or $Source -match '\\AppData\\Local(\\|$)') { return 'never %LOCALAPPDATA% (browser profile, cookies)' }
  if (Test-LabShortNameSegment $Source) { return 'source has an 8.3 short name segment' }
  if ((ConvertTo-LabPathKey $Source) -ne (ConvertTo-LabPathKey "$r\$script:LabAppId")) { return "source is not exactly %APPDATA%\$script:LabAppId" }
  $l = @($Links | Where-Object { $_ })
  if ($l.Count) { return 'link (symlink/junction) in the path of the app data folder' }
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
  return ,$problems.ToArray()
}
```

- [ ] **Passo 4: Implementar em `disco.ps1`** (acrescentar no fim):

```powershell
function Remove-LabCredentialFile {
  # Deletes ONE file, only after Get-LabCredentialCleanRefusal says yes. Never -Recurse.
  param([string]$File, [string]$RepoRoot)
  $p = Get-LabCredentialPaths $RepoRoot
  $links = @(Get-LabLinksInPath $p.Dir $RepoRoot)
  $item = Get-Item -LiteralPath $File -Force -ErrorAction SilentlyContinue
  if ($item -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { $links += $File }
  if ($item -and $item.PSIsContainer) { throw "[credential] refusing to delete a folder: $File" }
  $refusal = Get-LabCredentialCleanRefusal -File $File -RepoRoot $RepoRoot -Links $links
  if ($refusal) { throw "[credential] refusing to delete ${File}: $refusal" }
  if ($item) { Remove-Item -LiteralPath $File -Force }
}

function Remove-LabCredential {
  # test-down / failed test-up: the copy and any .tmp left by an interrupted copy. Nothing else in the folder.
  param([string]$RepoRoot)
  $p = Get-LabCredentialPaths $RepoRoot
  if (-not (Test-Path -LiteralPath $p.Dir -PathType Container)) { return }
  Remove-LabCredentialFile -File $p.File -RepoRoot $RepoRoot
  foreach ($f in @(Get-ChildItem -LiteralPath $p.Dir -Force -File -Filter '.credentials.json.*.tmp')) {
    Remove-LabCredentialFile -File $f.FullName -RepoRoot $RepoRoot
  }
}

function Copy-LabCredential {
  # Copies the login of the account signed in on this PC (spec A4) into .sandbox-work\credencial. The bytes are never
  # read into a variable, printed or logged; errors never carry the user folder.
  param([string]$RepoRoot, [string]$UserProfile, [string]$ConfigDir)
  $src = Join-Path $UserProfile '.claude\.credentials.json'
  $refusal = Get-LabCredentialSourceRefusal -Source $src -UserProfile $UserProfile -ConfigDir $ConfigDir -Links (Get-LabLinksAboveRoot $src)
  if ($refusal) { throw "[credential] refused: $refusal" }
  if (-not (Test-Path -LiteralPath $src -PathType Leaf)) { throw '[credential] no account is signed in on this PC (%USERPROFILE%\.claude\.credentials.json is missing): run claude and /login, then test-up again' }
  $p = Get-LabCredentialPaths $RepoRoot
  New-Item -ItemType Directory -Force -Path $p.Dir | Out-Null
  $links = @(Get-LabLinksInPath $p.Dir $RepoRoot)
  if ($links.Count) { throw "[credential] refused: link (symlink/junction) in $($links[0])" }
  $tmp = "$($p.Dir)\.credentials.json.$([guid]::NewGuid().ToString('N')).tmp"
  try {
    try {
      [IO.File]::Copy($src, $tmp)
      if (Test-Path -LiteralPath $p.File) { [IO.File]::Replace($tmp, $p.File, $null) } else { [IO.File]::Move($tmp, $p.File) }
    } catch { throw '[credential] copy failed (the login file could not be read or written)' }
  } finally {
    if (Test-Path -LiteralPath $tmp) { Remove-LabCredentialFile -File $tmp -RepoRoot $RepoRoot }
  }
}
```

- [ ] **Passo 5: Rodar os testes puros** com os dois PowerShell → `all passed`, exit 0.

- [ ] **Passo 6: `travas.sandbox.ps1`** (só roda no guest; aqui só editar e conferir a sintaxe):
  - Trocar o bloco que define `$Base` (linhas 10-11 atuais) por:
    ```powershell
    # Spec A6: everything this test creates lives on the guest-only Z: drive (sandbox\disco-z.ps1), never on C:.
    $RootReal = $null
    try { New-Item -ItemType Directory -Force -Path $script:LabGuestTestRoot | Out-Null; $RootReal = (Get-Item -LiteralPath $script:LabGuestTestRoot -Force).FullName } catch { }
    $why = $(if ($RootReal) { Get-LabGuestTestRootRefusal $RootReal } else { 'the Z: test root could not be created (is Z: mounted? see C:\lab\out\disco-z.json)' })
    if (-not $why -and @(Get-LabLinksAboveRoot $RootReal).Count) { $why = 'link (junction/symlink) above the test root' }
    if ($why) { Write-Host "REFUSED: $why"; exit 3 }
    $Base = Join-Path $RootReal ('travas-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $Base | Out-Null
    ```
    e atualizar o comentário do topo (`C:\labdata` → `Z:\claudia-teste-sandbox`).
  - **A7:** apagar inteiro o `Check 'a source that is a drive root is refused, nothing copied'` (o que passa `'Q:\'`).
  - Acrescentar antes do resumo final:
    ```powershell
    function New-FakeLogin([string]$Name) {
      $up = Join-Path $Base $Name
      New-Item -ItemType Directory -Force (Join-Path $up '.claude') | Out-Null
      [IO.File]::WriteAllText((Join-Path $up '.claude\.credentials.json'), 'fake-login-for-test')
      return $up
    }
    Check 'the test base is on Z: and its real path is under the guest test root' {
      if (Get-LabGuestTestRootRefusal (Get-Item -LiteralPath $Base).FullName) { throw 'base not under Z:\claudia-teste-sandbox' }
    }
    Check 'credential: copy creates exactly .sandbox-work\credencial\.credentials.json, same bytes, no .tmp left' {
      $r = Join-Path $Base 'cred-a'; $up = New-FakeLogin 'perfil-a'
      Copy-LabCredential -RepoRoot $r -UserProfile $up -ConfigDir ''
      $p = Get-LabCredentialPaths $r
      if ([IO.File]::ReadAllText($p.File) -ne 'fake-login-for-test') { throw 'content differs' }
      if (@(Get-ChildItem -LiteralPath $p.Dir -Force).Count -ne 1) { throw 'extra files in credencial' }
    }
    Check 'credential: a second copy replaces the first' {
      $r = Join-Path $Base 'cred-b'; $up = New-FakeLogin 'perfil-b'
      Copy-LabCredential -RepoRoot $r -UserProfile $up -ConfigDir ''
      [IO.File]::WriteAllText((Join-Path $up '.claude\.credentials.json'), 'second')
      Copy-LabCredential -RepoRoot $r -UserProfile $up -ConfigDir ''
      if ([IO.File]::ReadAllText((Get-LabCredentialPaths $r).File) -ne 'second') { throw 'not replaced' }
    }
    Check 'credential: CLAUDE_CONFIG_DIR set refuses and copies nothing' {
      $r = Join-Path $Base 'cred-c'; $up = New-FakeLogin 'perfil-c'
      Should-Refuse { Copy-LabCredential -RepoRoot $r -UserProfile $up -ConfigDir 'Z:\outra' }
      if (Test-Path -LiteralPath (Get-LabCredentialPaths $r).File) { throw 'copied anyway' }
    }
    Check 'credential: no login on the PC refuses with the /login hint' {
      $r = Join-Path $Base 'cred-d'; $up = Join-Path $Base 'perfil-vazio'; New-Item -ItemType Directory -Force $up | Out-Null
      $msg = $null; try { Copy-LabCredential -RepoRoot $r -UserProfile $up -ConfigDir '' } catch { $msg = $_.Exception.Message }
      if ("$msg" -notmatch '/login') { throw "got: $msg" }
    }
    Check 'credential: remove deletes only that file; a sentinel in the same folder is kept' {
      $r = Join-Path $Base 'cred-e'; $up = New-FakeLogin 'perfil-e'
      Copy-LabCredential -RepoRoot $r -UserProfile $up -ConfigDir ''
      $p = Get-LabCredentialPaths $r; $keep = Join-Path $p.Dir 'keep.txt'; [IO.File]::WriteAllText($keep, 'keep')
      Remove-LabCredential -RepoRoot $r
      if (Test-Path -LiteralPath $p.File) { throw 'copy survived' }
      if (-not (Test-Path -LiteralPath $keep)) { throw 'sentinel deleted' }
      if (-not (Test-Path -LiteralPath (Join-Path $up '.claude\.credentials.json'))) { throw 'source deleted' }
    }
    Check 'credential: a stray .tmp is removed, any other file is refused and kept' {
      $r = Join-Path $Base 'cred-f'; $p = Get-LabCredentialPaths $r; New-Item -ItemType Directory -Force $p.Dir | Out-Null
      $tmp = "$($p.File).0123456789abcdef0123456789abcdef.tmp"; [IO.File]::WriteAllText($tmp, 'x')
      Remove-LabCredential -RepoRoot $r
      if (Test-Path -LiteralPath $tmp) { throw '.tmp survived' }
      $other = Join-Path $p.Dir 'outro.json'; [IO.File]::WriteAllText($other, 'x')
      Should-Refuse { Remove-LabCredentialFile -File $other -RepoRoot $r }
      if (-not (Test-Path -LiteralPath $other)) { throw 'other file deleted' }
    }
    Check 'credential: credencial folder as a junction is refused, target kept' {
      $r = Join-Path $Base 'cred-g'; New-Item -ItemType Directory -Force (Join-Path $r '.sandbox-work') | Out-Null
      $target = Join-Path $Base 'alvo-cred'; $keep = New-Sentinel $target
      [IO.File]::WriteAllText((Join-Path $target '.credentials.json'), 'x')
      New-Item -ItemType Junction -Path (Get-LabCredentialPaths $r).Dir -Target $target | Out-Null
      Should-Refuse { Remove-LabCredential -RepoRoot $r }
      Should-Refuse { Copy-LabCredential -RepoRoot $r -UserProfile (New-FakeLogin 'perfil-g') -ConfigDir '' }
      if (-not (Test-Path -LiteralPath $keep) -or -not (Test-Path -LiteralPath (Join-Path $target '.credentials.json'))) { throw 'target changed' }
    }
    ```
  - Conferir sintaxe sem rodar: `pwsh -NoProfile -Command "$e=$null; [void][Management.Automation.Language.Parser]::ParseFile((Resolve-Path 'sandbox-lab/scripts/test/travas.sandbox.ps1'), [ref]$null, [ref]$e); $e.Count"` → `0`. Repetir com `powershell` (5.1).
  - **Não rodar** `travas.sandbox.ps1` no PC (ele recusa com exit 3, mas a regra é não rodar). Ele roda na Tarefa 12.

- [ ] **Passo 7: Provas de mutação** (colar cada `FAILED`, restaurar):
  1. `Get-LabHostDriveRefusal`: trocar `'^[Zz]:'` por `'^[Yy]:'` → `FAILED - host drives: ...`.
  2. `Get-LabGuestTestRootRefusal`: trocar `StartsWith("$r\", ...)` por `StartsWith($r, ...)` → `FAILED - guest test root: other drives, siblings, ...` (o irmão `-irmao` passa a ser aceito).
  3. `Get-LabCredentialSourceRefusal`: apagar a linha do `ConfigDir` → `FAILED - credential source: CLAUDE_CONFIG_DIR set is refused`.
  4. `Get-LabCredentialCleanRefusal`: trocar `{32}` por `+` → `FAILED - credential clean: ...` (`.abc.tmp` aceito).
  5. `Get-LabCachePlan`: trocar o `else { 'conflito' ... }` final por `$acao = 'baixar'` → `FAILED - cache plan: ...`.
  6. `Get-LabRealDataSourceRefusal`: apagar a linha do `LOCALAPPDATA` → `FAILED - real data: LOCALAPPDATA, ...`.
  7. `Get-LabLayoutImportRefusal`: trocar `{8}` por `+` → `FAILED - layout import: ...`.
  8. `Get-LabLayoutManifestProblems`: apagar o laço do `not in the manifest` → `FAILED - layout manifest: ...`.

- [ ] **Passo 8: Commit.**
```bash
git add sandbox-lab/scripts/lib/travas.ps1 sandbox-lab/scripts/lib/disco.ps1 sandbox-lab/scripts/test/travas.test.ps1 sandbox-lab/scripts/test/travas.sandbox.ps1
git commit -m "feat(lab): pure guards for the Z: test root, host Z: drive, Claude credential copy, cache dir, real app data mount and MSVC layout import

Drops the Q:\ case from travas.sandbox.ps1: drive roots are covered by the pure test only.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019iZMxAmFGsgenP4pjSBmb6"
```

---

### Tarefa 3: Detector de token, cache e credencial proibidos no git

**Modelo:** Sonnet.

**Files:**
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/proibidos.mjs`
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs`
- Create: `.claude/skills/claudia-teste-sandbox/scripts/conferir-token.mjs`
- Test: `.claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs`
- Modify: `.gitignore`, `sandbox-lab/.gitignore`

**Interfaces:**
- Consumes: `arquivosProibidos`, `textoParaConferir`, `achaCaminhoPessoal` (existentes); `falhaDe` de `comum.mjs`.
- Produces: `achaToken(texto) → Array<{ linha: number }>`; CLI `conferir-token.mjs --pasta <p> [--pasta <p>]` → última linha JSON `{arquivos, achados:[{arquivo, linha}], pulados}`, exit 1 se houver achado.

- [ ] **Passo 1: Testes que falham** — acrescentar a `proibidos.test.mjs` (e `achaToken` ao `import`):

```js
// Fake tokens are assembled at run time, so this source file never holds a token-shaped string.
const falso = (tipo) => ["sk", "ant", tipo, "A1b2C3d4E5f6G7h8I9j0K1l2M3n4"].join("-");

test("achaToken: finds OAuth access, refresh and API key shapes by line, never returns the text", () => {
  const texto = ["linha ok", `token=${falso("oat01")}`, "x", `"${falso("ort01")}"`, `ANTHROPIC_API_KEY=${falso("api03")}`].join("\n");
  const achados = achaToken(texto);
  assert.deepEqual(achados, [{ linha: 2 }, { linha: 4 }, { linha: 5 }]);
  assert.equal(JSON.stringify(achados).includes("A1b2C3"), false);
});

test("achaToken: finds a credential JSON with a long access or refresh token", () => {
  const chave = ["access", "Token"].join("");
  assert.deepEqual(achaToken(`{"${chave}": "${"x".repeat(30)}"}`), [{ linha: 1 }]);
});

test("achaToken: ignores the bare prefix, short values, empty tokens and ordinary text", () => {
  const chave = ["refresh", "Token"].join("");
  for (const t of ["sk-ant-", `sk-ant-oat01-curto`, `{"${chave}": ""}`, `{"${chave}": "abc"}`, "a skate-ant 01", ""]) {
    assert.deepEqual(achaToken(t), [], t);
  }
});

test("flags the lab installer cache and any Claude credential file", () => {
  const lista = ["sandbox-lab/shared/cache/rust.msi", ".credentials.json", "x/.claude/.credentials.json", ".sandbox-work/credencial/.credentials.json"];
  assert.deepEqual(nomes(lista), lista);
  assert.deepEqual(nomes(["docs/credentials.md", "x/credentials.json.example"]), []);
});
```

- [ ] **Passo 2: Rodar e ver falhar.** `node --test .claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs` → `achaToken` não exportado, exit ≠ 0.

- [ ] **Passo 3: Implementar em `proibidos.mjs`:**
  - Em `PASTAS_PROIBIDAS`, acrescentar `["sandbox-lab/shared/cache/", "lab installer cache (ignored; downloaded by cache-up.ps1)"]`.
  - Em `arquivosProibidos`, depois da regra de `.key`: `if (!motivo && (a === ".credentials.json" || a.endsWith("/.credentials.json"))) motivo = "Claude Code login file";`
  - Acrescentar:
    ```js
    // Anthropic token shapes (OAuth access "oat", refresh "ort", API key "api") and a credential JSON with a long token.
    // Findings carry only the line number: the matched text is a secret and must never be printed.
    const PADROES_TOKEN = [
      /sk-ant-[a-z]{2,8}\d{2}-[A-Za-z0-9_-]{20,}/,
      /"(?:accessToken|refreshToken)"\s*:\s*"[^"\s]{20,}"/,
    ];

    /** One finding per line that holds a token: { linha } (1-based). Never returns the text. */
    export function achaToken(texto) {
      const achados = [];
      String(texto ?? "").split(/\r?\n/).forEach((l, i) => {
        if (PADROES_TOKEN.some((p) => p.test(l))) achados.push({ linha: i + 1 });
      });
      return achados;
    }
    ```

- [ ] **Passo 4: `conferir-repo.mjs`:** importar `achaToken`; num laço sobre **todos** os arquivos versionados (não só `textoParaConferir`), pular os que não existem, ler como `latin1` (binários também: um token num `.sqlite` aparece em ASCII) e juntar `{ arquivo, linha }`; imprimir `TOKEN <arquivo>:<linha>` (sem o texto); incluir `tokens` no JSON final e no exit 1. Atualizar o comentário do topo.

- [ ] **Passo 5: Criar `conferir-token.mjs`:**

```js
// Read-only scan of folders for Anthropic tokens (spec A4/A5). Prints only file and line, never the matched text.
// Usage: node .claude/skills/claudia-teste-sandbox/scripts/conferir-token.mjs --pasta <folder> [--pasta <folder>]
// Does not follow links. Never writes, moves or deletes anything.
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "node:util";
import { achaToken } from "./proibidos.mjs";
import { falhaDe } from "./comum.mjs";

const LIMITE = 256 * 1024 * 1024;

function* arquivos(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isSymbolicLink()) continue; // junctions are reported as links on Windows
    if (e.isDirectory()) yield* arquivos(p);
    else if (e.isFile()) yield p;
  }
}

try {
  const { values } = parseArgs({ options: { pasta: { type: "string", multiple: true } } });
  if (!values.pasta?.length) throw Object.assign(new Error("Usage: node conferir-token.mjs --pasta <folder> [--pasta <folder>]"), { code: "ERR_PARSE_ARGS_USAGE" });
  let total = 0;
  const achados = [];
  const pulados = [];
  for (const pasta of values.pasta) {
    const raiz = path.resolve(pasta);
    if (!fs.existsSync(raiz)) continue;
    for (const f of arquivos(raiz)) {
      total++;
      if (fs.statSync(f).size > LIMITE) { pulados.push(f); continue; }
      for (const a of achaToken(fs.readFileSync(f, "latin1"))) achados.push({ arquivo: f, linha: a.linha });
    }
  }
  for (const a of achados) console.error(`TOKEN ${a.arquivo}:${a.linha}`);
  console.log(JSON.stringify({ arquivos: total, achados, pulados }));
  process.exit(achados.length ? 1 : 0);
} catch (e) { falhaDe(e); }
```

- [ ] **Passo 6: `.gitignore` (raiz):** depois de `sandbox-lab/shared/out/`, acrescentar `sandbox-lab/shared/cache/`. `sandbox-lab/.gitignore`: acrescentar `shared/cache/`.

- [ ] **Passo 7: Rodar e ver passar.**
  `node --test .claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs` → exit 0.
  `node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs` → `"tokens":0`, exit 0 (só lê via `git ls-files`). Se apontar `TOKEN` num arquivo já versionado: **não** abrir nem imprimir a linha, **não** criar exceção na regra; parar e relatar ao orquestrador o arquivo e o número da linha (pode ser segredo real no git, o que vira assunto do usuário).
  `git check-ignore -v sandbox-lab/shared/cache/x.msi` → mostra as duas regras.

- [ ] **Passo 8: Provas de mutação:**
  1. Trocar `\d{2}` por `\d{3}` no primeiro padrão → `FAILED` em `achaToken: finds OAuth access, ...`.
  2. Fazer `achaToken` devolver `{ linha: i + 1, trecho: l }` → mesmo teste falha (a asserção `includes("A1b2C3")`).
  3. Apagar a regra do `.credentials.json` → `FAILED - flags the lab installer cache and any Claude credential file`.

- [ ] **Passo 9: Commit.**
```bash
git add .claude/skills/claudia-teste-sandbox/scripts/proibidos.mjs .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs .claude/skills/claudia-teste-sandbox/scripts/conferir-token.mjs .claude/skills/claudia-teste-sandbox/scripts/test/proibidos.test.mjs .gitignore sandbox-lab/.gitignore
git commit -m "feat(lab): detect Anthropic tokens in versioned files and guest output; keep the cache and credentials out of git

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019iZMxAmFGsgenP4pjSBmb6"
```

---

### Tarefa 4: Raiz `Z:` dos testes no lado Node (`tempDir`, `testes-no-guest`)

**Modelo:** Sonnet.

**Files:**
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/caminhos-lab.mjs`
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/test/util.mjs`
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs`
- Test: `.claude/skills/claudia-teste-sandbox/scripts/test/caminhos-lab.test.mjs`, `.claude/skills/claudia-teste-sandbox/scripts/test/testes-no-guest.test.mjs`

**Interfaces:**
- Consumes: `estaNoSandbox`, `MARCADOR_GUEST` (existentes).
- Produces: `RAIZ_TESTE_GUEST = "Z:\\claudia-teste-sandbox"`, `motivoRecusaRaizTeste(real): string|null`, `raizDeCenarios({ noSandbox }): "Z:\\" | null` (usada pela Tarefa 5).

- [ ] **Passo 1: Testes que falham.** Em `caminhos-lab.test.mjs`, acrescentar ao import `RAIZ_TESTE_GUEST, motivoRecusaRaizTeste, raizDeCenarios` e:

```js
test("guest test root literal is pinned (travas.ps1 pins the same literal)", () => {
  assert.equal(RAIZ_TESTE_GUEST, "Z:\\claudia-teste-sandbox");
});

test("motivoRecusaRaizTeste: the root and anything under it, any letter case", () => {
  for (const p of ["Z:\\claudia-teste-sandbox", "z:\\CLAUDIA-TESTE-SANDBOX\\cs test a 1", "Z:\\claudia-teste-sandbox\\a\\b"]) {
    assert.equal(motivoRecusaRaizTeste(p), null, p);
  }
});

test("motivoRecusaRaizTeste: C:, other folders on Z:, siblings, dotted, relative, UNC and empty are refused", () => {
  for (const p of ["C:\\labdata\\tmp\\x", "Z:\\", "Z:\\tmp\\x", "Z:\\claudia-teste-sandbox-irmao\\x", "Z:\\claudia-teste-sandbox\\..\\x",
    "Z:\\claudia-teste-sandbox\\.\\x", "claudia-teste-sandbox\\x", "\\\\srv\\s\\claudia-teste-sandbox", "", null]) {
    assert.notEqual(motivoRecusaRaizTeste(p), null, String(p));
  }
});

test("raizDeCenarios: Z:\\ in the guest, nothing on the PC (D1: scenarios only in the Sandbox)", () => {
  assert.equal(raizDeCenarios({ noSandbox: true }), "Z:\\");
  assert.equal(raizDeCenarios({ noSandbox: false }), null);
  assert.equal(raizDeCenarios({}), null);
});
```

Em `testes-no-guest.test.mjs`, trocar a string esperada do primeiro teste por:

```js
assert.equal(c, `set LAB_NONCE=${NONCE}&& (if not exist Z:\\claudia-teste-sandbox exit /b 97)&& set TEMP=Z:\\tmp&& set TMP=Z:\\tmp&& (if not exist Z:\\tmp mkdir Z:\\tmp)&& node --test .claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs`);
```

e o título para `"node tests: explicit files, guest temp on Z:, refuses without the Z: test root, nonce first"`.

- [ ] **Passo 2: Rodar e ver falhar.** `node --test .claude/skills/claudia-teste-sandbox/scripts/test/caminhos-lab.test.mjs .claude/skills/claudia-teste-sandbox/scripts/test/testes-no-guest.test.mjs` → falhas nomeadas.

- [ ] **Passo 3: Implementar em `caminhos-lab.mjs`** (no fim):

```js
/**
 * Root of every folder the Sandbox tests create (spec A6): a VHD mounted as Z: inside the guest only (sandbox\disco-z.ps1).
 * travas.ps1 keeps the same literal ($script:LabGuestTestRoot); a pure test on each side pins it.
 */
export const RAIZ_TESTE_GUEST = "Z:\\claudia-teste-sandbox";

/** PURE. null when `real` (from fs.realpathSync.native) is the guest test root or inside it; otherwise the reason. */
export function motivoRecusaRaizTeste(real) {
  const r = String(real ?? "");
  if (r.trim() === "") return "path is required";
  if (r.split(/[\\/]+/).some((s) => s === "." || s === "..")) return "path has '.' or '..' segments";
  if (w.normalize(r) !== r) return "path is not normalized";
  const k = r.replace(/\\+$/, "").toLowerCase();
  const raiz = RAIZ_TESTE_GUEST.toLowerCase();
  if (k === raiz || k.startsWith(raiz + "\\")) return null;
  return `real path is not under ${RAIZ_TESTE_GUEST}`;
}

/** PURE. tempRoot for scenario folders (D1, spec addendum 8.1): Z:\ inside the guest; null on the PC, where scenarios never run. */
export function raizDeCenarios({ noSandbox } = {}) {
  return noSandbox === true ? "Z:\\" : null;
}
```

- [ ] **Passo 4: `util.mjs`** — substituir `tempDir`:

```js
/** Temp folder with a space in its name, on the guest-only Z: drive (spec A6). Sandbox only. */
export function tempDir(prefixo) {
  exigirSandbox(); // before any folder is created
  fs.mkdirSync(RAIZ_TESTE_GUEST, { recursive: true });
  const raiz = fs.realpathSync.native(RAIZ_TESTE_GUEST);
  const m1 = motivoRecusaRaizTeste(raiz);
  if (m1) throw new Error(`tempDir: refused (${m1}): ${raiz}`);
  const dir = fs.mkdtempSync(path.join(raiz, `cs test ${prefixo} `));
  const m2 = motivoRecusaRaizTeste(fs.realpathSync.native(dir));
  if (m2) throw new Error(`tempDir: refused (${m2}): ${dir}`);
  return dir;
}
```

com `import { estaNoSandbox, MARCADOR_GUEST, RAIZ_TESTE_GUEST, motivoRecusaRaizTeste } from "../caminhos-lab.mjs";` e sem o import de `os` se não for mais usado (o `exigirSandbox` usa `os.userInfo()`: mantenha o import).

- [ ] **Passo 5: `testes-no-guest.mjs`** — trocar a linha `const prefixo = ...` por:

```js
  const prefixo = `set LAB_NONCE=${nonce}&& (if not exist Z:\\claudia-teste-sandbox exit /b 97)&& set TEMP=Z:\\tmp&& set TMP=Z:\\tmp&& (if not exist Z:\\tmp mkdir Z:\\tmp)`;
```

e o comentário do topo: o comando roda pelo `guest-build.ps1` do ambiente `testes` (`compose.json`), que usa o Node instalado no guest; o `exit /b 97` para tudo se o disco `Z:` não estiver montado.

- [ ] **Passo 6: Rodar e ver passar** (mesmo comando do Passo 2) → exit 0. Rodar também `proibidos.test.mjs` e `manifesto.test.mjs` (regressão do CI) com a lista explícita.

- [ ] **Passo 7: Provas de mutação:**
  1. `motivoRecusaRaizTeste`: trocar `raiz + "\\"` por `raiz` → `FAILED` em `... siblings ...`.
  2. `testes-no-guest.mjs`: voltar `TEMP=C:\\labdata\\tmp` → `FAILED` no teste do comando.
  3. `raizDeCenarios`: trocar `: null` por `: "C:\\"` → `FAILED - raizDeCenarios: ...`.

- [ ] **Passo 8: Commit.**
```bash
git add .claude/skills/claudia-teste-sandbox/scripts/caminhos-lab.mjs .claude/skills/claudia-teste-sandbox/scripts/test/util.mjs .claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs .claude/skills/claudia-teste-sandbox/scripts/test/caminhos-lab.test.mjs .claude/skills/claudia-teste-sandbox/scripts/test/testes-no-guest.test.mjs
git commit -m "feat(sandbox-skill): Sandbox tests create folders only under Z:\\claudia-teste-sandbox in the guest

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019iZMxAmFGsgenP4pjSBmb6"
```

---

### Tarefa 5: Cenários só no guest — lado Node (`novoCenarioDir`, `cenario-copia`, `cenario-no-guest.mjs`)

**Modelo:** Sonnet. Depende da Tarefa 4 e da aprovação do §8 da spec (Tarefa 0).

**Files:**
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/comum.mjs`
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/cenario-copia.mjs` (cabeçalho e linhas 82-87), comentários de uso em `cenario-novo.mjs` e `cenario-mock.mjs`
- Create: `.claude/skills/claudia-teste-sandbox/scripts/cenario-no-guest.mjs`
- Test: `.claude/skills/claudia-teste-sandbox/scripts/test/cenario-no-guest.test.mjs` (puro, entra no CI na Tarefa 10), `.claude/skills/claudia-teste-sandbox/scripts/test/motivo-recusa.test.mjs` (puro)

**Interfaces:**
- Consumes: `raizDeCenarios({ noSandbox })` → `"Z:\\"` ou `null`, `motivoRecusaRaizTeste`, `estaNoSandbox`, `MARCADOR_GUEST` (Tarefa 4); `onde`, `sandboxId`, `noGuest`, `validarInt`, `falhaDe` (existentes).
- Produces: `raizCenariosAtual()` (lança no PC), `saidaPadraoCenario(nome, tempRoot)`, `ORIGEM_DADOS_REAIS = "C:\\labdadosreais"` (em `comum.mjs`); `comandoDeCenarioNoGuest({ cenario, autorizado, manterAutomacao, nonce })` (pura, em `cenario-no-guest.mjs`). O script do guest `guest-cenario.ps1` é da Tarefa 5b e recebe exatamente `-Cenario <novo|mock|copia> -Nonce <hex> [-Autorizado] [-ManterAutomacao]`.

- [ ] **Passo 1: Testes puros que falham.**

`cenario-no-guest.test.mjs`:

```js
// Pure tests of the guest command builder for scenarios (D1): strings only, no disk, no wsb.
import test from "node:test";
import assert from "node:assert/strict";
import { comandoDeCenarioNoGuest } from "../cenario-no-guest.mjs";

const NONCE = "0123abcd0123abcd";

test("novo and mock: fixed guest script, nonce, nothing else", () => {
  assert.equal(comandoDeCenarioNoGuest({ cenario: "novo", nonce: NONCE }), `& C:\\labsandbox\\app\\guest-cenario.ps1 -Cenario novo -Nonce ${NONCE}`);
  assert.equal(comandoDeCenarioNoGuest({ cenario: "mock", nonce: NONCE }), `& C:\\labsandbox\\app\\guest-cenario.ps1 -Cenario mock -Nonce ${NONCE}`);
});

test("copia: needs --autorizado; --manter-automacao passes through", () => {
  assert.throws(() => comandoDeCenarioNoGuest({ cenario: "copia", nonce: NONCE }), /autorizado/);
  assert.equal(comandoDeCenarioNoGuest({ cenario: "copia", autorizado: true, nonce: NONCE }), `& C:\\labsandbox\\app\\guest-cenario.ps1 -Cenario copia -Nonce ${NONCE} -Autorizado`);
  assert.equal(comandoDeCenarioNoGuest({ cenario: "copia", autorizado: true, manterAutomacao: true, nonce: NONCE }), `& C:\\labsandbox\\app\\guest-cenario.ps1 -Cenario copia -Nonce ${NONCE} -Autorizado -ManterAutomacao`);
});

test("refuses unknown scenarios, injected text, flags on the wrong scenario and a bad nonce", () => {
  for (const c of ["", "Novo", "copia; del x", "../copia", "outro"]) assert.throws(() => comandoDeCenarioNoGuest({ cenario: c, nonce: NONCE }), /unknown scenario/, c);
  assert.throws(() => comandoDeCenarioNoGuest({ cenario: "novo", autorizado: true, nonce: NONCE }), /only apply to copia/);
  assert.throws(() => comandoDeCenarioNoGuest({ cenario: "mock", manterAutomacao: true, nonce: NONCE }), /only apply to copia/);
  assert.throws(() => comandoDeCenarioNoGuest({ cenario: "novo", nonce: "x&del" }), /nonce/);
});
```

Em `motivo-recusa.test.mjs` (puro; importar `saidaPadraoCenario`):

```js
test("saidaPadraoCenario: same default folder novoCenarioDir uses, under <tempRoot>\\claudia-teste-sandbox", () => {
  assert.equal(saidaPadraoCenario("copia", "Z:\\"), "Z:\\claudia-teste-sandbox\\copia");
  assert.equal(motivoRecusa(saidaPadraoCenario("mock", "Z:\\"), { repoRoot: REPO, tempRoot: "Z:\\" }), null);
});
```

(Usa separador `\`: rodar no Windows; o arquivo é puro e não está no CI Linux.)

- [ ] **Passo 2: Rodar e ver falhar.** `node --test .claude/skills/claudia-teste-sandbox/scripts/test/cenario-no-guest.test.mjs .claude/skills/claudia-teste-sandbox/scripts/test/motivo-recusa.test.mjs` → módulo/export inexistente.

- [ ] **Passo 3: `comum.mjs`.**

```js
import { raizPrincipalDe, caminhosDoLab, ambienteGitLimpo, estaNoSandbox, MARCADOR_GUEST, raizDeCenarios, motivoRecusaRaizTeste } from "./caminhos-lab.mjs";

/** Guest folder where test-up -DadosReais mounts the app's Roaming data folder, read-only (spec addendum 8.1). */
export const ORIGEM_DADOS_REAIS = "C:\\labdadosreais";

/** PURE. Default scenario folder for a given tempRoot. cenario-copia uses it for its overlap check. */
export function saidaPadraoCenario(nome, tempRoot) {
  return path.join(tempRoot, NOME_BASE, nome);
}

/** Z:\ inside the guest (two factors). On the PC it throws: scenarios run only in the Sandbox (D1). */
export function raizCenariosAtual() {
  const noSandbox = estaNoSandbox({ identidade: os.userInfo().username, temBootstrap: fs.existsSync(MARCADOR_GUEST) });
  const raiz = raizDeCenarios({ noSandbox });
  if (!raiz) throw new Error("scenarios run only inside Windows Sandbox (D1): from the PC use cenario-no-guest.mjs --cenario <novo|mock|copia>");
  return raiz;
}
```

Em `novoCenarioDir`:
  - primeira linha: `const tempRoot = raizCenariosAtual();` (no PC, lança antes de qualquer acesso a disco);
  - `const dir = path.resolve(saida || saidaPadraoCenario(nome, tempRoot));`
  - logo depois do `motivoRecusa`: `const mz = motivoRecusaRaizTeste(dir); if (mz) throw new Error(\`novoCenarioDir: refused (${mz}): ${dir}\`);`
  - no bloco `if (st)`, depois de calcular `real`: `if (motivoRecusaRaizTeste(real)) throw new Error(\`novoCenarioDir: refused (real path not on Z:: ${real}): ${dir}\`);`
  - depois do `mkdirSync`, antes de gravar `.cenario`: `if (motivoRecusaRaizTeste(fs.realpathSync.native(dir))) throw new Error(\`novoCenarioDir: refused (real path not on Z: after create): ${dir}\`);`

- [ ] **Passo 4: `cenario-copia.mjs`.** Cabeçalho: "roda só no guest (D1); `--origem` padrão `C:\labdadosreais` (montado só leitura por `test-up -DadosReais`)". No `try`, nesta ordem:

```js
  const tempRoot = raizCenariosAtual(); // throws on the PC before anything is read (D1)
  const origem = path.resolve(values.origem || ORIGEM_DADOS_REAIS);
  if (!fs.existsSync(origem) || !fs.statSync(origem).isDirectory()) {
    throw new Error(`Source data not found: ${origem}. The real data is mounted only by test-up -DadosReais, with the user's authorization for this round.`);
  }
  // Same default folder novoCenarioDir uses.
  const saidaPath = path.resolve(values.saida || saidaPadraoCenario("copia", tempRoot));
```

importando `ORIGEM_DADOS_REAIS, raizCenariosAtual, saidaPadraoCenario` de `./comum.mjs`; remover `os` e `process.env.APPDATA`/`TEMP` (não são mais usados). O teste do `--autorizado` antes de tudo continua igual. Em `cenario-novo.mjs` e `cenario-mock.mjs`, trocar no comentário de uso `%TEMP%\claudia-teste-sandbox\<x>` por `Z:\claudia-teste-sandbox\<x> (only inside the guest; from the PC use cenario-no-guest.mjs)`.

- [ ] **Passo 5: `cenario-no-guest.mjs`.**

```js
// Creates a data scenario INSIDE the guest (D1: never on the PC) through C:\labsandbox\app\guest-cenario.ps1, then waits
// for C:\labwork\run\cenario.json (= <work folder>\guest-rw\run\cenario.json on the host) carrying this run's nonce.
// Usage: node cenario-no-guest.mjs --cenario novo|mock|copia [--autorizado] [--manter-automacao] [--id <id>] [--timeout 600]
// copia needs --autorizado (ask the user in this round) and a lab started with test-up -DadosReais.
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { parseArgs } from "node:util";
import { pathToFileURL } from "node:url";
import { onde, sandboxId, noGuest, validarInt, falhaDe } from "./comum.mjs";

const CENARIOS = ["novo", "mock", "copia"];
const uso = (msg) => Object.assign(new Error(msg), { code: "ERR_PARSE_ARGS_USAGE" });

/** PURE. The PowerShell line run in the guest. The scenario comes only from a fixed list, so nothing needs quoting. */
export function comandoDeCenarioNoGuest({ cenario, autorizado = false, manterAutomacao = false, nonce }) {
  if (!/^[a-f0-9]{8,64}$/.test(String(nonce))) throw new Error("nonce must be 8-64 hex characters");
  if (!CENARIOS.includes(cenario)) throw uso(`unknown scenario: ${cenario} (novo, mock, copia)`);
  if (cenario !== "copia" && (autorizado || manterAutomacao)) throw uso("--autorizado and --manter-automacao only apply to copia");
  if (cenario === "copia" && !autorizado) throw uso("copia needs --autorizado (ask the user in this round)");
  return `& C:\\labsandbox\\app\\guest-cenario.ps1 -Cenario ${cenario} -Nonce ${nonce}${autorizado ? " -Autorizado" : ""}${manterAutomacao ? " -ManterAutomacao" : ""}`;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    const { values } = parseArgs({ options: {
      cenario: { type: "string" }, autorizado: { type: "boolean" }, "manter-automacao": { type: "boolean" },
      id: { type: "string" }, timeout: { type: "string", default: "600" },
    } });
    const timeout = validarInt(values.timeout, 1, 7200, "--timeout");
    const nonce = crypto.randomBytes(8).toString("hex");
    const cmd = comandoDeCenarioNoGuest({ cenario: values.cenario, autorizado: Boolean(values.autorizado), manterAutomacao: Boolean(values["manter-automacao"]), nonce });
    const id = sandboxId(values.id);
    const result = path.join(onde().guestRw, "run", "cenario.json");
    noGuest(id, "ExistingLogin", cmd);
    const fim = Date.now() + timeout * 1000;
    let j = null;
    while (Date.now() < fim) {
      try { const t = JSON.parse(fs.readFileSync(result, "utf8")); if (t.nonce === nonce && t.exit !== null && t.exit !== undefined) { j = t; break; } } catch { /* not written yet */ }
      await new Promise((r) => setTimeout(r, 2000));
    }
    if (!j) throw new Error(`no result for this run in ${result} after ${timeout}s`);
    console.log(JSON.stringify({ exit: j.exit, cenario: j.cenario, saida: j.saida, erro: j.erro ?? null }));
    process.exit(j.exit === 0 ? 0 : 1);
  } catch (e) { falhaDe(e); }
}
```

- [ ] **Passo 6: Rodar e ver passar** (lista do Passo 2 + `caminhos-lab.test.mjs`, `onde.test.mjs`) → exit 0. Os testes de Sandbox (`comum`, `novo`, `mock`, `copia`) não rodam aqui: Tarefa 12. Conferir que nenhum deles depende de rodar no PC: todos já usam `tempDir()`.

- [ ] **Passo 7: Provas de mutação:**
  1. `comandoDeCenarioNoGuest`: apagar a linha do `copia && !autorizado` → `FAILED - copia: needs --autorizado ...`.
  2. Trocar `CENARIOS.includes(cenario)` por `typeof cenario === "string"` → `FAILED - refuses unknown scenarios ...`.
  3. `saidaPadraoCenario`: devolver `path.join(tempRoot, nome)` → `FAILED - saidaPadraoCenario: ...`.

- [ ] **Passo 8: Commit.**
```bash
git add .claude/skills/claudia-teste-sandbox/scripts/comum.mjs .claude/skills/claudia-teste-sandbox/scripts/cenario-copia.mjs .claude/skills/claudia-teste-sandbox/scripts/cenario-novo.mjs .claude/skills/claudia-teste-sandbox/scripts/cenario-mock.mjs .claude/skills/claudia-teste-sandbox/scripts/cenario-no-guest.mjs .claude/skills/claudia-teste-sandbox/scripts/test/cenario-no-guest.test.mjs .claude/skills/claudia-teste-sandbox/scripts/test/motivo-recusa.test.mjs
git commit -m "feat(sandbox-skill): data scenarios run only inside the guest, on Z:; host triggers them with cenario-no-guest.mjs

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019iZMxAmFGsgenP4pjSBmb6"
```

---

### Tarefa 5b: Cenários só no guest — publicar só o app, `guest-cenario.ps1`, `guest-run.ps1 -Cenario`

**Modelo:** Sonnet. Depende da Tarefa 7 (acrescenta uma função a `guest-install.lib.ps1`) e da aprovação do §8.

**Files:**
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/publicar.mjs`
- Modify: `.claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs` (teste de Sandbox)
- Modify: `sandbox-lab/scripts/app-stage.ps1` (tira `-AppData` e `-LocalAppData`)
- Modify: `sandbox-lab/sandbox/app/guest-run.ps1`
- Create: `sandbox-lab/sandbox/app/guest-cenario.ps1`
- Modify: `sandbox-lab/sandbox/guest-install.lib.ps1` (acrescentar `Get-LabGuestScenarioRefusal`)
- Test: `sandbox-lab/scripts/test/guest-install.test.ps1` (acrescentar)

**Interfaces:**
- Consumes: `Get-LabGuestToolEnv` (Tarefa 7); comando de `comandoDeCenarioNoGuest` (Tarefa 5).
- Produces: `publicar({ app }, { raizPrincipal, rodarStage })`; `app-stage.ps1 -App <p> [-WorkDir] [-Clean]`; `guest-run.ps1 [-Exe] [-Cenario <Z:\claudia-teste-sandbox\nome>] [-NoStart]`; `guest-cenario.ps1 -Cenario -Nonce [-Autorizado] [-ManterAutomacao]` → `C:\labwork\run\cenario.json`.

- [ ] **Passo 1: Teste puro que falha** — em `guest-install.test.ps1`, antes do resumo:

```powershell
Check 'scenario folder for guest-run: only a direct child of Z:\claudia-teste-sandbox' {
  foreach ($ok in @('Z:\claudia-teste-sandbox\novo', 'z:\CLAUDIA-TESTE-SANDBOX\mock', 'Z:\claudia-teste-sandbox\cs test x 1')) {
    if (Get-LabGuestScenarioRefusal $ok) { throw "must allow '$ok'" }
  }
  foreach ($bad in @('', 'Z:\claudia-teste-sandbox', 'Z:\claudia-teste-sandbox\a\b', 'Z:\claudia-teste-sandbox\..\x', 'C:\labdata\novo', 'Z:\tmp\novo', 'Z:\claudia-teste-sandbox-irmao\novo', 'novo', '\\srv\s\novo')) {
    if (-not (Get-LabGuestScenarioRefusal $bad)) { throw "must refuse '$bad'" }
  }
}
```

- [ ] **Passo 2: Rodar e ver falhar.**

- [ ] **Passo 3: `guest-install.lib.ps1`** — acrescentar:

```powershell
function Get-LabGuestScenarioRefusal([string]$Path) {
  # guest-run.ps1 -Cenario: a scenario folder created by the scenario scripts in the guest (D1), nothing else.
  if ([string]::IsNullOrWhiteSpace($Path)) { return 'scenario folder is required' }
  if ($Path -notmatch '^[Zz]:\\[Cc][Ll][Aa][Uu][Dd][Ii][Aa]-[Tt][Ee][Ss][Tt][Ee]-[Ss][Aa][Nn][Dd][Bb][Oo][Xx]\\[^\\.][^\\]*$') { return 'scenario folder must be Z:\claudia-teste-sandbox\<name>' }
  return $null
}
```

- [ ] **Passo 4: `guest-cenario.ps1`** (5.1, ASCII):

```powershell
# guest-cenario.ps1 - runs INSIDE the Sandbox (wsb exec -r ExistingLogin). Creates a data scenario on Z: with the Node the
# guest installed (spec addendum 8.1: scenarios never run on the PC). copia reads only C:\labdadosreais (read-only mount,
# present only when test-up ran with -DadosReais). Output: C:\labwork\run\cenario.json {nonce, cenario, exit, saida, erro}
param(
  [ValidateSet('novo', 'mock', 'copia')][string]$Cenario,
  [ValidatePattern('^[a-f0-9]{8,64}$')][string]$Nonce,
  [switch]$Autorizado,
  [switch]$ManterAutomacao
)
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot '..\guest-install.lib.ps1')
$Out = 'C:\labwork\run'
New-Item -ItemType Directory -Force $Out | Out-Null
$utf8 = New-Object System.Text.UTF8Encoding($false)
$res = [ordered]@{ nonce = $Nonce; cenario = $Cenario; exit = $null; saida = "Z:\claudia-teste-sandbox\$Cenario"; erro = $null }
function Save-Result { [IO.File]::WriteAllText((Join-Path $Out 'cenario.json'), ($res | ConvertTo-Json), $utf8) }
function Stop-With([int]$Code, [string]$Why) { $res.exit = $Code; $res.erro = $Why; Save-Result; exit $Code }

$inst = $null
try { $inst = Get-Content -LiteralPath 'C:\lab\out\instalar.json' -Raw | ConvertFrom-Json } catch { }
$toolEnv = Get-LabGuestToolEnv $inst
if (-not $toolEnv) { Stop-With 1 'instalar.json missing or not ok: this ambiente must install node (compose.json)' }
$node = @(@($toolEnv.paths) | ForEach-Object { Join-Path $_ 'node.exe' } | Where-Object { Test-Path -LiteralPath $_ }) | Select-Object -First 1
if (-not $node) { Stop-With 1 'node.exe not found in the installed tools' }
$script = "C:\labproject\.claude\skills\claudia-teste-sandbox\scripts\cenario-$Cenario.mjs"
$argv = @($script, '--saida', $res.saida)
if ($Cenario -eq 'copia') {
  if (-not (Test-Path -LiteralPath 'C:\labdadosreais' -PathType Container)) { Stop-With 2 'real data not mounted: start the lab with test-up -DadosReais (only with the user authorization for this round)' }
  $argv += @('--origem', 'C:\labdadosreais')
  if ($Autorizado) { $argv += '--autorizado' }
  if ($ManterAutomacao) { $argv += '--manter-automacao' }
}
$lines = @(& $node @argv 2>&1 | ForEach-Object { "$_" })
$res.exit = $LASTEXITCODE
if ($res.exit -ne 0) { $res.erro = ($lines | Select-Object -Last 3) -join ' | ' }
Save-Result
exit $res.exit
```

- [ ] **Passo 5: `guest-run.ps1`.** Parâmetro novo `[string]$Cenario = ''`; dot-source `Join-Path $PSScriptRoot '..\guest-install.lib.ps1'`; a seção "2. dados do app" passa a ler do cenário, não do deploy:

```powershell
# 2. app data from a scenario created in the guest (D1); the deploy carries only the app
$r.dataFiles = [ordered]@{}
if ($Cenario) {
  $why = Get-LabGuestScenarioRefusal $Cenario
  $item = Get-Item -LiteralPath $Cenario -Force -ErrorAction SilentlyContinue
  if (-not $why -and -not $item) { $why = 'scenario folder not found' }
  if (-not $why -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { $why = 'scenario folder is a link' }
  if (-not $why) { $why = Get-LabGuestScenarioRefusal $item.FullName }   # the real path, too
  if ($why) { $r.errors += "cenario: $why" }
  else {
    foreach ($pair in @(@('appdata', $env:APPDATA), @('localappdata', $env:LOCALAPPDATA))) {
      $src = Join-Path $item.FullName $pair[0]
      if (-not (Test-Path -LiteralPath $src)) { continue }
      try {
        robocopy $src $pair[1] /E /XJ /NFL /NDL /NP /R:1 /W:1 | Out-Null
        $files = @(Get-ChildItem -LiteralPath $src -Recurse -File)
        foreach ($f in $files) { $t = Join-Path $pair[1] $f.FullName.Substring($src.Length + 1); if (Test-Path -LiteralPath $t) { (Get-Item -LiteralPath $t).IsReadOnly = $false } }
        $r.dataFiles[$pair[0]] = $files.Count
      } catch { $r.errors += "$($pair[0]): $_" }
    }
  }
}
```

Atualizar o comentário do topo (passo 2: `-Cenario`, não `C:\appdeploy\appdata`; uso `-Cenario Z:\claudia-teste-sandbox\<nome>`).

- [ ] **Passo 6: `app-stage.ps1`.** Tirar os parâmetros `-AppData`/`-LocalAppData`, as duas linhas `Resolve-Source` deles, o laço que copia `appdata`/`localappdata` e as pastas `appdata`/`localappdata` do `New-Item` (o deploy passa a ter só `app\` e o marcador). `.SYNOPSIS`: "Publica só o app; os dados do app vêm de um cenário criado no guest (D1)."

- [ ] **Passo 7: `publicar.mjs`.** `publicar({ app }, { raizPrincipal, rodarStage })`: tirar a leitura de `cenario.json`; `rodarStage(c.appStage, ["-App", app, "-WorkDir", c.trabalho, "-Clean"])`; devolver `{ workDir, deploy }`. CLI: só `--app` (sem `--app` → exit 2); `--cenario` deixa de existir (o `parseArgs` estrito já sai com 2). Comentário do topo: "publishes only the app; data comes from a scenario created in the guest (cenario-no-guest.mjs) and installed by guest-run.ps1 -Cenario".

- [ ] **Passo 8: `publicar.test.mjs`** (teste de Sandbox; aqui só editar):
  - em `montar`: tirar a criação do cenário (linhas 18-20) e `cen` do retorno;
  - trocar todo `publicar({ app: m.app, cenario: m.cen }, m.deps)` por `publicar({ app: m.app }, m.deps)`;
  - no primeiro teste, trocar as asserções das linhas 33 e 35 por `assert.deepEqual(m.chamadas[0].args, ["-App", m.app, "-WorkDir", path.join(m.raiz, ".sandbox-work"), "-Clean"]);` e o título por `"first publish: app-stage gets only -App, -WorkDir <root>\\.sandbox-work and -Clean; marker written"`;
  - trocar o teste `"a folder without cenario.json is refused"` por `test("the old --cenario option is gone", () => { assert.equal(rodar(path.join(SCRIPTS, "publicar.mjs"), ["--app", "x", "--cenario", "y"]).code, 2); });`
  - no último teste, a segunda linha passa a ser `assert.equal(rodar(script, []).code, 2);` e o título `"CLI: --work-dir and --cenario no longer exist and a missing --app exits 2"`.

- [ ] **Passo 9: Rodar e ver passar** `guest-install.test.ps1` (dois PowerShell); parser sem erros em `guest-cenario.ps1`, `guest-run.ps1`, `app-stage.ps1`; ASCII nos `.ps1` de `sandbox/`; `node --check .claude/skills/claudia-teste-sandbox/scripts/publicar.mjs`. `git grep -n "AppData\|cenario" -- sandbox-lab/scripts/app-stage.ps1` → nada. **Não rodar** `publicar.test.mjs`, `guest-cenario.ps1` nem `guest-run.ps1` no PC (Tarefa 12).

- [ ] **Passo 10: Prova de mutação:** em `Get-LabGuestScenarioRefusal`, trocar `[^\\]*$` por `.*$` → `FAILED - scenario folder for guest-run: ...` (o `a\b` passa).

- [ ] **Passo 11: Commit.**
```bash
git add .claude/skills/claudia-teste-sandbox/scripts/publicar.mjs .claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs sandbox-lab/scripts/app-stage.ps1 sandbox-lab/sandbox/app/guest-run.ps1 sandbox-lab/sandbox/app/guest-cenario.ps1 sandbox-lab/sandbox/guest-install.lib.ps1 sandbox-lab/scripts/test/guest-install.test.ps1
git commit -m "feat(lab): publish only the app; scenario data is created and installed inside the guest

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019iZMxAmFGsgenP4pjSBmb6"
```

---

### Tarefa 6: Disco `Z:` e conta no guest (`disco-z.ps1`, `bootstrap.ps1`)

**Modelo:** Sonnet.

**Files:**
- Create: `sandbox-lab/sandbox/guest-z.lib.ps1`
- Create: `sandbox-lab/sandbox/disco-z.ps1`
- Modify: `sandbox-lab/sandbox/bootstrap.ps1`
- Test: `sandbox-lab/scripts/test/guest-z.test.ps1`

**Interfaces:**
- Consumes: `guest.config.json` campos `conta`, `instalar` (Tarefa 8 gera).
- Produces: `C:\lab\out\disco-z.json`; `status.json` campos `elevated`, `discoZ`, `conta`, `instalar`; `bootstrap.ps1` inicia `C:\labsandbox\instalar.ps1` (Tarefa 7) destacado quando elevado.

- [ ] **Passo 1: Teste puro que falha** — `sandbox-lab/scripts/test/guest-z.test.ps1`:

```powershell
# Pure tests for sandbox/guest-z.lib.ps1 (the decisions disco-z.ps1 makes before touching a disk). No disk.
#   pwsh -NoProfile -File sandbox-lab/scripts/test/guest-z.test.ps1
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../../sandbox/guest-z.lib.ps1"
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) { try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" } }
function Should-BeNull($v) { if ($null -ne $v) { throw "expected null, got '$v'" } }
function Should-Match($v, [string]$re) { if ($null -eq $v -or "$v" -notmatch $re) { throw "expected /$re/, got '$v'" } }

Check 'format: allowed only when Z: is on the disk attached from our image' {
  Should-BeNull (Get-LabZFormatRefusal -ImageAttached $true -ImageDiskNumber 3 -PartitionDiskNumber 3)
  Should-Match (Get-LabZFormatRefusal -ImageAttached $true -ImageDiskNumber 3 -PartitionDiskNumber 0) 'disk 0'
  Should-Match (Get-LabZFormatRefusal -ImageAttached $false -ImageDiskNumber 3 -PartitionDiskNumber 3) 'not attached'
  Should-Match (Get-LabZFormatRefusal -ImageAttached $true -ImageDiskNumber $null -PartitionDiskNumber 3) 'unknown'
  Should-Match (Get-LabZFormatRefusal -ImageAttached $true -ImageDiskNumber 3 -PartitionDiskNumber $null) 'unknown'
}
Check 'start: create only when neither Z: nor the image exist; reuse only our own Z:' {
  if ((Get-LabZStartAction -ZExists $false -ImageExists $false -FormatRefusal $null) -ne 'create') { throw 'create' }
  if ((Get-LabZStartAction -ZExists $true -ImageExists $true -FormatRefusal $null) -ne 'reuse') { throw 'reuse' }
  Should-Match (Get-LabZStartAction -ZExists $true -ImageExists $true -FormatRefusal 'Z: is on disk 0, the image is disk 3') '^refuse'
  Should-Match (Get-LabZStartAction -ZExists $true -ImageExists $false -FormatRefusal 'the image is not attached') '^refuse'
  Should-Match (Get-LabZStartAction -ZExists $false -ImageExists $true -FormatRefusal $null) '^refuse'
}
Check 'letter and image are pinned' {
  if ($script:LabZLetter -cne 'Z' -or $script:LabZImage -cne 'C:\labdata\z.vhdx') { throw 'changed' }
}
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
```

- [ ] **Passo 2: Rodar e ver falhar.** `pwsh -NoProfile -File sandbox-lab/scripts/test/guest-z.test.ps1` → arquivo da lib não existe.

- [ ] **Passo 3: `sandbox-lab/sandbox/guest-z.lib.ps1`** (PowerShell 5.1, ASCII):

```powershell
# Pure decisions for disco-z.ps1. Dot-sourced in the guest (C:\labsandbox) and by the pure test on the host. No disk.
$script:LabZLetter = 'Z'
$script:LabZImage = 'C:\labdata\z.vhdx'

function Get-LabZFormatRefusal {
  # Format only the partition that sits on the disk attached from our own image file.
  param([bool]$ImageAttached, $ImageDiskNumber, $PartitionDiskNumber)
  if (-not $ImageAttached) { return 'the image is not attached' }
  if ($null -eq $ImageDiskNumber -or $null -eq $PartitionDiskNumber) { return 'disk number unknown' }
  if ([int]$ImageDiskNumber -ne [int]$PartitionDiskNumber) { return "Z: is on disk $PartitionDiskNumber, the image is disk $ImageDiskNumber" }
  return $null
}

function Get-LabZStartAction {
  # 'create' (nothing there yet), 'reuse' (Z: already on our image), or 'refuse: <reason>'.
  param([bool]$ZExists, [bool]$ImageExists, $FormatRefusal)
  if (-not $ZExists -and -not $ImageExists) { return 'create' }
  if (-not $ZExists) { return 'refuse: the image exists but Z: is not mounted' }
  if ($FormatRefusal) { return "refuse: Z: is already used by another disk ($FormatRefusal)" }
  return 'reuse'
}
```

- [ ] **Passo 4: `sandbox-lab/sandbox/disco-z.ps1`** (PowerShell 5.1, ASCII; roda como admin no guest):

```powershell
# disco-z.ps1 - runs INSIDE the Sandbox as an administrator (bootstrap.ps1 when the logon is elevated, otherwise
# test-up.ps1 through 'wsb exec -r System'). Creates C:\labdata\z.vhdx, mounts it as Z: and creates Z:\tmp and
# Z:\claudia-teste-sandbox (spec A6). Formats only the partition of the disk attached from that exact image file.
# Output: C:\lab\out\disco-z.json
param([int]$MaxMB = 20480)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'guest-z.lib.ps1')
$Image = $script:LabZImage
$OutFile = 'C:\lab\out\disco-z.json'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$sw = [Diagnostics.Stopwatch]::StartNew()
$res = [ordered]@{ ok = $false; letra = $script:LabZLetter; imagem = $Image; segundos = $null; erro = $null }

function Save-Result { $res.segundos = [math]::Round($sw.Elapsed.TotalSeconds, 1); [IO.File]::WriteAllText($OutFile, ($res | ConvertTo-Json), $utf8) }
function Invoke-DiskPart([string[]]$Lines) {
  $f = 'C:\labdata\disco-z-' + [guid]::NewGuid().ToString('N') + '.txt'
  [IO.File]::WriteAllLines($f, $Lines)
  $ErrorActionPreference = 'Continue'   # PS 5.1: native stderr must not become a terminating error here
  $out = & diskpart.exe /s $f 2>&1
  if ($LASTEXITCODE -ne 0) { throw "diskpart failed ($LASTEXITCODE): $($out -join ' | ')" }
}
function Get-State {
  $img = Get-DiskImage -ImagePath $Image -ErrorAction SilentlyContinue
  $part = Get-Partition -DriveLetter $script:LabZLetter -ErrorAction SilentlyContinue
  $imgDisk = $(if ($img -and $img.Attached) { $img.Number } else { $null })
  $partDisk = $(if ($part) { $part.DiskNumber } else { $null })
  return @{ img = $img; part = $part; refusal = (Get-LabZFormatRefusal -ImageAttached ([bool]($img -and $img.Attached)) -ImageDiskNumber $imgDisk -PartitionDiskNumber $partDisk) }
}

try {
  # Guest only: the marker is mapped only in the guest (the identity is SYSTEM when run through wsb exec).
  if (-not (Test-Path -LiteralPath 'C:\labsandbox\bootstrap.ps1' -PathType Leaf)) { throw 'not inside the lab guest' }
  $zExists = Test-Path -LiteralPath 'Z:\'
  $s = $(if ($zExists) { Get-State } else { @{ refusal = $null } })
  $action = Get-LabZStartAction -ZExists $zExists -ImageExists (Test-Path -LiteralPath $Image) -FormatRefusal $s.refusal
  if ($action -like 'refuse*') { throw $action }
  if ($action -eq 'create') {
    New-Item -ItemType Directory -Force 'C:\labdata' | Out-Null
    Invoke-DiskPart @(
      "create vdisk file=`"$Image`" maximum=$MaxMB type=expandable",
      "select vdisk file=`"$Image`"",
      'attach vdisk',
      'create partition primary',
      "assign letter=$($script:LabZLetter)"
    )
    $s = Get-State
    if ($s.refusal) { throw "nothing formatted: $($s.refusal)" }
    $s.part | Format-Volume -FileSystem NTFS -NewFileSystemLabel 'LABZ' -Confirm:$false | Out-Null
  }
  New-Item -ItemType Directory -Force 'Z:\tmp', 'Z:\claudia-teste-sandbox' | Out-Null
  $item = Get-Item -LiteralPath 'Z:\claudia-teste-sandbox' -Force
  if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Z:\claudia-teste-sandbox is a link' }
  $res.ok = $true
} catch { $res.erro = $_.Exception.Message }
Save-Result
if ($res.ok) { exit 0 } else { exit 1 }
```

- [ ] **Passo 5: `bootstrap.ps1`:**
  - No `$status` inicial, acrescentar: `elevated = $false`, `discoZ = 'skipped'`, `conta = 'nao-pedida'`, `instalar = 'nada'`.
  - Depois de "1. pasta de saida gravavel", acrescentar:
    ```powershell
    $status.elevated = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    ```
  - Depois da seção 2 (banco), acrescentar:
    ```powershell
    # --- 2b. Z: drive for everything the Sandbox tests create (spec A6). Needs an administrator. ---
    $status.phase = 'disco-z'
    if ($status.elevated) {
      try {
        $p = Start-Process powershell.exe -PassThru -Wait -WindowStyle Hidden -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File','C:\labsandbox\disco-z.ps1')
        $status.discoZ = $(if ($p.ExitCode -eq 0) { 'ok' } else { 'failed' })
      } catch { $status.discoZ = 'failed'; AddErr "disco-z: $_" }
    } else { $status.discoZ = 'needs-system'; Log 'logon is not elevated: test-up runs disco-z.ps1 as System' }
    Save-Status

    # --- 2c. Claude account (spec A4): copy the mounted login into this user's profile. Only presence is recorded. ---
    if ($cfg.conta) {
      $status.phase = 'conta'
      try {
        $src = 'C:\labcred\.credentials.json'
        if (Test-Path -LiteralPath $src -PathType Leaf) {
          $dir = Join-Path $env:USERPROFILE '.claude'
          New-Item -ItemType Directory -Force $dir | Out-Null
          $dst = Join-Path $dir '.credentials.json'
          Copy-Item -LiteralPath $src -Destination $dst -Force
          Set-ItemProperty -LiteralPath $dst -Name IsReadOnly -Value $false
          $status.conta = 'presente'
        } else { $status.conta = 'ausente'; AddErr 'conta: the login file is not mounted' }
      } catch { $status.conta = 'falhou'; AddErr 'conta: copy failed' }   # never log the exception text
      Save-Status
    }
    ```
  - Depois de `$status.bootstrap = ...; Save-Status` (fim do core) e antes da seção 6 (MCP):
    ```powershell
    # --- 5b. packages (spec A3): instalar.ps1 runs detached, in parallel with the MCP phase; test-up waits for instalar.json ---
    if (@($cfg.instalar | Where-Object { $_ }).Count -gt 0) {
      if ($status.elevated) {
        Start-Process powershell.exe -WindowStyle Hidden -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File','C:\labsandbox\instalar.ps1') | Out-Null
        $status.instalar = 'iniciado'
      } else { $status.instalar = 'needs-system' }
      Save-Status
    }
    ```
  - Atualizar o comentário do topo (mapeamentos: `C:\labcred` quando a conta é pedida; disco `Z:`).

- [ ] **Passo 6: Rodar e ver passar** `guest-z.test.ps1` com os dois PowerShell. Conferir sintaxe (sem executar) de `disco-z.ps1` e `bootstrap.ps1` com `[Management.Automation.Language.Parser]::ParseFile` em `pwsh` e `powershell` → 0 erros. Conferir ASCII: `pwsh -NoProfile -Command "(Get-Content sandbox-lab/sandbox/disco-z.ps1 -Raw) -match '[^\x00-\x7F]'"` → `False` (igual para a lib). **Não executar `disco-z.ps1` no PC.**

- [ ] **Passo 7: Provas de mutação:**
  1. `Get-LabZFormatRefusal`: apagar a comparação dos números → `FAILED - format: ...`.
  2. `Get-LabZStartAction`: trocar `if (-not $ZExists) { return 'refuse: ...' }` por `return 'create'` → `FAILED - start: ...`.

- [ ] **Passo 8: Commit.**
```bash
git add sandbox-lab/sandbox/guest-z.lib.ps1 sandbox-lab/sandbox/disco-z.ps1 sandbox-lab/sandbox/bootstrap.ps1 sandbox-lab/scripts/test/guest-z.test.ps1
git commit -m "feat(lab): guest-only Z: drive from a VHD, Claude login copy in the guest, detached package install

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019iZMxAmFGsgenP4pjSBmb6"
```

---

### Tarefa 7: Instalação no guest (`instalar.ps1`) e `guest-build.ps1` sem `C:\hosttools`

**Modelo:** Sonnet.

**Files:**
- Create: `sandbox-lab/sandbox/guest-install.lib.ps1`
- Create: `sandbox-lab/sandbox/instalar.ps1`
- Modify: `sandbox-lab/sandbox/app/guest-build.ps1:1-36` (cabeçalho e seção de ambiente)
- Test: `sandbox-lab/scripts/test/guest-install.test.ps1`

**Interfaces:**
- Consumes: `guest.config.json` `instalar` (contrato).
- Produces: `C:\lab\out\instalar.json` (contrato); `Get-LabGuestToolEnv` usado por `guest-build.ps1`.

- [ ] **Passo 1: Teste puro que falha** — `sandbox-lab/scripts/test/guest-install.test.ps1`:

```powershell
# Pure tests for sandbox/guest-install.lib.ps1. No disk.
#   pwsh -NoProfile -File sandbox-lab/scripts/test/guest-install.test.ps1
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../../sandbox/guest-install.lib.ps1"
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) { try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" } }
function R([string]$n, $e) { [pscustomobject]@{ nome = $n; exit = $e } }

Check 'exit: 3010 (reboot requested) is success, null is failure' {
  if ((ConvertTo-LabInstallExit 0) -ne 0 -or (ConvertTo-LabInstallExit 3010) -ne 0) { throw 'success codes' }
  if ((ConvertTo-LabInstallExit 1603) -ne 1603 -or (ConvertTo-LabInstallExit $null) -ne 1) { throw 'failure codes' }
}
Check 'ok: every expected package, in order, all exit 0' {
  $exp = @('msvc', 'rust', 'node', 'claude-code')
  if (-not (Test-LabInstallOk -Expected $exp -Results @((R 'msvc' 0), (R 'rust' 0), (R 'node' 0), (R 'claude-code' 0)))) { throw 'all ok' }
  if (Test-LabInstallOk -Expected $exp -Results @((R 'msvc' 0), (R 'rust' 1603))) { throw 'stopped early' }
  if (Test-LabInstallOk -Expected $exp -Results @((R 'msvc' 0), (R 'rust' 0), (R 'node' 0), (R 'claude-code' 2))) { throw 'last failed' }
  if (Test-LabInstallOk -Expected @('rust', 'msvc') -Results @((R 'msvc' 0), (R 'rust' 0))) { throw 'order' }
  if (-not (Test-LabInstallOk -Expected @() -Results @())) { throw 'nothing to install is ok' }
}
Check 'tool env: PATH folders and vcvars from instalar.json; nothing when not ok' {
  $inst = [pscustomobject]@{ ok = $true; pacotes = @(
    [pscustomobject]@{ nome = 'msvc'; binDir = $null; vcvars = 'C:\labtools\msvc\VC\Auxiliary\Build\vcvars64.bat' },
    [pscustomobject]@{ nome = 'rust'; binDir = 'C:\Program Files\Rust stable MSVC 1.95\bin'; vcvars = $null },
    [pscustomobject]@{ nome = 'node'; binDir = 'C:\labtools\node\node-v22.23.3-win-x64'; vcvars = $null }) }
  $e = Get-LabGuestToolEnv $inst
  if (($e.paths -join '|') -ne 'C:\Program Files\Rust stable MSVC 1.95\bin|C:\labtools\node\node-v22.23.3-win-x64') { throw "paths $($e.paths -join '|')" }
  if ($e.vcvars -ne 'C:\labtools\msvc\VC\Auxiliary\Build\vcvars64.bat') { throw 'vcvars' }
  if ($null -ne (Get-LabGuestToolEnv ([pscustomobject]@{ ok = $false; pacotes = @() }))) { throw 'not ok -> null' }
  if ($null -ne (Get-LabGuestToolEnv $null)) { throw 'missing -> null' }
}
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
```

- [ ] **Passo 2: Rodar e ver falhar.**

- [ ] **Passo 3: `sandbox-lab/sandbox/guest-install.lib.ps1`** (5.1, ASCII):

```powershell
# Pure decisions for instalar.ps1 and guest-build.ps1. Dot-sourced in the guest and by the pure test on the host. No disk.

function ConvertTo-LabInstallExit($Exit) {
  # 3010 = success, reboot requested (the guest is never rebooted). No code at all is a failure.
  if ($null -eq $Exit) { return 1 }
  if ([int]$Exit -eq 3010) { return 0 }
  return [int]$Exit
}

function Test-LabInstallOk {
  # True only when every expected package ran, in the expected order, and exited 0.
  param([string[]]$Expected = @(), [object[]]$Results = @())
  $names = @(@($Results) | Where-Object { $_ } | ForEach-Object { [string]$_.nome })
  $exp = @(@($Expected) | Where-Object { $_ })
  if ($names.Count -ne $exp.Count) { return $false }
  for ($i = 0; $i -lt $exp.Count; $i++) { if ($names[$i] -ne $exp[$i]) { return $false } }
  return -not @(@($Results) | Where-Object { $_ -and $_.exit -ne 0 }).Count
}

function Get-LabGuestToolEnv {
  # From instalar.json: folders to put first on PATH and the vcvars64.bat to import. $null when the install is not ok.
  param($Instalar)
  if ($null -eq $Instalar -or -not $Instalar.ok) { return $null }
  $paths = @(@($Instalar.pacotes) | Where-Object { $_ -and $_.binDir } | ForEach-Object { [string]$_.binDir })
  $vc = @(@($Instalar.pacotes) | Where-Object { $_ -and $_.vcvars } | ForEach-Object { [string]$_.vcvars }) | Select-Object -First 1
  return [ordered]@{ paths = $paths; vcvars = $vc }
}
```

- [ ] **Passo 4: `sandbox-lab/sandbox/instalar.ps1`** (5.1, ASCII):

```powershell
# instalar.ps1 - runs INSIDE the Sandbox as an administrator (started by bootstrap.ps1 when the logon is elevated,
# otherwise by test-up.ps1 through 'wsb exec -r System'). Installs, from the read-only cache C:\lab\cache, the packages
# guest.config.json lists (already in the order msvc, rust, node, claude-code; spec A3). No network needed.
# Output: C:\lab\out\instalar.json (+ C:\lab\out\instalar-<name>.log). Tools go to C:\labtools (guest disk).
# 'Continue' on purpose: in PowerShell 5.1 with 'Stop', any stderr line of a native exe (rustc, cmd, claude) becomes a
# terminating error. Failures are decided by exit codes and explicit throws below.
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot 'guest-install.lib.ps1')
$OutFile = 'C:\lab\out\instalar.json'
$Tools = 'C:\labtools'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$elevated = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$res = [ordered]@{ inicio = (Get-Date).ToString('o'); fim = $null; ok = $false; elevado = $elevated; erro = $null; pacotes = @() }
function Save-Result { [IO.File]::WriteAllText($OutFile, ($res | ConvertTo-Json -Depth 5), $utf8) }
function Stop-With([string]$Why) { $res.erro = $Why; $res.fim = (Get-Date).ToString('o'); Save-Result; exit 1 }

if (-not (Test-Path -LiteralPath 'C:\labsandbox\bootstrap.ps1' -PathType Leaf)) { Stop-With 'not inside the lab guest' }
if (-not $elevated) { Stop-With 'not elevated' }
$cfg = Get-Content -LiteralPath 'C:\labsandbox\guest.config.json' -Raw | ConvertFrom-Json
$wanted = @(@($cfg.instalar) | Where-Object { $_ })
Save-Result

function Test-Sha([string]$File, [string]$Sha) { return ((Get-FileHash -LiteralPath $File -Algorithm SHA256).Hash -ieq $Sha) }
function Invoke-Package($p) {
  $r = [ordered]@{ nome = $p.nome; versao = $p.versao; exit = $null; segundos = $null; binDir = $null; vcvars = $null; versaoVista = $null; erro = $null }
  $sw = [Diagnostics.Stopwatch]::StartNew()
  try {
    switch ($p.tipo) {
      'vs-layout' {
        # Install from the offline layout without network (--noWeb); exe inside the layout, as Microsoft documents.
        # The layout came from the cache-msvc guest (spec addendum 8.2); the pinned bootstrapper is the fallback.
        $exe = Join-Path $p.arquivo 'layout\vs_buildtools.exe'
        if (-not (Test-Path -LiteralPath $exe)) { $exe = Join-Path $p.arquivo 'layout\vs_setup.exe' }
        if (-not (Test-Path -LiteralPath $exe)) { $exe = $p.bootstrapper }
        $argv = @('--noWeb', '--quiet', '--wait', '--norestart', '--nocache', '--installPath', "$Tools\msvc") + @($p.componentes | ForEach-Object { '--add'; $_ })
        $proc = Start-Process -FilePath $exe -ArgumentList $argv -Wait -PassThru
        $r.exit = ConvertTo-LabInstallExit $proc.ExitCode
        $vc = "$Tools\msvc\VC\Auxiliary\Build\vcvars64.bat"
        if ($r.exit -eq 0 -and -not (Test-Path -LiteralPath $vc)) { $r.exit = 1; $r.erro = 'vcvars64.bat not found after install' }
        if ($r.exit -eq 0) { $r.vcvars = $vc; $r.versaoVista = (cmd.exe /d /c "`"$vc`" >nul 2>&1 && where cl.exe" 2>&1 | Select-Object -First 1) }
      }
      'msi' {
        if (-not (Test-Sha $p.arquivo $p.sha256)) { throw 'sha256 differs from compose.json' }
        $log = "C:\lab\out\instalar-$($p.nome).log"
        $proc = Start-Process -FilePath msiexec.exe -ArgumentList @('/i', "`"$($p.arquivo)`"", '/qn', '/norestart', '/l*v', "`"$log`"") -Wait -PassThru
        $r.exit = ConvertTo-LabInstallExit $proc.ExitCode
        $rustc = Get-ChildItem -Path 'C:\Program Files\Rust*' -Recurse -Filter rustc.exe -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($r.exit -eq 0 -and -not $rustc) { $r.exit = 1; $r.erro = 'rustc.exe not found under C:\Program Files\Rust*' }
        if ($rustc) { $r.binDir = $rustc.DirectoryName; $r.versaoVista = (& $rustc.FullName --version 2>&1 | Select-Object -First 1) }
      }
      'zip' {
        if (-not (Test-Sha $p.arquivo $p.sha256)) { throw 'sha256 differs from compose.json' }
        Expand-Archive -LiteralPath $p.arquivo -DestinationPath "$Tools\node" -Force
        $node = Get-ChildItem -Path "$Tools\node" -Recurse -Filter node.exe | Select-Object -First 1
        if (-not $node) { throw 'node.exe not found after extract' }
        $r.binDir = $node.DirectoryName
        $r.versaoVista = (& $node.FullName --version 2>&1 | Select-Object -First 1)
        $r.exit = $(if ($LASTEXITCODE -eq 0) { 0 } else { $LASTEXITCODE })
      }
      'exe' {
        if (-not (Test-Sha $p.arquivo $p.sha256)) { throw 'sha256 differs from compose.json' }
        $d = "$Tools\claude"
        New-Item -ItemType Directory -Force $d | Out-Null
        Copy-Item -LiteralPath $p.arquivo -Destination "$d\claude.exe" -Force
        # Pinned version: no background update inside the guest (decision D2e).
        [Environment]::SetEnvironmentVariable('DISABLE_AUTOUPDATER', '1', 'Machine'); $env:DISABLE_AUTOUPDATER = '1'
        $r.binDir = $d
        $r.versaoVista = (& "$d\claude.exe" --version 2>&1 | Select-Object -First 1)
        $r.exit = $(if ($LASTEXITCODE -eq 0) { 0 } else { $LASTEXITCODE })
      }
      default { throw "unknown tipo '$($p.tipo)'" }
    }
  } catch { $r.exit = 1; $r.erro = $_.Exception.Message }
  $r.segundos = [math]::Round($sw.Elapsed.TotalSeconds, 1)
  return $r
}

foreach ($p in $wanted) {
  $r = Invoke-Package $p
  $res.pacotes += $r
  Save-Result
  if ($r.exit -ne 0) { break }   # later packages depend on earlier ones (Rust links with MSVC)
}
# Interactive shells in the guest also find the tools.
$dirs = @($res.pacotes | Where-Object { $_.binDir } | ForEach-Object { $_.binDir })
if ($dirs.Count) { [Environment]::SetEnvironmentVariable('Path', (($dirs + [Environment]::GetEnvironmentVariable('Path', 'Machine')) -join ';'), 'Machine') }
$res.ok = Test-LabInstallOk -Expected @($wanted | ForEach-Object { $_.nome }) -Results $res.pacotes
$res.fim = (Get-Date).ToString('o')
Save-Result
if ($res.ok) { exit 0 } else { exit 1 }
```

- [ ] **Passo 5: `guest-build.ps1`:** trocar o cabeçalho (linhas 1-2 sobre "toolchains do host") por "usa o que `instalar.ps1` instalou (`C:\lab\out\instalar.json`; ambiente `testes` do `compose.json`)"; dot-source `Join-Path $PSScriptRoot '..\guest-install.lib.ps1'`; substituir **toda** a seção `# --- ambiente: toolchains do host ...` até `Log "node=..."` por:

```powershell
# --- environment: what instalar.ps1 installed in this guest (C:\lab\out\instalar.json, ambiente 'testes') ---
$inst = $null
try { $inst = Get-Content -LiteralPath 'C:\lab\out\instalar.json' -Raw | ConvertFrom-Json } catch { }
$toolEnv = Get-LabGuestToolEnv $inst
if (-not $toolEnv) {
  Log 'instalar.json missing or not ok: start the lab with -Ambiente testes and wait for test-up to finish'
  # The host waits for a "build" step carrying its nonce: report the failure there instead of letting it time out.
  $res.steps['build'] = [ordered]@{ cmd = $Build; exit = 1; seconds = 0 }
  $res.end = (Get-Date).ToString('o'); Save
  exit 1
}
if ($toolEnv.vcvars) {
  # Import the MSVC environment (PATH, INCLUDE, LIB) the way a developer prompt does.
  foreach ($line in (cmd.exe /d /c "`"$($toolEnv.vcvars)`" >nul 2>&1 && set")) { if ($line -match '^([^=]+)=(.*)$') { Set-Item -Path "Env:$($Matches[1])" -Value $Matches[2] } }
}
$env:PATH = ((@($toolEnv.paths) + $env:PATH) -join ';')
$env:DISABLE_AUTOUPDATER = '1'
$env:CARGO_HOME = 'C:\labdata\cargo-home'
$env:CARGO_TARGET_DIR = 'C:\labdata\cargo-target'
$env:npm_config_cache = 'C:\labdata\npm-cache'
$res.env = [ordered]@{ pacotes = @(@($inst.pacotes) | ForEach-Object { "$($_.nome) $($_.versao)" }); cargoTarget = $env:CARGO_TARGET_DIR }
Log "node=$(cmd /c node -v 2>&1) cargo=$(cmd /c cargo -V 2>&1) cl=$(cmd /c where cl.exe 2>&1 | Select-Object -First 1)"
Save
```

O resto (cópia do projeto com `robocopy /E` para `C:\labdata\project`, passos `install`/`build`) fica igual.

- [ ] **Passo 6: Rodar e ver passar** `guest-install.test.ps1` (dois PowerShell); parser sem erros em `instalar.ps1`, `guest-build.ps1`, lib (dois PowerShell); ASCII conferido. Conferir que não sobrou `hosttools`: `git grep -n hosttools -- sandbox-lab` → só ocorrências em docs que a Tarefa 10 troca (nenhuma em `.ps1`). **Não executar `instalar.ps1` no PC.**

- [ ] **Passo 7: Provas de mutação:**
  1. `ConvertTo-LabInstallExit`: apagar a linha do 3010 → `FAILED - exit: ...`.
  2. `Test-LabInstallOk`: apagar o laço de ordem → `FAILED - ok: ...`.
  3. `Get-LabGuestToolEnv`: apagar `-or -not $Instalar.ok` → `FAILED - tool env: ...`.

- [ ] **Passo 8: Commit.**
```bash
git add sandbox-lab/sandbox/guest-install.lib.ps1 sandbox-lab/sandbox/instalar.ps1 sandbox-lab/sandbox/app/guest-build.ps1 sandbox-lab/scripts/test/guest-install.test.ps1
git commit -m "feat(lab): install MSVC, Rust, Node and Claude Code in the guest from the offline cache; guest-build uses them

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019iZMxAmFGsgenP4pjSBmb6"
```

---

### Tarefa 8: `test-up.ps1` lendo o `compose.json`, `test-down.ps1` apagando a cópia, perfis antigos saem

**Modelo:** Sonnet. Depende das Tarefas 1, 2, 6, 7.

**Files:**
- Modify: `sandbox-lab/scripts/test-up.ps1` (reescrita das seções de configuração, geração, início e espera)
- Modify: `sandbox-lab/scripts/test-down.ps1` (fim)
- Modify: `sandbox-lab/scripts/validate-config.ps1` (seções 1 e 3)
- Delete: `sandbox-lab/lab.config.json`, `sandbox-lab/lab.config.app.json`, `sandbox-lab/lab.config.app-winapp.json`, `sandbox-lab/lab.config.app-build.json` (`git rm`, arquivos versionados; não é deleção de dados)

**Interfaces:**
- Consumes: todo o contrato de `compose.ps1`, `travas.ps1`, `disco.ps1`; scripts do guest `disco-z.ps1`, `instalar.ps1`.
- Produces: `test-up.ps1 -Ambiente <nome> [-Project <p>] [-DadosReais] [-GenerateOnly] [-Force] [-NoWait] [-WaitForMcp] [-TimeoutSec n]` com exit 0/1/2 de `Get-LabUpVerdict`; `-Config` deixa de existir. A parte `-DadosReais` e o campo `msvc` só entram com o §8 da spec aprovado.
- Regra de disco: o `test-up` só **cria** pastas (`guest-rw`, `cache-saida`, `credencial`); os dados reais são só montados, só leitura, e nunca copiados para o PC.

- [ ] **Passo 1: Reescrever `test-up.ps1`** mantendo o que não muda (projeto com `Resolve-LabRealPath`/`Get-LabProjectRefusal`, checagem de ferramentas, detecção de Sandbox rodando, `-Force` com `Get-LabDownRefusal`, `Clear-LabOut`, laço de espera do `status.json`). Estrutura final, nesta ordem:

```powershell
<#
.SYNOPSIS
  Sobe o lab a partir de um ambiente de compose.json: valida (Get-LabComposeRefusal), recusa se o PC tiver Z:, gera
  sandbox\sandbox.generated.wsb e sandbox\guest.config.json, limpa shared\out, copia a credencial do Claude quando o
  ambiente pede (conta: true), inicia o Sandbox pelo wsb CLI e espera status.json, disco-z.json e instalar.json.
.PARAMETER Ambiente     Nome do ambiente em compose.json (padrao: exemplo).
.PARAMETER Project      Pasta do projeto montada read-only em C:\labproject (absoluta). Padrao: sandbox-lab\sample-project.
.PARAMETER TimeoutSec   Espera maxima pelo status.json (padrao: bootTimeoutSec do ambiente).
.PARAMETER GenerateOnly So gera os arquivos (validate-config.ps1); nao copia credencial nem sobe nada.
.PARAMETER Force        Se ja houver um Sandbox rodando, derruba antes (test-down.ps1).
.PARAMETER NoWait       Nao espera nada depois de iniciar.
.PARAMETER WaitForMcp   Espera tambem a fase MCP terminar.
.PARAMETER DadosReais   Monta SO LEITURA a pasta de dados do app no PC (%APPDATA%\io.github.johngabie.claudia-rh, achada
                        pela API) em C:\labdadosreais, para o cenario copia no guest. So com autorizacao do usuario NAQUELA
                        rodada. Nunca %LOCALAPPDATA%.
#>
[CmdletBinding()]
param([int]$TimeoutSec = 0, [switch]$GenerateOnly, [switch]$Force, [switch]$NoWait, [switch]$WaitForMcp, [string]$Ambiente = 'exemplo', [string]$Project, [switch]$DadosReais)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib\travas.ps1')
. (Join-Path $PSScriptRoot 'lib\compose.ps1')
. (Join-Path $PSScriptRoot 'lib\disco.ps1')
$Root = Split-Path $PSScriptRoot -Parent
$RepoRoot = Split-Path $Root -Parent
$rootProblem = Get-LabRootProblem $RepoRoot
if ($rootProblem) { throw "[test-up] repo root $rootProblem" }

# --- compose.json: pure checks before anything is written ---
$compose = Get-Content -LiteralPath (Join-Path $Root 'compose.json') -Raw | ConvertFrom-Json
$refusal = Get-LabComposeRefusal -Compose $compose -Ambiente $Ambiente
if ($refusal) { throw "[test-up] compose.json refused: $refusal" }
$driveRefusal = Get-LabHostDriveRefusal -DriveNames @([IO.DriveInfo]::GetDrives() | ForEach-Object { $_.Name })
if ($driveRefusal) { throw "[test-up] $driveRefusal" }
$amb = $compose.ambientes.PSObject.Properties[$Ambiente].Value
$cfg = $compose.comum
$conta = [bool]$amb.conta
$install = @(ConvertTo-LabGuestInstall -Compose $compose -Ambiente $Ambiente)
$mounts = @(Get-LabComposeMounts -Compose $compose -Ambiente $Ambiente)
Write-Host "[test-up] ambiente: $Ambiente (pacotes: $((@($install | ForEach-Object { $_.nome })) -join ', '); conta: $conta)"

function Resolve-LabPath([string]$p) {
  # compose.json holds only lab-relative paths (Get-LabComposeRefusal refuses absolute ones and variables).
  if ([IO.Path]::IsPathRooted($p)) { throw "[test-up] absolute path in compose.json: $p" }
  return [IO.Path]::GetFullPath((Join-Path $Root $p))
}
# (To-Guest e XmlEsc como hoje; $SharedDir, $OutDir, $SandboxDir como hoje.)
# $ProjectDir: -Project como hoje; sem -Project -> Resolve-LabPath 'sample-project'. Depois Resolve-LabRealPath +
# Get-LabProjectRefusal como hoje. $ExtDir = Resolve-LabPath $cfg.extensionsDir. Existencia das pastas como hoje.

# --- host folders behind the mount names (never written in compose.json) ---
$Work = Join-Path $RepoRoot '.sandbox-work'
$HostOf = @{ trabalho = (Join-Path $Work 'guest-rw'); deploy = (Join-Path $Work 'deploy'); 'cache-saida' = (Join-Path $Work 'cache-saida') }
foreach ($m in $mounts) {
  $h = $HostOf[$m.nome]
  if ($m.nome -in @('trabalho', 'cache-saida') -and -not $GenerateOnly) { New-Item -ItemType Directory -Force -Path $h | Out-Null }   # creates only
  $links = @(Get-LabLinksInPath $h $RepoRoot)
  if ($links.Count) { throw "[test-up] link (junction/symlink) in the path of mount '$($m.nome)': $($links[0])" }
  if (-not (Test-Path -LiteralPath $h -PathType Container)) { throw "[test-up] folder for mount '$($m.nome)' not found$(if ($m.nome -eq 'deploy') { ': publish first (publicar.mjs)' })" }
}
$cred = Get-LabCredentialPaths $RepoRoot

# --- real app data for the copia scenario (spec addendum 8.1): read-only, found by the API, only with -DadosReais ---
$RealData = $null
if ($DadosReais) {
  $roaming = [Environment]::GetFolderPath('ApplicationData')
  $src = Join-Path $roaming $script:LabAppId
  $why = Get-LabRealDataSourceRefusal -Source $src -RoamingAppData $roaming -Links (Get-LabLinksAboveRoot $src)
  if ($why) { throw "[test-up] -DadosReais refused: $why" }
  if (-not (Test-Path -LiteralPath $src -PathType Container)) { throw '[test-up] -DadosReais: the app has no data folder on this PC (%APPDATA%\io.github.johngabie.claudia-rh)' }
  $RealData = $src
  Write-Host '[test-up] dados reais do app: montados SO LEITURA em C:\labdadosreais (so com autorizacao do usuario nesta rodada)'
}
$msvcLayout = $(if (@($mounts | Where-Object { $_.nome -eq 'cache-saida' }).Count) { ConvertTo-LabGuestMsvcLayout -Compose $compose } else { $null })

if (-not $GenerateOnly) {
  if ($msvcLayout) {
    $boot = Join-Path $SharedDir ('cache\' + $msvcLayout.bootstrapper.Substring(($script:LabCacheGuestDir + '\').Length))
    if (-not (Test-Path -LiteralPath $boot -PathType Leaf)) { throw '[test-up] the MSVC bootstrapper is not in shared\cache: run scripts\cache-up.ps1 first. No Sandbox was started.' }
  }
  # (checagem de ferramentas Test-LabTools como hoje)
  # --- cache present (size only; the guest checks SHA-256) ---
  $missing = @()
  foreach ($p in $install) {
    $name = $p.arquivo.Substring(($script:LabCacheGuestDir + '\').Length)
    $h = Join-Path $SharedDir "cache\$name"
    if ($p.tipo -eq 'vs-layout') { if (-not (Test-Path -LiteralPath $h -PathType Container)) { $missing += $name } }
    else { $f = Get-Item -LiteralPath $h -ErrorAction SilentlyContinue; if (-not $f -or $f.Length -ne [long]$p.bytes) { $missing += $name } }
  }
  if ($missing.Count) { throw "[test-up] cache incomplete in shared\cache ($($missing -join ', ')): run scripts\cache-up.ps1 first. No Sandbox was started." }
}

# --- guest.config.json (read by bootstrap.ps1, bootstrap-mcp.ps1, instalar.ps1). No host path, no secret. ---
$extList = @($cfg.extensions | ForEach-Object { To-Guest (Join-Path $cfg.extensionsDir $_) })
$guest = [ordered]@{
  generatedAt = (Get-Date).ToString('o'); ambiente = $Ambiente
  startUrl = $cfg.chrome.startUrl; testServerPort = [int]$cfg.chrome.testServerPort; remoteDebuggingPort = [int]$cfg.chrome.remoteDebuggingPort
  chromeExe = To-Guest $cfg.chrome.exe; extensions = $extList; dbBackup = To-Guest $cfg.dbBackup; workDb = 'C:\labdata\app.sqlite'
  mcpPort = [int]$cfg.mcp.port; mcpAuthKeyEnv = $cfg.mcp.authKeyEnv; mcpAuthKeyFile = $(if ($cfg.mcp.authKeyFile) { To-Guest $cfg.mcp.authKeyFile } else { $null })
  outDir = 'C:\lab\out'; conta = $conta; dadosReais = [bool]$RealData; instalar = $install; msvc = $msvcLayout
}
$utf8 = New-Object System.Text.UTF8Encoding($false)
[IO.File]::WriteAllText((Join-Path $SandboxDir 'guest.config.json'), ($guest | ConvertTo-Json -Depth 6), $utf8)

# --- .wsb ---
$tpl = Get-Content (Join-Path $SandboxDir 'sandbox.template.wsb') -Raw
$map = @{
  '{{SHARED_DIR}}' = $SharedDir; '{{OUT_DIR}}' = $OutDir; '{{PROJECT_DIR}}' = $ProjectDir; '{{SANDBOX_DIR}}' = $SandboxDir
  '{{MEMORY_MB}}' = [string][int]$amb.memoriaMB; '{{VGPU}}' = [string]$cfg.vGPU
  '{{NETWORKING}}' = $(if ($amb.rede) { 'Enable' } else { 'Disable' }); '{{CLIPBOARD}}' = [string]$cfg.clipboardRedirection
}
foreach ($k in $map.Keys) { $tpl = $tpl.Replace($k, (XmlEsc $map[$k])) }
if ($tpl -match '\{\{[A-Z_]+\}\}') { throw "Placeholder nao substituido no template: $($Matches[0])" }
function New-LabMappedFolderXml([string]$HostDir, [string]$Sandbox, [bool]$ReadOnly) {
  return "    <MappedFolder>`r`n      <HostFolder>$(XmlEsc $HostDir)</HostFolder>`r`n      <SandboxFolder>$(XmlEsc $Sandbox)</SandboxFolder>`r`n      <ReadOnly>$(([string]$ReadOnly).ToLowerInvariant())</ReadOnly>`r`n    </MappedFolder>`r`n"
}
$extra = ''
foreach ($m in $mounts) { $extra += New-LabMappedFolderXml $HostOf[$m.nome] $m.sandbox $m.readOnly }
if ($conta) { $extra += New-LabMappedFolderXml $cred.Dir 'C:\labcred' $true }
if ($RealData) { $extra += New-LabMappedFolderXml $RealData 'C:\labdadosreais' $true }   # always read-only
if ($extra) { $tpl = $tpl.Replace('  </MappedFolders>', $extra + '  </MappedFolders>') }
$null = [xml]$tpl
$WsbPath = Join-Path $SandboxDir 'sandbox.generated.wsb'
[IO.File]::WriteAllText($WsbPath, $tpl, $utf8)
Write-Host "[test-up] gerado: $WsbPath"
if ($GenerateOnly) { return }

# (deteccao de Sandbox rodando, -Force e Clear-LabOut como hoje)
$StatusFile = Join-Path $OutDir 'status.json'

# --- start (wsb CLI required: the System fallback below uses wsb exec) ---
$wsb = Get-Command wsb -ErrorAction SilentlyContinue
if (-not $wsb) { throw "[test-up] 'wsb' not found: the lab needs the Windows Sandbox CLI. No Sandbox was started." }
$startGuest = {
  Write-Host '[test-up] iniciando via wsb CLI'
  $r = (& $wsb.Source start --config $tpl 2>&1) -join "`n"
  Write-Host $r
  $sid = [regex]::Match($r, '[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}').Value
  if (-not $sid) { throw "[test-up] nao consegui obter o Id do Sandbox da saida do wsb: $r" }
  Write-Host "[test-up] wsb connect --id $sid (abre a janela; dispara o LogonCommand)"
  Start-Process -FilePath $wsb.Source -ArgumentList @('connect', '--id', $sid) -WindowStyle Hidden
  return $sid
}
$id = Invoke-LabStartWithCredential -Conta $conta `
  -Copy { Copy-LabCredential -RepoRoot $RepoRoot -UserProfile ([Environment]::GetFolderPath('UserProfile')) -ConfigDir $env:CLAUDE_CONFIG_DIR } `
  -Start $startGuest `
  -Cleanup { Remove-LabCredential -RepoRoot $RepoRoot }
if ($conta) { Write-Host '[test-up] conta: login copiado para .sandbox-work\credencial (conteudo nunca exibido; test-down apaga)' }
if ($NoWait) { return }

# (laco de espera do status.json como hoje; timeout: -TimeoutSec ou $amb.bootTimeoutSec ou 240)

function Wait-LabJson([string]$File, [int]$Sec, [scriptblock]$Done) {
  $deadline = (Get-Date).AddSeconds($Sec); $last = $null
  while ((Get-Date) -lt $deadline) {
    if (Test-Path -LiteralPath $File) { try { $last = Get-Content -LiteralPath $File -Raw | ConvertFrom-Json; if (& $Done $last) { return $last } } catch { } }
    Start-Sleep -Seconds 3
  }
  return $last
}
function Invoke-LabGuestSystem([string]$Script, [switch]$Detached) {
  $cmd = "powershell -NoProfile -ExecutionPolicy Bypass -File $Script"
  if ($Detached) { Start-Process -FilePath $wsb.Source -ArgumentList "exec --id $id -r System -c `"$cmd`"" -WindowStyle Hidden | Out-Null }
  else { $null = & $wsb.Source exec --id $id -r System -c $cmd 2>&1 }
}
if ($st -and $st.elevated -eq $false) {
  Write-Host '[test-up] logon is not elevated: running disco-z.ps1 (and instalar.ps1) as System'
  Invoke-LabGuestSystem 'C:\labsandbox\disco-z.ps1'
  if ($install.Count) { Invoke-LabGuestSystem 'C:\labsandbox\instalar.ps1' -Detached }
}
$dz = Wait-LabJson (Join-Path $OutDir 'disco-z.json') 180 { param($j) $null -ne $j.ok }
$inst = $null
if ($install.Count) {
  $instTimeout = [int]$(if ($amb.instalarTimeoutSec) { $amb.instalarTimeoutSec } else { 3600 })
  Write-Host "[test-up] aguardando instalar.json (ate ${instTimeout}s)..."
  $inst = Wait-LabJson (Join-Path $OutDir 'instalar.json') $instTimeout { param($j) [bool]$j.fim }
}
$v = Get-LabUpVerdict -Status $st -DiscoZ $dz -Instalar $inst -PrecisaInstalar ($install.Count -gt 0)
if ($st) { $st | ConvertTo-Json -Depth 8 }
foreach ($p in @($inst.pacotes | Where-Object { $_ })) { Write-Host ("[test-up] instalar {0} {1}: exit={2} {3}s" -f $p.nome, $p.versao, $p.exit, $p.segundos) }
if ($v.exit -eq 0) { Write-Host "[test-up] OK: $($v.motivo)" -ForegroundColor Green } else { Write-Host "[test-up] $($v.motivo)" -ForegroundColor Yellow }
exit $v.exit
```

O `exit 2` antigo ("status.json nao apareceu") passa a vir de `Get-LabUpVerdict` (Status nulo → 2): remover o `Write-Error ...; exit 2` antigo do laço.

- [ ] **Passo 2: `test-down.ps1`:** dot-source `lib\travas.ps1` e `lib\disco.ps1`; trocar o fim (a partir de `if ($still) { ... exit 1 }`) por:

```powershell
$code = 0
if ($still) { Write-Warning "[test-down] ainda ativos: $($still | ForEach-Object { "$($_.Name)($($_.Id))" })"; $code = 1 }
else { Write-Host "[test-down] Sandbox encerrado." -ForegroundColor Green }
# The copy of the Claude login (spec A4) never outlives the Sandbox: only that file, through the pure guard.
$RepoRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
try { Remove-LabCredential -RepoRoot $RepoRoot } catch { Write-Warning "[test-down] login copy not removed: $($_.Exception.Message)"; $code = 1 }
exit $code
```

- [ ] **Passo 3: `validate-config.ps1`:** dot-source `lib\compose.ps1`; seção 1 lê `compose.json` (PASS/FAIL de JSON válido), roda `Get-LabComposeRefusal` para cada ambiente (um PASS/FAIL por ambiente), usa `$cfg = $compose.comum` nas checagens que sobram (chaves `extensionsDir`, `dbBackup`, `chrome`, `mcp`; caminho `projectPath` vira `'sample-project'`; tira `memoryMB` e `networking`); seção 3 chama `test-up.ps1 -GenerateOnly -Ambiente exemplo`. A lista de parse da seção 2 já cobre `scripts\lib` e `sandbox\`.

- [ ] **Passo 4: Remover os perfis antigos:** `git rm sandbox-lab/lab.config.json sandbox-lab/lab.config.app.json sandbox-lab/lab.config.app-winapp.json sandbox-lab/lab.config.app-build.json`.

- [ ] **Passo 5: Conferências no PC (sem subir nada):**
  - Parser sem erros em `test-up.ps1`, `test-down.ps1`, `validate-config.ps1` (dois PowerShell).
  - `git grep -n "lab\.config\|-Config \|hosttools\|ExpandEnvironmentVariables" -- sandbox-lab/scripts sandbox-lab/sandbox` → nada.
  - Testes puros de regressão: `compose.test.ps1`, `travas.test.ps1`, `guest-z.test.ps1`, `guest-install.test.ps1` (dois PowerShell) → `all passed`.
  - **Não rodar** `test-up.ps1` (nem `-GenerateOnly`: grava `guest.config.json`, que está montado num Sandbox que pode estar rodando) e **não rodar** `test-down.ps1` nem `validate-config.ps1`. Ficam para a Tarefa 12.

- [ ] **Passo 6: Prova de mutação da fiação (leitura, sem executar):** a regra "apaga a cópia se o start falhar" é provada pelo teste puro da Tarefa 1; aqui conferir por leitura que o `test-up.ps1` passa por `Invoke-LabStartWithCredential` e não chama `Copy-LabCredential` em nenhum outro lugar: `git grep -n "Copy-LabCredential" -- sandbox-lab/scripts/test-up.ps1` → uma linha, dentro do `-Copy`.

- [ ] **Passo 7: Commit.**
```bash
git add sandbox-lab/scripts/test-up.ps1 sandbox-lab/scripts/test-down.ps1 sandbox-lab/scripts/validate-config.ps1
git commit -m "feat(lab): test-up reads named environments from compose.json; test-down removes the login copy

Removes the lab.config*.json profiles and the host toolchain mappings.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019iZMxAmFGsgenP4pjSBmb6"
```
(Os `git rm` do Passo 4 já estão no índice e entram neste commit.)

---

### Tarefa 9: `cache-up.ps1` (downloads e importação do layout) e `layout-msvc.ps1` (layout montado no guest)

**Modelo:** Sonnet. Depende das Tarefas 1 e 2 e da aprovação do §8.

**Files:**
- Create: `sandbox-lab/scripts/cache-up.ps1`
- Create: `sandbox-lab/sandbox/layout-msvc.ps1`
- Test: `sandbox-lab/scripts/test/cache-up.test.ps1` (estático e puro: lê o texto dos dois scripts)

**Interfaces:**
- Consumes: `Get-LabPackageRefusal`, `$script:LabInstallOrder` (Tarefa 1); `Get-LabCacheDirRefusal`, `Get-LabCachePlan`, `Get-LabLayoutImportRefusal`, `Get-LabLayoutManifestProblems` (Tarefa 2); `Get-LabLinksInPath`, `Get-LabReparsePoints` (existentes); `guest.config.json` campo `msvc` (Tarefa 8, de `ConvertTo-LabGuestMsvcLayout`).
- Produces:
  - `scripts\cache-up.ps1 [-SoPlano]`: baixa Rust, Node, Claude Code e o **bootstrapper** do MSVC (`shared\cache\vs_buildtools-<v>.exe`, nunca executado no PC).
  - `scripts\cache-up.ps1 -ImportarMsvc <id>`: importa `.sandbox-work\cache-saida\<id>\msvc-<v>` para `shared\cache\msvc-<v>\{layout\,layout.sha256}`.
  - `sandbox\layout-msvc.ps1`: roda no guest como System no ambiente `cache-msvc` e grava `C:\lab\out\layout-msvc.json` `{ok,id,versao,arquivos,bytes,segundos,erro}`.
  - Em todos: um `shared\cache\cache-up-<data>.json` novo por execução.

Regra de disco no host (vale para o script inteiro): **só cria**. Nada é apagado nem sobrescrito, nem em `shared\cache\` nem em `.sandbox-work\cache-saida\` (o usuário limpa a `cache-saida` à mão quando quiser).

- [ ] **Passo 1: Teste estático que falha** — `sandbox-lab/scripts/test/cache-up.test.ps1`:

```powershell
# Pure test: reads the text of cache-up.ps1 and layout-msvc.ps1 (this repository). cache-up must never delete, move over,
# overwrite, or run anything of Visual Studio on the PC (spec A2 and addendum 8.2).
#   pwsh -NoProfile -File sandbox-lab/scripts/test/cache-up.test.ps1
$ErrorActionPreference = 'Stop'
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) { try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" } }
$text = Get-Content -LiteralPath "$PSScriptRoot/../cache-up.ps1" -Raw
$guest = Get-Content -LiteralPath "$PSScriptRoot/../../sandbox/layout-msvc.ps1" -Raw

Check 'cache-up has no delete or overwrite command' {
  $bad = '(?im)\b(Remove-Item|rmdir|rd|del|erase|Clear-Content|Clear-Item|Move-Item|Copy-Item|robocopy|Set-Content|Out-File)\b|\.Delete\(|\]::Replace\('
  $m = [regex]::Matches($text, $bad)
  if ($m.Count) { throw "found: $(@($m | ForEach-Object { $_.Value }) -join ', ')" }
  foreach ($c in [regex]::Matches($text, '\[IO\.File\]::Copy\([^)]*\)')) { if ($c.Value -notmatch ',\s*\$false\)$') { throw "File.Copy without overwrite=`$false: $($c.Value)" } }
}
Check 'cache-up never runs the Visual Studio bootstrapper on the PC' {
  $m = [regex]::Matches($text, '(?im)Start-Process|Invoke-Item|--layout|--add\b|--noWeb')
  if ($m.Count) { throw "found: $(@($m | ForEach-Object { $_.Value }) -join ', ')" }
}
Check 'cache-up checks with the pure guards and renames only new files and folders' {
  foreach ($must in @('Get-LabCacheDirRefusal', 'Get-LabCachePlan', 'Get-LabLayoutImportRefusal', 'Get-LabLayoutManifestProblems', 'Get-LabReparsePoints', '.tmp', '[IO.File]::Move(', '[IO.Directory]::Move(')) {
    if (-not $text.Contains($must)) { throw "missing: $must" }
  }
}
Check 'layout-msvc copies into a new folder and never mirrors' {
  if ($guest -match '(?i)/MIR|/PURGE|/MOV|Remove-Item') { throw 'mirror or delete in layout-msvc.ps1' }
  foreach ($must in @('C:\labcachesaida', 'layout.sha256', '--layout')) { if (-not $guest.Contains($must)) { throw "missing: $must" } }
}
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
```

- [ ] **Passo 2: Rodar e ver falhar** (arquivos não existem).

- [ ] **Passo 3: `sandbox-lab/scripts/cache-up.ps1`:**

```powershell
<#
.SYNOPSIS
  Cache de instaladores do guest em sandbox-lab\shared\cache\ (fora do git; spec A2 e adendo 8.2). So cria: nunca
  apaga, nunca sobrescreve. Um arquivo existente com outro hash e um conflito: nada e baixado.
  Sem parametros: baixa Rust, Node, Claude Code e o bootstrapper do MSVC. O bootstrapper NUNCA roda no PC: o layout do
  MSVC e montado num Sandbox (test-up -Ambiente cache-msvc + sandbox\layout-msvc.ps1 como System) e trazido com
  -ImportarMsvc, que confere SHA-256 e tamanho de cada arquivo no PC antes e depois de copiar.
.PARAMETER SoPlano       Mostra o plano e sai sem baixar nada.
.PARAMETER ImportarMsvc  Id (8 hex) gravado pelo layout-msvc.ps1 em shared\out\layout-msvc.json.
#>
[CmdletBinding()]
param([switch]$SoPlano, [string]$ImportarMsvc)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
. (Join-Path $PSScriptRoot 'lib\travas.ps1')
. (Join-Path $PSScriptRoot 'lib\compose.ps1')
. (Join-Path $PSScriptRoot 'lib\disco.ps1')
$Lab = Split-Path $PSScriptRoot -Parent
$RepoRoot = Split-Path $Lab -Parent
$Cache = Join-Path $Lab 'shared\cache'
$refusal = Get-LabCacheDirRefusal -CacheDir $Cache -LabDir $Lab -Links @(Get-LabLinksInPath $Cache $Lab)
if ($refusal) { throw "[cache-up] refusing to write in ${Cache}: $refusal" }
$compose = Get-Content -LiteralPath (Join-Path $Lab 'compose.json') -Raw | ConvertFrom-Json
$msvc = $compose.pacotes.PSObject.Properties['msvc'].Value
$utf8 = New-Object System.Text.UTF8Encoding($false)
New-Item -ItemType Directory -Force -Path $Cache | Out-Null
$log = [ordered]@{ inicio = (Get-Date).ToString('o'); acao = $(if ($ImportarMsvc) { 'importar-msvc' } else { 'baixar' }); itens = @() }
function Save-RunLog { $log.fim = (Get-Date).ToString('o'); [IO.File]::WriteAllText((Join-Path $Cache ('cache-up-{0}.json' -f (Get-Date -Format 'yyyyMMdd-HHmmss'))), ($log | ConvertTo-Json -Depth 4), $utf8) }
function Get-LabShortId { return [guid]::NewGuid().ToString('N').Substring(0, 8) }
function Get-LabFileStamp([string]$File) { return "$((Get-FileHash -LiteralPath $File -Algorithm SHA256).Hash.ToLowerInvariant()) $((Get-Item -LiteralPath $File).Length)" }
function Get-LabTreeStamps([string]$Dir) {
  # relative path with '/' -> "<sha256> <bytes>" for every file under $Dir (links were refused before)
  $base = (Get-Item -LiteralPath $Dir).FullName.TrimEnd('\') + '\'
  $found = @{}
  foreach ($f in Get-ChildItem -LiteralPath $Dir -Recurse -File -Force) { $found[$f.FullName.Substring($base.Length).Replace('\', '/')] = Get-LabFileStamp $f.FullName }
  return $found
}

if ($ImportarMsvc) {
  # --- import the layout built in the guest (adendo 8.2) ---
  if ($ImportarMsvc -notmatch '^[0-9a-f]{8}$') { throw '[cache-up] -ImportarMsvc takes the 8-hex id from shared\out\layout-msvc.json' }
  $why = Get-LabPackageRefusal -Nome 'msvc' -Pacote $msvc
  if ($why) { throw "[cache-up] compose.json: $why" }
  $src = Join-Path $RepoRoot ".sandbox-work\cache-saida\$ImportarMsvc\msvc-$($msvc.versao)"
  $links = @(Get-LabLinksInPath $src $RepoRoot) + @(Get-LabReparsePoints $src)
  $why = Get-LabLayoutImportRefusal -Source $src -RepoRoot $RepoRoot -Versao $msvc.versao -Links $links
  if ($why) { throw "[cache-up] refusing to import ${src}: $why" }
  if (-not (Test-Path -LiteralPath (Join-Path $src 'layout.sha256') -PathType Leaf)) { throw "[cache-up] $src has no layout.sha256: run layout-msvc.ps1 again" }
  $final = Join-Path $Cache $msvc.arquivo
  if (Test-Path -LiteralPath $final) { throw "[cache-up] $($msvc.arquivo) is already in the cache: nothing imported (nothing is ever overwritten)" }
  $manifest = Get-Content -LiteralPath (Join-Path $src 'layout.sha256') -Raw
  Write-Host '[cache-up] conferindo o layout vindo do guest (SHA-256 de cada arquivo)...'
  $problems = @(Get-LabLayoutManifestProblems -ManifestText $manifest -Found (Get-LabTreeStamps (Join-Path $src 'layout')))
  if ($problems.Count) { throw "[cache-up] the layout in cache-saida does not match its layout.sha256 ($($problems.Count)): $(($problems | Select-Object -First 5) -join '; '). Nothing copied." }
  $tmp = "$final.$(Get-LabShortId).tmp"
  [IO.Directory]::CreateDirectory($tmp) | Out-Null
  $srcLayout = (Get-Item -LiteralPath (Join-Path $src 'layout')).FullName.TrimEnd('\') + '\'
  foreach ($f in Get-ChildItem -LiteralPath $srcLayout -Recurse -File -Force) {
    $dst = Join-Path $tmp ('layout\' + $f.FullName.Substring($srcLayout.Length))
    [IO.Directory]::CreateDirectory((Split-Path $dst -Parent)) | Out-Null
    [IO.File]::Copy($f.FullName, $dst, $false)
  }
  [IO.File]::Copy((Join-Path $src 'layout.sha256'), (Join-Path $tmp 'layout.sha256'), $false)
  $problems = @(Get-LabLayoutManifestProblems -ManifestText $manifest -Found (Get-LabTreeStamps (Join-Path $tmp 'layout')))
  if ($problems.Count) { throw "[cache-up] the copy differs from layout.sha256 ($($problems.Count)); partial folder kept as $(Split-Path $tmp -Leaf), nothing renamed" }
  [IO.Directory]::Move($tmp, $final)   # throws if $final exists
  $log.itens += [ordered]@{ nome = 'msvc'; versao = $msvc.versao; id = $ImportarMsvc }
  Save-RunLog
  Write-Host "[cache-up] layout do MSVC $($msvc.versao) importado para shared\cache\$($msvc.arquivo)." -ForegroundColor Green
  exit 0
}

# --- downloads (Rust, Node, Claude Code, MSVC bootstrapper); the MSVC layout folder is only reported ---
$pkgs = @()
foreach ($nome in $script:LabInstallOrder) {
  $prop = $compose.pacotes.PSObject.Properties[$nome]
  if (-not $prop) { continue }
  $p = $prop.Value
  $why = Get-LabPackageRefusal -Nome $nome -Pacote $p
  if ($why) { throw "[cache-up] compose.json: $why" }
  if ($p.tipo -eq 'vs-layout') {
    $pkgs += [pscustomobject]@{ nome = 'msvc-bootstrapper'; arquivo = $p.bootstrapper.arquivo; sha256 = $p.bootstrapper.sha256; url = $p.bootstrapper.url }
    $pkgs += [pscustomobject]@{ nome = 'msvc'; arquivo = $p.arquivo; sha256 = ''; url = $null }
  } else {
    $pkgs += [pscustomobject]@{ nome = $nome; arquivo = $p.arquivo; sha256 = $p.sha256; url = $p.url }
  }
}
$existentes = @{}
foreach ($k in $pkgs) {
  $path = Join-Path $Cache $k.arquivo
  if (Test-Path -LiteralPath $path -PathType Container) { $existentes[$k.arquivo] = 'dir' }
  elseif (Test-Path -LiteralPath $path -PathType Leaf) { $existentes[$k.arquivo] = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() }
}
$plan = @(Get-LabCachePlan -Pacotes $pkgs -Existentes $existentes)
foreach ($p in $plan) { Write-Host ("[cache-up] {0,-18} {1,-9} {2}" -f $p.nome, $p.acao, $p.motivo) }
$conflicts = @($plan | Where-Object { $_.acao -eq 'conflito' })
if ($conflicts.Count) { throw "[cache-up] $($conflicts.Count) conflict(s): nothing was downloaded and the existing files were kept. Decide by hand what to do with them." }
if ($SoPlano) { exit 0 }

function Save-LabDownload([string]$Url, [string]$Final, [string]$Sha) {
  # curl.exe ships with Windows 10+. New .tmp next to the final name; rename only after the hash matches.
  $tmp = "$Final.$(Get-LabShortId).tmp"
  & curl.exe -fL --retry 3 --silent --show-error -o $tmp $Url
  if ($LASTEXITCODE -ne 0) { throw "download failed ($LASTEXITCODE): $Url (partial file kept as $(Split-Path $tmp -Leaf))" }
  $got = (Get-FileHash -LiteralPath $tmp -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($got -ne $Sha) { throw "sha256 differs for ${Url}: expected $Sha, got $got (file kept as $(Split-Path $tmp -Leaf); nothing renamed)" }
  [IO.File]::Move($tmp, $Final)   # throws if $Final exists: never overwrites
}
foreach ($item in @($plan | Where-Object { $_.acao -eq 'baixar' -and $_.nome -ne 'msvc' })) {
  $k = $pkgs | Where-Object { $_.nome -eq $item.nome }
  $sw = [Diagnostics.Stopwatch]::StartNew()
  Write-Host "[cache-up] baixando $($k.nome)..."
  Save-LabDownload -Url $k.url -Final (Join-Path $Cache $k.arquivo) -Sha $k.sha256
  $log.itens += [ordered]@{ nome = $k.nome; arquivo = $k.arquivo; segundos = [math]::Round($sw.Elapsed.TotalSeconds, 1) }
}
Save-RunLog
if (@($plan | Where-Object { $_.nome -eq 'msvc' -and $_.acao -eq 'baixar' }).Count) {
  Write-Host '[cache-up] falta o layout do MSVC. Ele e montado num Sandbox (o PC nao executa o Visual Studio):' -ForegroundColor Yellow
  Write-Host '  1) scripts\test-up.ps1 -Ambiente cache-msvc -Project <projeto>   2) wsb exec --id <id> -r System -c "powershell -NoProfile -ExecutionPolicy Bypass -File C:\labsandbox\layout-msvc.ps1"'
  Write-Host '  3) ler o id em shared\out\layout-msvc.json   4) scripts\test-down.ps1   5) scripts\cache-up.ps1 -ImportarMsvc <id>'
}
exit 0
```

- [ ] **Passo 4: `sandbox-lab/sandbox/layout-msvc.ps1`** (5.1, ASCII):

```powershell
# layout-msvc.ps1 - runs INSIDE the Sandbox as System (wsb exec -r System), ambiente cache-msvc (spec addendum 8.2).
# Builds the offline layout of the VS Build Tools on the guest disk, lists every file with size and SHA-256
# (layout.sha256), and copies it into a NEW folder C:\labcachesaida\<id>\msvc-<version> (= <repo>\.sandbox-work\cache-saida
# on the host). The host checks and imports it with cache-up.ps1 -ImportarMsvc <id>. Output: C:\lab\out\layout-msvc.json
$ErrorActionPreference = 'Continue'   # PS 5.1: native stderr must not become a terminating error
$OutFile = 'C:\lab\out\layout-msvc.json'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$sw = [Diagnostics.Stopwatch]::StartNew()
$res = [ordered]@{ ok = $false; id = $null; versao = $null; arquivos = 0; bytes = 0; segundos = $null; erro = $null }
function Save-Result { $res.segundos = [math]::Round($sw.Elapsed.TotalSeconds, 1); [IO.File]::WriteAllText($OutFile, ($res | ConvertTo-Json), $utf8) }
function Stop-With([string]$Why) { $res.erro = $Why; Save-Result; exit 1 }

if (-not (Test-Path -LiteralPath 'C:\labsandbox\bootstrap.ps1' -PathType Leaf)) { Stop-With 'not inside the lab guest' }
$cfg = Get-Content -LiteralPath 'C:\labsandbox\guest.config.json' -Raw | ConvertFrom-Json
$m = $cfg.msvc
if (-not $m) { Stop-With 'guest.config.json has no msvc entry: start the lab with -Ambiente cache-msvc' }
if (-not (Test-Path -LiteralPath 'C:\labcachesaida' -PathType Container)) { Stop-With 'C:\labcachesaida is not mounted' }
$res.versao = $m.versao
Save-Result
if ((Get-FileHash -LiteralPath $m.bootstrapper -Algorithm SHA256).Hash -ine $m.sha256) { Stop-With 'bootstrapper sha256 differs from compose.json' }
$work = 'C:\labdata\msvc-layout'   # short path on the guest disk (Microsoft asks for less than 80 characters)
if (Test-Path -LiteralPath $work) { Stop-With "$work already exists: run on a fresh guest" }
New-Item -ItemType Directory -Force 'C:\labdata' | Out-Null
$boot = 'C:\labdata\vs_buildtools.exe'
[IO.File]::Copy($m.bootstrapper, $boot, $false)
$argv = @('--layout', $work, '--lang', 'en-US', '--quiet', '--wait') + @($m.componentes | ForEach-Object { '--add'; $_ })
$p = Start-Process -FilePath $boot -ArgumentList $argv -Wait -PassThru
if ($p.ExitCode -ne 0) { Stop-With "vs_buildtools --layout exit $($p.ExitCode)" }
# '<sha256> <bytes> <relative/path>' per file, same shape as shared/tools.sha256.
$base = (Get-Item -LiteralPath $work).FullName.TrimEnd('\') + '\'
$lines = New-Object System.Collections.Generic.List[string]
foreach ($f in Get-ChildItem -LiteralPath $work -Recurse -File -Force) {
  $lines.Add(('{0} {1} {2}' -f (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $f.Length, $f.FullName.Substring($base.Length).Replace('\', '/')))
  $res.arquivos++; $res.bytes += $f.Length
}
$id = [guid]::NewGuid().ToString('N').Substring(0, 8)
$dest = "C:\labcachesaida\$id\msvc-$($m.versao)"
New-Item -ItemType Directory -Path "$dest\layout" | Out-Null
robocopy $work "$dest\layout" /E /XJ /NFL /NDL /NP /R:1 /W:1 | Out-Null   # new folder: /E only, never a mirror
if ($LASTEXITCODE -ge 8) { Stop-With "robocopy exit $LASTEXITCODE" }
[IO.File]::WriteAllLines("$dest\layout.sha256", $lines.ToArray())
$res.id = $id
$res.ok = $true
Save-Result
exit 0
```

- [ ] **Passo 5: Rodar e ver passar** `cache-up.test.ps1` (dois PowerShell); parser sem erros em `cache-up.ps1` e `layout-msvc.ps1`; ASCII em `layout-msvc.ps1`. **Não executar** `cache-up.ps1` (baixa vários GB) nem `layout-msvc.ps1` (Tarefa 12).

- [ ] **Passo 6: Provas de mutação** (colar `FAILED`, restaurar):
  1. Acrescentar `Remove-Item -LiteralPath $tmp` dentro de `Save-LabDownload` → `FAILED - cache-up has no delete or overwrite command`.
  2. Trocar um `[IO.File]::Copy(..., $false)` por `..., $true)` → mesmo teste falha.
  3. Acrescentar `Start-Process -FilePath (Join-Path $Cache $msvc.bootstrapper.arquivo)` → `FAILED - cache-up never runs the Visual Studio bootstrapper on the PC`.

- [ ] **Passo 7: Commit.**
```bash
git add sandbox-lab/scripts/cache-up.ps1 sandbox-lab/sandbox/layout-msvc.ps1 sandbox-lab/scripts/test/cache-up.test.ps1
git commit -m "feat(lab): cache-up downloads pinned installers and imports the MSVC layout built in the Sandbox (create-only, hash-checked)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019iZMxAmFGsgenP4pjSBmb6"
```

---

### Tarefa 10: Docs, skill e CI

**Modelo:** Haiku (mecânica; o texto novo está abaixo). Depende de 1–9.

**Files:**
- Modify: `.github/workflows/ci.yml` (job `repo-guard`, passo PowerShell)
- Modify: `sandbox-lab/README.md`, `sandbox-lab/OBJECTIVES.md`, `sandbox-lab/sample-project/README.md`, `sandbox-lab/scripts/app-stage.ps1` (só o comentário `.SYNOPSIS`)
- Modify: `.claude/skills/claudia-teste-sandbox/SKILL.md` (seções 1, 2, 5, 7 e 9), `references/laboratorio.md` (seção "Perfis"), `references/limites.md` (itens 1 e 4)

- [ ] **Passo 1: CI.** No passo `pure guard tests (PowerShell)`, trocar o `run` por:
```yaml
        run: |
          ./sandbox-lab/scripts/test/travas.test.ps1; if ($LASTEXITCODE) { exit $LASTEXITCODE }
          ./sandbox-lab/scripts/test/compose.test.ps1; if ($LASTEXITCODE) { exit $LASTEXITCODE }
          ./sandbox-lab/scripts/test/cache-up.test.ps1; if ($LASTEXITCODE) { exit $LASTEXITCODE }
          ./sandbox-lab/scripts/test/guest-z.test.ps1; if ($LASTEXITCODE) { exit $LASTEXITCODE }
          ./sandbox-lab/scripts/test/guest-install.test.ps1; if ($LASTEXITCODE) { exit $LASTEXITCODE }
```
(Cada script sai com 1 em falha; o `if ($LASTEXITCODE)` garante que o passo falha no primeiro script que falhar, em vez de seguir com o código do último.) No passo `pure guard tests (node)`, acrescentar à lista explícita `.claude/skills/claudia-teste-sandbox/scripts/test/cenario-no-guest.test.mjs`.

- [ ] **Passo 2: SKILL.md.**
  - Seção 1: acrescentar "Instaladores do guest (`sandbox-lab/shared/cache/`) ficam fora do git; se o `test-up.ps1` disser que o cache está incompleto, **pare e diga ao usuário** para rodar `sandbox-lab\scripts\cache-up.ps1` (baixa alguns GB; só cria, nunca apaga)."
  - Seção 2: trocar as linhas de perfis por: `scripts\test-up.ps1 -Ambiente <nome> -Project <projeto>`; tabela de ambientes do `compose.json`: `app` (sem rede, `wsb exec`), `app-winapp` (rede, internet bloqueada por `guest-firewall.ps1`), `testes` (MSVC, Rust 1.95.0, Node 22.23.3 e Claude Code 2.1.285 instalados no guest a partir do cache; conta Claude logada no PC; resultado em `C:\lab\out\instalar.json`), `exemplo`. Pastas do guest: acrescentar `Z:\claudia-teste-sandbox` e `Z:\tmp` (disco só do guest) e `C:\labcred` (login copiado, RO, só no `testes`).
  - Seção 4 (cenários, D1): os cenários rodam **só no guest**, em `Z:\claudia-teste-sandbox\<cenario>`. Do PC: `node .claude/skills/claudia-teste-sandbox/scripts/cenario-no-guest.mjs --cenario novo|mock|copia [--autorizado] [--manter-automacao]` (Sandbox de pé num ambiente que instala `node`: `app`, `app-winapp` ou `testes`). O `copia` exige **duas** autorizações do usuário naquela rodada: subir com `test-up.ps1 -DadosReais` (monta só leitura a pasta Roaming do app em `C:\labdadosreais`; nunca `%LOCALAPPDATA%`) e passar `--autorizado`. A cópia dos dados reais existe só no `Z:` do guest e some no `test-down`. Tirar o texto antigo sobre `%TEMP%` e `--saida` no PC.
  - Seção 5: `publicar.mjs --app <exe|instalador|pasta>` publica **só o app** (sem `--cenario`); os dados entram no guest com `guest-run.ps1 -Cenario Z:\claudia-teste-sandbox\<cenario> [-NoStart]`. "Build dentro do guest: ambiente `testes`". Fluxo típico: `publicar.mjs` → `test-up.ps1 -Ambiente app-winapp` → `guest-firewall.ps1` → `cenario-no-guest.mjs` → `guest-run.ps1 -Cenario ...` → observar → `test-down.ps1`.
  - Seção 1 (cache): o layout do MSVC é montado num Sandbox: `test-up.ps1 -Ambiente cache-msvc`, depois `wsb exec --id <id> -r System -c "powershell -NoProfile -ExecutionPolicy Bypass -File C:\labsandbox\layout-msvc.ps1"`, ler o `id` em `shared\out\layout-msvc.json`, `test-down.ps1` e `cache-up.ps1 -ImportarMsvc <id>`. O PC nunca executa o instalador do Visual Studio; `.sandbox-work\cache-saida\` fica para o usuário limpar à mão.
  - Seção 7: trocar "não faça login em contas reais no guest" por "não faça login manual no guest; a conta do Claude só entra pelo ambiente `testes` (o `test-up` copia o login do PC e o `test-down` apaga a cópia). Cada sessão do Claude no guest gasta a cota do usuário."
  - Seção 9: Sandbox → `test-up.ps1 -Ambiente testes -Project <projeto>`; os testes criam pastas só em `Z:\claudia-teste-sandbox`; puros: acrescentar `compose.test.ps1`, `cache-up.test.ps1`, `guest-z.test.ps1`, `guest-install.test.ps1` e dizer que o CI roda os cinco `.test.ps1`.
- [ ] **Passo 3: `references/laboratorio.md`:** seção "Perfis (`-Config`)" vira "Ambientes (`-Ambiente`, `compose.json`)" com a mesma tabela da SKILL; tirar a frase sobre `%USERPROFILE%`/`%ProgramFiles%`; tabela "Pastas no guest" ganha `Z:`, `C:\labcred`, `C:\labtools`.
- [ ] **Passo 4: `references/limites.md`:** item 1 → ambiente `app`; item 4 → ambiente `testes`. `references/pastas-de-dados.md`: dizer que a cópia dos dados reais acontece só dentro do guest (`C:\labdadosreais` só leitura → `Z:`).
- [ ] **Passo 5: `sandbox-lab/README.md`:** tabela "versionado / fora do git": `compose.json` no lugar de `lab.config*.json`; `shared/cache/` nos ignorados; nova seção curta "Cache de instaladores" (`scripts\cache-up.ps1 [-SoPlano]`, só cria, conflito para tudo, tamanho aproximado: Rust 304 MB, Node 36 MB, Claude Code 244 MB, MSVC **não verificado**). `OBJECTIVES.md` e `sample-project/README.md`: trocar menções a `lab.config*.json` por `compose.json`/ambiente `exemplo`. `app-stage.ps1`: o `.SYNOPSIS` cita "ambientes `app` e `app-winapp` do `compose.json`".
- [ ] **Passo 6: Conferências:** `git grep -n "lab\.config\|hosttools\|-Config " -- sandbox-lab .claude/skills` → só em `docs/` históricos (nada em `sandbox-lab/` e `.claude/skills/`); `node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs` → exit 0; parser em `app-stage.ps1`.
- [ ] **Passo 7: Commit** (lista explícita dos arquivos tocados), mensagem `docs(lab): compose.json environments, installer cache, Z: drive and login copy in the skill and README; CI runs the new pure tests` + rodapé.

---

### Tarefa 11: Revisão de código (Opus)

**Modelo:** Opus, independente de quem escreveu. Skill `superpowers:requesting-code-review`.

- [ ] **Passo 1:** Ler a spec, este plano e o diff inteiro da branch desde `9b135f8`.
- [ ] **Passo 2:** Conferir, com arquivo:linha:
  - Todo caminho que apaga (`Remove-LabCredentialFile`, `Clear-LabOut`, `novoCenarioDir`, `disco-z.ps1` formatando) passa antes pela função pura certa e não aceita raiz de disco, repositório, pasta do usuário nem link.
  - A credencial nunca aparece em log, `status.json`, `guest.config.json`, mensagem de erro (incluindo exceções repassadas) nem no `.wsb` (o `.wsb` só tem a **pasta** `.sandbox-work\credencial`).
  - `compose.json` sem caminho do host; `Get-LabComposeRefusal` roda antes de qualquer escrita no `test-up`.
  - `cache-up.ps1` sem deleção nem sobrescrita, e sem executar nada do Visual Studio no PC; a importação confere SHA-256 antes **e** depois de copiar e só renomeia no fim.
  - Dados reais (D1): só a pasta Roaming do app, só leitura, só com `-DadosReais`; nenhum script de cenário roda no PC (`raizCenariosAtual` lança antes de qualquer acesso a disco); `cenario-copia` com `fs.cpSync(..., dereference: true)` a partir de `C:\labdadosreais`: conferir que um link dentro da origem não leva a cópia para fora do que está montado.
  - Os cinco itens do Review Focus têm teste e o teste falha se a regra quebrar (refazer uma mutação de amostra).
  - Compatibilidade PowerShell 5.1 nos arquivos de `sandbox/` (sem `??`, `?.`, ternário, `-Parallel`) e ASCII.
  - Nenhum caminho absoluto de máquina (`conferir-repo.mjs`).
- [ ] **Passo 3:** Rodar de novo todos os testes puros (lista explícita, dois PowerShell) e `conferir-repo.mjs`, conferindo o exit code de cada um.
- [ ] **Passo 4:** Relatório com achados (bloqueante / não bloqueante). Bloqueantes voltam para a tarefa dona; não bloqueantes viram issue no Linear (skill `linear-claudia`).

---

### Tarefa 12: Verificação no Sandbox (roteiro)

**Modelo:** Opus. **Só com autorização explícita do usuário nesta rodada** para: (a) rodar `cache-up.ps1` (baixa alguns GB) e montar o layout do MSVC num Sandbox, (b) derrubar um Sandbox que esteja rodando e subir os do laboratório (são três subidas: `cache-msvc`, `testes`, `app-winapp`), (c) usar a conta Claude logada no PC (gasta cota), (d) opcional: `-DadosReais` + `copia --autorizado` com os dados reais do app. Sem uma delas, pular os passos que dependem dela e relatar como não verificado.

Comandos a partir da raiz do checkout principal (`onde.mjs` → `lab`, `projeto`). Toda saída do guest é lida em `sandbox-lab\shared\out\` e `.sandbox-work\guest-rw\` (o `wsb exec` não devolve saída). Nunca abrir nem imprimir `.credentials.json`.

- [ ] **Passo 1a: Downloads no PC.** `pwsh -NoProfile -File sandbox-lab\scripts\cache-up.ps1 -SoPlano` (plano) → `cache-up.ps1` (exit 0; baixa Rust, Node, Claude Code e o bootstrapper; avisa que falta o layout do MSVC). Conferir no Gerenciador de Tarefas/`Get-Process` que nenhum `vs_buildtools`/`vs_setup` rodou no PC.
- [ ] **Passo 1b: Layout do MSVC num Sandbox.** `test-up.ps1 -Ambiente cache-msvc -Project <projeto>` (exit 0) → `wsb exec --id <id> -r System -c "powershell -NoProfile -ExecutionPolicy Bypass -File C:\labsandbox\layout-msvc.ps1"` → esperar `shared\out\layout-msvc.json` com `ok: true`; anotar `id`, `arquivos`, `bytes`, `segundos` → `test-down.ps1` (exit 0).
- [ ] **Passo 1c: Importar no PC.** `cache-up.ps1 -ImportarMsvc <id>` → exit 0, `shared\cache\msvc-<versão>\layout\` e `layout.sha256` presentes. Prova de recusa sem apagar nada: rodar `-ImportarMsvc <id>` de novo → recusa com "already in the cache", nada muda. `.sandbox-work\cache-saida\<id>\` continua lá (não é limpo pelo script).
- [ ] **Passo 1d: Idempotência.** Listagem antes e depois de **mais uma** execução sem parâmetros, gravada no scratchpad:
  `Get-ChildItem sandbox-lab\shared\cache -Recurse -File | Select-Object FullName, Length, LastWriteTimeUtc | ConvertTo-Json > <scratch>\cache-antes.json` e o mesmo para `cache-depois.json`; esperado: o plano imprime só `ok`, e as listagens são iguais exceto o novo `cache-up-<data>.json`.
- [ ] **Passo 2: Validação estática.** `pwsh -NoProfile -File sandbox-lab\scripts\validate-config.ps1` → `O1: OK`.
- [ ] **Passo 3: Estado da conta no PC, só metadados.** `Get-Item "$env:USERPROFILE\.claude\.credentials.json" | Select-Object Length, LastWriteTimeUtc` (anotar) e `claude -p "diga ok"` no PC → responde.
- [ ] **Passo 4: Subir.** `pwsh -NoProfile -File sandbox-lab\scripts\test-up.ps1 -Ambiente testes -Project <projeto do onde.mjs>` → exit 0. Ler `shared\out\status.json` (`bootstrap: ok`, `elevated`, `discoZ`, `conta: presente`), `shared\out\disco-z.json` (`ok: true`, `segundos`), `shared\out\instalar.json` (`ok: true`, cada pacote `exit: 0`, tempos). Anotar se o LogonCommand era elevado (fecha o "não verificado" do admin).
- [ ] **Passo 5: Versões.** Do `instalar.json`: `versaoVista` do rust = `rustc 1.95.0 ...`, do node = `v22.23.3`, do claude-code começando com `2.1.285`, do msvc um caminho de `cl.exe`.
- [ ] **Passo 6: `cargo test --lib` no guest.** Com o id de `wsb list`:
  `wsb exec --id <id> -r ExistingLogin -c "powershell -NoProfile -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-build.ps1 -Precisa cargo -Install 'npm.cmd ci --no-audit --no-fund' -Build 'npm.cmd run build && cd src-tauri && cargo test --lib'"`
  Esperar `.sandbox-work\guest-rw\build\result.json` com `end` e ler `steps.install.exit` e `steps.build.exit` = 0; log em `guest-build.log`. (O `npm run build` antes é porque o `tauri::generate_context!` precisa da pasta do front: **não verificado** se é necessário para `--lib`.)
- [ ] **Passo 7: Os 6 testes de Sandbox**, um comando por arquivo (exit 0 em cada):
  ```
  node .claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs --node .claude/skills/claudia-teste-sandbox/scripts/test/comum.test.mjs
  node .claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs --node .claude/skills/claudia-teste-sandbox/scripts/test/novo.test.mjs
  node .claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs --node .claude/skills/claudia-teste-sandbox/scripts/test/mock.test.mjs
  node .claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs --node .claude/skills/claudia-teste-sandbox/scripts/test/copia.test.mjs
  node .claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs --node .claude/skills/claudia-teste-sandbox/scripts/test/publicar.test.mjs
  node .claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs --ps sandbox-lab/scripts/test/travas.sandbox.ps1
  ```
  Conferir no `guest-build.log` que as pastas criadas começam com `Z:\claudia-teste-sandbox` e que nada foi criado em `C:\labdata\tmp`.
- [ ] **Passo 8: Claude Code com a conta no guest.**
  `wsb exec --id <id> -r ExistingLogin -c "cmd /c if not exist C:\labwork\run mkdir C:\labwork\run && C:\labtools\claude\claude.exe -p \"diga ok\" > C:\labwork\run\claude-p.txt 2>&1"`
  Ler `.sandbox-work\guest-rw\run\claude-p.txt` → contém a resposta. Depois gravar só metadados do login no guest:
  `wsb exec --id <id> -r ExistingLogin -c "powershell -NoProfile -Command \"Get-Item $env:USERPROFILE\.claude\.credentials.json | Select-Object Length, LastWriteTimeUtc | ConvertTo-Json | Out-File C:\labwork\run\cred-meta.json\""`
  (comparar a data com a da cópia: mudou = o guest renovou o token).
- [ ] **Passo 9: Busca do token.** `node .claude/skills/claudia-teste-sandbox/scripts/conferir-token.mjs --pasta sandbox-lab/shared/out --pasta .sandbox-work/guest-rw` → `"achados":[]`, exit 0.
- [ ] **Passo 10: Risco da renovação (A4).** No PC: `claude -p "diga ok"` → ainda responde; metadados do login do PC de novo (Passo 3). Registrar: se o guest renovou (Passo 8) e o PC continua funcionando → risco descartado nesta rodada; se o guest **não** renovou → continua **não verificado** (a sessão curta não forçou renovação); se o PC pedir login → **parar**, relatar e propor o recuo da spec (token de longa duração, opção 2).
- [ ] **Passo 11: Derrubar.** `pwsh -NoProfile -File sandbox-lab\scripts\test-down.ps1` → exit 0; `Test-Path .sandbox-work\credencial\.credentials.json` → `False`; nenhum `.credentials.json.*.tmp` na pasta.
- [ ] **Passo 11b: Cenários no guest e publicar só o app (D1).** Publicar um build do app: `node .claude/skills/claudia-teste-sandbox/scripts/publicar.mjs --app <exe ou instalador>` (exit 0; `.sandbox-work\deploy` só com `app\` e o marcador). `test-up.ps1 -Ambiente app-winapp -Project <projeto>` (exit 0; `instalar.json` só com `node` em exit 0) → `guest-firewall.ps1` como System (**conferir o exit 0 e o `C:\labwork\run\firewall.txt` com as linhas `Domain/Private/Public enabled=True`, `regra lab-block-internet-out: 1`, `INTERNET: BLOQUEADA (...)` e `storage.googleapis.com: BLOQUEADO` ANTES de qualquer `guest-run`**; sem isso, parar: o `guest-run.ps1` do `app-winapp` recusa abrir o app e sai com 1, e uma recusa dele é o resultado esperado, não um defeito) → `cenario-no-guest.mjs --cenario mock` (exit 0, `saida` = `Z:\claudia-teste-sandbox\mock`) → `wsb exec --id <id> -r ExistingLogin -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1 -Cenario Z:\claudia-teste-sandbox\mock -NoStart"` → `guest-run.json` com `dataFiles.appdata` > 0 e sem `errors`. Recusas: `cenario-no-guest.mjs --cenario copia` sem `--autorizado` → exit 2; com `--autorizado` mas sem `-DadosReais` → `cenario.json` com `exit: 2` e a mensagem do `-DadosReais`. **Não** rodar `cenario-*.mjs` no PC para "provar a recusa": eles apagam a pasta de saída, e a recusa fora do guest já é provada pelo teste puro de `raizDeCenarios` (Tarefa 4) e pela leitura de `raizCenariosAtual` (primeira linha de `novoCenarioDir` e do `try` do `cenario-copia`).
- [ ] **Passo 11c (só com a autorização (d)): `copia` com dados reais.** `test-down.ps1`; `test-up.ps1 -Ambiente app-winapp -Project <projeto> -DadosReais` → `cenario-no-guest.mjs --cenario copia --autorizado` → exit 0. Conferir no guest que `C:\labdadosreais` é só leitura (uma tentativa de escrita grava erro em `C:\labwork\run\`), e no PC que a pasta de dados do app não mudou (listagem de nomes, tamanhos e datas antes e depois, sem abrir arquivos). `test-down.ps1`.
- [ ] **Passo 12: Fechamento.** `node .claude/skills/claudia-teste-sandbox/scripts/conferir-repo.mjs` → exit 0. Relatório com comandos, números e caminhos relativos ao repositório; tudo o que não rodou vai como "não verificado". Atualizar `references/limites.md` com o que foi verificado (data, comando, resultado) num commit de docs.

---

## Não verificado (2026-10-07)

- Se o `LogonCommand` do Sandbox roda com privilégio de administrador (o plano trata os dois casos; a Tarefa 12, Passo 4, responde).
- Se `Get-DiskImage`/`Get-Partition`/`Format-Volume` funcionam no guest como o `diskpart` funcionou na investigação (a investigação provou só o `diskpart` com a letra V:).
- Versão exata e bootstrapper de versão fixa do VS 2022 Build Tools (a fixar na Tarefa 1, Passo 1); tamanho do layout offline (VCTools + Windows 11 SDK 26100) e tempo para montá-lo no guest `cache-msvc`; se o `--layout` precisa de administrador (por isso roda como System); se o layout montado no guest depende de algo daquela máquina; se a cópia de alguns GB para a pasta mapeada gravável (`C:\labcachesaida`) é lenta ou falha; nome do executável dentro do layout (`vs_buildtools.exe` ou `vs_setup.exe`; o `instalar.ps1` tenta os dois e cai no bootstrapper fixado).
- Adendo §8 da spec (cenários só no guest, `-DadosReais`, layout do MSVC no Sandbox): **aprovado pelo usuário em 2026-10-07**.
- Se o banco do app montado só leitura em `C:\labdadosreais`, com o app aberto no PC, copia de forma consistente (o `copia` consolida o WAL na cópia e tenta até 3 vezes, como antes; o comportamento por pasta mapeada não foi testado).
- Se `guest-run.ps1 -Cenario` com `robocopy /E /XJ` reproduz o que o deploy `appdata` fazia antes (contagem de arquivos e atributos).
- Se a instalação `--noWeb` a partir do layout num **pasta montada só leitura** funciona no guest, e se precisa importar os certificados da pasta `Certificates` do layout; tempo de instalação do MSVC e do Rust a cada boot.
- Pasta onde o MSI do Rust instala por padrão (o plano procura `rustc.exe` em `C:\Program Files\Rust*`).
- Se o `claude.exe` copiado (sem `claude install`) roda no guest como `WDAGUtilityAccount`, e se `claude -p` precisa de algo além de `.credentials.json` (ex.: `.claude.json` de onboarding).
- Se a renovação do token pelo guest invalida o login do PC (risco da spec A4; Tarefa 12, Passo 10).
- Se `cargo test --lib` precisa do `npm run build` antes (pasta do front para o `generate_context!`).
- Valores da pesquisa conferidos por download dos arquivos oficiais de checksum em 2026-10-07: hashes e tamanhos de Rust 1.95.0 (MSI), Node 22.23.3 (zip) e Claude Code 2.1.285 (`manifest.json`). A documentação do Claude Code confirma: credencial no Windows em `%USERPROFILE%\.claude\.credentials.json`, `CLAUDE_CONFIG_DIR` muda a pasta, `DISABLE_AUTOUPDATER=1` desliga a atualização em segundo plano, binário em `downloads.claude.ai/claude-code-releases/<versão>/win32-x64/claude.exe` com SHA-256 no `manifest.json`. A documentação do rustup diz que o MSVC só é necessário para compilar, não para instalar o Rust.

---

## Adendo de 2026-10-07: correções da revisão Opus (decisões do usuário)

- **I2, "seguir sem parar":** `instalar.ps1` não para mais no primeiro pacote que falha; registra cada um e segue (Rust, Node, Claude Code), com `ok` global falso se algum falhar. `guest-install.lib.ps1` ganhou `Test-LabInstallNeed` e `Get-LabGuestToolEnv -Precisa todos|node|cargo`: `node` (testes de Sandbox e cenários) só exige o Node instalado com exit 0 e a instalação terminada; `cargo` exige MSVC e Rust ok; `todos` (padrão) exige `ok`. Só entram no PATH e no `vcvars` os pacotes com exit 0. `guest-build.ps1 -Precisa` (padrão `todos`), `testes-no-guest.mjs` passa `-Precisa node` e `guest-cenario.ps1` usa `node`. O veredito do `test-up` (`Get-LabUpVerdict`) segue com exit 1 e o motivo, agora com "installed ok: ...", sem esconder a falha do MSVC.
- **I1:** `travas.sandbox.ps1` usa `New-FakeRepo 'k2'` no segundo bloco. **I3:** `Get-LabRealDataSourceRefusal -LinksInside` (o `test-up` passa `Get-LabReparsePoints` da origem) e `cenario-copia.mjs` com `dereference: false` + filtro que lança em link (`copia-filtro.mjs`, puro). A cópia continua sem reproduzir links; agora recusa em vez de seguir.
- **Minor 2:** `Get-LabUpVerdict -PrecisaConta`: com `conta: true`, exige `status.conta == 'presente'`. **Minor 8:** `Get-LabRealDataAmbienteRefusal`: `-DadosReais` só em `app` e `app-winapp`, recusado antes de gerar, montar ou copiar. **Minor 1:** o `test-up` chama `Remove-LabCredential` assim que o `status.json` mostra `conta: presente` (o `bootstrap.ps1` lê `C:\labcred` uma vez só, no passo 2c, e copia para o perfil do guest); `test-down` e o start que falha continuam apagando. **Minor 5:** `Test-LabGuestScriptDecision`/`Test-LabInGuest` (marcador `bootstrap.ps1` mais identidade `WDAGUtilityAccount`, ou SYSTEM decidido por `IsSystem` com o perfil `WDAGUtilityAccount` presente) em `instalar`, `disco-z`, `layout-msvc`, `guest-run`, `guest-cenario` e `guest-build`.
- **Docs/CI:** formato do `cenario.json` corrigido; CI com `motivo-recusa`, `onde`, `sandbox` e `copia-filtro` (puros por leitura); `SKILL.md` aponta o `cargo test --lib` para o `guest-build.ps1 -Precisa cargo`; comentário de `travas.ps1` sem `-AppData`/`-LocalAppData`.
- **Não verificado:** nada disto rodou no Sandbox (só testes puros, parser e `node --check`). Se `IsSystem` é verdadeiro e `C:\Users\WDAGUtilityAccount` existe no guest quando o `wsb exec -r System` roda `disco-z`/`instalar`; se o `Get-LabReparsePoints` da pasta de dados reais é rápido com o perfil grande; se a pasta de dados reais do app tem links hoje (`diagnostics\latest`): nesse caso o `-DadosReais` passa a recusar até o link sair da origem.

## Adendo de 2026-10-07 (2): segunda revisão Opus (decisões do usuário)

- **Travas provadas por teste:** `sandbox-lab/scripts/test/guest-travas.test.ps1` (estático e puro, lê o texto pelo parser, não executa nada) confere, para `instalar`, `disco-z`, `layout-msvc`, `bootstrap-mcp`, `guest-run`, `guest-cenario`, `guest-build` e `guest-firewall`, que a chamada `Test-LabInGuest` (fora de função) vem antes do primeiro efeito (`New-Item`, `Save-Result`, `robocopy`, `Start-Process`, `WriteAllText`, firewall, etc.), que a lib é carregada antes dela e que a linha da trava só faz `exit 1`, sem gravar nada. Confere também as ligações `-Precisa node` (`testes-no-guest.mjs`, `guest-cenario.ps1`), `-PrecisaConta` e `-LinksInside`/`-FileLinksInside` no `test-up.ps1`, e a função da remoção cedo da credencial. No CI.
- **Firewall e MCP:** `guest-firewall.ps1` carrega `guest-install.lib.ps1` e exige a trava (roda como System; o perfil WDAG e o marcador existem no guest). `bootstrap-mcp.ps1` exige a trava, exceto em `-DryRun`; o `bootstrap.ps1` o inicia como usuário do guest, então os dois fatores valem. O `bootstrap.ps1` não pode usar o marcador (ele é o marcador) e não foi alterado.
- **Recusa sem gravar:** em `instalar`, `layout-msvc` e `disco-z` a trava só faz `Write-Host` e `exit 1` (no `disco-z`, fora do `try`, para não cair no `Save-Result`).
- **Pequenos:** `-Precisa cargo` exige msvc, rust e node. `Test-LabCredentialRemovableNow` (pura, testada) decide a remoção cedo da credencial no `test-up`. `-LinksInside` sem `@()`. `limites.md` alinhado ao `SKILL.md`.
- **Exceção de link (decisão do usuário, "segue"):** o único link aceito na origem do `copia` é `diagnostics\latest` quando é link de **arquivo** (logger de depuração do Claude Code, aponta para `claude-startup.log`): é pulado, não seguido nem copiado. Qualquer outro link, o mesmo nome como pasta/junction ou em outro nível recusa. Node: `motivoLinkNaOrigem(rel, {ehLink, ehPasta})` devolve `"pular"`, `"recusar"` ou `null` (sem `stat` ou com `stat` falhando conta como pasta); PowerShell: `Get-LabRealDataSourceRefusal -FileLinksInside` (o `test-up` passa `Get-LabFileLinks`, que lê só atributos). Testes puros com mutação nos dois lados.
- **Não verificado:** nada rodou no Sandbox; que o `fs.statSync` de um link de arquivo devolve `isDirectory() === false` (o teste usa fakes); que o link real tem o atributo ReparsePoint sem Directory; a trava do `bootstrap-mcp` e do firewall no guest de verdade.

## Adendo de 2026-10-07 (3): terceira revisão Opus (decisões do usuário)

- **Firewall que falha não abre o app com internet:** `compose.json` ganhou `"internet": false` no `app-winapp` (só vale com `rede: true`; validado em `Get-LabComposeRefusal`). O `test-up` grava `internetBloqueada` no `guest.config.json` (`Test-LabAmbienteInternetBlocked`, sem caminho do host). O `guest-run.ps1` lê esse campo (`Get-LabGuestInternetPermitida`, fail closed: sem arquivo ou sem booleano = bloqueada) e `C:\labwork\run\firewall.txt`, e só inicia o app se `Get-LabAppStartRefusal` (puro, em `guest-install.lib.ps1`) não recusar: exige os 3 perfis `enabled=True`, exatamente uma `regra lab-block-internet-out`, `INTERNET: BLOQUEADA (` e `storage.googleapis.com: BLOQUEADO`, e nenhuma linha `ACESSIVEL`. Recusa: erro em `guest-run.json` e exit 1, sem `Stop-Process` nem `Start-Process`. A Tarefa 12 (Passo 11b) confere o exit e o `firewall.txt` antes de qualquer `guest-run`.
- **Exceção de `diagnostics\latest` confere o destino:** o link só é tolerado se o destino, lido sem seguir o link (`Read-LabLinkTarget`: `.Target`, e `.LinkTarget` no pwsh 7), for exatamente `claude-startup.log` ou `<Source>\diagnostics\claude-startup.log` (sem diferenciar maiúsculas, `/` lido como `\`). `Test-LabLatestLinkTargetOk` e `Get-LabRealDataSourceRefusal -LinkTargets` são puros; qualquer outro destino (`..`, 8.3, UNC, outra letra, vazio, ilegível, mais de um) recusa. `Get-LabFileLinks` deixa de listar link cujo destino não se lê.
- **Travas por lista de permitidos:** `guest-travas.test.ps1` foi reescrito sobre a árvore do parser (nada é executado). A trava tem de ser um `if` no nível de cima, com a condição exatamente `-not (Test-LabInGuest)` e corpo só `Write-Host`/`Write-Output` + `exit 1`; antes dela só `param`, atribuições simples, funções e o dot-source de `guest-*.lib.ps1`; nenhuma função do script se chama `Test-LabInGuest`. Para isso `instalar`, `layout-msvc` e `disco-z` passaram a ter a trava logo depois do dot-source (antes de `New-Object`/`GetCurrent`), e o `bootstrap-mcp` perdeu o desvio `-not $DryRun -and` (o `-DryRun` agora também exige o guest). 14 mutações em memória (parser) e 6 em cópias no scratchpad dão `FAILED` nomeado.
- **Important 4 (a conferir na Tarefa 12):** o `stat` do link no guest pode falhar e recusar a cópia; se acontecer, é recusa segura, mas precisa ser vista ao vivo.
- **Não verificado:** nada rodou no Sandbox; que `.Target`/`.LinkTarget` de um link de arquivo real devolve o texto como foi criado (relativo ou absoluto, sem prefixo `\??\`); que o link real do Claude Code aponta para `claude-startup.log` num desses dois formatos (se não, a cópia recusa, o que é o lado seguro); o filtro de `cenario-copia.mjs` (Node) não confere o destino do link.

## Adendo de 2026-10-07 (4): correção do `return ,$links` (contagem de links sempre 1)

- **Bug:** `Get-LabLinksInPath` fazia `return ,$links`; `@(Get-LabLinksInPath ...)` virava um array com UM elemento (o array vazio), então `.Count` dava 1 sem link algum. O `test-up` (laço dos mounts) e o `Copy-LabCredential` recusavam sempre (`link ... in the path of mount 'cache-saida': ` com motivo vazio). Os demais chamadores passavam a lista a guardas puras que filtram com `Where-Object { $_ }`, por isso não quebravam. `Get-LabLinksAboveRoot` (`return ,@(...)`) devolvia a lista aninhada duas vezes.
- **Correção na raiz:** `Get-LabLinksInPath`, `Get-LabReparsePoints` e `Get-LabFileLinks` devolvem a lista plana (`return $x`, sem vírgula). Nova função pura `ConvertTo-LabLinkList` (`travas.ps1`) achata e filtra qualquer retorno (nada, `$null`, string, array, array dentro de array, vazios) em strings não vazias. TODO chamador escreve `@(ConvertTo-LabLinkList (Get-Lab...))`; `Get-LabLinksAboveRoot` já devolve pela função. Assim `@(...).Count` é o número real de links, e `[0]` é o caminho. As funções puras (`Get-LabMissingTools`, `Get-LabComposeStrings`, `Test-LabTools`) mantêm `return ,$x` de propósito: seus chamadores usam atribuição ou `foreach`, que desembrulham uma vez, e nenhum usa `@(f)`.
- **Testes:** `travas.test.ps1` (puro) imita os retornos reais (`,@()`, `,@('Y:\inventado\a')`, dois links, `$null`, aninhado) e confere tamanho e tipo. `guest-travas.test.ps1` (estático, parser) exige que toda chamada dos quatro finders esteja dentro de `ConvertTo-LabLinkList` (exceto linha marcada `# raw-call-on-purpose`, só no teste de Sandbox) e que as quatro definições não usem vírgula unária; com mutações em memória. `travas.sandbox.ps1` ganhou o caso real (pasta sem link = 0, junction criada pelo teste = 1), que roda só no guest.
- **Não verificado:** o caso novo de `travas.sandbox.ps1` e o `test-up` com mounts reais não rodaram (só parser e testes puros).

## Adendo de 2026-10-08 (5): revisão Opus do commit `80dcb38` (layout incompleto e ligação do `disco-z`)

- **Layout incompleto não é mais aceito:** `Get-LabVsLayoutRefusal` (`guest-install.lib.ps1`) recusa, mesmo com `Completed 'Layout' operation`, um stdout com marcador de falha (`fail*`, linha `]e:`, `exception`, `error:`/`error 0x`, HRESULT `0x8xxxxxxx`) e uma pasta de layout sem `Catalog.json` ou sem `vs_buildtools.exe`/`vs_setup.exe` (nomes do primeiro nível, passados por `-LayoutFiles` pelo `layout-msvc.ps1`). Marcadores escolhidos contra o log real de sucesso (zero ocorrências); `error` solto e `corrupt` ficaram de fora de propósito (nome do pacote `ErrorList`; a linha `Skipping layout with missing or corrupt payload` aparece em toda rodada). O trecho do log real é fixture do teste.
- **`disco-z` com teste estático:** a criação do VHD virou `Get-LabZCreateDiskPart` (puro, testado como `Get-LabZFormatDiskPart`); `guest-travas.test.ps1` prova pela árvore do parser que todo `Invoke-DiskPart` recebe só um dos dois construtores, que a formatação vem depois de `if ($s.format) { throw }` e que não há comando de disco nem literal `select disk`/`clean` no script; 8 mutações com `FAILED` nomeado.
- **Minors:** cada `robocopy` do `layout-msvc.ps1` grava seu código em `layout-msvc.json` e falha com 8 ou mais; a etapa 2 ganhou o limite de 15 minutos da etapa 1 e mata só o processo que o próprio script iniciou.
- **Não verificado:** nada rodou no Sandbox (só testes puros/estáticos e parser); os marcadores `]e:` e `0x8xxxxxxx` não foram vistos em nenhum log real.

## Adendo de 2026-10-08 (6): exit -1 do Node no `instalar.ps1`

- **Bug (visto ao vivo, 2026-10-08):** `(& $node.FullName --version 2>&1 | Select-Object -First 1)` seguido de `$LASTEXITCODE` dava -1 no Windows PowerShell 5.1 (o `Select-Object -First` interrompe o processo nativo), com o Node funcionando (`versaoVista` = `v22.23.3`). O mesmo padrao existia no Claude Code e, so para a versao, no Rust e no MSVC.
- **Correcao:** captura da saida inteira, `$code = $LASTEXITCODE` logo depois, primeira linha so no fim. A decisao e a funcao pura `ConvertTo-LabVersionProbe` (`guest-install.lib.ps1`): so o exit 0 e sucesso, -1 com linha de versao NAO e aceito, sem codigo = 1. `guest-install.test.ps1` cobre a regra; `guest-travas.test.ps1` prova pela arvore do parser que nenhum `Select-Object -First`/`-Last` vem direto depois de comando nativo em `sandbox/*.ps1` e `sandbox/app/*.ps1` (4 mutacoes com `FAILED` nomeado). O `guest-build.ps1` tambem foi ajustado (`where cl.exe`).
- **Nao verificado:** nada rodou no Sandbox (so testes puros/estaticos e parser, em pwsh e Windows PowerShell 5.1); o exit 0 do Node num guest novo fica para a proxima rodada. Item 16 de `.claude/skills/claudia-teste-sandbox/references/limites.md`.

## Adendo de 2026-10-08 (7): dois bugs achados ao vivo (Tarefa 12)

- **Bug 1, segunda cópia da credencial:** `[IO.File]::Replace($tmp, $p.File, $null)` falhava no guest (`The path is not of a legal form`): o PowerShell passa `$null` como `""` a um parâmetro `string`. Agora o terceiro argumento é `[NullString]::Value` (continua atômico: a cópia antiga só é trocada quando a nova existe). `guest-travas.test.ps1` lê a árvore do parser de `sandbox-lab/scripts` e `sandbox-lab/sandbox` e falha, com arquivo e linha, se algum `[IO.File]::Replace` receber `$null` ou `''`; 3 mutações em `disco.ps1`. O caso `credential: a second copy replaces the first` de `travas.sandbox.ps1` já existia e não foi rodado.
- **Bug 2a, `copia` com `diagnostics\latest` real:** o link é de arquivo com destino absoluto do PC; no guest o `statSync` falhava e o link contava como pasta. O Node não distingue link de arquivo de link de pasta/junction sem seguir (`lstatSync` só diz "link"; `readlinkSync` devolve o texto do destino, sem o tipo), então **a decisão é do PC**: `test-up` calcula `Test-LabLatestLinkApproved` (link de arquivo, destino exato, mesmas checagens da recusa) e grava em `guest.config.json` só o booleano `dadosReaisLatestLink` (sem caminho). `guest-cenario.ps1` lê o fato (`Test-LabGuestLatestLinkApproved`: `true` literal e `dadosReais` verdadeiro) e passa `--latest-link-aprovado` ao `cenario-copia.mjs`. `filtroSemLinks` agora só recebe `lstat` (nunca `stat`/`readlink`): pula exatamente `diagnostics\latest` quando aprovado, qualquer outro link, ou o mesmo nome sem aprovação, lança.
- **Bug 2b, cópia parcial:** os scripts de cenário já gravam `cenario.json` por último (a cópia, a consolidação dos bancos e o desligamento da automação vêm antes; um teste de ordem sobre o texto do `cenario-copia.mjs` trava isso). Novo: `guest-run.ps1 -Cenario` lê o `cenario.json` (arquivo comum, não link) e recusa a pasta se faltar, estiver vazio ou inválido (`Get-LabGuestScenarioJsonRefusal`, puro: objeto com `cenario` em novo/mock/copia, `criadoEm` e lista `arquivos`), antes de copiar qualquer dado. A cópia parcial não é apagada. O marcador `.cenario` continua sendo criado no início (o `novoCenarioDir` precisa dele para refazer a pasta), por isso não vale como prova de término.
- **Não verificado:** nada rodou no Sandbox (só testes puros/estáticos e parser, pwsh e Windows PowerShell 5.1; `node --test` dos puros). `[NullString]::Value` com `File.Replace` no guest e o `copia` com os dados reais ficam para a próxima rodada ao vivo. Itens 13 a 17 de `limites.md`.
