# Regras de agentes, ciclo de vida das specs e controle de versão — spec

Data: 2026-10-19. Branch: `feat/regras-agentes` (a partir de `dev`). Status: **aprovada pelo usuário em 2026-10-19** (mensagem "blz bora", depois de a spec estar escrita e de eu perguntar se aprovava; interpretei como aprovação, e o usuário pode corrigir). Revisão 2026-10-19, confirmada pelo usuário na conversa (após a primeira revisão independente): ajustes de camadas, push e hooks, ver o fim desta seção de histórico em §9. Ampliação §10 (revisão 3): aprovada em 2026-10-19.  Revisão 4 (P7 reescrita, P16 a P18): aprovada em 2026-10-19. Checks obrigatórios: os 5 jobs.

Esta spec só muda documentos de regras (`CLAUDE.md`, novos arquivos de papéis, convenção de specs, hooks de contexto). Não toca código de produto nem o prompt de runtime.

## 1. Pedido do usuário (palavras dele)

- "i made better roules for calude in this project i wanna adopt in this project claudia too, but need to study what we gonna change and what gonna keep, on smart pet, i loved the agents.md and the flow of developing"
- "ai teria que adaptar para as ferramentas que ja estao aqui"
- "aqui o agente de versao, eu posso pedir pro claude aprovar por terminal, no outro ele me forca abrir o github. o versao, tbm lida com backup remote e sanitizacao das branchs."
- "junto com o arquiteto, eu preciso que ele tbm seja bom de gerenciar as specs pq hoje qualquer modificacao no plano vira uma spec nova que vira plano, branch e fica tudo espaguetificado [...] precisa corrigir isso, para quando fizer sentido so expandir ou reescrever uma spec e planos (parte ou totalmente)"
- "tem como ter 2 repo, eu queria ter um privado somente para backups ai fazer push constantes nao é um problema, no publico abrimos pr e fazemos como deve ser feito" (repo de backup: `https://github.com/JohnGabie/claudia-rh-backup.git`, privado, vazio)
- "lembra de voltar para as branchs e fazer rebase"

## 2. Leitura do pedido

1. Trazer do `smart-pets-mod` o formato de papéis em arquivos (`agents/`), adaptado às ferramentas daqui (skills Superpowers, Linear, worktrees, Sandbox, `gh`). Sem nada específico de Minecraft.
2. Um agente de versão: aprovação e merge pelo terminal, remote de backup privado, higiene de branches.
3. Um arquiteto que mantém specs e planos como documentos vivos, em vez de criar spec nova a cada ajuste.
4. Dois remotes: `origin` (público, PRs) e `backup` (privado, push constante).
5. Rebase das branches paradas sobre `dev` (operação à parte, ver §6).

Estado verificado em 2026-10-19 (só leitura): 21 specs e 17 planos com nome por data no `dev` (a contagem inicial de 18 e 15 veio de outro checkout, mais antigo), várias cadeias da mesma área (browser: 4 specs; sandbox: 3); 9 branches de feature locais, nenhuma no `origin`; `dev` local 84 commits à frente do `origin/dev`; `main` parada desde julho; 3 branches já integradas ainda existentes; 5 com trabalho não integrado.

## 3. Decisões do usuário (respondidas em 2026-10-19)

| Tema | Decisão |
|---|---|
| Specs | **Spec viva por área**: arquivo de nome estável, histórico de revisões, seções marcadas. Plano idem. |
| Limite de recusas do revisor | **5 rodadas** (mantém o valor atual). |
| Agente de versão | **Completo**: backup remoto, push de todas as branches, relatório de saúde, tag `archive/*`, aprovação e merge pelo terminal. |
| Diário de contexto antes de compactar | **Entra nesta leva**, com hooks reescritos para Windows. |

## 4. Propostas do assistente (NÃO DECIDIDAS, cada uma com recomendação)

### P1. Pasta `agents/` com um arquivo por papel
Arquivos: `agents/arquiteto.md`, `executor.md`, `revisor.md`, `qa.md`, `versao.md`, `modelos.md`. Cada um com "Papel / Faz / Não faz / formato de saída". O orquestrador cola o arquivo do papel no prompt do subagente. As skills Superpowers continuam sendo o esqueleto; os arquivos só fixam o que o papel faz e devolve.
**Recomendo.**

### P2. Camadas de modelo em um único lugar
`agents/modelos.md` mapeia FORTE, MÉDIO e RÁPIDO para Opus, Sonnet e Haiku. O `CLAUDE.md` e os papéis falam só em camadas. Regra de subida: executor que falha 2 vezes na mesma coisa sobe uma camada. Revisor e arquiteto ficam em FORTE.
**Recomendo.**

