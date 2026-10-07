# Spec: pendências finais da branch do browser (D6 a D9)

> **Status: RASCUNHO, aguardando aprovação do usuário.**

Branch: `feat/browser-cdp-mcp`. Base desta spec: `56ced76`.
Origem: plano `docs/superpowers/plans/2026-10-06-fechar-branch-browser.md`, seção "Respostas do usuário às decisões (2026-10-07)".
Specs relacionadas: `2026-10-02-browser-cdp-motor-design.md` (§5.5 guarda de envio, §5.6 Activity log). A numeração da guarda no motor é §5.5; a §5.4 lista as tools.

---

## 1. Pedido do usuário (palavras dele)

- **D6:** "Cara isso pra ser antes precisa ser rápido, sem perder qualidade. Pra nao enrolar mais essa branch. Entao fax Tudo em agentes em uma workthree. Com revisor no final"
- **D7:** "Corrvir agora e fazer agora nessas pendências desta branch"
- **D8:** "Fazer antes do merge"
- **D9:** "Sim, nesta branch se for pequeno"

## 2. Leitura do pedido

- **D6.** Corrigir a CLA-37 antes do merge: a guarda de envio deixou passar o botão "Upload" (`input type=submit` dentro de `<form>`) sem `confirm_submit`, porque ela só reconhece envio pelo nome. Rápido, sem perder qualidade, feito por agentes numa worktree, com revisor no fim.
- **D7.** Nesta branch, agora: gravar no Activity log os eventos do próprio browser (download, lançamento, queda, crash de aba, diálogo), que a spec do motor §5.6 promete e o código não faz.
- **D8.** Antes do merge: tratar diálogos nativos da página (`alert`, `confirm`, `Page.javascriptDialogOpening`).
- **D9.** Nesta branch, se for pequeno: desligar o JavaScript na janela invisível que gera o PDF. Se não for pequeno, vai para o Linear.

Nada além disso. O que segue em "Propostas" é do assistente e **não está decidido**.

## 3. Como as quatro se ligam

- D7 e D8 escrevem no mesmo lugar: a linha `kind = "browser"` do Activity log. O diálogo (D8) é um dos eventos do D7.
- D6 e D8 mexem no mesmo arquivo (`mcp/tools/browser.rs`). Por isso não rodam em paralelo (ver §8).
- D9 é isolado (`documentos/pdf.rs`).

---

## 4. D6 — Guarda de envio (CLA-37)

### 4.1 O que o código faz hoje (lido em `56ced76`)

- `guard::looks_like_submit(nome)` compara o nome com `src-tauri/submit-patterns.txt` (substring, `=` exato, `~` palavra inteira).
- `mcp/tools/browser.rs`, `INFO_FN` (roda num mundo isolado): marca `isSubmit` **só quando o botão não tem nome** e é `button type=submit` ou `input type=submit|image` dentro de form.
- `run_guard`: nome casou com a lista → `check_submit` (exige confirmação e **gasta** o único envio da vaga no dia). Estrutural (sem nome, iframe, ilegível) → `check_confirmation` (exige confirmação, **não gasta**). Resto → passa.
- Resultado: um submit de formulário **com nome** fora da lista ("Upload", "Concluir", "Finish", "Done") passa sem confirmação. É a CLA-37.
- `browser_key` Enter: olha o elemento focado e acrescenta o **nome** do botão padrão do form (`formDefault`). Mesmo furo: se esse nome está fora da lista, passa.
- Teste live já existente que deve continuar verde: `a_named_submit_type_button_in_a_form_is_not_blocked_just_for_its_type` ("Próximo" `type=submit` passa).

### 4.2 Propostas (NÃO DECIDIDAS)

