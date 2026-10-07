# Laboratório Windows Sandbox (referência)

Origem: skill `sandbox-lab` do laboratório, trazida para o repositório em 2026-10-07 (spec `docs/superpowers/specs/2026-10-07-laboratorio-no-repo-design.md`). **Onde este texto e o `CLAUDE.md` da raiz ou a `SKILL.md` da `claudia-teste-sandbox` divergem, valem eles.**

O laboratório fica em `sandbox-lab/` do checkout principal (`node .claude/skills/claudia-teste-sandbox/scripts/onde.mjs` → `lab`). Contrato e critérios de aprovação do laboratório genérico: `sandbox-lab/OBJECTIVES.md`.

## Regra de seguranca n. 1

**Nunca aja no desktop do host.** Todo comando de UI vai para o guest:
- winapp: SEMPRE com `--on sandbox` (ou `winapp target ... sandbox`). Se der erro, nao remova o `--on sandbox`.
- MCP: use apenas o servidor `windows-sandbox` deste projeto (ele roda DENTRO do guest).
  Se houver outro Windows-MCP local/stdio configurado, nao use para este lab.
- Nao rode `windows-mcp serve` no host.

## Ciclo de vida

```powershell
scripts\validate-config.ps1     # O1: valida configs sem subir nada
scripts\test-up.ps1             # sobe o Sandbox (so existe UM por vez no Windows)
scripts\test-down.ps1           # derruba e descarta tudo do guest
```

- Antes de qualquer `winapp ... --on sandbox`, confira que o lab ja esta de pe
  (`wsb list` ou `shared\out\status.json` com `bootstrap: "ok"`). Se nao houver Sandbox, o winapp
  CRIA um novo, sem os mapeamentos do lab - rode `test-up.ps1` primeiro.
- Nao feche o Sandbox nem rode `wsb stop` por conta propria; use `test-down.ps1` ao encerrar o teste.

## Pastas no guest

| Guest | Host | Acesso |
|---|---|---|
| `C:\lab` | `shared\` | read-only |
| `C:\lab\out` | `shared\out\` | **gravavel** (logs, status.json, mcp-status.json) |
| `C:\labproject` | projeto do `compose.json` / `test-up.ps1 -Project` | read-only |
| `C:\labsandbox` | `sandbox\` | read-only (bootstrap) |
| `C:\labdata` | (local do guest) | gravavel, some no test-down |

Banco de trabalho: `C:\labdata\app.sqlite` (copia de `C:\lab\db\backup.sqlite`).
**Nunca** abra `shared\db\backup.sqlite` para escrita - o hash dele e verificado no fim (O4/O10).

## MCP (Windows-MCP no guest)

- Servidor: Windows-MCP (CursorTouch) `streamable-http` em `0.0.0.0:8000`, path `/mcp`,
  auth `Authorization: Bearer <shared\secrets\mcp-auth.key>`. Sobe sozinho no boot
  (`sandbox\bootstrap-mcp.ps1`, config em `shared\mcp.json`).
- Status do guest: `shared\out\mcp-status.json` (`ok`, `ip`, `url`, `durationSec`, `errors`)
  e log em `shared\out\bootstrap-mcp.log` / `mcp-server.err.log`.
- O IP do guest muda a cada boot. Depois do `test-up`:

```powershell
scripts\mcp-sync.ps1 -Wait 300 -Test        # reescreve .mcp.json com o IP atual e lista as tools
$env:LAB_MCP_AUTH_KEY = (Get-Content shared\secrets\mcp-auth.key -Raw).Trim()
claude mcp list                             # windows-sandbox; na 1a vez aparece "Pending approval" -> aprove ao abrir o `claude` no lab
```

  (`.mcp.json` usa `${LAB_MCP_AUTH_KEY}`; reinicie o Claude Code apos o sync para pegar o novo IP.)
- O `.mcp.json` do laboratório (`sandbox-lab/.mcp.json`) é gerado pelo `mcp-sync.ps1`, fica fora do git e nunca contém a chave (a opção `-Literal` foi removida).
- Tools uteis: `Snapshot`/`Screenshot` (ver a tela do guest), `Click`, `Type`, `Shortcut`, `App`, `PowerShell` (roda no guest).

## winapp no guest

Skills detalhadas: `.claude/skills/winapp-sandbox` e `.claude/skills/winapp-ui-automation`. Resumo:

```powershell
winapp ui list-windows --on sandbox
winapp ui inspect --on sandbox -a chrome
winapp ui search --on sandbox "Pesquisar" -a chrome
winapp ui invoke --on sandbox <Elemento> -a chrome
winapp ui screenshot --on sandbox -a chrome -o evidence\<objetivo>-<passo>.png
winapp target screenshot sandbox -o evidence\desktop.png
```

Chrome do guest: Chrome for Testing com a extensao `lab-ext`, CDP na porta 9222 (dentro do guest),
pagina de teste servida em `http://localhost:8080/test-page.html` no guest.

