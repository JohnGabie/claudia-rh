# Motor do browser da Claudia (CDP direto): plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. O método de execução já foi escolhido pelo usuário: ver "Protocolo de execução".

**Goal:** O app passa a ser dono de um Chrome próprio (Chrome for Testing, perfil persistente, tema laranja, janela sempre visível) e a expor o controle dele como tools no servidor MCP, para qualquer IA dirigir o browser sem a extensão Claude in Chrome. Inclui o Activity log do modo desenvolvedor e a remoção do `--chrome`.

**Architecture:** O backend Rust do app (processo principal) baixa, lança e vigia o Chrome. O processo MCP (`claudia-rh --mcp-serve`, um por sessão de IA) é só cliente: lê a porta de debug de um `state.json`, conecta por WebSocket (cliente CDP próprio, ~12 comandos) e executa as tools `browser_*`. Uma guarda no código impede envio acidental de candidatura. Um banco separado (`activity-log.db`) grava, só no modo desenvolvedor, ações, eventos, falhas e navegação, com prints.

**Tech Stack:** Rust (Tauri 2, tokio, rusqlite, rmcp 2.2), `reqwest` (rustls), `sha2`, `tokio-tungstenite`, `zip`; React + TypeScript para a interface; Node 24 só para scripts de validação.

**Spec:** `docs/superpowers/specs/2026-10-02-browser-cdp-motor-design.md` (motor) e, só para contexto de fronteira, `2026-10-02-contas-do-browser-design.md` (ciclo 2, **fora deste plano**). Pesquisa: `docs/pesquisa/2026-09-28-DECISAO-browser.md`. Implementação de referência em JS: `prototipo-playwright/cdp/`.

## Global Constraints

Cada tarefa herda isto. Valores copiados da spec e do `CLAUDE.md`.

- **Nenhum passo de instalação novo para o usuário.** O que o app não puder empacotar, baixar ou configurar sozinho, não entra.
- Comentários de código em **inglês**; mensagens de commit em **inglês**; identificadores novos em **inglês**; copy da UI em **pt-BR** (e a chave equivalente em `en.ts`).
- **Nunca** mudar o `identifier` Tauri `io.github.johngabie.claudia-rh`.
- **Não editar** `src-tauri/src/prompt_sistema_runtime.md` sem pedido explícito do usuário. Este plano só **propõe** a mudança (Tarefa 5.1).
- Browser = **Chrome for Testing**, versão e SHA-256 **fixados** em `src-tauri/browser-pin.json` (valores definidos na Tarefa 0.1, nunca no código).
- Pasta do browser: `<dados-locais>/browser/`, com `<dados-locais>` = `%LOCALAPPDATA%\io.github.johngabie.claudia-rh` (`app_local_data_dir`). Nunca em `%APPDATA%` (Roaming) nem na pasta de instalação.
- Janela **sempre visível** em execução. `--headless` só em teste (`LaunchOptions::headless_for_tests`, `#[cfg(test)]` ou testes de integração).
- **Nenhuma senha passa pelo app.** Campo de senha: texto e recorte "antes" **nunca** gravados no Activity log.
- Activity log: **desligado por padrão**; banco separado `<dados-locais>/dev/activity-log.db` + pasta `capturas/`; **nenhuma tool MCP lê isso**; retenção 30 dias e teto de 1 GB; falha ao gravar nunca atrasa nem derruba a ação do agente.
- Guarda de envio: o agente decide sozinho **dentro** da guarda (vaga em `candidatando`, orçamento do dia, site permitido). Sem confirmação humana por rodada.
- Trailer de commit para os agentes Haiku: `Co-Authored-By: Claude Haiku 4.5 <noreply@anthropic.com>`.
- Branch: `feat/browser-cdp-mcp`. **Nunca** commit em `main` ou `dev`. **Não** dar push sem pedido do usuário.

## Ajustes em relação à spec

Decididos ao ler o código; a spec é atualizada junto com este plano.

1. **`--browser-state <arquivo>` em vez de `--browser-port`.** A porta muda quando o Chrome religa. O processo MCP lê o `state.json` a cada (re)conexão.
2. **Nomes das tools em inglês** (convenção do repo: identificadores novos em inglês): `browser_open`, `browser_read`, `browser_click`, `browser_type`, `browser_key`, `browser_crop`, `browser_click_at`. Parâmetro de guarda: `confirm_submit`. As descrições continuam em pt-BR, como as tools existentes.
3. **O browser é preparado em segundo plano na abertura do app** (baixar, tema, lançar), não no clique de "iniciar sessão". Na primeira abertura é o "primeira vez que precisa". Assim a primeira sessão não espera 150 MB.
4. **Navegação do usuário só é gravada durante sessões de IA.** O rastreador de navegação roda no processo MCP (onde está a conexão CDP). Fora de sessão, nada é gravado. Limite a documentar na UI.
5. **Sessão e vaga do registro** vêm do banco principal (`sessoes` sem `terminada_em`; vaga em `candidatando`), sem mudar a assinatura de `write_mcp_config`.

## Review Focus

Entradas que a spec implica mas nenhuma tarefa óbvia exercita; cada uma tem teste na tarefa dona.

1. **Chrome já rodando com o perfil** (segunda instância do app, ou Chrome que sobreviveu a um crash): nunca lançar um segundo Chrome no mesmo `user-data-dir`; reaproveitar via `state.json` + sonda. → Tarefa 1.7.
2. **`state.json` velho**: porta reaproveitada por outro processo (o Chrome pessoal do usuário usa a 9222). A sonda tem de conferir que a versão é a fixada; senão, relança. → Tarefa 1.7.
3. **Ref obsoleta**: `browser_click("e12")` depois de a página mudar não pode clicar em outro elemento; erro claro "ref obsoleta, chame browser_read". → Tarefa 2.2.
4. **Burlar a guarda pelo teclado**: `browser_key("Enter")` com o foco no botão "Enviar candidatura" tem de passar pela guarda, como o clique. → Tarefa 2.2.
5. **Download interrompido ou `chrome.exe` posto em quarentena pelo antivírus**: nunca deixar instalação pela metade; erro com mensagem própria e recomeço do zero. → Tarefa 1.2.

## Protocolo de execução (escolhido pelo usuário)

- **Quem despacha:** só a conversa principal. Subagentes não despacham subagentes.
- **Implementadores:** subagente `model: "haiku"`, uma tarefa por vez, recebendo o texto da tarefa **e** as seções "Global Constraints" e "Ajustes" deste plano. Instrução fixa: se o código do plano não compilar, corrigir o mínimo, **registrar a diferença** no relatório e seguir; nunca expandir o escopo.
- **Revisores:** subagente `model: "opus"`, **depois de cada tarefa** (portão): confere contra a spec, procura teste que passa sem provar nada, placeholders, escopo extra, violação das Global Constraints. Reprovou → o Haiku corrige → o Opus reconfere.
- **Revisão de fase:** ao fim de cada fase, um Opus revisa o diff inteiro da fase e roda `cargo test`.
- **Paralelismo:** só entre tarefas de arquivos disjuntos, marcadas "pode rodar em paralelo". O resto é sequencial.
- **Pontos com o usuário** (o agente para e pergunta): resultado da Fase 0 (portão), qualquer teste que desinstale o app, qualquer envio real de candidatura, e a decisão do desinstalador (Tarefa 4.2).
- **Ordem das fases:** 0 (validar limitações) → 1 → 2 → 3 → 4 → 5 → 6 (testes e validação). Fase 5 (remover `--chrome`) só começa com a Fase 6.2 da rodada A verde.

## Mapa de arquivos

```
src-tauri/browser-pin.json            versão, sha256, base_url do Chrome for Testing
src-tauri/browser-theme.json          chaves do tema laranja (definidas na Tarefa 0.3)
src-tauri/submit-patterns.txt         nomes de botão que contam como "enviar candidatura"
src-tauri/src/browser/mod.rs          caminhos e re-exports
src-tauri/src/browser/pin.rs          lê browser-pin.json
src-tauri/src/browser/install.rs      baixar, conferir hash, extrair, renomear
src-tauri/src/browser/cdp.rs          cliente CDP (WebSocket) + alvos + aba visível
src-tauri/src/browser/refs.rs         árvore de acessibilidade -> refs e<N>
src-tauri/src/browser/guard.rs        guarda de envio
src-tauri/src/browser/theme.rs        tema laranja no perfil
src-tauri/src/browser/manager.rs      lançar, vigiar, religar, state.json
src-tauri/src/browser/nav.rs          NavTracker (histórico de navegação)
src-tauri/src/browser/service.rs      preparo em segundo plano + eventos para a UI
src-tauri/src/browser/cleanup.rs      limpeza do desinstalador
src-tauri/src/activity_log/mod.rs     banco, gravar, listar, limpar, modo dev
src-tauri/src/activity_log/shots.rs   arquivos de captura e retenção
src-tauri/src/activity_log/sink.rs    ActionSink que grava no banco
src-tauri/src/mcp/tools/browser.rs    BrowserTools (as ações) + DbGuardContext
src-tauri/src/mcp/server.rs           tools browser_* no rmcp (modificar)
src-tauri/src/mcp/mod.rs              flags --browser-state e --local-data-dir (modificar)
src-tauri/src/commands/activity.rs    comandos Tauri do Activity log
src-tauri/tests/fixtures/apply-form.html   página local com as armadilhas
src-tauri/tests/browser_live.rs       testes com Chrome de verdade (#[ignore])
src-tauri/tests/mcp-browser-smoke.mjs      cliente MCP por stdio, para a Fase 6
src-tauri/tests/activity-log-validate.mjs  validação do Activity log, para a Fase 6
src/components/ActivityLogView.tsx    lista + detalhe com prints
src/components/Configuracoes.tsx      seção "Modo desenvolvedor" (modificar)
```

---

# FASE 0: validar as limitações

Nenhum código de produto, exceto `browser-pin.json` e `browser-theme.json`. Cada tarefa termina com um registro em `docs/pesquisa/2026-10-02-validacao-browser.md` (criado na 0.1, acrescentado nas outras) no formato **Hipótese / O que foi feito / Resultado / Decisão**. **Falhou = parar e perguntar ao usuário.** Nada da Fase 1 em diante começa antes do portão da Tarefa 0.5.

### Task 0.1: Fonte do download, licença e fixação da versão

**Files:**
- Create: `src-tauri/browser-pin.json`
- Create: `docs/pesquisa/2026-10-02-validacao-browser.md`

**Interfaces:**
- Produces: `browser-pin.json` com exatamente `{"version": "...", "sha256": "...", "base_url": "https://storage.googleapis.com/chrome-for-testing-public"}`; consumido pela Tarefa 1.1.

- [ ] **Step 1: Buscar a versão estável atual**

Run (Git Bash):
```bash
curl -s https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json -o "$TEMP/cft.json"
node -e "const j=require(process.env.TEMP+'/cft.json');const s=j.channels.Stable;console.log(s.version);console.log(s.downloads.chrome.find(d=>d.platform==='win64').url)"
```
Expected: duas linhas, a versão (`154.x.x.x` ou mais nova) e uma URL terminando em `/win64/chrome-win64.zip`.

- [ ] **Step 2: Baixar o zip e calcular o SHA-256**

```bash
URL="<a URL impressa no passo 1>"
curl -L -o "$TEMP/cft.zip" "$URL"
powershell -c "(Get-FileHash $env:TEMP\cft.zip -Algorithm SHA256).Hash.ToLower()"
ls -l "$TEMP/cft.zip"
```
Expected: um hash de 64 caracteres hexadecimais e um zip de ~150 MB. Se o download falhar ou o tamanho for menor que 50 MB, **parar** e reportar.

- [ ] **Step 3: Escrever `src-tauri/browser-pin.json`** com a versão e o hash obtidos:

```json
{
  "version": "<versão do passo 1>",
  "sha256": "<hash do passo 2, minúsculo>",
  "base_url": "https://storage.googleapis.com/chrome-for-testing-public"
}
```
(Substituir os dois valores entre `<>` pelos obtidos; o arquivo final não pode conter `<` nem `>`.)

- [ ] **Step 4: Licença e redistribuição**

Ler (WebFetch ou navegador) os termos do Chrome for Testing (`https://googlechromelabs.github.io/chrome-for-testing/`) e a seção de redistribuição dos Termos de Serviço do Google Chrome. Registrar em 2 a 4 frases, com o trecho citado: (a) baixar da Google na máquina do usuário é permitido? (b) redistribuir dentro do nosso instalador é permitido? O plano só **baixa**, então só (a) precisa ser sim.

- [ ] **Step 5: Escrever o registro**: criar `docs/pesquisa/2026-10-02-validacao-browser.md` com o título, a seção "0.1" no formato Hipótese/O que foi feito/Resultado/Decisão, a versão, o hash, a URL e a conclusão sobre a licença.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/browser-pin.json docs/pesquisa/2026-10-02-validacao-browser.md
git commit -m "docs(browser): pin Chrome for Testing version and sha256, record licence reading"
```

### Task 0.2: A sessão do LinkedIn sobrevive no Chrome for Testing

**Files:**
- Modify: `docs/pesquisa/2026-10-02-validacao-browser.md` (acrescentar "0.2")
- Scratch (não commitar): `%TEMP%/cft/`

**Interfaces:**
- Consumes: o zip de `%TEMP%/cft.zip` da 0.1.

Hipótese: o perfil `prototipo-playwright/perfil-claudia` (criado pelo Chrome 153, logado no LinkedIn) continua autenticado quando aberto pelo Chrome for Testing. **Cuidado:** isto só lê; não clicar em nada, não candidatar.

- [ ] **Step 1: Extrair e copiar o perfil**

```bash
mkdir -p "$TEMP/cft" && cd "$TEMP/cft" && unzip -q -o ../cft.zip
cp -r "<raiz do repo>/prototipo-playwright/perfil-claudia" "$TEMP/cft-profile"
```
(Trabalhar sempre na **cópia**, nunca no perfil original.)

- [ ] **Step 2: Lançar o Chrome for Testing com a cópia**

```bash
"$TEMP/cft/chrome-win64/chrome.exe" --remote-debugging-port=9777 --user-data-dir="$TEMP/cft-profile" --no-first-run &
sleep 6
curl -s http://127.0.0.1:9777/json/version
```
Expected: JSON com `"Browser": "Chrome/<versão da 0.1>"`. Se `Browser` for outra versão, a porta é de outro processo: parar.

- [ ] **Step 3: Conferir que há sessão (só leitura)**

Criar `%TEMP%/cft-check.mjs` (usa o cliente de referência):
```js
import { alvos, ligar, novaAba, sleep } from "<raiz do repo>/prototipo-playwright/cdp/cdp.mjs";
const aba = await novaAba(9777, "https://www.linkedin.com/feed/");
await sleep(6000);
const c = await ligar(aba.webSocketDebuggerUrl);
const r = await c.cdp("Runtime.evaluate", { expression: "location.href + ' | ' + document.title", returnByValue: true });
console.log(r.result.result.value);
c.fechar();
```
Run: `node "$TEMP/cft-check.mjs"`
Expected (sessão viva): uma URL em `linkedin.com/feed` e um título que **não** contém "Entrar" nem "Sign in". Se a URL for `/login`, `/authwall` ou `/checkpoint`, a hipótese **falhou**.

- [ ] **Step 4: Fechar o Chrome** (`taskkill /IM chrome.exe /F` apenas para o processo com `--remote-debugging-port=9777`: usar `powershell -c "Get-CimInstance Win32_Process -Filter \"Name='chrome.exe'\" | ? { $_.CommandLine -match '9777' } | % { Stop-Process -Id $_.ProcessId -Force }"`; **não** matar o Chrome pessoal do usuário).

- [ ] **Step 5: Registrar** a seção "0.2" com a URL e o título vistos e a decisão: passou (seguir) ou falhou (portão: perguntar ao usuário; talvez ele precise fazer login uma vez na janela da Claudia, o que muda a expectativa de "memória das contas").

- [ ] **Step 6: Commit** `git add docs/pesquisa/2026-10-02-validacao-browser.md && git commit -m "docs(browser): validate LinkedIn session persistence in Chrome for Testing"`

### Task 0.3: O tema laranja sem clique do usuário

**Files:**
- Create: `src-tauri/browser-theme.json`
- Modify: `docs/pesquisa/2026-10-02-validacao-browser.md` (acrescentar "0.3")

**Interfaces:**
- Produces: `browser-theme.json` no formato `{"preferences": <objeto JSON a mesclar em Default/Preferences> | null, "policy": <{"key": "...", "values": {...}}> | null}`; consumido pela Tarefa 1.6.

Hipótese: dá para gravar a cor `#D97757` (ARGB `0xFFD97757`, como inteiro com sinal `-2525353`) no `Preferences` do perfil **antes** da primeira abertura e o Chrome a mostra. Não se sabe qual chave o Chrome 154 usa; a tarefa descobre.

- [ ] **Step 1: Perfil novo e limpo**

```bash
rm -rf "$TEMP/cft-theme" && mkdir -p "$TEMP/cft-theme/Default"
```

- [ ] **Step 2: Descobrir a chave.** Abrir o Chrome for Testing com um perfil novo (`--user-data-dir="$TEMP/cft-theme-ref"`), escolher manualmente uma cor em "Personalizar o Chrome > Cor do tema", fechar, e ler `Default/Preferences`:
```bash
node -e "const p=JSON.parse(require('fs').readFileSync(process.env.TEMP+'/cft-theme-ref/Default/Preferences','utf8'));console.log(JSON.stringify({autogenerated:p.autogenerated,browser_theme:p.browser&&p.browser.theme,extensions_theme:p.extensions&&p.extensions.theme},null,1))"
```
Anotar as chaves que mudaram. Esta é a única etapa manual: pedir ao usuário para escolher a cor se o agente não conseguir fazê-lo.

- [ ] **Step 3: Escrever o patch no perfil limpo e abrir**

Com as chaves descobertas, criar `$TEMP/cft-theme/Default/Preferences` contendo só o patch com a cor laranja (inteiro `-2525353`), abrir o Chrome for Testing com `--user-data-dir="$TEMP/cft-theme"`, e **tirar um screenshot do sistema** da janela:
```powershell
Add-Type -AssemblyName System.Windows.Forms,System.Drawing
$b=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp=New-Object System.Drawing.Bitmap $b.Width,$b.Height
[System.Drawing.Graphics]::FromImage($bmp).CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size)
$bmp.Save("$env:TEMP\cft-theme.png")
```
Ler a imagem (Read) e conferir se a barra superior do Chrome está laranja. **Não** é aceitável concluir só pelo arquivo.

- [ ] **Step 4: Se o `Preferences` não funcionar**, testar a política `BrowserThemeColor`. **Condição de aceite:** só vale se a chave de registro for **exclusiva do Chrome for Testing** (ex.: sob `Google\Chrome for Testing`). Uma política sob `Software\Policies\Google\Chrome` também alteraria o Chrome pessoal do usuário e **viola** "não tocar no Chrome do usuário": nesse caso, registrar como falha, **sem** usar.

- [ ] **Step 5: Escrever `src-tauri/browser-theme.json`** com o que funcionou, por exemplo:
```json
{ "preferences": { "<chaves descobertas no passo 2>": -2525353 }, "policy": null }
```
Se nada funcionou: `{ "preferences": null, "policy": null }` e a decisão registrada é "tema não aplicável sem clique; alternativa: ícone/aviso" (portão: perguntar ao usuário).

- [ ] **Step 6: Registrar** a seção "0.3" (com o caminho do screenshot e o que a imagem mostrou) e commitar `browser-theme.json` e o doc.

### Task 0.4: O que o desinstalador do Tauri já apaga

**Files:**
- Modify: `docs/pesquisa/2026-10-02-validacao-browser.md` (acrescentar "0.4")

Só leitura. **Nunca** rodar o instalador ou desinstalador nesta máquina: ele apagaria os dados reais do usuário.

- [ ] **Step 1: Achar o template NSIS do Tauri**
```bash
ls ~/.cargo/registry/src/*/ | grep -i tauri-bundler
find ~/.cargo/registry/src -path "*tauri-bundler*" -name "installer.nsi" | head -3
```
- [ ] **Step 2: Ler** o `installer.nsi` e responder por escrito: (a) existe a caixa "Delete the application data" na desinstalação? (b) que pastas ela apaga (`$APPDATA\...`, `$LOCALAPPDATA\...`)? (c) onde ficam os hooks (`NSIS_HOOK_PREUNINSTALL`, `NSIS_HOOK_POSTUNINSTALL`) e como se configura `installerHooks` em `tauri.conf.json`? (d) dá para ler `$DeleteAppDataCheckboxState` no hook?
- [ ] **Step 3: Registrar** a seção "0.4" com as respostas e a conclusão: a caixa nativa já cobre a opção 2 da spec ("remover também os meus dados") ou não? A segunda opção (só o browser) exige uma página própria (nsDialogs)? Citar as linhas lidas.
- [ ] **Step 4: Commit** `git commit -am "docs(browser): record what the Tauri NSIS uninstaller already deletes"`

### Task 0.5: Portão da Fase 0

- [ ] **Step 1: Revisão Opus** de `docs/pesquisa/2026-10-02-validacao-browser.md`: cada hipótese tem evidência (saída de comando, caminho de imagem), nenhuma conclusão sem prova, `browser-pin.json` sem `<`/`>`.
- [ ] **Step 2: Resumo ao usuário** com as quatro hipóteses (passou/falhou) e **esperar o "pode seguir"**. Se 0.2 ou 0.3 falharam, as respostas do usuário mudam a spec antes da Fase 1.

---

# FASE 1: fundações (sem browser, tudo testável sozinho)

Rodar `cargo test` em `src-tauri/` ao fim de cada tarefa. Antes da 1.1, registrar o **baseline do clippy**: `cd src-tauri && cargo clippy 2>&1 | grep -c warning` (guardar o número em `docs/pesquisa/2026-10-02-validacao-browser.md`, seção "baseline").

### Task 1.1: Dependências, módulo e caminhos

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs` (acrescentar `pub mod browser;` junto dos outros `mod`)
- Create: `src-tauri/src/browser/mod.rs`
- Create: `src-tauri/src/browser/pin.rs`

**Interfaces:**
- Produces: `browser::browser_root(local:&Path)->PathBuf`, `browser::profile_dir(local)`, `browser::state_file(local)`, `browser::chrome_exe(local,&str)`, `browser::pin::{Pin, pin()}` com `Pin::zip_url(&self)->String`.

- [ ] **Step 1: Dependências.** Em `[dependencies]` acrescentar:
```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream", "json"] }
sha2 = "0.10"
futures-util = "0.3"
tokio-tungstenite = "0.26"
base64 = "0.22"
```
Run: `cd src-tauri && cargo check` — Expected: compila. Se o `cargo` reclamar de versão, usar a mais próxima que ele indicar e registrar.

- [ ] **Step 2: Testes que falham** em `src-tauri/src/browser/mod.rs`:
```rust
// Browser engine: our own Chrome for Testing, driven over CDP.
pub mod pin;

use std::path::{Path, PathBuf};

/// Folder that holds everything browser-related (binary + profile).
pub fn browser_root(local_data_dir: &Path) -> PathBuf {
    local_data_dir.join("browser")
}

/// Persistent Chrome profile: cookies, i.e. the accounts the user logged into.
pub fn profile_dir(local_data_dir: &Path) -> PathBuf {
    browser_root(local_data_dir).join("profile")
}

/// Where the app tells MCP processes which debug port the browser uses.
pub fn state_file(local_data_dir: &Path) -> PathBuf {
    browser_root(local_data_dir).join("state.json")
}

pub fn chrome_exe(local_data_dir: &Path, version: &str) -> PathBuf {
    browser_root(local_data_dir).join(version).join("chrome-win64").join("chrome.exe")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_live_under_one_browser_folder() {
        let local = Path::new("C:/L/io.github.johngabie.claudia-rh");
        assert_eq!(browser_root(local), local.join("browser"));
        assert_eq!(profile_dir(local), local.join("browser").join("profile"));
        assert_eq!(state_file(local), local.join("browser").join("state.json"));
        assert_eq!(
            chrome_exe(local, "154.0.1.2"),
            local.join("browser").join("154.0.1.2").join("chrome-win64").join("chrome.exe")
        );
    }
}
```
E em `pin.rs`:
```rust
// The pinned Chrome for Testing build. Version and hash live in browser-pin.json
// so a Chrome bump is a data change, not a code change.
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Pin {
    pub version: String,
    pub sha256: String,
    pub base_url: String,
}

impl Pin {
    pub fn zip_url(&self) -> String {
        format!("{}/{}/win64/chrome-win64.zip", self.base_url.trim_end_matches('/'), self.version)
    }
}