### P3. Formatos obrigatórios
- Brief de tarefa: ID, objetivo, contexto (caminhos), fora de escopo, arquivos que pode tocar, contrato, testes, critério de aceite, camada, riscos, dependências.
- Resposta do executor: status, branch/PR, mudanças, verificado (com comando), não verificado, armadilhas, dúvidas.
- Veredito do revisor: APROVADO, REJEITADO-CORRIGIR ou REJEITADO-REFAZER, com problemas (arquivo, regra, o que fazer), evidência e se precisa de teste humano. "Looks good to me" permanece como a frase da aprovação.
- Relatório do QA: cada critério PASSOU, FALHOU ou NÃO VERIFICADO, com evidência.
**Recomendo.**

### P4. Regras de independência e de teste
- O revisor lê spec e diff **antes** da justificativa do executor.
- Quando o arquiteto escreve testes primeiro, o executor não os enfraquece; se parecem errados, pergunta.
- Conversa e memória do modelo não são fonte da verdade; vale a ordem: spec aprovada, docs, `CLAUDE.md`.
**Recomendo.**

### P5. Seção "O humano decide" no `CLAUDE.md`
Reúne o que hoje está espalhado: gosto e identidade visual, escopo, merge, release, segurança, qualquer ação irreversível ou que saia da máquina.
**Recomendo.**

### P6. Specs e planos vivos (ciclo de vida)
- **Uma spec por área** em `docs/superpowers/specs/<area>.md` (ex.: `browser.md`, `sandbox.md`), sem data no nome. O plano correspondente em `docs/superpowers/plans/<area>.md`.
- Um **índice** `docs/superpowers/INDEX.md`: área, spec, plano, status, branches ativas, specs antigas que ela absorveu.
- Cada spec tem **histórico de revisões** (data, o que mudou, quem aprovou) e cada seção um marcador: `estável`, `ampliada AAAA-MM-DD`, `reescrita AAAA-MM-DD`, `rascunho`.
- **Mudança de plano no meio do caminho** = editar a spec (ampliar ou reescrever o trecho) e o plano (tarefa marcada `feita`, `obsoleta` ou `nova`), no mesmo ramo. Só o **trecho alterado** pede nova aprovação (já é a regra de hoje).
- **Spec nova** só para uma área nova. Critério: a mudança não cabe em nenhuma seção de spec existente.
- O arquiteto decide e justifica "ampliar, reescrever ou nova" em uma linha, e o usuário confirma na aprovação.
- **Specs antigas** não são migradas agora (ver §6). Ficam como estão, marcadas no índice como `histórica` ou `absorvida por <área>`.
**Recomendo.**

### P7. Agente de versão
- **Remote `backup`** = `https://github.com/JohnGabie/claudia-rh-backup.git` (privado). `origin` continua o repo público.
- **Push ao backup**: `git push backup --all` e `--tags`, sem cerimônia, ao fim de cada tarefa, após cada merge e antes de qualquer operação arriscada. Nunca `--mirror` (apagaria no backup o que for apagado localmente).
- **Origin** *(reescrita 2026-10-19, aprovada)*: só `dev`, `main`, tags `v*` e as branches de PR (`feat/*`, `fix/*`, `sync/*`). **Push ao `origin` sempre com refs nomeadas uma a uma; nunca `--all`, `--tags` nem `--mirror`** (publicariam no repo público as branches de trabalho, a `recovery` e as tags internas `archive/*`, `backup/*`, `pre-rebase/*`).
- **Do revisor ao merge** *(reescrita 2026-10-19, aprovada; substitui "merge local ou `gh pr merge`, PR é só registro")*:
  1. O revisor escreve "Looks good to me" e **chama o agente de versão**, passando a branch, a tarefa e a evidência.
  2. O agente de versão (camada RÁPIDO ou MÉDIO, conforme a carga; decisão do usuário) empurra a branch **ao `backup`** a cada tarefa aprovada. *(Reescrita 2026-10-19, decisão do usuário "Só no fim da feature":)* **só quando todas as tarefas da feature tiverem "Looks good to me"** ele faz a varredura de segredos, empurra a branch ao `origin` (ref nomeada), abre o PR contra `dev` (`gh pr create --base dev`, modelo de `.github/PULL_REQUEST_TEMPLATE.md`) e acompanha o CI (`gh pr checks`). Nada de feature incompleta no `origin`. Não precisa de OK do usuário para empurrar e abrir o PR: a autorização é o "Looks good to me" do revisor na última tarefa.
  3. Devolve ao revisor um relatório (o que empurrou, link do PR, estado do CI). O revisor confere e **dispensa** o agente de versão se estiver tudo certo; se não, devolve com o que falta. CI que falha segue a P13.
  4. **O merge continua sendo do usuário**: com o CI verde, **o orquestrador** mostra o resumo ao usuário (um subagente não fala com ele) e, só depois do "pode fechar" dele na conversa, chama o agente de versão, que confere de novo o CI e faz `gh pr merge --merge`, registrando a decisão no PR.
  5. Depois do merge: `git fetch origin`, o `dev` (ou `main`) local avança **só por `git pull --ff-only`**, e o agente empurra ao `backup`. **Não existe mais merge local em `dev` ou `main`**: um merge local e o merge do PR gerariam commits diferentes para o mesmo trabalho, e o `dev` local divergiria do `origin/dev`.
