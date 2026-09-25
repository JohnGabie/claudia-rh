# Spec: Linear como caderno de tasks do claudia-rh

> Spec de design. Não é o plano de implementação.

**Data:** 2026-09-24
**Branch:** `feat/skill-linear` (a criar a partir de `dev`)
**Repo:** `claudia-rh/`

## 1. Problema

Um bug ou uma dívida encontrados a meio de uma sessão não têm onde ir. Ou interrompem o trabalho em curso, ou ficam na cabeça do autor, ou perdem-se.

A lista `Dívida conhecida` no `CLAUDE.md` é o sintoma: é manual, e já apodreceu — a cópia na raiz do workspace lista `npm-package/` com alterações não commitadas, e o `git status` está limpo a 2026-09-24.

Objetivo único: **descarregar da cabeça**. Não é gestão de projeto.

## 2. Objetivo

1. `anota isso` cria uma issue, sem interrogatório.
2. Trabalho descoberto a meio de uma sessão é registado sem parar o que está em curso.
3. Uma issue só fecha com evidência **e** confirmação.
4. Versões (`v0.3.0`, `v0.4.0`, …) existem como agrupamento, para saber o que entra na próxima release.

## 3. Fora de escopo

Cycles. Milestones. Labels de área. Convenção de prioridade escrita. Workflow states novos. Templates. Sub-issues, initiatives, estimates. Sync com GitHub Issues. Reestruturar o `CLAUDE.md` para lá de uma linha de referência.

Tudo isto foi considerado e cortado: são estrutura que custa a manter e não serve o objetivo de descarregar da cabeça.

## 4. Desenho

### 4.1 O modelo, inteiro

| | |
|---|---|
| Uma task, bug ou feature | Issue no time `Claudia-rh` (prefixo `CLA`) |
| Uma versão | Project com o nome da versão |
| Classificação | Os 3 labels que já existem: `Bug`, `Feature`, `Improvement` |
| Urgência | Priority nativa, só quando for óbvia |

Issue que é para a próxima release entra no project da versão. O resto fica no backlog sem project. Não há mais nada.

Nenhum label novo. Nenhuma taxonomia de área — com esta escala de backlog, a busca do Linear resolve.

### 4.2 Gatilhos

**a) Pedido explícito** — `anota isso`, `abre um bug para X`. Cria, devolve o ID, segue. Não pergunta sobre campos inferíveis do contexto da conversa; a fricção aqui é o que faz a pessoa deixar de anotar.

**b) Descoberta durante o trabalho** — encontrou algo fora do escopo do que está a fazer: cria a issue, avisa numa linha, **continua o trabalho em curso**. Este gatilho é o que entrega o objetivo do ciclo.

Critério: regista o que **sobrevive à sessão atual** — trabalho real que alguém teria de fazer depois. Não regista dúvida passageira nem coisa que vai ser corrigida nos próximos minutos. Registar tudo enche o caderno de ruído e o caderno deixa de ser lido.

**c) Fechamento** — nunca autónomo.

### 4.3 Fechamento

1. Reunir evidência verificável: output de teste, hash de merge em `dev`, comando executado.
2. Perguntar, citando a evidência:
   > `CLA-12` parece pronta — `cargo test` 14/14 verde, merge `a1b2c3` em `dev`. Posso fechar?
3. Com o sim: `Done` + comentário com a evidência.

Não conta: "implementei", "deve estar a funcionar". Um caderno que mente é pior que caderno nenhum — e aqui o caderno é memória, portanto o erro não tem quem o apanhe.

### 4.4 O skill

Um ficheiro só:

```
claudia-rh/.claude/skills/linear-claudia/SKILL.md
```

Contém os gatilhos, a regra de fechamento, e a mini-doc da API no fim. Não há `references/` — dividir um ficheiro deste tamanho é estrutura sem retorno.

### 4.5 A mini-doc

**MCP é o caminho normal.** `save_issue` (cria e atualiza), `list_issues`, `save_project`, `save_comment`. Já autenticado.

**GraphQL só se o MCP não alcançar.** Endpoint `https://api.linear.app/graphql`, header `Authorization: <API_KEY>` cru, sem `Bearer`. Chave em *Settings → Account → Security & Access*. O MCP não cria workflow states nem templates — é a única lacuna conhecida, e nenhuma das duas é precisa aqui.

**IDs embutidos**, para poupar um levantamento por sessão:

| | |
|---|---|
| Team `Claudia-rh` | `85ba56ec-dc3e-45b6-871a-fb8ee75c0a5c` |
| `Backlog` | `955b5e31-66e3-40f0-8cff-e53262738639` |
| `Todo` | `5568daad-14c5-481c-b196-fd9ca649ff4e` |
| `In Progress` | `ce964a2a-4713-4258-af6d-e475ef0a1bca` |
| `Done` | `d3ac5cac-23b6-48a0-b4a4-3fcdf4ff4bbc` |
| `Canceled` | `3e77c784-86a0-448d-8d67-6f0c5aacbfdb` |

São cache. Se uma chamada falhar por ID desconhecido, reconfirmar com `list_issue_statuses`.

### 4.6 Referência no `CLAUDE.md`

Uma linha na secção `Superpowers`, apontando para o skill. Nada mais — o `CLAUDE.md` aponta, o skill contém. A secção `Dívida conhecida` fica como está.

### 4.7 Setup inicial

Duas ações, pelo MCP:

1. Criar o project `v0.3.0`.
2. Cancelar as 4 issues de onboarding do Linear (`CLA-1` a `CLA-4`). Não há tool de delete; ficam em `Canceled`.

## 5. Ficheiros

| Ficheiro | Mudança |
|---|---|
| `.claude/skills/linear-claudia/SKILL.md` | novo |
| `CLAUDE.md` | uma linha na secção `Superpowers` |

Nenhum ficheiro de código é tocado.

## 6. Verificação

```text
"anota uma task: o botão de export não tem estado de loading"
→ cria issue, devolve ID, não faz perguntas

durante outro trabalho, encontra dívida colateral
→ cria issue, avisa numa linha, CONTINUA o que estava a fazer

"acabei a CLA-12"  (sem evidência)
→ NÃO fecha; procura a evidência ou pede-a

evidência reunida
→ apresenta e PERGUNTA; só fecha depois do sim
```

Critério humano: ao fim de um mês, o caderno ainda está a ser usado — porque anotar nunca custou mais do que guardar na cabeça.

## 7. Superpowers

Spec aprovada → `writing-plans` → implementação em `feat/skill-linear` a partir de `dev` → review → merge em `dev`.
