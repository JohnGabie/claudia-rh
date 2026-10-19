# Regras de agentes, ciclo de vida das specs e controle de versão — spec

Data: 2026-10-19. Branch: `feat/regras-agentes` (a partir de `dev`). Status: **aprovada pelo usuário em 2026-10-19** (mensagem "blz bora", depois de a spec estar escrita e de eu perguntar se aprovava; interpretei como aprovação, e o usuário pode corrigir). Revisão 2026-10-19, confirmada pelo usuário na conversa (após a primeira revisão independente): ajustes de camadas, push e hooks, ver o fim desta seção de histórico em §9. Ampliação §10 (revisão 3): rascunho, aguardando aprovação.

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
- **Origin**: só `dev`, `main`, tags de release e branches que viram PR.
- **Aprovação e merge pelo terminal**: o usuário responde na conversa (pergunta com opções); o agente registra a decisão no corpo do commit de merge ou no PR e executa o merge local ou `gh pr merge` quando o usuário mandar. O PR no `origin` é registro, não bloqueia o fluxo.
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
| 2026-10-19 | §10 | ampliada (rascunho) | P11 a P14: guardião de estabilidade, espelho do público no backup, PR e CI, pacote de recuperação. | aguardando o usuário |

## 10. Ampliação (revisão 3, 2026-10-19): estabilidade, backup, PR e CI, pacote de recuperação — **rascunho, aguardando aprovação**

Marcador: `ampliada 2026-10-19` (rascunho). Amplia a P7; não contradiz o que está aprovado. Só este trecho pede nova aprovação.

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
- Os "commits constantes" só existem em **branches `feat/*` e `fix/*`**, porque commit direto em `dev` continua proibido. Logo o `dev` do backup é o `origin/dev` mais os merges ainda não publicados, e **sempre contém o `origin/dev`** (invariante verificável). Os commits constantes aparecem nas branches de feature no backup.
- "Estável" no `dev` público é por **feature completa**, não por commit: a feature só entra quando passou pela porta da P11.
- Se o `main` não for ao backup, a release fica restaurável só pelo `origin` e pelas tags. Custa quase nada espelhar o `main` e as tags `v*`; recomendo espelhar (P12).

### 10.4 Propostas do assistente (NÃO DECIDIDAS, cada uma com recomendação)

**P11. Guardião da estabilidade de `dev` e `main` públicos.**
- `main`: só release (`dev` → `main` + tag `vX.Y.Z`) da versão **fechada**, depois do smoke de uso real anexado; o usuário manda.
- `origin/dev`: só feature completa, com "Looks good to me" por tarefa, unitários, integração e regressão verdes (`cargo test` só no Sandbox; `tsc`, build e clippy no PC) e smoke da feature quando couber. Feature incompleta fica em branch e vai só ao backup.
- `dev` nunca fica atrás de `main`; sem commit direto; só `merge --no-ff` aprovado pelo usuário.
- `dev` ou `main` quebrou: `git revert` do merge (nunca reset, nunca force push), aviso ao usuário e ao arquiteto, tarefa no Linear.
- O agente **recusa** o merge se faltar o "Looks good to me", a evidência das rodadas ou o CI verde, e diz o que falta.
- Ponto em aberto: `main` e `dev` **não têm proteção de branch no GitHub** (verificado em 2026-10-19: "Branch not protected"). Ativar proteção (PR obrigatório, CI obrigatório) é decisão do usuário; sem ela a estabilidade depende só do agente e das regras locais.
- **Recomendo.**

**P12. Backup como espelho do público mais o que ainda não foi publicado.**
- Invariantes, verificadas por um relatório só de leitura que compara `git ls-remote origin` com `git ls-remote backup`: toda ponta de branch e toda tag do `origin` é igual ou **ancestral** da mesma ref no `backup`; nenhuma ref do `backup` apaga ou reescreve história do `origin`.
- Espelhar também `main` e as tags `v*` (a restauração da release fica completa). *Alternativa: dispensar o `main`, como o usuário sugeriu; perde a restauração direta da release.*
- Push só pelo agente de versão, nos momentos descritos em `agents/versao.md` (fim de tarefa aprovada, depois de merge, antes de operação arriscada, ao encerrar a sessão). `--all` e `--tags`, nunca `--mirror`.
- **Recomendo.**