- **Saúde das branches** (relatório sob demanda, e ao voltar de pausa): integradas, paradas há mais de 14 dias, sem backup, defasagem em relação a `dev`, worktrees sobrando, trabalho não commitado. Remoção só com tag `archive/<branch>` empurrada ao backup e OK explícito.
- **Sem force push, sem reescrever histórico publicado.** Rebase só em branch nunca publicada no `origin`; no `backup` o rebase exige `--force-with-lease` e é feito **antes** do primeiro push.
- **Checagem de segredos** antes do primeiro push ao backup e antes de cada PR: varredura do histórico por chaves, tokens e caminhos pessoais.
- Não faz sem o usuário: merge, apagar branch remota, force push, publicar release, mudar visibilidade de repo, criar remote novo.
**Recomendo.**

### P8. Diário de contexto antes de compactar (entra nesta leva)
- Pasta `.journal/` (no `.gitignore`), um arquivo por diário: objetivo, decisões e porquês, preferências do usuário, feito e verificado, em andamento, próximos passos, perguntas abertas, armadilhas, o que não refazer.
- `.claude/settings.json` com hooks `PreCompact` (bloqueia `/compact` manual enquanto o diário estiver incompleto; a compactação automática nunca bloqueia) e `SessionStart` com `compact|resume` (reinjeta o último diário).
- Scripts em **PowerShell** (shell principal do projeto), versionados em `scripts/dev/`. Nada de `.sh` ou `python3`.
- O diário não é fonte da verdade e nunca leva segredos. O que for durável é promovido às specs.
**Recomendo.**

### P9. Enxugar o `CLAUDE.md`
O `CLAUDE.md` do repo fica com regras e fluxo. A seção "O que este repo já é" e a "Dívida conhecida" vão para `docs/estado-do-projeto.md`. O `CLAUDE.md` aponta para ele.
**Recomendo.**

### P10. Template de PR
Garantir as seções "O que mudou", "Como foi verificado" e "NÃO verificado". Hoje o template existe; **não verificado** se já as tem.
**Recomendo.**

## 5. Impacto

- **Documentos**: `CLAUDE.md` (do repo e a ponte do workspace, se fizer sentido) reescrito em partes; novos `agents/*.md`, `docs/superpowers/INDEX.md`, `docs/estado-do-projeto.md`; template de PR.
- **Configuração**: `.claude/settings.json` novo (hooks), `.gitignore` com `.journal/`, remote `backup`.
- **Scripts**: PowerShell para o diário de contexto e seus hooks (entregues). O relatório de saúde de branches é feito por comandos git descritos em `agents/versao.md`; um script próprio e o script de backup ficam **adiados** (o primeiro push ao backup só acontece por pedido do usuário).
- **Dados/segurança**: nenhum código que apague arquivo. A remoção de branch é `git branch -d` com tag de arquivo antes; nenhum script apaga pasta. Segue o `CLAUDE.md` global de deleção.
- **Fora do repo**: o repo `claudia-rh-backup` recebe commits assim que o usuário autorizar o primeiro push.

## 6. Fora do escopo

- Migrar as 21 specs e 17 planos existentes para specs vivas (ciclo próprio, por área, com aprovação do usuário a cada área).
- O rebase das branches paradas sobre `dev` e a limpeza das integradas: operação feita **depois** desta spec aprovada e **antes** do primeiro push ao backup, uma branch por vez, parando em conflito.
- Backup de dados fora do git (`debug-logs`, `.sandbox-work`, dados do usuário).
- Qualquer mudança em `prompt_sistema_runtime.md` ou em código de produto.
- O trabalho não commitado em `feat/lista-de-sites` (`Cargo.toml`, `.vite/`, `prototipo-playwright/`): decisão do usuário, à parte.

## 7. Critérios de verificação

- Revisão de leitura: cada regra nova em um só lugar; nenhuma contradição entre `CLAUDE.md`, `agents/*.md` e o índice; sem nome de modelo fora de `modelos.md`; sem caminho absoluto de máquina.
- Hooks: o script de PreCompact devolve código 2 com diário incompleto e 0 com diário completo (teste com entrada simulada, pasta temporária criada pelo próprio teste, sem apagar nada). Sessão real de compactação: **não verificado** até acontecer.
- Backup: `git remote -v` mostra `backup`; `git push backup --all` só depois do OK e da varredura de segredos; conferir com `git ls-remote backup`.
- Índice: toda spec e todo plano existente aparece uma vez.

