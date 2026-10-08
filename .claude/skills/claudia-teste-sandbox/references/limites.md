# Limites do laboratório que afetam o teste

Cada item traz o comando que demonstra o comportamento. **Não verificado** quer dizer que o comando ainda não foi rodado contra um Sandbox de verdade para reproduzi-lo (2026-10-05: itens 1, 2, 3, 5, 6 e 8 verificados num Sandbox real); a fonte indica onde o laboratório documenta o comportamento. Quem reproduzir um item atualiza a linha com data, comando e resultado.

Comandos de host a partir do `lab` do `onde.mjs` (`sandbox-lab/` do checkout principal); `<id>` vem de `wsb list`.

## 1. winapp não alcança o guest com `networking: Disable`

- **Comportamento:** no ambiente `app` (sem rede), `winapp ... --on sandbox` não chega ao guest. Controle e prints só por `wsb exec` + `C:\labsandbox\app\guest-observe.ps1` / `guest-click.ps1`.
- **Demonstra:** `scripts\test-up.ps1 -Ambiente app -Project <projeto>` e depois `winapp ui list-windows --on sandbox`.
- **Fonte:** `$comment` de `app` no `compose.json` e tabela de ambientes em `references/laboratorio.md`.
- **Situação:** verificado (2026-10-05). Com `app` de pé, `winapp ui list-windows --on sandbox` saiu com 70 em ~7 s ("A Windows Sandbox is already running but did not respond") e não criou um segundo Sandbox.

## 2. App iniciado por `winapp target exec` termina quando o comando termina

- **Comportamento:** o processo filho morre junto com o `winapp target exec`. Para o app continuar aberto, inicie com `wsb exec --id <id> -r ExistingLogin -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1"` (ou `cdp-guest.mjs --iniciar`, que já faz isso).
- **Demonstra:** `winapp target exec sandbox -- powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1` e, depois que o comando volta, `winapp ui list-windows --on sandbox` sem a janela do app.
- **Fonte:** cabeçalho de `sandbox\app\guest-run.ps1` e seção de perfis `app*` do `references/laboratorio.md`.
- **Situação:** verificado (2026-10-05). `winapp target exec sandbox -- powershell ... guest-run.ps1` voltou com 0 e, depois, `winapp ui list-windows --on sandbox` não listou o app. Contraprova: iniciado por `wsb exec`, o app seguiu aberto minutos depois. Observação: o `winapp target exec` **devolve** o stdout (alternativa para comandos curtos que não deixam app aberto).

## 3. `wsb exec` não devolve a saída do processo

- **Comportamento:** o stdout do comando executado no guest não volta ao host. Scripts do guest gravam o resultado em `C:\labwork\...` (ex.: `C:\labwork\run\guest-run.json`), lido no host em `.sandbox-work\guest-rw\...`. `cdp-guest.mjs` e `banco-guest.mjs` seguem esse padrão. O host recebe apenas uma linha localizada com o código de saída do guest (item 8).
- **Demonstra:** `wsb exec --id <id> -r ExistingLogin -c "cmd /c echo marcador"`: o host não imprime `marcador`, só a linha do código de saída.
- **Fonte:** interface do laboratório na spec do repositório; `wsb exec --help` não menciona saída do processo.
- **Situação:** verificado (2026-10-05) com o comando acima.

## 4. Target do cargo em pasta mapeada falha com `os error 1392`

