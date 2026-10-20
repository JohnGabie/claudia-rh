# Claudia RH

App desktop Windows (Tauri v2: Rust + React/TypeScript) que automatiza candidaturas com um agente de IA e um browser próprio.

Este arquivo é o ponto de entrada de qualquer sessão de coding **neste repositório**.

## Ordem de leitura

1. `docs/arquitetura-sistema-candidaturas.md` — spec de sistema. Se o que você for construir contradiz este documento, o documento vence — assinale a contradição em vez de a resolver em silêncio.
2. `docs/padrao-design-claudia-rh.md` — identidade visual, tokens, navegação, layout.
3. `src-tauri/src/prompt_sistema_runtime.md` — texto que a app injeta na sessão de execução. **Não edite sem pedido explícito.**
4. Este `CLAUDE.md`.
5. `docs/superpowers/INDEX.md` — acha a spec e o plano da área em que você vai mexer (uma spec viva por área; o que está datado é histórico ou foi absorvido). Convenção em `docs/superpowers/CONVENCAO.md`. O que foi provado e o que falta provar no browser: `docs/superpowers/evidencias/2026-10-feat-browser-cdp-mcp.md`.
6. `docs/estado-do-projeto.md` — o que o repo já é e a dívida conhecida (saiu deste arquivo em 2026-10-19).
7. `agents/` — o papel de cada agente (arquiteto, executor, revisor, QA, versão) e `agents/modelos.md`. Leia o do papel que você vai exercer.
8. `.claude/QUALIDADE.md` — roadmap de qualidade/refactor (quando existir).

**Fonte da verdade, nesta ordem:** spec aprovada, docs, este arquivo. Conversa e memória do modelo **não são fonte**. Se duas fontes conflitam, ou a spec está vaga: pare e pergunte, não decida sozinho.

## Convenções

- **Comentários de código: inglês.** Sempre.
- **Commit messages: inglês.**
- **Identificadores novos: inglês.** Código legado em português não se renomeia em massa.
- **Copy da UI: pt-BR.** Docs vivos (`docs/`, este ficheiro) também em pt-BR.
- **README público: inglês.**

## Git

```
main     → versão fechada. Só recebe release (dev → main + tag vX.Y.Z). Nunca commit direto.
dev      → features completas e estáveis rumo à próxima versão. Nunca commit direto.
feat/*   → uma preocupação por branch, criada a partir de dev atualizado.
fix/*    → igual, para correções.
```

Fluxo: `dev` nunca fica atrás de `main` → `git checkout -b feat/<nome>` a partir de `dev` → cada tarefa com "Looks good to me" vai só ao `backup` → feature completa vira PR contra `dev` → merge só depois do "pode fechar" do usuário → release = PR `dev` → `main` + tag, por ordem do usuário.

**Merge em `dev` e em `main` só por PR no GitHub.** Nenhum merge é feito fora do PR; o `dev` e o `main` locais só avançam por `git pull --ff-only`. Detalhes em `agents/versao.md`.

### Dois remotes

| Remote | Repo | Uso |
|---|---|---|
| `origin` | `JohnGabie/claudia-rh` (público) | `main`, `dev`, tags `v*` e as branches que viram PR. Push **só por ref nomeada**, uma a uma (nunca `--all`, `--tags` nem `--mirror`), e só da lista permitida: `main`, `dev`, `feat/*`, `fix/*`, `sync/*` e tags `v*`. |
| `backup` | `JohnGabie/claudia-rh-backup` (privado) | Espelho do `origin` mais o que ainda não foi publicado (branches de trabalho e a branch `recovery`). `git push backup --all` e `--tags`, nunca `--mirror`. Nunca recebe PR. |

O `backup` só é configurado e recebe o primeiro push com o OK do usuário, depois da varredura de segredos. O push não cobre trabalho não commitado nem arquivos do `.gitignore`. A lista completa e o porquê estão em `agents/versao.md`.

### Do revisor ao merge

O usuário decide **na conversa**, não precisa abrir o GitHub. O fluxo completo está em `agents/versao.md`; em resumo:

1. Cada tarefa com "Looks good to me" do revisor vai **só ao `backup`**, pelo agente de versão. Feature incompleta não vai ao `origin`.
2. Quando **todas** as tarefas da feature têm "Looks good to me", o agente de versão faz a varredura de segredos, empurra a branch ao `origin` por ref nomeada, abre o PR contra `dev` (`--base dev`) e acompanha o CI. Isso não pede outro OK: a autorização é o "Looks good to me" da última tarefa.
3. **Com o CI verde**, o orquestrador mostra ao usuário o resumo (tarefa, commits, verificado, **não** verificado, riscos, estado do CI). Só depois do "pode fechar" do usuário ele chama o agente de versão para o merge do PR, com a decisão registrada no PR. O agente de versão confere o CI de novo na hora do merge e **recusa** se faltar o "Looks good to me", a evidência ou qualquer check verde.

