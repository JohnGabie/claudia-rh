# Spec: Diagnóstico — logs, crashes, travamentos e pacote para o dev

> Spec de design. Não é o plano de implementação.

**Data:** 2026-09-17
**Branch:** `feat/diagnostico-logs`
**Repo:** `claudia-rh/`

## 1. Problema

Hoje o Rust usa `eprintln!` (some na build instalada) e um `claude-perfil-stderr.log` solto. Não há ficheiro de log da sessão de execução, nem panic hook, nem deteção de freeze. Quando algo corre mal de noite (retry da API, stream parado, sessão morta), no dia seguinte não há rasto.

O utilizador precisa de:

1. Um histórico local que sobreviva ao fechar a app.
2. Nas Configurações, **exportar um zip** para enviar ao desenvolvedor (WhatsApp/email). Nada sobe sozinho para a nuvem.
3. **No desenvolvimento**, a IA de coding (esta sessão) tem de conseguir ler os logs **directo do disco**, sem o utilizador gerar zip.

## 2. Decisões já tomadas

- Entrega ao dev em produção: zip local, nunca Sentry/upload automático.
- Conteúdo da sessão IA: eventos estruturados + **últimos ~100 KB do PTY**, redigidos. Não a transcrição inteira. Credenciais e YAML do perfil **nunca** entram.
- Acesso em desenvolvimento: ficheiros no workspace, gitignored.

## 3. Objetivo

1. Toda a sessão (app + PTY + MCP + panics + watchdog) deixa rasto em ficheiros rotativos.
2. Configurações: botão **“Exportar diagnóstico”** gera `ClaudiaRH-diagnostico-YYYYMMDD-HHMM.zip` no Ambiente de Trabalho (ou pasta escolhida via diálogo).
3. Com `npm run tauri dev`, os mesmos logs aparecem em `claudia-rh/debug-logs/` (caminho do repo). Qualquer sessão Grok/Claude lê `debug-logs/` sem pedir zip.
4. Redacção automática: emails, telefones, tokens. Nunca passwords, keyring, `candidate_base.yaml`, `search_variants.yaml`.

## 4. Fora de escopo

- Sentry, Aptabase, ou qualquer telemetria na nuvem.
- Session replay / screenshots da UI.
- Gravar o SQLite inteiro no zip.
- Alterar o `identifier` Tauri.
- Partir Perfil/Dashboard.

## 5. Pesquisa (o que usar e o que não)

