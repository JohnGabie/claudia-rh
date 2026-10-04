# Claudia RH

App desktop Windows (Tauri v2: Rust + React/TypeScript) que automatiza candidaturas com um agente de IA e um browser próprio.

Este arquivo é o ponto de entrada de qualquer sessão de coding **neste repositório**.

## Ordem de leitura

1. `docs/arquitetura-sistema-candidaturas.md` — spec de sistema. Se o que você for construir contradiz este documento, o documento vence — assinale a contradição em vez de a resolver em silêncio.
2. `docs/padrao-design-claudia-rh.md` — identidade visual, tokens, navegação, layout.
3. `src-tauri/src/prompt_sistema_runtime.md` — texto que a app injeta na sessão de execução. **Não edite sem pedido explícito.**
4. Este `CLAUDE.md`.
5. `.claude/QUALIDADE.md` — roadmap de qualidade/refactor (quando existir).

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
- **Neste projeto seguimos estritamente spec driven development.** Sem spec aprovada pelo usuário, não se escreve código. Ver "Spec driven e rodadas de teste" abaixo.
- Se uma decisão não está na arquitectura nem no design, pare e pergunte.

### Por que a regra dos passos existe

O setup chegou a ter uns 12 passos: instalar o Claude Code, acertar o PATH, instalar a extensão Claude in Chrome, registrar o native messaging host, abrir o app, criar o perfil. A extensão e o native host saíram: o browser agora é baixado pelo app. Cada passo é uma chance do usuário desistir antes de ver o produto funcionando.

Toda solução que "resolve" um problema adicionando um passo está trocando uma dor por outra — e a dor do setup é paga por **todo** usuário, enquanto a dor técnica é paga uma vez por quem constrói.

Na prática, isso descarta de saída: extensões de browser de terceiros (mesmo boas, mesmo open source), qualquer coisa que rode por `npx`, e servidores MCP externos que o usuário precise instalar. Se a capacidade for mesmo necessária, o caminho é embutir — não pedir.

Exceção única: algo que venha **dentro do instalador do Claudia RH**, com a nossa cara e a nossa responsabilidade. Aí não é um passo novo, é parte do produto.

## Superpowers

Specs: `docs/superpowers/specs/`
Planos: `docs/superpowers/plans/`

Um ciclo = spec aprovada → plano → branch a partir de `dev` → implementação → **teste no app rodando (`claudia-teste-e2e`)** → review → merge em `dev`.

O passo de teste é obrigatório:

- Roda o app de verdade (`tauri dev` com CDP) e exercita o que mudou. `cargo test` e `tsc` não bastam.
- O resultado entra no ledger ou na descrição do PR, com evidência: o que foi aberto, o que foi chamado, o que a tela mostrou.
- Mudança só de backend, sem efeito visível: chama o comando por `invoke` e confere o banco.
- Bug corrigido só fecha depois de reproduzido antes e confirmado depois.

Tasks, bugs e o que entra em cada versão vivem no Linear (time `CLA`), não em listas neste ficheiro. Como registar e quando pode fechar: skill `linear-claudia`.

## Spec driven e rodadas de teste

O objetivo de tudo isto é a **maturação do projeto**: cada mudança deixa o produto mais previsível, mais testado e mais fácil de confiar.

### Spec primeiro, sempre

1. **Toda mudança começa numa spec** em `docs/superpowers/specs/` (data no nome). A spec contém o que o usuário pediu, separado do que o assistente propõe.
2. **O usuário aprova a spec antes de qualquer código.** Escrever a spec e já lançar a implementação não vale: a aprovação é um passo próprio. O que o usuário não pediu fica marcado como proposta e só entra se ele aceitar.
3. Depois da aprovação: plano em `docs/superpowers/plans/` → implementação → testes → review. Se a implementação descobrir que a spec está errada ou incompleta, **para e atualiza a spec** em vez de seguir por conta própria.
4. A spec é a fonte da verdade do que foi pedido: o código e os testes respondem a ela, não o contrário. Decisões novas entram na spec com data.

### Rodadas de teste (todas, a cada mudança relevante)

| Rodada | O que é | Quando |
|---|---|---|
| **Unitários** | `cargo test --lib`, `npx tsc --noEmit` | sempre, antes de cada commit |
| **Integração** | os testes live com Chrome real (`src-tauri/tests/browser_live.rs` e afins, `#[ignore]`, com `CLAUDIA_TEST_CHROME`), smoke MCP (`tests/mcp-browser-smoke.mjs`) e validação do Activity log | ao mexer em browser, MCP, banco ou comandos |
| **Regressão** | a suíte inteira roda de novo depois de cada correção. Bug corrigido ganha um teste que falha antes e passa depois, e esse teste fica | sempre que um bug é corrigido |
| **Automáticos com IA** | `/claudia-teste`: o agente roda o app de verdade (`tauri dev` com CDP), abre a tela, clica, chama os comandos e confere o banco; e rodadas de agentes de verdade contra páginas de teste, com um revisor lendo o Activity log | ao mexer em tela, comando ou ferramenta do agente, e antes de dar qualquer coisa por concluída |

