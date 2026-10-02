# Spec: motor do browser da Claudia (CDP direto)

> Spec de design. Não é o plano de implementação. Ciclo 1 de 3 (ver seção 9).

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
| Memória das contas | Perfil persistente em `<dados>/browser/perfil/`. **Só sessão (cookies); nenhuma senha passa pelo app** | Igual a um browser normal; sem cofre de senhas num app que entrega ações a agentes de terceiros |
| Tema | Laranja `#D97757` (`--accent` do design) como padrão | Pedido do usuário |
| Cliente CDP | **Escrito por nós** sobre WebSocket, ~12 comandos. Sem `chromiumoxide` | O protótipo inteiro é um cliente de 40 linhas; uma dependência grande a menos; testável com servidor WebSocket falso |
| Envio de candidatura | O agente decide sozinho, **dentro da guarda** (seção 5.5) | Princípio do projeto: autonomia com contenção no código, sem confirmação humana por rodada |
| Trilha de ações | **Só em modo desenvolvedor**, banco separado, sem tool MCP para ler | Debug e desenvolvimento, não auditoria do usuário |

## 4. Arquitetura

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

- O app escreve a porta de debug num arquivo em `<dados>/browser/estado.json` e a passa ao processo MCP por `--browser-port`, como já faz com `--notify-port`.
- O processo MCP é só cliente: conecta, executa o comando CDP, devolve.
- `dispatch` (`src-tauri/src/mcp/mod.rs`) hoje é síncrono e devolve `String`. As tools de browser precisam de um caminho assíncrono e de resposta com **imagem** (`ContentBlock` de imagem do `rmcp`). Isso é mudança estrutural e entra no plano como tarefa própria.

## 5. Componentes

### 5.1 `browser::instalacao`

Garante que o Chrome existe em `<dados>/browser/<versão>/chrome-win64/chrome.exe`.

1. Se existe e o `chrome.exe --version` bate com a versão fixada: usa.
2. Senão: baixa `https://storage.googleapis.com/chrome-for-testing-public/<versão>/win64/chrome-win64.zip`, confere o SHA-256 fixado, extrai numa pasta temporária e **só então** renomeia para a pasta final (nunca deixa instalação pela metade).
3. Emite progresso (bytes, percentual) para a UI por evento Tauri.

Erros tratados com mensagem própria: sem rede, hash diferente, zip corrompido, disco cheio, antivírus bloqueando o `.exe`. O download é retomável do zero (apaga a pasta temporária e recomeça).

A URL base e a pasta de destino são **parâmetros**, para o teste apontar para um servidor local.

### 5.2 `browser::gestor` (BrowserManager)

- Lança o Chrome com `--remote-debugging-port=<livre>`, `--user-data-dir=<dados>/browser/perfil`, `--no-first-run`, `--no-default-browser-check`. **Nunca** `--headless` em execução.
- Escolhe porta livre e grava `estado.json`. A porta 9222 costuma ser do Chrome pessoal do usuário (já aconteceu no teste do WebView2).
- Vigia: se o processo morre, religa e emite evento. A trilha registra a queda.
- **Aba ativa:** antes de qualquer ação, `/json/activate/{id}` e poll de `document.visibilityState` até `visible`. Aba congelada engole input em silêncio (armadilha 1).
- **Tema:** grava a cor no `Preferences` do perfil **antes** da primeira abertura. Se o Chrome não aceitar, o plano tem tarefa de investigar a alternativa (política `BrowserThemeColor` por registro em HKCU, que não exige administrador).

### 5.3 `browser::cdp`

Cliente fino sobre `tokio-tungstenite`. Faz: conectar ao alvo `type=page`, enviar comando com id, casar resposta, receber eventos. Comandos usados:

`Page.navigate`, `Accessibility.getFullAXTree`, `DOM.resolveNode`, `DOM.scrollIntoViewIfNeeded`, `DOM.getBoxModel`, `Input.dispatchMouseEvent`, `Input.dispatchKeyEvent`, `Input.insertText`, `Page.captureScreenshot` (com `clip`), `Runtime.evaluate`, `Network.getCookies` (usado no ciclo 2).

### 5.4 Tools MCP (o que o agente enxerga)

