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
| T8 | nova | pendente | `agents/versao.md` ampliado (P7 reescrita, P11 a P15, camada RÁPIDO/MÉDIO e FORTE só na recuperação) |
| T9 | nova | pendente | `CLAUDE.md` (Git, PR e CI, "O humano decide") e `agents/revisor.md` (chamar o versionador, conferir o relatório e dispensá-lo; push de branch e PR autorizados pelo "Looks good to me" da última tarefa) |
| T10 | nova | pendente | `scripts/dev/verify-backup.ps1` (só leitura) e teste |
| T11 | nova | pendente | pacote de recuperação: `recovery-pack.ps1`, limpeza por campo (P17), varredura e teste |
| T12 | nova | pendente | hook `pre-push` (lista permitida com `sync/*`; bloqueia `recovery*`, `wip/*`, tags internas e `--all`/`--tags` acidentais no `origin`) e teste |
| T15 | nova | pendente | P16: diário no checkout principal (`git rev-parse --git-common-dir`), teste e conferência do `.journal/` antes de remover worktree |
| T16 | nova | pendente | P18: tirar a tabela de branches do `INDEX.md`; o estado vem do relatório do versionador |
| T13 | nova | pendente | comandos de proteção (`gh api`) escritos com a reversão; **não executados** |
| T14 | nova | pendente | PR de sincronização do `dev` local e ativação, **só com o "pode aplicar"** |

### T8. `agents/versao.md` ampliado
- Incluir: guardião de estabilidade de `dev` e `main` (P11), espelho no backup e invariantes (P12), PR e CI com repasse de falha a executor, arquiteto, revisor ou usuário (P13), pacote de recuperação (P14), proteção de branch (P15) e o fluxo "abre o PR, mostra o resumo, só com a resposta afirmativa do usuário faz o merge".
- Aceite: nenhum nome de modelo; nenhuma contradição com `CLAUDE.md` (revisor confere por grep); o texto de repasse de falha usa o formato de brief do arquiteto.

### T9. `CLAUDE.md`
- Seção Git: `main` = versão fechada, `dev` = features estáveis rumo à próxima versão; `origin` sempre por PR depois da proteção; backup como espelho do `origin` mais o que ainda não foi publicado; `recovery` só no backup.
- "O humano decide": ativar ou desligar proteção de branch, criar tag `v*`, merge no `origin`.
- Aceite: nenhuma regra dura removida (diff); "push ao `origin`" continua só do usuário.

### T10. `scripts/dev/verify-backup.ps1` (só leitura)
- Compara `git ls-remote origin` com `git ls-remote backup`: cada ponta de branch e cada tag do `origin` precisa ser igual ou ancestral da ref no `backup`; lista o que está só no backup e o que está atrasado.
- Teste em repositórios temporários criados pelo teste (dois remotes locais `file://`), com prova de mutação.
- Aceite: código de saída 0 se a invariante vale, 1 se não; nenhuma escrita.

### T11. Pacote de recuperação
- `scripts/dev/recovery-pack.ps1`: monta, numa worktree temporária da branch órfã `recovery`, os arquivos da **lista permitida** (config do projeto, config global e memória, `STATE.md`, patches do trabalho não commitado, último `.journal/`); roda a varredura de segredos antes de gravar; recusa gravar se achar algo; não apaga nada.
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
