# Spec: Arranque real da sessão Claude (não só o banner)

> Spec de design. Não é o plano de implementação.

**Data:** 2026-09-17
**Branch:** `feat/fix-sessao-spawn`
**Repo:** `claudia-rh/`

## 1. Problema

O utilizador clica em procurar vagas e vê:

```
[Claudia RH] Starting Claude session (reason: manual · mode: autonomous)…
```

O Dashboard fica verde a pulsar. **O Claude nunca aparece.** O texto fica infinito.

Isto não é o Claude a arrancar. É a app a escrever no xterm *antes* de existir um processo. Se o spawn falhar ou o `claude.exe` ficar mudo, a UI já prometeu uma sessão viva.

Medido nesta máquina (2026-09-17):

| Dado | Valor |
|---|---|
| Prompt montado (`runtime.md` + YAMLs) | **30 147 caracteres** |
| Linha de comando aproximada | **~30 425** |
| Limite `CreateProcessW` | **32 767** (incluindo NUL) |
| `claude.exe` npm (o que o código procura primeiro) | **não existe** |
| `claude.exe` real | WinGet `...\Anthropic.ClaudeCode_...\claude.exe` **v2.1.220** |
| `mcp-config.json` | aponta para `target\debug\claudia-rh.exe` + `--notify-port` antigo |

## 2. Estudo (fontes)

### 2.1 Windows não cabe o prompt na argv

[CreateProcessW](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw): *“The maximum length of this string is 32,767 characters, including the Unicode terminating null character.”* Raymond Chen (The Old New Thing): o limite vem de `UNICODE_STRING`; o `cmd.exe` ainda é mais curto (8192). LLVM usa **32 000** de margem de segurança.

O prompt da Claudia é **quase o tecto**. YAML tem aspas e newlines; o `portable-pty` / ConPTY ainda tem de *quotar* cada argumento. Qualquer crescimento do perfil rebenta o spawn, ou o `claude` recebe argv truncada e fica mudo.

**Conclusão:** nunca passar o prompt como `--system-prompt <texto>`. Passar um **caminho de ficheiro**.

### 2.2 Flags oficiais do Claude Code (v2.1+)

