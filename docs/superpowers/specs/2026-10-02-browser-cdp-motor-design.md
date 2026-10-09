# Spec: motor do browser da Claudia (CDP direto)

> Spec de design. Não é o plano de implementação. Ciclo 1 de 3 (ver seção 9).

> **Status: implementada em `feat/browser-cdp-mcp`.** Pendências: `docs/superpowers/evidencias/2026-10-feat-browser-cdp-mcp.md`.

**Data:** 2026-10-02
**Branch:** `feat/browser-cdp-mcp` (a partir de `feat/exec-prototipo-playwright`, que contém `dev` mais a pesquisa)
**Issue:** CLA-31
**Base:** `docs/pesquisa/2026-09-28-DECISAO-browser.md` e `2026-09-28-proposta-cdp-direto.md`

## 1. Problema

Hoje a sessão de execução é `claude --chrome`: o controle do browser depende do Claude Code e da extensão Claude in Chrome. Isso custa três coisas:

- trocar de conta de IA mexe no plugin e desestabiliza quando o token estoura;
- usuários pediram outras IAs (Grok, Gemini, OpenCode), e `--chrome` é a única flag sem equivalente;
- o setup tem ~12 passos, dois deles só para a extensão (instalar e registrar o native messaging host, `welcome.rs:19`).

A pesquisa de 2026-09-28 provou que o CDP direto resolve: uma candidatura real foi submetida só com ele.

## 2. Objetivo

O app passa a **fornecer** o controle do browser, como tools no servidor MCP que ele já tem. Qualquer IA que fale MCP dirige o browser. Nenhum passo novo de instalação para o usuário.

## 3. Decisões tomadas

| Decisão | Escolha | Por quê |
|---|---|---|
| Quem é o browser | **Um Chrome nosso**, mesmo que o usuário tenha Chrome | O perfil já teria de ser dedicado (Chrome ≥136 ignora a porta de debug no perfil padrão); versão fixa evita que um update do Chrome quebre o CDP; um caminho a testar em vez de dois |
| De onde vem | **Chrome for Testing**, baixado na primeira vez que o app precisa | Build oficial da Google para automação; instalador do Claudia continua pequeno; a licença do Chrome provavelmente não permite redistribuir dentro do nosso instalador (a confirmar no plano) |
| Integridade do download | Versão e SHA-256 **fixados no código do app** | O JSON da Google (`last-known-good-versions-with-downloads.json`) não traz hash; fixar também dá versão travada |
| Janela | **Sempre visível** em execução. Headless só em teste | O usuário tem de ver o que a IA faz |
| Memória das contas | Perfil persistente em `<dados-locais>/browser/profile/`. **Só sessão (cookies); nenhuma senha passa pelo app** | Igual a um browser normal; sem cofre de senhas num app que entrega ações a agentes de terceiros |
| Tema | Laranja `#D97757` (`--accent` do design) como padrão | Pedido do usuário | posteriormente modificar o icone do browser para uma versao todo laranja para fazer a diferença
| Cliente CDP | **Escrito por nós** sobre WebSocket, ~12 comandos. Sem `chromiumoxide` | O protótipo inteiro é um cliente de 40 linhas; uma dependência grande a menos; testável com servidor WebSocket falso |
| Envio de candidatura | O agente decide sozinho, **dentro da guarda** (seção 5.5) | Princípio do projeto: autonomia com contenção no código, sem confirmação humana por rodada |
| Activity log | **Sempre ligado** (decisão de 2026-10-03, substitui "só em modo desenvolvedor"), banco separado, sem tool MCP para ler | Debug e desenvolvimento; o usuário pode ver em Configurações |

## 4. Arquitetura

**Onde o browser mora.** Tudo do browser (o Chrome extraído e o perfil com as sessões) fica numa pasta só, `<dados-locais>/browser/`, onde `<dados-locais>` é `%LOCALAPPDATA%\io.github.johngabie.claudia-rh` (`app_local_data_dir` do Tauri). Não é a pasta de instalação, porque um update do app a substitui e levaria as contas logadas, e `Program Files` exigiria administrador. Também não é `%APPDATA%` (Roaming), que perfis corporativos sincronizam pela rede, e não deve carregar 150 MB de binário e cookies. O instalador e o app **não escrevem no registro** nem tocam no Chrome do usuário por causa do browser. A remoção no desinstalador está na seção 5.7.