### Fase final de toda feature: smoke test de uso real

Antes de dar uma feature por concluída, a última fase é um **smoke test automatizado de uso real**: usar o produto como o usuário usaria, pelo app de verdade, e não só pelos testes. Ele roda o `/claudia-teste` e entra na descrição do PR com evidência (o que foi aberto, o que foi chamado, o que a tela mostrou, prints).

O que rodar depende da feature:

| Tipo de feature | Smoke de uso real |
|---|---|
| **Tela ou frontend** | abrir o app (`tauri dev` com CDP), navegar até a tela, exercitar o fluxo completo (abrir, clicar, voltar, estados vazio, erro e com dados, nos dois idiomas) e tirar prints. |
| **Comando ou backend sem efeito visível** | chamar o comando por `invoke` no app rodando e conferir o banco. |
| **Browser, MCP ou ferramenta do agente** | abrir o app e rodar uma rodada de agente de verdade pelas ferramentas, com um revisor lendo o Activity log. |
| **Qualquer coisa que muda a busca de vagas** | abrir o app e **executar a procura de vagas** (iniciar a sessão pelo botão da UI) e acompanhar até o fim. |

Limites do smoke:

- **Procura de vagas real usa sites reais e contas do usuário.** Só roda quando o usuário autorizar aquela rodada. **Nunca envia candidatura**, salvo se o usuário disser explicitamente "pode enviar".
- Sem autorização, o smoke de agente roda contra as páginas locais de `src-tauri/tests/fixtures/`.
- O smoke não substitui as outras rodadas: ele vem **depois** de unitários, integração e regressão estarem verdes.
- Se o smoke achar um problema, a feature **não está concluída**: corrige, roda a regressão de novo e repete o smoke.

Regras que valem para todas:

- **Mutação:** teste de regra nova só vale se, ao quebrar o código de propósito, o teste **nomeado** falhar. Cole a linha `FAILED` como prova. Teste que não consegue falhar não protege nada.
- **Exit code real:** confira o código de saída do comando, sem `grep`/`tail` escondendo falha.
- **Evidência, não afirmação:** diga o que foi executado e o que foi visto. O que não foi verificado vai escrito como "não verificado".
- **Nunca em site real sem pedido:** os testes automáticos usam páginas locais (`src-tauri/tests/fixtures/`), Chrome de teste e pastas temporárias. Login, candidatura e envio reais só quando o usuário pedir.
- **Dados do usuário são intocáveis:** testes não escrevem no banco real, no perfil do browser do app nem em `%LOCALAPPDATA%\io.github.johngabie.claudia-rh`. Antes de qualquer coisa destrutiva, backup.
- **Review por outro modelo:** mudanças de segurança ou de ferramenta do agente passam por uma revisão independente (Opus) antes do merge, e os achados ou viram correção ou ficam registrados.
- Falha achada fora do escopo vira task no Linear (skill `linear-claudia`), não fica só na memória.

## Execução por agentes

Quem faz cada etapa. O modelo escolhido depende da complexidade da tarefa, não do hábito.

| Etapa | Modelo | Observação |
|---|---|---|
| **Levantamento de requisitos e brainstorm** | **Opus** ou **Sonnet** | Opus quando a decisão é ampla, ambígua ou mexe em segurança/arquitetura; Sonnet quando o escopo é claro. |
| **Escrever a spec e o plano** | **Opus** ou **Sonnet** | Mesmo critério. A spec precisa da aprovação do usuário (ver acima). |
| **Execução** | sempre por **agentes em git worktree** | **Haiku** é o recomendado para tarefa mecânica e bem delimitada; **Sonnet** quando há complexidade (lógica nova, vários arquivos, decisões de desenho). |
| **Revisão de código** | **Opus** | Independente de quem escreveu. Obrigatória para segurança e ferramentas do agente. |
| **Teste** | **Opus** recomendado | Usa as skills do `/claudia-teste` e **observa sempre o app e o browser do app** (ver abaixo). Lê o Activity log e cruza com o que o agente disse ter visto. |

Regras da execução:

- **Sempre agente, sempre worktree isolada.** Uma preocupação por agente. Frentes paralelas só se mexem em **arquivos diferentes**; o que as une é um **contrato escrito na spec** (nomes de comandos, assinaturas), e quem integra é o orquestrador, depois de os dois terminarem.
- **A worktree pode nascer num commit antigo.** O agente confere a base e alinha com a branch da feature antes de começar.
- **Cada agente usa o próprio `CARGO_TARGET_DIR`** (fora do `target/` do repo, que o app em desenvolvimento pode estar usando) e confere o código de saída real.
- **O agente não herda confiança.** O orquestrador roda de novo os testes depois de integrar e não repassa "passou" sem ter conferido. Testes escritos por Haiku precisam de prova por mutação, porque já apareceram testes que não conseguiam falhar.
- **O agente não decide fora da spec.** Se achar algo que a spec não cobre, para e relata. Escopo, segurança e edição do prompt (`prompt_sistema_runtime.md`) só com pedido ou aprovação do usuário.
- **Limites do agente:** nada de push, nada de tocar nos dados do usuário, no app aberto ou em sites reais, salvo autorização explícita da rodada.
- Agente que cai (limite de sessão, erro de API) não deixou nada útil por padrão: confira a worktree e relance com o mesmo escopo.
- Remova a worktree só depois de integrar ou de descartar de propósito; nada com trabalho não commitado some sem olhar antes.

### Quem testa: observar o app e o browser do app

Toda rodada de teste feita por agente usa as skills do `/claudia-teste` e **observa o que realmente aconteceu**, não só o que o agente relatou:

- **O app:** a tela (print e árvore), o estado do banco, o log de diagnóstico (`debug-logs/`) e o que a UI mostra depois de cada ação. Compare a UI com o banco.
- **O browser do app (Claudia browser):** a janela e a aba reais, observadas por CDP e por print: a página carregada, o tamanho da janela, o estado do login, o que o agente leu e o que ficou na tela. Nunca confie só no resultado devolvido pela ferramenta.
- **O Activity log** como terceira fonte: cada chamada tem linha, prints antes e depois, duração e resultado. Divergência entre o relato do agente, o log e a tela é um achado, não ruído.
- **Observar é só leitura:** não clicar em envio, não escrever no banco real, não mexer no perfil do browser nem fechar o app ou o browser do usuário sem pedir. Para ver o browser sem interferir, abra uma janela de teste própria em vez de usar a aba do agente.

### Uso do Superpowers

O Superpowers é o **esqueleto** do processo (brainstorming, spec/plano, execução em worktree, revisão, verificação antes de concluir), mas **não vale cegamente 100%**. Onde as regras deste arquivo são mais estritas, elas vencem:

- a spec precisa de **aprovação explícita do usuário** como passo próprio, mesmo que a skill sugira seguir adiante;
- a fase final é o **smoke de uso real**, que não é opcional;
- a decisão de integrar (merge, push, PR) é **sempre do usuário**, nunca automática ao fim de um ramo.

Skills do Superpowers que ajudam e devem ser usadas: `brainstorming` e `writing-plans` antes de criar, `using-git-worktrees` e `subagent-driven-development` na execução, `requesting-code-review`, `systematic-debugging` para bugs e `verification-before-completion` antes de afirmar que algo funciona.

## Diagnóstico

Em `npm run tauri dev`, lê `debug-logs/events.jsonl` e `debug-logs/pty-tail.log` na raiz do repo **antes** de perguntar o que aconteceu. Em produção (Windows): `%APPDATA%\io.github.johngabie.claudia-rh\diagnostics\`.

## Build

- Frontend: `npm run build` (`tsc` + vite). Typecheck: `npx tsc --noEmit`.
- Backend: em `src-tauri/`, `cargo check` / `cargo test` / `cargo clippy`.
- App: `npm run tauri dev`.

## Teste em máquina limpa

| Quero… | Skill |
|---|---|
| ver, clicar ou conferir o app rodando, tirar print, reproduzir bug | `claudia-teste-e2e` (local ou no Sandbox) |
| montar um Windows limpo, escolher os dados, publicar ou instalar o app, ligar o CDP no guest | `claudia-teste-sandbox` |

A `claudia-teste-sandbox` usa um laboratório Windows Sandbox mantido fora do repositório, localizado pela variável `SANDBOX_LAB_DIR`. Cenários de dados: `novo`, `mock` e `copia` (este só com autorização do usuário na rodada). Falha do app num Windows limpo é resultado do teste: registre no Linear, não contorne no laboratório.

## Dívida conhecida

- Toast nativo Windows: `notificacoes.rs` emite eventos Tauri mas não chama o plugin de notificação — o toast do sistema não dispara.
- Pré-requisitos Chrome / plano da conta: não validados na UI.
- Pesquisa pontual da aba Feedback: não implementada.

## O que este repo já é

v0.2.0. Perfil = `candidate_base.yaml` + `search_variants.yaml` (módulo `src/components/perfil/`). Dashboard em `src/components/dashboard/`. MCP embutido em `src-tauri/src/mcp/`. Browser próprio em `src-tauri/src/browser/` (Chrome for Testing, CDP, ferramentas `browser_*` no MCP); Activity log (sempre ligado) em `src-tauri/src/activity_log/`. Sessão de execução = `claude --no-chrome` no PTY, só com o browser pronto.