pub fn pin() -> Pin {
    serde_json::from_str(include_str!("../../browser-pin.json")).expect("browser-pin.json is valid JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_file_is_well_formed() {
        let p = pin();
        assert_eq!(p.version.split('.').count(), 4, "version looks like 154.0.8037.92");
        assert_eq!(p.sha256.len(), 64);
        assert!(p.sha256.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(p.zip_url().starts_with("https://"));
        assert!(p.zip_url().ends_with("/win64/chrome-win64.zip"));
    }
}
```
- [ ] **Step 3:** Em `lib.rs`, acrescentar `pub mod browser;` perto de `pub mod mcp;`.
- [ ] **Step 4: Run** `cd src-tauri && cargo test browser::` — Expected: 2 testes passam.
- [ ] **Step 5: Commit** `git add src-tauri && git commit -m "feat(browser): module skeleton, paths and the pinned Chrome build"`

### Task 1.2: Instalação do Chrome (baixar, conferir, extrair)

**Files:**
- Create: `src-tauri/src/browser/install.rs`
- Modify: `src-tauri/src/browser/mod.rs` (acrescentar `pub mod install;`)

**Interfaces:**
- Produces: `install::ensure_installed(root:&Path, version:&str, zip_url:&str, expected_sha256:&str, on_progress:&(dyn Fn(Progress)+Send+Sync)) -> Result<PathBuf, InstallError>` (devolve o caminho do `chrome.exe`); `Progress{downloaded:u64,total:Option<u64>}`; `InstallError::{Network(String),HashMismatch{expected:String,actual:String},BadZip(String),Io(String)}` com `Display`.

Cobre o Review Focus 5: nada de instalação pela metade; recomeço do zero.

- [ ] **Step 1: Escrever `install.rs` com os testes primeiro** (a implementação vem no Step 3). Código completo dos testes:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    fn temp_root(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("claudia-install-{tag}-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sha(b: &[u8]) -> String { format!("{:x}", Sha256::digest(b)) }

    fn make_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            for (name, data) in files {
                zw.start_file(*name, opts).unwrap();
                zw.write_all(data).unwrap();
            }
            zw.finish().unwrap();
        }
        buf.into_inner()
    }

    /// Serves `body` with the given status on a random local port. Returns the URL.
    async fn serve(body: Vec<u8>, status: u16) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { break };
                let body = body.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let _ = sock.read(&mut buf).await;
                    let head = format!("HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                    let _ = sock.write_all(head.as_bytes()).await;
                    let _ = sock.write_all(&body).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        format!("http://127.0.0.1:{port}/chrome-win64.zip")
    }

    fn no_progress() -> impl Fn(Progress) + Send + Sync { |_p| {} }

    #[tokio::test]
    async fn installs_from_a_valid_zip_and_reports_progress() {
        let root = temp_root("ok");
        let zip = make_zip(&[("chrome-win64/chrome.exe", b"MZfake")]);
        let url = serve(zip.clone(), 200).await;
        let seen = Arc::new(AtomicU64::new(0));
        let s2 = seen.clone();
        let cb = move |p: Progress| { s2.store(p.downloaded, Ordering::SeqCst); };
        let exe = ensure_installed(&root, "1.2.3.4", &url, &sha(&zip), &cb).await.unwrap();
        assert!(exe.exists());
        assert_eq!(exe, root.join("1.2.3.4").join("chrome-win64").join("chrome.exe"));
        assert_eq!(seen.load(Ordering::SeqCst), zip.len() as u64);
        assert!(!root.join(".tmp-1.2.3.4").exists(), "temp folder is cleaned up");
    }

    #[tokio::test]
    async fn wrong_hash_installs_nothing() {
        let root = temp_root("hash");
        let zip = make_zip(&[("chrome-win64/chrome.exe", b"MZfake")]);
        let url = serve(zip, 200).await;
        let err = ensure_installed(&root, "1.2.3.4", &url, &"0".repeat(64), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::HashMismatch { .. }), "{err}");
        assert!(!root.join("1.2.3.4").exists());
        assert!(!root.join(".tmp-1.2.3.4").exists());
    }

    #[tokio::test]
    async fn zip_without_chrome_exe_is_rejected() {
        let root = temp_root("noexe");
        let zip = make_zip(&[("chrome-win64/readme.txt", b"x")]);
        let url = serve(zip.clone(), 200).await;
        let err = ensure_installed(&root, "1.2.3.4", &url, &sha(&zip), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::BadZip(_)), "{err}");
        assert!(!root.join("1.2.3.4").exists());
    }

    #[tokio::test]
    async fn garbage_that_is_not_a_zip_is_rejected() {
        let root = temp_root("garbage");
        let body = b"not a zip at all".to_vec();
        let url = serve(body.clone(), 200).await;
        let err = ensure_installed(&root, "1.2.3.4", &url, &sha(&body), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::BadZip(_)), "{err}");
    }

    #[tokio::test]
    async fn http_error_is_a_network_error() {
        let root = temp_root("404");
        let url = serve(b"nope".to_vec(), 404).await;
        let err = ensure_installed(&root, "1.2.3.4", &url, &"0".repeat(64), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::Network(_)), "{err}");
    }

    #[tokio::test]
    async fn connection_refused_is_a_network_error() {
        let root = temp_root("refused");
        let err = ensure_installed(&root, "1.2.3.4", "http://127.0.0.1:1/x.zip", &"0".repeat(64), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::Network(_)), "{err}");
    }

    #[tokio::test]
    async fn already_installed_skips_the_network() {
        let root = temp_root("again");
        let exe = root.join("1.2.3.4").join("chrome-win64").join("chrome.exe");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, b"MZ").unwrap();
        let got = ensure_installed(&root, "1.2.3.4", "http://127.0.0.1:1/unreachable.zip", &"0".repeat(64), &no_progress()).await.unwrap();
        assert_eq!(got, exe);
    }

    #[tokio::test]
    async fn unwritable_destination_is_an_io_error() {
        // A *file* where the root folder should be: create_dir_all must fail.
        // Stand-in for "disk full / no permission", which cannot be provoked portably.
        let base = temp_root("io");
        let file = base.join("im-a-file");
        std::fs::write(&file, b"x").unwrap();
        let root = file.join("browser");
        let zip = make_zip(&[("chrome-win64/chrome.exe", b"MZ")]);
        let url = serve(zip.clone(), 200).await;
        let err = ensure_installed(&root, "1.2.3.4", &url, &sha(&zip), &no_progress()).await.unwrap_err();
        assert!(matches!(err, InstallError::Io(_)), "{err}");
    }

    #[tokio::test]
    async fn stale_temp_folder_from_a_crashed_install_is_discarded() {
        let root = temp_root("stale");
        let stale = root.join(".tmp-1.2.3.4");
        std::fs::create_dir_all(&stale).unwrap();
        std::fs::write(stale.join("chrome.zip"), b"half a download").unwrap();
        let zip = make_zip(&[("chrome-win64/chrome.exe", b"MZ")]);
        let url = serve(zip.clone(), 200).await;
        let exe = ensure_installed(&root, "1.2.3.4", &url, &sha(&zip), &no_progress()).await.unwrap();
        assert!(exe.exists());
    }
}
```
- [ ] **Step 2: Run** `cargo test browser::install` — Expected: FAIL (não compila: `ensure_installed` não existe).
- [ ] **Step 3: Implementação** (acima dos testes, no mesmo arquivo):
```rust
// Download, verify, extract and atomically place Chrome for Testing.
// Never leaves a half-installed folder: everything happens in a temp dir and
// is renamed into place only after the hash and the layout check out.
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum InstallError {
    Network(String),
    HashMismatch { expected: String, actual: String },
    BadZip(String),
    Io(String),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "falha de rede ao baixar o browser: {e}"),
            Self::HashMismatch { expected, actual } => {
                write!(f, "o arquivo baixado não confere (esperado {expected}, obtido {actual})")
            }
            Self::BadZip(e) => write!(f, "o arquivo do browser está corrompido ou incompleto: {e}"),
            Self::Io(e) => write!(f, "erro de disco ao instalar o browser: {e}"),
        }
    }
}

impl std::error::Error for InstallError {}

#[derive(Debug, Clone, Copy)]
pub struct Progress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

fn io(e: std::io::Error) -> InstallError {
    InstallError::Io(e.to_string())
}

pub async fn ensure_installed(
    root: &Path,
    version: &str,
    zip_url: &str,
    expected_sha256: &str,
    on_progress: &(dyn Fn(Progress) + Send + Sync),
) -> Result<PathBuf, InstallError> {
    let exe = root.join(version).join("chrome-win64").join("chrome.exe");
    if exe.exists() {
        return Ok(exe);
    }

    let tmp = root.join(format!(".tmp-{version}"));
    let _ = std::fs::remove_dir_all(&tmp); // leftover from a crashed install
    std::fs::create_dir_all(&tmp).map_err(io)?;

    let result = install_into(&tmp, root, version, zip_url, expected_sha256, on_progress).await;
    let _ = std::fs::remove_dir_all(&tmp);
    result.map(|_| exe)
}

async fn install_into(
    tmp: &Path,
    root: &Path,
    version: &str,
    zip_url: &str,
    expected_sha256: &str,
    on_progress: &(dyn Fn(Progress) + Send + Sync),
) -> Result<(), InstallError> {
    let zip_path = tmp.join("chrome.zip");
    let actual = download(zip_url, &zip_path, on_progress).await?;
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        return Err(InstallError::HashMismatch { expected: expected_sha256.to_string(), actual });
    }

    let extract = tmp.join("extract");
    let file = std::fs::File::open(&zip_path).map_err(io)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| InstallError::BadZip(e.to_string()))?;
    archive.extract(&extract).map_err(|e| InstallError::BadZip(e.to_string()))?;
    if !extract.join("chrome-win64").join("chrome.exe").exists() {
        return Err(InstallError::BadZip("chrome-win64/chrome.exe is missing from the archive".into()));
    }

    let dest = root.join(version);
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::rename(&extract, &dest).map_err(io)?;
    Ok(())
}

/// Streams the body to `to`, hashing as it goes. Returns the lowercase SHA-256.
async fn download(
    url: &str,
    to: &Path,
    on_progress: &(dyn Fn(Progress) + Send + Sync),
) -> Result<String, InstallError> {
    use futures_util::StreamExt;
    let resp = reqwest::get(url).await.map_err(|e| InstallError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(InstallError::Network(format!("HTTP {}", resp.status())));
    }
    let total = resp.content_length();
    let mut file = std::fs::File::create(to).map_err(io)?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| InstallError::Network(e.to_string()))?;
        hasher.update(&chunk);
        file.write_all(&chunk).map_err(io)?;
        downloaded += chunk.len() as u64;
        on_progress(Progress { downloaded, total });
    }
    Ok(format!("{:x}", hasher.finalize()))
}
```
- [ ] **Step 4: Em `browser/mod.rs`** acrescentar `pub mod install;`. **Run** `cargo test browser::install` — Expected: 9 testes passam. Se o `unwritable_destination` falhar por o Windows permitir o caminho, registrar e trocar por uma pasta somente leitura criada no teste; não remover o teste.
- [ ] **Step 5: Commit** `git add src-tauri && git commit -m "feat(browser): download, verify and atomically install Chrome for Testing"`

### Task 1.3: Cliente CDP

**Files:**
- Create: `src-tauri/src/browser/cdp.rs`
- Modify: `src-tauri/src/browser/mod.rs` (`pub mod cdp;`)

**Interfaces:**
- Produces: `cdp::CdpClient::{connect(&str)->Result<Self,String>, with_timeout(Duration)->Self, is_closed()->bool, subscribe()->broadcast::Receiver<Value>, call(&str, Value)->Result<Value,String>}`; `cdp::Target{id,kind,title,url,ws_url}`; `cdp::list_targets(port)->Result<Vec<Target>,String>`; `cdp::first_page(port)->Result<Target,String>`; `cdp::bring_to_front(port:u16, target_id:&str, client:&CdpClient)->Result<(),String>`.

- [ ] **Step 1: Testes que falham** (no fim de `cdp.rs`), com um servidor WebSocket falso:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use std::time::Duration;
    use tokio_tungstenite::tungstenite::Message;

    /// Fake CDP endpoint. Methods: "Boom" -> error reply, "Emit" -> event then
    /// reply, "Close" -> drops the socket without replying, "Hang" -> never
    /// replies, anything else -> {"echo": method}.
    async fn fake_server() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((sock, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let ws = tokio_tungstenite::accept_async(sock).await.unwrap();
                    let (mut tx, mut rx) = ws.split();
                    while let Some(Ok(Message::Text(t))) = rx.next().await {
                        let v: Value = serde_json::from_str(t.as_str()).unwrap();
                        let id = v["id"].clone();
                        match v["method"].as_str().unwrap() {
                            "Boom" => { let _ = tx.send(Message::Text(json!({"id": id, "error": {"message": "nope"}}).to_string().into())).await; }
                            "Emit" => {
                                let _ = tx.send(Message::Text(json!({"method": "Test.event", "params": {"n": 1}}).to_string().into())).await;
                                let _ = tx.send(Message::Text(json!({"id": id, "result": {}}).to_string().into())).await;
                            }
                            "Close" => break,
                            "Hang" => {}
                            m => { let _ = tx.send(Message::Text(json!({"id": id, "result": {"echo": m}}).to_string().into())).await; }
                        }
                    }
                });
            }
        });
        format!("ws://127.0.0.1:{port}")
    }

    #[tokio::test]
    async fn call_returns_the_result() {
        let c = CdpClient::connect(&fake_server().await).await.unwrap();
        let r = c.call("Page.navigate", json!({})).await.unwrap();
        assert_eq!(r["echo"], "Page.navigate");
    }

    #[tokio::test]
    async fn concurrent_calls_get_their_own_replies() {
        let c = std::sync::Arc::new(CdpClient::connect(&fake_server().await).await.unwrap());
        let (a, b) = tokio::join!(c.call("A", json!({})), c.call("B", json!({})));
        assert_eq!(a.unwrap()["echo"], "A");
        assert_eq!(b.unwrap()["echo"], "B");
    }

    #[tokio::test]
    async fn error_reply_becomes_err() {
        let c = CdpClient::connect(&fake_server().await).await.unwrap();
        let e = c.call("Boom", json!({})).await.unwrap_err();
        assert!(e.contains("nope"), "{e}");
    }

    #[tokio::test]
    async fn events_are_broadcast() {
        let c = CdpClient::connect(&fake_server().await).await.unwrap();
        let mut rx = c.subscribe();
        c.call("Emit", json!({})).await.unwrap();
        let ev = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
        assert_eq!(ev["method"], "Test.event");
    }

    #[tokio::test]
    async fn closed_connection_fails_the_pending_call_and_is_flagged() {
        let c = CdpClient::connect(&fake_server().await).await.unwrap();
        let e = c.call("Close", json!({})).await.unwrap_err();
        assert!(e.contains("closed"), "{e}");
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(c.is_closed());
        assert!(c.call("Again", json!({})).await.unwrap_err().contains("closed"));
    }

    #[tokio::test]
    async fn a_call_that_never_gets_a_reply_times_out() {
        let c = CdpClient::connect(&fake_server().await).await.unwrap().with_timeout(Duration::from_millis(150));
        let e = c.call("Hang", json!({})).await.unwrap_err();
        assert!(e.contains("timeout"), "{e}");
    }

    #[tokio::test]
    async fn list_targets_parses_the_json_endpoint() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut b = [0u8; 1024];
            let _ = s.read(&mut b).await;
            let body = r#"[{"id":"T1","type":"page","title":"x","url":"about:blank","webSocketDebuggerUrl":"ws://127.0.0.1:1/devtools/page/T1"},{"id":"S1","type":"service_worker","title":"sw","url":"u","webSocketDebuggerUrl":"ws://x"}]"#;
            let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await;
        });
        let t = first_page(port).await.unwrap();
        assert_eq!(t.id, "T1");
        assert_eq!(t.kind, "page");
    }
}
```
- [ ] **Step 2: Run** `cargo test browser::cdp` — Expected: FAIL (não compila).
- [ ] **Step 3: Implementação** (acima dos testes):
```rust
// Minimal CDP client: one WebSocket to one page target. Zero magic: JSON in,
// JSON out, ids matched to oneshot channels, events broadcast.
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>;

pub struct CdpClient {
    tx: mpsc::UnboundedSender<String>,
    pending: Pending,
    next_id: AtomicU64,
    events: broadcast::Sender<Value>,
    closed: Arc<AtomicBool>,
    timeout: Duration,
}