- **Comportamento:** build dentro do guest (ambiente `testes`) com o target do cargo numa pasta mapeada quebra os build scripts. Use o disco do guest (`C:\labdata\cargo-target`, padrão).
- **Demonstra:** `scripts\test-up.ps1 -Ambiente testes -Project <projeto>` e depois `wsb exec --id <id> -r ExistingLogin -c "cd <projeto> && CARGO_TARGET_DIR=C:\labwork\target cargo test --lib"` (quando o target fica fora do disco do guest); o log em `C:\labwork\` mostra `os error 1392`.
- **Fonte:** cabeçalho de `sandbox\app\guest-build.ps1`.
- **Situação:** não verificado.

## 5. Só um Sandbox por vez

- **Comportamento:** o Windows só mantém uma instância do Windows Sandbox. Antes de `test-up.ps1`, confira `wsb list`; se já houver um de outro teste, não o derrube sem falar com o usuário. Sem Sandbox de pé, `winapp ... --on sandbox` cria um novo **sem** os mapeamentos do laboratório.
- **Demonstra:** com um Sandbox de pé, `wsb start` falha ou não cria uma segunda instância (`wsb list` continua com um id).
- **Fonte:** seção "Ciclo de vida" do `references/laboratorio.md`.
- **Situação:** verificado (2026-10-05): `wsb start` com um Sandbox de pé falhou com `CO_E_APPSINGLEUSE` (0x800401F6) e `wsb list` manteve um id só.

## 6. `.exe` de `tauri build --no-bundle` não instala pré-requisitos

- **Comportamento:** o exe solto depende do runtime WebView2 já presente no guest. O instalador NSIS (`tauri build`) cuida do WebView2, mas o bootstrapper precisa de rede durante a instalação: instale antes de rodar `guest-firewall.ps1`. Sem WebView2 o app não abre e não há CDP; isso é resultado do teste.
- **Demonstra:** publicar o exe de `--no-bundle`, rodar `guest-run.ps1` e ler `C:\labwork\run\guest-run.json` (`webview2.registryPv` vazio). Na prática o exe abre uma janela "Claudia RH" em branco mais o diálogo nativo "Could not find the WebView2 Runtime" (`windows = ["Error"]` no JSON).
- **Fonte:** passo 1 do `sandbox\app\guest-run.ps1` (registra o estado do WebView2); spec, seção "CDP do app dentro do Sandbox".
- **Situação:** verificado (2026-10-05): o exe solto mostra o diálogo acima; o instalador `/S` com rede levou ~46 s, instalou o WebView2 e o app em `%LOCALAPPDATA%\claudia-rh\`, e não abre o app ao terminar.

## 7. WebView2 só aceita CDP no `127.0.0.1` do guest

- **Comportamento:** com `--remote-debugging-port`, o DevTools do WebView2 escuta só no loopback do guest; o host não alcança direto pelo IP do guest. `cdp-guest.mjs` cria, como System, `netsh interface portproxy add v4tov4 listenaddress=0.0.0.0 listenport=<porta-host> connectaddress=127.0.0.1 connectport=<porta>` e uma regra de firewall de entrada para `<porta-host>`.
- **Demonstra:** com o app aberto com CDP no guest, `curl http://<ip do guest>:9333/json/version` (ip de `wsb ip --id <id>`) não responde; depois de `node .claude/skills/claudia-teste-sandbox/scripts/cdp-guest.mjs --id <id>`, `curl http://<ip do guest>:19333/json/version` responde.
- **Fonte:** spec do repositório, seção "CDP do app dentro do Sandbox".
- **Situação:** portproxy e regra de firewall verificados como funcionais (2026-10-05), mas CDP não respondeu em nenhuma tentativa (release e debug). Não há resposta HTTP em `http://127.0.0.1:9333/json/version` no guest, com ou sem `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` e com ou sem política por app.

## 8. `wsb exec` sempre sai com 0 no host

- **Comportamento:** o código de saída do `wsb exec` é 0 mesmo quando o processo no guest falha; o código do guest só aparece como linha localizada no stdout (pt-BR: "Processo encerrado com o código: 3"). `erroDeExec` em `scripts/comum.mjs` lê a última linha desse formato (tolerante a idioma) e trata código não zero como erro, com `err.status` igual ao código do guest; sem a linha, não é erro. Status não zero do próprio `wsb` continua erro.
- **Demonstra:** `wsb exec --id <id> -r ExistingLogin -c "cmd /c exit 7"`: exit 0 no host, stdout com "... código: 7".
- **Situação:** verificado (2026-10-05, pt-BR; também com `powershell -EncodedCommand` e `exit 3`). Outros idiomas: o parser aceita "code", mas o texto exato não foi visto ao vivo.

