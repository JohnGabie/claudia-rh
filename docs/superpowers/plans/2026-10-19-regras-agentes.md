# Plano: regras de agentes, specs vivas, controle de versão e diário de contexto

Spec: `docs/superpowers/specs/2026-10-19-regras-agentes-design.md` (aprovada pelo usuário em 2026-10-19, mensagem "blz bora").
Branch: `feat/regras-agentes`. Só documentos, scripts de apoio e configuração. Sem código de produto.

Fontes lidas antes de escrever cada arquivo: o `CLAUDE.md` deste repo e os arquivos do `smart-pets-mod` (`CLAUDE.md`, `agents/*.md`, `.claude/`). Reler a fonte no momento de escrever, não confiar na memória.

Decisões do usuário já fechadas: spec viva por área; 5 rodadas de recusa; agente de versão completo; diário de contexto entra agora.

## Tarefas (uma por ciclo executor ↔ revisor)

### T1. Estado do projeto fora do `CLAUDE.md`
- Criar `docs/estado-do-projeto.md` com **o texto atual, sem perder nada**, de "Dívida conhecida" e "O que este repo já é".
- Ainda não mexer no `CLAUDE.md`.
- Aceite: cada linha das duas seções existe no arquivo novo (diff de conteúdo).

### T2. Papéis em `agents/`
- Criar `agents/modelos.md` (FORTE = Opus, MÉDIO = Sonnet, RÁPIDO = Haiku; regra de subida após 2 falhas).
- Criar `agents/arquiteto.md`, `executor.md`, `revisor.md`, `qa.md`, `versao.md`, adaptados: skills Superpowers, Linear, Sandbox, `claudia-teste-e2e`, `gh`, worktrees. Sem Minecraft, sem `.sh`.
- Formatos: brief, resposta do executor, veredito (com "Looks good to me"), relatório do QA.
- Arquiteto: ciclo de vida das specs (ampliar, reescrever, nova).
- Versão: remote `backup`, push `--all`, aprovação no terminal, saúde das branches, `archive/*`.
- Aceite: nenhum nome de modelo fora de `modelos.md`; nenhum caminho absoluto de máquina.

### T3. Specs e planos vivos
- Criar `docs/superpowers/INDEX.md` com todas as specs e planos existentes (cada um uma vez): área, status (`viva`, `histórica`, `absorvida por <área>`), branches ativas.
- Criar `docs/superpowers/CONVENCAO.md`: nome estável por área, histórico de revisões, marcador por seção, quando ampliar, reescrever ou criar nova, plano com tarefas `feita`/`obsoleta`/`nova`.
- Ler o cabeçalho de cada spec e plano antes de classificar. Em caso de dúvida, `a confirmar` no índice.
- Aceite: cada spec, plano e evidência existente aparece exatamente uma vez no índice (21 specs, 17 planos, 3 evidências em 2026-10-19).

### T4. `CLAUDE.md` reescrito
- Remover as duas seções movidas na T1 e apontar para `docs/estado-do-projeto.md`.
- Trocar a tabela de modelos por camadas e apontar para `agents/`.
- Limite de recusas continua 5. Acrescentar: revisor lê o diff antes da justificativa; teste do arquiteto não se enfraquece; conversa e memória não são fonte; seção "O humano decide".
- Git: remote `backup`, push `--all`, aprovação no terminal, referência ao agente de versão.
- Specs: apontar para `INDEX.md` e `CONVENCAO.md`; a "Ordem de leitura" item 5 usa o índice.
- Manter intactas as regras duras, a de apagar arquivo, Sandbox, passos de instalação, definição de pronto.
- Aceite: diff mostra só as mudanças previstas; nenhuma regra dura removida.

### T5. Template de PR e `.gitignore`
- `.github/PULL_REQUEST_TEMPLATE.md`: acrescentar "Como foi verificado" e "NÃO verificado", mantendo o que existe.
- `.gitignore`: `.journal/`.