O servidor MCP **não vive dentro do app**. É o mesmo binário em `--mcp-serve`, subido por cada sessão de IA via stdio, e morre com ela. O browser tem de sobreviver às sessões, logo o dono é o app.

```
App Tauri (processo principal)                 Processo MCP (um por sessão de IA)
 ├ BrowserManager (Rust)                         ├ tools browser_* (novas)
 │   baixa, lança, vigia, religa o Chrome        │     └ cliente CDP ──────────┐
 │   perfil persistente, tema, janela visível    ├ tools existentes (vagas, …)  │
 ├ Guarda de envio (estado: orçamento, vaga)     │                              │
 └ UI (React): só mostra estado                  │                              │
        │                                                                       │
        └──── lança ──► Chrome (--remote-debugging-port, --user-data-dir) ◄─ WebSocket
```

- O app escreve a porta de debug num arquivo em `<dados-locais>/browser/state.json` e passa o caminho dele ao processo MCP por `--browser-state` (a porta muda quando o Chrome religa; o MCP relê o arquivo a cada conexão), como já faz com `--notify-port`.
- O processo MCP é só cliente: conecta, executa o comando CDP, devolve.
- `dispatch` (`src-tauri/src/mcp/mod.rs`) hoje é síncrono e devolve `String`. As tools de browser precisam de um caminho assíncrono e de resposta com **imagem** (`ContentBlock` de imagem do `rmcp`). Isso é mudança estrutural e entra no plano como tarefa própria.

## 5. Componentes

### 5.1 `browser::instalacao`

Garante que o Chrome existe em `<dados-locais>/browser/<versão>/chrome-win64/chrome.exe`.

1. Se existe e o `chrome.exe --version` bate com a versão fixada: usa.
2. Senão: baixa `https://storage.googleapis.com/chrome-for-testing-public/<versão>/win64/chrome-win64.zip`, confere o SHA-256 fixado, extrai numa pasta temporária e **só então** renomeia para a pasta final (nunca deixa instalação pela metade).
3. Emite progresso (bytes, percentual) para a UI por evento Tauri.

Erros tratados com mensagem própria: sem rede, hash diferente, zip corrompido, disco cheio, antivírus bloqueando o `.exe`. O download é retomável do zero (apaga a pasta temporária e recomeça).

A URL base e a pasta de destino são **parâmetros**, para o teste apontar para um servidor local.

### 5.2 `browser::gestor` (BrowserManager)

- Lança o Chrome com `--remote-debugging-port=<livre>`, `--user-data-dir=<dados-locais>/browser/profile`, `--no-first-run`, `--no-default-browser-check`. **Nunca** `--headless` em execução.
- Escolhe porta livre e grava `state.json`. A porta 9222 costuma ser do Chrome pessoal do usuário (já aconteceu no teste do WebView2).
- Vigia: se o processo morre, religa e emite evento. O activity log registra a queda.
- **Aba ativa:** antes de qualquer ação, `/json/activate/{id}` e poll de `document.visibilityState` até `visible`. Aba congelada engole input em silêncio (armadilha 1).
- **Tema:** grava a cor no `Preferences` do perfil **antes** da primeira abertura. Se o Chrome não aceitar, o plano tem tarefa de investigar a alternativa (política `BrowserThemeColor` por registro em HKCU, que não exige administrador).

### 5.3 `browser::cdp`

Cliente fino sobre `tokio-tungstenite`. Faz: conectar ao alvo `type=page`, enviar comando com id, casar resposta, receber eventos. Comandos usados:

`Page.navigate`, `Accessibility.getFullAXTree`, `DOM.resolveNode`, `DOM.scrollIntoViewIfNeeded`, `DOM.getBoxModel`, `Input.dispatchMouseEvent`, `Input.dispatchKeyEvent`, `Input.insertText`, `Page.captureScreenshot` (com `clip`), `Runtime.evaluate`, `Network.getCookies` (usado no ciclo 2).

### 5.4 Tools MCP (o que o agente enxerga)

| Tool | O que faz |
|---|---|
| `browser_open(url)` | navega. Recusa domínio de conta não logada ou não permitida (ciclo 2) |
| `browser_read(recorte?)` | árvore de acessibilidade com **refs** (`e12`) por elemento; `recorte` limita a um nó, ex.: o modal |
| `browser_click(ref, confirm_submit?)` | resolve, rola até o elemento, clica. Se o alvo é um `<input>` escondido (rádio), clica no `label[for]` |
| `browser_type(ref, texto)` | foca e insere o texto |
| `browser_key(tecla)` | Enter, Tab, Esc |
| `browser_crop(x,y,w,h)` | imagem do recorte (~390 tok) |
| `browser_click_at(x,y)` | clique por coordenada, para o que a árvore não expõe |