## 9. CDP no guest não funciona

- **Hipótese refutada (2026-10-05):** "o WebView2 reaproveita um navegador já em execução e ignora argumentos novos" foi testada com release e debug: todos os `claudia-rh.exe` e `msedgewebview2.exe` encerrados antes de iniciar com `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`; o `msedgewebview2.exe` principal continuou sem `--remote-debugging-port` (só traz os argumentos que o app passa pela API), sem `DevToolsActivePort` e com `/json/version` recusando conexão. Causa desconhecida; o caminho restante é um opt-in no app para abrir a porta em builds de teste (spec própria).
- **Comportamento:** a porta de depuração remota do WebView2 não responde, nem com build release nem com build de debug. Com o app rodando no guest (debug ou release), o processo do `msedgewebview2.exe` não exibe `--remote-debugging-port` na linha de comando mesmo com `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333` definida no shell que inicia o app (só o ambiente desse shell foi confirmado; o ambiente do `claudia-rh.exe` não foi lido). Política por app (`HKCU\Software\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments`) também é ignorada. O `portproxy` do `cdp-guest.mjs` e a regra de firewall funcionam (confirmado ao vivo), mas não há resposta HTTP. A causa não foi isolada (possível diferença: runtime WebView2 do guest vs. host, usuário `WDAGUtilityAccount`).
- **Demonstra:** `node .claude/skills/claudia-teste-sandbox/scripts/cdp-guest.mjs --iniciar --exe-path <exe no guest>` (debug ou release): exit 1 em ~90 s, "CDP did not answer ... within 90s".
- **Situação:** falha verificada com ambos os builds (2026-10-05), inclusive com o guest sem instâncias anteriores. Use UIA (item 11) e os degraus 2 e 3 da escada de prints (`claudia-teste-e2e`) em vez de CDP.

## 10. Espaço livre no C: do host

- **Comportamento:** o disco virtual do Windows Sandbox fica no C: do host, mesmo quando o repositório (e a pasta de trabalho `.sandbox-work/`) está em outro disco. Com pouco espaço livre no C:, a instalação silenciosa do app (`/S`) pode esgotar o disco. Confira o espaço livre antes de `test-up` e mantenha uma folga de vários GB. Derrube o Sandbox entre etapas pesadas para liberar espaço.
- **Demonstra:** a conferência do espaço, não a lentidão: rode `Get-PSDrive C` no host (coluna `Free`) antes de `test-up.ps1` e confirme a folga.
- **Situação:** verificado ao vivo (2026-10-05): com a pasta de trabalho em outro disco, o C: chegou a 0 bytes durante a instalação silenciosa.

## 11. UIA funciona no guest (substitui o CDP)

- **Comportamento:** com o app instalado e perfil com rede, `winapp ui inspect|search|invoke --on sandbox -a claudia-rh` lê o conteúdo web do WebView2 e navega. O conteúdo web só aparece com profundidade alta (`-d 30`); com a profundidade padrão, mesmo com `-i`, só a barra de título nativa. `invoke` clica por id do elemento ou por nome (só navegação; nunca envio, exclusão ou busca); por nome, a verificação foi feita com `-w <hwnd>`, por id funciona sem ele. Sem `-w <hwnd>`, `winapp ui screenshot` captura também uma janela minúscula extra. Limites: sem `invoke` de comandos Tauri e sem JavaScript.
- **Demonstra:** `winapp ui inspect --on sandbox -a claudia-rh -d 30`; `winapp ui search --on sandbox "<texto>" -a claudia-rh`; `winapp ui invoke --on sandbox <id> -a claudia-rh` (por nome: `winapp ui invoke --on sandbox "<nome>" -a claudia-rh -w <hwnd>`).
- **Situação:** verificado ao vivo (2026-10-05) com o app de release instalado e o cenário `mock`. A origem da página é `http://tauri.localhost/` (casa com `PAGINA_APP` em `regras.mjs`).