Fonte canónica: [CLI reference](https://code.claude.com/docs/en/cli-reference) (2026-09). Confirmado no `claude --help` local (v2.1.220): aparece `--system-prompt[-file]`.

| Flag | Comportamento | Modos (doc EN actual) |
|---|---|---|
| `--system-prompt` | Substitui o prompt default **pelo texto** | interactivo + print |
| `--system-prompt-file` | Substitui pelo **conteúdo de um ficheiro** | interactivo + print |
| `--append-system-prompt-file` | Acrescenta ficheiro ao default | interactivo + print |

`--system-prompt` e `--system-prompt-file` são **mutuamente exclusivos**.

A doc antiga da Anthropic dizia que `--system-prompt-file` era só print-mode. **A doc actual diz que os cinco flags funcionam nos dois modos.** A Claudia precisa de **substituir** o prompt (regras de honestidade, pausa, Chrome) — não de append. Logo: `--system-prompt-file`.

Não usar `CLAUDE.md` no workspace: isso *acrescenta* convenções de código; não substitui a identidade do agente de candidaturas.

### 2.3 MCP no boot

Claude Code faz **um handshake MCP no arranque**. Se o servidor stdio não responder, a sessão pode marcar o MCP como morto ou ficar à espera.

Nesta app, `mcp-config.json` é reescrito em cada spawn com `current_exe()` + `--notify-port`. Problemas observados:

- Ficheiro no disco apontava para `target\debug\claudia-rh.exe` mesmo quando isso já não é o processo vivo.
- `--notify-port` fica obsoleto se a app reiniciar.
- Sem `--strict-mcp-config`, o Claude ainda mistura MCP do `~/.claude.json` do utilizador.

`--strict-mcp-config` ([doc](https://code.claude.com/docs/en/cli-reference)): *Only use MCP servers from `--mcp-config`.*

`--debug-file <path>`: log de arranque do próprio Claude — útil no zip de diagnóstico.

Issue [#57413](https://github.com/anthropics/claude-code/issues/57413): no Windows, `claude.exe` zombi a segurar `~/.claude.json.lock` → **loading infinito**. A app deve reportar spawn falhado e, se possível, não deixar filhos órfãos.

### 2.4 `--chrome`

`--chrome` é o flag correcto para a extensão. No Windows há bugs de named pipe ([#47167](https://github.com/anthropics/claude-code/issues/47167), [#50465](https://github.com/anthropics/claude-code/issues/50465)). Já existe watcher de “Browser extension is not connected”. **Não remover `--chrome`.** Só não deixar a UI mentir se o Claude ainda não imprimiu nada.

### 2.5 Prompt inicial

Hoje: espera 5 s e escreve `Inicia a sessao de candidaturas.` + Enter no PTY (porque detectar o prompt via ANSI é frágil).

A CLI aceita o query como argumento posicional: `claude "explain this project"` inicia a sessão **já com essa mensagem**. Evita o timer cego.

## 3. Objetivo

1. O banner “Starting…” só significa “vou tentar”. Se o spawn falhar, o **erro aparece no terminal** e a sessão **não** fica verde.
2. O prompt vai num ficheiro; a argv fica curta (`--system-prompt-file <path>`).
3. `claude.exe` resolve-se também no WinGet / `where`, não só no npm.
4. MCP: config gerada no *mesmo* spawn, `--strict-mcp-config`, `current_exe()` fresco.
5. Primeira tarefa: argumento posicional, não type-after-5s.
6. Eventos de diagnóstico: `session.spawn_ok` / `session.spawn_err` com a mensagem (redigida).

## 4. Fora de escopo

- Trocar `--chrome` por Playwright MCP.
- Reescrever o texto do prompt de runtime.
- PTY size 24×80 → size do xterm (follow-up; não é a causa do hang mudo).
- Matar zombis `claude.exe` do utilizador automaticamente (perigoso).

## 5. Desenho

### 5.1 Ficheiro do prompt

Em `iniciar_sessao`, depois de `montar_prompt_sistema`:

```
data_dir/workspace/.claude-system-prompt.txt
```

(workspace já é git repo para evitar o diálogo de trust.) Escrever o prompt completo (UTF-8). Não commitar (está em `data_dir`, fora do repo da app).

Args do `claude`:

```
[--dangerously-skip-permissions]   # se modo autónomo
--chrome
--mcp-config <path>
--strict-mcp-config
--system-prompt-file <workspace/.claude-system-prompt.txt>
--debug-file <diagnostics/claude-startup.log>   # se diagnostics::PATHS existir
Inicia a sessao de candidaturas.
```

**Não** passar `--system-prompt`. **Não** o texto do prompt na argv.

Se o Claude desta máquina rejeitar `--system-prompt-file` em modo interactivo (versão antiga), o spawn falha com stderr visível — aí documentar fallback. Nesta máquina (2.1.220) a doc oficial cobre interactivo.

### 5.2 Resolução de `claude.exe`

Estender `claude_program()` (Windows), por esta ordem:

1. `%APPDATA%\npm\node_modules\@anthropic-ai\claude-code\bin\claude.exe` (já existe)
2. Qualquer `claude.exe` nos dirs do `PATH` (já existe)
3. **Novo:** `where.exe claude` / `Get-Command` equivalente em Rust: procurar também
   `%LOCALAPPDATA%\Microsoft\WinGet\Links\claude.exe`
   e `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Anthropic.ClaudeCode_*\claude.exe` (glob)
4. Último: `"claude"` (PATH do processo)

Se nenhum ficheiro existir, **return Err** *antes* de `session-started`, com texto:

```
[Claudia RH] claude.exe não encontrado. Instala Claude Code (WinGet ou npm) e reabre a app.
```

### 5.3 Ordem do ciclo de vida (obrigatória)

```
1. INSERT sessoes (ainda sem “UI activa”)
2. montar prompt → escrever ficheiro
3. write_mcp_config (exe + notify-port deste processo)
4. spawn PTY
5. SE spawn Ok:
     emit pty-output "Starting… (pid=… path=…)"
     emit session-started
     emit_event session spawn_ok
     iniciar reader + NÃO timer de 5s
   SENÃO:
     emit pty-output "[Claudia RH] Falha ao iniciar: {erro}"
     UPDATE sessoes terminada_em, motivo_termino='spawn_err'
     emit session-ended "spawn_err"
     emit_event session spawn_err (msg=redact(erro))
     NÃO emit session-started
```

Hoje o passo 5 está invertido (banner + `session-started` *antes* do spawn). É isso que deixa o verde infinito.

### 5.4 MCP

`write_mcp_config` já usa `current_exe()`. Garantir que corre **no mesmo `iniciar_sessao`**, imediatamente antes do spawn (já é o caso). Acrescentar `--strict-mcp-config` aos args do Claude.

Não incluir `--debug` no MCP config em produção (hoje o JSON no disco tinha `--debug` porque a app era `tauri dev`). O flag `--debug` no filho MCP só se `cfg!(debug_assertions)`.

### 5.5 Prompt inicial

Remover o thread de 5 s + `write continue`. Passar a string `Inicia a sessao de candidaturas.` como **último argumento posicional** do `CommandBuilder` (é o `query` da CLI).

### 5.6 PTY size

Manter 24×80 neste ciclo (follow-up). O hang mudo não é o tamanho.

### 5.7 Diagnóstico

- `emit_event("session", "spawn_ok"|"spawn_err", Some(id), msg)`
- `--debug-file` para `diagnostics/claude-startup.log` (entra no zip já allowlisted se o nome for `claude-startup.log` — **estender o allowlist do zip** neste ciclo, uma linha).

## 6. Ficheiros

| Ficheiro | Mudança |
|---|---|
| `src-tauri/src/commands/mod.rs` | `claude_program()` + WinGet |
| `src-tauri/src/commands/sessao.rs` | ordem de vida; prompt file; args; erros no PTY |
| `src-tauri/src/pty_manager.rs` | `iniciar_claude` recebe query posicional; remove timer 5s |
| `src-tauri/src/commands/perfil.rs` | `write_mcp_config`: `--debug` só em debug_assertions |
| `src-tauri/src/diagnostics/mod.rs` | allow `claude-startup.log` no zip |
| testes em `commands/mod.rs` ou `sessao` | `claude_program` encontra um path existente quando o exe está no PATH; argv de spawn **não** contém o corpo do YAML |

## 7. Verificação

```text
# argv curta
# teste unitário: args after build must not contain "nome_completo" / email do perfil

# spawn falha visível
# (renomear claude temporariamente em teste de integração manual)
# terminal mostra "Falha ao iniciar"; Dashboard NÃO pulsa verde

# spawn ok
# terminal mostra banner E a seguir output do Claude (não só o banner)
```

Critério humano: clicar “procurar vagas agora” → em ≤10 s ou há TUI/texto do Claude no Terminal, ou uma linha de erro explícita. Nunca banner sozinho + verde infinito.

## 8. Superpowers

Spec aprovada → `writing-plans` → implementação em `feat/fix-sessao-spawn` → review → merge em `dev`.