As armadilhas 1 (aba congelada), 2 (modal sem `role=dialog` no DOM, só na árvore de acessibilidade), 3 (rádio com `opacity:0`), 4 (`getBoxModel` exige `scrollIntoViewIfNeeded`) e 5 (botão de avanço que muda de nome) ficam **dentro do app**. O agente não precisa saber delas.

`browser_contas` pertence ao ciclo 2.

**Tools acrescentadas depois desta tabela:** `browser_text` e `browser_scroll` (spec `2026-10-03-mcp-leitura-robusta-design.md`) e `browser_upload` (spec `2026-10-04-anexar-curriculo-design.md`), num total de dez tools `browser_*`. `browser_key` aceita também Enter e Espaço sobre botões, sob a mesma guarda. **Diálogos nativos** (`alert`, `confirm`, `prompt`, `beforeunload`) são respondidos pelo app numa tarefa própria (política P2, spec `2026-10-08-pendencias-finais-browser-design.md` §6), e a queda da aba vira erro da tool.

### 5.5 Guarda de envio

`browser_click`, `browser_click_at` e `browser_key` (Enter e Espaço) verificam o nome acessível do alvo. Se parece envio ("Enviar candidatura", "Submit application", "Send application", "Postuler", etc., lista em `src-tauri/submit-patterns.txt`, testada), o clique é **recusado** a menos que a chamada traga `confirm_submit: <vaga_id>` e as três condições valham:

1. a vaga está em `candidatando`;
2. o orçamento diário não estourou;
3. o site está logado e permitido (ciclo 2; até lá, só 1 e 2).

Recusa devolve texto claro ao agente, que se corrige. A guarda é código do app, não instrução de prompt. Em clique por coordenada, a guarda lê o elemento sob o ponto antes de clicar.

**Regra completa implementada (2026-10-07/08, spec `2026-10-08-pendencias-finais-browser-design.md`, opção C), em quatro degraus, em `browser/guard.rs` (`classify_submit`):**
1. o nome casa com um padrão de envio do arquivo → exige a confirmação e **gasta** o único envio da vaga no dia (nomes fracos, linhas `^` como "Apply", só contam quando o botão envia um `<form>`);
2. alvo estrutural (iframe, botão sem nome, nó ilegível) → exige a confirmação, **sem gastar**;
3. o alvo envia um `<form>` e o nome **não** é de navegação (linhas `!` do arquivo: "Próximo", "Next", "Voltar"...; igualdade exata, nunca substring; "Salvar" fica de fora) → exige a confirmação, **sem gastar** (a decisão G2: o envio real continua liberado depois);
4. o resto passa.

**Risco aceito:** uma página hostil pode enviar pelos próprios handlers de JavaScript, e o botão feito só de JavaScript fora de `<form>` e o envio implícito de campo único não são pegos. A **porta de debug** do Chrome (`--remote-debugging-port`, local) **não tem autenticação**: qualquer processo local pode falar CDP com o browser e contornar a guarda. É a contenção da CLA-30 (confinamento da sessão) que reduz o risco; a opção de interceptar o envio pela rede (D) fica no Linear.

### 5.6 Activity log (sempre ligado)

Serve para **debug e desenvolvimento**: reconstruir, passo a passo e com imagem, o que o agente fez no browser e onde algo quebrou. Não é auditoria para o usuário final.

**Sempre ligado.** *Decisão de 2026-10-03: não há mais o interruptor "Modo desenvolvedor" nem o arquivo `dev-mode.json`; um `dev-mode.json` deixado por versão anterior é ignorado.* Tudo abaixo é gravado em toda ação do agente. Nas Configurações há a seção "Activity log" com um botão **"Ver registros"** que mostra a lista, e o "Limpar tudo". Como grava sempre, a limpeza por idade e tamanho (30 dias, 1 GB) roda uma vez ao iniciar o processo e depois no máximo a cada 10 minutos, não a cada ação, e uma falha ao gravar nunca atrasa nem derruba a ação do browser.

