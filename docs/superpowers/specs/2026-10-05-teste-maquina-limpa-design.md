# Teste em máquina limpa (Windows Sandbox) e observação do app — design

## Pedido do usuário

- Usar um laboratório Windows Sandbox, mantido **fora do repositório**, para testar o Claudia RH num Windows limpo e isolado.
- Duas skills especializadas:
  - uma **nova** que mostre ao agente como o laboratório funciona e o que fazer, deixando o agente elaborar os testes conforme a situação (inclusive escrever testes automáticos, se fizer sentido);
  - a **`claudia-teste` renomeada** e ampliada para observar o app por CDP **localmente e no Sandbox, quando disponível**.
- **Cenários de dados**, cada um com script: cópia do banco do usuário, tudo novo (sem dados) e dados mockados. O agente decide na hora qual faz sentido.
- Só o cenário com os dados do usuário exige que o agente peça autorização. No resto, sem travas extras.
- **Prints** da tela, importantes para o frontend e para depurar quando outras ferramentas falham.
- Ajustar o `CLAUDE.md`.
- Arquivos agnósticos: legíveis e executáveis por qualquer agente sem contexto externo.

## Leitura do pedido

- **Laboratório externo:** o repositório não contém o laboratório; só o encontra por variável de ambiente e usa a interface pública dele.
- **Duas skills, papéis separados:**
  - `claudia-sandbox` (nova): o **ambiente** — laboratório, cenários de dados, publicar/instalar o app no Sandbox, ligar o CDP do app dentro do guest.
  - `claudia-app` (a `claudia-teste` renomeada): **observar e exercitar o app rodando** — tela, cliques, comandos, banco, agentes, prints — com alvo local (`tauri dev` no host) ou Sandbox.
  - Sem skill guarda-chuva: a escolha entre as duas é feita pela `description` de cada uma e por uma tabela curta no `CLAUDE.md`.
- **Skill como mapa:** descreve capacidades, limites e regras. Não impõe roteiro de teste.
- **Três cenários:** produzem uma pasta de dados pronta para publicar no Sandbox. A escolha é do agente.
- **Autorização:** obrigatória apenas para o cenário que lê os dados reais do usuário.
- **Prints:** sempre possíveis, com caminhos alternativos quando o CDP não está disponível.

## Contexto: interface do laboratório

O laboratório é uma pasta independente (variável `SANDBOX_LAB_DIR`) com:

| Item | Uso |
|---|---|
| `scripts\test-up.ps1 -Config <perfil>` / `scripts\test-down.ps1` | sobe / derruba o Sandbox (um por vez); o id da instância aparece na saída do `test-up` e em `wsb list` |
| `scripts\app-stage.ps1 -App <exe\|pasta> -AppData <pastas> -LocalAppData <pastas> [-WorkDir] [-Clean]` | publica app e dados em `<WorkDir>\deploy` (no guest: `C:\appdeploy`); `<WorkDir>\guest-rw` é gravável no guest como `C:\labwork` |
| `lab.config.app.json` | app sem rede |
| `lab.config.app-winapp.json` | app com rede para o winapp; internet bloqueada por `sandbox\app\guest-firewall.ps1` |
| `lab.config.app-build.json` | build dentro do guest com toolchains do host em read-only |
| `sandbox\` | montada read-only no guest em `C:\labsandbox` (scripts do guest: `C:\labsandbox\app\...`) |
| `sandbox\app\guest-run.ps1 [-Exe <nome.exe>] [-NoStart]` | no guest (`-NoStart`: só instala os dados): copia `appdata`/`localappdata` para `%APPDATA%`/`%LOCALAPPDATA%` e inicia o exe |
| `sandbox\app\guest-chrome-probe.ps1`, `guest-observe.ps1`, `guest-click.ps1` | observar Chrome com DevTools, tela/árvore UIA, clique sem winapp |
| `shared\` | montada read-only no guest em `C:\lab`; `shared\tools\sqlite\sqlite3.exe` = `C:\lab\tools\sqlite\sqlite3.exe` |
| `.claude\skills\sandbox-lab\SKILL.md` | documentação completa do laboratório |

Variáveis exigidas pelos perfis `app*`: `LAB_APP_PROJECT` (pasta do projeto, montada read-only em `C:\labproject`) e `LAB_WORK_DIR` (pasta de trabalho). Controle do guest pelo host: `wsb exec --id <id> -r ExistingLogin|System -c "<comando>"` (sem saída de processo) e `winapp ... --on sandbox` / `winapp target ... sandbox` (exige `networking: Enable`).

## Design

### Componentes

```
.claude/skills/claudia-sandbox/          (nova)
├─ SKILL.md
├─ scripts/
│  ├─ comum.ps1            ← constantes e funções compartilhadas
│  ├─ cenario-novo.ps1     ← sem dados
│  ├─ cenario-mock.ps1     ← dados fictícios
│  ├─ cenario-copia.ps1    ← cópia dos dados do usuário (exige -Autorizado)
│  ├─ publicar.ps1         ← app + cenário → app-stage.ps1 do laboratório
│  ├─ cdp-guest.ps1        ← inicia o app no guest com CDP e expõe a porta ao host
│  ├─ banco-guest.ps1      ← snapshot do banco do guest para o host
│  └─ test/
├─ fixtures/               ← candidate_base.yaml, search_variants.yaml, strategy.md, seed.sql
├─ references/             ← limites.md, pastas-de-dados.md
└─ evals/evals.json

