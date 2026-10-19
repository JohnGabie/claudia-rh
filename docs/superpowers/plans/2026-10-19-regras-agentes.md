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