### Branches e agente de versão

O agente de versão (`agents/versao.md`) cuida do backup, do push da branch e do PR (com o acompanhamento do CI), do guardião de estabilidade (recusa declarar a feature pronta para fechar sem revisão em todas as tarefas, rodadas verdes e CI verde), do pacote de recuperação (branch `recovery`, **só no `backup`**), do relatório de saúde das branches (integradas, paradas, sem backup, worktrees sobrando), de tag `archive/*` antes de remover qualquer branch e do rebase de branch parada sobre `dev` (só em branch nunca publicada no `origin`, de preferência antes do primeiro push ao backup). A proteção de branch no `origin` é ativada e desligada só pelo usuário. Remover branch ou worktree: só `git branch -d` e `git worktree remove` sem `--force`, depois da tag de arquivo, de conferir o `.journal/` e o trabalho não commitado da worktree, e com o OK do usuário.

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
- **Nada de caminho absoluto de máquina** (`C:Users<nome>...`, a pasta do usuário, o caminho do laboratório) em código, testes, scripts, skills, planos ou docs. Use variáveis de ambiente (`CARGO_TARGET_DIR`, `USERPROFILE`), caminhos relativos ao repositório ou as APIs (`temp_dir()`, `os.tmpdir()`, `app_data_dir()`). Os que já existem estão numa issue do Linear para ajustar.
- **Nunca usar o Chrome pessoal do usuário** (nem o perfil dele, nem cópia dele) em teste, script ou agente. Só o Chrome do Claudia-RH (o que o app baixa) ou o Chrome de teste com perfil temporário.
- **Teste nunca usa caminho real** (`C:\`, o repositório, `AppData`, arquivo do sistema), nem para ler, nem para provar que a recusa funciona: a recusa se testa na função de validação pura. **Todo teste que apaga qualquer arquivo roda só no Windows Sandbox** (skill `claudia-teste-sandbox`). No PC do usuário: só compilar, clippy, `tsc`, build e testes de front que não tocam o disco. Motivo: em 2026-10-05 um teste que passava `C:\` para uma função de apagar destruiu o disco do usuário.

### Por que a regra dos passos existe

O setup chegou a ter uns 12 passos: instalar o Claude Code, acertar o PATH, instalar a extensão Claude in Chrome, registrar o native messaging host, abrir o app, criar o perfil. A extensão e o native host saíram: o browser agora é baixado pelo app. Cada passo é uma chance do usuário desistir antes de ver o produto funcionando.

Toda solução que "resolve" um problema adicionando um passo está trocando uma dor por outra — e a dor do setup é paga por **todo** usuário, enquanto a dor técnica é paga uma vez por quem constrói.

Na prática, isso descarta de saída: extensões de browser de terceiros (mesmo boas, mesmo open source), qualquer coisa que rode por `npx`, e servidores MCP externos que o usuário precise instalar. Se a capacidade for mesmo necessária, o caminho é embutir — não pedir.

Exceção única: algo que venha **dentro do instalador do Claudia RH**, com a nossa cara e a nossa responsabilidade. Aí não é um passo novo, é parte do produto.

## Processo de desenvolvimento

Objetivo: **maturar o projeto**. Cada mudança deixa o produto mais previsível, mais testado e mais fácil de confiar. As regras abaixo valem para qualquer mudança, em qualquer sessão, independente do assunto.

Onde ficam os artefatos: specs em `docs/superpowers/specs/`, planos em `docs/superpowers/plans/`, índice em `docs/superpowers/INDEX.md`. **Uma spec viva por área** (nome estável, sem data, com histórico de revisões e marcador por seção); mudou o plano, **amplia-se ou reescreve-se** a spec e o plano da área, em vez de criar spec nova. Spec nova só para área nova. As specs datadas que já existem seguem como estão, no índice, até a área ser migrada. Regras completas em `docs/superpowers/CONVENCAO.md`. Tasks, bugs e o que entra em cada versão vivem no Linear (time `CLA`), não em listas neste ficheiro; para registrar e fechar, use a skill `linear-claudia`.

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

O modelo depende da complexidade da tarefa, não do hábito. As regras falam em **camadas** (FORTE, MÉDIO, RÁPIDO); o mapeamento para nomes de modelo vive só em `agents/modelos.md`. O papel de cada agente, o que ele faz, o que não faz e o formato do que devolve estão em `agents/`.

| Etapa | Papel | Camada | Critério |
|---|---|---|---|
| Levantamento de requisitos, brainstorm, spec e plano | Arquiteto (`agents/arquiteto.md`) | FORTE, sempre | Arquiteto e Revisor ficam sempre em FORTE. A spec exige aprovação do usuário (seção 1). |
| Execução | Executor (`agents/executor.md`) em **git worktree**, sempre | RÁPIDO para tarefa mecânica e bem delimitada; MÉDIO quando há lógica nova, vários arquivos ou decisão de desenho; FORTE se toca segurança, dados ou ferramenta do agente | O brief da tarefa traz a camada. |
| Revisão de código | Revisor (`agents/revisor.md`) | FORTE, sempre | Independente de quem escreveu. Obrigatória para segurança, dados e ferramentas do agente. |
| Teste | QA (`agents/qa.md`) | FORTE (recomendado, como já era antes de 2026-10-19) | Observa o app e o browser do app (seção 5) e cruza com o que o agente relatou. |
| Branches, backup, push, PR e CI, merge do PR | Versão (`agents/versao.md`) | RÁPIDO na rotina; MÉDIO em conflito, triagem de CI e relatório de saúde; FORTE só em recuperação de histórico | Não implementa nem revisa mérito. |

**Brief e retorno têm formato fixo** (brief do arquiteto, resposta do executor, veredito do revisor, relatório do QA), descritos nos arquivos de `agents/`. Cada agente recebe um brief curto e os caminhos dos arquivos relevantes, **não o histórico da conversa**.

**Independência e teste:**
- O revisor lê a spec e o diff **antes** da justificativa do executor.
- Quando o arquiteto escreve os testes primeiro, o executor **não os enfraquece**; se parecem errados, pergunta.
- Código que apaga, move ou sobrescreve arquivos em lote: o orquestrador cola as regras de deleção do `CLAUDE.md` global no prompt do subagente.

**Regras de execução por agentes:**
- **Uma preocupação por agente, uma worktree isolada por agente.** Frentes em paralelo só mexem em **arquivos diferentes**; o que as une é um **contrato escrito na spec** (nomes de comandos, assinaturas, formatos). Quem integra é o orquestrador, depois que todas terminarem.
- **A base da worktree pode estar desatualizada.** Antes de começar, o agente confere o commit de base e o alinha com a branch da feature.
- **Cada agente usa o próprio `CARGO_TARGET_DIR`**, fora do `target/` do repositório (o app em desenvolvimento pode estar usando esse).
- **O orquestrador não repassa "passou" sem ter rodado.** Depois de integrar, roda de novo as rodadas da seção 3 e confere o código de saída.
- **O agente não decide fora da spec.** O que a spec não cobre, ele relata e para. Escopo, segurança e edição do prompt de runtime (`src-tauri/src/prompt_sistema_runtime.md`) só com pedido ou aprovação do usuário.
- **Limites do agente:** sem push (ao `origin` só o agente de versão, por ref nomeada, depois do "Looks good to me" da feature completa; ao `backup` só o agente de versão; ver "Dois remotes"), sem tocar nos dados do usuário, no app que o usuário está usando ou em sites reais, salvo autorização explícita daquela rodada.
- **Agente que falha** (limite de sessão, erro de API) deve ser tratado como se não tivesse entregado nada: confira a worktree e relance com o mesmo escopo.
- **Remover worktree** só depois de integrar, ou de descartar de propósito olhando antes se há trabalho não commitado.

#### Ciclo obrigatório executor ↔ revisor (uma tarefa por vez)

Motivo desta regra: a branch já quebrou quando o trabalho foi tocado inteiro numa tacada só, sem passar por desenvolver, testar, validar e aprovar ou recusar. Por isso:

1. **Uma tarefa do plano por ciclo.** Nunca entregar várias tarefas, nem a feature inteira, numa única execução. A tarefa seguinte só começa depois do OK da atual.
2. **O ciclo é:** o **Executor** desenvolve e faz commit → um **Revisor** (agente separado do executor, camada FORTE quando a tarefa toca segurança, dados ou ferramenta do agente; ver `agents/revisor.md`) valida → **aprova ou recusa** → se recusa, o executor corrige e o ciclo recomeça (teste, validação, nova decisão) → repete até o revisor escrever **"Looks good to me"**.
3. **O executor não é encerrado, descartado nem substituído enquanto o revisor não der o "Looks good to me".** Dar a tarefa por concluída, matar o executor ou seguir para a próxima antes disso é violação. Quem corrige é o próprio executor; o orquestrador **nunca corrige no lugar dele** (a correção pularia a revisão). Só se troca de executor nas exceções já previstas: o agente falhou (limite de sessão, erro de API) ou falhou **2 vezes na mesma coisa**, caso em que o orquestrador relança com o mesmo escopo, uma camada acima (`agents/modelos.md`), e o ciclo continua.
4. **Limite de rodadas:** cinco recusas seguidas na mesma tarefa, e o orquestrador **para e pergunta ao usuário**; não segue adiante com a tarefa aberta.
5. **O que o Revisor faz (não basta ler o diff):**
   - **Back:** roda os testes (no Sandbox quando a regra de `cargo test` manda), procura bugs com entradas hostis, banco indisponível e falhas parciais, confere um a um os requisitos da spec e a prova de mutação de cada regra nova.
   - **Front:** abre o app de verdade (skill `claudia-teste-e2e`), olha a tela e responde: está aceitável? está bonita, no padrão de `docs/padrao-design-claudia-rh.md`? está quebrada? atende os requisitos? Confere os estados vazio, erro e com dados, nos dois idiomas, com prints; roda `npx tsc --noEmit`, `npm run build` e os testes de front.
6. **A aprovação é um veredito escrito do revisor, com evidência** (comandos, código de saída, prints). "Passou" sem evidência, ou aprovação de quem só leu o código, não vale para tarefa de front nem de back. Recusa também é escrita, com o achado, o arquivo e a entrada que falha.

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
5. Revisão de código pelo Revisor (camada FORTE, `agents/revisor.md`) feita, com os achados corrigidos ou registrados, e **cada tarefa passou pelo ciclo executor ↔ revisor (seção 2) e tem o "Looks good to me" escrito do revisor**.
6. Nada em "não verificado" que contradiga o que foi afirmado.

**Empurrar a branch ao `origin` e abrir o PR são autorizados pelo "Looks good to me" da feature completa** e feitos pelo agente de versão. **Merge, release, tag `v*` e ativar ou desligar proteção de branch são sempre decisão do usuário**, nunca automáticos ao fim de um ramo.

### 7. Superpowers

O Superpowers é o esqueleto do processo, **mas as regras deste arquivo vencem** onde forem mais estritas: aprovação explícita da spec como passo próprio, smoke de uso real obrigatório e merge e release decididos pelo usuário. Skills a usar: `brainstorming` e `writing-plans` antes de criar; `using-git-worktrees` e `subagent-driven-development` na execução; `requesting-code-review` na revisão; `systematic-debugging` em bugs; `verification-before-completion` antes de afirmar que algo funciona.

## Diagnóstico

Em `npm run tauri dev`, lê `debug-logs/events.jsonl` e `debug-logs/pty-tail.log` na raiz do repo **antes** de perguntar o que aconteceu. Em produção (Windows): `%APPDATA%\io.github.johngabie.claudia-rh\diagnostics\`. O que o agente fez no browser fica no Activity log (`%LOCALAPPDATA%\io.github.johngabie.claudia-rh\dev\activity-log.db`, só leitura; tela em Configurações → Activity log).

## Build

- Frontend: `npm run build` (`tsc` + vite). Typecheck: `npx tsc --noEmit`.
- Backend: em `src-tauri/`, `cargo check` / `cargo clippy --all-targets` / `cargo test --no-run` no PC. **`cargo test` (qualquer um, inclusive `--lib`) roda só no Sandbox**: vários testes criam e apagam pastas, e um deles já apagou o disco do usuário.
- App: `npm run tauri dev`.
- Testes live (Chrome real, `#[ignore]`; **só no Sandbox, nunca no PC**): `CLAUDIA_TEST_CHROME=%TEMP%\cft\chrome-win64\chrome.exe cargo test --test browser_live --test viewport_live -- --ignored --test-threads=2 --skip icon`. Os de ícone abrem janela visível e rodam à parte.
- Se o app estiver aberto, use `CARGO_TARGET_DIR` fora do `target/` do repo: o Windows trava o `.exe` em uso.

## Teste em máquina limpa

| Quero… | Skill |
|---|---|
| ver, clicar ou conferir o app rodando, tirar print, reproduzir bug | `claudia-teste-e2e` (local ou no Sandbox) |
| montar um Windows limpo, escolher os dados, publicar ou instalar o app, ligar o CDP no guest | `claudia-teste-sandbox` |

A `claudia-teste-sandbox` usa o laboratório Windows Sandbox em `sandbox-lab/`, na raiz do repositório. Os ambientes ficam declarados em `sandbox-lab/compose.json` (`test-up.ps1 -Ambiente <nome>`); o ambiente `testes` usa as ferramentas do PC (Rust, Node, MSVC e Windows SDK) montadas **só leitura** em `C:\hosttools` e instala só o Claude Code no guest (cache local em `sandbox-lab/shared/cache/`, baixado por `cache-up.ps1`, que só cria); o `testes-limpo` instala tudo no guest, sem ferramenta do PC. As duas usam a conta Claude logada no PC na hora da subida (só o arquivo de credencial, apagado no `test-down`). Scripts e `compose.json` são versionados; as ferramentas (`sandbox-lab/shared/tools/`, conferidas por `sandbox-lab/shared/tools.sha256`), o cache, a chave do MCP (`sandbox-lab/shared/secrets/`), as saídas e a pasta de trabalho (`.sandbox-work/`) ficam fora do git. De uma worktree, a skill usa o laboratório e a pasta de trabalho do checkout principal e testa o projeto da worktree. Nenhuma variável de ambiente de caminho.

Tudo o que os testes criam ou apagam no guest fica no disco `Z:` (VHD do próprio guest), que não existe no PC. Cenários de dados: `novo`, `mock` e `copia`, montados **só dentro do Sandbox**; o `copia` lê os dados reais só com `test-up -DadosReais` e autorização do usuário na rodada. Falha do app num Windows limpo é resultado do teste: registre no Linear, não contorne no laboratório.

## Estado do projeto e dívida conhecida

O que o repo já é (browser próprio, ferramentas do agente, guarda de envio, diálogos, documentos, lista de sites, Activity log) e a dívida conhecida vivem em **`docs/estado-do-projeto.md`**. Atualize lá quando o estado mudar; este arquivo guarda só regras e fluxo.

Sessão de execução = `claude --no-chrome` no PTY; a sessão espera o browser abrir.

## Diário de contexto antes de compactar

Antes de **qualquer** compactação (manual ou automática) o agente escreve um diário em `.journal/AAAA-MM-DD-HHMMSS-<branch>-<worktree>-<motivo>.md` (`<worktree>` = 8 caracteres de um hash da pasta da worktree) **do checkout principal** (fora do git; mesmo numa worktree, para o diário sobreviver à remoção dela): gere o esqueleto com `scripts/dev/journal-skeleton.ps1` e preencha **todas** as seções (objetivo; decisões e porquês, inclusive opções rejeitadas; preferências e correções do usuário, literais quando possível; feito e verificado com evidência; em andamento; próximos passos em ordem; perguntas abertas; armadilhas; arquivos para reler; o que NÃO refazer; o que promover), apagando os marcadores `<!-- TODO-CONTEXT -->`.

- Os hooks de `.claude/settings.json` aplicam isto: `PreCompact` bloqueia o `/compact` manual enquanto houver marcador (a compactação automática nunca bloqueia) e `SessionStart` reinjeta o último diário ao retomar.
- O diário é rascunho local, não fonte da verdade. O que for durável vai para spec, plano ou `docs/`. Nunca leva segredo.
- Estado de verificação: os scripts foram testados com entrada simulada (`scripts/dev/test-diario.ps1`). **Não verificado:** o comportamento numa compactação real do Claude Code, o nome real do campo de entrada do `PreCompact` (o script lê `trigger` e aceita `compaction_type`; tipo desconhecido nunca bloqueia) e qual shell executa o comando do hook no Windows.

## O humano decide

Cabe ao usuário, nunca ao agente: gosto e identidade visual, escopo e prioridade, aprovação de spec, **merge, release, criar tag `v*`, ativar ou desligar proteção de branch**, segurança e dados do usuário, edição do `prompt_sistema_runtime.md`, qualquer ação irreversível ou que saia da máquina (publicar, enviar candidatura, apagar branch, criar remote). Exceções, feitas pelo agente de versão: o push ao `backup` (depois do primeiro OK do usuário) e o push da branch ao `origin` com a abertura do PR (autorizados pelo "Looks good to me" da feature completa). Na dúvida entre decidir e perguntar, pergunte.