| Opção | O que é | Prós | Contras |
|---|---|---|---|
| A | Ampliar a lista de nomes | Simples | Nunca fecha: sempre haverá "Concluir", "Færdig", "Done" fora da lista |
| B | Todo submit de formulário é envio | Fecha o furo | Formulário de vários passos ("Próximo" é submit) pede confirmação a cada passo; o agente trava |
| **C** | Submit de formulário é envio, **exceto** nomes de navegação, com a lista de exceções em dados | Fecha o furo e não trava os passos | Exceção mal escolhida reabre o furo; precisa de lista curta e testada |
| D | Interceptar o envio pela rede (CDP `Fetch`) | Pega até botão JS sem form | Não sabe qual POST é "a candidatura"; muito maior; não cabe em "rápido" |
| E | Detectar o último passo do formulário | Mais preciso em tese | Heurística por site; frágil; grande |

**Recomendação: C** (a mesma de 2026-10-05). D fica como segunda camada futura no Linear, não nesta branch.

### 4.3 Regra exata proposta (opção C)

**"Submit de formulário"** é o elemento que, clicado, dispara o envio nativo de um `<form>`:
1. `<button>` sem atributo `type` (o padrão do HTML é `submit`; `el.type` já devolve `"submit"`), ou `<button type=submit>`;
2. `<input type=submit>` ou `<input type=image>`;
3. e o elemento pertence a um form: `el.form` (cobre o atributo `form="id"` de botão fora da tag) ou ancestral `<form>`.

Não é submit de formulário: `button type=button`, `button type=reset`, `<a>`, `div role=button`, e qualquer submit **sem form** (não envia nada nativamente).

**Decisão da guarda, nesta ordem** (para cada alvo examinado: o ref, o nó real sob o ponto, o focado):
1. Algum nome casa com `submit-patterns.txt` → envio por nome: `check_submit` (exige confirmação e gasta o envio). Igual a hoje. **O padrão de envio vence a exceção**: "Revisar e enviar" é envio.
2. Iframe, nó ilegível, submit sem nome → estrutural, igual a hoje.
3. **Novo:** submit de formulário cujo nome **não** é de navegação → estrutural: `check_confirmation` (exige confirmação válida; ver ponto em aberto 4.6-1 sobre gastar ou não).
4. Submit de formulário cujo nome é de navegação → passa.
5. Resto → passa.

**Nome de navegação:** o nome, depois do `fold()` já existente, é **igual** (não substring) a um item da lista de exceções. Todos os nomes colhidos daquele elemento (nome em cache, nome vivo da árvore de acessibilidade, nome do mundo isolado) precisam ser de navegação; se um deles não for, não é exceção.

**Onde fica a lista:** no próprio `submit-patterns.txt`, com prefixo `!` (um item por linha, exato). Proposta inicial, curta de propósito:

```
!avancar        !proximo        !continuar      !seguinte
!revisar        !voltar         !anterior
!next           !continue       !review         !back        !previous
!naeste         !videre         !fortsaet       !tilbage     (dinamarquês, já dobrado)
!weiter         !zuruck         !suivant        !retour
```

(Formato final: um por linha. A grafia dobrada sai do `fold()`; o agente confere cada item com teste.)

**Enter (`browser_key`):** Enter num campo dentro de form equivale a clicar no **botão padrão** do form. O `INFO_FN` passa a devolver também se esse botão padrão existe; a regra acima se aplica a ele. Form sem botão submit: passa, como hoje (ver limite 4.5-2). Space no botão focado: a regra se aplica ao próprio botão.

**`browser_upload`:** o caminho "botão que abre o seletor" já chama a guarda sem confirmação e recusa se for envio. Com C, um botão submit de form com nome "Upload" passa a ser recusado ali. É o comportamento certo (clicar num submit envia o form); a mensagem de recusa ganha a dica "use a ref do campo de arquivo". Ver risco 4.7-3.

**Mensagem ao agente** (estrutural nova): "RECUSADO: este botão envia um formulário. Se for o envio da candidatura, repita com confirm_submit = <id da vaga>." Sem dizer qual é o id ativo (regra atual).

### 4.4 Arquivos