### T6. Diário de contexto e hooks (PowerShell)
- `scripts/dev/journal-skeleton.ps1`, `scripts/dev/pre-compact.ps1`, `scripts/dev/session-start.ps1`, `.claude/settings.json` com `PreCompact` e `SessionStart`.
- Referência: `.claude/hooks/*.sh` do smart-pets (releitura antes de escrever).
- Nenhum script apaga arquivo.
- Aceite: teste com entrada simulada e pasta temporária criada pelo próprio teste: diário incompleto => saída 2; completo => saída 0. A compactação real continua **não verificada**.

### T7. Verificação final
- Busca por caminho absoluto de máquina e por nome de modelo fora de `modelos.md`.
- Conferir links e caminhos citados nos arquivos novos.
- Revisão independente do conjunto.

## Fora deste plano
Remote `backup` e primeiro push (depois da varredura de segredos e do OK do usuário), rebase das branches paradas, migração das specs antigas para specs vivas.

## Adiado (registrado após a revisão independente)

- Script próprio de relatório de saúde de branches e script de backup: **adiados**. O relatório é descrito em `agents/versao.md` e feito com comandos git; o backup só roda por pedido do usuário. Motivo: evitar script que mexe em remote antes da primeira decisão do usuário.

## Extras adotados do smart-pets, fora da spec (o usuário confirmou em 2026-10-19: ficam todos)

- Executor líder com no máximo 2 níveis (`agents/executor.md`, `agents/modelos.md`).
- Alvo de ≲ 400 linhas por PR e aviso para binário > 5 MB (`agents/versao.md`).
- Seções "Risks" e "Spec and review" no template de PR.
- Executor em camada FORTE quando a tarefa toca segurança, dados ou ferramenta do agente (`CLAUDE.md`).

## Estado das tarefas

| Tarefa | Estado | Revisor | Observação |
|---|---|---|---|
| T1 a T6 | feitas | Rodada 1: REJEITADO-CORRIGIR (13 achados). Rodada 2: REJEITADO-CORRIGIR (2 regressões). Rodada 3 (2026-10-19): APROVADO, "Looks good to me" | Commit revisado: `d1756e5` |
| T7 | feita | coberta pelas rodadas de revisão acima | Usuário confirmou em 2026-10-19: spec aprovada e extras mantidos |

## Prova de mutação do `scripts/dev/test-diario.ps1` (2026-10-19)

Cópia dos scripts em pasta temporária, repositório intocado. Linhas coladas da execução:

- Mutação "ignorar `trigger` e assumir manual": `FAILED  trigger auto is never blocked (expected 0, got 2)`, `FAILED  compaction_type auto (fallback field) is never blocked (expected 0, got 2)`, `FAILED  unknown type (no field) never blocks (expected 0, got 2)`, `FAILED  unreadable stdin never blocks (expected 0, got 2)`.
- Mutação `exit 2` trocado por `exit 0` (prova independente do revisor): `FAILED  trigger manual with an unfilled diary is blocked (expected 2, got 0)` e `FAILED  compaction_type manual (fallback field) is blocked (expected 2, got 0)`.
- A mutação "ignorar `trigger` e assumir manual" que eu rodei removeu a leitura de `trigger` e trocou o padrão `unknown` por `manual`; por isso o caso `compaction_type auto` também falhou. A versão do revisor desligou só o `trigger`.
- Sem mutação: 9 casos `PASS`, código de saída 0.


---

# Parte 2: estabilidade, backup espelho, PR e CI, pacote de recuperação, proteção de branch

Cumpre a spec `2026-10-19-regras-agentes-design.md`, revisões 3 e 4 (seção 10 e P7 reescrita, aprovadas pelo usuário em 2026-10-19). Checks obrigatórios: os 5 jobs do `ci.yml`. O merge do PR é sempre com o "pode fechar" do usuário. Branch: `feat/regras-agentes`. Uma tarefa por ciclo executor ↔ revisor; recusa volta ao executor; cinco recusas seguidas, para e pergunta.

