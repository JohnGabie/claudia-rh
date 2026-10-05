# Teste em máquina limpa (Windows Sandbox) — design

## Pedido do usuário

- Usar um laboratório Windows Sandbox, mantido **fora do repositório**, para testar o Claudia RH num Windows limpo e isolado.
- Uma **skill nova** que mostre ao agente como o laboratório funciona e o que fazer, deixando o agente elaborar os testes conforme a situação (inclusive escrever testes automáticos, se fizer sentido).
- **Cenários de dados**, cada um com script: cópia do banco do usuário, tudo novo (sem dados) e dados mockados. O agente decide na hora qual faz sentido.
- Só o cenário com os dados do usuário exige que o agente peça autorização. No resto, sem travas extras.
- Ajustar o `CLAUDE.md` e a skill `claudia-teste` para apontar para isso.
- Arquivos agnósticos: legíveis e executáveis por qualquer agente sem contexto externo.

## Leitura do pedido

- **Laboratório externo:** o repositório não contém o laboratório; só o encontra por variável de ambiente e usa a interface pública dele.
- **Skill como mapa:** a skill descreve capacidades, limites e regras. Não impõe roteiro de teste.
- **Três cenários:** produzem uma pasta de dados pronta para publicar no Sandbox. A escolha é do agente.
- **Autorização:** obrigatória apenas para o cenário que lê os dados reais do usuário.

## Contexto: interface do laboratório

O laboratório é uma pasta independente (variável `SANDBOX_LAB_DIR`) com:

| Item | Uso |
|---|---|
| `scripts\test-up.ps1 -Config <perfil>` / `scripts\test-down.ps1` | sobe / derruba o Sandbox (um por vez); o id da instância aparece na saída do `test-up` e em `wsb list` |
| `sandbox\` | montada read-only no guest em `C:\labsandbox` (scripts do guest: `C:\labsandbox\app\...`) |
| `scripts\app-stage.ps1 -App <exe\|pasta> -AppData <pastas> -LocalAppData <pastas> [-WorkDir] [-Clean]` | publica app e dados em `<WorkDir>\deploy` (no guest: `C:\appdeploy`) |
| `lab.config.app.json` | app sem rede |
| `lab.config.app-winapp.json` | app com rede para o winapp; internet bloqueada por `sandbox\app\guest-firewall.ps1` |
| `lab.config.app-build.json` | build dentro do guest com toolchains do host em read-only |
| `sandbox\app\guest-run.ps1` | no guest: copia `appdata`/`localappdata` para `%APPDATA%`/`%LOCALAPPDATA%` e inicia o exe |
| `sandbox\app\guest-chrome-probe.ps1`, `guest-observe.ps1`, `guest-click.ps1` | observar Chrome com DevTools, tela/árvore UIA, clique sem winapp |
| `shared\tools\sqlite\sqlite3.exe` | sqlite3 portátil |
| `.claude\skills\sandbox-lab\SKILL.md` | documentação completa do laboratório |

Variáveis exigidas pelos perfis `app*`: `LAB_APP_PROJECT` (pasta do projeto, montada read-only em `C:\labproject`) e `LAB_WORK_DIR` (pasta de trabalho).

## Design

### Componentes

```
.claude/skills/claudia-sandbox/
├─ SKILL.md
├─ scripts/
│  ├─ comum.ps1            ← constantes e funções compartilhadas (identifier, lab, sqlite3, cenario.json)
│  ├─ cenario-novo.ps1     ← sem dados
│  ├─ cenario-mock.ps1     ← dados fictícios
│  ├─ cenario-copia.ps1    ← cópia dos dados do usuário (exige -Autorizado)
│  ├─ publicar.ps1         ← app + cenário → app-stage.ps1 do laboratório
│  └─ test/cenarios.test.ps1
├─ fixtures/
│  ├─ candidate_base.yaml
│  ├─ search_variants.yaml
│  ├─ strategy.md
│  └─ seed.sql
├─ references/
│  ├─ limites.md           ← comportamentos conhecidos do Sandbox/winapp
│  └─ pastas-de-dados.md   ← o que o app guarda em %APPDATA% e %LOCALAPPDATA%
└─ evals/evals.json
```

### Contrato dos cenários

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

`publicar.ps1 -App <exe|instalador|pasta> -Cenario <pasta do cenário> [-WorkDir]` chama `app-stage.ps1` do laboratório com `-AppData <Cenario>\appdata\io.github.johngabie.claudia-rh -Clean`. `-WorkDir` padrão: `$env:LAB_WORK_DIR`, senão `%TEMP%\claudia-sandbox\work`.

### Conteúdo da skill

1. Pré-requisitos: `SANDBOX_LAB_DIR`; ausente → parar e dizer o que falta.
2. Laboratório em uma tela + ponteiro para `sandbox-lab/SKILL.md` do laboratório.
3. Específico do app: pastas de dados, browser próprio baixado na primeira abertura, `debug-logs`, Activity log.
4. Cenários: o que cada um produz e quando faz sentido; `copia` exige pedir autorização ao usuário naquela rodada.
5. O que publicar: exe de `tauri build --no-bundle` (sem pré-requisitos) ou instalador (cuida de pré-requisitos como o WebView2; instalação silenciosa NSIS com `/S`).
6. Observar: app (tela, árvore, banco, logs), browser do app (CDP dentro do guest), Activity log.
7. Regras: nunca acionar envio de candidatura; falha do app num Windows limpo é resultado do teste — registrar, não contornar; achado fora do escopo vira task no Linear (`linear-claudia`).
8. Testes: o agente desenha os testes e decide se escreve testes automáticos.

Gatilhos: testar em Windows limpo/isolado, Sandbox, instalador, primeira abertura, desinstalação, "funciona num PC novo", testar com dados do usuário sem mexer neles. Não dispara para o app em `tauri dev` no host (`claudia-teste`).

### Mudanças em arquivos existentes

- `claudia-teste/SKILL.md`: uma frase na descrição (Windows limpo/Sandbox → `claudia-sandbox`) e um item em "Mais detalhe". Scripts sem mudança.
- `claudia-teste/evals/evals.json`: casos de máquina limpa como "não deve disparar".
- `CLAUDE.md`: seção curta "Teste em máquina limpa" (quando usar, skill, variável `SANDBOX_LAB_DIR`).

### Erros

- Laboratório ausente ou incompleto: scripts param com mensagem que nomeia a variável/arquivo.
- `copia` sem dados de origem: exit 1 com o caminho procurado.
- `sqlite3` ausente: `mock`/`copia` falham com o caminho procurado (`-Sqlite` permite outro).

## Critérios de teste

- `scripts/test/cenarios.test.ps1` (sem Sandbox): cada cenário com origem/destino temporários; `copia` sem `-Autorizado` → exit 2; origem não modificada (hash antes/depois); automação desligada na cópia; banco do `mock` abre e tem as linhas do seed; contrato de `cenario.json`. Exit code ≠ 0 em qualquer falha.
- Prova de mutação: quebrar uma regra (ex.: não desligar `disparo`) faz o teste nomeado falhar.
- Smoke no Sandbox (`lab.config.app-winapp.json`, internet bloqueada): cenários `novo` e `mock` com o exe de release, prints da tela de cada um; `copia` só com autorização do usuário naquela rodada.
- Evals de disparo: `claudia-sandbox` dispara nos casos dela; `claudia-teste` não dispara nos de máquina limpa e mantém os resultados atuais.
- Varredura de agnosticismo nos arquivos novos/alterados: sem referências a sessões, pessoas, máquinas ou caminhos pessoais.

## Fora do escopo

- Mudar o laboratório.
- Mudar código do app (achados viram tasks no Linear).
- Conectar os scripts da `claudia-teste` ao WebView2 dentro do guest.

## Não verificado

- Instalação silenciosa do instalador NSIS dentro do Sandbox com internet bloqueada (o bootstrapper do WebView2 pode precisar de rede).