**Onde fica.** `<dados-locais>/dev/`, **fora** da pasta onde as sessões de IA trabalham e separado do banco principal:

```
dev/
  activity-log.db          banco SQLite só do activity log (nunca o banco das vagas)
  capturas/          os prints, um arquivo por imagem; o banco guarda só o caminho
```

Nenhuma tool MCP lê isso, e nenhum prompt o menciona. Só a UI do app, com o modo ligado, lê.

**Quatro tipos de registro, na mesma linha do tempo:**

| Tipo | O que grava |
|---|---|
| **Ação** | instante, sessão de IA, vaga (se houver), tool (`browser_click`, …), alvo por **nome acessível** e papel (não só o ref `e12`), texto digitado, URL da página, resultado (ok, erro, ou recusado pela guarda, com o motivo), duração |
| **Evento do browser** | **implementado (2026-10-07, decisão G4 de 2026-10-08), 11 eventos** em `activity_log::BrowserEvent`: `download_started`, `installed`, `install_failed`, `launched`, `launch_failed`, `gone` (queda: duas verificações seguidas falharam), `relaunched`, `not_relaunched` (caiu mas nenhuma sessão precisa dele), `gave_up` (fechou várias vezes seguidas), `tab_crashed` e `dialog` (tipo, texto cortado em 200 caracteres, URL, aceito ou não, e a tool em andamento; o valor padrão de um `prompt()` nunca entra). **Fora, por decisão:** progresso do download e "aba ativada". Gravados com `kind = browser` e filtráveis na tela |
| **Falha** | erro do CDP com a mensagem original, exceção do app, e a captura completa da tela no instante |
| **Navegação** | o histórico de navegação: cada página que o browser carregou, em ordem. Instante, URL, título, código HTTP, tempo de carga, cadeia de redirecionamentos e **quem navegou** (agente via `browser_open`, clique em link, redirecionamento do site, ou o próprio usuário na janela) |

**Histórico de navegação.** O app escuta os eventos do CDP (`Page.frameNavigated`, `Page.navigatedWithinDocument`, `Network.responseReceived` do frame principal) e grava uma linha por navegação, ligada à sessão de IA e à vaga em curso. Distinguir quem navegou é possível porque o app sabe quando uma tool `browser_*` está em andamento: navegação durante a tool é do agente; fora dela, é do site ou do usuário. Isso responde perguntas de debug como "por que o agente caiu na página de login?" ou "em que ponto o LinkedIn nos redirecionou?". Navegações do **usuário** (por exemplo, o login que ele faz na janela) são gravadas só como URL e instante, sem print. O histórico que o próprio Chrome guarda no perfil continua existindo, mas é do browser e não separa agente de usuário; o activity log é a fonte para o debug. Na tela, entra como uma visão "navegação" ao lado da lista de ações, com o caminho de URLs em ordem.

**Prints.** Em toda ação que muda a página (clicar, escrever, navegar, tecla) o app grava duas imagens:
- **antes:** recorte em volta do alvo, com margem, para ver *no que* ele clicou;
- **depois:** a janela inteira, reduzida, para ver *o que aconteceu*.

Ação só de leitura (`browser_read`, `browser_crop`) não gera print, pois a resposta dela já é a imagem ou o texto. Em **falha**, a janela inteira em tamanho cheio. As imagens são JPEG (qualidade ~70) para caber muito sem pesar. Tirar o print usa o mesmo `Page.captureScreenshot` com `clip` que o recorte já usa; nenhum comando CDP novo.

**Privacidade e limites:**
- O texto digitado é gravado; **campos de senha nunca**, e o recorte "antes" de um campo de senha **não é gravado**.
- Os prints podem mostrar dados pessoais (CV, telefone, mensagens). Ficam só nesta máquina, só com o modo ligado, e saem na retenção automática (30 dias / 1 GB) e no desinstalador (seção 5.7). Não há "Limpar tudo" na tela: ela é somente leitura.
- Retenção de 30 dias **e** teto de espaço (1 GB): ao passar, apaga o mais antigo primeiro. Rodar com o modo ligado por semanas não pode encher o disco.
- Uma falha ao gravar o activity log (disco cheio, banco travado) **nunca derruba nem atrasa** a ação do agente: o erro é só registrado no `diagnostics` já existente e a ação segue.

