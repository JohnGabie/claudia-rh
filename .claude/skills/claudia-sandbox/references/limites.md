# Limites do laboratório que afetam o teste

Cada item traz o comando que demonstra o comportamento. **Não verificado** quer dizer que o comando ainda não foi rodado contra um Sandbox de verdade para reproduzi-lo (2026-10-05: itens 1, 2, 3, 5, 6 e 8 verificados num Sandbox real); a fonte indica onde o laboratório documenta o comportamento. Quem reproduzir um item atualiza a linha com data, comando e resultado.

Comandos de host a partir de `%SANDBOX_LAB_DIR%`; `<id>` vem de `wsb list`.

## 1. winapp não alcança o guest com `networking: Disable`

- **Comportamento:** no perfil `lab.config.app.json` (sem rede), `winapp ... --on sandbox` não chega ao guest. Controle e prints só por `wsb exec` + `C:\labsandbox\app\guest-observe.ps1` / `guest-click.ps1`.
- **Demonstra:** `scripts\test-up.ps1 -Config lab.config.app.json` e depois `winapp ui list-windows --on sandbox`.
- **Fonte:** `$comment` de `lab.config.app.json` e tabela de perfis em `.claude\skills\sandbox-lab\SKILL.md`.
- **Situação:** verificado (2026-10-05). Com `lab.config.app.json` de pé, `winapp ui list-windows --on sandbox` saiu com 70 em ~7 s ("A Windows Sandbox is already running but did not respond") e não criou um segundo Sandbox.

## 2. App iniciado por `winapp target exec` termina quando o comando termina

- **Comportamento:** o processo filho morre junto com o `winapp target exec`. Para o app continuar aberto, inicie com `wsb exec --id <id> -r ExistingLogin -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1"` (ou `cdp-guest.mjs --iniciar`, que já faz isso).
- **Demonstra:** `winapp target exec sandbox -- powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1` e, depois que o comando volta, `winapp ui list-windows --on sandbox` sem a janela do app.
- **Fonte:** cabeçalho de `sandbox\app\guest-run.ps1` e seção de perfis `app*` do `sandbox-lab\SKILL.md`.
- **Situação:** verificado (2026-10-05). `winapp target exec sandbox -- powershell ... guest-run.ps1` voltou com 0 e, depois, `winapp ui list-windows --on sandbox` não listou o app. Contraprova: iniciado por `wsb exec`, o app seguiu aberto minutos depois. Observação: o `winapp target exec` **devolve** o stdout (alternativa para comandos curtos que não deixam app aberto).

## 3. `wsb exec` não devolve a saída do processo

- **Comportamento:** o stdout do comando executado no guest não volta ao host. Scripts do guest gravam o resultado em `C:\labwork\...` (ex.: `C:\labwork\run\guest-run.json`), lido no host em `<work-dir>\guest-rw\...`. `cdp-guest.mjs` e `banco-guest.mjs` seguem esse padrão. O host recebe apenas uma linha localizada com o código de saída do guest (item 8).
- **Demonstra:** `wsb exec --id <id> -r ExistingLogin -c "cmd /c echo marcador"`: o host não imprime `marcador`, só a linha do código de saída.
- **Fonte:** interface do laboratório na spec do repositório; `wsb exec --help` não menciona saída do processo.
- **Situação:** verificado (2026-10-05) com o comando acima.

## 4. Target do cargo em pasta mapeada falha com `os error 1392`

- **Comportamento:** build dentro do guest (`lab.config.app-build.json`) com o target do cargo numa pasta mapeada quebra os build scripts. Use o disco do guest (`C:\labdata\cargo-target`, padrão do `guest-build.ps1`).
- **Demonstra:** `winapp target exec sandbox -- powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-build.ps1` com `CARGO_TARGET_DIR` apontando para `C:\labwork\target`; o log em `C:\labwork\build\guest-build.log` mostra `os error 1392`.
- **Fonte:** cabeçalho de `sandbox\app\guest-build.ps1`.
- **Situação:** não verificado.

## 5. Só um Sandbox por vez

- **Comportamento:** o Windows só mantém uma instância do Windows Sandbox. Antes de `test-up.ps1`, confira `wsb list`; se já houver um de outro teste, não o derrube sem falar com o usuário. Sem Sandbox de pé, `winapp ... --on sandbox` cria um novo **sem** os mapeamentos do laboratório.
- **Demonstra:** com um Sandbox de pé, `wsb start` falha ou não cria uma segunda instância (`wsb list` continua com um id).
- **Fonte:** seção "Ciclo de vida" do `sandbox-lab\SKILL.md`.
- **Situação:** verificado (2026-10-05): `wsb start` com um Sandbox de pé falhou com `CO_E_APPSINGLEUSE` (0x800401F6) e `wsb list` manteve um id só.

## 6. `.exe` de `tauri build --no-bundle` não instala pré-requisitos