- `src-tauri/submit-patterns.txt` — linhas `!`.
- `src-tauri/src/browser/guard.rs` — `looks_like_submit` ignora linhas `!`; nova função pura `is_navigation_name(&str) -> bool`.
- `src-tauri/src/mcp/tools/browser.rs` — `INFO_FN` (campo `formSubmit`, sem a condição "sem nome"; `formDefaultIsSubmit`), `Info`, `Probe`, `run_guard` (passo 3), `key` (Enter), mensagem do `upload`.
- `src-tauri/tests/fixtures/guard-form.html` — **novo**.
- `src-tauri/tests/browser_live.rs` — testes novos.

### 4.5 Limites documentados (não resolvidos por C)

1. Botão **fora de form** com nome desconhecido e envio por JavaScript (`fetch`) continua passando. Só D (rede) pegaria. Vai para o Linear como segunda camada.
2. Enter num form **sem** botão submit (envio implícito de campo único) continua passando, como hoje. Fechar isso trava caixas de busca.
3. Página hostil que envia pelos próprios handlers (já está no inventário, Linear).

### 4.6 Pontos em aberto (precisam do usuário)

1. **Gastar ou não o envio do dia** no submit de form com nome desconhecido. Recomendação: **não gastar** (como o estrutural de hoje). Motivo: um "Salvar" no meio do formulário, confirmado, gastaria o único envio e bloquearia o envio real. Custo: o envio real com nome desconhecido ("Concluir") não conta no "uma vez por vaga por dia". O orçamento diário e a vaga em `candidatando` continuam valendo.
2. A lista inicial de exceções acima. "Salvar"/"Save" **não** entra (pode ser o fim em algum ATS); vai pedir confirmação.

### 4.7 Riscos

1. Exceção larga demais reabre o furo. Mitigação: só igualdade exata, lista curta, padrão de envio vence.
2. Fricção: botões como "Salvar", "Upload", "Aplicar filtro" dentro de form passam a pedir confirmação. Fora de `candidatando`, o agente não tem como confirmar: fica travado nesse botão e deve registrar pendência. Aceitável por ser o lado seguro.
3. `browser_upload` com botão submit que abre o seletor passa a recusar. O agente pode usar o `input type=file` direto (já suportado). Não verificado no LinkedIn real.

### 4.8 Testes

**Unitários** (`guard.rs` e `browser.rs`, sem browser):
- `is_navigation_name`: todos os itens da lista, com acento, caixa e espaços variados; "Revisar e enviar", "Next step and submit", "Upload", "Concluir" **não** são.
- `looks_like_submit` não é afetado pelas linhas `!` (os testes atuais continuam verdes).
- Decisão pura sobre `Probe` (já há `probe_of` nos testes): form-submit + nome desconhecido → exige confirmação; + nome de navegação → passa; + nome de envio → gasta; nomes mistos (um de navegação, outro não) → exige confirmação.

**Live** (Chrome de teste, `guard-form.html`, **só no Sandbox**: o harness cria e apaga o perfil temporário):
- `input type=submit value="Upload"` em form: recusado sem `confirm_submit` (regressão da CLA-37, falha antes e passa depois).
- `<button>Concluir</button>` sem `type` em form: recusado.
- `input type=image alt="Ir"` em form: recusado.
- `<button type=submit>Next</button>` em form: passa.
- `<button type=button>Upload</button>` em form: passa.
- `<button type=submit>Revisar e enviar</button>`: recusado, e com confirmação gasta o envio.
- Botão "Finish" fora de form com `onclick`: passa (limite 4.5-1 fica provado e escrito no teste).
- Enter num campo de form cujo botão padrão é "Concluir": recusado; botão padrão "Próximo": passa.
- `browser_click_at` sobre o "Upload": recusado.
- O teste existente "Próximo" continua verde.