## 12. Cenário `copia` verificado ao vivo

- **Comportamento:** a cópia segue a ordem origem somente leitura, automação desligada, firewall antes da primeira abertura. A cópia não segue nem reproduz links: o único aceito (pulado, não copiado) é `diagnostics\latest` como link de arquivo, criado pelo Claude Code, com destino lido sem seguir o link e exatamente `claude-startup.log` ou `<origem>\diagnostics\claude-startup.log` (outro destino, ou destino ilegível, recusa), e o `claude-startup.log` real é copiado normalmente; qualquer outro link na origem recusa a cópia (`test-up -DadosReais` e `cenario-copia.mjs`). `*.db-wal` e `*.db-shm` não aparecem na cópia (consolidados no banco).
- **Situação:** verificado ao vivo (2026-10-05): origem idêntica antes e depois (caminho, tamanho e data), `disparo.ativo` e `skip_permissions` falsos no guest antes e depois de abrir o app, contagens do banco no guest iguais às da cópia no host.

## 13. Disco `Z:` não fica pronto no guest

- **Comportamento:** o `disco-z.ps1` cria o VHD e o `diskpart` cria a partição e atribui a letra `Z`, mas o volume fica RAW (sem formatação). No guest, `Get-DiskImage` vê a imagem anexada (`Number: 1`), porém `Get-Disk` volta vazio e `Get-Partition -DriveLetter Z` volta nada, mesmo como System. Por isso `Get-LabZFormatRefusal` recusa formatar e o `disco-z.json` fica com `ok: false`, `erro: "nothing formatted: disk number unknown"` (4 a 5 s). A recusa é segura (nada é formatado), mas sem `Z:` os testes de Sandbox (`testes-no-guest.mjs`, que sai com `exit /b 97` sem `Z:\claudia-teste-sandbox`) e os cenários (`lstat 'Z:\claudia-teste-sandbox\<cenario>'` com `UNKNOWN: unknown error`) não rodam, e o `test-up` sai com 1.
- **Demonstra:** `scripts\test-up.ps1 -Ambiente app-winapp -Project <projeto>` e depois ler `shared\out\disco-z.json`; no guest, como System, `diskpart` com `list volume` mostra `Z  RAW  Partição  19 GB`.
- **Situação:** verificado (2026-10-07) em 4 subidas seguidas (`cache-msvc` duas vezes, `app-winapp`, `app-winapp -DadosReais`) e de novo em 2026-10-08 (3 subidas `cache-msvc`, scripts do checkout principal). **Corrigido em 2026-10-08** (abaixo).
- **Causa (verificada ao vivo, 2026-10-08):** no Windows Sandbox o provedor de Storage não lista nada: `Get-Disk`, `Get-Partition`, `Get-Volume` e as classes `MSFT_Disk`/`MSFT_Partition`/`MSFT_Volume` voltam vazios, inclusive para o disco 0, mesmo como System e depois de `Update-HostStorageCache`; `Get-DiskImage ... | Get-Disk` falha com `No MSFT_Disk objects found with property 'Number' equal to '1'`. Funcionam: `Get-DiskImage -ImagePath C:\labdata\z.vhdx` (`Attached: True`, `Number: 1`, `DevicePath: \\.\PhysicalDrive1`) e o CIMv2 (`Win32_LogicalDisk` `Z:` → `Win32_DiskPartition` com `DiskIndex: 1` → `Win32_DiskDrive` `\\.\PHYSICALDRIVE1`); `diskpart` com `select vdisk file=...` + `detail vdisk` mostra `Disco nº associado: 1` e o volume `Z RAW`.
- **Correção:** o `disco-z.ps1` lê os fatos de `Get-DiskImage` e do CIMv2 e decide em `guest-z.lib.ps1` (puro): `Get-LabZOwnerRefusal` (imagem anexada; número do disco conhecido e diferente de 0; `Z:` é exatamente uma partição, no mesmo número de disco e no mesmo `DevicePath` da imagem), `Get-LabZFormatRefusal` (dono, exatamente 1 partição no disco da imagem e `Z:` sem sistema de arquivos: nunca formata de novo) e `Get-LabZReadyRefusal` (dono e NTFS, depois de formatar e para reaproveitar). Formata pelo `diskpart` com o disco escolhido **pelo arquivo** (`select vdisk file="C:\labdata\z.vhdx"`, `select partition 1`, `format fs=ntfs quick label=LABZ`, em `Get-LabZFormatDiskPart`), nunca por número. `Format-Volume` não serve no Sandbox (usa o mesmo provedor vazio). O `disco-z.json` ganhou `disco` e `fs`.
- **Demonstra (2026-10-08, `cache-msvc`):** no guest, como System, a imagem RAW deixada pela versão antiga foi desanexada pelo próprio arquivo (`select vdisk file=...`, `detach vdisk`) e só esse arquivo foi apagado; o `disco-z.ps1` corrigido (rodado da worktree montada em `C:\labproject`) saiu com 0 e `{"ok": true, "disco": 1, "fs": "NTFS"}` em 5,7 s; a segunda execução (reaproveita) saiu com 0 em 1,9 s. `Win32_LogicalDisk` `Z:` = NTFS, `LABZ`, 21.472.735.232 bytes; `C:` continuou NTFS; escrita e leitura em `Z:\tmp\w.txt` ok; Node 22.23.3 (extraído do cache dentro do guest): `fs.realpathSync.native` de `Z:\claudia-teste-sandbox\probe-rp` devolveu `Z:\claudia-teste-sandbox\probe-rp` (e `Z:\tmp`), sem trocar de letra. Testes puros: `sandbox-lab/scripts/test/guest-z.test.ps1` (pwsh e Windows PowerShell, exit 0), 8 mutações com `FAILED` nomeado.
- **Não verificado:** a subida completa pelo `test-up` com a correção (o laboratório usa os scripts do checkout principal, que fica com a versão antiga até a integração).

