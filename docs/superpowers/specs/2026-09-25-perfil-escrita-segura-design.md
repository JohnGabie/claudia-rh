# Spec: escrita segura do perfil e o canal que faltava

> Spec de design. Não é o plano de implementação.

**Data:** 2026-09-25
**Branch:** `feat/perfil-escrita-segura` (a partir de `dev`)
**Issues:** CLA-11 (urgente), CLA-13 (parcial)

## 1. Problema

Em 25/09/2026, durante uma sessão de Perfil, o agente sondou o schema de `gaps_conhecidos` enviando um YAML mínimo. A ferramenta aceitou, respondeu *"Perfil atualizado com sucesso: 0 experiência(s), 0 projeto(s), 0 competência(s)"* e zerou o `candidate_base.yaml`. O ficheiro foi restaurado por sorte — havia cópia no contexto da conversa.

### Causa raiz

`CandidatoBase` tem `#[serde(default)]` em todos os campos (`commands/perfil.rs:147-159`). Um YAML de uma linha desserializa como um perfil completo e vazio. `update_profile` valida a sintaxe (`mcp/tools/profile.rs:38`) e nunca a plausibilidade: sintaxe válida, escrita feita.

Os defaults não são descuido. Existem porque YAMLs legados gerados pelo Claude vinham incompletos. Removê-los parte o legado; a proteção tem de estar noutro sítio.

### O que já existia e não segurou

`commands/perfil.rs:676` e `commands/prompts.rs:172` mandam o modelo confirmar com o usuário antes de gravar. É instrução de prompt, não garantia de código. No incidente o modelo nem estava num fluxo de gravação — estava a sondar — e a tool aceitou na mesma.

**Lição que orienta este desenho: o que protege o perfil tem de viver no Rust.**

## 2. O segundo problema, encontrado ao desenhar isto

A CLA-13 pede criar uma entidade para perguntas globais, com `vaga_id` nullable, porque `pendencias.vaga_id` é `NOT NULL` e prende tudo a uma vaga.

Essa entidade já existe: `propostas_perfil` (`db/schema.sql:36-44`), com `vaga_id INTEGER REFERENCES vagas(id)` — nullable. E à volta dela está tudo construído:

| Peça | Onde |
|---|---|
| Contagem e listagem | `db/queries.rs:216`, `:236` |
| Promoção para o perfil | `commands/estado.rs:94` |
| Notificação | `notificacoes.rs:49` |
| Badge na Sidebar | `src/App.tsx:143` |
| Contador no Dashboard | `src/components/dashboard/index.tsx:109` |

Os únicos `INSERT INTO propostas_perfil` do repositório estão em testes (`db/queries.rs:460`, `:465`). **Em produção nada escreve ali, porque não há tool MCP que o faça.**

É por isso que o badge está sempre a zero, e é por isso que as perguntas globais foram parar a `search_variants.yaml → perguntas_pendentes` — o canal paralelo que a CLA-13 descreve como sintoma. A tabela não estava em falta; estava órfã.

## 3. Objetivo

1. Nenhuma escrita no perfil destrói o estado anterior sem deixar cópia.
2. Uma escrita que remove conteúdo diz-lo no terminal, onde o usuário já está.
3. Só uma sessão com o usuário presente pode gravar no perfil.
4. A sessão autónoma tem um canal real para propor mudanças de perfil, e esse canal é o que a UI já mostra.

## 4. Decisões tomadas com o usuário

Registadas porque restringem o que a implementação pode fazer.

**Sem etapa de aprovação.** Foi considerado e rejeitado um fluxo de staging (`candidate_base.yaml.pending` + aprovar/rejeitar na aba Perfil). Palavras do usuário: *"eu estou pedindo para ele editar alguma coisa no meu perfil, e aí tenho que aprovar lá na interface, sendo que eu já estou no terminal. Acho muita etapa, um overengineering desnecessário."*

A consequência de desenho: quando o usuário está presente, a proteção é **visibilidade e reversibilidade**, não bloqueio. O diff aparece onde ele já está a olhar; o backup garante que se desfaz.

**O eixo não é destrutivo vs. aditivo — é pedido vs. não pedido.** O medo do usuário é o agente editar sozinho, sem ninguém consciente da sessão. Daí a regra 3: a proibição incide sobre a sessão autónoma, não sobre o tipo de mudança.

