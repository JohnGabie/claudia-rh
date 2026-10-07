# Claudia RH

App desktop Windows (Tauri v2: Rust + React/TypeScript) que automatiza candidaturas com um agente de IA e um browser próprio.

Este arquivo é o ponto de entrada de qualquer sessão de coding **neste repositório**.

## Ordem de leitura

1. `docs/arquitetura-sistema-candidaturas.md` — spec de sistema. Se o que você for construir contradiz este documento, o documento vence — assinale a contradição em vez de a resolver em silêncio.
2. `docs/padrao-design-claudia-rh.md` — identidade visual, tokens, navegação, layout.
3. `src-tauri/src/prompt_sistema_runtime.md` — texto que a app injeta na sessão de execução. **Não edite sem pedido explícito.**
4. Este `CLAUDE.md`.
5. A spec da área em que você vai mexer, em `docs/superpowers/specs/` (browser: `2026-10-02-browser-cdp-motor-design.md` e `2026-10-03-mcp-leitura-robusta-design.md`; contas: `2026-10-02-contas-do-browser-design.md`; documentos: `2026-10-04-anexar-curriculo-design.md`, ainda rascunho).
6. `.claude/QUALIDADE.md` — roadmap de qualidade/refactor (quando existir).

## Convenções

- **Comentários de código: inglês.** Sempre.
- **Commit messages: inglês.**
- **Identificadores novos: inglês.** Código legado em português não se renomeia em massa.
- **Copy da UI: pt-BR.** Docs vivos (`docs/`, este ficheiro) também em pt-BR.
- **README público: inglês.**

## Git

```
main     → só releases (tag vX.Y.Z). Nunca commit direto.
dev      → integração. Sempre contém main + features já merged.
feat/*   → uma preocupação por branch, criada a partir de dev atualizado.
fix/*    → igual, para correções.
```

Fluxo: atualizar `dev` com `main` → `git checkout -b feat/<nome>` a partir de `dev` → review → merge em `dev` → release faz merge `dev` → `main` + tag.

### Ao voltar ao projeto depois de uma pausa

Se o último commit em qualquer branch tem mais de uma semana, o assistente faz um levantamento **só de leitura** antes de começar qualquer coisa nova:

- última tag e o que há em `dev` desde ela (`git log <tag>..dev --oneline`);
- branches `feat/*` e `fix/*` ainda não integradas em `dev`, com a data do último commit de cada;
- worktrees que sobraram e trabalho não commitado;
- as tasks correspondentes no Linear.

E apresenta um resumo curto, nesta ordem de prioridade:

1. **Lançar o que já está pronto.** Se `dev` tem trabalho concluído sem tag, a sugestão é fazer a release antes de abrir frente nova. Código pronto que não foi lançado é o acúmulo mais barato de resolver.
2. **Dar destino a cada branch parada.** Uma de três saídas, nunca "deixa aí":
   - **Terminar**, se falta pouco. Antes, atualizar a branch com `dev`, reler a spec para ver se ainda vale e rodar as rodadas de teste de novo, porque a base mudou.
   - **Fatiar**, se uma parte está completa e útil sozinha. Isso muda o escopo, então a spec é atualizada e aprovada antes.
   - **Descartar ou adiar**, se o contexto mudou. Fica registrado no Linear o porquê, e a branch sai só depois de conferir que não há nada não commitado.
3. **Só então, trabalho novo.**

Orientação geral: **fechar antes de abrir**. No máximo uma branch de feature em andamento além da que está em uso; se já há uma parada, o assistente pergunta o destino dela antes de criar outra.

Isto é sugestão, como o resto desta seção. Se o usuário quiser ir direto ao que veio fazer, o assistente menciona o que está acumulado em uma linha e segue.

## Regras duras