## 14. Layout offline do MSVC falha no guest

- **Comportamento:** `layout-msvc.ps1` (ambiente `cache-msvc`) termina em 17 a 19 s com `erro: "vs_buildtools --layout exit -532462766"` (`0xE0434352`, exceção .NET). O log de eventos do guest mostra `vs_layout.exe` encerrado por `System.IO.IOException` em `System.Console.WriteLine` (`Microsoft.VisualStudio.Setup.Layout.ConsoleHelper.WriteMessage`): o processo aberto pelo `Start-Process` sob `wsb exec` não tem console válido. Acontece com `-r System` e com `-r ExistingLogin`. O download do instalador em si funcionou (`dd_bootstrapper_*.log`).
- **Consequência:** sem `shared\cache\msvc-17.14.41`, o `test-up -Ambiente testes` recusa antes de subir e antes de copiar a credencial (`cache incomplete in shared\cache (msvc-17.14.41) ... No Sandbox was started.`). Por isso a instalação no guest `testes`, o `cargo test --lib`, o `claude -p` com a conta e o risco da renovação do token (A4) continuam **não verificados**.
- **Situação:** verificado (2026-10-07), duas tentativas em guests novos. **Corrigido em 2026-10-08** (abaixo).
- **Causa (verificada ao vivo, 2026-10-08):** o `vs_buildtools --layout` inicia o `setup.exe`, que inicia o `vs_layout.exe` com um console que não funciona. Não resolveram (mesmo erro em 15 a 18 s, `LastTaskResult 3762504530` = `0xE0434352` nas tarefas): redirecionar a saída do bootstrapper (`Start-Process -RedirectStandardOutput/-RedirectStandardError`: só as linhas `Preparing:` do próprio bootstrapper chegam ao arquivo), tarefa agendada na sessão interativa do `WDAGUtilityAccount` (com e sem `--quiet`) e `wsb exec -r System` sem `--quiet`.
- **O que funciona:** o `vs_layout.exe` chamado **direto**, com os mesmos argumentos que o bootstrapper passa e stdout/stderr em arquivos: baixa e confere tudo (`Completed 'Layout' operation`) e só no fim cai num finalizador (`System.Console.ControlCHooker.Unhook()`, `IOException: Parâmetro incorreto`); por isso o código de saída continua `-532462766`.
- **Correção (`layout-msvc.ps1`):** etapa 1, o bootstrapper roda e o script **copia** (sem apagar nada) para `C:\labdata\vs-boot` o que ele extrai e baixa na pasta temporária (`vs_layout.exe` com os arquivos do instalador, `vs_setup_bootstrapper.json`, `vs_installer.opc`), porque o bootstrapper remove as pastas temporárias ao falhar; etapa 2, `vs_layout.exe layout ...` direto, com a saída em `C:\lab\out\layout-msvc.stdout.txt` e `.stderr.txt`. O veredito é puro (`Get-LabVsLayoutRefusal`, em `guest-install.lib.ps1`): exige a linha `Completed 'Layout' operation`; com código diferente de 0, aceita uma única exceção, e só a do finalizador `ControlCHooker.Unhook`/`Finalize`. O `layout-msvc.json` ganhou `bootstrapperExit` e `vsLayoutExit`.
- **Demonstra (2026-10-08):** guest novo `cache-msvc`, `wsb exec -r System` com o `layout-msvc.ps1` corrigido → `layout-msvc.json` com `ok: true`, `id: f73441c9`, 669 arquivos, 2.460.064.127 bytes (~2,3 GiB), 68,7 s no total, `bootstrapperExit` e `vsLayoutExit` = `-532462766`. No PC, depois do `test-down`: `cache-up.ps1 -ImportarMsvc f73441c9` → exit 0 em 11 s (`shared\cache\msvc-17.14.41\layout` com 669 arquivos e os mesmos bytes, mais `layout.sha256`); a segunda execução → exit 1, `msvc-17.14.41 is already in the cache: nothing imported`. Testes puros: `guest-install.test.ps1` (pwsh e Windows PowerShell, exit 0), 4 mutações do veredito com `FAILED` nomeado.
- **Não verificado:** se instalar a partir desse layout no `testes` (`instalar.ps1`) esbarra no mesmo problema de console (o instalador também passa pelo `setup.exe`); a robustez da cópia da etapa 1 em máquina mais lenta (a janela entre extrair e falhar foi de vários segundos nas 2 vezes em que rodou); o layout depende da versão do instalador que a Microsoft serve no dia (`vs_installer.opc` não é fixado).