**Mutação** (obrigatória): tirar o passo 3 do `run_guard` → o teste do "Upload" falha (colar a linha `FAILED`); tirar o `!` de "next" do arquivo → o teste do "Next" falha; fazer a exceção valer por substring → o teste "Revisar e enviar" falha.

---

## 5. D7 — Eventos do próprio browser no Activity log

### 5.1 O que o código faz hoje

- O Activity log tem os tipos `action | browser | failure | navigation` (`activity_log/mod.rs`). A tela já tem o filtro "browser" (`ActivityLogScreen.tsx`), mas **ninguém grava** `kind = "browser"`: o filtro fica sempre vazio.
- O `LogSink` (`activity_log/sink.rs`) só é criado no processo MCP (`mcp/server.rs`). O processo do app (onde vivem download, lançamento e vigia) não grava nada.
- `browser/service.rs` já passa por todos os pontos: `install()` (download e erro), `prepare()` (lançado, erro ao lançar), `watch()` (queda, relançamento, desistência, "não relança porque não há sessão").

### 5.2 Eventos propostos (NÃO DECIDIDOS)

| Evento (`tool`) | Onde nasce | Processo | Quando |
|---|---|---|---|
| `download_started` | `service::install`, primeiro progresso | app | Só quando há download de verdade |
| `installed` | `service::install`, sucesso depois de download | app | Com a versão |
| `install_failed` | `service::install`, erro | app | Com a mensagem (sem rede, hash, disco...) |
| `launched` | `service::prepare`, `ready` | app | Com versão e porta |
| `launch_failed` | `service::prepare`, erro | app | Com a mensagem |
| `gone` | `service::watch`, `WatchAction::Relaunch` | app | Duas checagens seguidas falharam |
| `relaunched` | `service::watch`, depois do `prepare` | app | |
| `not_relaunched` | `service::watch`, sem sessão ativa | app | Janela fechada pelo usuário fica fechada |
| `gave_up` | `service::watch`, `WatchAction::GiveUp` | app | Fechou várias vezes seguidas |
| `tab_crashed` | evento CDP `Inspector.targetCrashed` | MCP | Na aba do agente |
| `dialog` | evento CDP `Page.javascriptDialogOpening` | MCP | Ver D8 |

Fora: progresso por pedaço do download (milhares de linhas) e "aba ativada" (a cada ação; vira ruído). Recomendação: esta tabela.

### 5.3 Formato da linha (contrato entre frentes)

Em `activity_log` (frente D7):

```rust
pub struct BrowserEvent {
    pub event: &'static str,      // um nome da tabela 5.2
    pub ok: bool,
    pub detail: Option<String>,   // curto: versão, porta, tipo do diálogo; nunca texto digitado
    pub url: Option<String>,
    pub error: Option<String>,
    pub extra: Option<serde_json::Value>,
}
impl LogSink { pub fn browser_event(&self, ev: BrowserEvent) } // síncrono, melhor esforço, nunca entra em pânico
```

Grava: `kind = "browser"`, `tool = event`, `target = detail`, `url`, `result = "ok" | "error"`, `error`, `extra` (JSON), sessão e vaga pelo `context()` já existente. Sem prints.

### 5.4 Onde gravar sem atrasar nada

- **App:** `service.rs` cria um `LogSink` com `app_data_dir` e `app_local_data_dir` e grava por `tauri::async_runtime::spawn_blocking`, sem esperar o resultado. Download, lançamento e vigia seguem no mesmo ritmo.
- **MCP:** o tratador de eventos de página (D8) já roda em tarefa própria; grava por `spawn_blocking` também.
- Falha ao gravar: vai para o `diagnostics` (comportamento atual do `LogSink::write`), nunca para o usuário nem para o agente.
- Os dois processos já escrevem no mesmo banco em WAL (spec §5.6).

### 5.5 Privacidade e retenção

- Nenhum evento carrega texto digitado nem senha. O texto de um diálogo é texto da página, cortado em 200 caracteres (ver D8); o valor padrão de `prompt()` não é gravado.
- Mesma tabela, mesma limpeza (30 dias, 1 GB). Nada novo para apagar.