## 8. Pontos em aberto / não verificado

- Se o hook `PreCompact` do Claude Code no Windows realmente bloqueia `/compact` e qual é o nome real do campo de entrada (o script lê `trigger` e, como reserva, `compaction_type`; tipo desconhecido nunca bloqueia): **não verificado**. Também **não verificado**: qual shell executa o comando do hook no Windows (Git Bash, PowerShell ou cmd), de que depende a expansão de `${CLAUDE_PROJECT_DIR}`.
- Se o template de PR atual tem "NÃO verificado": **não verificado**.
- Se `gh pr merge` por terminal basta para o fluxo do usuário sem abrir o GitHub: **não verificado**.
- Nome final das áreas (`browser`, `sandbox`, `documentos`, `perfil`, `testes`, `processo`...): proposta, o usuário ajusta ao migrar.
- Se o `CLAUDE.md` da pasta workspace (`curriculum-apply/`, fora do git) também será ajustado: **não verificado**; fora do repo.

## 9. Histórico de revisões desta spec

| Data | Seções | Tipo | O que mudou | Aprovação |
|---|---|---|---|---|
| 2026-10-19 | todas | criada | Versão inicial com P1 a P10 e as 4 decisões do usuário. | "blz bora" (2026-10-19) |
| 2026-10-19 | P2, P3, P7, P8, §5, §6, §8 | ajustada | Após a revisão independente: arquiteto sempre FORTE; QA mantido em FORTE recomendado (como era no `CLAUDE.md` antes); "push" nas regras de integração passa a significar `origin`, com a exceção do push ao `backup` pelo agente de versão; rebase de branch já no backup exige OK explícito; hook lê `trigger` com `compaction_type` de reserva e nunca bloqueia tipo desconhecido; `.claude/settings.json` passa a ser versionado; scripts de saúde e de backup adiados; contagens corrigidas. São correções de coerência dentro do que o usuário já aprovou; os itens "Extras" do plano pedem confirmação dele. | usuário confirmou na conversa em 2026-10-19 (spec aprovada e extras mantidos) |
| 2026-10-19 | §10 | ampliada | P11 a P14 (guardião de estabilidade, espelho no backup, PR e CI, pacote de recuperação) e P15 (proteção de branch). | usuário aprovou na conversa em 2026-10-19; checks "1 + linux" a confirmar |
| 2026-10-19 | P7, 10.3, P13, P14, P15, 10.10 | reescrita | Revisão 4: merge só por PR no GitHub e `dev` local só por `pull --ff-only`; o revisor chama o agente de versão, que empurra e abre o PR sem OK do usuário (resposta do usuário); push ao `origin` só por ref nomeada; `sync/*` na lista do hook; propostas P16 a P18. | usuário aprovou em 2026-10-19: revisão 4, merge com o "pode fechar" dele, P16 a P18, FORTE só na recuperação, os 5 jobs obrigatórios |
| 2026-10-19 | P7 (passo 2) | reescrita | Resolve a contradição P7 x P11 apontada pelo revisor da T8: cada tarefa aprovada vai só ao backup; a branch vai ao `origin` e ganha PR (contra `dev`) só quando a feature inteira tiver "Looks good to me". | decisão do usuário em 2026-10-19: "Só no fim da feature" |
| 2026-10-19 | P7 (passo 4), P11 | esclarecida | Apontado pelo revisor da T9: quem mostra o resumo e pede o "pode fechar" é o orquestrador (um subagente não fala com o usuário); o versionador confere de novo o CI na hora do merge e recusa se não estiver verde. P11 deixa de citar `merge --no-ff`. Não muda quem decide: o merge segue do usuário. | usuário confirmou em 2026-10-19 |
| 2026-10-19 | P12 | ampliada | Apontado pelo revisor da T10: tag do `origin` tem de apontar para o mesmo commit no `backup`; branch continua aceitando o backup à frente. | decisão do usuário em 2026-10-19: "Tag tem que ser igual" |
| 2026-10-20 | P14, P17, P15 (hook) | ampliada | Sobras das revisões da T11 e da T12, decididas pelo usuário: o hook pre-push normaliza a URL (decodifica `%xx`, resolve `.` e `..`, tira barra final) antes de reconhecer o público; o hook recusa force push (atualização que não contém o commit anterior) em qualquer ref ao público; o formato das tags `v*` fica como está (`v` + dígitos e pontos, sufixo opcional); o pacote de recuperação passa a pegar `accessToken`/`apiKey` e parecidos em texto, a recusar junction nos níveis entre a pasta global e a memória, e a mascarar segredos também nas mensagens de erro do git. | decisões do usuário em 2026-10-20 |
| 2026-10-20 | P7, P14, P15 | ampliada | Após a revisão do conjunto (rodada 1), decidido pelo usuário: sem limite de tamanho de PR ("nao ligo"; o extra "≲ 400 linhas" sai); o hook aceita no máximo UMA branch de trabalho por push ao público (tags `v*` à parte; `--all` com mais de uma branch de trabalho é recusado; com uma só, equivale a um push nomeado); os testes dos scripts rodam tanto pelo PowerShell quanto pelo Git Bash. Decisões do orquestrador, dentro do aprovado: varredura de segredos por script próprio (`scripts/dev/scan-secrets.ps1`) com a mesma lista de padrões do pacote de recuperação; o hook é instalado por cópia na pasta de hooks comum do repositório (vale para toda worktree e toda branch), por script de instalação, em vez de `core.hooksPath` relativo; antes da primeira execução real do pacote de recuperação (sem `refs/heads/recovery`), o agente roda só `-WhatIf` e pergunta ao usuário. | decisões do usuário em 2026-10-20 |
| 2026-10-20 | P14, §10.5 | esclarecida | O pacote de recuperação grava por índice temporário, sem worktree; leva todos os diários (como implementado e aprovado na T11). | não muda regra |
| 2026-10-20 | vários | esclarecida | Textos alinhados à revisão do conjunto, rodada 1 (B1-B4) e às T21/T22/T24. | não muda regra |