## 15. Rodada de 2026-10-07 (Tarefa 12 do plano do ambiente declarativo): o que funcionou

- **`cache-up.ps1`:** `-SoPlano` exit 0 com 5 itens `baixar`; a execução baixou bootstrapper do MSVC, Rust 1.95.0, Node 22.23.3 e Claude Code 2.1.285 com SHA-256 conferido em 84 s, exit 0, sem nenhum processo do Visual Studio no PC. Segunda execução: plano só com `ok` (o `msvc` segue `baixar` porque o layout não existe) e listagem do cache igual à anterior, exceto o novo `cache-up-<data>.json`. `-ImportarMsvc` com id sem layout recusa com `has no layout.sha256`, exit 1, sem gravar.
- **LogonCommand elevado:** `status.json` com `elevated: true` em todas as subidas (fecha esse "não verificado"; o `disco-z.ps1` e o `instalar.ps1` rodam no próprio bootstrap).
- **`app-winapp`:** Node 22.23.3 instalado do cache em ~60 s (`instalar.json` `ok: true`, `versaoVista: v22.23.3`). `guest-firewall.ps1` como System: exit 0 e `firewall.txt` com `Domain/Private/Public enabled=True`, `regra lab-block-internet-out: 1`, `INTERNET: BLOQUEADA (...)` e `storage.googleapis.com: BLOQUEADO`.
- **Publicar só o app:** `publicar.mjs --app <instalador>` exit 0; `.sandbox-work\deploy` só com `app\<instalador>` e o marcador.
- **Recusas do `copia`:** sem `--autorizado`, exit 2 no host antes de chegar ao guest; com `--autorizado` e sem `-DadosReais`, `cenario.json` com `exit: 2` e a mensagem do `-DadosReais`. O `cenario-no-guest.mjs` sai com 1 nesse caso, porque o `wsb exec` relata "código: 1" e o despachante para antes de ler o `cenario.json`. `guest-run.ps1 -Cenario` com pasta ausente grava `errors: ["cenario: scenario folder not found"]` e não copia dados.
- **`-DadosReais`:** mount gerado com `ReadOnly=true` em `C:\labdadosreais` e `guest.config.json` com `dadosReais: true`. No guest, `File.Open(..., CreateNew)` dentro do mount recusou com `Access ... is denied`, e o guest enxergou os mesmos 253 itens do PC. No PC, a listagem da pasta de dados do app (caminho relativo, tamanho, data, atributos) ficou idêntica antes e depois (0 diferenças), sem arquivo de sonda. A cópia em si (`cenario-copia.mjs`) não rodou por causa do item 13.