**Quem escreve.** O processo MCP grava as ações (é onde elas acontecem); o app grava os eventos do browser. Os dois escrevem no mesmo `activity-log.db`, com SQLite em modo WAL para os dois processos não se bloquearem.

**A tela (decisão de 2026-10-03: tela própria, somente leitura).** Nas Configurações a seção "Activity log" tem a descrição e o botão "Ver registros", que abre uma **tela própria** (não é aba da sidebar) com o botão "← Voltar" no topo, que volta às Configurações. A tela só lê: não existe "Limpar tudo" nem nenhuma outra escrita; quem apaga é a retenção automática. Lista cronológica (mais novo primeiro, 100 por página com "Carregar mais") com filtro por tipo (ação, falha, navegação, browser), por sessão e por vaga. Cada linha mostra hora local, passo, alvo, resultado e duração, com as falhas em destaque (cor de perigo, só como estado). Clicar numa linha abre, ao lado, o print antes e o depois, os campos nomeados (hora, tipo, passo, alvo, papel, texto, endereço, resultado, erro, duração, sessão, vaga) e os dados técnicos recolhidos num bloco secundário; os prints só carregam do registro aberto. A linha do tempo mais rica por vaga fica para o ciclo 2.

**Limite honesto:** o agente roda com permissões liberadas e pode ler qualquer arquivo que o usuário do Windows leia. Estar fora da pasta de trabalho, não ser citado em prompt ou tool e o confinamento da CLA-30 reduzem o acesso acidental, mas não o impedem por completo. Aqui isso é aceitável porque o conteúdo é o do próprio usuário, na máquina dele.

**Testes:** gravação e leitura do banco; retenção por idade e por teto de espaço (apaga o mais antigo); campo de senha sem texto nem print; falha de gravação não afeta a ação; modo desligado não cria arquivo algum; dois processos escrevendo ao mesmo tempo; navegação do agente, de redirecionamento e do usuário classificadas certo na página local de teste; cadeia de redirecionamentos completa; o recorte "antes" cai sobre o alvo certo, conferido contra a página local de teste.

### 5.7 Desinstalação

> **Decisão do usuário (D5, 2026-10-07): a Tarefa 4.2 (limpeza própria na desinstalação) foi cortada.** Vale **só a caixa padrão "Delete the application data" do instalador NSIS do Tauri**, que remove as duas pastas do `identifier` (`%APPDATA%` e `%LOCALAPPDATA%`). **Não existe** a opção "só o browser" nem a função `limpar_dados(opcoes)` descrita abaixo; código de limpeza novo só com spec própria e teste no Sandbox. A desinstalação real nunca foi testada nesta máquina (só Sandbox ou VM). O texto abaixo é a proposta original, mantida como histórico.

O desinstalador oferece duas opções, **ambas desmarcadas por padrão**, porque apagar é irreversível:

1. **Remover o browser e as sessões:** apaga `<dados-locais>/browser/` (o Chrome e as contas logadas).
2. **Remover também os meus dados:** apaga o que o app guarda em `%APPDATA%\io.github.johngabie.claudia-rh` (perfil do candidato, banco de vagas e candidaturas, arquivos gerados, diagnósticos) e `<dados-locais>` inteiro, incluindo o activity log. A opção 2 inclui a 1. O texto da opção diz o que será apagado, em português claro, e que não há como desfazer.

Sem nenhuma marcada, desinstalar não apaga nada do usuário (comportamento de hoje).

- A lógica de apagar vive numa função do app, `limpar_dados(opcoes)`, que recebe as pastas por parâmetro. O desinstalador só a chama. Assim ela é testável contra pastas temporárias, sem desinstalar nada de verdade.
- O Tauri v2 já tem uma caixa "Delete the application data" no desinstalador NSIS. A primeira tarefa desta parte do plano é **verificar o que ela já apaga** e completar com um hook de desinstalação (`NSIS_HOOK_POSTUNINSTALL`) só o que faltar, em vez de reescrever o desinstalador.
- Teste automatizado: instalar em modo silencioso, criar arquivos em ambas as pastas, desinstalar com cada combinação de opções e conferir o que sobrou. O `identifier` Tauri não muda (regra dura do projeto); os caminhos dependem dele.

## 6. Remoção da dependência do Claude