- **Comportamento:** o exe solto depende do runtime WebView2 já presente no guest. O instalador NSIS (`tauri build`) cuida do WebView2, mas o bootstrapper precisa de rede durante a instalação: instale antes de rodar `guest-firewall.ps1`. Sem WebView2 o app não abre e não há CDP; isso é resultado do teste.
- **Demonstra:** publicar o exe de `--no-bundle`, rodar `guest-run.ps1` e ler `C:\labwork\run\guest-run.json` (`webview2.registryPv` vazio). Na prática o exe abre uma janela "Claudia RH" em branco mais o diálogo nativo "Could not find the WebView2 Runtime" (`windows = ["Error"]` no JSON).
- **Fonte:** passo 1 do `sandbox\app\guest-run.ps1` (registra o estado do WebView2); spec, seção "CDP do app dentro do Sandbox".
- **Situação:** verificado (2026-10-05): o exe solto mostra o diálogo acima; o instalador `/S` com rede levou ~46 s, instalou o WebView2 e o app em `%LOCALAPPDATA%\claudia-rh\`, e não abre o app ao terminar.

## 7. WebView2 só aceita CDP no `127.0.0.1` do guest

- **Comportamento:** com `--remote-debugging-port`, o DevTools do WebView2 escuta só no loopback do guest; o host não alcança direto pelo IP do guest. `cdp-guest.mjs` cria, como System, `netsh interface portproxy add v4tov4 listenaddress=0.0.0.0 listenport=<porta-host> connectaddress=127.0.0.1 connectport=<porta>` e uma regra de firewall de entrada para `<porta-host>`.
- **Demonstra:** com o app aberto com CDP no guest, `curl http://<ip do guest>:9333/json/version` (ip de `wsb ip --id <id>`) não responde; depois de `node .claude/skills/claudia-sandbox/scripts/cdp-guest.mjs --id <id>`, `curl http://<ip do guest>:19333/json/version` responde.
- **Fonte:** spec do repositório, seção "CDP do app dentro do Sandbox".
- **Situação:** portproxy e regra de firewall verificados como funcionais (2026-10-05), mas CDP não respondeu em nenhuma tentativa (release e debug). Não há resposta HTTP em `http://127.0.0.1:9333/json/version` no guest, com ou sem `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` e com ou sem política por app.

## 8. `wsb exec` sempre sai com 0 no host

- **Comportamento:** o código de saída do `wsb exec` é 0 mesmo quando o processo no guest falha; o código do guest só aparece como linha localizada no stdout (pt-BR: "Processo encerrado com o código: 3"). `erroDeExec` em `scripts/comum.mjs` lê a última linha desse formato (tolerante a idioma) e trata código não zero como erro, com `err.status` igual ao código do guest; sem a linha, não é erro. Status não zero do próprio `wsb` continua erro.
- **Demonstra:** `wsb exec --id <id> -r ExistingLogin -c "cmd /c exit 7"`: exit 0 no host, stdout com "... código: 7".
- **Situação:** verificado (2026-10-05, pt-BR; também com `powershell -EncodedCommand` e `exit 3`). Outros idiomas: o parser aceita "code", mas o texto exato não foi visto ao vivo.

## 9. CDP no guest não funciona

- **Comportamento:** a porta de depuração remota do WebView2 não responde, nem com build release nem com build de debug. Com o app rodando no guest (debug ou release), o processo do `msedgewebview2.exe` não exibe `--remote-debugging-port` na linha de comando mesmo com `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333` no ambiente do processo (`claudia-rh.exe`). Política por app (`HKCU\Software\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments`) também é ignorada. O `portproxy` do `cdp-guest.mjs` e a regra de firewall funcionam (confirmado ao vivo), mas não há resposta HTTP. A causa não foi isolada (possível diferença: runtime WebView2 do guest vs. host, usuário `WDAGUtilityAccount`).
- **Demonstra:** `node .claude/skills/claudia-sandbox/scripts/cdp-guest.mjs --iniciar --exe-path <exe no guest>` (debug ou release): exit 1 em ~90 s, "CDP did not answer ... within 90s".
- **Situação:** falha verificada com ambos os builds (2026-10-05). Use degraus 2 e 3 da escada de prints (`claudia-app`) em vez de CDP.

## Evals de disparo

Medidos com `node .claude/skills/claudia-sandbox/evals/rodar-evals.mjs sonnet` sobre `evals/evals.json` (9 "deve disparar", 6 "não deve"). Critério: a **primeira** chamada de ferramenta do modelo é `Skill(claudia-sandbox)`. Uma execução `INVALIDO` (limite de sessão, falha do CLI) não conta, nem como acerto nem como falha.

| Rodada (2026-10-05) | Resultado | Inválidas |
|---|---|---|
| 1 | 15/15 | 0 |
| 2 | 15/15 | 0 |

Nas duas rodadas, os 9 casos "deve" chamaram `Skill(claudia-sandbox)`. Nos "não deve": olhar a tela do app aberto, print do Perfil e "olha o app que tá rodando no Sandbox" foram para `Skill(claudia-app)`; o registro no Linear para `Skill(linear-claudia)`; `cargo test` e o ajuste de CSS começaram por ferramenta comum (`PowerShell`, `Grep`), sem skill.

Limites da medição: um modelo só, duas rodadas, prompts curtos. A descrição não precisou de ajuste depois da primeira versão (894 caracteres). Ao mudar a descrição desta skill ou a da `claudia-app`, rode de novo os evals das duas: os casos de fronteira ("olha o app no Sandbox" e "monta um Windows limpo") dependem das duas descrições juntas.