## Evals de disparo

(Não rodar `rodar-evals.mjs` até a Tarefa 0b: CLA-56.) Medidos com `node .claude/skills/claudia-teste-sandbox/evals/rodar-evals.mjs sonnet` sobre `evals/evals.json` (9 "deve disparar", 6 "não deve"). Critério: a **primeira** chamada de ferramenta do modelo é `Skill(claudia-teste-sandbox)`. Uma execução `INVALIDO` (limite de sessão, falha do CLI) não conta, nem como acerto nem como falha.

Medido com o nome anterior da skill (`claudia-sandbox`) e a primeira versão da descrição (894 caracteres); a medição após o rename está abaixo.

| Rodada (2026-10-05) | Resultado | Inválidas |
|---|---|---|
| 1 | 15/15 | 0 |
| 2 | 15/15 | 0 |

Nas duas rodadas, os 9 casos "deve" chamaram `Skill(claudia-teste-sandbox)`. Nos "não deve": olhar a tela do app aberto, print do Perfil e "olha o app que tá rodando no Sandbox" foram para `Skill(claudia-teste-e2e)`; o registro no Linear para `Skill(linear-claudia)`; `cargo test` e o ajuste de CSS começaram por ferramenta comum (`PowerShell`, `Grep`), sem skill.

Limites da medição: um modelo só, duas rodadas, prompts curtos. A descrição não precisou de ajuste de conteúdo depois da primeira versão (894 caracteres); o rename a levou a 900. Ao mudar a descrição desta skill ou a da `claudia-teste-e2e`, rode de novo os evals das duas: os casos de fronteira ("olha o app no Sandbox" e "monta um Windows limpo") dependem das duas descrições juntas.

### Após o rename

Medições com a skill renomeada para `claudia-teste-sandbox`. O rename levou a descrição de 894 para 900 caracteres, então o 15/15 abaixo (Sonnet, 2 rodadas) mede a versão de 900 caracteres:

| Rodada | Resultado | Inválidas |
|---|---|---|
| 1 | 15/15 | 0 |
| 2 | 15/15 | 0 |

Nos dois casos "deve" relacionados ao Sandbox: ambos chamaram `Skill(claudia-teste-sandbox)`. Sem regressão nos 6 casos "não deve".