## 10. Ampliação (revisão 3, 2026-10-19): estabilidade, backup, PR e CI, pacote de recuperação, proteção de branch — **aprovada pelo usuário em 2026-10-19** (ver 10.8)

Marcador: `ampliada 2026-10-19`. Amplia a P7; não contradiz o que está aprovado. Só este trecho pede nova aprovação.

### 10.1 Pedido do usuário (palavras dele)

- "algo que eu gostaria de add é agent-versao, ele tem que cuidar para apenas cuidar para dev e main serem sempre versoes e branch estaveis. e no outro sempre dando push quando puder, naquele backup vamos subir as skills etc... nao env, mas configuracoes para ajudar caso se perca podemos recupear onde paramos sem dor de cabeça, nesse backup é importante ser uma copia exata da publica, alem de conter essa versao de backup"
- "A publica sempre vai conter as versoes "estavel" na dev sao as fetures em desenvolvimento para compor a 0.3v na man vai compor ai a 0.3v fechada e estavel. Na privada, talvez nao precise da main da publica, mas a dev sim, e la vai ser o backup da publica, onde vai haver mais commits constantes. isso faz sentido? Esse agente é resoposnavel tbm para lidar com os pr, CI, em caso de falha repassar a outros agentes responsavel, para pedir sugestao."
- Cadência do push: "Só o agente de versão" (sem hook automático).
- Pacote de recuperação: config do projeto, config global e memória, estado de trabalho; "cuidado com a limpeza desses dados, para nao contaminar a publica".
- Sobre o que é "estável": "falei da publica, mas eu expliquei melhor agora" (vale o que ele descreveu acima).

### 10.2 Leitura do pedido

1. No **público** (`origin`): `dev` junta as features já estáveis rumo à 0.3; `main` recebe só a 0.3 fechada e estável (release).
2. No **privado** (`backup`): o `dev` é o backup do `dev` público, com mais commits e mais constantes; o `main` talvez não precise.
3. Só o agente de versão faz push (ao backup e, quando o usuário mandar, ao origin). Sem hook automático.
4. O agente de versão também cuida de PR e CI e, se algo falhar, repassa a quem é responsável e pede sugestão.
5. O backup guarda também um pacote para retomar o trabalho sem dor (config do projeto, config global e memória, estado de trabalho), limpo de segredos e sem nunca ir ao repo público.

### 10.3 "Isso faz sentido?"

Sim, com três ressalvas que viram regra:
- Os "commits constantes" só existem em **branches `feat/*` e `fix/*`**, porque commit direto em `dev` continua proibido. *(Reescrita 2026-10-19, aprovada:)* com o merge só por PR (P7), o `dev` do backup é **igual** ao `origin/dev` depois de cada merge, atualizado pelo agente de versão; o que é a mais no backup são as branches de trabalho e a `recovery`. Invariante verificável: toda ref do `origin` é igual ou ancestral da mesma ref no `backup`. **Exceção única, a desfazer na T14:** hoje o `dev` local está 84 commits à frente do `origin/dev`, fruto de merges locais antigos; o PR de sincronização resolve isso uma vez.
- "Estável" no `dev` público é por **feature completa**, não por commit: a feature só entra quando passou pela porta da P11.
- Se o `main` não for ao backup, a release fica restaurável só pelo `origin` e pelas tags. Custa quase nada espelhar o `main` e as tags `v*`; recomendo espelhar (P12).

### 10.4 Propostas do assistente (aprovadas em 2026-10-19; os trechos marcados "reescrita 2026-10-19" foram aprovados na revisão 4)

