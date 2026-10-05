# Pastas de dados do app

Identifier: `io.github.johngabie.claudia-rh` (nunca muda; define a pasta de dados e a identidade do instalador).

Cada nome abaixo foi conferido no código com `grep` em `src-tauri/src`. O que não aparece no código está marcado **não verificado**. Conferir de novo quando a versão em teste for outra:

```
grep -rn '<nome>' src-tauri/src
```

## `%APPDATA%\io.github.johngabie.claudia-rh` (`app_data_dir()`)

É a pasta que os cenários produzem e que `cenario-copia.mjs` copia.

| Item | O que é | Onde no código |
|---|---|---|
| `candidate_base.yaml` | perfil do candidato | `commands/perfil.rs`, `commands/welcome.rs`, `mcp/tools/profile.rs` |
| `candidate_base.yaml.bak-1`, `candidate_base.yaml.tmp` | cópia anterior e escrita atômica do perfil | `mcp/tools/profile.rs` |
| `search_variants.yaml` | variantes de busca | `commands/perfil.rs`, `mcp/server.rs` |
| `strategy.md` | estratégia do candidato | `commands/perfil.rs`, `mcp/tools/profile.rs` |
| `claudia_rh.db` (+ `-wal`/`-shm` com o app aberto) | banco SQLite (vagas, candidaturas, sessões) | `lib.rs`, `commands/perfil.rs`, `mcp/tools/mod.rs` |
| `claudia_rh.db.pre-migration` | backup antes de migração de dados antigos | `migration.rs` |
| `disparo.json` | disparo por ociosidade (`ativo`, `limiar_minutos`, `limite_diario`, `limite_tempo_minutos`, `janelas`). Arquivo ausente ou incompleto: o app assume `ativo: true` | `lib.rs` (`load_idle_config`), `commands/sessao.rs` |
| `modo_autonomo.json` | `skip_permissions` da sessão de execução | `commands/sessao.rs` |
| `notif.json` | preferências de notificação | `commands/notif.rs` |
| `welcome_seen` | existe depois que o welcome foi concluído; sem ele o welcome aparece | `commands/welcome.rs` |
| `diagnostics\` | `events.jsonl`, `pty-tail.log`, `panic.log`, `claude-startup.log` | `diagnostics/mod.rs` |
| `prompts\` | prompts editáveis pelo usuário (com `.bak` ao restaurar) | `commands/prompts.rs` |
| `workspace\` | pasta de trabalho das sessões | `commands/sessao.rs`, `commands/linkedin.rs` |
| `curriculos\`, `cover_letters\` | documentos gerados | `commands/curriculos.rs`, `commands/cover_letter.rs` |

## `%LOCALAPPDATA%\io.github.johngabie.claudia-rh`

Nunca copiado pelo cenário `copia`: pode ter perfil de navegador, cookies e sessões.

| Item | Situação |
|---|---|
| `logs\` (plugin de log, rotação de 5 arquivos) | o código chama `app_log_dir()` (`lib.rs`); no Windows o Tauri usa `%LOCALAPPDATA%\<identifier>\logs`. Caminho exato **não verificado** num guest |
| `EBWebView\` (dados do WebView2) | padrão do Tauri, não referenciado no código; **não verificado** |
| browser próprio baixado na primeira abertura | **não verificado**: nada no código desta versão baixa navegador |
| Activity log | **não verificado**: nenhum arquivo com esse nome no código desta versão |

## Fora das pastas do app

- `debug-logs/` na raiz do repositório: só em build de debug (`tauri dev`, `cfg!(debug_assertions)` em `diagnostics/mod.rs`). Não existe no guest com build de release.
- O welcome (`verificar_setup` em `commands/welcome.rs`) consulta o Claude Code instalado e a chave de registro do native host do Chrome (`Software\Google\Chrome\NativeMessagingHosts\com.anthropic.claude_code_browser_extension`); não grava nessas pastas.
- App instalado pelo instalador NSIS: o padrão do Tauri para instalação por usuário é `%LOCALAPPDATA%\<productName>` (`productName` = `claudia-rh`); **não verificado** no guest, confira antes de usar em `--exe-path`.
