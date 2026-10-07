---
name: sandbox-lab
description: Como usar o win-sandbox-lab - subir/derrubar o Windows Sandbox descartavel (test-up/test-down), controlar apps e o Chrome do guest com winapp --on sandbox ou com o MCP windows-sandbox (Windows-MCP via HTTP), onde gravar evidencias, onde fica o banco de trabalho e quais pastas sao read-only. Use sempre que for testar, automatizar UI, tirar screenshot ou validar objetivos (O1-O10) deste lab.
---

# sandbox-lab

Lab de teste descartavel: build no host, execucao num Windows Sandbox limpo, controlado por IA.
Contrato e criterios de aprovacao: `OBJECTIVES.md` (raiz do lab). Leia antes de agir.

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
| `C:\labproject` | projeto do `lab.config.json` | read-only |
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
- Tools uteis: `Snapshot`/`Screenshot` (ver a tela do guest), `Click`, `Type`, `Shortcut`, `App`, `PowerShell` (roda no guest).

## winapp no guest

Skills detalhadas: `winapp-sandbox` e `winapp-ui-automation` (nesta pasta). Resumo:

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

## Perfis (`-Config`)

`scripts\test-up.ps1 -Config <perfil>`; sem `-Config` usa `lab.config.json` (projeto de exemplo).
Caminhos nos perfis aceitam variaveis de ambiente (`%USERPROFILE%`). Cada perfil explica o uso no `$comment`.

| Perfil | Para que serve | Rede |
|---|---|---|
| `lab.config.json` | lab de exemplo (O1-O10) | ligada |
| `lab.config.app.json` | rodar um app desktop; controle via `wsb exec` + `sandbox\app\guest-observe.ps1` / `guest-click.ps1` (o winapp nao alcanca o guest sem rede) | desligada |
| `lab.config.app-winapp.json` | rodar um app desktop controlado pelo winapp; rode antes `sandbox\app\guest-firewall.ps1` como System para bloquear a internet | ligada, internet bloqueada |
| `lab.config.app-build.json` | build do projeto dentro do guest (`sandbox\app\guest-build.ps1`) com toolchains do host montados read-only | ligada |

Perfis `lab.config.app*.json`:
- Variaveis de ambiente no host: `LAB_APP_PROJECT` (pasta do projeto, montada read-only em `C:\labproject`)
  e `LAB_WORK_DIR` (pasta de trabalho; `deploy\` vira `C:\appdeploy` read-only e `guest-rw\` vira `C:\labwork` gravavel).
  Use copias: nunca aponte para o projeto ou os dados originais se o teste puder grava-los.
- Publicar o app: `scripts\app-stage.ps1 -App <exe ou pasta> [-AppData <pasta>...] [-LocalAppData <pasta>...]`.
  As pastas de dados devem ser copias preparadas para teste (automacoes com efeito externo desligadas).
- No guest (`winapp target exec sandbox -- powershell -ExecutionPolicy Bypass -File <script>`):
  - `C:\labsandbox\app\guest-run.ps1 [-Exe x.exe]` - copia os dados para `%APPDATA%`/`%LOCALAPPDATA%` e inicia o app.
    Rode com `wsb exec --id <id> -r ExistingLogin -c "powershell -ExecutionPolicy Bypass -File ..."` (id em `wsb list`):
    iniciado por `winapp target exec`, o app e encerrado quando o comando termina.
  - `C:\labsandbox\app\guest-chrome-probe.ps1 [-Port N] [-Navigate <url>]` - acha o Chrome com DevTools ligado e lista as abas.
  - `C:\labsandbox\app\guest-observe.ps1 -Process <nome>` / `guest-click.ps1 -X -Y` - screenshot, arvore UIA e clique sem winapp.
  - Saidas em `C:\labwork\...` (= `%LAB_WORK_DIR%\guest-rw\...` no host).
- Nao acione no app nenhuma acao com efeito externo real (envios, publicacoes, pagamentos) e nao faca login em contas reais.
- Falhas do proprio app no Windows limpo (dependencia ausente, erro de inicializacao) sao resultado do teste: registre, nao contorne.

## Evidencias

- Host: `evidence\` (screenshots, relatorios) - nome `O<n>-<descricao>.png|md`.
- Guest: so `C:\lab\out` sobrevive (aparece em `shared\out\`). Copie o que importar antes do `test-down`.
- Valide resultados lendo os arquivos/screenshots, nao apenas em relatorios anteriores (O9).
