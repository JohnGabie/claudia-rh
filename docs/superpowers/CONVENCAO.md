# Convenção de specs e planos vivos

Criada em 2026-10-19 (spec `2026-10-19-regras-agentes-design.md`, P6). Quem aplica: o Arquiteto (`agents/arquiteto.md`). O índice está em `INDEX.md`.

## Problema que isto resolve

Cada ajuste virava spec nova, que virava plano novo, que virava branch nova. A mesma frente ficou espalhada em vários arquivos datados, sem dizer qual vale hoje. A regra agora é: **uma spec viva por área; o resto é histórico.**

## Regras

1. **Uma spec viva por área**, em `docs/superpowers/specs/<area>.md`, sem data no nome. Exemplos: `browser.md`, `sandbox.md`, `sites.md`. O plano correspondente fica em `docs/superpowers/plans/<area>.md`.
2. **Toda spec e todo plano entram no `INDEX.md`** no mesmo commit em que nascem ou mudam de estado.
3. **Antes de criar um arquivo, procurar no índice.** Se a área existe, edita-se a spec dela.
4. **Spec nova só para área nova**: a mudança não cabe em nenhuma seção de spec existente.
5. **Spec datada existente não se apaga nem se renomeia.** Quando a spec viva da área é criada, as datadas ficam no índice como `absorvida por <área>`, com a nota do que a viva herdou.
6. **Migração por área**, uma de cada vez, com aprovação do usuário a cada área. Não há migração em massa.

## Cabeçalho de toda spec viva

```
# <Área>: spec viva
Estado: viva | rascunho
Última revisão aprovada: AAAA-MM-DD (mensagem do usuário)
Plano: plans/<area>.md
Absorve: <specs datadas, se houver>
Issues no Linear: CLA-NN, ...
```

## Seções com marcador

Cada seção `##` termina o título com um marcador:

| Marcador | Significado |
|---|---|
| `estável` | Aprovada e sem mudança desde a criação. |
| `ampliada AAAA-MM-DD` | Recebeu acréscimo sem contradizer o que estava aprovado. |
| `reescrita AAAA-MM-DD` | O texto anterior foi substituído (parcial ou total). |
| `rascunho` | Escrita, ainda não aprovada. Não vale como pedido do usuário. |

## Histórico de revisões

No fim da spec viva, uma tabela, uma linha por revisão:

| Data | Seções | Tipo | O que mudou e por quê | Aprovação |
|---|---|---|---|---|
| AAAA-MM-DD | §3, §5 | ampliada | uma linha | mensagem do usuário em AAAA-MM-DD |

O que foi **reescrito** guarda o texto antigo no histórico do git; a linha da tabela diz em qual commit.

## Como mudar uma spec ou um plano no meio do caminho

1. O arquiteto escolhe **ampliar**, **reescrever (parte ou tudo)** ou **spec nova** e escreve a justificativa em uma linha (ver a tabela em `agents/arquiteto.md`).
2. Edita o trecho, marca a seção e acrescenta a linha no histórico. O trecho fica `rascunho` até a aprovação.
3. **Só o trecho alterado** pede nova aprovação, e a aprovação é uma mensagem do usuário **depois** de o trecho estar escrito.
4. Atualiza o plano no mesmo ramo: cada tarefa tem estado `feita`, `obsoleta` ou `nova`. Tarefa feita não se apaga; tarefa obsoleta diz por quê.
5. O plano cita a **revisão da spec** que cumpre (a data da última revisão aprovada).
6. Atualiza o `INDEX.md`.
7. O código só muda depois da aprovação do trecho. Se a implementação descobrir que a spec está errada, para e volta ao passo 1.

## Plano vivo

```
# Plano <área>
Cumpre a spec: <área>.md, revisão AAAA-MM-DD

| Tarefa | Estado | Revisor (veredito) | Observação |
|---|---|---|---|
| T1 ... | feita | Looks good to me (AAAA-MM-DD) | |
| T2 ... | obsoleta | n/a | substituída pela T5: motivo |
| T5 ... | nova | pendente | |
```

Uma tarefa por ciclo executor ↔ revisor continua valendo.

## O que não muda

- Spec primeiro: sem spec aprovada, não se escreve código.
- Estrutura da spec: pedido do usuário nas palavras dele, leitura, propostas **não decididas**, impacto, fora do escopo, pontos em aberto (`CLAUDE.md` seção 1).
- Merge, release e tag `v*` são decisão do usuário; empurrar a branch ao `origin` e abrir o PR são do agente de versão, depois do "Looks good to me" da feature completa (ver `agents/versao.md`).