.claude/skills/claudia-app/              (renomeada de claudia-teste)
├─ SKILL.md                ← alvo local ou Sandbox; seção de prints
├─ scripts/                ← cdp.mjs, subir.mjs, ver.mjs, banco.mjs, observar.mjs, invocar.mjs, regras.mjs
│  ├─ print-janela.ps1     ← print da janela nativa ou da tela inteira (sem CDP)
│  └─ test/
├─ references/, evals/
```

### Contrato dos cenários (`claudia-sandbox`)

Cada `cenario-*.ps1`:
- Parâmetro `-Saida <pasta>` (padrão `%TEMP%\claudia-sandbox\<cenario>`); a pasta é recriada a cada execução.
- Produz `<Saida>\appdata\io.github.johngabie.claudia-rh\` (pode ficar vazia) e `<Saida>\localappdata\` (vazia).
- Grava `<Saida>\cenario.json`: `{ "cenario", "criadoEm", "origem", "arquivos" }`.
- Última linha do stdout: o JSON de `cenario.json`. Exit 0 em sucesso, 2 em erro de uso, 1 em falha.

| Cenário | Conteúdo |
|---|---|
| `novo` | pasta de dados vazia: o app nasce do zero (welcome, criação de perfil). |
| `mock` | YAMLs e `strategy.md` de `fixtures/`; `claudia_rh.db` criado de `src-tauri/src/db/schema.sql` + `fixtures/seed.sql`; `welcome_seen`; `disparo.json` com `ativo: false`; `modo_autonomo.json` com `skip_permissions: false`. Dados fictícios, sem pessoa real. |
| `copia` | sem `-Autorizado`: recusa com exit 2. Com: copia `%APPDATA%\io.github.johngabie.claudia-rh` só lendo a origem (banco copiado com `-wal`/`-shm` e consolidado na cópia com `PRAGMA wal_checkpoint(TRUNCATE)`); `%LOCALAPPDATA%` (perfil do browser, cookies) **não** é copiado. Na cópia, `disparo.json.ativo=false` e `modo_autonomo.json.skip_permissions=false`, salvo `-ManterAutomacao`. |

`disparo.json` é sempre gravado com todos os campos (`ativo`, `limiar_minutos`, `limite_diario`, `limite_tempo_minutos`, `janelas`): sem `limiar_minutos` o app ignora o arquivo e assume `ativo: true` (`src-tauri/src/lib.rs`, `load_idle_config`).

`publicar.ps1 -App <exe|instalador|pasta> -Cenario <pasta do cenário> [-WorkDir]` chama `app-stage.ps1` do laboratório com `-AppData <Cenario>\appdata\io.github.johngabie.claudia-rh -Clean`. `-WorkDir` padrão: `$env:LAB_WORK_DIR`, senão `%TEMP%\claudia-sandbox\work`.

### CDP do app dentro do Sandbox (`claudia-sandbox`)

`cdp-guest.ps1 [-Id <sandbox>] [-Porta 9333] [-PortaHost 19333] [-Iniciar]`, rodado no host:
1. Resolve o id (único em `wsb list` se omitido) e o IP do guest (`wsb ip`).
2. Como System no guest: `netsh interface portproxy add v4tov4 listenaddress=0.0.0.0 listenport=<PortaHost> connectaddress=127.0.0.1 connectport=<Porta>` e regra de firewall de entrada para `<PortaHost>`.
3. Com `-Iniciar`: como o usuário do guest, define `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<Porta>` e roda `C:\labsandbox\app\guest-run.ps1` no mesmo processo.
4. Espera até `http://<ip>:<PortaHost>/json/version` responder (timeout com mensagem) e imprime `{ "host", "porta" }` para usar como `CDP_HOST`/`CDP_PORT`.