impl CdpClient {
    pub async fn connect(ws_url: &str) -> Result<Self, String> {
        let (ws, _) = connect_async(ws_url).await.map_err(|e| format!("cdp connect: {e}"))?;
        let (mut sink, mut stream) = ws.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let (events, _) = broadcast::channel(512);
        let closed = Arc::new(AtomicBool::new(false));

        tokio::spawn(async move {
            while let Some(m) = rx.recv().await {
                if sink.send(Message::Text(m.into())).await.is_err() {
                    break;
                }
            }
        });

        let (p2, ev2, c2) = (pending.clone(), events.clone(), closed.clone());
        tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                let Message::Text(t) = msg else { continue };
                let Ok(v) = serde_json::from_str::<Value>(t.as_str()) else { continue };
                if let Some(id) = v.get("id").and_then(Value::as_u64) {
                    if let Some(s) = p2.lock().await.remove(&id) {
                        let r = match v.get("error") {
                            Some(e) => Err(e.to_string()),
                            None => Ok(v.get("result").cloned().unwrap_or(Value::Null)),
                        };
                        let _ = s.send(r);
                    }
                } else {
                    let _ = ev2.send(v);
                }
            }
            c2.store(true, Ordering::SeqCst);
            for (_, s) in p2.lock().await.drain() {
                let _ = s.send(Err("connection closed".into()));
            }
        });

        Ok(Self { tx, pending, next_id: AtomicU64::new(1), events, closed, timeout: Duration::from_secs(30) })
    }

    pub fn with_timeout(mut self, d: Duration) -> Self {
        self.timeout = d;
        self
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Value> {
        self.events.subscribe()
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        if self.is_closed() {
            return Err("connection closed".into());
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (s, r) = oneshot::channel();
        self.pending.lock().await.insert(id, s);
        let msg = json!({ "id": id, "method": method, "params": params }).to_string();
        if self.tx.send(msg).is_err() {
            self.pending.lock().await.remove(&id);
            return Err("connection closed".into());
        }
        match tokio::time::timeout(self.timeout, r).await {
            Ok(Ok(res)) => res,
            Ok(Err(_)) => Err("connection closed".into()),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(format!("timeout calling {method}"))
            }
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Target {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(rename = "webSocketDebuggerUrl", default)]
    pub ws_url: String,
}

pub async fn list_targets(port: u16) -> Result<Vec<Target>, String> {
    let r = reqwest::get(format!("http://127.0.0.1:{port}/json/list"))
        .await
        .map_err(|e| format!("browser unreachable on port {port}: {e}"))?;
    r.json::<Vec<Target>>().await.map_err(|e| format!("bad /json/list reply: {e}"))
}

pub async fn first_page(port: u16) -> Result<Target, String> {
    list_targets(port)
        .await?
        .into_iter()
        .find(|t| t.kind == "page" && !t.ws_url.is_empty())
        .ok_or_else(|| "the browser has no open page".to_string())
}

/// A hidden tab silently swallows input events. Activate it and wait until the
/// page reports itself visible (trap 1 from the prototype).
pub async fn bring_to_front(port: u16, target_id: &str, client: &CdpClient) -> Result<(), String> {
    let _ = reqwest::get(format!("http://127.0.0.1:{port}/json/activate/{target_id}")).await;
    for _ in 0..25 {
        let r = client
            .call("Runtime.evaluate", json!({ "expression": "document.visibilityState", "returnByValue": true }))
            .await?;
        if r["result"]["value"] == "visible" {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err("the tab stayed hidden; input would be swallowed".into())
}
```
- [ ] **Step 4:** acrescentar `pub mod cdp;` em `mod.rs`. **Run** `cargo test browser::cdp` — Expected: 7 testes passam.
- [ ] **Step 5: Commit** `git add src-tauri && git commit -m "feat(browser): minimal CDP client with targets and visible-tab guard"`

### Task 1.4: Árvore de acessibilidade em refs (pode rodar em paralelo com 1.5 e 1.6)

**Files:**
- Create: `src-tauri/src/browser/refs.rs`
- Modify: `src-tauri/src/browser/mod.rs` (`pub mod refs;`)

**Interfaces:**
- Produces: `refs::RefNode{reference:String, role:String, name:String, backend_node_id:i64, disabled:bool}`; `refs::build_refs(ax:&Value, scope:Option<&str>)->Result<Vec<RefNode>,String>`; `refs::render(&[RefNode])->String`; `refs::find<'a>(&'a [RefNode], &str)->Option<&'a RefNode>`.

- [ ] **Step 1: Testes que falham** em `refs.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tree() -> Value {
        json!({ "nodes": [
            { "nodeId": "1", "ignored": false, "role": {"value": "RootWebArea"}, "name": {"value": "fixture"}, "childIds": ["2", "6", "7"], "backendDOMNodeId": 1 },
            { "nodeId": "2", "ignored": false, "role": {"value": "dialog"}, "name": {"value": "Candidatar-se"}, "childIds": ["3", "4", "5"], "backendDOMNodeId": 2 },
            { "nodeId": "3", "ignored": false, "role": {"value": "textbox"}, "name": {"value": "Nome"}, "backendDOMNodeId": 3 },
            { "nodeId": "4", "ignored": false, "role": {"value": "button"}, "name": {"value": "Avançar"}, "backendDOMNodeId": 4 },
            { "nodeId": "5", "ignored": true,  "role": {"value": "button"}, "name": {"value": "Escondido"}, "backendDOMNodeId": 5 },
            { "nodeId": "6", "ignored": false, "role": {"value": "link"},   "name": {"value": "Fora do modal"}, "backendDOMNodeId": 6 },
            { "nodeId": "7", "ignored": false, "role": {"value": "button"}, "name": {"value": "Desligado"},
              "properties": [{"name": "disabled", "value": {"type": "boolean", "value": true}}], "backendDOMNodeId": 7 }
        ]})
    }

    #[test]
    fn keeps_only_visible_interactive_nodes_with_sequential_refs() {
        let n = build_refs(&tree(), None).unwrap();
        let got: Vec<(&str, &str, &str)> = n.iter().map(|r| (r.reference.as_str(), r.role.as_str(), r.name.as_str())).collect();
        assert_eq!(got, vec![
            ("e1", "textbox", "Nome"), ("e2", "button", "Avançar"),
            ("e3", "link", "Fora do modal"), ("e4", "button", "Desligado"),
        ]);
        assert!(n[3].disabled);
        assert_eq!(n[1].backend_node_id, 4);
    }

    #[test]
    fn scope_limits_to_the_descendants_of_the_matching_node() {
        let n = build_refs(&tree(), Some("dialog")).unwrap();
        let names: Vec<&str> = n.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["Nome", "Avançar"]);
    }

    #[test]
    fn scope_matches_by_name_too_and_ignores_case() {
        let n = build_refs(&tree(), Some("candidatar")).unwrap();
        assert_eq!(n.len(), 2);
    }

    #[test]
    fn unknown_scope_is_an_error_not_an_empty_list() {
        let e = build_refs(&tree(), Some("nada disto")).unwrap_err();
        assert!(e.contains("scope"), "{e}");
    }

    #[test]
    fn render_and_find() {
        let n = build_refs(&tree(), None).unwrap();
        let text = render(&n);
        assert!(text.contains("e2 button \"Avançar\""), "{text}");
        assert!(text.contains("e4 button \"Desligado\" (disabled)"), "{text}");
        assert_eq!(find(&n, "e3").unwrap().name, "Fora do modal");
        assert!(find(&n, "e99").is_none());
    }

    #[test]
    fn empty_page_renders_a_hint() {
        let n = build_refs(&json!({ "nodes": [] }), None).unwrap();
        assert!(render(&n).contains("nenhum"));
    }
}
```
- [ ] **Step 2: Run** `cargo test browser::refs` — Expected: FAIL.
- [ ] **Step 3: Implementação**:
```rust
// Turns Accessibility.getFullAXTree into short refs (e1, e2, ...) the agent can
// click by. The accessibility tree is where the accessible name lives; it is
// what made "12 of 12 named fields" possible in the prototype.
use serde_json::Value;
use std::collections::{HashMap, HashSet};

const INTERACTIVE: &[&str] = &[
    "button", "link", "textbox", "searchbox", "combobox", "radio", "checkbox",
    "menuitem", "tab", "option", "switch", "spinbutton", "listbox",
];

#[derive(Debug, Clone, PartialEq)]
pub struct RefNode {
    pub reference: String,
    pub role: String,
    pub name: String,
    pub backend_node_id: i64,
    pub disabled: bool,
}

fn val(v: &Value) -> String {
    v["value"].as_str().unwrap_or("").to_string()
}

fn is_disabled(n: &Value) -> bool {
    n["properties"].as_array().map_or(false, |ps| {
        ps.iter().any(|p| p["name"] == "disabled" && p["value"]["value"] == true)
    })
}

pub fn build_refs(ax: &Value, scope: Option<&str>) -> Result<Vec<RefNode>, String> {
    let nodes = ax["nodes"].as_array().cloned().unwrap_or_default();
    let by_id: HashMap<String, &Value> = nodes
        .iter()
        .filter_map(|n| n["nodeId"].as_str().map(|id| (id.to_string(), n)))
        .collect();

    let allowed: Option<HashSet<String>> = match scope {
        None => None,
        Some(s) => {
            let needle = s.to_lowercase();
            let root = nodes.iter().find(|n| {
                val(&n["role"]).to_lowercase() == needle || val(&n["name"]).to_lowercase().contains(&needle)
            });
            let Some(root) = root else {
                return Err(format!("scope \"{s}\" not found in the page"));
            };
            let mut seen = HashSet::new();
            let mut stack = vec![root["nodeId"].as_str().unwrap_or("").to_string()];
            while let Some(id) = stack.pop() {
                if !seen.insert(id.clone()) {
                    continue;
                }
                if let Some(n) = by_id.get(&id) {
                    for c in n["childIds"].as_array().into_iter().flatten() {
                        if let Some(cid) = c.as_str() {
                            stack.push(cid.to_string());
                        }
                    }
                }
            }
            Some(seen)
        }
    };

    let mut out = Vec::new();
    for n in &nodes {
        let id = n["nodeId"].as_str().unwrap_or("");
        if let Some(a) = &allowed {
            if !a.contains(id) {
                continue;
            }
        }
        if n["ignored"] == true {
            continue;
        }
        let role = val(&n["role"]);
        if !INTERACTIVE.contains(&role.as_str()) {
            continue;
        }
        let name = val(&n["name"]);
        if name.is_empty() && role != "textbox" && role != "searchbox" {
            continue;
        }
        let Some(backend) = n["backendDOMNodeId"].as_i64() else { continue };
        out.push(RefNode {
            reference: format!("e{}", out.len() + 1),
            role,
            name,
            backend_node_id: backend,
            disabled: is_disabled(n),
        });
    }
    Ok(out)
}

pub fn render(nodes: &[RefNode]) -> String {
    if nodes.is_empty() {
        return "(nenhum elemento interativo encontrado)".to_string();
    }
    nodes
        .iter()
        .map(|n| {
            let name = if n.name.is_empty() { "(sem nome)" } else { &n.name };
            format!("{} {} \"{}\"{}", n.reference, n.role, name, if n.disabled { " (disabled)" } else { "" })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn find<'a>(nodes: &'a [RefNode], reference: &str) -> Option<&'a RefNode> {
    nodes.iter().find(|n| n.reference == reference)
}
```
- [ ] **Step 4:** `pub mod refs;` em `mod.rs`. **Run** `cargo test browser::refs` — Expected: 6 passam.
- [ ] **Step 5: Commit** `git add src-tauri && git commit -m "feat(browser): accessibility tree to clickable refs"`

### Task 1.5: Guarda de envio (pode rodar em paralelo com 1.4 e 1.6)

**Files:**
- Create: `src-tauri/submit-patterns.txt`
- Create: `src-tauri/src/browser/guard.rs`
- Modify: `src-tauri/src/browser/mod.rs` (`pub mod guard;`)

**Interfaces:**
- Produces: `guard::looks_like_submit(&str)->bool`; `guard::GuardContext` (trait: `vaga_status(&self,i64)->Option<String>`, `applications_today(&self)->i64`, `daily_limit(&self)->i64`, `site_allowed(&self,&str)->bool`); `guard::Verdict::{Allow, Refuse(String)}`; `guard::check_submit(name:&str, confirm:Option<i64>, ctx:&dyn GuardContext)->Verdict`.

- [ ] **Step 1: `submit-patterns.txt`** (um padrão por linha, já sem acento e em minúsculas):
```
enviar candidatura
enviar solicitacao
submit application
submit your application
send application
postuler
indsend ansogning
send ansogning
bewerbung absenden
```
- [ ] **Step 2: Testes que falham** em `guard.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct Ctx { status: Option<&'static str>, today: i64, limit: i64, allowed: bool }
    impl GuardContext for Ctx {
        fn vaga_status(&self, _id: i64) -> Option<String> { self.status.map(String::from) }
        fn applications_today(&self) -> i64 { self.today }
        fn daily_limit(&self) -> i64 { self.limit }
        fn site_allowed(&self, _url: &str) -> bool { self.allowed }
    }
    fn ok() -> Ctx { Ctx { status: Some("candidatando"), today: 2, limit: 10, allowed: true } }

    #[test]
    fn recognises_submit_buttons_in_several_languages_and_cases() {
        for n in ["Enviar candidatura", "ENVIAR CANDIDATURA", "Submit application", "Enviar solicitação",
                  "  Send   application ", "Indsend ansøgning", "Postuler"] {
            assert!(looks_like_submit(n), "{n}");
        }
    }

    #[test]
    fn ordinary_buttons_are_not_submit() {
        for n in ["Avançar", "Avaliar", "Revisar", "Enviar mensagem", "Próximo", "Continuar", ""] {
            assert!(!looks_like_submit(n), "{n}");
        }
    }

    #[test]
    fn non_submit_clicks_are_always_allowed() {
        let ctx = Ctx { status: None, today: 99, limit: 1, allowed: false };
        assert_eq!(check_submit("Avançar", None, &ctx), Verdict::Allow);
    }

    #[test]
    fn submit_without_confirmation_is_refused() {
        assert!(matches!(check_submit("Enviar candidatura", None, &ok()), Verdict::Refuse(m) if m.contains("confirm_submit")));
    }

    #[test]
    fn submit_needs_the_vaga_to_be_candidatando() {
        let ctx = Ctx { status: Some("analisada"), ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", Some(7), &ctx), Verdict::Refuse(m) if m.contains("candidatando")));
        let none = Ctx { status: None, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", Some(7), &none), Verdict::Refuse(_)));
    }

    #[test]
    fn submit_stops_at_the_daily_budget() {
        let ctx = Ctx { today: 10, limit: 10, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", Some(7), &ctx), Verdict::Refuse(m) if m.contains("orçamento")));
    }

    #[test]
    fn a_zero_limit_means_no_budget_cap() {
        let ctx = Ctx { today: 500, limit: 0, ..ok() };
        assert_eq!(check_submit("Enviar candidatura", Some(7), &ctx), Verdict::Allow);
    }

    #[test]
    fn submit_is_refused_on_a_site_that_is_not_allowed() {
        let ctx = Ctx { allowed: false, ..ok() };
        assert!(matches!(check_submit("Enviar candidatura", Some(7), &ctx), Verdict::Refuse(_)));
    }

    #[test]
    fn submit_passes_when_all_three_conditions_hold() {
        assert_eq!(check_submit("Enviar candidatura", Some(7), &ok()), Verdict::Allow);
    }
}
```
(O `site_allowed` recebe a URL; em `check_submit` a URL não é conhecida: o chamador passa o contexto já ligado à página. Ver Step 3: `check_submit` chama `ctx.site_allowed("")` e a implementação real, na 2.3, é `true` até o ciclo 2.)

- [ ] **Step 3: Implementação**:
```rust
// The send guard. It lives in app code, not in a prompt: the model can be
// wrong, injected or lazy, this cannot.
const PATTERNS: &str = include_str!("../../submit-patterns.txt");

/// Lowercase, strip accents and collapse whitespace so "Enviar  solicitação"
/// and "enviar solicitacao" compare equal.
fn fold(s: &str) -> String {
    let mapped: String = s
        .chars()
        .flat_map(|c| {
            let c = c.to_lowercase().next().unwrap_or(c);
            let r: &str = match c {
                'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => "a",
                'é' | 'è' | 'ê' | 'ë' => "e",
                'í' | 'ì' | 'î' | 'ï' => "i",
                'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'ø' => "o",
                'ú' | 'ù' | 'û' | 'ü' => "u",
                'ç' => "c",
                'æ' => "ae",
                _ => return vec![c],
            };
            r.chars().collect::<Vec<_>>()
        })
        .collect();
    mapped.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn looks_like_submit(accessible_name: &str) -> bool {
    let name = fold(accessible_name);
    if name.is_empty() {
        return false;
    }
    PATTERNS.lines().map(str::trim).filter(|l| !l.is_empty()).any(|p| name.contains(p))
}

pub trait GuardContext: Send + Sync {
    fn vaga_status(&self, vaga_id: i64) -> Option<String>;
    fn applications_today(&self) -> i64;
    /// 0 means "no cap".
    fn daily_limit(&self) -> i64;
    fn site_allowed(&self, url: &str) -> bool;
}

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Allow,
    Refuse(String),
}

pub fn check_submit(name: &str, confirm: Option<i64>, ctx: &dyn GuardContext) -> Verdict {
    if !looks_like_submit(name) {
        return Verdict::Allow;
    }
    let Some(vaga_id) = confirm else {
        return Verdict::Refuse(
            "RECUSADO: este botão envia a candidatura. Repita a chamada com confirm_submit = <id da vaga> \
             (a vaga precisa estar em 'candidatando')."
                .into(),
        );
    };
    match ctx.vaga_status(vaga_id).as_deref() {
        Some("candidatando") => {}
        other => {
            return Verdict::Refuse(format!(
                "RECUSADO: a vaga {vaga_id} está em '{}', não em 'candidatando'.",
                other.unwrap_or("inexistente")
            ))
        }
    }
    let limit = ctx.daily_limit();
    if limit > 0 && ctx.applications_today() >= limit {
        return Verdict::Refuse(format!("RECUSADO: o orçamento diário de {limit} candidaturas já foi usado."));
    }
    if !ctx.site_allowed("") {
        return Verdict::Refuse("RECUSADO: este site não está permitido para envio.".into());
    }
    Verdict::Allow
}
```
- [ ] **Step 4:** `pub mod guard;` em `mod.rs`. **Run** `cargo test browser::guard` — Expected: 9 passam.
- [ ] **Step 5: Commit** `git add src-tauri && git commit -m "feat(browser): send guard enforced in app code"`

### Task 1.6: Tema laranja no perfil (pode rodar em paralelo com 1.4 e 1.5)

**Files:**
- Create: `src-tauri/src/browser/theme.rs`
- Modify: `src-tauri/src/browser/mod.rs` (`pub mod theme;`)

**Interfaces:**
- Consumes: `src-tauri/browser-theme.json` (Tarefa 0.3).
- Produces: `theme::deep_merge(&mut Value,&Value)`, `theme::apply_preferences(profile_dir:&Path, patch:&Value)->Result<(),String>`, `theme::apply_theme(profile_dir:&Path)->Result<(),String>`.

Dirigido por dados: o código é genérico; o que funciona (ou não) foi decidido na 0.3. Se a 0.3 concluiu "tema não aplicável", `apply_theme` não faz nada e o teste de integração do tema (6.2) é marcado "não aplicável" no relatório.

- [ ] **Step 1: Testes que falham**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("claudia-theme-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn deep_merge_keeps_unrelated_keys_and_overwrites_leaves() {
        let mut base = json!({ "a": { "keep": 1, "x": 1 }, "b": 2 });
        deep_merge(&mut base, &json!({ "a": { "x": 9, "new": 3 } }));
        assert_eq!(base, json!({ "a": { "keep": 1, "x": 9, "new": 3 }, "b": 2 }));
    }

    #[test]
    fn creates_preferences_in_a_fresh_profile() {
        let dir = temp();
        apply_preferences(&dir, &json!({ "browser": { "theme": { "k": -2525353 } } })).unwrap();
        let text = std::fs::read_to_string(dir.join("Default").join("Preferences")).unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["browser"]["theme"]["k"], -2525353);
    }

    #[test]
    fn preserves_what_chrome_already_wrote() {
        let dir = temp();
        std::fs::create_dir_all(dir.join("Default")).unwrap();
        std::fs::write(dir.join("Default").join("Preferences"), r#"{"profile":{"name":"x"}}"#).unwrap();
        apply_preferences(&dir, &json!({ "browser": { "theme": { "k": 1 } } })).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("Default").join("Preferences")).unwrap()).unwrap();
        assert_eq!(v["profile"]["name"], "x");
        assert_eq!(v["browser"]["theme"]["k"], 1);
    }

    #[test]
    fn a_corrupt_preferences_file_is_replaced_not_fatal() {
        let dir = temp();
        std::fs::create_dir_all(dir.join("Default")).unwrap();
        std::fs::write(dir.join("Default").join("Preferences"), "{ not json").unwrap();
        apply_preferences(&dir, &json!({ "a": 1 })).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("Default").join("Preferences")).unwrap()).unwrap();
        assert_eq!(v["a"], 1);
    }

    #[test]
    fn the_shipped_theme_file_parses() {
        let cfg: Value = serde_json::from_str(include_str!("../../browser-theme.json")).unwrap();
        assert!(cfg.get("preferences").is_some() && cfg.get("policy").is_some());
    }
}
```
- [ ] **Step 2: Run** `cargo test browser::theme` — Expected: FAIL.
- [ ] **Step 3: Implementação**:
```rust
// Applies the orange theme to the dedicated profile before Chrome's first launch.
// What to write is data (browser-theme.json), found and verified in Task 0.3.
use serde_json::Value;
use std::path::Path;

pub fn deep_merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                deep_merge(b.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (b, p) => *b = p.clone(),
    }
}