### 5.6 Tela

O filtro "browser" já existe e a coluna mostra `tool` (o nome do evento). **Proposta (não decidida):** não mexer na tela. Opcional: rótulos pt-BR para os nomes de evento. Recomendação: não, nesta branch; é tela de debug.

### 5.7 Arquivos

- `src-tauri/src/activity_log/mod.rs` — `BrowserEvent` (ou em `sink.rs`, à escolha do agente, desde que o caminho público fique `crate::activity_log::BrowserEvent`).
- `src-tauri/src/activity_log/sink.rs` — `LogSink::browser_event` e testes.
- `src-tauri/src/browser/service.rs` — chamadas nos pontos da tabela.
- `tab_crashed` e `dialog`: frente D8 (§8), porque nascem no mesmo tratador de eventos.

### 5.8 Testes

- **Unitários** (pastas temporárias; gravam e a limpeza do sink apaga capturas → **rodam no Sandbox**): `browser_event` grava `kind = "browser"` com os campos certos; filtro `kind = "browser"` devolve só essas linhas; falha ao gravar não propaga (mesmo padrão de `a_write_failure_never_panics_and_never_propagates`).
- **Ordem de eventos** em `service.rs`: as funções `install`, `prepare` e `watch` já recebem `publish` e `spawn` injetáveis. Proposta: injetar também um "gravador de evento" (`&dyn Fn(BrowserEvent)`) e testar com um gravador em memória: download que falha → `download_started`, `install_failed`; lançamento ok → `launched`; vigia com processo morto e sessão ativa → `gone`, `relaunched`; sem sessão → `not_relaunched`. Os testes de `service.rs` que sobem servidor HTTP local e extraem zip escrevem e apagam em pasta temporária → **Sandbox**.
- **Mutação:** remover a chamada de `gone` → o teste da vigia falha.
- **Smoke** (`/claudia-teste-e2e`, Sandbox): abrir o app, "Abrir o browser", fechar a janela do Chrome com sessão ativa, ver no Activity log com filtro "browser" as linhas `launched`, `gone`, `relaunched`.

### 5.9 Riscos

- Volume: a vigia roda a cada 3 s; só transições geram linha, nunca a checagem em si.
- `context()` abre o banco principal a cada evento; é raro, aceitável.

---

## 6. D8 — Diálogos nativos (`alert`, `confirm`, `prompt`, `beforeunload`)

### 6.1 O problema

Um diálogo nativo aberto **trava a página**: `Runtime.evaluate` e os eventos de input ficam sem resposta até o timeout. Hoje o código não trata `Page.javascriptDialogOpening` (o `Page.enable` já é chamado ao abrir a aba). O agente vê só "o browser não respondeu a tempo".

### 6.2 Propostas de política (NÃO DECIDIDAS)

| Opção | `alert` | `confirm` | `prompt` | `beforeunload` |
|---|---|---|---|---|
| P1: recusar tudo | OK | Cancelar | Cancelar | Ficar |
| **P2: seguro, com exceção para envio aprovado** | OK | Cancelar, **exceto** durante um clique que a guarda aprovou com `confirm_submit`: aí OK | Cancelar | Sair só durante `browser_open`; senão Ficar |
| P3: deixar o agente decidir (tool nova `browser_dialog`) | — | — | — | — |

**Recomendação: P2.**
- `confirm` com Cancelar é o lado seguro: um "Enviar candidatura?" nunca é aceito por acaso. Mas, se a guarda já aprovou o envio, um "Tem certeza?" do site tem de ser aceito, senão o envio aprovado nunca acontece.
- `beforeunload` durante `browser_open`: o agente pediu para sair; sair não envia nada, só perde o que não foi enviado.
- P3 rejeitada: a página fica travada esperando o agente, e daria ao modelo um caminho para aceitar um `confirm` de envio fora da guarda.

