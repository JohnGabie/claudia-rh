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