pub fn apply_preferences(profile_dir: &Path, patch: &Value) -> Result<(), String> {
    let path = profile_dir.join("Default").join("Preferences");
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let mut current: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    deep_merge(&mut current, patch);
    std::fs::write(&path, serde_json::to_string(&current).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

pub fn apply_theme(profile_dir: &Path) -> Result<(), String> {
    let cfg: Value = serde_json::from_str(include_str!("../../browser-theme.json")).map_err(|e| e.to_string())?;
    if let Some(p) = cfg.get("preferences").filter(|p| !p.is_null()) {
        apply_preferences(profile_dir, p)?;
    }
    #[cfg(windows)]
    if let Some(pol) = cfg.get("policy").filter(|p| !p.is_null()) {
        apply_policy(pol)?;
    }
    Ok(())
}

#[cfg(windows)]
fn apply_policy(pol: &Value) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let key = pol["key"].as_str().ok_or("policy.key missing")?;
    let (k, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(key).map_err(|e| e.to_string())?;
    for (name, v) in pol["values"].as_object().into_iter().flatten() {
        k.set_value(name, &v.as_str().unwrap_or("").to_string()).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```
- [ ] **Step 4:** `pub mod theme;` em `mod.rs`. **Run** `cargo test browser::theme` — Expected: 5 passam.
- [ ] **Step 5: Commit** `git add src-tauri && git commit -m "feat(browser): data-driven orange theme for the dedicated profile"`

### Task 1.7: Gestor do Chrome (lançar, reaproveitar, vigiar)

**Files:**
- Create: `src-tauri/src/browser/manager.rs`
- Modify: `src-tauri/src/browser/mod.rs` (`pub mod manager;`)

**Interfaces:**
- Produces: `manager::LaunchOptions{exe,profile_dir,headless}` com `LaunchOptions::visible(exe,profile)->Self` e `#[doc(hidden)] LaunchOptions::headless_for_tests(exe,profile)->Self` (não `#[cfg(test)]`, porque os testes de integração são outro crate; o nome e o `doc(hidden)` sinalizam o uso); `manager::build_args(&LaunchOptions,u16)->Vec<String>`; `manager::free_port()->Result<u16,String>`; `manager::BrowserState{port,pid,version}` (Serialize/Deserialize/Clone/PartialEq/Debug); `manager::{write_state,read_state}`; `manager::probe(port)->Option<String>`; `manager::wait_ready(port,Duration)->Result<String,String>`; `manager::ensure_running(&LaunchOptions, version:&str, state_path:&Path)->Result<BrowserState,String>`.

Cobre os Review Focus 1 e 2.

- [ ] **Step 1: Testes que falham** em `manager.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn temp() -> PathBuf {
        let d = std::env::temp_dir().join(format!("claudia-mgr-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Fake /json/version endpoint answering with the given Browser string.
    async fn fake_version(browser: &'static str) -> u16 {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = l.accept().await {
                let mut b = [0u8; 1024];
                let _ = s.read(&mut b).await;
                let body = format!(r#"{{"Browser":"{browser}"}}"#);
                let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await;
            }
        });
        port
    }

    #[test]
    fn production_launch_is_never_headless() {
        let o = LaunchOptions::visible("C:/c/chrome.exe".into(), "C:/p".into());
        let a = build_args(&o, 9333);
        assert!(!a.iter().any(|x| x.contains("headless")), "{a:?}");
        assert!(a.contains(&"--remote-debugging-port=9333".to_string()));
        assert!(a.iter().any(|x| x == "--user-data-dir=C:/p"));
        assert!(a.contains(&"--no-first-run".to_string()));
        assert!(a.contains(&"--no-default-browser-check".to_string()));
    }

    #[test]
    fn test_launch_is_headless() {
        let o = LaunchOptions::headless_for_tests("c".into(), "p".into());
        assert!(build_args(&o, 1).iter().any(|x| x.starts_with("--headless")));
    }

    #[test]
    fn free_port_is_actually_free() {
        let p = free_port().unwrap();
        assert!(std::net::TcpListener::bind(("127.0.0.1", p)).is_ok());
    }

    #[test]
    fn state_roundtrips_and_garbage_reads_as_none() {
        let dir = temp();
        let f = dir.join("sub").join("state.json");
        let s = BrowserState { port: 9333, pid: 42, version: "154.0.1.2".into() };
        write_state(&f, &s).unwrap();
        assert_eq!(read_state(&f), Some(s));
        std::fs::write(&f, "{ nope").unwrap();
        assert_eq!(read_state(&f), None);
        assert_eq!(read_state(&dir.join("missing.json")), None);
    }

    #[tokio::test]
    async fn probe_reads_the_browser_string() {
        let port = fake_version("Chrome/154.0.1.2").await;
        assert_eq!(probe(port).await.as_deref(), Some("Chrome/154.0.1.2"));
        assert_eq!(probe(1).await, None);
    }

    #[tokio::test]
    async fn a_live_browser_with_the_pinned_version_is_reused_not_relaunched() {
        // The exe path does not exist: any attempt to spawn would fail the test.
        let port = fake_version("Chrome/154.0.1.2").await;
        let dir = temp();
        let state_path = dir.join("state.json");
        write_state(&state_path, &BrowserState { port, pid: 1, version: "154.0.1.2".into() }).unwrap();
        let o = LaunchOptions::visible(dir.join("no-such-chrome.exe"), dir.join("p"));
        let s = ensure_running(&o, "154.0.1.2", &state_path).await.unwrap();
        assert_eq!(s.port, port);
    }

    #[tokio::test]
    async fn a_port_that_now_belongs_to_another_browser_is_not_reused() {
        // Review Focus 2: someone else (the user's own Chrome) answers on the old port.
        let port = fake_version("Chrome/120.0.0.0").await;
        let dir = temp();
        let state_path = dir.join("state.json");
        write_state(&state_path, &BrowserState { port, pid: 1, version: "154.0.1.2".into() }).unwrap();
        let o = LaunchOptions::visible(dir.join("no-such-chrome.exe"), dir.join("p"));
        // It must try to launch ours (and fail here, because the exe does not exist).
        let e = ensure_running(&o, "154.0.1.2", &state_path).await.unwrap_err();
        assert!(e.contains("spawn") || e.contains("launch"), "{e}");
    }

    #[tokio::test]
    async fn stale_state_with_a_dead_port_launches_again() {
        let dir = temp();
        let state_path = dir.join("state.json");
        write_state(&state_path, &BrowserState { port: 1, pid: 1, version: "154.0.1.2".into() }).unwrap();
        let o = LaunchOptions::visible(dir.join("no-such-chrome.exe"), dir.join("p"));
        assert!(ensure_running(&o, "154.0.1.2", &state_path).await.is_err());
    }
}
```
- [ ] **Step 2: Run** `cargo test browser::manager` — Expected: FAIL.
- [ ] **Step 3: Implementação**:
```rust
// Launches and reuses the one Chrome the app owns. The app (main process) owns
// the browser; MCP processes only read state.json to find the port.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct LaunchOptions {
    pub exe: PathBuf,
    pub profile_dir: PathBuf,
    headless: bool,
}

impl LaunchOptions {
    /// The only constructor production code may use: the user must always see the window.
    pub fn visible(exe: PathBuf, profile_dir: PathBuf) -> Self {
        Self { exe, profile_dir, headless: false }
    }

    /// Tests only. Never call this from app code.
    #[doc(hidden)]
    pub fn headless_for_tests(exe: PathBuf, profile_dir: PathBuf) -> Self {
        Self { exe, profile_dir, headless: true }
    }
}

pub fn build_args(o: &LaunchOptions, port: u16) -> Vec<String> {
    let mut a = vec![
        format!("--remote-debugging-port={port}"),
        format!("--user-data-dir={}", o.profile_dir.display()),
        "--no-first-run".to_string(),
        "--no-default-browser-check".to_string(),
    ];
    if o.headless {
        a.push("--headless=new".to_string());
    }
    a
}

/// 9222 is usually the user's own Chrome, so always ask the OS for a free port.
pub fn free_port() -> Result<u16, String> {
    let l = std::net::TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
    l.local_addr().map(|a| a.port()).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserState {
    pub port: u16,
    pub pid: u32,
    pub version: String,
}

pub fn write_state(path: &Path, s: &BrowserState) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or("state path has no parent")?).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub fn read_state(path: &Path) -> Option<BrowserState> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Returns the "Browser" string (e.g. "Chrome/154.0.8037.92") if something
/// speaking CDP answers on `port`.
pub async fn probe(port: u16) -> Option<String> {
    let r = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/json/version"))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .ok()?;
    let v: serde_json::Value = r.json().await.ok()?;
    v["Browser"].as_str().map(String::from)
}

pub async fn wait_ready(port: u16, timeout: Duration) -> Result<String, String> {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Some(b) = probe(port).await {
            return Ok(b);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    Err(format!("the browser did not answer on port {port} within {timeout:?}"))
}

/// Reuses a live browser of the pinned version; otherwise launches ours.
/// Never starts a second Chrome on the same profile.
pub async fn ensure_running(o: &LaunchOptions, version: &str, state_path: &Path) -> Result<BrowserState, String> {
    if let Some(s) = read_state(state_path) {
        if let Some(b) = probe(s.port).await {
            if b.contains(version) {
                return Ok(s);
            }
        }
    }
    let port = free_port()?;
    std::fs::create_dir_all(&o.profile_dir).map_err(|e| e.to_string())?;
    let child = std::process::Command::new(&o.exe)
        .args(build_args(o, port))
        .spawn()
        .map_err(|e| format!("could not spawn the browser ({}): {e}", o.exe.display()))?;
    wait_ready(port, Duration::from_secs(20)).await.map_err(|e| format!("launch failed: {e}"))?;
    let state = BrowserState { port, pid: child.id(), version: version.to_string() };
    write_state(state_path, &state)?;
    Ok(state)
}
```
- [ ] **Step 4:** `pub mod manager;` em `mod.rs`. **Run** `cargo test browser::manager` — Expected: 8 passam.
- [ ] **Step 5: Commit** `git add src-tauri && git commit -m "feat(browser): launch, reuse and probe the app-owned Chrome"`

### Task 1.8: Portão da Fase 1

- [ ] **Step 1:** `cd src-tauri && cargo test` e `cargo clippy` (comparar com o baseline; **não** pode ter warning novo nos arquivos `browser/`).
- [ ] **Step 2: Revisão de fase pelo Opus** de `git diff dev...HEAD -- src-tauri/src/browser src-tauri/Cargo.toml src-tauri/*.json src-tauri/*.txt`: spec §3, §5.1 a §5.5; teste que passa sem provar nada; o Review Focus 1, 2 e 5 estão cobertos.

---

# FASE 2: Chrome de verdade e MCP

### Task 2.1: Página de teste e harness de integração (valida o CDP em Rust com um Chrome real)

**Files:**
- Create: `src-tauri/tests/fixtures/apply-form.html`
- Create: `src-tauri/tests/browser_live.rs`

**Interfaces:**
- Consumes: `browser::{cdp, refs, manager}`.
- Produces: o helper de teste `launch()` e a convenção: testes `#[ignore]`, rodados com `CLAUDIA_TEST_CHROME=<chrome.exe> cargo test --test browser_live -- --ignored`.

Esta tarefa responde a incógnita original da CLA-31: o cliente CDP próprio funciona com um Chrome real, a partir de Rust.

- [ ] **Step 1: Criar `apply-form.html`** (imita as armadilhas do LinkedIn):
```html
<!doctype html>
<html lang="pt"><head><meta charset="utf-8"><title>fixture</title></head>
<body>
<div id="modal" role="dialog" aria-label="Candidatar-se">
  <label for="name">Nome</label>
  <input id="name" type="text">
  <label for="pw">Senha</label>
  <input id="pw" type="password">
  <div style="position:relative;height:24px">
    <!-- trap 3: a real radio, invisible, under another element -->
    <input id="r1" type="radio" name="g" style="opacity:0;position:absolute;left:0;top:0;width:13px;height:13px">
    <label for="r1">Sim</label>
  </div>
  <button id="adv">Avançar</button>
  <a id="lnk" href="#next">Link</a>
</div>
<script>
  // trap 5: the advance button renames itself. The last label is the submit.
  const labels = ["Avançar", "Avaliar", "Enviar candidatura"];
  let step = 0;
  window.__submitted = false;
  const adv = document.getElementById("adv");
  adv.addEventListener("click", () => {
    if (labels[step] === "Enviar candidatura") { window.__submitted = true; return; }
    step += 1; adv.textContent = labels[step];
  });
</script>
</body></html>
```
- [ ] **Step 2: Escrever `browser_live.rs`** com o harness e os primeiros testes:
```rust
//! Live tests: a real Chrome (headless) against tests/fixtures/apply-form.html.
//! Run: CLAUDIA_TEST_CHROME="C:/path/chrome.exe" cargo test --test browser_live -- --ignored
use claudia_rh_lib::browser::{cdp::{self, CdpClient}, manager::{self, LaunchOptions}, refs};
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

fn chrome() -> PathBuf {
    PathBuf::from(std::env::var("CLAUDIA_TEST_CHROME").expect("set CLAUDIA_TEST_CHROME to chrome.exe"))
}

fn fixture_url() -> String {
    let p = std::fs::canonicalize("tests/fixtures/apply-form.html").unwrap();
    format!("file:///{}", p.display().to_string().trim_start_matches(r"\\?\").replace('\\', "/"))
}

struct Live { client: CdpClient, port: u16, pid: u32, profile: PathBuf }

impl Drop for Live {
    fn drop(&mut self) {
        let _ = std::process::Command::new("taskkill").args(["/PID", &self.pid.to_string(), "/T", "/F"]).output();
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

async fn launch() -> Live {
    let profile = std::env::temp_dir().join(format!("claudia-live-{}-{}", std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let state_path = profile.join("state.json");
    let o = LaunchOptions::headless_for_tests(chrome(), profile.join("p"));
    let st = manager::ensure_running(&o, "", &state_path).await.expect("chrome launches");
    let page = cdp::first_page(st.port).await.unwrap();
    let client = CdpClient::connect(&page.ws_url).await.unwrap();
    for m in ["Page.enable", "DOM.enable", "Accessibility.enable"] {
        client.call(m, json!({})).await.unwrap();
    }
    client.call("Page.navigate", json!({ "url": fixture_url() })).await.unwrap();
    for _ in 0..40 {
        let r = client.call("Runtime.evaluate", json!({ "expression": "document.readyState", "returnByValue": true })).await.unwrap();
        if r["result"]["value"] == "complete" { break; }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Live { client, port: st.port, pid: st.pid, profile }
}

async fn eval(c: &CdpClient, expr: &str) -> serde_json::Value {
    c.call("Runtime.evaluate", json!({ "expression": expr, "returnByValue": true })).await.unwrap()["result"]["value"].clone()
}

#[tokio::test]
#[ignore]
async fn real_chrome_answers_cdp_and_we_can_read_the_accessibility_tree() {
    let l = launch().await;
    let ax = l.client.call("Accessibility.getFullAXTree", json!({})).await.unwrap();
    let nodes = refs::build_refs(&ax, None).unwrap();
    let names: Vec<&str> = nodes.iter().map(|n| n.name.as_str()).collect();
    assert!(names.contains(&"Nome"), "{names:?}");
    assert!(names.contains(&"Avançar"), "{names:?}");
    assert!(names.contains(&"Sim"), "{names:?}");
    let _ = l.port;
}

#[tokio::test]
#[ignore]
async fn a_second_ensure_running_reuses_the_same_chrome() {
    let l = launch().await;
    // Same state file as launch(): a second call must not start another Chrome.
    let state_path = l.profile.join("state.json");
    let o = LaunchOptions::headless_for_tests(chrome(), l.profile.join("p"));
    let again = manager::ensure_running(&o, "", &state_path).await.unwrap();
    assert_eq!(again.port, l.port);
    assert_eq!(again.pid, l.pid);
}

#[tokio::test]
#[ignore]
async fn the_chrome_we_launched_reports_itself_visible() {
    let l = launch().await;
    let page = cdp::first_page(l.port).await.unwrap();
    cdp::bring_to_front(l.port, &page.id, &l.client).await.unwrap();
    assert_eq!(eval(&l.client, "document.visibilityState").await, "visible");
}

#[tokio::test]
#[ignore]
async fn the_applied_profile_preferences_give_a_normal_language_list() {
    // Task 0.6: a one-language navigator.languages is a weak automation signal.
    // Apply the shipped theme/preferences to the profile BEFORE launching, as the app does.
    let profile = std::env::temp_dir().join(format!("claudia-lang-{}", std::process::id()));
    claudia_rh_lib::browser::theme::apply_theme(&profile.join("p")).unwrap();
    let state_path = profile.join("state.json");
    let o = LaunchOptions::headless_for_tests(chrome(), profile.join("p"));
    let st = manager::ensure_running(&o, "", &state_path).await.unwrap();
    let page = cdp::first_page(st.port).await.unwrap();
    let client = CdpClient::connect(&page.ws_url).await.unwrap();
    client.call("Page.navigate", json!({ "url": fixture_url() })).await.unwrap();
    tokio::time::sleep(Duration::from_millis(800)).await;
    let n = eval(&client, "navigator.languages.length").await;
    let _ = std::process::Command::new("taskkill").args(["/PID", &st.pid.to_string(), "/T", "/F"]).output();
    let _ = std::fs::remove_dir_all(&profile);
    assert!(n.as_i64().unwrap() >= 2, "navigator.languages has {n} entries");
}
```
Nota: `ensure_running(&o, "", ...)` usa a versão vazia (`b.contains("")` é sempre verdadeiro) nos testes, porque o Chrome do teste pode não ser o fixado.

- [ ] **Step 3: Rodar** com o Chrome da Tarefa 0.2:
```bash
cd src-tauri && CLAUDIA_TEST_CHROME="$TEMP/cft/chrome-win64/chrome.exe" cargo test --test browser_live -- --ignored --test-threads=1
```
Expected: 3 passam. Se falhar por o `file://` não carregar em headless, registrar e trocar a página por um servidor HTTP local de 15 linhas **no teste**, sem mudar o HTML.
- [ ] **Step 4: Commit** `git add src-tauri && git commit -m "test(browser): live harness proving the Rust CDP client against a real Chrome"`

### Task 2.2: As ações do browser (`BrowserTools`)

**Files:**
- Create: `src-tauri/src/mcp/tools/browser.rs`
- Modify: `src-tauri/src/mcp/tools/mod.rs` (acrescentar `pub mod browser;`)
- Modify: `src-tauri/tests/browser_live.rs` (testes das ações)

**Interfaces:**
- Consumes: `browser::{cdp, guard, manager::read_state, refs}`.
- Produces:
  - `ToolOutput::{Text(String), Image{data_base64:String, mime:String}}`
  - `ActionRecord{tool:String, target:Option<String>, role:Option<String>, text:Option<String>, url:Option<String>, result:String, error:Option<String>, duration_ms:u64, shot_before:Option<Vec<u8>>, shot_after:Option<Vec<u8>>}`
  - `trait ActionSink: Send+Sync { fn enabled(&self)->bool; fn action(&self, rec:ActionRecord); fn navigation(&self, rec:crate::browser::nav::NavRecord); }` e `NoSink`
  - `BrowserTools::new(state_file:PathBuf, guard:Arc<dyn GuardContext>, sink:Arc<dyn ActionSink>)->Self`, `.allow_file_urls()->Self`
  - `async fn open(&self,&str)->Result<String,String>`, `read(&self,Option<&str>)`, `click(&self,&str,Option<i64>)`, `type_text(&self,&str,&str)`, `key(&self,&str,Option<i64>)`, `crop(&self,f64,f64,f64,f64)->Result<ToolOutput,String>`, `click_at(&self,f64,f64,Option<i64>)`
  - `DbGuardContext{data_dir:PathBuf}` implementando `GuardContext`

`NavRecord` vem da Tarefa 3.4; para esta tarefa compilar, criar antes um stub **mínimo e real** em `browser/nav.rs`: `pub struct NavRecord { pub url:String, pub status:Option<i64>, pub redirects:Vec<String>, pub initiator:String }` (a 3.4 acrescenta o `NavTracker` no mesmo arquivo).

Cobre os Review Focus 3 e 4.

- [ ] **Step 1: Criar `browser/nav.rs` com o `NavRecord`** e `pub mod nav;` em `browser/mod.rs`:
```rust
// Navigation history records (the tracker that produces them arrives in Task 3.4).
#[derive(Debug, Clone, PartialEq)]
pub struct NavRecord {
    pub url: String,
    pub status: Option<i64>,
    pub redirects: Vec<String>,
    /// "agent" while a browser_* tool is running; "outside-tool" otherwise.
    pub initiator: String,
}
```
- [ ] **Step 2: Escrever os testes live primeiro** (acrescentar a `browser_live.rs`). Usam `BrowserTools` contra o Chrome do harness; para isso o harness precisa escrever o `state.json` (já o faz, via `ensure_running`) e a página tem de estar aberta: `BrowserTools::open` navega.
```rust
use claudia_rh_lib::browser::guard::GuardContext;
use claudia_rh_lib::mcp::tools::browser::{BrowserTools, NoSink, ToolOutput};
use std::sync::Arc;

struct AllowAll;
impl GuardContext for AllowAll {
    fn vaga_status(&self, _: i64) -> Option<String> { Some("candidatando".into()) }
    fn applications_today(&self) -> i64 { 0 }
    fn daily_limit(&self) -> i64 { 10 }
    fn site_allowed(&self, _: &str) -> bool { true }
}
struct DenyAll;
impl GuardContext for DenyAll {
    fn vaga_status(&self, _: i64) -> Option<String> { Some("analisada".into()) }
    fn applications_today(&self) -> i64 { 0 }
    fn daily_limit(&self) -> i64 { 10 }
    fn site_allowed(&self, _: &str) -> bool { true }
}

fn tools(l: &Live, g: Arc<dyn GuardContext>) -> BrowserTools {
    BrowserTools::new(l.profile.join("state.json"), g, Arc::new(NoSink)).allow_file_urls()
}

fn ref_of(listing: &str, role: &str, name: &str) -> String {
    listing.lines().find(|l| l.contains(&format!("{role} \"{name}\""))).unwrap_or_else(|| panic!("no {role} {name} in:\n{listing}"))
        .split_whitespace().next().unwrap().to_string()
}

#[tokio::test] #[ignore]
async fn click_by_ref_advances_the_button_and_it_renames_itself() {
    let l = launch().await; let t = tools(&l, Arc::new(AllowAll));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.click(&ref_of(&list, "button", "Avançar"), None).await.unwrap();
    let list2 = t.read(None).await.unwrap();
    assert!(list2.contains("button \"Avaliar\""), "{list2}");
}

#[tokio::test] #[ignore]
async fn the_hidden_radio_is_checked_through_its_label() {
    let l = launch().await; let t = tools(&l, Arc::new(AllowAll));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.click(&ref_of(&list, "radio", "Sim"), None).await.unwrap();
    assert_eq!(eval(&l.client, "document.getElementById('r1').checked").await, true);
}

#[tokio::test] #[ignore]
async fn typing_fills_the_field_and_a_password_field_is_flagged() {
    let l = launch().await; let t = tools(&l, Arc::new(AllowAll));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.type_text(&ref_of(&list, "textbox", "Nome"), "Ana").await.unwrap();
    assert_eq!(eval(&l.client, "document.getElementById('name').value").await, "Ana");
}

#[tokio::test] #[ignore]
async fn submit_is_refused_without_confirmation_and_allowed_with_it() {
    let l = launch().await; let t = tools(&l, Arc::new(AllowAll));
    t.open(&fixture_url()).await.unwrap();
    for _ in 0..2 {
        let list = t.read(None).await.unwrap();
        let adv = list.lines().find(|x| x.contains("button") && !x.contains("Enviar")).map(|x| x.split_whitespace().next().unwrap().to_string()).unwrap();
        t.click(&adv, None).await.unwrap();
    }
    let list = t.read(None).await.unwrap();
    let submit = ref_of(&list, "button", "Enviar candidatura");
    let e = t.click(&submit, None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "{e}");
    assert_eq!(eval(&l.client, "window.__submitted").await, false);
    t.click(&submit, Some(7)).await.unwrap();
    assert_eq!(eval(&l.client, "window.__submitted").await, true);
}

#[tokio::test] #[ignore]
async fn submit_is_refused_when_the_vaga_is_not_candidatando() {
    let l = launch().await; let t = tools(&l, Arc::new(DenyAll));
    t.open(&fixture_url()).await.unwrap();
    eval(&l.client, "document.getElementById('adv').textContent='Enviar candidatura'; true").await;
    let list = t.read(None).await.unwrap();
    let e = t.click(&ref_of(&list, "button", "Enviar candidatura"), Some(7)).await.unwrap_err();
    assert!(e.starts_with("RECUSADO") && e.contains("candidatando"), "{e}");
}

#[tokio::test] #[ignore]
async fn pressing_enter_on_a_focused_submit_button_is_refused_too() {
    // Review Focus 4: the keyboard must not be a way around the guard.
    let l = launch().await; let t = tools(&l, Arc::new(AllowAll));
    t.open(&fixture_url()).await.unwrap();
    eval(&l.client, "(()=>{const b=document.getElementById('adv'); b.textContent='Enviar candidatura'; b.focus(); return true})()").await;
    let e = t.key("Enter", None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "{e}");
    assert_eq!(eval(&l.client, "window.__submitted").await, false);
}

#[tokio::test] #[ignore]
async fn a_stale_ref_after_navigation_is_an_error_not_a_wrong_click() {
    // Review Focus 3.
    let l = launch().await; let t = tools(&l, Arc::new(AllowAll));
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    let adv = ref_of(&list, "button", "Avançar");
    t.open(&fixture_url()).await.unwrap(); // new document, refs are gone
    let e = t.click(&adv, None).await.unwrap_err();
    assert!(e.contains("obsoleta"), "{e}");
}

#[tokio::test] #[ignore]
async fn crop_returns_a_png_image() {
    let l = launch().await; let t = tools(&l, Arc::new(AllowAll));
    t.open(&fixture_url()).await.unwrap();
    match t.crop(0.0, 0.0, 200.0, 100.0).await.unwrap() {
        ToolOutput::Image { data_base64, mime } => { assert_eq!(mime, "image/png"); assert!(data_base64.len() > 100); }
        other => panic!("expected an image, got {other:?}"),
    }
}

#[tokio::test] #[ignore]
async fn click_at_a_coordinate_is_guarded_by_the_element_under_it() {
    let l = launch().await; let t = tools(&l, Arc::new(AllowAll));
    t.open(&fixture_url()).await.unwrap();
    eval(&l.client, "document.getElementById('adv').textContent='Enviar candidatura'; true").await;
    let rect = eval(&l.client, "(()=>{const r=document.getElementById('adv').getBoundingClientRect();return [r.x+r.width/2, r.y+r.height/2]})()").await;
    let (x, y) = (rect[0].as_f64().unwrap(), rect[1].as_f64().unwrap());
    let e = t.click_at(x, y, None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "{e}");
    assert_eq!(eval(&l.client, "window.__submitted").await, false);
}

#[tokio::test] #[ignore]
async fn opening_a_dead_address_reports_the_browser_error() {
    let l = launch().await; let t = tools(&l, Arc::new(AllowAll));
    let e = t.open("http://127.0.0.1:1/").await.unwrap_err();
    assert!(e.to_lowercase().contains("err_") || e.contains("falhou"), "{e}");
}

#[tokio::test] #[ignore]
async fn a_non_http_url_is_refused_before_touching_the_browser() {
    let l = launch().await;
    let t = BrowserTools::new(l.profile.join("state.json"), Arc::new(AllowAll), Arc::new(NoSink)); // file URLs NOT allowed
    let e = t.open("file:///c:/windows/win.ini").await.unwrap_err();
    assert!(e.contains("http"), "{e}");
    let e2 = t.open("javascript:alert(1)").await.unwrap_err();
    assert!(e2.contains("http"), "{e2}");
}
```
Os testes de `DbGuardContext` (unitários, sem Chrome) vão no `mod tests` de `browser.rs` (Step 4).

- [ ] **Step 3: Implementar `mcp/tools/browser.rs`.** Código completo:
```rust
// The agent-facing browser actions. Each one: make sure the tab is visible,
// run the guard, act, and (if the Activity log is on) report what happened.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::browser::cdp::{self, CdpClient};
use crate::browser::guard::{self, GuardContext, Verdict};
use crate::browser::manager::read_state;
use crate::browser::nav::NavRecord;
use crate::browser::refs::{self, RefNode};

#[derive(Debug)]
pub enum ToolOutput {
    Text(String),
    Image { data_base64: String, mime: String },
}

pub struct ActionRecord {
    pub tool: String,
    pub target: Option<String>,
    pub role: Option<String>,
    pub text: Option<String>,
    pub url: Option<String>,
    /// "ok" | "error" | "refused"
    pub result: String,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub shot_before: Option<Vec<u8>>,
    pub shot_after: Option<Vec<u8>>,
}

pub trait ActionSink: Send + Sync {
    /// When false, the tools skip screenshots entirely (dev mode off = zero cost).
    fn enabled(&self) -> bool;
    fn action(&self, rec: ActionRecord);
    fn navigation(&self, rec: NavRecord);
}

pub struct NoSink;
impl ActionSink for NoSink {
    fn enabled(&self) -> bool { false }
    fn action(&self, _rec: ActionRecord) {}
    fn navigation(&self, _rec: NavRecord) {}
}

#[derive(Default)]
struct Outcome {
    message: String,
    target: Option<String>,
    role: Option<String>,
    text: Option<String>,
    shot_before: Option<Vec<u8>>,
}

struct Page {
    client: Arc<CdpClient>,
    target_id: String,
    port: u16,
}

pub struct BrowserTools {
    state_file: PathBuf,
    guard: Arc<dyn GuardContext>,
    sink: Arc<dyn ActionSink>,
    allow_file_urls: bool,
    page: Mutex<Option<Page>>,
    /// (document id at read time, refs). The id changes on every real navigation.
    refs: Mutex<(String, Vec<RefNode>)>,
    agent_active: Arc<AtomicBool>,
}

const STALE: &str = "ref obsoleta: a página mudou. Chame browser_read de novo e use as refs novas.";

impl BrowserTools {
    pub fn new(state_file: PathBuf, guard: Arc<dyn GuardContext>, sink: Arc<dyn ActionSink>) -> Self {
        Self {
            state_file, guard, sink, allow_file_urls: false,
            page: Mutex::new(None), refs: Mutex::new((String::new(), Vec::new())),
            agent_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Tests only: lets browser_open load the local fixture page.
    pub fn allow_file_urls(mut self) -> Self {
        self.allow_file_urls = true;
        self
    }

    // ── connection ────────────────────────────────────────────────────────
    async fn page(&self) -> Result<(Arc<CdpClient>, String, u16), String> {
        let mut g = self.page.lock().await;
        if let Some(p) = g.as_ref() {
            if !p.client.is_closed() {
                cdp::bring_to_front(p.port, &p.target_id, &p.client).await?;
                return Ok((p.client.clone(), p.target_id.clone(), p.port));
            }
        }
        let st = read_state(&self.state_file)
            .ok_or("o browser da Claudia ainda não está pronto (preparando ou não iniciou)")?;
        let t = cdp::first_page(st.port).await?;
        let client = Arc::new(CdpClient::connect(&t.ws_url).await?);
        for m in ["Page.enable", "DOM.enable", "Accessibility.enable"] {
            client.call(m, json!({})).await?;
        }
        if self.sink.enabled() {
            client.call("Network.enable", json!({})).await?;
            crate::browser::nav::spawn_tracker(&client, self.sink.clone(), self.agent_active.clone());
        }
        cdp::bring_to_front(st.port, &t.id, &client).await?;
        *g = Some(Page { client: client.clone(), target_id: t.id.clone(), port: st.port });
        Ok((client, t.id, st.port))
    }

    async fn eval(c: &CdpClient, expr: &str) -> Result<Value, String> {
        let r = c.call("Runtime.evaluate", json!({ "expression": expr, "returnByValue": true })).await?;
        Ok(r["result"]["value"].clone())
    }

    async fn doc_id(c: &CdpClient) -> String {
        Self::eval(c, "String(performance.timeOrigin)").await.ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
    }

    async fn resolve_ref(&self, c: &CdpClient, reference: &str) -> Result<RefNode, String> {
        let g = self.refs.lock().await;
        if g.0.is_empty() || g.0 != Self::doc_id(c).await {
            return Err(STALE.into());
        }
        refs::find(&g.1, reference).cloned().ok_or_else(|| format!("ref {reference} não existe. Chame browser_read."))
    }

    // ── screenshots (only when the Activity log is on) ───────────────────
    async fn capture(c: &CdpClient, clip: Option<Value>, scale: f64, quality: u32) -> Option<Vec<u8>> {
        let mut p = json!({ "format": "jpeg", "quality": quality });
        match clip {
            Some(mut cl) => { cl["scale"] = json!(scale); p["clip"] = cl; }
            None => {
                // Whole viewport at a reduced scale.
                let m = c.call("Page.getLayoutMetrics", json!({})).await.ok()?;
                let v = &m["cssVisualViewport"];
                p["clip"] = json!({ "x": 0, "y": 0, "width": v["clientWidth"], "height": v["clientHeight"], "scale": scale });
            }
        }
        let r = c.call("Page.captureScreenshot", p).await.ok()?;
        base64::engine::general_purpose::STANDARD.decode(r["data"].as_str()?).ok()
    }

    /// Scroll into view (trap 4), read the box, and take the "before" crop.
    async fn locate(&self, c: &CdpClient, n: &RefNode, shoot: bool) -> Result<(Option<Vec<u8>>, f64, f64), String> {
        let id = n.backend_node_id;
        c.call("DOM.scrollIntoViewIfNeeded", json!({ "backendNodeId": id })).await.map_err(|_| STALE.to_string())?;
        let m = c.call("DOM.getBoxModel", json!({ "backendNodeId": id })).await.map_err(|_| STALE.to_string())?;
        let q: Vec<f64> = m["model"]["content"].as_array().ok_or(STALE)?.iter().filter_map(Value::as_f64).collect();
        if q.len() < 8 { return Err(STALE.into()); }
        let xs = [q[0], q[2], q[4], q[6]];
        let ys = [q[1], q[3], q[5], q[7]];
        let (minx, maxx) = (xs.iter().cloned().fold(f64::MAX, f64::min), xs.iter().cloned().fold(f64::MIN, f64::max));
        let (miny, maxy) = (ys.iter().cloned().fold(f64::MAX, f64::min), ys.iter().cloned().fold(f64::MIN, f64::max));
        let shot = if shoot && self.sink.enabled() {
            let clip = json!({ "x": (minx - 40.0).max(0.0), "y": (miny - 40.0).max(0.0),
                               "width": (maxx - minx) + 80.0, "height": (maxy - miny) + 80.0 });
            Self::capture(c, Some(clip), 1.0, 70).await
        } else { None };
        Ok((shot, (minx + maxx) / 2.0, (miny + maxy) / 2.0))
    }

    async fn mouse_click(c: &CdpClient, x: f64, y: f64) -> Result<(), String> {
        c.call("Input.dispatchMouseEvent", json!({ "type": "mouseMoved", "x": x, "y": y })).await?;
        for t in ["mousePressed", "mouseReleased"] {
            c.call("Input.dispatchMouseEvent", json!({ "type": t, "x": x, "y": y, "button": "left", "clickCount": 1 })).await?;
        }
        Ok(())
    }

    /// Hidden radios/checkboxes sit under other elements (trap 3): click the label.
    async fn click_label_or_self(c: &CdpClient, backend_id: i64) -> Result<(), String> {
        let o = c.call("DOM.resolveNode", json!({ "backendNodeId": backend_id })).await.map_err(|_| STALE.to_string())?;
        let oid = o["object"]["objectId"].as_str().ok_or(STALE)?;
        c.call("Runtime.callFunctionOn", json!({
            "objectId": oid, "returnByValue": true,
            "functionDeclaration": "function(){const l=this.id?document.querySelector('label[for=\"'+CSS.escape(this.id)+'\"]'):null;(l||this).click();return l?'label':'self'}"
        })).await?;
        Ok(())
    }

    // ── recording ─────────────────────────────────────────────────────────
    async fn finish(&self, tool: &str, started: Instant, r: Result<Outcome, String>, c: Option<&CdpClient>) -> Result<String, String> {
        self.agent_active.store(false, Ordering::SeqCst);
        let mut rec = ActionRecord {
            tool: tool.to_string(), target: None, role: None, text: None, url: None,
            result: "ok".into(), error: None, duration_ms: started.elapsed().as_millis() as u64,
            shot_before: None, shot_after: None,
        };
        let out = match r {
            Ok(o) => {
                rec.target = o.target; rec.role = o.role; rec.text = o.text; rec.shot_before = o.shot_before;
                if let (true, Some(c)) = (self.sink.enabled(), c) {
                    rec.shot_after = Self::capture(c, None, 0.5, 60).await;
                }
                Ok(o.message)
            }
            Err(e) => {
                rec.result = if e.starts_with("RECUSADO") { "refused" } else { "error" }.into();
                rec.error = Some(e.clone());
                if let (true, Some(c)) = (self.sink.enabled(), c) {
                    rec.shot_after = Self::capture(c, None, 1.0, 80).await; // full-size on failure
                }
                Err(e)
            }
        };
        if self.sink.enabled() {
            if let Some(c) = c { rec.url = Self::eval(c, "location.href").await.ok().and_then(|v| v.as_str().map(String::from)); }
            self.sink.action(rec);
        }
        out
    }

    // ── tools ─────────────────────────────────────────────────────────────
    pub async fn open(&self, url: &str) -> Result<String, String> {
        let started = Instant::now();
        let lower = url.to_lowercase();
        let ok_scheme = lower.starts_with("http://") || lower.starts_with("https://") || (self.allow_file_urls && lower.starts_with("file://"));
        if !ok_scheme {
            return self.finish("browser_open", started, Err("só URLs http ou https são permitidas".into()), None).await;
        }
        self.agent_active.store(true, Ordering::SeqCst);
        let page = self.page().await;
        let (c, _, _) = match page { Ok(p) => p, Err(e) => return self.finish("browser_open", started, Err(e), None).await };
        let r = async {
            let nav = c.call("Page.navigate", json!({ "url": url })).await?;
            if let Some(err) = nav["errorText"].as_str() {
                return Err(format!("a navegação falhou: {err}"));
            }
            for _ in 0..100 {
                if Self::eval(&c, "document.readyState").await? == "complete" { break; }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            *self.refs.lock().await = (String::new(), Vec::new());
            Ok(Outcome { message: format!("Abriu {url}. Chame browser_read para ver a página."), target: Some(url.to_string()), ..Default::default() })
        }.await;
        self.finish("browser_open", started, r, Some(&c)).await
    }

    pub async fn read(&self, scope: Option<&str>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.page().await { Ok(p) => p, Err(e) => return self.finish("browser_read", started, Err(e), None).await };
        let r = async {
            let ax = c.call("Accessibility.getFullAXTree", json!({})).await?;
            let nodes = refs::build_refs(&ax, scope)?;
            let text = refs::render(&nodes);
            *self.refs.lock().await = (Self::doc_id(&c).await, nodes);
            Ok(Outcome { message: text, ..Default::default() })
        }.await;
        self.finish("browser_read", started, r, Some(&c)).await
    }

    pub async fn click(&self, reference: &str, confirm: Option<i64>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.page().await { Ok(p) => p, Err(e) => return self.finish("browser_click", started, Err(e), None).await };
        let r = async {
            let n = self.resolve_ref(&c, reference).await?;
            if let Verdict::Refuse(why) = guard::check_submit(&n.name, confirm, self.guard.as_ref()) {
                return Err(why);
            }
            let (shot, x, y) = self.locate(&c, &n, true).await?;
            if n.role == "radio" || n.role == "checkbox" {
                Self::click_label_or_self(&c, n.backend_node_id).await?;
            } else {
                Self::mouse_click(&c, x, y).await?;
            }
            Ok(Outcome { message: format!("Clicou em {} \"{}\".", n.role, n.name), target: Some(n.name), role: Some(n.role), shot_before: shot, ..Default::default() })
        }.await;
        self.finish("browser_click", started, r, Some(&c)).await
    }

    pub async fn type_text(&self, reference: &str, text: &str) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.page().await { Ok(p) => p, Err(e) => return self.finish("browser_type", started, Err(e), None).await };
        let r = async {
            let n = self.resolve_ref(&c, reference).await?;
            let o = c.call("DOM.resolveNode", json!({ "backendNodeId": n.backend_node_id })).await.map_err(|_| STALE.to_string())?;
            let oid = o["object"]["objectId"].as_str().ok_or(STALE)?.to_string();
            let t = c.call("Runtime.callFunctionOn", json!({ "objectId": oid, "returnByValue": true, "functionDeclaration": "function(){return this.type||''}" })).await?;
            let is_password = t["result"]["value"] == "password";
            let (shot, _, _) = self.locate(&c, &n, !is_password).await?;
            c.call("DOM.focus", json!({ "backendNodeId": n.backend_node_id })).await.map_err(|_| STALE.to_string())?;
            c.call("Input.insertText", json!({ "text": text })).await?;
            Ok(Outcome {
                message: format!("Escreveu em {} \"{}\".", n.role, n.name),
                target: Some(n.name), role: Some(n.role),
                // Passwords: neither the text nor the "before" crop are ever recorded.
                text: if is_password { None } else { Some(text.to_string()) },
                shot_before: if is_password { None } else { shot },
            })
        }.await;
        self.finish("browser_type", started, r, Some(&c)).await
    }

    pub async fn key(&self, key: &str, confirm: Option<i64>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.page().await { Ok(p) => p, Err(e) => return self.finish("browser_key", started, Err(e), None).await };
        let r = async {
            let (code, vk, text) = match key {
                "Enter" => ("Enter", 13, "\r"),
                "Tab" => ("Tab", 9, ""),
                "Escape" => ("Escape", 27, ""),
                " " | "Space" => ("Space", 32, " "),
                other => return Err(format!("tecla não suportada: {other} (use Enter, Tab, Escape ou Space)")),
            };
            if code == "Enter" || code == "Space" {
                // Keyboard must not bypass the guard: check what has the focus.
                let name = Self::eval(&c, "(()=>{const e=document.activeElement;return e?(e.getAttribute('aria-label')||e.innerText||e.value||''):''})()")
                    .await?.as_str().unwrap_or("").to_string();
                if let Verdict::Refuse(why) = guard::check_submit(&name, confirm, self.guard.as_ref()) {
                    return Err(why);
                }
            }
            let key_name = if code == "Space" { " " } else { code };
            for t in ["keyDown", "keyUp"] {
                let mut p = json!({ "type": t, "key": key_name, "code": code, "windowsVirtualKeyCode": vk });
                if t == "keyDown" && !text.is_empty() { p["text"] = json!(text); }
                c.call("Input.dispatchKeyEvent", p).await?;
            }
            Ok(Outcome { message: format!("Pressionou {key}."), target: Some(key.to_string()), ..Default::default() })
        }.await;
        self.finish("browser_key", started, r, Some(&c)).await
    }

    pub async fn crop(&self, x: f64, y: f64, w: f64, h: f64) -> Result<ToolOutput, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.page().await { Ok(p) => p, Err(e) => { self.finish("browser_crop", started, Err(e.clone()), None).await.ok(); return Err(e); } };
        let r = async {
            let s = c.call("Page.captureScreenshot", json!({ "format": "png", "clip": { "x": x, "y": y, "width": w, "height": h, "scale": 1 } })).await?;
            s["data"].as_str().map(String::from).ok_or_else(|| "o browser não devolveu imagem".to_string())
        }.await;
        match r {
            Ok(data) => {
                self.finish("browser_crop", started, Ok(Outcome { message: "recorte".into(), ..Default::default() }), Some(&c)).await.ok();
                Ok(ToolOutput::Image { data_base64: data, mime: "image/png".into() })
            }
            Err(e) => { self.finish("browser_crop", started, Err(e.clone()), Some(&c)).await.ok(); Err(e) }
        }
    }

    pub async fn click_at(&self, x: f64, y: f64, confirm: Option<i64>) -> Result<String, String> {
        let started = Instant::now();
        self.agent_active.store(true, Ordering::SeqCst);
        let (c, _, _) = match self.page().await { Ok(p) => p, Err(e) => return self.finish("browser_click_at", started, Err(e), None).await };
        let r = async {
            let expr = format!(
                "(()=>{{const e=document.elementFromPoint({x},{y});const b=e&&e.closest('button,a,[role=button],input[type=submit]');return b?(b.getAttribute('aria-label')||b.innerText||b.value||''):''}})()"
            );
            let name = Self::eval(&c, &expr).await?.as_str().unwrap_or("").to_string();
            if let Verdict::Refuse(why) = guard::check_submit(&name, confirm, self.guard.as_ref()) {
                return Err(why);
            }
            Self::mouse_click(&c, x, y).await?;
            Ok(Outcome { message: format!("Clicou em ({x}, {y})."), target: Some(name), ..Default::default() })
        }.await;
        self.finish("browser_click_at", started, r, Some(&c)).await
    }
}

/// Guard context backed by the main database. `site_allowed` is `true` until the
/// accounts cycle (cycle 2) adds per-site permissions.
pub struct DbGuardContext {
    pub data_dir: PathBuf,
}

impl GuardContext for DbGuardContext {
    fn vaga_status(&self, vaga_id: i64) -> Option<String> {
        let c = super::open_db(&self.data_dir).ok()?;
        c.query_row("SELECT status FROM vagas WHERE id = ?1", [vaga_id], |r| r.get(0)).ok()
    }
    fn applications_today(&self) -> i64 {
        super::open_db(&self.data_dir).ok().and_then(|c| crate::db::queries::candidaturas_hoje(&c).ok()).unwrap_or(0)
    }
    fn daily_limit(&self) -> i64 {
        std::fs::read_to_string(self.data_dir.join("disparo.json")).ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v["limite_diario"].as_i64())
            .unwrap_or(10)
    }
    fn site_allowed(&self, _url: &str) -> bool { true }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::test_support::{seed_db, temp_dir};

    #[test]
    fn db_context_reads_status_budget_and_limit() {
        let dir = temp_dir("guard-ctx");
        seed_db(&dir);
        let ctx = DbGuardContext { data_dir: dir.clone() };
        assert_eq!(ctx.vaga_status(1).as_deref(), Some("descoberta"));
        assert_eq!(ctx.vaga_status(999), None);
        assert_eq!(ctx.daily_limit(), 10, "default when disparo.json is absent");
        std::fs::write(dir.join("disparo.json"), r#"{"limite_diario": 3}"#).unwrap();
        assert_eq!(ctx.daily_limit(), 3);
        let conn = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        conn.execute("UPDATE vagas SET status='candidatando' WHERE id=1", []).unwrap();
        conn.execute("INSERT INTO candidaturas (vaga_id, enviada_em, pasta_arquivos, metodo) VALUES (1, datetime('now'), 'x', 'easy_apply')", []).unwrap();
        assert_eq!(ctx.vaga_status(1).as_deref(), Some("candidatando"));
        assert_eq!(ctx.applications_today(), 1);
    }
}
```
- [ ] **Step 4:** Criar o stub do rastreador em `browser/nav.rs` para o `spawn_tracker` existir (a lógica real chega na 3.4). Acrescentar ao `nav.rs`:
```rust
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use super::cdp::CdpClient;
use crate::mcp::tools::browser::ActionSink;

/// Wired for real in Task 3.4; until then a no-op so the tools compile.
pub fn spawn_tracker(_client: &Arc<CdpClient>, _sink: Arc<dyn ActionSink>, _agent_active: Arc<AtomicBool>) {}
```
- [ ] **Step 5: Run** `cargo test mcp::tools::browser` (1 teste de unidade) e depois o live:
```bash
cd src-tauri && CLAUDIA_TEST_CHROME="$TEMP/cft/chrome-win64/chrome.exe" cargo test --test browser_live -- --ignored --test-threads=1
```
Expected: todos passam (3 da 2.1 + 10 desta tarefa). Em falha, **não** relaxar o teste: corrigir o código ou registrar a causa.
- [ ] **Step 6: Commit** `git add src-tauri && git commit -m "feat(mcp): browser actions with ref-based clicks and the send guard"`

### Task 2.3: Ligar as tools ao servidor MCP

**Files:**
- Modify: `src-tauri/src/mcp/mod.rs` (flags e `McpConfig`)
- Modify: `src-tauri/src/mcp/server.rs` (tools `browser_*`)
- Modify: `src-tauri/src/commands/perfil.rs:460-491` (`write_mcp_config`: passar as duas flags novas)
- Create: `src-tauri/tests/mcp-browser-smoke.mjs`

**Interfaces:**
- Consumes: `BrowserTools`, `DbGuardContext`, `ToolOutput`.
- Produces: 7 tools no MCP: `browser_open(url)`, `browser_read(scope?)`, `browser_click(reference, confirm_submit?)`, `browser_type(reference, text)`, `browser_key(key, confirm_submit?)`, `browser_crop(x,y,width,height)`, `browser_click_at(x,y,confirm_submit?)`; flags `--browser-state <path>` e `--local-data-dir <path>`.

- [ ] **Step 1: Verificar a API de imagem do `rmcp` 2.2** (não confiar na memória):
```bash
grep -rn "pub fn image" ~/.cargo/registry/src/*/rmcp-2.2.0/src/model/ | head
grep -rn "pub fn text" ~/.cargo/registry/src/*/rmcp-2.2.0/src/model/content.rs | head
```
Anotar a assinatura exata. O código abaixo assume `ContentBlock::image(data: String, mime_type: String)`. Se for outra, ajustar **só** essa linha e registrar.
- [ ] **Step 2: `mcp/mod.rs`:** acrescentar a `McpConfig` os campos `pub browser_state: Option<PathBuf>` e `pub local_data_dir: Option<PathBuf>`; em `cli_main` ler `--browser-state` e `--local-data-dir` com `parse_flag_value(...).map(PathBuf::from)`; atualizar `cfg_for` nos `dispatch_tests` com `browser_state: None, local_data_dir: None`.
- [ ] **Step 3: `write_mcp_config` (`perfil.rs`):** depois do bloco de `--notify-port`, acrescentar:
```rust
    if let Ok(local) = app.path().app_local_data_dir() {
        args.push("--browser-state".to_string());
        args.push(crate::browser::state_file(&local).to_string_lossy().into_owned());
        args.push("--local-data-dir".to_string());
        args.push(local.to_string_lossy().into_owned());
    }
```
- [ ] **Step 4: `server.rs`:** acrescentar os structs de argumentos (mesmo estilo dos existentes) e o campo `browser`:
```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct OpenArgs {
    /// URL completa, http ou https
    url: String,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ReadArgs {
    /// Limita a leitura ao elemento cujo papel ou nome contém este texto, ex.: "dialog"
    scope: Option<String>,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ClickArgs {
    /// Referência do elemento, ex.: "e12", vinda de browser_read
    reference: String,
    /// Só para botões de enviar candidatura: ID da vaga (precisa estar em 'candidatando')
    confirm_submit: Option<i64>,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct TypeArgs {
    /// Referência do campo, ex.: "e5"
    reference: String,
    /// Texto a escrever
    text: String,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct KeyArgs {
    /// Enter, Tab, Escape ou Space
    key: String,
    /// Só se o foco estiver num botão de enviar candidatura: ID da vaga
    confirm_submit: Option<i64>,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CropArgs {
    /// Canto superior esquerdo do recorte, em pixels CSS
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ClickAtArgs {
    x: f64,
    y: f64,
    /// Só para botões de enviar candidatura: ID da vaga
    confirm_submit: Option<i64>,
}
```
`ClaudiaMcp` ganha `browser: Arc<crate::mcp::tools::browser::BrowserTools>`. Nos métodos do `#[tool_router] impl`:
```rust
    #[tool(description = "Abre uma URL (http ou https) no browser da Claudia. Depois chame browser_read para ver a página.")]
    async fn browser_open(&self, Parameters(a): Parameters<OpenArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.browser_text("browser_open", self.browser.open(&a.url).await)
    }

    #[tool(description = "Lista os elementos interativos da página, cada um com uma ref (e1, e2, ...) para usar em browser_click e browser_type. 'scope' limita a um bloco, ex.: \"dialog\" para só o modal.")]
    async fn browser_read(&self, Parameters(a): Parameters<ReadArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.browser_text("browser_read", self.browser.read(a.scope.as_deref()).await)
    }

    #[tool(description = "Clica no elemento pela ref de browser_read. Botões que enviam a candidatura são recusados a menos que confirm_submit traga o ID de uma vaga em 'candidatando', dentro do orçamento diário.")]
    async fn browser_click(&self, Parameters(a): Parameters<ClickArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.browser_text("browser_click", self.browser.click(&a.reference, a.confirm_submit).await)
    }

    #[tool(description = "Escreve texto no campo indicado pela ref de browser_read.")]
    async fn browser_type(&self, Parameters(a): Parameters<TypeArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.browser_text("browser_type", self.browser.type_text(&a.reference, &a.text).await)
    }

    #[tool(description = "Pressiona uma tecla (Enter, Tab, Escape ou Space) no elemento em foco. Enter ou Space sobre um botão de enviar candidatura passam pela mesma guarda do clique.")]
    async fn browser_key(&self, Parameters(a): Parameters<KeyArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.browser_text("browser_key", self.browser.key(&a.key, a.confirm_submit).await)
    }

    #[tool(description = "Devolve uma imagem de um recorte da página (x, y, largura, altura em pixels CSS). Use quando a árvore de browser_read não mostrar o que você precisa.")]
    async fn browser_crop(&self, Parameters(a): Parameters<CropArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        match self.browser.crop(a.x, a.y, a.width, a.height).await {
            Ok(crate::mcp::tools::browser::ToolOutput::Image { data_base64, mime }) => {
                crate::diagnostics::emit_event("mcp", "tool_ok", None, "browser_crop");
                Ok(CallToolResult::success(vec![ContentBlock::image(data_base64, mime)]))
            }
            Ok(crate::mcp::tools::browser::ToolOutput::Text(t)) => self.browser_text("browser_crop", Ok(t)),
            Err(e) => self.browser_text("browser_crop", Err(e)),
        }
    }

    #[tool(description = "Clica numa coordenada da página. Só quando a árvore de browser_read não expõe o elemento. A guarda de envio olha o elemento sob o ponto.")]
    async fn browser_click_at(&self, Parameters(a): Parameters<ClickAtArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
        self.browser_text("browser_click_at", self.browser.click_at(a.x, a.y, a.confirm_submit).await)
    }
```
E, no `impl ClaudiaMcp` (o bloco sem `#[tool_router]`), o helper:
```rust
    fn browser_text(&self, tool: &str, r: Result<String, String>) -> Result<CallToolResult, rmcp::ErrorData> {
        match &r {
            Ok(_) => crate::diagnostics::emit_event("mcp", "tool_ok", None, tool),
            Err(e) => crate::diagnostics::emit_event("mcp", "tool_err", None, &format!("{tool}: {e}")),
        }
        match r {
            Ok(m) => Ok(CallToolResult::success(vec![ContentBlock::text(m)])),
            // Tool-level error: the model sees the message and can self-correct.
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(e)])),
        }
    }
```
Em `run(cfg)` (fim de `server.rs`), construir o `BrowserTools` quando houver `cfg.browser_state`, com `Arc::new(DbGuardContext { data_dir: cfg.data_dir.clone() })` e `Arc::new(NoSink)` (o sink real entra na 3.3). Sem `--browser-state`, as tools devolvem o erro "o browser da Claudia ainda não está pronto": construir com um `PathBuf` inexistente, para o erro sair pelo caminho normal.
- [ ] **Step 5: Cliente MCP de fumaça** `src-tauri/tests/mcp-browser-smoke.mjs` (usado aqui e na Fase 6). Fala JSON-RPC por stdio com o binário:
```js
// Usage: node mcp-browser-smoke.mjs <claudia-rh.exe> <data-dir> <local-data-dir> [--url <url>]
// Starts `claudia-rh --mcp-serve`, lists tools and (with --url) opens the page and reads it.
import { spawn } from "node:child_process";
import { join } from "node:path";

const [exe, dataDir, localDir, ...rest] = process.argv.slice(2);
const urlIdx = rest.indexOf("--url");
const url = urlIdx >= 0 ? rest[urlIdx + 1] : null;
const child = spawn(exe, ["--mcp-serve", "--data-dir", dataDir, "--local-data-dir", localDir,
  "--browser-state", join(localDir, "browser", "state.json"), "--session-kind", "autonomous"], { stdio: ["pipe", "pipe", "inherit"] });

let id = 0; const waiting = new Map(); let buf = "";
child.stdout.on("data", (d) => {
  buf += d; let i;
  while ((i = buf.indexOf("\n")) >= 0) {
    const line = buf.slice(0, i); buf = buf.slice(i + 1);
    if (!line.trim()) continue;
    const m = JSON.parse(line);
    if (m.id && waiting.has(m.id)) { waiting.get(m.id)(m); waiting.delete(m.id); }
  }
});
const send = (method, params) => new Promise((res) => {
  const i = ++id; waiting.set(i, res);
  child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id: i, method, params }) + "\n");
});
const notify = (method, params) => child.stdin.write(JSON.stringify({ jsonrpc: "2.0", method, params }) + "\n");

const init = await send("initialize", { protocolVersion: "2025-03-26", capabilities: {}, clientInfo: { name: "smoke", version: "1" } });
notify("notifications/initialized", {});
const tools = (await send("tools/list", {})).result.tools.map((t) => t.name);
console.log("TOOLS", tools.join(","));
for (const need of ["browser_open", "browser_read", "browser_click", "browser_type", "browser_key", "browser_crop", "browser_click_at"]) {
  if (!tools.includes(need)) { console.log("FALTA", need); process.exitCode = 1; }
}
if (url) {
  const o = await send("tools/call", { name: "browser_open", arguments: { url } });
  console.log("OPEN", JSON.stringify(o.result.content?.[0]?.text ?? o.error));
  const r = await send("tools/call", { name: "browser_read", arguments: {} });
  console.log("READ\n" + (r.result.content?.[0]?.text ?? JSON.stringify(r.error)));
}
child.kill();
```
- [ ] **Step 6: Run** `cd src-tauri && cargo test` — Expected: tudo passa (incluindo os `dispatch_tests` atualizados). Depois `cargo build` e:
```bash
node tests/mcp-browser-smoke.mjs target/debug/claudia-rh.exe "$TEMP/mcp-data" "$TEMP/mcp-local"
```
Expected: linha `TOOLS` com as 7 tools `browser_*` e nenhuma linha `FALTA`.
- [ ] **Step 7: Commit** `git add src-tauri && git commit -m "feat(mcp): expose the browser tools over MCP"`

### Task 2.4: O app prepara o browser em segundo plano

**Files:**
- Create: `src-tauri/src/browser/service.rs`
- Modify: `src-tauri/src/browser/mod.rs` (`pub mod service;`)
- Modify: `src-tauri/src/lib.rs` (iniciar no `setup`, registrar o comando `browser_status`)

**Interfaces:**
- Consumes: `install::ensure_installed`, `theme::apply_theme`, `manager::{ensure_running, LaunchOptions}`, `pin::pin`.
- Produces: `service::Status{stage:String, downloaded:u64, total:Option<u64>, message:Option<String>}` (`Serialize`, `Clone`), `service::percent(&Status)->Option<u8>`, `service::start(app:AppHandle)` (async), comando Tauri `browser_status()->Status`; evento Tauri `browser-progress` com o `Status`. Estágios: `idle`, `downloading`, `launching`, `ready`, `error`.

- [ ] **Step 1: Testes que falham** (a lógica pura, `percent`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn s(downloaded: u64, total: Option<u64>) -> Status {
        Status { stage: "downloading".into(), downloaded, total, message: None }
    }
    #[test]
    fn percent_is_none_without_a_total_and_clamped_to_100() {
        assert_eq!(percent(&s(10, None)), None);
        assert_eq!(percent(&s(0, Some(200))), Some(0));
        assert_eq!(percent(&s(50, Some(200))), Some(25));
        assert_eq!(percent(&s(999, Some(200))), Some(100));
        assert_eq!(percent(&s(1, Some(0))), None);
    }
}
```
- [ ] **Step 2: Implementação:**
```rust
// Prepares the browser in the background right after the app opens: install,
// theme, launch. The UI follows along through the "browser-progress" event.
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Status {
    pub stage: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
}

impl Status {
    fn stage(stage: &str) -> Self {
        Self { stage: stage.into(), downloaded: 0, total: None, message: None }
    }
}

pub fn percent(s: &Status) -> Option<u8> {
    match s.total {
        Some(t) if t > 0 => Some(((s.downloaded.min(t) * 100) / t) as u8),
        _ => None,
    }
}

pub struct StatusState(pub Arc<Mutex<Status>>);

fn publish(app: &AppHandle, st: Status) {
    if let Some(state) = app.try_state::<StatusState>() {
        if let Ok(mut g) = state.0.lock() { *g = st.clone(); }
    }
    let _ = app.emit("browser-progress", st);
}

pub async fn start(app: AppHandle) {
    let local = match app.path().app_local_data_dir() {
        Ok(d) => d,
        Err(e) => return publish(&app, Status { message: Some(e.to_string()), ..Status::stage("error") }),
    };
    let pin = super::pin::pin();
    let root = super::browser_root(&local);
    if let Err(e) = std::fs::create_dir_all(&root) {
        return publish(&app, Status { message: Some(e.to_string()), ..Status::stage("error") });
    }

    publish(&app, Status::stage("downloading"));
    let app2 = app.clone();
    let cb = move |p: super::install::Progress| {
        publish(&app2, Status { stage: "downloading".into(), downloaded: p.downloaded, total: p.total, message: None });
    };
    let exe = match super::install::ensure_installed(&root, &pin.version, &pin.zip_url(), &pin.sha256, &cb).await {
        Ok(p) => p,
        Err(e) => return publish(&app, Status { message: Some(e.to_string()), ..Status::stage("error") }),
    };

    publish(&app, Status::stage("launching"));
    let profile = super::profile_dir(&local);
    if let Err(e) = super::theme::apply_theme(&profile) {
        log::warn!("browser theme not applied: {e}");
    }
    let opts = super::manager::LaunchOptions::visible(exe, profile);
    match super::manager::ensure_running(&opts, &pin.version, &super::state_file(&local)).await {
        Ok(_) => publish(&app, Status::stage("ready")),
        Err(e) => publish(&app, Status { message: Some(e), ..Status::stage("error") }),
    }
}

#[tauri::command]
pub fn browser_status(state: tauri::State<'_, StatusState>) -> Status {
    state.0.lock().map(|g| g.clone()).unwrap_or_else(|_| Status::stage("error"))
}
```
- [ ] **Step 3: `lib.rs`:** no `setup`, antes de `Ok(())`: `app.manage(browser::service::StatusState(Arc::new(Mutex::new(browser::service::Status { stage: "idle".into(), downloaded: 0, total: None, message: None }))));` e `tauri::async_runtime::spawn(browser::service::start(app.handle().clone()));`. Em `generate_handler![...]` acrescentar `browser::service::browser_status,`. (`Status::stage` é privado; para o `lib.rs` construir o "idle", tornar `pub fn idle() -> Status` em `service.rs` e usá-lo: `browser::service::Status::idle()`.)
- [ ] **Step 4: Run** `cargo test browser::service` (1 passa) e `cargo check`.
- [ ] **Step 5: Commit** `git add src-tauri && git commit -m "feat(browser): prepare the browser in the background at app start"`

### Task 2.5: Portão da Fase 2

- [ ] **Step 1:** `cargo test` + o live (`--ignored`, `CLAUDIA_TEST_CHROME` do 0.2) + `clippy` sem warning novo.
- [ ] **Step 2: Revisão de fase pelo Opus** do diff da fase: Review Focus 3 e 4 provados por teste que falharia se a guarda fosse removida (o revisor **remove a guarda numa cópia e confere que o teste quebra**); `ToolOutput`/imagem; o `state.json` lido a cada reconexão.

---

# FASE 3: Activity log

`activity_log` mora em `src-tauri/src/activity_log/`. Banco: `<dados-locais>/dev/activity-log.db`; capturas: `<dados-locais>/dev/capturas/`. O interruptor do modo desenvolvedor mora em `<dados>/dev-mode.json` (pasta de dados normal).

### Task 3.1: Banco, gravação, listagem, limpeza e interruptor

**Files:**
- Create: `src-tauri/src/activity_log/mod.rs`
- Modify: `src-tauri/src/lib.rs` (`pub mod activity_log;`)

**Interfaces:**
- Produces:
  - `ActivityLog::open(dir:&Path)->Result<Self,String>` (cria `dir`, abre `dir/activity-log.db`, WAL, `busy_timeout` 5 s, tabela `events`)
  - `NewEvent{kind,session_id:Option<i64>,vaga_id:Option<i64>,tool,target,role,text,url,result,error:Option<String>,duration_ms:Option<i64>,extra:Option<String>}` com `Default`
  - `record(&self,&NewEvent,shot_before:Option<&[u8]>,shot_after:Option<&[u8]>)->Result<i64,String>`
  - `EventRow` (`Serialize`) com os mesmos campos + `id`, `ts`, `shot_before:Option<String>`, `shot_after:Option<String>` (caminhos relativos a `dir`)
  - `Filter{session_id:Option<i64>,vaga_id:Option<i64>,kind:Option<String>,limit:i64}` com `Default` (limit 200)
  - `list(&self,&Filter)->Result<Vec<EventRow>,String>`, `get(&self,i64)->Result<Option<EventRow>,String>`, `clear(&self)->Result<(),String>`, `dir(&self)->&Path`
  - `activity_log::is_enabled(data_dir:&Path)->bool`, `activity_log::set_enabled(data_dir:&Path,bool)->Result<(),String>` (arquivo `dev-mode.json`)
- `kind` ∈ `action | browser | failure | navigation`.

- [ ] **Step 1: Testes que falham** (em `mod.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("claudia-al-{tag}-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }
    fn ev(kind: &str, tool: &str) -> NewEvent {
        NewEvent { kind: kind.into(), tool: Some(tool.into()), ..Default::default() }
    }

    #[test]
    fn dev_mode_is_off_by_default_and_toggles() {
        let d = temp("flag");
        assert!(!is_enabled(&d));
        set_enabled(&d, true).unwrap();
        assert!(is_enabled(&d));
        set_enabled(&d, false).unwrap();
        assert!(!is_enabled(&d));
    }

    #[test]
    fn a_garbage_flag_file_means_off() {
        let d = temp("flag-bad");
        std::fs::write(d.join("dev-mode.json"), "{ nope").unwrap();
        assert!(!is_enabled(&d));
    }

    #[test]
    fn records_and_lists_newest_first() {
        let log = ActivityLog::open(&temp("list")).unwrap();
        let a = log.record(&ev("action", "browser_open"), None, None).unwrap();
        let b = log.record(&ev("action", "browser_click"), None, None).unwrap();
        let rows = log.list(&Filter::default()).unwrap();
        assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![b, a]);
        assert_eq!(rows[0].tool.as_deref(), Some("browser_click"));
        assert!(!rows[0].ts.is_empty());
    }

    #[test]
    fn filters_by_kind_session_and_vaga() {
        let log = ActivityLog::open(&temp("filter")).unwrap();
        let mut e1 = ev("action", "t"); e1.session_id = Some(1); e1.vaga_id = Some(10);
        let mut e2 = ev("failure", "t"); e2.session_id = Some(2); e2.vaga_id = Some(10);
        log.record(&e1, None, None).unwrap();
        log.record(&e2, None, None).unwrap();
        assert_eq!(log.list(&Filter { kind: Some("failure".into()), ..Default::default() }).unwrap().len(), 1);
        assert_eq!(log.list(&Filter { session_id: Some(1), ..Default::default() }).unwrap().len(), 1);
        assert_eq!(log.list(&Filter { vaga_id: Some(10), ..Default::default() }).unwrap().len(), 2);
        assert_eq!(log.list(&Filter { limit: 1, ..Default::default() }).unwrap().len(), 1);
    }

    #[test]
    fn get_returns_the_row_or_none() {
        let log = ActivityLog::open(&temp("get")).unwrap();
        let id = log.record(&ev("action", "t"), None, None).unwrap();
        assert!(log.get(id).unwrap().is_some());
        assert!(log.get(9999).unwrap().is_none());
    }

    #[test]
    fn two_connections_can_write_at_once_without_lock_errors() {
        // Review: the MCP process and the app both write to the same file.
        let d = temp("conc");
        let (a, b) = (ActivityLog::open(&d).unwrap(), ActivityLog::open(&d).unwrap());
        let ha = std::thread::spawn(move || (0..50).for_each(|_| { a.record(&ev("action", "a"), None, None).unwrap(); }));
        let hb = std::thread::spawn(move || (0..50).for_each(|_| { b.record(&ev("action", "b"), None, None).unwrap(); }));
        ha.join().unwrap(); hb.join().unwrap();
        let log = ActivityLog::open(&d).unwrap();
        assert_eq!(log.list(&Filter { limit: 1000, ..Default::default() }).unwrap().len(), 100);
    }

    #[test]
    fn clear_empties_the_table() {
        let log = ActivityLog::open(&temp("clear")).unwrap();
        log.record(&ev("action", "t"), None, None).unwrap();
        log.clear().unwrap();
        assert!(log.list(&Filter::default()).unwrap().is_empty());
    }
}
```
(Os testes de capturas e retenção entram na 3.2, junto com `shots.rs`. Nesta tarefa `record` ignora os `Option<&[u8]>`: a 3.2 os grava.)

- [ ] **Step 2: Run** `cargo test activity_log` — Expected: FAIL (não compila).
- [ ] **Step 3: Implementação:**
```rust
// Dev-mode Activity log: what the agent did in the browser, with screenshots.
// A separate database from the vagas DB, never exposed through MCP.
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub mod shots;
pub mod sink;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    kind TEXT NOT NULL,
    session_id INTEGER, vaga_id INTEGER,
    tool TEXT, target TEXT, role TEXT, text TEXT, url TEXT,
    result TEXT, error TEXT, duration_ms INTEGER,
    shot_before TEXT, shot_after TEXT, extra TEXT
);
CREATE INDEX IF NOT EXISTS events_ts ON events(ts);
";

#[derive(Default, Clone, Debug)]
pub struct NewEvent {
    pub kind: String,
    pub session_id: Option<i64>,
    pub vaga_id: Option<i64>,
    pub tool: Option<String>,
    pub target: Option<String>,
    pub role: Option<String>,
    pub text: Option<String>,
    pub url: Option<String>,
    pub result: Option<String>,
    pub error: Option<String>,
    pub duration_ms: Option<i64>,
    pub extra: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct EventRow {
    pub id: i64,
    pub ts: String,
    pub kind: String,
    pub session_id: Option<i64>,
    pub vaga_id: Option<i64>,
    pub tool: Option<String>,
    pub target: Option<String>,
    pub role: Option<String>,
    pub text: Option<String>,
    pub url: Option<String>,
    pub result: Option<String>,
    pub error: Option<String>,
    pub duration_ms: Option<i64>,
    pub shot_before: Option<String>,
    pub shot_after: Option<String>,
    pub extra: Option<String>,
}

pub struct Filter {
    pub session_id: Option<i64>,
    pub vaga_id: Option<i64>,
    pub kind: Option<String>,
    pub limit: i64,
}

impl Default for Filter {
    fn default() -> Self {
        Self { session_id: None, vaga_id: None, kind: None, limit: 200 }
    }
}

pub struct ActivityLog {
    conn: Mutex<Connection>,
    dir: PathBuf,
}

const COLS: &str = "id, ts, kind, session_id, vaga_id, tool, target, role, text, url, result, error, duration_ms, shot_before, shot_after, extra";

fn row(r: &rusqlite::Row) -> rusqlite::Result<EventRow> {
    Ok(EventRow {
        id: r.get(0)?, ts: r.get(1)?, kind: r.get(2)?, session_id: r.get(3)?, vaga_id: r.get(4)?,
        tool: r.get(5)?, target: r.get(6)?, role: r.get(7)?, text: r.get(8)?, url: r.get(9)?,
        result: r.get(10)?, error: r.get(11)?, duration_ms: r.get(12)?,
        shot_before: r.get(13)?, shot_after: r.get(14)?, extra: r.get(15)?,
    })
}

impl ActivityLog {
    pub fn open(dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let conn = Connection::open(dir.join("activity-log.db")).map_err(|e| e.to_string())?;
        // WAL + a busy timeout: the MCP process and the app write to the same file.
        conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| e.to_string())?;
        conn.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(())).map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        Ok(Self { conn: Mutex::new(conn), dir: dir.to_path_buf() })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn record(&self, e: &NewEvent, shot_before: Option<&[u8]>, shot_after: Option<&[u8]>) -> Result<i64, String> {
        let id = {
            let c = self.conn.lock().map_err(|e| e.to_string())?;
            c.execute(
                "INSERT INTO events (kind, session_id, vaga_id, tool, target, role, text, url, result, error, duration_ms, extra)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![e.kind, e.session_id, e.vaga_id, e.tool, e.target, e.role, e.text, e.url, e.result, e.error, e.duration_ms, e.extra],
            ).map_err(|e| e.to_string())?;
            c.last_insert_rowid()
        };
        let _ = (shot_before, shot_after); // saved from Task 3.2 on
        Ok(id)
    }

    pub fn list(&self, f: &Filter) -> Result<Vec<EventRow>, String> {
        let c = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = c.prepare(&format!(
            "SELECT {COLS} FROM events
             WHERE (?1 IS NULL OR session_id = ?1) AND (?2 IS NULL OR vaga_id = ?2) AND (?3 IS NULL OR kind = ?3)
             ORDER BY id DESC LIMIT ?4"
        )).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![f.session_id, f.vaga_id, f.kind, f.limit], row).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get(&self, id: i64) -> Result<Option<EventRow>, String> {
        let c = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = c.prepare(&format!("SELECT {COLS} FROM events WHERE id = ?1")).map_err(|e| e.to_string())?;
        let mut rows = stmt.query_map([id], row).map_err(|e| e.to_string())?;
        rows.next().transpose().map_err(|e| e.to_string())
    }

    pub fn clear(&self) -> Result<(), String> {
        self.conn.lock().map_err(|e| e.to_string())?.execute("DELETE FROM events", []).map_err(|e| e.to_string())?;
        Ok(())
    }
}

// ── dev-mode switch (lives in the normal data dir, not in the log folder) ──
pub fn is_enabled(data_dir: &Path) -> bool {
    std::fs::read_to_string(data_dir.join("dev-mode.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v["enabled"].as_bool())
        .unwrap_or(false)
}

pub fn set_enabled(data_dir: &Path, on: bool) -> Result<(), String> {
    std::fs::write(data_dir.join("dev-mode.json"), serde_json::json!({ "enabled": on }).to_string()).map_err(|e| e.to_string())
}
```
Criar também `shots.rs` e `sink.rs` **vazios com um comentário de uma linha** (`// filled in Task 3.2.` / `// filled in Task 3.3.`) para o `mod` compilar. (Comentário real, não placeholder de lógica: os arquivos são preenchidos nas próximas tarefas, e os testes desta tarefa não dependem deles.)
- [ ] **Step 4:** `pub mod activity_log;` em `lib.rs`. **Run** `cargo test activity_log` — Expected: 7 passam.
- [ ] **Step 5: Commit** `git add src-tauri && git commit -m "feat(activity-log): separate database, filters and the dev-mode switch"`

### Task 3.2: Capturas e retenção

**Files:**
- Modify: `src-tauri/src/activity_log/shots.rs`
- Modify: `src-tauri/src/activity_log/mod.rs` (`record` passa a gravar as imagens; `purge`)

**Interfaces:**
- Produces: `shots::save(dir:&Path, event_id:i64, which:&str, bytes:&[u8])->Result<String,String>` (caminho **relativo** a `dir`, ex.: `capturas/20261002/17-before.jpg`); `ActivityLog::purge(&self, max_age_days:i64, max_bytes:u64)->Result<usize,String>` (devolve quantas linhas apagou); `shots::resolve(dir:&Path, rel:&str)->Result<PathBuf,String>` (rejeita `..`, caminhos absolutos e qualquer coisa fora de `capturas/`).

- [ ] **Step 1: Testes que falham** (acrescentar ao `mod tests` de `mod.rs`):
```rust
    #[test]
    fn screenshots_are_saved_and_referenced_by_relative_path() {
        let d = temp("shots");
        let log = ActivityLog::open(&d).unwrap();
        let id = log.record(&ev("action", "browser_click"), Some(b"BEFORE"), Some(b"AFTER")).unwrap();
        let row = log.get(id).unwrap().unwrap();
        let (b, a) = (row.shot_before.unwrap(), row.shot_after.unwrap());
        assert!(b.starts_with("capturas/") && b.ends_with("-before.jpg"), "{b}");
        assert_eq!(std::fs::read(d.join(&b)).unwrap(), b"BEFORE");
        assert_eq!(std::fs::read(d.join(&a)).unwrap(), b"AFTER");
    }

    #[test]
    fn an_event_without_screenshots_has_none() {
        let log = ActivityLog::open(&temp("noshots")).unwrap();
        let id = log.record(&ev("action", "t"), None, None).unwrap();
        let row = log.get(id).unwrap().unwrap();
        assert!(row.shot_before.is_none() && row.shot_after.is_none());
    }

    #[test]
    fn purge_by_age_removes_old_rows_and_their_files() {
        let d = temp("purge-age");
        let log = ActivityLog::open(&d).unwrap();
        let old = log.record(&ev("action", "old"), Some(b"x"), None).unwrap();
        let fresh = log.record(&ev("action", "fresh"), Some(b"y"), None).unwrap();
        let old_file = d.join(log.get(old).unwrap().unwrap().shot_before.unwrap());
        log.conn.lock().unwrap().execute("UPDATE events SET ts = datetime('now','-40 days') WHERE id = ?1", [old]).unwrap();
        assert_eq!(log.purge(30, u64::MAX).unwrap(), 1);
        assert!(log.get(old).unwrap().is_none());
        assert!(!old_file.exists());
        assert!(log.get(fresh).unwrap().is_some());
    }

    #[test]
    fn purge_by_size_drops_the_oldest_first() {
        let d = temp("purge-size");
        let log = ActivityLog::open(&d).unwrap();
        let blob = vec![0u8; 10_000];
        let ids: Vec<i64> = (0..4).map(|_| log.record(&ev("action", "t"), Some(&blob), None).unwrap()).collect();
        // Budget for ~2 images: the two oldest must go.
        assert_eq!(log.purge(30, 25_000).unwrap(), 2);
        assert!(log.get(ids[0]).unwrap().is_none() && log.get(ids[1]).unwrap().is_none());
        assert!(log.get(ids[2]).unwrap().is_some() && log.get(ids[3]).unwrap().is_some());
    }

    #[test]
    fn clear_also_removes_the_capture_files() {
        let d = temp("clear-files");
        let log = ActivityLog::open(&d).unwrap();
        log.record(&ev("action", "t"), Some(b"x"), None).unwrap();
        log.clear().unwrap();
        let left = std::fs::read_dir(d.join("capturas")).map(|r| r.count()).unwrap_or(0);
        assert_eq!(left, 0);
    }

    #[test]
    fn resolve_rejects_traversal_and_foreign_paths() {
        let d = temp("resolve");
        let log = ActivityLog::open(&d).unwrap();
        let id = log.record(&ev("action", "t"), Some(b"x"), None).unwrap();
        let ok = log.get(id).unwrap().unwrap().shot_before.unwrap();
        assert!(shots::resolve(&d, &ok).is_ok());
        for bad in ["../secret.txt", "capturas/../../x", "C:/Windows/win.ini", "/etc/passwd", "capturas/../activity-log.db"] {
            assert!(shots::resolve(&d, bad).is_err(), "{bad}");
        }
    }
```
- [ ] **Step 2: Run** `cargo test activity_log` — Expected: FAIL.
- [ ] **Step 3: `shots.rs`:**
```rust
// Screenshot files for the Activity log. The database stores relative paths.
use std::path::{Path, PathBuf};

const FOLDER: &str = "capturas";

pub fn save(dir: &Path, event_id: i64, which: &str, bytes: &[u8]) -> Result<String, String> {
    let day = chrono::Utc::now().format("%Y%m%d").to_string();
    let rel = format!("{FOLDER}/{day}/{event_id}-{which}.jpg");
    let full = dir.join(&rel);
    std::fs::create_dir_all(full.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&full, bytes).map_err(|e| e.to_string())?;
    Ok(rel)
}

/// Turns a stored relative path into a real path, refusing anything that could
/// escape the captures folder (the UI asks for images by this path).
pub fn resolve(dir: &Path, rel: &str) -> Result<PathBuf, String> {
    let p = Path::new(rel);
    let clean = !p.is_absolute()
        && rel.starts_with(&format!("{FOLDER}/"))
        && p.components().all(|c| matches!(c, std::path::Component::Normal(_)));
    if !clean {
        return Err("caminho de captura inválido".into());
    }
    Ok(dir.join(p))
}

pub fn remove(dir: &Path, rel: &str) {
    if let Ok(p) = resolve(dir, rel) {
        let _ = std::fs::remove_file(p);
    }
}

pub fn remove_all(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir.join(FOLDER));
}

pub fn size(dir: &Path, rel: &str) -> u64 {
    resolve(dir, rel).ok().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len()).unwrap_or(0)
}
```
- [ ] **Step 4: `mod.rs`:** em `record`, depois do `INSERT`, gravar as imagens e atualizar a linha (substituir a linha `let _ = (shot_before, shot_after);`):
```rust
        for (which, bytes) in [("before", shot_before), ("after", shot_after)] {
            if let Some(b) = bytes {
                // A failed image must never lose the event itself.
                if let Ok(rel) = shots::save(&self.dir, id, which, b) {
                    let col = if which == "before" { "shot_before" } else { "shot_after" };
                    let c = self.conn.lock().map_err(|e| e.to_string())?;
                    let _ = c.execute(&format!("UPDATE events SET {col} = ?1 WHERE id = ?2"), params![rel, id]);
                }
            }
        }
```
E acrescentar ao `impl ActivityLog`:
```rust
    pub fn clear(&self) -> Result<(), String> { /* replace the earlier body with: */
        self.conn.lock().map_err(|e| e.to_string())?.execute("DELETE FROM events", []).map_err(|e| e.to_string())?;
        shots::remove_all(&self.dir);
        Ok(())
    }

    /// Drops rows older than `max_age_days`, then the oldest rows until the
    /// captures fit in `max_bytes`. Returns how many rows were removed.
    pub fn purge(&self, max_age_days: i64, max_bytes: u64) -> Result<usize, String> {
        let mut removed = 0;
        let old: Vec<(i64, Option<String>, Option<String>)> = {
            let c = self.conn.lock().map_err(|e| e.to_string())?;
            let mut stmt = c.prepare("SELECT id, shot_before, shot_after FROM events WHERE ts < datetime('now', ?1)").map_err(|e| e.to_string())?;
            let r = stmt.query_map([format!("-{max_age_days} days")], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(|e| e.to_string())?;
            r.collect::<Result<_, _>>().map_err(|e| e.to_string())?
        };
        for (id, b, a) in old {
            self.drop_row(id, b.as_deref(), a.as_deref())?;
            removed += 1;
        }
        loop {
            let (total, oldest) = {
                let c = self.conn.lock().map_err(|e| e.to_string())?;
                let mut stmt = c.prepare("SELECT id, shot_before, shot_after FROM events ORDER BY id ASC").map_err(|e| e.to_string())?;
                let all: Vec<(i64, Option<String>, Option<String>)> = stmt
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(|e| e.to_string())?
                    .collect::<Result<_, _>>().map_err(|e| e.to_string())?;
                let total: u64 = all.iter().map(|(_, b, a)| {
                    b.as_deref().map_or(0, |p| shots::size(&self.dir, p)) + a.as_deref().map_or(0, |p| shots::size(&self.dir, p))
                }).sum();
                (total, all.into_iter().next())
            };
            match oldest {
                Some((id, b, a)) if total > max_bytes => { self.drop_row(id, b.as_deref(), a.as_deref())?; removed += 1; }
                _ => break,
            }
        }
        Ok(removed)
    }

    fn drop_row(&self, id: i64, before: Option<&str>, after: Option<&str>) -> Result<(), String> {
        for p in [before, after].into_iter().flatten() { shots::remove(&self.dir, p); }
        self.conn.lock().map_err(|e| e.to_string())?.execute("DELETE FROM events WHERE id = ?1", [id]).map_err(|e| e.to_string())?;
        Ok(())
    }
```
(Substituir o `clear` da 3.1 por esta versão; não ficar com dois.)
- [ ] **Step 5: Run** `cargo test activity_log` — Expected: 13 passam.
- [ ] **Step 6: Commit** `git add src-tauri && git commit -m "feat(activity-log): capture files, retention by age and size, safe path resolution"`

### Task 3.3: Gravar as ações das tools (e a redação de senha)

**Files:**
- Modify: `src-tauri/src/activity_log/sink.rs`
- Modify: `src-tauri/src/mcp/server.rs` (`run`: usar o sink real quando `--local-data-dir` existir)
- Modify: `src-tauri/tests/browser_live.rs` (testes de gravação)

**Interfaces:**
- Consumes: `ActionSink`, `ActionRecord`, `ActivityLog`, `is_enabled`.
- Produces: `sink::LogSink::new(data_dir:PathBuf, local_dir:PathBuf)->Self` implementando `ActionSink`: `enabled()` relê o `dev-mode.json` a cada chamada; `action()` converte para `NewEvent{kind: "action" ou "failure"}` (falha = `result != "ok"`), resolve `session_id`/`vaga_id` pelo banco principal, grava e engole qualquer erro (só `diagnostics::emit_event("activity_log","write_failed",...)`).

- [ ] **Step 1: Testes que falham** (em `sink.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity_log::{set_enabled, Filter};
    use crate::mcp::tools::browser::{ActionRecord, ActionSink};
    use crate::mcp::tools::test_support::{seed_db, temp_dir};

    fn rec(tool: &str, result: &str) -> ActionRecord {
        ActionRecord {
            tool: tool.into(), target: Some("Avançar".into()), role: Some("button".into()), text: None,
            url: Some("https://x/y".into()), result: result.into(),
            error: if result == "ok" { None } else { Some("boom".into()) },
            duration_ms: 12, shot_before: Some(b"B".to_vec()), shot_after: Some(b"A".to_vec()),
        }
    }

    #[test]
    fn off_by_default_writes_nothing_and_creates_no_files() {
        let data = temp_dir("sink-off-d"); let local = temp_dir("sink-off-l");
        let s = LogSink::new(data, local.clone());
        assert!(!s.enabled());
        s.action(rec("browser_click", "ok")); // even if called, must not create the dev folder
        assert!(!local.join("dev").exists());
    }

    #[test]
    fn when_on_it_records_actions_and_failures_with_shots() {
        let data = temp_dir("sink-on-d"); let local = temp_dir("sink-on-l");
        set_enabled(&data, true).unwrap();
        let s = LogSink::new(data, local.clone());
        assert!(s.enabled());
        s.action(rec("browser_click", "ok"));
        s.action(rec("browser_click", "error"));
        s.action(rec("browser_click", "refused"));
        let log = ActivityLog::open(&local.join("dev")).unwrap();
        let rows = log.list(&Filter::default()).unwrap();
        assert_eq!(rows.len(), 3);
        let kinds: Vec<&str> = rows.iter().map(|r| r.kind.as_str()).collect();
        assert_eq!(kinds, vec!["failure", "failure", "action"]); // newest first
        assert!(rows[2].shot_before.is_some() && rows[2].shot_after.is_some());
        assert_eq!(rows[0].result.as_deref(), Some("refused"));
    }

    #[test]
    fn session_and_vaga_come_from_the_main_database() {
        let data = temp_dir("sink-ctx-d"); let local = temp_dir("sink-ctx-l");
        seed_db(&data);
        let conn = rusqlite::Connection::open(data.join("claudia_rh.db")).unwrap();
        conn.execute("INSERT INTO sessoes (iniciada_em, motivo_disparo) VALUES (datetime('now'), 'manual')", []).unwrap();
        conn.execute("UPDATE vagas SET status='candidatando' WHERE id=1", []).unwrap();
        set_enabled(&data, true).unwrap();
        LogSink::new(data, local.clone()).action(rec("browser_open", "ok"));
        let row = ActivityLog::open(&local.join("dev")).unwrap().list(&Filter::default()).unwrap().remove(0);
        assert_eq!(row.vaga_id, Some(1));
        assert!(row.session_id.is_some());
    }

    #[test]
    fn a_write_failure_never_panics_and_never_propagates() {
        let data = temp_dir("sink-fail-d");
        set_enabled(&data, true).unwrap();
        // local dir is a *file*: opening the log folder must fail.
        let local = temp_dir("sink-fail-l").join("im-a-file");
        std::fs::write(&local, b"x").unwrap();
        LogSink::new(data, local).action(rec("browser_click", "ok")); // must simply return
    }
}
```
- [ ] **Step 2: Run** `cargo test activity_log::sink` — Expected: FAIL.
- [ ] **Step 3: `sink.rs`:**
```rust
// The ActionSink that feeds the Activity log. Best effort by design: a logging
// problem must never slow down or break what the agent is doing.
use std::path::PathBuf;

use super::{is_enabled, ActivityLog, NewEvent};
use crate::browser::nav::NavRecord;
use crate::mcp::tools::browser::{ActionRecord, ActionSink};

pub struct LogSink {
    data_dir: PathBuf,
    local_dir: PathBuf,
}

impl LogSink {
    pub fn new(data_dir: PathBuf, local_dir: PathBuf) -> Self {
        Self { data_dir, local_dir }
    }

    fn open(&self) -> Option<ActivityLog> {
        match ActivityLog::open(&self.local_dir.join("dev")) {
            Ok(l) => Some(l),
            Err(e) => {
                crate::diagnostics::emit_event("activity_log", "write_failed", None, &e);
                None
            }
        }
    }

    /// Current session and vaga, from the main database. Missing is fine.
    fn context(&self) -> (Option<i64>, Option<i64>) {
        let Ok(c) = rusqlite::Connection::open(self.data_dir.join("claudia_rh.db")) else { return (None, None) };
        let s = c.query_row("SELECT id FROM sessoes WHERE terminada_em IS NULL ORDER BY id DESC LIMIT 1", [], |r| r.get(0)).ok();
        let v = c.query_row("SELECT id FROM vagas WHERE status='candidatando' ORDER BY id DESC LIMIT 1", [], |r| r.get(0)).ok();
        (s, v)
    }
}

impl ActionSink for LogSink {
    fn enabled(&self) -> bool {
        is_enabled(&self.data_dir)
    }

    fn action(&self, rec: ActionRecord) {
        if !self.enabled() {
            return;
        }
        let Some(log) = self.open() else { return };
        let (session_id, vaga_id) = self.context();
        let ev = NewEvent {
            kind: if rec.result == "ok" { "action".into() } else { "failure".into() },
            session_id, vaga_id,
            tool: Some(rec.tool), target: rec.target, role: rec.role, text: rec.text, url: rec.url,
            result: Some(rec.result), error: rec.error, duration_ms: Some(rec.duration_ms as i64), extra: None,
        };
        if let Err(e) = log.record(&ev, rec.shot_before.as_deref(), rec.shot_after.as_deref()) {
            crate::diagnostics::emit_event("activity_log", "write_failed", None, &e);
        }
        let _ = log.purge(30, 1_073_741_824); // 30 days, 1 GB
    }

    fn navigation(&self, n: NavRecord) {
        if !self.enabled() {
            return;
        }
        let Some(log) = self.open() else { return };
        let (session_id, vaga_id) = self.context();
        let extra = serde_json::json!({ "status": n.status, "redirects": n.redirects, "initiator": n.initiator }).to_string();
        let ev = NewEvent {
            kind: "navigation".into(), session_id, vaga_id, tool: Some("navigation".into()),
            url: Some(n.url), result: n.status.map(|s| s.to_string()), extra: Some(extra), ..Default::default()
        };
        if let Err(e) = log.record(&ev, None, None) {
            crate::diagnostics::emit_event("activity_log", "write_failed", None, &e);
        }
    }
}
```
- [ ] **Step 4: `server.rs` `run`:** quando `cfg.local_data_dir` for `Some`, usar `Arc::new(LogSink::new(cfg.data_dir.clone(), local.clone()))` no lugar de `NoSink`.
- [ ] **Step 5: Teste live de gravação** (acrescentar a `browser_live.rs`), com um sink que guarda em memória:
```rust
use claudia_rh_lib::mcp::tools::browser::{ActionRecord, ActionSink};
use claudia_rh_lib::browser::nav::NavRecord;
use std::sync::Mutex;

struct Mem(Mutex<Vec<ActionRecord>>);
impl ActionSink for Mem {
    fn enabled(&self) -> bool { true }
    fn action(&self, r: ActionRecord) { self.0.lock().unwrap().push(r); }
    fn navigation(&self, _: NavRecord) {}
}

#[tokio::test] #[ignore]
async fn recorded_actions_carry_target_shots_and_redact_passwords() {
    let l = launch().await;
    let mem = Arc::new(Mem(Mutex::new(vec![])));
    let t = BrowserTools::new(l.profile.join("state.json"), Arc::new(AllowAll), mem.clone()).allow_file_urls();
    t.open(&fixture_url()).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.type_text(&ref_of(&list, "textbox", "Nome"), "Ana").await.unwrap();
    t.type_text(&ref_of(&list, "textbox", "Senha"), "segredo123").await.unwrap();
    let _ = t.click("e99", None).await; // unknown ref -> failure
    let recs = mem.0.lock().unwrap();
    let name = recs.iter().find(|r| r.tool == "browser_type" && r.target.as_deref() == Some("Nome")).unwrap();
    assert_eq!(name.text.as_deref(), Some("Ana"));
    assert!(name.shot_before.is_some() && name.shot_after.is_some(), "mutating action has before+after");
    let pw = recs.iter().find(|r| r.tool == "browser_type" && r.target.as_deref() == Some("Senha")).unwrap();
    assert!(pw.text.is_none(), "password text must never be recorded");
    assert!(pw.shot_before.is_none(), "password 'before' crop must never be recorded");
    let fail = recs.iter().find(|r| r.result != "ok").unwrap();
    assert!(fail.error.is_some() && fail.shot_after.is_some(), "failure keeps a full screenshot");
}
```
- [ ] **Step 6: Run** `cargo test activity_log` (4 novos passam) e o live com `--ignored`.
- [ ] **Step 7: Commit** `git add src-tauri && git commit -m "feat(activity-log): record browser actions with shots, redacting passwords"`

### Task 3.4: Histórico de navegação (`NavTracker`)

**Files:**
- Modify: `src-tauri/src/browser/nav.rs` (substituir o stub de `spawn_tracker` pela implementação)

**Interfaces:**
- Produces: `nav::NavTracker::new(agent_active:Arc<AtomicBool>)`, `on_event(&mut self, &Value)->Option<NavRecord>`; `nav::spawn_tracker(client:&Arc<CdpClient>, sink:Arc<dyn ActionSink>, agent_active:Arc<AtomicBool>)`.

- [ ] **Step 1: Testes que falham** (em `nav.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::Ordering;

    fn tracker() -> (NavTracker, Arc<AtomicBool>) {
        let a = Arc::new(AtomicBool::new(false));
        (NavTracker::new(a.clone()), a)
    }
    fn req(url: &str, redirect_from: Option<&str>) -> Value {
        let mut p = json!({ "type": "Document", "request": { "url": url } });
        if let Some(r) = redirect_from { p["redirectResponse"] = json!({ "url": r, "status": 302 }); }
        json!({ "method": "Network.requestWillBeSent", "params": p })
    }
    fn resp(url: &str, status: i64, kind: &str) -> Value {
        json!({ "method": "Network.responseReceived", "params": { "type": kind, "response": { "url": url, "status": status } } })
    }

    #[test]
    fn a_plain_navigation_is_one_record() {
        let (mut t, _) = tracker();
        assert!(t.on_event(&req("https://a/", None)).is_none());
        let r = t.on_event(&resp("https://a/", 200, "Document")).unwrap();
        assert_eq!((r.url.as_str(), r.status, r.redirects.len()), ("https://a/", Some(200), 0));
    }

    #[test]
    fn a_redirect_chain_is_collected_in_order() {
        let (mut t, _) = tracker();
        t.on_event(&req("https://a/", None));
        t.on_event(&req("https://b/", Some("https://a/")));
        t.on_event(&req("https://c/", Some("https://b/")));
        let r = t.on_event(&resp("https://c/", 200, "Document")).unwrap();
        assert_eq!(r.redirects, vec!["https://a/".to_string(), "https://b/".to_string()]);
        assert_eq!(r.url, "https://c/");
    }

    #[test]
    fn subresources_are_ignored() {
        let (mut t, _) = tracker();
        assert!(t.on_event(&resp("https://a/app.js", 200, "Script")).is_none());
        assert!(t.on_event(&resp("https://a/x.png", 200, "Image")).is_none());
    }

    #[test]
    fn the_initiator_follows_whether_a_tool_is_running() {
        let (mut t, active) = tracker();
        active.store(true, Ordering::SeqCst);
        assert_eq!(t.on_event(&resp("https://a/", 200, "Document")).unwrap().initiator, "agent");
        active.store(false, Ordering::SeqCst);
        assert_eq!(t.on_event(&resp("https://b/", 200, "Document")).unwrap().initiator, "outside-tool");
    }

    #[test]
    fn same_document_navigation_is_recorded_without_a_status() {
        let (mut t, _) = tracker();
        let ev = json!({ "method": "Page.navigatedWithinDocument", "params": { "url": "https://a/#step2" } });
        let r = t.on_event(&ev).unwrap();
        assert_eq!((r.url.as_str(), r.status), ("https://a/#step2", None));
    }

    #[test]
    fn unrelated_events_are_ignored() {
        let (mut t, _) = tracker();
        assert!(t.on_event(&json!({ "method": "Page.loadEventFired", "params": {} })).is_none());
    }
}
```
- [ ] **Step 2: Run** `cargo test browser::nav` — Expected: FAIL.
- [ ] **Step 3: Implementação** (substitui o stub; manter o `NavRecord`):
```rust
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::cdp::CdpClient;
use crate::mcp::tools::browser::ActionSink;

pub struct NavTracker {
    agent_active: Arc<AtomicBool>,
    redirects: Vec<String>,
}

impl NavTracker {
    pub fn new(agent_active: Arc<AtomicBool>) -> Self {
        Self { agent_active, redirects: Vec::new() }
    }

    fn initiator(&self) -> String {
        if self.agent_active.load(Ordering::SeqCst) { "agent" } else { "outside-tool" }.to_string()
    }

    pub fn on_event(&mut self, ev: &Value) -> Option<NavRecord> {
        let p = &ev["params"];
        match ev["method"].as_str()? {
            "Network.requestWillBeSent" if p["type"] == "Document" => {
                if let Some(from) = p["redirectResponse"]["url"].as_str() {
                    self.redirects.push(from.to_string());
                }
                None
            }
            "Network.responseReceived" if p["type"] == "Document" => Some(NavRecord {
                url: p["response"]["url"].as_str()?.to_string(),
                status: p["response"]["status"].as_i64(),
                redirects: std::mem::take(&mut self.redirects),
                initiator: self.initiator(),
            }),
            "Page.navigatedWithinDocument" => Some(NavRecord {
                url: p["url"].as_str()?.to_string(),
                status: None,
                redirects: vec![],
                initiator: self.initiator(),
            }),
            _ => None,
        }
    }
}

/// Feeds the sink from the CDP event stream. Runs inside the MCP process, so it
/// only sees navigations while an AI session is connected (documented limit).
pub fn spawn_tracker(client: &Arc<CdpClient>, sink: Arc<dyn ActionSink>, agent_active: Arc<AtomicBool>) {
    let mut rx = client.subscribe();
    tokio::spawn(async move {
        let mut tracker = NavTracker::new(agent_active);
        while let Ok(ev) = rx.recv().await {
            if let Some(rec) = tracker.on_event(&ev) {
                sink.navigation(rec);
            }
        }
    });
}
```
- [ ] **Step 4: Run** `cargo test browser::nav` — Expected: 6 passam. Depois `cargo check` (o `BrowserTools::page` já chama `spawn_tracker`).
- [ ] **Step 5: Teste live** (acrescentar a `browser_live.rs`): abrir a página com um sink em memória que também guarda `navigation`, e conferir que houve um `NavRecord` com `initiator == "agent"` e a URL do fixture. (Adaptar `Mem` para guardar `Vec<NavRecord>` num segundo `Mutex`.)
- [ ] **Step 6: Commit** `git add src-tauri && git commit -m "feat(browser): navigation history with redirect chains and initiator"`

### Task 3.5: Comandos Tauri, interruptor e visualizador

**Files:**
- Create: `src-tauri/src/commands/activity.rs`
- Modify: `src-tauri/src/commands/mod.rs` (declarar e reexportar `activity`, seguindo exatamente como `welcome` é declarado)
- Modify: `src-tauri/src/lib.rs` (registrar os comandos em `generate_handler!`)
- Create: `src/components/ActivityLogView.tsx`
- Modify: `src/components/Configuracoes.tsx` (seção "Modo desenvolvedor")
- Modify: `src/i18n/en.ts`, `src/i18n/pt.ts`

**Interfaces:**
- Produces (comandos): `activity_dev_mode_get()->bool`, `activity_dev_mode_set(on:bool)->Result<(),String>`, `activity_log_list(session_id:Option<i64>,vaga_id:Option<i64>,kind:Option<String>)->Result<Vec<EventRow>,String>`, `activity_log_get(id:i64)->Result<Option<EventRow>,String>`, `activity_log_image(path:String)->Result<String,String>` (data URL base64), `activity_log_clear()->Result<(),String>`.

- [ ] **Step 1: Teste que falha** (puro, em `activity.rs`): o resolvedor de imagem rejeita caminhos fora de `capturas/` e devolve um data URL para um arquivo válido:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_data_url_for_a_valid_capture_and_error_for_a_bad_path() {
        let d = std::env::temp_dir().join(format!("claudia-img-{}", std::process::id()));
        std::fs::create_dir_all(d.join("capturas/20261002")).unwrap();
        std::fs::write(d.join("capturas/20261002/1-before.jpg"), b"JPEGDATA").unwrap();
        let url = image_data_url(&d, "capturas/20261002/1-before.jpg").unwrap();
        assert!(url.starts_with("data:image/jpeg;base64,"), "{url}");
        assert!(image_data_url(&d, "../x.jpg").is_err());
        assert!(image_data_url(&d, "capturas/20261002/missing.jpg").is_err());
    }
}
```
- [ ] **Step 2: Implementação** (`activity.rs`):
```rust
// Tauri commands for the dev-mode Activity log. The UI is the only reader.
use base64::Engine;
use std::path::Path;
use tauri::{AppHandle, Manager};

use crate::activity_log::{self, shots, ActivityLog, EventRow, Filter};

fn log_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("dev"))
}

pub(crate) fn image_data_url(dir: &Path, rel: &str) -> Result<String, String> {
    let path = shots::resolve(dir, rel)?;
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

#[tauri::command]
pub fn activity_dev_mode_get(app: AppHandle) -> bool {
    app.path().app_data_dir().map(|d| activity_log::is_enabled(&d)).unwrap_or(false)
}

#[tauri::command]
pub fn activity_dev_mode_set(app: AppHandle, on: bool) -> Result<(), String> {
    let d = app.path().app_data_dir().map_err(|e| e.to_string())?;
    activity_log::set_enabled(&d, on)
}

#[tauri::command]
pub fn activity_log_list(app: AppHandle, session_id: Option<i64>, vaga_id: Option<i64>, kind: Option<String>) -> Result<Vec<EventRow>, String> {
    let log = ActivityLog::open(&log_dir(&app)?)?;
    log.list(&Filter { session_id, vaga_id, kind, limit: 500 })
}

#[tauri::command]
pub fn activity_log_get(app: AppHandle, id: i64) -> Result<Option<EventRow>, String> {
    ActivityLog::open(&log_dir(&app)?)?.get(id)
}

#[tauri::command]
pub fn activity_log_image(app: AppHandle, path: String) -> Result<String, String> {
    image_data_url(&log_dir(&app)?, &path)
}

#[tauri::command]
pub fn activity_log_clear(app: AppHandle) -> Result<(), String> {
    ActivityLog::open(&log_dir(&app)?)?.clear()
}
```
Registrar os 6 comandos em `generate_handler![...]` (`commands::activity::activity_dev_mode_get`, etc., no mesmo estilo dos outros).
- [ ] **Step 3: Chaves de i18n.** Abrir `src/i18n/en.ts` e `src/i18n/pt.ts`, achar o objeto `settings` e acrescentar (mesma estrutura nos dois; o tipo `Strings` vem de `en.ts`):
```ts
// en.ts (inside `settings`)
devMode: {
  title: "Developer mode",
  desc: "Records what the AI does in the browser (clicks, typing, pages, errors), with screenshots, on this computer only. Off by default.",
  toggle: "Record the Activity log",
  logTitle: "Activity log",
  refresh: "Refresh",
  clear: "Clear everything",
  clearConfirm: "Delete the whole Activity log and its screenshots? This cannot be undone.",
  empty: "Nothing recorded yet.",
  filterAll: "All",
  kindAction: "Action",
  kindFailure: "Failure",
  kindNavigation: "Navigation",
  kindBrowser: "Browser",
  colTime: "Time",
  colTool: "Step",
  colTarget: "Target",
  colResult: "Result",
  before: "Before",
  after: "After",
  details: "Details",
  limitNote: "Navigation by you is only recorded while an AI session is connected.",
},
// pt.ts (inside `settings`)
devMode: {
  title: "Modo desenvolvedor",
  desc: "Registra o que a IA faz no browser (cliques, digitação, páginas, erros), com prints, só neste computador. Desligado por padrão.",
  toggle: "Gravar o Activity log",
  logTitle: "Activity log",
  refresh: "Atualizar",
  clear: "Limpar tudo",
  clearConfirm: "Apagar todo o Activity log e os prints? Não dá para desfazer.",
  empty: "Nada registrado ainda.",
  filterAll: "Todos",
  kindAction: "Ação",
  kindFailure: "Falha",
  kindNavigation: "Navegação",
  kindBrowser: "Browser",
  colTime: "Hora",
  colTool: "Passo",
  colTarget: "Alvo",
  colResult: "Resultado",
  before: "Antes",
  after: "Depois",
  details: "Detalhes",
  limitNote: "A navegação feita por você só é gravada enquanto uma sessão de IA está conectada.",
},
```
- [ ] **Step 4: `ActivityLogView.tsx`:**
```tsx
import React, { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useT } from "../i18n";

interface EventRow {
  id: number; ts: string; kind: string; session_id: number | null; vaga_id: number | null;
  tool: string | null; target: string | null; role: string | null; text: string | null;
  url: string | null; result: string | null; error: string | null; duration_ms: number | null;
  shot_before: string | null; shot_after: string | null; extra: string | null;
}

const Shot: React.FC<{ path: string | null; label: string }> = ({ path, label }) => {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    setSrc(null);
    if (path) invoke<string>("activity_log_image", { path }).then(setSrc).catch(() => setSrc(null));
  }, [path]);
  if (!path) return null;
  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>{label}</div>
      {src ? <img src={src} alt={label} style={{ maxWidth: "100%", border: "1px solid var(--border)", borderRadius: 6 }} /> : null}
    </div>
  );
};

export const ActivityLogView: React.FC = () => {
  const t = useT().settings.devMode;
  const [rows, setRows] = useState<EventRow[]>([]);
  const [kind, setKind] = useState<string>("");
  const [open, setOpen] = useState<EventRow | null>(null);

  const load = useCallback(() => {
    invoke<EventRow[]>("activity_log_list", { sessionId: null, vagaId: null, kind: kind || null })
      .then(setRows).catch(() => setRows([]));
  }, [kind]);
  useEffect(load, [load]);

  const clear = async () => {
    if (!window.confirm(t.clearConfirm)) return;
    await invoke("activity_log_clear");
    setOpen(null);
    load();
  };

  const kinds: [string, string][] = [["", t.filterAll], ["action", t.kindAction], ["failure", t.kindFailure], ["navigation", t.kindNavigation], ["browser", t.kindBrowser]];

  return (
    <div style={{ marginTop: 12 }}>
      <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
        <strong>{t.logTitle}</strong>
        <select value={kind} onChange={(e) => setKind(e.target.value)}>
          {kinds.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
        </select>
        <button onClick={load}>{t.refresh}</button>
        <button onClick={clear}>{t.clear}</button>
      </div>
      <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>{t.limitNote}</div>
      {rows.length === 0 ? <div style={{ color: "var(--text-secondary)" }}>{t.empty}</div> : (
        <table style={{ width: "100%", fontSize: 13, borderCollapse: "collapse" }}>
          <thead><tr><th>{t.colTime}</th><th>{t.colTool}</th><th>{t.colTarget}</th><th>{t.colResult}</th></tr></thead>
          <tbody>
            {rows.map((r) => (
              <tr key={r.id} onClick={() => setOpen(r)} style={{ cursor: "pointer", background: r.kind === "failure" ? "var(--danger-soft, #fbe9e7)" : undefined }}>
                <td>{r.ts.replace("T", " ").replace("Z", "")}</td>
                <td>{r.tool}</td>
                <td>{r.target ?? r.url}</td>
                <td>{r.result}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {open && (
        <div style={{ marginTop: 12, padding: 12, border: "1px solid var(--border)", borderRadius: 8 }}>
          <div style={{ display: "flex", gap: 12 }}>
            <Shot path={open.shot_before} label={t.before} />
            <Shot path={open.shot_after} label={t.after} />
          </div>
          <div style={{ marginTop: 8, fontSize: 12 }}>{t.details}</div>
          <pre style={{ fontSize: 12, whiteSpace: "pre-wrap" }}>{JSON.stringify(open, null, 2)}</pre>
        </div>
      )}
    </div>
  );
};
```
(Usar as variáveis de CSS que `Configuracoes.tsx` já usa; se `--danger-soft` não existir, trocar por um token existente de `padrao-design-claudia-rh.md`.)
- [ ] **Step 5: `Configuracoes.tsx`:** importar `ActivityLogView`; adicionar estado `const [devMode, setDevMode] = useState(false);`; no `useEffect` inicial: `invoke<boolean>("activity_dev_mode_get").then((v) => setDevMode(!!v)).catch(() => {});`; antes do fechamento da página, uma seção seguindo exatamente o padrão da "Modo autónomo" (`SectionTitle` + descrição + `ToggleSwitch`):
```tsx
<div>
  <SectionTitle>{t.settings.devMode.title}</SectionTitle>
  <p>{t.settings.devMode.desc}</p>
  <ToggleSwitch
    checked={devMode}
    onChange={async (v: boolean) => { setDevMode(v); await invoke("activity_dev_mode_set", { on: v }); }}
  />
  {t.settings.devMode.toggle}
  {devMode && <ActivityLogView />}
</div>
```
(Ajustar as props do `ToggleSwitch` ao que o componente de fato recebe: ler `src/components/ui/ToggleSwitch.tsx` e copiar o uso da linha 169 de `Configuracoes.tsx`.)
- [ ] **Step 6: Run** `cd src-tauri && cargo test commands::activity` (1 passa), `cargo check`, depois `cd .. && npx tsc --noEmit` — Expected: sem erros.
- [ ] **Step 7: Commit** `git add -A && git commit -m "feat(activity-log): dev-mode switch and viewer with before/after screenshots"` (conferir com `git status` que só entraram arquivos desta tarefa).

### Task 3.6: Portão da Fase 3

- [ ] **Step 1:** `cargo test`, o live `--ignored`, `npx tsc --noEmit`, `npm run build`, `clippy` sem warning novo.
- [ ] **Step 2: Revisão de fase pelo Opus:** nenhum caminho de leitura do Activity log chega ao MCP (`grep -rn "activity" src-tauri/src/mcp/server.rs` não pode listar tool nenhuma); campo de senha sem texto nem print; falha de gravação não propaga; o interruptor desligado não cria arquivo.

---

# FASE 4: instalação e desinstalação

### Task 4.1: A interface mostra o preparo do browser

**Files:**
- Create: `src/components/BrowserBanner.tsx`
- Modify: `src/App.tsx` (renderizar o banner no topo)
- Modify: `src/i18n/en.ts`, `src/i18n/pt.ts`

- [ ] **Step 1: Chaves** em `en.ts`/`pt.ts` (novo objeto `browserSetup` no mesmo nível de `settings`):
```ts
// en
browserSetup: { downloading: "Preparing the Claudia browser", launching: "Starting the Claudia browser", ready: "Claudia browser ready", error: "Could not prepare the browser", retry: "Try again" },
// pt
browserSetup: { downloading: "Preparando o browser da Claudia", launching: "Iniciando o browser da Claudia", ready: "Browser da Claudia pronto", error: "Não foi possível preparar o browser", retry: "Tentar de novo" },
```
- [ ] **Step 2: `BrowserBanner.tsx`:**
```tsx
import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useT } from "../i18n";

interface Status { stage: string; downloaded: number; total: number | null; message: string | null }

export const BrowserBanner: React.FC = () => {
  const t = useT().browserSetup;
  const [s, setS] = useState<Status | null>(null);
  useEffect(() => {
    invoke<Status>("browser_status").then(setS).catch(() => {});
    const un = listen<Status>("browser-progress", (e) => setS(e.payload));
    return () => { un.then((f) => f()); };
  }, []);
  if (!s || s.stage === "idle" || s.stage === "ready") return null;
  const pct = s.total ? Math.min(100, Math.round((s.downloaded * 100) / s.total)) : null;
  const label = s.stage === "downloading" ? t.downloading : s.stage === "launching" ? t.launching : t.error;
  return (
    <div role="status" style={{ padding: "8px 16px", background: "var(--accent-soft)", color: "var(--accent-strong)", fontSize: 13 }}>
      {label}{pct !== null ? ` — ${pct}%` : ""}{s.stage === "error" && s.message ? `: ${s.message}` : ""}
    </div>
  );
};
```
- [ ] **Step 3:** Em `App.tsx`, importar e renderizar `<BrowserBanner />` logo abaixo do `TitleBar`.
- [ ] **Step 4: Run** `npx tsc --noEmit` e `npm run build` — Expected: limpos.
- [ ] **Step 5: Commit** `git add -A && git commit -m "feat(ui): show browser preparation progress"`

### Task 4.2: Limpeza do desinstalador

**Files:**
- Create: `src-tauri/src/browser/cleanup.rs`
- Modify: `src-tauri/src/browser/mod.rs` (`pub mod cleanup;`)
- Modify: `src-tauri/src/mcp/mod.rs` ou `main.rs` (o modo `--cleanup`; seguir como `--mcp-serve` é interceptado em `cli_main`)
- Modify: `src-tauri/tauri.conf.json` (`bundle.windows.nsis.installerHooks`) e criar `src-tauri/installer-hooks.nsh`, **conforme a Tarefa 0.4**

**Interfaces:**
- Produces: `cleanup::CleanupOptions{remove_browser:bool, remove_user_data:bool}`; `cleanup::clean(local:&Path, roaming:&Path, o:&CleanupOptions)->Result<Vec<PathBuf>,String>` (devolve o que apagou); modo CLI `claudia-rh --cleanup [--browser] [--user-data]`.

**Segurança:** este é código que **apaga pastas**. `clean` só apaga se o nome final do caminho for exatamente `io.github.johngabie.claudia-rh` (ou `browser` dentro de um caminho assim). Nunca rodar o desinstalador nem `--cleanup` real nesta máquina: os testes usam pastas temporárias com esse nome.

- [ ] **Step 1: Testes que falham:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const ID: &str = "io.github.johngabie.claudia-rh";

    fn tree(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("claudia-clean-{tag}-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let (local, roaming) = (base.join("Local").join(ID), base.join("Roaming").join(ID));
        std::fs::create_dir_all(local.join("browser").join("profile")).unwrap();
        std::fs::create_dir_all(local.join("dev")).unwrap();
        std::fs::create_dir_all(&roaming).unwrap();
        std::fs::write(local.join("browser").join("profile").join("Cookies"), b"c").unwrap();
        std::fs::write(local.join("dev").join("activity-log.db"), b"d").unwrap();
        std::fs::write(roaming.join("claudia_rh.db"), b"db").unwrap();
        std::fs::write(roaming.join("candidate_base.yaml"), b"y").unwrap();
        (base, local, roaming)
    }

    #[test]
    fn nothing_selected_deletes_nothing() {
        let (_b, local, roaming) = tree("none");
        let gone = clean(&local, &roaming, &CleanupOptions { remove_browser: false, remove_user_data: false }).unwrap();
        assert!(gone.is_empty());
        assert!(local.join("browser").exists() && roaming.join("claudia_rh.db").exists());
    }

    #[test]
    fn browser_only_keeps_the_user_data() {
        let (_b, local, roaming) = tree("browser");
        clean(&local, &roaming, &CleanupOptions { remove_browser: true, remove_user_data: false }).unwrap();
        assert!(!local.join("browser").exists());
        assert!(local.join("dev").exists(), "the activity log is user data, not browser");
        assert!(roaming.join("claudia_rh.db").exists());
    }

    #[test]
    fn user_data_removes_everything_including_the_browser() {
        let (_b, local, roaming) = tree("all");
        clean(&local, &roaming, &CleanupOptions { remove_browser: false, remove_user_data: true }).unwrap();
        assert!(!local.exists() && !roaming.exists());
    }

    #[test]
    fn refuses_a_folder_that_is_not_the_apps_own() {
        let base = std::env::temp_dir().join(format!("claudia-clean-bad-{}", std::process::id()));
        let (local, roaming) = (base.join("Documents"), base.join("Roaming").join(ID));
        std::fs::create_dir_all(&local).unwrap();
        std::fs::create_dir_all(&roaming).unwrap();
        std::fs::write(local.join("important.txt"), b"!").unwrap();
        let e = clean(&local, &roaming, &CleanupOptions { remove_browser: false, remove_user_data: true }).unwrap_err();
        assert!(e.contains("recus") || e.contains("refus"), "{e}");
        assert!(local.join("important.txt").exists(), "nothing may be deleted when a path looks wrong");
    }

    #[test]
    fn a_missing_folder_is_not_an_error() {
        let (_b, local, roaming) = tree("missing");
        std::fs::remove_dir_all(local.join("browser")).unwrap();
        assert!(clean(&local, &roaming, &CleanupOptions { remove_browser: true, remove_user_data: false }).is_ok());
    }
}
```
- [ ] **Step 2: Run** `cargo test browser::cleanup` — Expected: FAIL.
- [ ] **Step 3: Implementação:**
```rust
// Uninstaller cleanup. Deletes folders, so it is paranoid: it only touches
// paths whose last component is exactly the app identifier.
use std::path::{Path, PathBuf};

const APP_ID: &str = "io.github.johngabie.claudia-rh";

#[derive(Debug, Clone, Copy)]
pub struct CleanupOptions {
    pub remove_browser: bool,
    pub remove_user_data: bool,
}

fn is_app_dir(p: &Path) -> bool {
    p.file_name().map_or(false, |n| n == APP_ID)
}

/// Returns what it deleted. Validates *every* path before deleting *any*.
pub fn clean(local: &Path, roaming: &Path, o: &CleanupOptions) -> Result<Vec<PathBuf>, String> {
    if !(o.remove_browser || o.remove_user_data) {
        return Ok(vec![]);
    }
    if !is_app_dir(local) || !is_app_dir(roaming) {
        return Err(format!("recusado: as pastas não são as do app ({APP_ID}); nada foi apagado"));
    }
    let targets: Vec<PathBuf> = if o.remove_user_data {
        vec![local.to_path_buf(), roaming.to_path_buf()]
    } else {
        vec![super::browser_root(local)]
    };
    let mut gone = vec![];
    for t in targets {
        if t.exists() {
            std::fs::remove_dir_all(&t).map_err(|e| format!("não foi possível apagar {}: {e}", t.display()))?;
            gone.push(t);
        }
    }
    Ok(gone)
}

/// `claudia-rh --cleanup [--browser] [--user-data]`, run by the uninstaller.
/// Returns true when the process ran in this mode and the GUI must not start.
pub fn cli_main() -> bool {
    let args: Vec<String> = std::env::args().collect();
    if !args.iter().any(|a| a == "--cleanup") {
        return false;
    }
    let opts = CleanupOptions {
        remove_browser: args.iter().any(|a| a == "--browser"),
        remove_user_data: args.iter().any(|a| a == "--user-data"),
    };
    let dir = |var: &str| std::env::var(var).ok().map(|b| PathBuf::from(b).join(APP_ID));
    match (dir("LOCALAPPDATA"), dir("APPDATA")) {
        (Some(l), Some(r)) => match clean(&l, &r, &opts) {
            Ok(g) => println!("removed {} folder(s)", g.len()),
            Err(e) => { eprintln!("{e}"); std::process::exit(1); }
        },
        _ => { eprintln!("LOCALAPPDATA/APPDATA not set"); std::process::exit(1); }
    }
    true
}
```
- [ ] **Step 4:** Em `main.rs`, chamar `browser::cleanup::cli_main()` antes de `mcp::cli_main()` (ler o `main.rs` de 9 linhas e seguir o padrão: se devolver `true`, `return`). Adicionar `pub mod cleanup;` em `browser/mod.rs`.
- [ ] **Step 5: O gancho do NSIS** conforme a 0.4. Esqueleto de `src-tauri/installer-hooks.nsh` (ajustar os nomes ao que a 0.4 achou no template):
```nsis
!macro NSIS_HOOK_PREUNINSTALL
  ; Runs before the files are removed, so the app binary is still there.
  ${If} $DeleteAppDataCheckboxState == 1
    ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --cleanup --user-data'
  ${EndIf}
!macroend
```
e em `tauri.conf.json`, em `bundle.windows.nsis`: `"installerHooks": "installer-hooks.nsh"`. **Ponto com o usuário:** a caixa nativa cobre a opção 2 (apagar os meus dados, que inclui o browser). A opção 1 (**só o browser**) pede uma página própria de nsDialogs. Se a 0.4 concluiu que isso é caro, **parar e perguntar** ao usuário se aceita só a caixa nativa na primeira versão.
- [ ] **Step 6: Run** `cargo test browser::cleanup` (5 passam) e `cargo build`. **Não** gerar nem rodar instalador aqui: o teste de desinstalação real (6.3-c) só roda numa VM ou no Windows Sandbox, com autorização do usuário.
- [ ] **Step 7: Commit** `git add -A && git commit -m "feat(browser): uninstaller cleanup with a path-safety guard"`

### Task 4.3: Portão da Fase 4

- [ ] `cargo test`, `npx tsc --noEmit`, `npm run build`; revisão Opus focada em `cleanup.rs` (nenhum caminho que apague fora das pastas do app) e no gancho NSIS.

---

# FASE 5: cortar a dependência do Claude

Só começa depois da **rodada A da Fase 6.2** verde (o browser próprio funcionando de ponta a ponta com `--chrome` ainda no lugar como rede de segurança).

### Task 5.1: Remover `--chrome` e a checagem da extensão

**Files:**
- Modify: `src-tauri/src/commands/sessao.rs:250` e os testes das linhas ~287 e ~297
- Modify: `src-tauri/src/commands/perfil.rs:720` e o comentário das linhas ~647-649
- Modify: `src-tauri/src/commands/welcome.rs` (e `src/components/Welcome.tsx`, `src/i18n/*`)
- Modify: `CLAUDE.md` (a frase "Sessão de execução = `claude --chrome` no PTY")
- Create: `docs/superpowers/plans/2026-10-02-propostas-pos-corte.md`

- [ ] **Step 1: Teste que falha.** Em `sessao.rs`, o teste que hoje faz `assert!(args.contains(&"--chrome".to_string()))` passa a afirmar o contrário, e um teste novo no `perfil.rs` (onde há testes de argumentos) idem:
```rust
        assert!(!args.iter().any(|a| a == "--chrome"), "the extension is no longer part of the session");
```
Run `cargo test commands::sessao` — Expected: FAIL (ainda empurra `--chrome`).
- [ ] **Step 2:** Apagar `args.push("--chrome".to_string());` em `sessao.rs` e a linha `"--chrome",` em `perfil.rs`; atualizar os comentários que dizem que o `--chrome` é necessário.
- [ ] **Step 3: `welcome.rs`:** remover o campo `nativehost_chrome` de `SetupStatus` e o bloco do `winreg` que o calcula (a dependência `winreg` continua, o tema usa). Trocar por `browser_pronto: bool` lido de `StatusState` (`stage == "ready"`).
- [ ] **Step 4: Frontend.** `grep -rn "nativehost_chrome" src/` e remover o passo da extensão do `Welcome.tsx`; trocar por uma linha "Browser da Claudia" ligada a `browser_status`. Remover as chaves de i18n do passo da extensão **só se** não forem usadas em outro lugar (`grep`).
- [ ] **Step 5: Run** `cargo test`, `npx tsc --noEmit`, `npm run build` — Expected: limpos.
- [ ] **Step 6: Contradições com a arquitetura.** `grep -rn "\-\-chrome\|Claude in Chrome\|nativehost\|extens" docs/arquitetura-sistema-candidaturas.md CLAUDE.md README.md` e listar **cada ocorrência** em `docs/superpowers/plans/2026-10-02-propostas-pos-corte.md` com a frase atual e a proposta de nova frase. **Não editar** `docs/arquitetura-sistema-candidaturas.md` (o `CLAUDE.md` manda assinalar a contradição, não resolvê-la em silêncio). Editar o `CLAUDE.md` do repo na frase de "O que este repo já é" e na seção Diagnóstico se citar a extensão.
- [ ] **Step 7: Proposta de prompt.** Ler `src-tauri/src/prompt_sistema_runtime.md` (**só leitura**) e, no mesmo arquivo de propostas, escrever a lista das linhas que mencionam `--chrome`, "Claude in Chrome" ou ferramentas de browser do Claude, e o texto proposto usando `browser_open`, `browser_read`, `browser_click`, `browser_type`, `browser_key`, `browser_crop`, `browser_click_at` e `confirm_submit`. **O prompt só muda com o pedido explícito do usuário.**
- [ ] **Step 8: Commit** `git add -A && git commit -m "feat: drop --chrome and the extension check; the app browser replaces them"`

### Task 5.2: Portão da Fase 5

- [ ] `cargo test`, `npx tsc --noEmit`, `npm run build`. Revisão Opus: `grep -rn "\-\-chrome" src-tauri/src src` só pode aparecer em testes que afirmam a **ausência**, e a lista de contradições está completa.

---

# FASE 6: testes e validação

Todas as rodadas dos itens 6.2 e 6.3 registram evidência em `docs/pesquisa/2026-10-02-validacao-execucao.md`: o que foi aberto, o que foi chamado, o que a tela mostrou, com caminhos de imagem. Sem evidência, a rodada não conta.

### Task 6.1: Testes padrão

- [ ] **Step 1:** `cd src-tauri && cargo test` — Expected: tudo verde, contagem anotada.
- [ ] **Step 2:** `CLAUDIA_TEST_CHROME=<chrome da 0.2> cargo test --test browser_live -- --ignored --test-threads=1` — Expected: todos passam; **conferir que o número de testes executados é igual ao número de `#[ignore]` no arquivo** (`grep -c "#\[ignore\]" tests/browser_live.rs`), para nenhum teste ser pulado em silêncio.
- [ ] **Step 3:** `cargo clippy` sem warning novo contra o baseline; `cd .. && npx tsc --noEmit && npm run build`.
- [ ] **Step 4:** Os 17 testes da skill: `node .claude/skills/claudia-teste/scripts/test/regras.test.mjs` — Expected: 17/17.
- [ ] **Step 5: Evidência de falha provocada (Opus):** o revisor remove a guarda de `click` numa cópia, roda os testes live e confere que `submit_is_refused_without_confirmation_and_allowed_with_it` **falha**; repete com a redação de senha em `type_text`. Testes que não quebram sem o código que protegem não valem.

### Task 6.2: Testes no app rodando, com `/claudia-teste`, várias vezes

Invocar a skill `claudia-teste` (ela sobe o app com CDP no WebView2). **Três rodadas**, cada uma com o app reiniciado do zero e a pasta `%LOCALAPPDATA%\io.github.johngabie.claudia-rh\browser\` apagada **antes da rodada A** (para exercitar o download de verdade; o perfil do usuário em `%APPDATA%` não é tocado).

**Rodada A: preparo do browser.**
- [ ] Subir o app. Conferir pela skill que o banner "Preparando o browser da Claudia" aparece com a porcentagem subindo, e some ao fim.
- [ ] Conferir no disco: `chrome.exe` em `<dados-locais>/browser/<versão>/chrome-win64/`, `state.json` com porta e pid, o Chrome **visível** (screenshot do sistema com a janela do Chrome).
- [ ] Tema: screenshot do sistema, a barra do Chrome laranja (ou "não aplicável" conforme a 0.3).
- [ ] Fechar o app e abrir de novo: **não** baixa de novo, reaproveita o Chrome vivo ou relança em < 20 s.
- [ ] Matar o `chrome.exe` pelo Gerenciador: o app o religa (ver o evento no `events.jsonl`).

**Rodada B: ações pela rede MCP real, na página local.**
- [ ] Com o app aberto e o browser pronto: `node src-tauri/tests/mcp-browser-smoke.mjs <claudia-rh.exe> <dados> <dados-locais> --url file:///...` não serve (só http/https): subir `npx http-server`? **Não** (regra de não instalar). Servir o fixture com o servidor embutido do Node: `node -e "require('http').createServer((q,s)=>s.end(require('fs').readFileSync('src-tauri/tests/fixtures/apply-form.html'))).listen(8765)"` e usar `--url http://127.0.0.1:8765/`.
- [ ] Esperado: `TOOLS` lista as 7 tools; `OPEN` ok; `READ` lista `button "Avançar"`, `textbox "Nome"`, `radio "Sim"`.
- [ ] Estender o script (ou chamar por um segundo script de 30 linhas) para: clicar `Avançar` pela ref, ler de novo e ver `Avaliar`; clicar o botão "Enviar candidatura" **sem** `confirm_submit` e ver `RECUSADO`; escrever no campo Nome.

**Rodada C: tudo de novo, com o modo desenvolvedor ligado.**
- [ ] Ligar o modo na tela de Configurações pela skill (clique no `ToggleSwitch`). Repetir a rodada B. Abrir o visualizador e conferir que as linhas e os prints aparecem.

Repetir A, B e C **pelo menos duas vezes cada** (6 execuções), registrando qualquer diferença entre elas. Falha intermitente conta como falha.

### Task 6.3: Aplicações reais

**Regra: nenhuma candidatura real é enviada sem o "pode enviar" explícito do usuário naquela rodada.** O padrão é parar no último passo, sem clicar em "Enviar candidatura".

- [ ] **a) LinkedIn real, só leitura.** O usuário faz login uma vez na janela do browser da Claudia (a "memória das contas"). Depois: `browser_open` numa vaga real, `browser_read` e `browser_read scope=dialog` no modal do Easy Apply; conferir que os campos têm nome (comparar com a medição de referência: modal 4 313 tok, 12 de 12 campos nomeados).
- [ ] **b) LinkedIn real, até o último passo, sem enviar.** Percorrer Avançar → Avaliar até o botão "Enviar candidatura" aparecer; tentar clicá-lo **sem** `confirm_submit` e ver o `RECUSADO` na **página real**. Repetir **3 vezes** em 3 vagas diferentes.
- [ ] **c) Persistência da sessão.** Fechar o app, abrir de novo: continua logado no LinkedIn (a "memória das contas").
- [ ] **d) Outras plataformas.** Abrir 2 outras das 8 plataformas testadas em 2026-09-28 (ex.: Indeed e uma página de empresa), só leitura, e registrar se o `browser_read` devolve elementos úteis.
- [ ] **e) Desinstalação real, só em VM/Sandbox.** Rodar o instalador gerado no Windows Sandbox, instalar, usar, desinstalar marcando cada combinação de opções, e conferir o que sobrou em cada pasta. **Não** rodar nesta máquina. Se não houver Sandbox, registrar "não executado" e pedir ao usuário.
- [ ] **f) Uma candidatura real de ponta a ponta**, **só se o usuário disser sim** nesta rodada: com `confirm_submit` de uma vaga em `candidatando`, o botão passa, o LinkedIn confirma, a candidatura aparece no histórico e o Activity log tem a linha do envio.