### 6.3 Nunca travar

- Um tratador em tarefa própria (como o `nav::spawn_tracker`) escuta `Page.javascriptDialogOpening` na aba do agente e responde na hora com `Page.handleJavaScriptDialog`, com timeout curto. Não depende de haver tool em andamento.
- Erro ao responder (diálogo já fechado) é ignorado e gravado no `diagnostics`.
- O mesmo tratador escuta `Inspector.targetCrashed` (D7, `tab_crashed`).

### 6.4 O que a tool devolve ao agente

- Diálogo durante uma tool: a resposta da tool ganha uma linha, antes da linha `estado:`:
  `Diálogo da página (confirm): "<texto, até 200 caracteres>". O app respondeu: Cancelar.`
- Diálogo fora de tool (timer da página): fica guardado e sai no início da resposta da próxima tool.
- O texto vem da página: vai entre aspas, marcado como "da página", nunca como instrução.
- Crash de aba: a tool devolve "a aba do browser travou e foi perdida; chame browser_open de novo" (texto final a ajustar pelo agente; precisa ser acionável).

### 6.5 Activity log

Uma linha por diálogo (contrato §5.3): `event = "dialog"`, `detail = tipo` (`alert|confirm|prompt|beforeunload`), `url` da página, `extra = { "message": "<200 caracteres>", "accepted": bool, "during_tool": "<nome>|null" }`. Sem o valor padrão do `prompt`.

### 6.6 Arquivos

- `src-tauri/src/browser/page_events.rs` — **novo**: tratador (diálogo + crash), política pura `dialog_answer(tipo, contexto) -> bool`.
- `src-tauri/src/browser/mod.rs` — `pub mod page_events;`.
- `src-tauri/src/mcp/tools/browser.rs` — iniciar o tratador junto do `nav::spawn_tracker`; marcar "clique aprovado pela guarda" e "browser_open em andamento" (dois `AtomicBool`); juntar as notas na resposta; método novo no `ActionSink`: `fn browser_event(&self, _ev: BrowserEvent) {}` (padrão vazio, para o `NoSink` e os sinks de teste).
- `src-tauri/src/activity_log/sink.rs` — `impl ActionSink for LogSink`: `browser_event` chama o `LogSink::browser_event` da frente D7.
- `src-tauri/tests/fixtures/dialogs.html` — **novo**.
- `src-tauri/tests/browser_live.rs` — testes novos.

### 6.7 Testes

- **Unitários:** tabela da `dialog_answer` (cada tipo × clique aprovado sim/não × `browser_open` sim/não); corte do texto em 200 caracteres; nota guardada sai uma vez só.
- **Live** (`dialogs.html`, **só no Sandbox**):
  - clique em botão com `alert` → a tool volta em menos de 5 s, com a nota; a página segue respondendo (`browser_read` funciona);
  - `confirm` sem aprovação → `window.__confirm === false`;
  - submit com `confirm("Enviar?")` no `onsubmit`, clicado com `confirm_submit` válido → `true` e o form enviado;
  - `prompt` → `null`;
  - página com `beforeunload` + `browser_open` para outra fixture → navega; clique em link na mesma página → fica, com a nota;
  - `alert` disparado por `setTimeout` fora de tool → aparece na próxima tool;
  - sink em memória recebe `dialog` com tipo e `accepted` certos;
  - `Page.crash` na aba do agente → `tab_crashed` no sink e a tool seguinte devolve erro acionável (não trava).
- **Mutação:** fazer `confirm` responder OK sempre → o teste "`confirm` sem aprovação" falha; desligar o tratador → o teste do `alert` estoura o tempo.

### 6.8 Riscos

- Corrida entre o clique aprovado e o diálogo: o `AtomicBool` "aprovado" precisa valer até o fim da tool (inclui a espera depois do clique), e voltar a falso sempre, inclusive em erro.
- `Page.javascriptDialogOpening` para diálogo que já estava aberto antes de o MCP conectar: **não verificado** se o Chrome reenvia no `Page.enable`.
- `Inspector.targetCrashed` sem `Inspector.enable`: **não verificado**; o agente confere e habilita se preciso.

