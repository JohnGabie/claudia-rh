# Regras de agentes, ciclo de vida das specs e controle de versão — spec

Data: 2026-10-19. Branch: `feat/regras-agentes` (a partir de `dev`). Status: **aguardando aprovação do usuário**.

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
- **Scripts**: PowerShell para o diário, para o relatório de saúde de branches e para o backup. O script de backup só roda por pedido do usuário na primeira vez.
- **Dados/segurança**: nenhum código que apague arquivo. A remoção de branch é `git branch -d` com tag de arquivo antes; nenhum script apaga pasta. Segue o `CLAUDE.md` global de deleção.
- **Fora do repo**: o repo `claudia-rh-backup` recebe commits assim que o usuário autorizar o primeiro push.

## 6. Fora do escopo

- Migrar as 18 specs e 15 planos existentes para specs vivas (ciclo próprio, por área, com aprovação do usuário a cada área).
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

- Se o hook `PreCompact` do Claude Code no Windows realmente bloqueia `/compact` e recebe `compaction_type` no mesmo formato do smart-pets: **não verificado**.
- Se o template de PR atual tem "NÃO verificado": **não verificado**.
- Se `gh pr merge` por terminal basta para o fluxo do usuário sem abrir o GitHub: **não verificado**.
- Nome final das áreas (`browser`, `sandbox`, `documentos`, `perfil`, `testes`, `processo`...): proposta, o usuário ajusta ao migrar.
- Se o `CLAUDE.md` da pasta workspace (`curriculum-apply/`, fora do git) também será ajustado: **não verificado**; fora do repo.