### Task 6.4: Validação do Activity log

Com o modo desenvolvedor **ligado** durante as rodadas 6.2-C e 6.3, rodar `src-tauri/tests/activity-log-validate.mjs` (Node 24, `node:sqlite`) contra `<dados-locais>/dev/activity-log.db`:

- [ ] **Step 1: Criar o script:**
```js
// Usage: node activity-log-validate.mjs <local-data-dir> [--expect-calls N]
// Cross-checks the Activity log against the files and the rules in the spec.
import { DatabaseSync } from "node:sqlite";
import { existsSync, statSync, readdirSync } from "node:fs";
import { join } from "node:path";

const [local, ...rest] = process.argv.slice(2);
const expectIdx = rest.indexOf("--expect-calls");
const expected = expectIdx >= 0 ? Number(rest[expectIdx + 1]) : null;
const dev = join(local, "dev");
let failed = 0;
const check = (ok, msg) => { console.log((ok ? "OK    " : "FALHOU") + " " + msg); if (!ok) failed++; };

const db = new DatabaseSync(join(dev, "activity-log.db"), { readOnly: true });
const rows = db.prepare("SELECT * FROM events ORDER BY id").all();
check(rows.length > 0, `há ${rows.length} registros`);

// 1. every call the client made is in the log
const actions = rows.filter((r) => r.kind === "action" || r.kind === "failure");
if (expected !== null) check(actions.length === expected, `ações+falhas = ${actions.length}, esperado ${expected}`);

// 2. every mutating action has before+after files that exist and are non-empty
const mutating = actions.filter((r) => r.result === "ok" && ["browser_click", "browser_type"].includes(r.tool));
for (const r of mutating) {
  for (const col of ["shot_after"]) {
    const p = r[col] && join(dev, r[col]);
    check(p && existsSync(p) && statSync(p).size > 0, `#${r.id} ${r.tool} tem ${col} legível`);
  }
}
// "before" is required except for password fields (text is null there too)
for (const r of mutating.filter((x) => x.tool === "browser_click")) {
  const p = r.shot_before && join(dev, r.shot_before);
  check(p && existsSync(p) && statSync(p).size > 0, `#${r.id} clique tem shot_before legível`);
}