Regras de deleção do `CLAUDE.md` global valem para todo script desta parte: nenhum apaga, move ou sobrescreve fora de pasta temporária criada pelo próprio script ou teste; nenhum teste usa caminho real; testes rodam só com pastas temporárias e repositórios temporários criados por eles.

| Tarefa | Estado | Revisor | Observação |
|---|---|---|---|
| T8 | feita | Rodada 1: REJEITADO-CORRIGIR (13 achados). Rodada 2: APROVADO, "Looks good to me" (commit `b0b7a18`) | `agents/versao.md` ampliado (P7 reescrita, P11 a P15, camada RÁPIDO/MÉDIO e FORTE só na recuperação) |
| T9 | feita | Rodada 1: REJEITADO-CORRIGIR (recusa do merge sumiu; CONVENCAO.md L79). Rodada 2: APROVADO, "Looks good to me" | `CLAUDE.md` (Git, PR e CI, "O humano decide") e `agents/revisor.md` (chamar o versionador, conferir o relatório e dispensá-lo; push de branch e PR autorizados pelo "Looks good to me" da última tarefa) |
| T10 | feita | Rodada 1: REJEITADO-CORRIGIR (prompt de credencial, origin inexistente sem teste; tags iguais por decisão do usuário). Rodada 2: REJEITADO-CORRIGIR (teste 12 aceitava linhas comentadas). Rodada 3: APROVADO, "Looks good to me" | `scripts/dev/verify-backup.ps1` (só leitura) e teste |
| T11 | feita | Rodada 1: REJEITADO-CORRIGIR (quatro regras de segurança sem prova, pasta proibida, junction na raiz, nome não varrido, regex com lacunas, aviso da cópia temporária). Rodada 2: APROVADO, "Looks good to me" (commit `0e9b79c`) | pacote de recuperação: `recovery-pack.ps1`, limpeza por campo (P17), varredura e teste |
| T12 | feita | Rodada 1: APROVADO, "Looks good to me" (commits `5da4364` e `77fe793`, com `.gitattributes` para o lançador sh) | hook `pre-push` (lista permitida com `sync/*`; bloqueia `recovery*`, `wip/*`, tags internas e `--all`/`--tags` acidentais no `origin`) e teste |
| T15 | feita | Rodadas 1, 2 e 3: REJEITADO-CORRIGIR (isolamento por branch, heurística dependente da árvore, HEAD destacado, repo sem commit, regra sem prova). Executor relançado em FORTE após 3 recusas. Rodada 4: APROVADO, "Looks good to me" (commit `c8ad62a`) | P16: diário no checkout principal (`git rev-parse --git-common-dir`), teste e conferência do `.journal/` antes de remover worktree |
| T16 | feita | Rodada 1: APROVADO, "Looks good to me" (commit `13be975`; relato do executor fora do formato, registrado) | P18: tirar a tabela de branches do `INDEX.md`; o estado vem do relatório do versionador |
| T17 | feita | Rodadas 1 e 2: REJEITADO-CORRIGIR (volta do `.journal` da worktree só no caso de falta do principal). Executor relançado em MÉDIO. Rodada 3: APROVADO, "Looks good to me" (commit `e398880`) | Sobras da T15: `agents/versao.md` (antes de remover worktree, conferir `<principal>.journal` filtrando pela branch ou pela worktree, e o `.journal/` da worktree só nos casos de volta) e a menção ao nome do diário na spec (`<stamp>-<branch>-<tag>-<motivo>.md`) |
| T18 | feita | Rodada 1: APROVADO, "Looks good to me" (commits `92c30d5` e `c7c29f2`) | Sobras da T11: (1) `agents/versao.md` e spec P14 e §10.5 dizem "worktree temporária", mas o pacote usa índice temporário e plumbing; `versao.md` diz "último .journal", o script leva todos os diários. (2) Opcional, decidir: checar junction nos níveis entre `-GlobalClaudeDir` e a memória; camelCase em texto (`accessToken: ...`) não é pego pela regex; stderr do git sem máscara. Sobras da T12: (3) URL do público em formas estranhas (`/.`, `x/..`, `%2D`) passa como não pública: documentar no cabeçalho ou normalizar a URL; (4) force push em `feat/*` passa (a P7 diz "sem force push"); (5) o padrão de tag aceita `v1`, `v1.2`, `v1.2.3.4` (mais largo que SemVer; o `release.yml` dispara em `v*`); (6) ativar o hook (`git config core.hooksPath scripts/dev/git-hooks`) é decisão do usuário e entra no runbook da T13. |
| T13 | feita | Rodada 1: REJEITADO-CORRIGIR (`-f` manda texto). Rodada 2: REJEITADO-CORRIGIR (filtro quebra no PowerShell 5.1; checagem de id só em comentário). Rodada 3: APROVADO, "Looks good to me" (commit `20bc1bd`) | comandos de proteção (`gh api`) escritos com a reversão; **não executados** |
| T14 | nova | pendente | PR de sincronização do `dev` local e ativação, **só com o "pode aplicar"** |