## 5. Desenho

### 5.1 Backup rotativo antes de cada escrita

Antes de gravar, o `candidate_base.yaml` atual passa a `candidate_base.yaml.bak-1`; os anteriores deslizam até `.bak-5`, e o sexto cai.

`.bak-1` é sempre o estado imediatamente anterior à última escrita.

A gravação mantém-se atómica: `.tmp` + `rename`, como em `mcp/tools/profile.rs:41-43`. A rotação acontece antes, e uma falha a rotacionar aborta a escrita — sem backup, não se escreve.

Se o `candidate_base.yaml` não existir (primeira gravação), não há o que rotacionar e a escrita segue.

### 5.2 A resposta passa a ser um diff

Hoje a resposta conta o que ficou. Passa a dizer o que mudou, comparando o perfil anterior com o novo.

Sem perdas:

```
Perfil gravado. +1 experiência, +3 competências.
```

Com perdas:

```
Perfil gravado.  −4 experiências, −10 projetos, −22 competências.
Anterior em candidate_base.yaml.bak-1.
```

A linha do backup só aparece quando algo foi removido — é aí que serve. Quando não há mudança nenhuma em nenhum bloco, diz-se isso explicitamente em vez de fingir trabalho.

O diff compara contagens por bloco (`experiencia`, `projetos`, `formacao`, `competencias`, `idiomas`, `gaps_conhecidos`) e assinala campos de `dados_pessoais` que tinham valor e passaram a vazio. Não é um diff textual linha a linha: o que importa é *o que desapareceu*, não como o YAML foi reformatado.

### 5.3 A tool sabe quem a chama

`write_mcp_config` (`commands/perfil.rs:447`) produz hoje a mesma configuração para as quatro origens. Ganha um parâmetro de tipo de sessão, propagado até ao servidor MCP por argumento de linha de comando.

| Call site | Tipo |
|---|---|
| `commands/perfil.rs:506` | interativa |
| `commands/perfil.rs:708` | interativa |
| `commands/linkedin.rs:68` | interativa |
| `commands/sessao.rs:52` | **autónoma** |

O default, na ausência do argumento, é **autónoma** — a opção restritiva. Um call site novo que se esqueça do parâmetro falha fechado, não aberto.

### 5.4 O que cada sessão pode fazer

**Interativa** — `update_profile` grava, com backup e diff.

**Autónoma** — `update_profile` não grava. Devolve ao agente uma mensagem que explica porquê e o que fazer em vez disso, e a chamada não é um erro de validação: é uma recusa com alternativa.

### 5.5 `propose_profile_change`: a tool que faltava

Uma tool MCP nova, disponível nas duas sessões, que insere em `propostas_perfil`:

| Campo | Origem |
|---|---|
| `pergunta` | o que o agente quer perguntar ou propor |
| `contexto` | porque surgiu — a vaga, a resposta a um formulário, o que for |
| `vaga_id` | opcional; `NULL` quando a proposta é global |
| `criada_em` | `datetime('now')` |
| `promovida` | `0` |

Insere e emite `nova-proposta`, o evento que `src/App.tsx:63` já escuta. O badge, a notificação e o contador do Dashboard passam a mexer-se sem uma linha de frontend nova.

### 5.6 Migração de `perguntas_pendentes` — bloqueada por decisão pendente

`search_variants.yaml` tem hoje um bloco `perguntas_pendentes` com 11 itens, alguns abertos desde 21/07 (`commands/perfil.rs:208`, `Vec<PerguntaPendente>`). São exatamente as perguntas globais que a tabela aceitaria.

**O obstáculo:** `src-tauri/src/prompt_sistema_runtime.md:144` instrui o agente a escrever nesse bloco:

> *"proponha uma entrada nova em `perguntas_pendentes` no `search_variants.yaml`"*

Migrar os dados sem mudar essa instrução deixa o prompt a apontar para um canal esvaziado, e o agente volta a enchê-lo na sessão seguinte. Mas `prompt_sistema_runtime.md` é protegido por `CLAUDE.md`: *"Não edite sem pedido explícito."*

As três saídas, por ordem de preferência:

1. **Migrar e editar o prompt** — uma linha, a trocar `perguntas_pendentes` por `propose_profile_change`. Canal único, que é o objetivo. Requer autorização explícita do usuário para o ficheiro protegido.
2. **Não migrar nesta iteração** — `perguntas_pendentes` fica como está, `propostas_perfil` passa a receber só o que vem da tool nova. Dois canais em vez de um, mas nenhum ficheiro protegido é tocado. Os 11 itens continuam invisíveis na UI.
3. **Migrar sem tocar no prompt** — rejeitada. Esvazia o bloco e deixa a instrução a contradizê-lo; o agente reconstrói o canal paralelo e o estado final é pior que o inicial.

A implementação não avança neste ponto sem a decisão. Os pontos 5.1 a 5.5 não dependem dela.

Este item corre num módulo próprio, não em `migration.rs` — esse é a migração one-shot do diretório de dados pré-v0.2 e tem outra responsabilidade.

## 6. Fora de escopo

**O contador de impacto** (*"esta pergunta bloqueou 49 vagas"*, pedido na CLA-13). Agregar propostas por pergunta só faz sentido com dados reais, e a tabela tem zero linhas hoje. Construir a agregação antes de existir o que agregar é desenhar contra um número imaginado. Fica para quando a tabela tiver histórico.

**`pendencias.vaga_id` deixar de ser `NOT NULL`** (o título da CLA-13). Com `propostas_perfil` a receber as perguntas globais, `pendencias` fica a ser o que o nome diz: o bloqueio de uma vaga concreta, que legitimamente morre com ela. O `NOT NULL` deixa de ser defeito e passa a ser a definição da entidade.

**API de patch** (`update_profile_field`, sugerida na CLA-11). Substituição total com backup e diff resolve o incidente. Uma segunda forma de escrever é mais superfície para proteger, com o mesmo resultado.

**Remover os `#[serde(default)]`.** Partem os YAMLs legados. Fica um comentário em `commands/perfil.rs` a explicar porque é que `update_profile` não pode confiar só no parse.

## 7. Ficheiros

| Ficheiro | Mudança |
|---|---|
| `src-tauri/src/mcp/tools/profile.rs` | backup rotativo, diff, gate por sessão |
| `src-tauri/src/mcp/tools/propostas.rs` | **novo** — `propose_profile_change` |
| `src-tauri/src/mcp/tools/mod.rs` | exportar a tool nova |
| `src-tauri/src/mcp/mod.rs` | despacho da tool nova |
| `src-tauri/src/mcp/server.rs` | declaração e descrição da tool nova |
| `src-tauri/src/commands/perfil.rs` | `write_mcp_config` com tipo de sessão; comentário sobre os defaults |
| `src-tauri/src/commands/linkedin.rs` | passar o tipo de sessão |
| `src-tauri/src/commands/sessao.rs` | passar o tipo de sessão |
| `src-tauri/src/db/queries.rs` | inserção em `propostas_perfil` |
| `src-tauri/src/perguntas_migracao.rs` | **novo, condicional** — só se a saída 1 de §5.6 for escolhida |
| `src-tauri/src/prompt_sistema_runtime.md` | **condicional e protegido** — uma linha, só com autorização explícita (§5.6) |

Nenhum ficheiro de frontend é tocado: a UI das propostas já existe.

## 8. Verificação

```text
YAML mínimo numa sessão interativa
→ grava, mas a resposta diz −4 experiências e aponta o .bak-1

.bak-1 contém exatamente o perfil anterior
→ restaurar é copiar um ficheiro

seis escritas seguidas
→ existem .bak-1 a .bak-5; o mais antigo caiu

update_profile numa sessão autónoma
→ não escreve; explica e aponta para propose_profile_change

propose_profile_change sem vaga_id
→ linha em propostas_perfil com vaga_id NULL; badge da Sidebar incrementa

migração com perguntas_pendentes no YAML   (só na saída 1 de §5.6)
→ 11 linhas criadas, bloco removido

migração a correr segunda vez
→ não faz nada
```

Critério humano: o usuário nunca mais tem de se lembrar de que o perfil pode desaparecer.

## 9. Superpowers

Spec aprovada → `writing-plans` → implementação em `feat/perfil-escrita-segura` → review → merge em `dev`.