**P11. Guardião da estabilidade de `dev` e `main` públicos.**
- `main`: só release (`dev` → `main` + tag `vX.Y.Z`) da versão **fechada**, depois do smoke de uso real anexado; o usuário manda.
- `origin/dev`: só feature completa, com "Looks good to me" por tarefa, unitários, integração e regressão verdes (`cargo test` só no Sandbox; `tsc`, build e clippy no PC) e smoke da feature quando couber. Feature incompleta fica em branch e vai só ao backup.
- `dev` nunca fica atrás de `main`; sem commit direto; merge só pelo PR (`gh pr merge --merge`), depois do "pode fechar" do usuário. *(Atualizada 2026-10-19: o `merge --no-ff` local deixou de existir na revisão 4.)*
- `dev` ou `main` quebrou: `git revert` do merge (nunca reset, nunca force push), aviso ao usuário e ao arquiteto, tarefa no Linear.
- O agente **recusa** o merge se faltar o "Looks good to me", a evidência das rodadas ou o CI verde, e diz o que falta.
- Ponto em aberto: `main` e `dev` **não têm proteção de branch no GitHub** (verificado em 2026-10-19: "Branch not protected"). Ativar proteção (PR obrigatório, CI obrigatório) é decisão do usuário; sem ela a estabilidade depende só do agente e das regras locais.
- **Recomendo.**

**P12. Backup como espelho do público mais o que ainda não foi publicado.**
- Invariantes, verificadas por um relatório só de leitura que compara `git ls-remote origin` com `git ls-remote backup`: toda ponta de branch do `origin` é igual ou **ancestral** da mesma ref no `backup`; *(ampliada 2026-10-19, decisão do usuário:)* toda **tag** do `origin` aponta para o **mesmo** commit no `backup` (tag diferente é erro, mesmo que o backup esteja à frente), para que restaurar uma release pelo backup traga exatamente o código publicado; nenhuma ref do `backup` apaga ou reescreve história do `origin`.
- Espelhar também `main` e as tags `v*` (a restauração da release fica completa). *Alternativa: dispensar o `main`, como o usuário sugeriu; perde a restauração direta da release.*
- Push só pelo agente de versão, nos momentos descritos em `agents/versao.md` (fim de tarefa aprovada, depois de merge, antes de operação arriscada, ao encerrar a sessão). `--all` e `--tags`, nunca `--mirror`.
- **Recomendo.**

**P13. PR e CI sob o agente de versão.**
- *(Reescrita 2026-10-19, aprovada:)* depois do "Looks good to me" da feature completa (todas as tarefas do plano), o agente de versão abre o PR no `origin` e acompanha o CI (o `ci.yml` roda em push para `main` e `dev` e em PR), como no fluxo "Do revisor ao merge" da P7. O merge só com CI verde e o "pode fechar" do usuário.
- **CI falhou:** o agente **não corrige**. Lê o log (`gh run view --log-failed`, só leitura), classifica e repassa no formato de brief, com o trecho do log, **pedindo sugestão** e não só correção: falha de código ou teste vai para o executor da tarefa (o revisor confere); contradição com a spec vai para o arquiteto; falha de ambiente, instável ou de infraestrutura vira tarefa no Linear e pergunta ao usuário; qualquer coisa de segurança vai para o revisor em FORTE. Conta as rodadas; na 5ª, para e pergunta ao usuário.
- O `release.yml` roda em tag e **publica** (release e npm). O agente nunca cria tag `v*` sem o usuário mandar.
- **Recomendo.**