Pré-requisito: perfil com rede (`lab.config.app-winapp.json`) e WebView2 presente no guest. O `.exe` de `tauri build --no-bundle` não instala o WebView2; o instalador sim. Sem WebView2 o app não abre e o CDP não existe: isso é resultado do teste.

`banco-guest.ps1 [-Id <sandbox>]`: no guest, `C:\lab\tools\sqlite\sqlite3.exe "%APPDATA%\io.github.johngabie.claudia-rh\claudia_rh.db" ".backup C:\labwork\db\claudia_rh.db"`; imprime o caminho no host (`<WorkDir>\guest-rw\db\claudia_rh.db`) para usar como `CLAUDIA_DB`. É um snapshot: repetir após cada ação que muda o banco.

### Observação com alvo local ou Sandbox (`claudia-app`)

- `CDP_HOST` (padrão `127.0.0.1`) e `CDP_PORT` em todos os scripts que falam CDP (`cdp.mjs`, `subir.mjs`, `ver.mjs`, `invocar.mjs`, `observar.mjs`).
- `regras.validarVersaoApp` aceita a página do `tauri dev` (`localhost:1420`) e a do build de release (origem `tauri.localhost`).
- `regras.decidirSubida` ganha `remoto`: com `CDP_HOST` diferente de `127.0.0.1`/`localhost`, nunca sobe `tauri dev`; se o CDP não responde, `abortar` com motivo apontando para `claudia-sandbox` (`cdp-guest.ps1 -Iniciar`).
- `banco.mjs` e `observar.mjs` usam `CLAUDIA_DB` quando definido; senão, o banco do `%APPDATA%` local.
- Regras de segurança existentes (rótulo perigoso, SQL só leitura, comando de escrita só com `--escrever`) valem igual nos dois alvos.

### Prints (`claudia-app`)

Escada, do mais fiel ao mais robusto; usar o primeiro que funcionar e dizer qual foi usado:

| # | Local (host) | Sandbox | Mostra |
|---|---|---|---|
| 1 | `node scripts/ver.mjs tela <arq>` | idem, com `CDP_HOST`/`CDP_PORT` | só a página do WebView2 |
| 2 | `pwsh scripts/print-janela.ps1 -Processo claudia-rh -Saida <arq>` | `winapp ui screenshot --on sandbox -a claudia-rh -o <arq>` | janela nativa inteira, inclusive diálogos nativos que o CDP não vê |
| 3 | `pwsh scripts/print-janela.ps1 -TelaInteira -Saida <arq>` | `winapp target screenshot sandbox -o <arq>` | tela inteira |
| 4 | — | sem rede: `C:\labsandbox\app\guest-observe.ps1` via `wsb exec` (PNG em `C:\labwork\run`) | tela inteira + árvore UIA |

`print-janela.ps1`: PowerShell + `System.Drawing`, sem dependências; captura o retângulo da janela principal do processo (DPI-aware) ou a tela inteira; só leitura (não foca, não clica). Prints vão para `debug-logs/prints/` por padrão (já fora do git) e podem conter dados pessoais: não anexar a issues públicas sem revisar.

Regra: toda afirmação sobre a UI vem com print; se nenhum degrau funcionar, dizer quais falharam e por quê.

### Rename `claudia-teste` → `claudia-app`

- `git mv .claude/skills/claudia-teste .claude/skills/claudia-app`; `name:` no frontmatter; `NOME` em `evals/rodar-evals.mjs`; comentários que citam o nome.
- `description`: observar/exercitar o app rodando, local ou no Sandbox; prints; bugs (reproduzir primeiro). Mantém os gatilhos atuais.
- `CLAUDE.md`: referências ao nome antigo passam ao novo.
- Specs e planos antigos em `docs/superpowers/` mantêm o nome antigo (registro histórico).

### Conteúdo da `claudia-sandbox`

