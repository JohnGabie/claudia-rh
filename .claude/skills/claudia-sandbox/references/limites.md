# Limites do laboratório que afetam o teste

Cada item traz o comando que demonstra o comportamento. **Não verificado** quer dizer que o comando ainda não foi rodado contra um Sandbox de verdade para reproduzi-lo; a fonte indica onde o laboratório documenta o comportamento. Quem reproduzir um item atualiza a linha com data, comando e resultado.

Comandos de host a partir de `%SANDBOX_LAB_DIR%`; `<id>` vem de `wsb list`.

## 1. winapp não alcança o guest com `networking: Disable`

- **Comportamento:** no perfil `lab.config.app.json` (sem rede), `winapp ... --on sandbox` não chega ao guest. Controle e prints só por `wsb exec` + `C:\labsandbox\app\guest-observe.ps1` / `guest-click.ps1`.
- **Demonstra:** `scripts\test-up.ps1 -Config lab.config.app.json` e depois `winapp ui list-windows --on sandbox`.
- **Fonte:** `$comment` de `lab.config.app.json` e tabela de perfis em `.claude\skills\sandbox-lab\SKILL.md`.
- **Situação:** não verificado.

## 2. App iniciado por `winapp target exec` termina quando o comando termina

- **Comportamento:** o processo filho morre junto com o `winapp target exec`. Para o app continuar aberto, inicie com `wsb exec --id <id> -r ExistingLogin -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1"` (ou `cdp-guest.mjs --iniciar`, que já faz isso).
- **Demonstra:** `winapp target exec sandbox -- powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1` e, depois que o comando volta, `winapp ui list-windows --on sandbox` sem a janela do app.
- **Fonte:** cabeçalho de `sandbox\app\guest-run.ps1` e seção de perfis `app*` do `sandbox-lab\SKILL.md`.
- **Situação:** não verificado.

## 3. `wsb exec` não devolve a saída do processo

- **Comportamento:** o stdout do comando executado no guest não volta ao host. Scripts do guest gravam o resultado em `C:\labwork\...` (ex.: `C:\labwork\run\guest-run.json`), lido no host em `<work-dir>\guest-rw\...`. `cdp-guest.mjs` e `banco-guest.mjs` seguem esse padrão e só tratam o código de saída do `wsb exec`.
- **Demonstra:** `wsb exec --id <id> -r ExistingLogin -c "cmd /c echo marcador"`: o host não imprime `marcador`.
- **Fonte:** interface do laboratório na spec do repositório; `wsb exec --help` não menciona saída do processo.
- **Situação:** não verificado.

## 4. Target do cargo em pasta mapeada falha com `os error 1392`

- **Comportamento:** build dentro do guest (`lab.config.app-build.json`) com o target do cargo numa pasta mapeada quebra os build scripts. Use o disco do guest (`C:\labdata\cargo-target`, padrão do `guest-build.ps1`).
- **Demonstra:** `winapp target exec sandbox -- powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-build.ps1` com `CARGO_TARGET_DIR` apontando para `C:\labwork\target`; o log em `C:\labwork\build\guest-build.log` mostra `os error 1392`.
- **Fonte:** cabeçalho de `sandbox\app\guest-build.ps1`.
- **Situação:** não verificado.

## 5. Só um Sandbox por vez

- **Comportamento:** o Windows só mantém uma instância do Windows Sandbox. Antes de `test-up.ps1`, confira `wsb list`; se já houver um de outro teste, não o derrube sem falar com o usuário. Sem Sandbox de pé, `winapp ... --on sandbox` cria um novo **sem** os mapeamentos do laboratório.
- **Demonstra:** com um Sandbox de pé, `wsb start` falha ou não cria uma segunda instância (`wsb list` continua com um id).
- **Fonte:** seção "Ciclo de vida" do `sandbox-lab\SKILL.md`.
- **Situação:** não verificado.

## 6. `.exe` de `tauri build --no-bundle` não instala pré-requisitos

- **Comportamento:** o exe solto depende do runtime WebView2 já presente no guest. O instalador NSIS (`tauri build`) cuida do WebView2, mas o bootstrapper precisa de rede durante a instalação: instale antes de rodar `guest-firewall.ps1`. Sem WebView2 o app não abre e não há CDP; isso é resultado do teste.
- **Demonstra:** publicar o exe de `--no-bundle`, rodar `guest-run.ps1` e ler `C:\labwork\run\guest-run.json` (`webview2.registryPv` vazio e app sem janela).
- **Fonte:** passo 1 do `sandbox\app\guest-run.ps1` (registra o estado do WebView2); spec, seção "CDP do app dentro do Sandbox".
- **Situação:** não verificado (inclusive a instalação silenciosa `/S` do NSIS no guest).

## 7. WebView2 só aceita CDP no `127.0.0.1` do guest

- **Comportamento:** com `--remote-debugging-port`, o DevTools do WebView2 escuta só no loopback do guest; o host não alcança direto pelo IP do guest. `cdp-guest.mjs` cria, como System, `netsh interface portproxy add v4tov4 listenaddress=0.0.0.0 listenport=<porta-host> connectaddress=127.0.0.1 connectport=<porta>` e uma regra de firewall de entrada para `<porta-host>`.
- **Demonstra:** com o app aberto com CDP no guest, `curl http://<ip do guest>:9333/json/version` (ip de `wsb ip --id <id>`) não responde; depois de `node .claude/skills/claudia-sandbox/scripts/cdp-guest.mjs --id <id>`, `curl http://<ip do guest>:19333/json/version` responde.
- **Fonte:** spec do repositório, seção "CDP do app dentro do Sandbox".
- **Situação:** não verificado.

## Evals de disparo

Medidos com `node .claude/skills/claudia-sandbox/evals/rodar-evals.mjs sonnet` sobre `evals/evals.json` (9 "deve disparar", 6 "não deve"). Critério: a **primeira** chamada de ferramenta do modelo é `Skill(claudia-sandbox)`. Uma execução `INVALIDO` (limite de sessão, falha do CLI) não conta, nem como acerto nem como falha.

| Rodada (2026-10-05) | Resultado | Inválidas |
|---|---|---|
| 1 | 15/15 | 0 |
| 2 | 15/15 | 0 |

Nas duas rodadas, os 9 casos "deve" chamaram `Skill(claudia-sandbox)`. Nos "não deve": olhar a tela do app aberto, print do Perfil e "olha o app que tá rodando no Sandbox" foram para `Skill(claudia-app)`; o registro no Linear para `Skill(linear-claudia)`; `cargo test` e o ajuste de CSS começaram por ferramenta comum (`PowerShell`, `Grep`), sem skill.

Limites da medição: um modelo só, duas rodadas, prompts curtos. A descrição não precisou de ajuste depois da primeira versão (894 caracteres). Ao mudar a descrição desta skill ou a da `claudia-app`, rode de novo os evals das duas: os casos de fronteira ("olha o app no Sandbox" e "monta um Windows limpo") dependem das duas descrições juntas.