---

## 7. D9 — JavaScript na janela invisível do PDF

### 7.1 O que o código faz hoje

`documentos/pdf.rs` abre uma `WebviewWindowBuilder` invisível com `WebviewUrl::External(file://...)` e chama `PrintToPdf` do WebView2. O JavaScript da página fica ligado. A janela não tem capability (sem IPC com o app), mas o HTML vem do agente ou de importação: script nele roda sem necessidade.

### 7.2 O HTML do currículo precisa de JS?

Não deveria: a impressão é nativa (`PrintToPdf`) e os HTML são documentos estáticos (`cover_letter.rs` escapa o texto; `texto.rs` já descarta `<script>` na extração). **Não verificado** contra os HTML que o usuário já tem na biblioteca; o agente faz um grep por `<script` nos modelos e na skill que gera o currículo e relata.

### 7.3 Propostas (NÃO DECIDIDAS)

| Opção | Como | Tamanho |
|---|---|---|
| **A** | Criar a janela em `about:blank`, desligar o script nas configurações do WebView2 (`ICoreWebView2Settings::SetIsScriptEnabled(false)`, via `with_webview`), e só então navegar para o arquivo. O sinal de "carregou" passa a ignorar o `about:blank` (conferir a URL no `on_page_load`) | ~20 linhas, só `pdf.rs` |
| B | Injetar `<meta http-equiv="Content-Security-Policy" content="script-src 'none'">` no HTML antes de imprimir | Mexe no arquivo ou numa cópia; o HTML registrado tem hash |
| C | Opção do builder do Tauri para desligar JS | **Não verificado** que exista no Tauri 2 |

**Recomendação: A.** A mudança de configuração vale a partir da próxima navegação, por isso o `about:blank` primeiro. É pequena: cabe na regra do D9 ("nesta branch se for pequeno"). Se o agente descobrir que não é (por exemplo, o Tauri não deixa navegar uma janela já criada), para e vai para o Linear, como o usuário pediu.

### 7.4 Arquivos

- `src-tauri/src/documentos/pdf.rs`.
- `src-tauri/tests/fixtures/pdf-com-script.html` — **novo**: texto fixo "TEXTO ESTATICO" e um `<script>` que troca o corpo por "JS RODOU".

### 7.5 Testes

- Precisa de WebView2 e de uma janela do app: não roda em `cargo test --lib`. Proposta: teste no app rodando (`/claudia-teste-e2e`, **Sandbox**, porque grava e apaga PDF temporário): gerar o PDF da fixture e conferir que o texto do PDF tem "TEXTO ESTATICO" e não tem "JS RODOU".
- **Mutação:** tirar o `SetIsScriptEnabled(false)` → o PDF passa a ter "JS RODOU".
- Regressão: gerar o PDF de um currículo normal da fixture e conferir que continua igual (páginas, texto).
- Como extrair o texto do PDF sem dependência nova: **não verificado**; o agente propõe (ex.: procurar a string no conteúdo do PDF não comprimido, ou ler pelo Chrome de teste) antes de escrever.

### 7.6 Riscos

- `on_page_load` de `about:blank` disparar o sinal de "carregou" antes do arquivo: tratado conferindo a URL.
- Algum HTML antigo que dependa de JS para montar o conteúdo sairia diferente. Improvável (7.2).

---

## 8. Divisão em frentes (arquivos disjuntos)

Duas ondas. Na mesma onda, nenhum arquivo é compartilhado. O orquestrador integra cada onda e roda a regressão antes da seguinte.

**Onda 1 (em paralelo):**