1. Pré-requisitos: `SANDBOX_LAB_DIR`; ausente → parar e dizer o que falta.
2. Laboratório em uma tela + ponteiro para `sandbox-lab/SKILL.md` do laboratório.
3. Específico do app: pastas de dados, browser próprio baixado na primeira abertura, `debug-logs`, Activity log.
4. Cenários: o que cada um produz e quando faz sentido; `copia` exige pedir autorização ao usuário naquela rodada.
5. O que publicar: exe de `tauri build --no-bundle` (sem pré-requisitos) ou instalador (cuida de pré-requisitos como o WebView2; instalação silenciosa NSIS com `/S`).
6. Ligar o CDP no guest (`cdp-guest.ps1`) e observar com a `claudia-app`; banco via `banco-guest.ps1`; prints pela escada.
7. Regras: nunca acionar envio de candidatura; falha do app num Windows limpo é resultado do teste — registrar, não contornar; achado fora do escopo vira task no Linear (`linear-claudia`).
8. Melhorias no próprio ferramental (laboratório, scripts e cenários desta skill, a skill em si): quando o agente notar algo que tornaria o teste mais fácil, rápido ou confiável, sugere ao usuário com o motivo e o custo; com o "sim" do usuário, implementa (o laboratório tem processo próprio, fora deste repositório); se não for o momento, registra no Linear (`linear-claudia`). Nunca muda o ferramental por conta própria no meio de um teste.
9. Testes: o agente desenha os testes e decide se escreve testes automáticos.

Gatilhos: testar em Windows limpo/isolado, Sandbox, instalador, primeira abertura, desinstalação, "funciona num PC novo", testar com dados do usuário sem mexer neles. Observar o app já rodando (local ou no Sandbox) é da `claudia-app`.

### `CLAUDE.md`

- Referências a `claudia-teste` → `claudia-app`.
- Seção curta "Teste em máquina limpa" com a tabela:

| Quero… | Skill |
|---|---|
| ver/clicar/conferir o app rodando, tirar print, reproduzir bug | `claudia-app` (local ou Sandbox) |
| montar Windows limpo, escolher dados, instalar/publicar o app, ligar CDP no guest | `claudia-sandbox` |

### Erros

- Laboratório ausente ou incompleto: scripts param com mensagem que nomeia a variável/arquivo.
- `copia` sem dados de origem: exit 1 com o caminho procurado.
- `sqlite3` ausente: `mock`/`copia` falham com os caminhos tentados (`-Sqlite` permite outro).
- CDP remoto sem resposta: `subir.mjs` aborta com motivo; `cdp-guest.ps1` sai com timeout e o URL testado.

## Critérios de teste

- `claudia-sandbox/scripts/test/*.test.ps1` (sem Sandbox): cada cenário com origem/destino temporários; `copia` sem `-Autorizado` → exit 2; origem não modificada (hash antes/depois); automação desligada na cópia; banco do `mock` abre e tem as linhas do seed; contrato de `cenario.json`; `publicar.ps1` contra laboratório falso. Exit ≠ 0 em qualquer falha.
- `claudia-app/scripts/test/regras.test.mjs`: página de release aceita; `remoto` sem CDP → `abortar`; `remoto` nunca → `subir`; casos atuais continuam verdes.
- `print-janela.ps1`: teste que captura a tela inteira para um arquivo temporário e confere PNG válido com dimensões > 0.
- Prova de mutação: quebrar uma regra faz o teste nomeado falhar.
- Smoke no Sandbox (`lab.config.app-winapp.json`, internet bloqueada após a instalação): cenários `novo` e `mock`; prints pela escada; CDP no guest com o app instalado pelo instalador (`ver.mjs arvore` e `tela` via `CDP_HOST`); `copia` só com autorização do usuário naquela rodada.
- Smoke local: `claudia-app` com `tauri dev` continua funcionando (`subir.mjs`, `ver.mjs arvore`, `print-janela.ps1`).
- Evals de disparo: `claudia-sandbox` dispara nos casos dela; `claudia-app` mantém os resultados da `claudia-teste` e não dispara nos casos de montar ambiente.
- Varredura de agnosticismo nos arquivos novos/alterados.

## Fora do escopo

- Mudar o laboratório.
- Mudar código do app (achados viram tasks no Linear).

## Não verificado

- Instalação silenciosa do instalador NSIS dentro do Sandbox (o bootstrapper do WebView2 precisa de rede durante a instalação).
- Origem exata da página no build de release (`tauri.localhost`, por `http` ou `https`): conferir no smoke e ajustar a regra se diferir.