## Ambientes (`-Ambiente`, `compose.json`)

`scripts\test-up.ps1 -Ambiente <nome> -Project <projeto>`; sem `-Projeto` usa `sample-project/`. Os ambientes são definidos em `compose.json`; cada um explica o uso no `$comment`.

| Ambiente | Para que serve | Rede | Instalações do guest |
|---|---|---|---|
| `exemplo` | lab de exemplo (O1-O10) | ligada | nenhuma |
| `app` | rodar um app desktop sem rede; controle via `wsb exec` | desligada | Node 22.23.3 |
| `app-winapp` | rodar um app desktop controlado pelo winapp; rede ligada para winapp e CDP, internet bloqueada por `guest-firewall.ps1` (System) | ligada | Node 22.23.3 |
| `cache-msvc` | montar o layout offline do MSVC: `wsb exec -r System` + `layout-msvc.ps1`; depois `cache-up.ps1 -ImportarMsvc <id>` | ligada | nenhuma |
| `testes` | testes no Sandbox, `cargo test --lib` e Claude Code com a conta do PC; MSVC 17.14, Rust 1.95.0, Node 22.23.3 e Claude Code 2.1.285 instalados no guest a partir do cache; resultado em `C:\lab\out\instalar.json` | ligada | MSVC, Rust, Node, Claude Code |

Projeto e pasta de trabalho:
- Projeto: `test-up.ps1 -Project <pasta>` (a skill passa o checkout atual; montado read-only em `C:\labproject`). Pasta de trabalho: `.sandbox-work/` do checkout principal (`deploy\` vira `C:\appdeploy` read-only, `guest-rw\` vira `C:\labwork` gravável). Sem variável de ambiente.
- Publicar o app: `node .claude/skills/claudia-teste-sandbox/scripts/publicar.mjs --app <exe|instalador|pasta>` (chama `scripts\app-stage.ps1 -Clean` com a trava do marcador `.lab-deploy`).

Dados no guest:
- Cenários: `node .claude/skills/claudia-teste-sandbox/scripts/cenario-no-guest.mjs --cenario novo|mock|copia [--autorizado] [--manter-automacao]`; resultado em `Z:\claudia-teste-sandbox\<cenario>`.
- Instalar dados no guest: `C:\labsandbox\app\guest-run.ps1 -Cenario Z:\claudia-teste-sandbox\<cenario> [-NoStart]` - copia os dados para `%APPDATA%`/`%LOCALAPPDATA%` do perfil do guest e opcionalmente inicia o app.
  Rode com `wsb exec --id <id> -r ExistingLogin -c "powershell -ExecutionPolicy Bypass -File ..."` (id em `wsb list`).
- Saídas em `C:\labwork\...` (= `.sandbox-work\guest-rw\...` no host).

Pastas no guest (além de `C:\lab`, `C:\labsandbox`, `C:\appdeploy`, `C:\labwork`):
- `C:\labcred` - login do Claude copiado do PC (read-only, só no ambiente `testes`).
- `C:\labdadosreais` - pasta Roaming do app copiada (read-only, só com `-DadosReais`, só no ambiente que pede conta).
- `C:\labtools` - ferramentas instaladas (`msvc`, `node`, `claude`).
- `C:\labdata` - workspace do guest (disco C).
- `Z:\claudia-teste-sandbox` - cenários de dados (disco Z, só para testes).
- `Z:\tmp` - temporário (disco Z).

Regras:
- Não acione no app nenhuma ação com efeito externo real (envios, publicações, pagamentos) e não faça login manual no guest.
- Falhas do próprio app no Windows limpo (dependência ausente, erro de inicialização) são resultado do teste: registre, não contorne.

## Evidencias

- Host: `sandbox-lab/evidence/` (ignorada pelo git; screenshots, relatorios) - nome `O<n>-<descricao>.png|md`.
- Guest: sobrevivem `C:\lab\out` (aparece em `shared\out\`, que o `test-up.ps1` limpa a cada subida) e `C:\labwork` (= `.sandbox-work\guest-rw\` no host, que o `test-up.ps1` **não** limpa). O resto do guest some no `test-down`; copie o que importar antes.
- Valide resultados lendo os arquivos/screenshots, nao apenas em relatorios anteriores (O9).