### T8. `agents/versao.md` ampliado
- Incluir: guardião de estabilidade de `dev` e `main` (P11), espelho no backup e invariantes (P12), PR e CI com repasse de falha a executor, arquiteto, revisor ou usuário (P13), pacote de recuperação (P14), proteção de branch (P15) e o fluxo "abre o PR, mostra o resumo, só com a resposta afirmativa do usuário faz o merge".
- Aceite: nenhum nome de modelo; nenhuma contradição com `CLAUDE.md` (revisor confere por grep); o texto de repasse de falha usa o formato de brief do arquiteto.

### T9. `CLAUDE.md`
- Seção Git: `main` = versão fechada, `dev` = features estáveis rumo à próxima versão; `origin` sempre por PR depois da proteção; backup como espelho do `origin` mais o que ainda não foi publicado; `recovery` só no backup.
- "O humano decide": ativar ou desligar proteção de branch, criar tag `v*`, merge no `origin`.
- Aceite: nenhuma regra dura removida (diff); merge, release e tag `v*` continuam do usuário; empurrar a branch ao `origin` e abrir o PR são do agente de versão, depois do "Looks good to me" da feature completa (revisão 4).

### T10. `scripts/dev/verify-backup.ps1` (só leitura)
- Compara `git ls-remote origin` com `git ls-remote backup`: cada ponta de branch e cada tag do `origin` precisa ser igual ou ancestral da ref no `backup`; lista o que está só no backup e o que está atrasado.
- Teste em repositórios temporários criados pelo teste (dois remotes locais `file://`), com prova de mutação.
- Aceite: código de saída 0 se a invariante vale, 1 se não; nenhuma escrita.

### T11. Pacote de recuperação
- `scripts/dev/recovery-pack.ps1`: monta, numa worktree temporária da branch órfã `recovery` (mecanismo real: índice temporário, sem worktree; ver T18), os arquivos da **lista permitida** (config do projeto, config global e memória, `STATE.md`, patches do trabalho não commitado, último `.journal/`); roda a varredura de segredos antes de gravar; recusa gravar se achar algo; não apaga nada.
- Lista de exclusão sempre ativa: `.env*`, credenciais, tokens, `sandbox-lab/shared/secrets/`, dados pessoais do perfil, banco do usuário.
- Teste só com pasta de origem falsa criada pelo teste, com segredo de mentira (padrão `sk-ant-` sintético) para provar a recusa, e prova de mutação.
- Ler `settings.local.json` e o `settings.json` global **só dentro do script, para varrer**; o conteúdo não é impresso.
- Aceite: com o segredo falso presente, saída diferente de 0 e nada gravado; sem ele, saída 0.

### T12. Hook `pre-push`
- Hook versionado em `scripts/dev/git-hooks/` (ativado por `git config core.hooksPath scripts/dev/git-hooks`, uma vez por clone) que recusa no remote `origin`: refs `recovery*` e `wip/*`, qualquer ref fora de `main`, `dev`, `feat/*`, `fix/*` e tags `v*`, e push direto a `main` ou `dev`.
- Teste com remotes locais temporários, com prova de mutação.
- **Não verificado:** como o Git for Windows executa o hook (sh chamando PowerShell).