Fontes: [Tauri Logging plugin v2](https://v2.tauri.app/plugin/logging/), [Aptabase guide](https://aptabase.com/blog/complete-guide-tauri-log/), prática de *support.zip* (Atlassian: redacção no pacote, não no vivo só), e o aviso de apps desktop de **não logar tokens** (Codeg 0.28.2).

| Escolha | Porquê |
|---|---|
| `tauri-plugin-log` + crate `log` | Oficial, Windows/macOS/Linux, `LogDir`, rotação `KeepSome`, JS `forwardConsole`. |
| **Não** Sentry neste ciclo | Sem servidor, PII (CV, vagas), consentimento. |
| Pacote tipo support.zip | Padrão de suporte desktop: o utilizador escolhe o que envia. |
| Redacção **ao escrever** e de novo **no zip** | Logs no disco também não devem ter password; o zip é a segunda barreira. |
| Espelho `debug-logs/` só em `debug_assertions` | A IA trabalha no repo; AppData é chato de achar e muda com o identifier. |

## 6. Onde vivem os ficheiros

### Produção (Windows)

Pasta de dados da app (`app_data_dir`), **não** misturar com o YAML do candidato:

```
%APPDATA%\io.github.johngabie.claudia-rh\diagnostics\
  claudia-rh.log          # plugin-log, rotativo
  claudia-rh.log.1
  events.jsonl            # eventos estruturados (uma linha JSON por evento)
  pty-tail.log            # anel dos últimos ~100 KB do PTY, já redigido, sem ANSI
  panic.log               # só se houver panic
```

Rotação do `claudia-rh.log`: `RotationStrategy::KeepSome` (guardar 5 ficheiros), `max_file_size` 2 MiB. Timezone local.

### Desenvolvimento (`tauri dev`)

Além da pasta de produção, **espelhar os mesmos quatro nomes** em:

```
claudia-rh/debug-logs/
```

Caminho relativo ao `CARGO_MANIFEST_DIR` / cwd do repo (`src-tauri/../debug-logs`). Adicionar `debug-logs/` ao `.gitignore` (já existe `logs` e `*.log`; o directório nomeado evita dúvidas).

Documentar em `CLAUDE.md`:

> Diagnóstico em desenvolvimento: `debug-logs/` na raiz do repo. Lê `events.jsonl` e `pty-tail.log` antes de perguntar o que aconteceu. Em produção no Windows: `%APPDATA%\io.github.johngabie.claudia-rh\diagnostics\`.

A IA **não precisa** do zip se estiver na mesma máquina.

## 7. O que se regista

### 7.1 Eventos estruturados (`events.jsonl`)

Uma linha JSON por evento, campos fixos:

```json
{"ts":"2026-09-17T02:14:03.512-03:00","lvl":"info","cat":"session","evt":"started","sid":42,"msg":"motivo=manual"}
```

Categorias (`cat`) e eventos (`evt`) mínimos:

| cat | evt | Quando |
|---|---|---|
| `app` | `boot`, `exit` | arranque / fecho |
| `session` | `started`, `ended`, `checkpoint`, `paused`, `resumed` | PTY de execução |
| `pty` | `api_stall`, `continue_sent`, `continue_exhausted`, `chrome_reconnect`, `chrome_failed` | watcher do PTY |
| `mcp` | `tool_ok`, `tool_err` | só **nome da tool** + ok/err — **sem args** |
| `ui` | `view` | mudança de aba (nome da view) |
| `watchdog` | `heartbeat_miss`, `recovered` | freeze suspeito |
| `panic` | `panic` | hook |

Nunca incluir o corpo do YAML, passwords, ou o prompt de sistema completo (é enorme e contém o perfil).

### 7.2 PTY tail (`pty-tail.log`)

No reader de `iniciar_claude` (já tem `line_buf`):

- Passar cada chunk por `strip_ansi` (já existe em `pty_manager.rs` na branch de API stall; se essa branch ainda não estiver em `dev`, copiar a função).
- Passar por `redact()`.
- Acrescentar a um anel em memória de **100 KiB**. Flush para disco no máximo a cada 2 s e ao terminar a sessão.

Não gravar o PTY genérico de teste (`iniciar` sem sessão).

### 7.3 Log geral (`claudia-rh.log`)

Substituir `eprintln!` de app por `log::info!` / `log::warn!` / `log::error!` nos módulos nossos (`claudia_rh_lib::*`). Dependências (hyper, tao) ficam em `Warn`.

Frontend: `forwardConsole` de `error` e `warn` para o plugin (não `console.log` de dados de perfil).

### 7.4 Panics

`std::panic::set_hook` **depois** de inicializar o logger:

- `log::error!("panic: {info}")`
- append atómico a `panic.log` (timestamp + payload)
- no hook, não fazer I/O complexo além disso

Não tentar “recuperar” o processo.

### 7.5 Watchdog de freeze

Thread ou `tokio` task:

- A UI (ou o reader do PTY enquanto a sessão está activa) actualiza um `AtomicU64` monotonic clock a cada evento `pty-output` e a cada 5 s no frontend (`heartbeat` invoke barato).
- Se a sessão estiver activa (`sessoes.terminada_em IS NULL`) e o heartbeat tiver mais de **60 s**, emitir `watchdog.heartbeat_miss` (uma vez até recuperar).
- Quando o heartbeat volta, `watchdog.recovered` com a duração.

Isto captura “a janela congelou / o PTY parou de debitar” — o caso da noite em que não houve zip.

## 8. Redacção

Função pura `redact(s: &str) -> String`, testada:

| Padrão | Substituição |
|---|---|
| emails | `[email]` |
| `+` / dígitos tipo telefone (≥ 9 dígitos seguidos) | `[phone]` |
| `sk-ant-` / `sk-` + 20+ alphanum | `[token]` |
| `Bearer ` + token | `Bearer [token]` |
| passwords óbvias (`password=`, `senha=`) | chave + `[redacted]` |

Não redigir nomes de empresas/vagas (precisamos deles para debug). Não incluir ficheiros `candidate_base.yaml` / `search_variants.yaml` / `notif.json` / keyring no zip.

Testes: string com email+telefone+token → os três mascarados; string sem PII inalterada.

## 9. Zip nas Configurações

Secção nova **Diagnóstico** (no fim da tela, não no topo):

1. Texto curto: “Gera um ficheiro com logs recentes (sem passwords nem o teu currículo) para enviares ao desenvolvedor.”
2. Botão **Exportar diagnóstico**.
3. Diálogo nativo “guardar como” (`tauri-plugin-dialog`, já no projecto), default `ClaudiaRH-diagnostico-YYYYMMDD-HHMM.zip` no Ambiente de Trabalho.
4. Toast/texto de sucesso com o caminho.

Conteúdo do zip:

```
manifest.txt          # versão app, OS, identifier, hora, sid da sessão activa se houver
claudia-rh.log*       # ficheiros rotativos existentes
events.jsonl
pty-tail.log          # se existir
panic.log             # se existir
```

Antes de meter no zip, correr `redact` outra vez linha a linha (segunda barreira).

Comando Tauri: `exportar_diagnostico(dest: PathBuf) -> Result<String, String>` (devolve o caminho).

Não anexa sozinho a email nem abre GitHub.

## 10. Mapa de ficheiros (implementação futura)

| Ficheiro | Papel |
|---|---|
| `src-tauri/src/diagnostics/mod.rs` | init paths, redact, ring PTY, zip |
| `src-tauri/src/diagnostics/redact.rs` | `redact` + testes |
| `src-tauri/src/diagnostics/watchdog.rs` | heartbeat |
| `src-tauri/src/lib.rs` | plugin-log, panic hook, `exportar_diagnostico` |
| `src-tauri/src/pty_manager.rs` | write no ring; eventos `api_stall` / `continue_sent` |
| `src-tauri/src/mcp/mod.rs` | `tool_ok`/`tool_err` sem args (hoje `mcp-debug.log` dumpa args — **não** copiar isso para o zip) |
| `src/components/Configuracoes.tsx` | secção Diagnóstico |
| `src/i18n/en.ts`, `pt.ts` | copy do botão |
| `CLAUDE.md` | caminhos `debug-logs/` e AppData |
| `.gitignore` | `debug-logs/` |

## 11. Verificação

```text
# testes da redacção
cargo test -p claudia-rh --lib redact

# debug-logs aparece com tauri dev
ls debug-logs/events.jsonl

# zip não contém yaml nem a palavra password em claro
# (inspecção do zip gerado)
```

Critério humano: reproduzir um `continue` por stall → linha em `events.jsonl` com `evt=continue_sent` e o PTY tail contém `stalled mid-stream` (ou a mensagem redigida). Matar a app no Gestor de Tarefas → `panic.log` ou pelo menos `app exit` no próximo boot (`boot` com `previous_unclean=true` se o último `exit` não existir).

## 12. Como o Superpowers continua

Spec aprovada → `writing-plans` → implementação em `feat/diagnostico-logs` → review → merge em `dev`.