| Tool | O que faz |
|---|---|
| `browser_abrir(url)` | navega. Recusa domínio de conta não logada ou não permitida (ciclo 2) |
| `browser_ler(recorte?)` | árvore de acessibilidade com **refs** (`e12`) por elemento; `recorte` limita a um nó, ex.: o modal |
| `browser_clicar(ref, confirmar_envio?)` | resolve, rola até o elemento, clica. Se o alvo é um `<input>` escondido (rádio), clica no `label[for]` |
| `browser_escrever(ref, texto)` | foca e insere o texto |
| `browser_tecla(tecla)` | Enter, Tab, Esc |
| `browser_recortar(x,y,w,h)` | imagem do recorte (~390 tok) |
| `browser_clicar_em(x,y)` | clique por coordenada, para o que a árvore não expõe |

As armadilhas 1 (aba congelada), 2 (modal sem `role=dialog` no DOM, só na árvore de acessibilidade), 3 (rádio com `opacity:0`), 4 (`getBoxModel` exige `scrollIntoViewIfNeeded`) e 5 (botão de avanço que muda de nome) ficam **dentro do app**. O agente não precisa saber delas.

`browser_contas` pertence ao ciclo 2.

### 5.5 Guarda de envio

`browser_clicar` e `browser_clicar_em` verificam o nome acessível do alvo. Se parece envio ("Enviar candidatura", "Submit application", "Send application", "Postuler", etc., lista em um arquivo de configuração testado), o clique é **recusado** a menos que a chamada traga `confirmar_envio: <vaga_id>` e as três condições valham:

1. a vaga está em `candidatando`;
2. o orçamento diário não estourou;
3. o site está logado e permitido (ciclo 2; até lá, só 1 e 2).

Recusa devolve texto claro ao agente, que se corrige. A guarda é código do app, não instrução de prompt. Em clique por coordenada, a guarda lê o elemento sob o ponto antes de clicar.

### 5.6 Trilha do modo desenvolvedor

- Opção **"Modo desenvolvedor"** nas configurações, **desligada por padrão**. Desligada, a trilha não é gravada.
- Banco **separado**, `<dados>/dev-trilha.db`, fora da pasta onde as sessões de IA trabalham. Nenhuma tool MCP lê esse banco. Só a UI do app, dentro do modo desenvolvedor, o lê.
- Uma linha por chamada: instante, sessão, vaga (se houver), ação, alvo por nome acessível (não só o ref), página, resultado. Entram também: crash de aba, queda do Chrome, ação falhada, clique recusado pela guarda.
- O texto digitado é gravado; **campos de senha nunca**. Retenção de 30 dias.
- **Limite honesto:** o agente roda com permissões liberadas e pode ler qualquer arquivo que o usuário do Windows leia. O arquivo fora da pasta de trabalho, nunca citado em prompt ou tool, e o confinamento da CLA-30 reduzem o acesso acidental, mas não o impedem por completo.
- A tela de visualização é do ciclo 2. No ciclo 1 só grava.

## 6. Remoção da dependência do Claude

- Sessão de execução deixa de passar `--chrome` (`src-tauri/src/commands/sessao.rs:250`) e a sessão de perfil idem (`commands/perfil.rs:720`). O MCP do app é registrado na sessão, como já é.
- A checagem da extensão e do native messaging host em `welcome.rs` sai do setup.
- `prompt_sistema_runtime.md` precisa mudar: o agente deixa de "dirigir o browser" e passa a "pedir ações pelas tools `browser_*`". **Esse arquivo não pode ser editado sem pedido explícito do usuário**, então a mudança é proposta no plano e só aplicada com aprovação.
- O `stash@{0}` guarda o `sessao.rs` incompleto do CLA-25 e toca o mesmo arquivo. A ordem de merge com o CLA-25 é decisão do usuário, e o plano deve evitar conflito.

## 7. Testes

Tudo automatizado, incluindo a instalação.

| Camada | O que roda | Browser |
|---|---|---|
| Unidade | cliente CDP contra servidor WebSocket falso; parser de refs e nomes; guarda de envio (lista de nomes, as três condições); gravação e retenção da trilha; redação de campo de senha | nenhum |
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

- **Ciclo 2:** lista de contas na UI, estado logado/não, permissão "a IA pode usar", pular sites sem login (spec `2026-10-02-contas-do-browser-design.md`), e a tela do modo desenvolvedor.
- **Ciclo 3:** a IA entrar num site, puxar o perfil do usuário e configurar o app. Depende dos dois primeiros.
- Várias contas por site, perfis múltiplos.
- Senhas guardadas pelo app. Descartado por decisão de segurança; reabrir só com cofre criptografado e confirmação.
- Android como destino (opção futura documentada em `docs/pesquisa/2026-09-28-android-emulador.md`).
- Observabilidade além de crash e queda: erros de console e de rede, screenshot em falha, detecção de captcha.