### T13. Comandos de proteção (escritos, não executados)
- Arquivo de runbook em `docs/` com os comandos `gh api` para: proteção ou ruleset de `main` e `dev` (PR obrigatório, 0 reviews, checks obrigatórios, `strict`, sem force push, sem apagar, `enforce_admins`), desligar squash e rebase, ruleset das tags `v*`, e a **reversão** de cada um.
- Antes de ativar: conferir os nomes dos jobs como contexto de check (`check-runs` de um commit recente) e a resposta da API para repositório público no plano atual.
- Aceite: revisor lê o runbook; nada foi executado contra o GitHub.

### T14. PR de sincronização e ativação (só com "pode aplicar" do usuário)
- Branch `sync/dev-AAAA-MM-DD` no `dev` local, push ao `origin`, PR para `origin/dev`, CI verde, aprovação do usuário na conversa, merge.
- Depois, ou antes se o usuário preferir, aplicar a T13 e conferir `gh api repos/.../branches/dev/protection`.
- Pedir a leitura dos checks ("1 + linux") antes de aplicar.

## Ordem

T8 e T9 (documentos) → T10 → T11 → T12 → revisão do conjunto → T13 → T14.

### Ajustes da revisão 4 (aprovada em 2026-10-19)

- **T8:** o fluxo "Do revisor ao merge" da P7 (revisor chama o versionador; ele empurra e abre o PR sem OK do usuário; devolve relatório; o revisor dispensa; merge com o "pode fechar"); merge local em `dev`/`main` não existe mais, o `dev` local só avança por `git pull --ff-only`; push ao `origin` só por ref nomeada.
- **T9:** `CLAUDE.md` linhas do fluxo Git e da aprovação pelo terminal alinhadas a isso (hoje ainda dizem merge local e "PR é registro").
- **T11:** limpeza por campo antes da regex (P17).
- **T13:** os 5 jobs como checks obrigatórios.
- **T15 e T16:** novas (P16 e P18).
- Ordem: T8 → T9 → T16 → T15 → T10 → T11 → T12 → revisão do conjunto → T13 → T14.

### Observações do revisor da T8 para a T9 (não bloquearam a T8)

- `agents/versao.md` L36: a frase "hoje falha: o `dev` local está 84 commits à frente" envelhece depois da T14; deixar claro que o push ao backup acontece mesmo quando o `--ff-only` falha.
- `agents/versao.md` L48 x L51: "`main` só recebe release" e o revert por PR contra `main`; escrever o revert de quebra como exceção explícita.
- `agents/versao.md` L34-35 e `agents/revisor.md`: escrever quem chama o versionador para mostrar o resumo e pedir o "pode fechar" depois de o revisor dispensá-lo (o orquestrador).
- `CLAUDE.md` L37, L48-50, L128, L143, L212, L257 e `agents/revisor.md` L54 e o fluxo de chamar e dispensar o versionador.

### Nota de processo (2026-10-19)

Nesta sessão, os passos de versionador depois de cada "Looks good to me" (fast-forward na `feat/regras-agentes`, push ao `backup`, remoção da worktree sem `--force`) foram feitos pelo orquestrador, por serem mecânicos e só locais ou no backup. Nada foi ao `origin`. A partir da T14, que mexe no `origin` e no GitHub, os passos ficam com o agente de versão.

### Decisões do usuário sobre as sobras da T11 e da T12 (2026-10-20)

- URL do público: **normalizar** no hook. Force push ao público: **recusar** no hook. Formato de tag: **manter** (`v` + dígitos e pontos). Três lacunas do pacote de recuperação: **corrigir**.

