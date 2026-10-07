# Auditoria de efeitos colaterais dos testes e scripts (2026-10-06)

Tarefa 0 do plano `docs/superpowers/plans/2026-10-06-fechar-branch-browser.md`. Dois auditores Opus fizeram a leitura e não executaram nada: um cobriu os testes Rust, o outro os testes e scripts JS/TS/PowerShell e os comandos do próprio plano. O orquestrador conferiu o achado principal no código.

Motivo da auditoria: em 2026-10-05 um teste apagou o disco C do usuário, e cerca de 12 horas de trabalho se perderam.

**Resultado: há `BLOQUEIA`. Nenhum teste do plano roda antes de as correções da seção 1 entrarem.**

## 1. BLOQUEIA: corrigir antes de qualquer execução

| # | Onde | O que acontece | Correção |
|---|---|---|---|
| B1 | `.claude/skills/claudia-teste-sandbox/scripts/test/comum.test.mjs:122-134` | Os testes "refuses if target is REPO_ROOT / ancestor / filesystem root" chamam `novoCenarioDir()`, que faz `fs.rmSync(dir, {recursive, force})`, passando **o repositório, a pasta acima dele e `C:\`**. Se a trava falhar (mutação, TDD que roda o teste antes da trava, regressão), o disco é apagado. É o candidato mais provável ao incidente de 2026-10-05 (a primeira versão de `novoCenarioDir` apagava sem trava nenhuma). Conferido pelo orquestrador. | Separar a validação numa função pura `motivoRecusa(dir)` que só devolve o motivo. Os testes de recusa testam **só** essa função. As três chamadas a `novoCenarioDir` com caminho real saem. |
| B2 | `.claude/skills/claudia-teste-sandbox/scripts/comum.mjs:34-59` (`novoCenarioDir`) | A trava é **lista negra**: recusa raiz, repositório e ancestral. Fora disso, apaga qualquer pasta que tenha `cenario.json` ou `.cenario`, inclusive em `%APPDATA%`. Não cobre `subst`, nome 8.3, `\\?\`, `\\localhost\c$`, junction no alvo, nem `TEMP=""`. A skill ainda incentiva usar `--saida` em outro disco. | Trocar por **lista branca**: só apaga dentro de `os.tmpdir()/claudia-teste-sandbox/`, com o caminho real (`realpathSync.native`), sem reparse point e com o marcador. Um `--saida` fora disso só é aceito se não existir ou estiver vazio, e nunca é apagado. |
| B3 | `.claude/skills/claudia-teste-sandbox/scripts/publicar.mjs:18-20` + `<laboratório externo>/scripts/app-stage.ps1` | Sempre passa `-Clean`, que roda `Remove-Item <work-dir>\deploy -Recurse -Force` e depois `robocopy /MIR`. O `<work-dir>` vem de argv ou de `LAB_WORK_DIR` sem validação. O script do laboratório fica fora do repositório, e o grep da auditoria não o alcança. | Exigir um marcador `.lab-deploy` e ausência de reparse point antes do `-Clean`, nos dois lados. |
| B4 | `.claude/skills/claudia-teste-e2e/scripts/subir.mjs:62-65` | Sobe o app sobre os **dados reais** com a automação ligada: `disparo.json` com `ativo:true` e limiar de 15 min, `modo_autonomo.json` com `skip_permissions:true`. O `idle_watcher` pode abrir uma sessão real de candidatura, com `--dangerously-skip-permissions`, enquanto o teste roda. | Abortar se `disparo.ativo !== false` ou `skip_permissions === true`. O usuário desliga na UI; o agente não escreve nos dados reais. |
| B5 | `.claude/skills/claudia-teste-e2e/scripts/invocar.mjs:24` | O JSON do argv é concatenado cru numa expressão JS, então dá para injetar outro `invoke` e contornar a classificação de leitura e escrita. Com `--escrever`, qualquer comando chega ao app real (`apagar_*`, `instalar_atualizacao`, `iniciar_sessao`). `obter_credencial` conta como leitura e devolve a senha em texto puro. | `JSON.parse` do argumento e `JSON.stringify` na expressão. Lista negra (`apagar_`, `excluir_`, `remover_`, `limpar_`, `desinstalar`, `instalar_`, `iniciar_`, `disparar_`, `enviar_`) mesmo com `--escrever`, liberada só com `--confirmo=<comando>`. `obter_credencial` sempre recusado. |
| B6 | `.claude/skills/*/evals/rodar-evals.mjs:21-24` | Lança dezenas de `claude -p` no repositório, com o modo de permissão `auto` do usuário. Entre as consultas há "desinstalação", "corrige este bug", "roda cargo test". A primeira ferramenta executa de verdade, o kill não pega a árvore de processos, e cada execução tem custo. | `--permission-mode plan` e `--disallowedTools` para Bash, PowerShell, Edit, Write, NotebookEdit e WebFetch; matar com `taskkill /PID <pid> /T /F`. Só roda com o usuário autorizando. |
| B7 | `src-tauri/src/browser/service.rs:120,507` (5 testes de `prepare`) | O `fake_spawn` devolve o PID fixo **555**. Num caminho de erro, roda o `taskkill /PID 555 /T /F` **real**, que mata qualquer processo da máquina que tenha esse número, com a árvore inteira. | Injetar o kill em `prepare`: a produção passa `manager::kill_tree`, os testes passam `\|_\| {}`. |

**Decisões do usuário, item por item (2026-10-06)** (valem sobre a tabela acima):

| # | Decisão |
|---|---|
| B1 | **Corrigir.** "Foi esse que ferrou meu sistema. Nunca mais faz isso." O teste nunca mais chama a função que apaga com caminho real. |
| B2 | **Endurecer** (lista branca dentro de `%TEMP%\claudia-teste-sandbox\`). Princípio do usuário: **teste cujo objetivo é apagar ou limpar coisas roda só no Sandbox.** Teste normal que só limpa a própria pasta temporária continua rodando na máquina. |
| B3 | **Pôr trava:** só apaga o `deploy` que tiver o marcador criado pelo próprio `publicar`. (O `deploy` é limpo antes de copiar, não depois da rodada.) |
| B4 | **Não recusar:** quando o agente sobe o app para teste, ele sobe com o disparo por ociosidade e o modo autônomo **desligados**, por uma variável que só vale no build de desenvolvimento. Os arquivos do usuário e o app normal ficam iguais. O `--dangerously-skip-permissions` em si fica para o usuário resolver depois. |
| B5 | **Lista negra no `--escrever`** (`apagar_`, `excluir_`, `remover_`, `limpar_`, `desinstalar`, `instalar_`, `iniciar_`, `disparar_`, `enviar_` só com `--confirmo=<comando>`) e **`obter_credencial` sempre recusado**. O parse do JSON não foi pedido. |
| B6 | **Linear, depois.** Fora do caminho do plano; os evals continuam só manuais. |
| B7 | **Higiene, não bloqueia:** o kill falso nos testes entra junto com a Tarefa 1. |

## 2. REVISAR: corrigir junto, sem bloquear os testes seguros

- `tests/browser_live.rs:32`, `viewport_live.rs:22`: `taskkill` pelo PID sem conferir que ainda é o Chrome do teste (PID reciclado). Antes de matar, conferir `chrome.exe` e que a porta ainda responde.
- `tests/browser_live.rs:11`, `viewport_live.rs:11`: `CLAUDIA_TEST_CHROME` executa qualquer `.exe`. Exigir caminho absoluto, `file_name() == chrome.exe` e que o arquivo exista.
- `tests/browser_live.rs:50-52`: um erro depois do spawn deixa o Chrome órfão (CLA-39). Coberto pela Tarefa 1.
- `src-tauri/src/documentos/seguranca.rs:66-70`: o teste lê `C:\Windows\win.ini` de verdade. Não usar caminho não-bare no `join`.
- `src-tauri/src/commands/curriculos.rs:816-828`: com `CLAUDIA_PDF_PROVA_DIR=""`, grava no diretório atual. A Tarefa 3 remove esse teste.
- `src-tauri/src/mcp/tools/browser.rs:1584`: o teste usa a raiz do `%TEMP%` como pasta de dados. Usar `test_support::temp_dir`.
- `browser/manager.rs:403,413,390`: testes com `ensure_running` real e `.exe` inexistente. Trocar por `ensure_running_with` com spawn e kill inertes.
- Testes de `documentos/*` e `diagnostics/*` apagam `temp/claudia-<x>-<pid>` antes de começar, sem nanos. É seguro (prefixo literal), mas um helper com nanos é melhor.
- `ver.mjs:36`: sobrescreve qualquer caminho do argv. Restringir a `debug-logs/prints/`.
- `regras.mjs:34-37` (`rotuloPerigoso`): não bloqueia "Iniciar procura", "Desinstalar", "Limpar", "Instalar atualização", "Gerar" nem "Sair".
- `mcp-browser-smoke.mjs:16,27`: com `CLAUDIA_APP_LOCAL`, usa o Chrome real do app e o Activity log real. Exigir uma flag explícita além da variável. Na linha 57, um `JSON.parse` que lança deixa o Chrome órfão.
- `subir.mjs:47,60`: não detecta o app instalado rodando sem CDP.
- `node --test` sem lista de arquivos pode descobrir `comum.test.mjs`. **Sempre passar os arquivos explicitamente.**

## 3. Produção: riscos que nenhum teste alcança hoje

- `browser/install.rs:100,131,164`: a `version` do `browser-pin.json` entra num `remove_dir_all` sem validação. `version = ""` apaga `<dados-locais>\browser` **inteiro, com o perfil e os logins**. Validar o formato (4 números) antes de apagar.
- `migration.rs:30`: com `new_dir` relativo, o alvo vira uma pasta no diretório atual. Recusar caminho relativo. `DATA_FILES` também não cobre subpastas (currículos e cartas antigos), então numa máquina com a pasta legada elas seriam apagadas.
- `mcp/mod.rs:358-360`: sem `--data-dir`, o MCP grava o perfil e os backups no diretório atual. Exigir a flag.
- `app_data_dir().unwrap_or_default()` em `feedback.rs:449`, `perfil.rs:405,663` e `cover_letter.rs:300`: hoje só leem. Viram caminho relativo se um dia escreverem.

## 4. Seguro rodar nesta máquina (depois da seção 1)

- `cargo test --lib`, **exceto** `browser::service::tests` até B7.
- `node --test tests/front/documentos-logica.test.ts tests/front/folha-a4.test.ts tests/front/onde-aparece.test.ts tests/front/ouvir.test.ts`.
- `.claude/skills/claudia-teste-e2e/scripts/test/{regras,banco,cdp}.test.mjs` e `.claude/skills/claudia-teste-sandbox/scripts/test/{sandbox,novo,mock,copia,publicar}.test.mjs`, com arquivos explícitos. `comum.test.mjs` só depois de B1.
- `mcp-browser-smoke.mjs` com `CLAUDIA_TEST_CHROME` e **sem** `CLAUDIA_APP_LOCAL`; `activity-log-validate.mjs` (só lê); `banco.mjs`; `ver.mjs arvore`.
- Testes live (`browser_live`, `viewport_live`): só depois da Tarefa 1 e das correções de `taskkill` e `CLAUDIA_TEST_CHROME` da seção 2. Pela regra global, a primeira rodada vai para o Sandbox.

## 5. Nunca rodar

- Os scripts de `prototipo-playwright/` (sites reais, perfil real) e o `rm -rf` do README dele.
- `npm-package/bin/claudia-rh.js` (baixa e executa um `.exe`).
- Desinstalação ou limpeza de dados do app fora do Sandbox.

## 6. Travas que o plano precisa ganhar

1. Ampliar o grep da Tarefa 0: `cpSync`, `renameSync`, `Move-Item -Force`, `writeFileSync` que sobrescreve, `git worktree remove` e o laboratório externo.
2. Apagar cache de build: o estado do shell não persiste entre chamadas, e `rm -rf "$VAR"` com a variável vazia é um desastre. Usar **caminho literal** com guarda:
   ```powershell
   $p='C:\Users\joaog\.cache\claudia-close-1'; $i=Get-Item -LiteralPath $p -Force -ErrorAction Stop
   if($i.FullName -ne $p -or ($i.Attributes -band [IO.FileAttributes]::ReparsePoint)){throw "recusado"}
   Remove-Item -LiteralPath $p -Recurse -Force
   ```
3. Backup (Tarefa 5): `if ($LASTEXITCODE -ge 8) { throw }` depois de cada `robocopy`. Confirmar que `Documents` não sincroniza com o OneDrive.
4. Fechar o app: pela janela, ou `taskkill /PID <pid gravado pelo subir> /T`. **Nunca** `/IM claudia-rh.exe`, `/IM node.exe` (que mata o próprio Claude Code) nem `Stop-Process -Name`.
5. Antes de `subir.mjs`: `disparo.ativo = false` e `skip_permissions = false`, conferidos por comando de leitura. Se não estiverem assim, o usuário desliga na UI.
6. "Gerar" currículo ou carta chama `claude --dangerously-skip-permissions --print`: tem custo e roda um agente sem permissões. Avisar o usuário antes.
7. Excluir o documento importado: o `id` tem de ser exatamente o devolvido pelo `importar_documento`, e o nome tem de bater pelo `obter_documento` antes de apagar.
8. Rodada de agente e validações do motor: `--data-dir` e `--local-data-dir` temporários e explícitos, nunca `CLAUDIA_APP_LOCAL`.
9. `git rm -r prototipo-playwright`: antes, `git rm -r -n` para conferir os 24 arquivos.
10. Worktree: antes do `git worktree remove`, listar reparse points dentro dela. A pasta solta só sai com `Remove-Item -LiteralPath`, com o caminho resolvido dentro de `.claude\worktrees\`, sem reparse point, e depois do sim do usuário.