**P13. PR e CI sob o agente de versão.**
- Quando o usuário manda integrar: o agente abre o PR no `origin` (`gh pr create`, com o modelo de `.github/PULL_REQUEST_TEMPLATE.md`), acompanha o CI (`gh pr checks`; o `ci.yml` roda em push para `main` e `dev` e em PR) e só faz o merge com CI verde e a aprovação do usuário na conversa.
- **CI falhou:** o agente **não corrige**. Lê o log (`gh run view --log-failed`, só leitura), classifica e repassa no formato de brief, com o trecho do log, **pedindo sugestão** e não só correção: falha de código ou teste vai para o executor da tarefa (o revisor confere); contradição com a spec vai para o arquiteto; falha de ambiente, instável ou de infraestrutura vira tarefa no Linear e pergunta ao usuário; qualquer coisa de segurança vai para o revisor em FORTE. Conta as rodadas; na 5ª, para e pergunta ao usuário.
- O `release.yml` roda em tag e **publica** (release e npm). O agente nunca cria tag `v*` sem o usuário mandar.
- **Recomendo.**

**P14. Pacote de recuperação na branch órfã `recovery`, só no backup.**
- Conteúdo (escolhido pelo usuário): (a) config do projeto: `.claude/` (skills, comandos, `settings.json`, `settings.local.json` se a varredura passar) e `.mcp.json` sem segredos; (b) config global e memória: `CLAUDE.md` global, lista de plugins instalados, a pasta de memória deste projeto, `settings.json` global só se a varredura passar; (c) estado de trabalho: `STATE.md` (branches, worktrees, o que está parado, próximo passo, ligação com o Linear), patches do trabalho não commitado e o último `.journal/`.
- **Fora, sempre:** `.env*`, credenciais, tokens, `sandbox-lab/shared/secrets/`, dados pessoais do perfil (`candidate_base.yaml`, `search_variants.yaml`, `strategy.md`, `disparo.json`, `notif.json`, `modo_autonomo.json`) e o banco do usuário.
- **Limpeza antes de gravar:** lista de caminhos permitidos (allowlist: nada entra por padrão) mais varredura de segredos, com os mesmos padrões usados em 2026-10-19, no que vai ser commitado; achou algo, não grava e avisa. O script só cria e sobrescreve arquivos **dentro da sua própria worktree temporária**; não apaga nada.
- **Isolamento para não contaminar o público:** a branch `recovery` é órfã (sem ancestral comum com `dev`), então não há merge acidental; o agente só empurra ao `origin` o que está na lista permitida (`main`, `dev`, `feat/*` e `fix/*` com PR, tags `v*`) e nunca `recovery` ou `wip/*`; um hook `pre-push` versionado recusa `recovery*` no `origin`; o conteúdo do pacote nunca vive numa pasta rastreada do repo de trabalho.
- Atualização do pacote: pelo agente de versão, no fim da tarefa, ao mudar a config e antes de operação arriscada. Não é automática.
- **Recomendo.**

### 10.5 Impacto

- Documentos: `agents/versao.md` ampliado (P11 a P14), `CLAUDE.md` (Git, "O humano decide") e o template de PR.
- Scripts novos, só de leitura ou que só criam: relatório de verificação do backup (`scripts/dev/verify-backup.ps1`), geração do pacote de recuperação (`scripts/dev/recovery-pack.ps1`, em worktree temporária) e o hook `pre-push` que bloqueia `recovery*` no `origin`.
- Configuração: nenhuma mudança no GitHub sem o usuário (proteção de branch, secrets).
- Segurança: pacote de recuperação com varredura obrigatória; nenhuma deleção em lote.

### 10.6 Fora do escopo

Ativar proteção de branch no GitHub; criar a release 0.3; backup do disco fora do git; qualquer mudança em `ci.yml` ou `release.yml`.

### 10.7 Não verificado

- O conteúdo dos arquivos de config global (`settings.json` com ou sem segredo) e do `settings.local.json`: não lidos.
- Se o hook `pre-push` em PowerShell roda no Git for Windows como esperado.
- O que o `ci.yml` cobre exatamente além de build (a leitura foi parcial); os testes Rust do CI não substituem o Sandbox.
- Se `gh pr checks` e `gh run view` bastam para o fluxo do usuário sem abrir o GitHub.