| Tarefa | Estado | Revisor | Observação |
|---|---|---|---|
| T19 | feita | Rodada 1: REJEITADO-CORRIGIR (tag publicada podia ser movida para frente ou recriada). Rodada 2: APROVADO, "Looks good to me" | Hook pre-push: normalizar a URL antes de reconhecer o público (`%xx`, `.`, `..`, barra final, maiúsculas) e recusar force push ao público (o sha remoto antigo, quando não for zeros, tem de ser ancestral do novo; se o objeto antigo não existir localmente, recusar). Testes e mutação. |
| T20 | feita | Rodada 1: REJEITADO-CORRIGIR (exceção não tratada sem máscara). Rodada 2: REJEITADO-CORRIGIR (aviso da pasta temporária sem teste). Rodada 3: APROVADO, "Looks good to me" | Pacote de recuperação: regex para `accessToken`, `apiKey`, `clientSecret` e parecidos em texto (camelCase); recusar junction nos níveis entre `-GlobalClaudeDir` e a memória; máscara nas mensagens de erro do git. Testes e mutação. |

A T18 (docs) passa a incluir também: o formato de tag decidido ("manter") no `agents/versao.md` e no cabeçalho do hook, e a ativação do hook no runbook da T13.

Ordem daqui em diante: T18 → T19 → T20 → T13 → revisão do conjunto → T14 (só com o "pode aplicar" do usuário).

### Para a revisão do conjunto (observações do revisor da T18)

- A descrição da T11 neste plano ainda diz "numa worktree temporária" e "último `.journal/`": é registro histórico da tarefa; o mecanismo real está em `agents/versao.md` e na spec.
- `agents/versao.md` ~L81-82: "primeira execução real é decisão do usuário" é ambíguo para um agente RÁPIDO; deixar explícito: se `refs/heads/recovery` não existe, perguntar ao usuário antes de rodar sem `-WhatIf`.
- Spec P14: "só cria e sobrescreve arquivos dentro da sua pasta temporária" omite que o script grava objetos no `.git` e a ref `refs/heads/recovery`.

### Revisão do conjunto, rodada 1 (2026-10-20): REJEITADO-CORRIGIR

Bloqueantes: B1 camada do revisor em `CLAUDE.md:158` contradiz "FORTE sempre"; B2 sem varredura de segredos antes do PR de sincronização e sem comando concreto; B3 hook desligado em silêncio com `core.hooksPath` relativo em checkouts sem o hook; B4 primeira execução do pacote de recuperação ambígua. Decisões do usuário: sem limite de tamanho de PR; uma branch de trabalho por push ao público; testes nos dois terminais.

| Tarefa | Estado | Revisor | Observação |
|---|---|---|---|
| T21 | feita | Rodada 1: REJEITADO-CORRIGIR (aviso de e-mail vazava parte do valor; nome da branch no intervalo não varrido; não varrido fechava com "Nada encontrado"). Rodada 2: APROVADO, "Looks good to me" | Varredura de segredos: `scripts/dev/lib/secret-patterns.ps1` (lista única de padrões, extraída do pacote), `scripts/dev/scan-secrets.ps1` (só leitura; varre o diff de um intervalo de commits, ex.: `origin/dev..<branch>`, e nomes de arquivos; exit 0/1/2; nunca mostra o valor) e teste; `recovery-pack.ps1` passa a usar a lista única; `test-recovery-pack.ps1` roda também pelo Git Bash. |
| T22 | feita | Rodada 1: REJEITADO-CORRIGIR (instalador dizia OK com `core.hooksPath` desviando o hook). Rodada 2: APROVADO, "Looks good to me" (commits `9a94989` e `51835bf`) | Hook: no máximo uma branch de trabalho por push ao público (tags `v*` à parte); lançador com bit de execução; `scripts/dev/install-hooks.ps1` que copia o hook para a pasta de hooks comum (`git rev-parse --git-common-dir`/hooks), não sobrescreve hook diferente existente, confere a instalação com um push simulado; testes, inclusive pelo Git Bash. |
| T23 | feita | Revisado dentro da revisão do conjunto: rodada 2 REJEITADO-CORRIGIR (opções do instalador descritas errado); rodada 3 APROVADO (commit `9aa50d3`) | Docs: B1, B2 (passo de varredura no runbook §9 e em `versao.md`, com o comando), B3 (instalação por `install-hooks.ps1` no runbook §7 e conferência real), B4 (primeira execução do pacote: só `-WhatIf` e perguntar), sem limite de PR, quem chama o versionador (o revisor pede ao orquestrador), `git fetch origin dev:dev` no lugar de `switch` + `pull`, integração da tarefa na branch da feature, `--tags` no backup do runbook, `INDEX.md` com o estado atual da spec, spec P13/P15 com "feature completa", nota de mecanismo real na descrição da T11, `CLAUDE.md` "Remover worktree" com os diários, `versao.md` frase de SemVer alinhada. Depende de T21 e T22. |
| T24 | feita | Rodada 1: APROVADO, "Looks good to me" (commit `f8e7480`) | Instalador do hook (observações do revisor da T22): conferência com `--no-follow-tags --recurse-submodules=no` nos dois pushes (nunca publicar fora do temporário); tirar ou corrigir o conselho "copy the two files into the folder it names" (leva a exit 1 eterno); checar commit em HEAD antes de gravar qualquer arquivo. |