**P14. Pacote de recuperação na branch órfã `recovery`, só no backup.**
- Conteúdo (escolhido pelo usuário): (a) config do projeto: `.claude/` (skills, comandos, `settings.json`, `settings.local.json` se a varredura passar) e `.mcp.json` sem segredos; (b) config global e memória: `CLAUDE.md` global, lista de plugins instalados, a pasta de memória deste projeto, `settings.json` global só se a varredura passar; (c) estado de trabalho: `STATE.md` (branches, worktrees, o que está parado, próximo passo, ligação com o Linear), patches do trabalho não commitado e todos os diários do `.journal\` do checkout principal e os que só existem no `.journal\` de alguma worktree.
- **Fora, sempre:** `.env*`, credenciais, tokens, `sandbox-lab/shared/secrets/`, dados pessoais do perfil (`candidate_base.yaml`, `search_variants.yaml`, `strategy.md`, `disparo.json`, `notif.json`, `modo_autonomo.json`) e o banco do usuário.
- **Limpeza antes de gravar:** lista de caminhos permitidos (allowlist: nada entra por padrão) mais varredura de segredos, com os mesmos padrões usados em 2026-10-19, no que vai ser commitado; achou algo, não grava e avisa. O script só cria e sobrescreve arquivos **dentro da sua própria pasta temporária** (a gravação é por índice temporário e plumbing, sem worktree); não apaga nada. Fora dessa pasta, o script grava objetos no `.git` do repositório e a ref `refs/heads/recovery`.
- **Isolamento para não contaminar o público:** a branch `recovery` é órfã (sem ancestral comum com `dev`), então não há merge acidental; o agente só empurra ao `origin` o que está na lista permitida (`main`, `dev`, `feat/*`, `fix/*` e `sync/*` com PR, tags `v*`), sempre por ref nomeada, e nunca `recovery` ou `wip/*`; um hook `pre-push` versionado recusa `recovery*` no `origin`; o conteúdo do pacote nunca vive numa pasta rastreada do repo de trabalho.
- Atualização do pacote: pelo agente de versão, no fim da tarefa, ao mudar a config e antes de operação arriscada. Não é automática.
- **Recomendo.**

### 10.5 Impacto

- Documentos: `agents/versao.md` ampliado (P11 a P14), `CLAUDE.md` (Git, "O humano decide") e o template de PR.
- Scripts novos, só de leitura ou que só criam: relatório de verificação do backup (`scripts/dev/verify-backup.ps1`), geração do pacote de recuperação (`scripts/dev/recovery-pack.ps1`, em pasta temporária com índice temporário e plumbing, sem worktree) e o hook `pre-push` que bloqueia `recovery*` no `origin`.
- Configuração: nenhuma mudança no GitHub sem o usuário (proteção de branch, secrets).
- Segurança: pacote de recuperação com varredura obrigatória; nenhuma deleção em lote.

### 10.6 Fora do escopo

Ativar proteção de branch no GitHub; criar a release 0.3; backup do disco fora do git; qualquer mudança em `ci.yml` ou `release.yml`.

### 10.7 Não verificado

- O conteúdo dos arquivos de config global (`settings.json` com ou sem segredo) e do `settings.local.json`: não lidos.
- Se o hook `pre-push` em PowerShell roda no Git for Windows como esperado.
- O que o `ci.yml` cobre exatamente além de build (a leitura foi parcial); os testes Rust do CI não substituem o Sandbox.
- Se `gh pr checks` e `gh run view` bastam para o fluxo do usuário sem abrir o GitHub.

### 10.8 Decisões do usuário sobre a seção 10 e a P15 (2026-10-19)

Respostas dele, na conversa, depois de a seção 10 estar escrita:

- Aprovou a seção 10 (P11 a P14) e pediu a proteção de branch: "vamos fazer a protecao da branch, na publica sempre abrir um pr e dps de aprovado fechar".
- Checks obrigatórios: respondeu "1 + linux" e, perguntado de novo, confirmou "Os 5 jobs" (2026-10-19): os 5 jobs do `ci.yml` (`npm ci (build)`, `npm install (fresh)`, `repo guard (lab files, keys, machine paths)`, `cargo test` e `windows (exe)`); os quatro primeiros já rodam em Linux (`ubuntu-latest`) e o último em Windows.
- PR obrigatório, **0 reviews exigidos pelo GitHub**, administrador **sujeito às regras** (a aprovação do usuário é dada na conversa).
- Extras: sem force push e sem apagar `dev`/`main`; branch atualizada antes do merge; só merge commit; proteger as tags `v*`.

### 10.9 P15. Proteção das branches `main` e `dev` no `origin` (aprovada nas linhas acima; os detalhes de execução são propostas minhas, **não decididas**)

Regras a aplicar no GitHub, por `gh api`, **só com o "pode aplicar" do usuário**:

1. **Ruleset ou proteção para `main` e `dev`**: PR obrigatório com 0 aprovações; checks obrigatórios (os 5 jobs acima, confirmados pelo usuário); branch atualizada antes do merge (`strict`); sem force push; sem apagar a branch; administradores sujeitos às regras (`enforce_admins`).
2. **Repositório**: `allow_squash_merge=false` e `allow_rebase_merge=false`, mantendo só merge commit.
3. **Tags `v*`**: ruleset que impede mover e apagar. *Proposta minha:* criar a tag continua possível ao usuário, porque o `release.yml` roda em tag e precisa de um jeito de lançar; o agente de versão nunca cria tag `v*` sozinho.
4. **Fluxo**: "sempre abrir um PR e, depois de aprovado, fechar" segue o fluxo "Do revisor ao merge" da P7 *(reescrita 2026-10-19, aprovada)*: o revisor aprova o código e chama o agente de versão (depois do "Looks good to me" da feature completa, para o `origin`; a cada tarefa aprovada só o `backup`); ele empurra e abre o PR sem pedir OK ao usuário; o merge (`gh pr merge --merge`) só depois do "pode fechar" do usuário. Nada de push direto em `main` ou `dev`, nem de merge local nelas.
6. **Hook `pre-push`** *(reescrita 2026-10-19, aprovada)*: a lista permitida no `origin` inclui `sync/*`, usada uma vez no PR de sincronização (T14); `recovery*`, `wip/*`, `archive/*`, `backup/*` e `pre-rebase/*` são sempre recusadas no `origin`.
5. **Reversão**: os comandos para desligar cada regra ficam escritos junto dos de ativar, porque o administrador também está sujeito a elas (emergência = desligar a regra, agir, religar, tudo com aviso ao usuário).

**Consequência a decidir antes de ativar (proposta minha):** o `dev` local está 84 commits à frente do `origin/dev`. Com a proteção ativa, esses commits só chegam ao `origin` por PR. Recomendo **publicar por um PR de sincronização antes ou logo depois de ativar** (branch `sync/dev-AAAA-MM-DD` apontando para o `dev` local, PR para `origin/dev`), conforme a regra "sempre PR". A alternativa, um último push direto antes de ativar, quebra a regra que o usuário pediu e **não** é recomendada.

**Não verificado:** se os nomes dos jobs são aceitos como contexto de check obrigatório sem ajuste (conferir com `gh api repos/.../commits/<sha>/check-runs` antes de ativar); se o plano gratuito do GitHub aplica proteção e ruleset de tag a repositório público (aplica, mas confirmar na resposta da API); se `enforce_admins` e `strict` bloqueiam o `release.yml` em tag.

### 10.10 Revisão de 2026-10-19 (revisão 4): resposta do usuário e propostas novas — **aprovada pelo usuário em 2026-10-19**

Origem: revisão de leitura feita na camada FORTE sobre a branch inteira, depois da troca de modelo pedida pelo usuário. Os achados que mudam regra foram corrigidos **no lugar**, nos trechos marcados "reescrita 2026-10-19" (aprovados) (P7 "Origin" e "Do revisor ao merge", 10.3, P13, P14, P15 itens 4 e 6).

**Pedido do usuário (palavras dele), sobre quem empurra e abre PR:** "apos o revsior dizer que o codigo esta ok, ele vai conversar com versionador, ele vai fazer o trabalho. esse agente vai ser algo como sonnet ou haiku dependendo da carga do trabalho, ele vai devolver o que foi feito e o revisador ai dispensar do trabalho se tudo tiver ok."

**Leitura:** o "Looks good to me" do revisor autoriza o agente de versão a empurrar e abrir o PR, sem pedir OK ao usuário; o agente de versão roda em RÁPIDO ou MÉDIO conforme a carga; ele devolve o que fez ao revisor, que confere e o dispensa. **Confirmado pelo usuário (2026-10-19):** o merge do PR continua com o "pode fechar" dele, como já estava na regra "integrar é decisão do usuário" e na resposta dele "na publica sempre abrir um pr e dps de aprovado fechar".

**Propostas novas (aceitas pelo usuário em 2026-10-19):**

**P16. Diário de contexto no checkout principal.** Hoje o diário vai para `.journal/` dentro da pasta do projeto da sessão; numa worktree, é a pasta da worktree. Pela documentação do git, `git worktree remove` sem `--force` não recusa arquivo **ignorado**, e o `.journal/` é ignorado: o diário seria apagado junto com a worktree (não testado; teste que apaga só roda no Sandbox). Proposta: os scripts gravam e leem em `.journal/` do checkout principal, achado por `git rev-parse --git-common-dir`; e o agente de versão confere o `.journal/` antes de remover qualquer worktree. **Aceita pelo usuário em 2026-10-19.**

**P17. Limpeza estrutural do pacote de recuperação.** A varredura por regex não pega token de formato desconhecido dentro de um `settings.json`. Proposta: antes de copiar qualquer JSON de configuração, remover sempre os campos `env`, `headers` e qualquer chave cujo nome contenha `key`, `token`, `secret` ou `password`; a regex vira segunda camada; `.credentials.json` e equivalentes nunca são lidos pelo script. **Aceita pelo usuário em 2026-10-19.**

**P18. Estado de branch fora do índice.** A tabela de branches do `docs/superpowers/INDEX.md` ficou desatualizada no mesmo dia (dizia "sem backup" para branches que já estavam no backup). Proposta: o índice guarda só specs e planos; o estado das branches fica no relatório do agente de versão, gerado na hora. **Aceita pelo usuário em 2026-10-19.**

**Camada do agente de versão** (decisão do usuário): RÁPIDO para rotina (push, abrir PR, acompanhar CI, backup), MÉDIO quando há conflito, triagem de falha de CI ou relatório de saúde. FORTE só para recuperação de histórico (reflog, restauração de branch perdida): aceito pelo usuário em 2026-10-19.

**Processo, registrado para não repetir:** as tarefas T1 a T6 desta spec foram escritas pelo próprio orquestrador e um revisor avaliou as seis juntas, o que contraria o ciclo "uma tarefa por vez, executor separado" que a própria spec reforça. A parte 2 (T8 a T14) segue o ciclo completo.