// 3. failures keep a screenshot and an error
for (const r of rows.filter((x) => x.kind === "failure")) {
  check(!!r.error, `#${r.id} falha tem erro`);
  const p = r.shot_after && join(dev, r.shot_after);
  check(p && existsSync(p) && statSync(p).size > 0, `#${r.id} falha tem print`);
}

// 4. passwords: no text, no 'before' shot for password targets
for (const r of rows.filter((x) => x.tool === "browser_type" && /senha|password/i.test(x.target ?? ""))) {
  check(r.text === null, `#${r.id} campo de senha sem texto gravado`);
  check(r.shot_before === null, `#${r.id} campo de senha sem recorte 'antes'`);
}

// 5. navigation rows carry a redirect list and an initiator
for (const r of rows.filter((x) => x.kind === "navigation")) {
  const e = JSON.parse(r.extra ?? "{}");
  check(Array.isArray(e.redirects) && ["agent", "outside-tool"].includes(e.initiator), `#${r.id} navegação com iniciador e redirects`);
}

// 6. nothing orphaned: every file on disk is referenced by a row
const referenced = new Set(rows.flatMap((r) => [r.shot_before, r.shot_after]).filter(Boolean).map((p) => p.replace(/\\/g, "/")));
const walk = (d) => readdirSync(d, { withFileTypes: true }).flatMap((e) => e.isDirectory() ? walk(join(d, e.name)) : [join(d, e.name)]);
const files = existsSync(join(dev, "capturas")) ? walk(join(dev, "capturas")) : [];
const orphans = files.filter((f) => !referenced.has(f.slice(dev.length + 1).replace(/\\/g, "/")));
check(orphans.length === 0, `sem arquivos órfãos (${orphans.length})`);