Ordem: T21 e T22 em paralelo → T23 → revisão do conjunto, rodada 2 → T14 (só com o "pode aplicar" do usuário).

### Observações do revisor da T21 (não bloquearam)

- Nomes de refs que não aparecem no `-Range` não são varridos; o push ao `backup` usa `--all`/`--tags` e leva todos os nomes ao repositório privado. Se quiser cobrir: varrer `git for-each-ref --format=%(refname)` antes do push ao backup.
- Senha curta com `@` que não é reconhecida como segredo pode aparecer parcialmente no aviso de e-mail (o valor já não era acusado; não piora).

### Revisão do conjunto: APROVADO na rodada 3 (2026-10-20), "Looks good to me"

Rodada 1: REJEITADO-CORRIGIR (B1 a B4). Rodada 2: REJEITADO-CORRIGIR (opções do instalador no runbook). Rodada 3: APROVADO. Os 6 testes dos scripts passam pelo PowerShell e pelo Git Bash (61, 42, 209, 83, 25 e 90 PASS).

Observações finais que não bloquearam:
- Runbook §9 passo 1: `git switch -c sync/dev-AAAA-MM-DD dev` troca a branch do checkout e leva trabalho não commitado; `git branch sync/dev-AAAA-MM-DD dev` faz o mesmo sem trocar.
- Runbook §9 passo 1.1: repetir a conferência da linha "(N commit(s))" da varredura.
- Riscos para o Linear: a válvula `CLAUDIA_ALLOW_PUBLIC_PUSH` pode ser definida por qualquer agente (a defesa é só a regra escrita); os hooks de `.claude/settings.json` rodam com `-ExecutionPolicy Bypass`; nomes de refs fora do `-Range` não são varridos antes do push ao backup.

Testes humanos pendentes: primeira instalação real do hook; push real ao público com o hook ativo; compactação real com o PreCompact; primeira execução real do pacote de recuperação; fluxo `gh pr merge` (T14).

### T14, parte 1: proteção aplicada no GitHub (2026-10-20, "pode aplicar" do usuário)

Decisões do usuário: um PR só (a feature leva junto os commits do `dev` local que ainda não estão no `origin`); aplicar a proteção agora; não registrar no Linear.

Aplicado pelo runbook, seções 1 a 5, com conferência só leitura depois:
- Repositório: só merge commit (`allow_squash_merge=false`, `allow_rebase_merge=false`).
- `dev` e `main`: PR obrigatório, 0 aprovações, os 5 checks do app 15368, `strict`, `enforce_admins`, sem force push, sem apagar.
- Ruleset `protect-release-tags` (id 25508255): `refs/tags/v*`, regras `update` e `deletion`, sem bypass.
- O plano gratuito com repositório público aceitou tudo (a dúvida "não verificado" do runbook ficou respondida).

Ainda não feito: hook local (o usuário não marcou), primeiras execuções de `verify-backup` e do pacote de recuperação (não marcadas), PR único da feature.