| Frente | Item | Arquivos (só estes) | Modelo |
|---|---|---|---|
| F1 | D6 guarda | `submit-patterns.txt`, `browser/guard.rs`, `mcp/tools/browser.rs`, `tests/browser_live.rs`, `tests/fixtures/guard-form.html` (novo) | Sonnet |
| F2 | D7 lado do app | `activity_log/mod.rs`, `activity_log/sink.rs`, `browser/service.rs` | Sonnet |
| F3 | D9 PDF | `documentos/pdf.rs`, `tests/fixtures/pdf-com-script.html` (novo) | Sonnet (Haiku se a opção A se confirmar trivial) |

**Onda 2 (depois de F1 e F2 integradas):**

| Frente | Item | Arquivos | Modelo |
|---|---|---|---|
| F4 | D8 diálogos + `tab_crashed` do D7 | `browser/page_events.rs` (novo), `browser/mod.rs`, `mcp/tools/browser.rs`, `activity_log/sink.rs`, `tests/browser_live.rs`, `tests/fixtures/dialogs.html` (novo) | Sonnet |

Contratos: `BrowserEvent` e `LogSink::browser_event` (§5.3), que F2 entrega e F4 usa; o método `ActionSink::browser_event` com padrão vazio (§6.6).

**Revisor Opus no fim**, sobre as quatro frentes juntas, com foco em: a regra da guarda e suas exceções, o `AtomicBool` de clique aprovado (D8), e que nenhum caminho novo apaga arquivo fora de pasta temporária.

### 8.1 Regras para os agentes

- Base da worktree = `feat/browser-cdp-mcp` atual; conferir o commit antes de começar.
- `CARGO_TARGET_DIR` próprio, fora do repositório.
- **No PC do usuário:** só `cargo check`, `cargo clippy`, `cargo build` e `npx tsc --noEmit`.
- **Só no Sandbox** (skill `claudia-teste-sandbox`): `cargo test --lib` desta área (os testes do sink e do `service.rs` gravam e apagam em pasta temporária) e todos os testes live (o harness cria e apaga o perfil temporário do Chrome de teste).
- Teste nunca usa caminho real; só `temp_dir()` com subpasta única. Nunca o Chrome pessoal do usuário. Só páginas de `tests/fixtures/`.
- Sem push. Sem editar `prompt_sistema_runtime.md`.

## 9. Fora do escopo

- Interceptação de envio pela rede (opção D do D6): Linear.
- Botão JS fora de form e envio implícito de campo único (limites 4.5).
- Rótulos pt-BR dos eventos na tela do Activity log.
- "Aba ativada" e progresso por pedaço como eventos.
- Seletor de arquivo nativo tardio (já decidido antes: sem Escape).
- Qualquer mudança em `prompt_sistema_runtime.md` (se a mensagem nova da guarda exigir instrução ao agente, isso volta ao usuário).

## 10. Pontos em aberto para o usuário

1. D6: confirmar a opção **C**.
2. D6: submit de form com nome desconhecido **gasta ou não** o envio do dia (recomendação: não gasta).
3. D6: a lista inicial de exceções de navegação (§4.3), e "Salvar" fora dela.
4. D7: a lista de eventos (§5.2), sem "aba ativada" e sem progresso.
5. D8: política **P2**, incluindo aceitar `confirm` só durante um clique já aprovado pela guarda e `beforeunload` só durante `browser_open`.
6. D9: opção **A**; se não for pequena, vai para o Linear.

## 11. Não verificado

- Se o Chrome reenvia `Page.javascriptDialogOpening` para diálogo já aberto ao chamar `Page.enable`.
- Se `Inspector.targetCrashed` chega sem `Inspector.enable`.
- Se o Tauri 2 permite navegar a janela invisível depois de mudar a configuração do WebView2 (opção A do D9), e se há opção do builder para desligar JS.
- Se algum HTML da biblioteca do usuário usa JavaScript.
- Comportamento da regra C no LinkedIn real (botão de upload, botão final) — depende da rodada real (D4).
- Nada desta spec foi compilado nem executado: é só leitura do código em `56ced76`.