- Sessão de execução deixa de passar `--chrome` (`src-tauri/src/commands/sessao.rs:250`) e a sessão de perfil idem (`commands/perfil.rs:720`). O MCP do app é registrado na sessão, como já é.
- A checagem da extensão e do native messaging host em `welcome.rs` sai do setup.
- `prompt_sistema_runtime.md` precisa mudar: o agente deixa de "dirigir o browser" e passa a "pedir ações pelas tools `browser_*`". **Esse arquivo não pode ser editado sem pedido explícito do usuário**, então a mudança é proposta no plano e só aplicada com aprovação.
- O `stash@{0}` guarda o `sessao.rs` incompleto do CLA-25 e toca o mesmo arquivo. A ordem de merge com o CLA-25 é decisão do usuário, e o plano deve evitar conflito.

## 7. Testes

Tudo automatizado, incluindo a instalação.

| Camada | O que roda | Browser |
|---|---|---|
| Unidade | cliente CDP contra servidor WebSocket falso; parser de refs e nomes; guarda de envio (lista de nomes, as três condições); gravação e retenção do activity log; redação de campo de senha | nenhum |
| Instalação | download, hash errado, zip corrompido, sem rede, disco cheio, instalação pela metade, versão já presente, contra servidor HTTP local com zip falso | nenhum |
| Integração | Chrome real contra uma página HTML local que reproduz as armadilhas: rádio `opacity:0`, botão que muda de nome, modal sem `role=dialog`, aba em segundo plano | **headless só aqui** |
| Visível | um teste com janela de verdade: tema laranja aplicado, `visibilityState` visível, perfil persiste entre duas aberturas | com janela |

O teste com o LinkedIn real continua manual e curto (`candidatura-completa.mjs --submeter` como referência). O passo "teste no app rodando" do ciclo usa a skill `claudia-teste`.

## 8. A provar na primeira tarefa do plano

Estas hipóteses ainda não foram testadas, e o plano começa por elas, antes de construir por cima:

1. O LinkedIn aceita login e mantém a sessão num Chrome for Testing (no Chrome normal, 3/3 páginas continuaram autenticadas depois de 11 min).
2. O tema laranja pode ser aplicado antes da primeira abertura sem clique do usuário.
3. A licença permite o que o plano faz: baixar da Google na máquina do usuário (esperado: sim); redistribuir (esperado: não, e não fazemos).
4. O cliente CDP próprio funciona dentro do runtime do app, com o MCP em processo separado (a incógnita original da CLA-31, agora sem `chromiumoxide`).

## 9. Fora do escopo deste ciclo

- **Ciclo 2:** lista de contas na UI, estado logado/não, permissão "a IA pode usar", pular sites sem login (spec `2026-10-02-contas-do-browser-design.md`), e a linha do tempo por vaga no Activity log (a lista simples com prints é deste ciclo, seção 5.6).
- **Ciclo 3:** a IA entrar num site, puxar o perfil do usuário e configurar o app. Depende dos dois primeiros.
- Várias contas por site, perfis múltiplos.
- Senhas guardadas pelo app. Descartado por decisão de segurança; reabrir só com cofre criptografado e confirmação.
- Android como destino (opção futura documentada em `docs/pesquisa/2026-09-28-android-emulador.md`).
- Observabilidade além de crash e queda: erros de console e de rede, screenshot em falha, detecção de captcha.


## 10. Ajustes feitos ao escrever o plano (2026-10-02)

Registrados em `docs/superpowers/plans/2026-10-02-browser-cdp-motor.md`. Onde divergem do texto acima, valem estes:

1. Nomes das tools em **inglês**, pela convenção do repo: `browser_open`, `browser_read`, `browser_click`, `browser_type`, `browser_key`, `browser_crop`, `browser_click_at`; parâmetro `confirm_submit`. As descrições seguem em pt-BR.
2. O processo MCP recebe `--browser-state <arquivo>` e `--local-data-dir`, não uma porta fixa.
3. Na abertura do app só o **download** do Chrome fixado acontece, em segundo plano (estado `installed`), para a primeira sessão de IA não esperar. O browser **só é lançado quando a IA é chamada** (início de sessão) ou pelo botão "Abrir o browser" (decisão de 2026-10-03, substitui "lançar na abertura").
4. O histórico de navegação **só é gravado durante sessões de IA**: o rastreador roda no processo MCP, onde está a conexão CDP. Navegação do usuário fora de uma sessão não é registrada. A tela avisa isso.
5. Sessão e vaga de cada registro vêm do banco principal (`sessoes` sem `terminada_em`; vaga em `candidatando`).