// 7. retention budget
const bytes = files.reduce((s, f) => s + statSync(f).size, 0);
check(bytes <= 1_073_741_824, `capturas ocupam ${(bytes / 1e6).toFixed(1)} MB (teto 1024 MB)`);

console.log(failed === 0 ? "\nTUDO OK" : `\n${failed} VERIFICAÇÕES FALHARAM`);
process.exit(failed === 0 ? 0 : 1);
```
- [ ] **Step 2: Cenários de validação**, cada um com registro de evidência:
  1. **Contagem:** executar N chamadas conhecidas pelo cliente MCP (o script de fumaça imprime quantas fez) e rodar `--expect-calls N`.
  2. **Modo desligado:** desligar o modo, repetir a rodada B e conferir que `activity-log.db` **não ganhou linhas** e `capturas/` **não ganhou arquivos** (comparar contagem antes/depois).
  3. **A IA não vê:** `tools/list` do MCP **não** tem nada com `activity`/`log`; `grep -rn "activity" src-tauri/src/mcp/server.rs` vazio; `grep -rln "activity-log" src-tauri/src/prompt_sistema_runtime.md src-tauri/src/prompt.rs` vazio.
  4. **Retenção:** com o app fechado, inserir 20 linhas com `ts` de 40 dias atrás e rodar uma ação com o modo ligado; conferir que as linhas antigas e os arquivos sumiram.
  5. **Concorrência:** abrir **dois** processos MCP ao mesmo tempo (dois `mcp-browser-smoke.mjs` em paralelo) com o modo ligado, 30 chamadas cada: nenhum erro de "database is locked" e 60 linhas no total.
  6. **Falha de gravação:** tornar `dev/` somente leitura (`attrib +R` na pasta e no `.db`), rodar uma ação: a ação **tem de funcionar** e o erro aparecer só no `events.jsonl`.
  7. **Visualizador:** na tela, filtrar por "Falha" e abrir uma linha: antes e depois aparecem; "Limpar tudo" apaga linhas **e** arquivos (rodar o validador depois: zero linhas, zero órfãos).

### Task 6.5: Revisão final e encerramento

- [ ] **Step 1: Revisão Opus do branch inteiro** (`git diff dev...HEAD`) contra a spec §3 a §8: cada decisão da tabela da §3 tem código e teste; Global Constraints cumpridas; nenhum placeholder; nenhuma mudança em `prompt_sistema_runtime.md`; `identifier` intocado.
- [ ] **Step 2: Fechar issues.** Pela skill `linear-claudia`: **não** fechar a CLA-31 sozinho; juntar a evidência (contagens de teste, rodadas A/B/C, a rodada real) e **perguntar** ao usuário, citando-a.
- [ ] **Step 3:** Invocar `superpowers:finishing-a-development-branch`. A integração (merge em `dev`, push) é **decisão do usuário**.

---

## Auto-revisão do plano

**Cobertura da spec:** §3 decisões: Chrome próprio (1.1, 1.2, 2.4), Chrome for Testing + hash fixado (0.1, 1.1, 1.2), janela visível (1.7), memória de contas/perfil persistente (1.7, 2.4, 6.3-c), tema (0.3, 1.6), cliente CDP próprio (1.3), guarda (1.5, 2.2), Activity log dev-only (3.1 a 3.5). §4 arquitetura: processos e `state.json` (1.7, 2.3, 2.4); `dispatch` assíncrono e imagem (2.2, 2.3). §5.1 a 5.7: 1.2, 1.7, 1.3, 2.2/2.3, 1.5/2.2, 3.x, 4.2. §6 remoção do Claude: 5.1 (prompt só como proposta). §7 testes: 4 camadas (unidade 1.x, instalação 1.2, integração 2.1/2.2, visível 6.2). §8 hipóteses: Fase 0. Nada da spec fica sem tarefa.

**Placeholders:** `browser-pin.json` e `browser-theme.json` têm valores definidos na Fase 0 (dados, não código); nenhuma tarefa de código contém "TBD". A única lacuna deliberada é a API de imagem do `rmcp` (2.3, Step 1), com verificação explícita e instrução de ajuste de uma linha.

**Consistência de tipos:** `GuardContext`/`check_submit`/`Verdict` (1.5) usados em 2.2; `ActionRecord`/`ActionSink`/`NavRecord`/`ToolOutput` definidos em 2.2 e consumidos em 3.3/3.4; `ActivityLog::{open,record,list,get,clear,purge}` (3.1/3.2) usados em 3.3 e 3.5; `Status`/`StatusState` (2.4) usados em 4.1 e 5.1; `LaunchOptions::visible`/`headless_for_tests` (1.7) usados em 2.1 e 2.4.

**Riscos que o plano assume e a Fase 0 testa:** sessão do LinkedIn no Chrome for Testing (0.2), tema sem clique (0.3), licença (0.1), desinstalador (0.4). Se qualquer um falhar, o portão da 0.5 devolve a decisão ao usuário antes de gastar a Fase 1.