- **NUNCA mudar o `identifier` Tauri** em `src-tauri/tauri.conf.json` (`io.github.johngabie.claudia-rh`). Determina pasta de dados do usuário e identidade do instalador. Teste `identifier_is_frozen` garante isto. Ver `src-tauri/src/migration.rs`.
- **Nenhum passo de instalação novo para o usuário.** O que o app não conseguir empacotar, baixar ou configurar sozinho, não entra. Isso vale para extensões de browser, runtimes (Node, Python), binários externos e servidores que precisam ser instalados à parte.
- **Spec driven development, estritamente.** Sem spec aprovada pelo usuário, não se escreve código. Ver "Processo de desenvolvimento" abaixo.
- Se uma decisão não está na arquitectura nem no design, pare e pergunte.
- **Nada de caminho absoluto de máquina** (`C:Users<nome>...`, a pasta do usuário, o caminho do laboratório) em código, testes, scripts, skills, planos ou docs. Use variáveis de ambiente (`SANDBOX_LAB_DIR`, `CARGO_TARGET_DIR`, `USERPROFILE`), caminhos relativos ao repositório ou as APIs (`temp_dir()`, `os.tmpdir()`, `app_data_dir()`). Os que já existem estão numa issue do Linear para ajustar.
- **Nunca usar o Chrome pessoal do usuário** (nem o perfil dele, nem cópia dele) em teste, script ou agente. Só o Chrome do Claudia-RH (o que o app baixa) ou o Chrome de teste com perfil temporário.
- **Teste nunca usa caminho real** (`C:\`, o repositório, `AppData`, arquivo do sistema), nem para ler, nem para provar que a recusa funciona: a recusa se testa na função de validação pura. **Todo teste que apaga qualquer arquivo roda só no Windows Sandbox** (skill `claudia-teste-sandbox`). No PC do usuário: só compilar, clippy, `tsc`, build e testes de front que não tocam o disco. Motivo: em 2026-10-05 um teste que passava `C:\` para uma função de apagar destruiu o disco do usuário.

### Por que a regra dos passos existe

O setup chegou a ter uns 12 passos: instalar o Claude Code, acertar o PATH, instalar a extensão Claude in Chrome, registrar o native messaging host, abrir o app, criar o perfil. A extensão e o native host saíram: o browser agora é baixado pelo app. Cada passo é uma chance do usuário desistir antes de ver o produto funcionando.

Toda solução que "resolve" um problema adicionando um passo está trocando uma dor por outra — e a dor do setup é paga por **todo** usuário, enquanto a dor técnica é paga uma vez por quem constrói.

Na prática, isso descarta de saída: extensões de browser de terceiros (mesmo boas, mesmo open source), qualquer coisa que rode por `npx`, e servidores MCP externos que o usuário precise instalar. Se a capacidade for mesmo necessária, o caminho é embutir — não pedir.

Exceção única: algo que venha **dentro do instalador do Claudia RH**, com a nossa cara e a nossa responsabilidade. Aí não é um passo novo, é parte do produto.

## Processo de desenvolvimento

Objetivo: **maturar o projeto**. Cada mudança deixa o produto mais previsível, mais testado e mais fácil de confiar. As regras abaixo valem para qualquer mudança, em qualquer sessão, independente do assunto.

Onde ficam os artefatos: specs em `docs/superpowers/specs/` (`AAAA-MM-DD-<tema>-design.md`), planos em `docs/superpowers/plans/`. Tasks, bugs e o que entra em cada versão vivem no Linear (time `CLA`), não em listas neste ficheiro; para registrar e fechar, use a skill `linear-claudia`.

### 1. Spec primeiro (spec driven development, sem exceção de conveniência)

**Quando exige spec:** qualquer feature, mudança de comportamento, mudança de ferramenta do agente, mudança de dados ou de segurança, e qualquer correção de bug que envolva decisão de desenho. **Não exige:** erro de digitação, comentário, formatação, doc sem mudança de regra, e correção cuja causa é óbvia e local (essa ainda exige o teste de regressão da seção 3).

**O que a spec contém**, nesta ordem:
1. **Pedido do usuário**, nas palavras dele, sem reinterpretar.
2. **Leitura do pedido**: o que cada item significa, sem acrescentar nada.
3. **Propostas do assistente**, separadas e marcadas como **não decididas**, cada uma com recomendação. O que o usuário não pediu só entra se ele aceitar.
4. Impacto (dados, telas, ferramentas, segurança), critérios de teste, fora do escopo e pontos em aberto. O que não foi verificado vai escrito como **não verificado**.

**Aprovação:** só vale uma mensagem do usuário que aprova a spec **depois** de ela estar escrita. Não é aprovação: o pedido original, silêncio, uma lista de requisitos colada, ou a recomendação do próprio assistente. **Escrever a spec e já iniciar a implementação é violação desta regra.** Ao terminar a spec, o assistente para, resume o que decidiu por conta própria e pergunta o que muda.

**Depois da aprovação:** plano → implementação → testes → review → smoke. Se a implementação descobrir que a spec está errada ou incompleta, **para, atualiza a spec com a data e pede nova aprovação do trecho alterado**, em vez de seguir por conta própria. A spec é a fonte da verdade do que foi pedido: código e testes respondem a ela.

### 2. Quem faz cada etapa

O modelo depende da complexidade da tarefa, não do hábito.

| Etapa | Modelo | Critério |
|---|---|---|
| Levantamento de requisitos e brainstorm | **Opus** ou **Sonnet** | Opus se a decisão é ampla, ambígua, ou toca segurança, dados ou arquitetura; Sonnet se o escopo é claro. |
| Spec e plano | **Opus** ou **Sonnet** | Mesmo critério. A spec exige aprovação do usuário (seção 1). |
| Execução | **agentes em git worktree**, sempre | **Haiku** para tarefa mecânica e bem delimitada (uma regra, poucos arquivos); **Sonnet** quando há lógica nova, vários arquivos ou decisão de desenho. |
| Revisão de código | **Opus** | Independente de quem escreveu. Obrigatória para segurança, dados e ferramentas do agente. |
| Teste | **Opus** (recomendado) | Observa o app e o browser do app (seção 5) e cruza com o que o agente relatou. |

**Regras de execução por agentes:**
- **Uma preocupação por agente, uma worktree isolada por agente.** Frentes em paralelo só mexem em **arquivos diferentes**; o que as une é um **contrato escrito na spec** (nomes de comandos, assinaturas, formatos). Quem integra é o orquestrador, depois que todas terminarem.
- **A base da worktree pode estar desatualizada.** Antes de começar, o agente confere o commit de base e o alinha com a branch da feature.
- **Cada agente usa o próprio `CARGO_TARGET_DIR`**, fora do `target/` do repositório (o app em desenvolvimento pode estar usando esse).
- **O orquestrador não repassa "passou" sem ter rodado.** Depois de integrar, roda de novo as rodadas da seção 3 e confere o código de saída.
- **O agente não decide fora da spec.** O que a spec não cobre, ele relata e para. Escopo, segurança e edição do prompt de runtime (`src-tauri/src/prompt_sistema_runtime.md`) só com pedido ou aprovação do usuário.
- **Limites do agente:** sem push, sem tocar nos dados do usuário, no app que o usuário está usando ou em sites reais, salvo autorização explícita daquela rodada.
- **Agente que falha** (limite de sessão, erro de API) deve ser tratado como se não tivesse entregado nada: confira a worktree e relance com o mesmo escopo.
- **Remover worktree** só depois de integrar, ou de descartar de propósito olhando antes se há trabalho não commitado.

### 3. Rodadas de teste

"Mudança relevante" = qualquer alteração em código de produção. Docs e comentários não precisam das rodadas.

| Rodada | O que é | Quando |
|---|---|---|
| **Unitários** | `cargo test --lib` e `npx tsc --noEmit` | antes de todo commit relevante |
| **Integração** | testes `#[ignore]` com Chrome real (`src-tauri/tests/*_live.rs`, com `CLAUDIA_TEST_CHROME` apontando para o Chrome de teste), smoke do MCP (`tests/mcp-browser-smoke.mjs`) e validação do Activity log | ao mexer em browser, MCP, banco ou comandos |
| **Regressão** | a suíte inteira roda de novo depois de cada correção ou integração. Todo bug corrigido ganha um teste que **falha antes e passa depois**, e esse teste permanece | sempre |
| **Automáticos com IA** | `/claudia-teste-e2e`: um agente roda o app de verdade (`tauri dev` com CDP), abre a tela, clica, chama comandos e confere o banco; e rodadas de agente contra páginas de teste, com um revisor lendo o Activity log | ao mexer em tela, comando ou ferramenta do agente |

**Regras comuns a todas as rodadas:**
- **Mutação:** todo teste de regra nova precisa de prova de que protege: quebre o código de propósito, confirme que o teste **nomeado** falha, cole a linha `FAILED` e restaure. Vale para qualquer autor, humano ou agente.
- **Código de saída real:** confira o `exit code` do comando. Nunca filtre a saída com `grep` ou `tail` de um jeito que esconda falha.
- **Evidência, não afirmação:** diga o que foi executado e o que foi visto, com comandos, números e caminhos de prints. O que não foi verificado é escrito como "não verificado".
- **Páginas locais por padrão:** testes automáticos usam `src-tauri/tests/fixtures/`, Chrome de teste e pastas temporárias. Site real, login real e envio real só com pedido explícito do usuário para aquela rodada.
- **Dados do usuário são intocáveis:** testes não escrevem no banco real, no perfil do browser do app nem em `%LOCALAPPDATA%\io.github.johngabie.claudia-rh`. Antes de qualquer operação destrutiva, faça backup.
- **Falha fora do escopo** vira task no Linear, não fica só na conversa.

### 4. Fase final de toda feature: smoke test de uso real

Antes de dar a feature por concluída, rode um **smoke automatizado de uso real**: use o produto como o usuário usaria, pelo app de verdade (`/claudia-teste-e2e`). Ele vem **depois** de unitários, integração e regressão estarem verdes e entra na descrição do PR com evidência (o que foi aberto, o que foi chamado, o que a tela mostrou, prints).

| Tipo de feature | Smoke |
|---|---|
| Tela ou frontend | abrir o app, navegar até a tela, exercitar o fluxo completo (abrir, clicar, voltar; estados vazio, erro e com dados; nos dois idiomas) e tirar prints |
| Comando ou backend sem efeito visível | chamar o comando por `invoke` no app rodando e conferir o banco |
| Browser, MCP ou ferramenta do agente | abrir o app e rodar uma rodada de agente pelas ferramentas, com um revisor lendo o Activity log |
| Qualquer mudança que afete a procura de vagas | abrir o app, iniciar a procura pelo botão da UI e acompanhar até o fim |

- A procura de vagas real usa sites reais e as contas do usuário: só roda com autorização do usuário para aquela rodada. **Nunca envia candidatura**, salvo se o usuário disser explicitamente "pode enviar". Sem autorização, o smoke do agente roda contra as páginas locais.
- Se o smoke achar problema, a feature **não está concluída**: corrija, rode a regressão e repita o smoke.

### 5. Quem testa: observar o app e o browser do app

Toda rodada de teste feita por agente usa as skills do `/claudia-teste-e2e` e observa **o que realmente aconteceu**, não só o que foi relatado. Três fontes, sempre:
- **O app:** a tela (print e árvore), o banco, o log de diagnóstico (`debug-logs/`) e o que a UI mostra depois de cada ação. Compare a UI com o banco.
- **O browser do app** (o Chrome gerido pelo próprio app): a janela e a aba reais, por CDP e por print. Observe a página carregada, o tamanho da janela, o estado do login, o que o agente leu e o que ficou na tela. Não confie só no que a ferramenta devolveu.
- **O Activity log:** cada chamada de ferramenta tem linha, prints, duração e resultado. Divergência entre o relato, o log e a tela é um **achado**, não ruído.

Observar é **só leitura**: não clique em envio, não escreva no banco real, não mexa no perfil do browser e não feche o app nem o browser do usuário sem pedir. Para ver o browser sem interferir, abra uma janela de teste própria em vez de usar a aba do agente.

### 6. Definição de pronto

Uma mudança só está pronta quando **todos** os itens são verdadeiros, com evidência:
1. Existe spec aprovada e o código a cumpre (ou a spec foi atualizada e reaprovada).
2. Unitários, integração e regressão verdes, com código de saída conferido.
3. Cada regra nova tem teste com prova de mutação.
4. Smoke de uso real feito e anexado, ou "não verificado" declarado com o motivo.
5. Revisão de código por Opus feita, com os achados corrigidos ou registrados.
6. Nada em "não verificado" que contradiga o que foi afirmado.

**Integrar (merge, push, PR, release) é sempre decisão do usuário**, nunca automática ao fim de um ramo.

### 7. Superpowers

O Superpowers é o esqueleto do processo, **mas as regras deste arquivo vencem** onde forem mais estritas: aprovação explícita da spec como passo próprio, smoke de uso real obrigatório e integração decidida pelo usuário. Skills a usar: `brainstorming` e `writing-plans` antes de criar; `using-git-worktrees` e `subagent-driven-development` na execução; `requesting-code-review` na revisão; `systematic-debugging` em bugs; `verification-before-completion` antes de afirmar que algo funciona.

## Diagnóstico

Em `npm run tauri dev`, lê `debug-logs/events.jsonl` e `debug-logs/pty-tail.log` na raiz do repo **antes** de perguntar o que aconteceu. Em produção (Windows): `%APPDATA%\io.github.johngabie.claudia-rh\diagnostics\`. O que o agente fez no browser fica no Activity log (`%LOCALAPPDATA%\io.github.johngabie.claudia-rh\dev\activity-log.db`, só leitura; tela em Configurações → Activity log).

## Build

- Frontend: `npm run build` (`tsc` + vite). Typecheck: `npx tsc --noEmit`.
- Backend: em `src-tauri/`, `cargo check` / `cargo test` / `cargo clippy`.
- App: `npm run tauri dev`.
- Testes live (Chrome real, `#[ignore]`): `CLAUDIA_TEST_CHROME=%TEMP%\cft\chrome-win64\chrome.exe cargo test --test browser_live --test viewport_live -- --ignored --test-threads=2 --skip icon`. Os de ícone abrem janela visível e rodam à parte.
- Se o app estiver aberto, use `CARGO_TARGET_DIR` fora do `target/` do repo: o Windows trava o `.exe` em uso.

## Teste em máquina limpa

| Quero… | Skill |
|---|---|
| ver, clicar ou conferir o app rodando, tirar print, reproduzir bug | `claudia-teste-e2e` (local ou no Sandbox) |
| montar um Windows limpo, escolher os dados, publicar ou instalar o app, ligar o CDP no guest | `claudia-teste-sandbox` |

A `claudia-teste-sandbox` usa um laboratório Windows Sandbox mantido fora do repositório, localizado pela variável `SANDBOX_LAB_DIR`. Cenários de dados: `novo`, `mock` e `copia` (este só com autorização do usuário na rodada). Falha do app num Windows limpo é resultado do teste: registre no Linear, não contorne no laboratório.

## Dívida conhecida

- **Browser, não provado no mundo real:** nada rodou ainda contra o LinkedIn com login e envio reais; os testes são em páginas locais e headless. O login do LinkedIn no browser do app precisa ser refeito pelo usuário (botão "Abrir o browser").
- **Agente sem como anexar arquivo:** não há `browser_upload`; formulário com "Anexar currículo" trava. O currículo gerado é HTML (site pede PDF). Spec de documentos em rascunho, aguardando aprovação do usuário.
- **Coordenadas:** `browser_crop` mede a partir do topo do documento e `browser_click_at` a partir da tela visível; depois de rolar, divergem.
- Página de erro do Chrome (ex.: 404) aparece como `estado: ok`; a descrição dentro de iframe não aparece em `browser_text`.
- A guarda de envio só libera com **uma** vaga em `candidatando`; com duas, recusa todo envio.
- Desinstalação real nunca foi testada (só em Sandbox/VM; nunca nesta máquina); não existe a opção "só o browser" nem backup antes de apagar.
- Licença do binário do Chrome for Testing não estabelecida e sem auto-update (CLA-34, antes de qualquer release público).
- Toast nativo Windows: `notificacoes.rs` emite eventos Tauri mas não chama o plugin de notificação — o toast do sistema não dispara.
- Pré-requisitos Chrome / plano da conta: não validados na UI.
- Pesquisa pontual da aba Feedback: não implementada.
- `submit_claims` é criada em uso, fora das migrations.

## O que este repo já é

v0.2.0. Perfil = `candidate_base.yaml` + `search_variants.yaml` (módulo `src/components/perfil/`). Dashboard em `src/components/dashboard/`. MCP embutido em `src-tauri/src/mcp/`.

**Browser próprio** (`src-tauri/src/browser/`): Chrome for Testing fixado em `browser-pin.json`, baixado pelo app na primeira abertura; perfil persistente (só cookies, sem senhas); tema e ícone laranja. **Só roda quando a IA é chamada** (início de sessão) ou pelo botão "Abrir o browser" nas Configurações; na abertura do app só baixa. Fecha com o app, e o app tem instância única. Garante viewport mínimo de 1200×800 a cada ferramenta.

**Ferramentas do agente** (MCP `claudia`, `mcp/tools/browser.rs`): `browser_open`, `browser_read`, `browser_text`, `browser_click`, `browser_type`, `browser_key`, `browser_scroll`, `browser_crop`, `browser_click_at`. Todo resultado de leitura termina na linha `estado:` (ok, login_wall, captcha, vazio, carregando). A **guarda de envio** (`browser/guard.rs`) vive no código: `confirm_submit` = id da única vaga em `candidatando`, orçamento diário, uma vez por vaga por dia.

**Activity log**: sempre ligado, banco separado, retenção de 30 dias ou 1 GB, tela própria somente leitura (Configurações → Activity log), sem senhas.

Sessão de execução = `claude --no-chrome` no PTY; a sessão espera o browser abrir.
