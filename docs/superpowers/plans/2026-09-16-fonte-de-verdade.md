# Fonte de verdade única — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deixar `claudia-rh/` com uma spec viva (arquitetura + design + `CLAUDE.md`) alinhada com `candidate_base.yaml` / `search_variants.yaml`, sem alterar o prompt injetado nem o código da app, e com `dev` a administrar as branches.

**Architecture:** Copiar os docs da pasta pai (fora do git) para `docs/` dentro do repo, aplicar os patches cirúrgicos da spec, normalizar pt-BR, criar o índice `CLAUDE.md`, marcar os originais da pasta pai como HISTÓRICO, alinhar `dev` com `main` e apagar leftover locals. Zero `.rs` / `.tsx` / `.sql` / prompt injetado.

**Tech Stack:** Markdown, git, PowerShell no Windows. Sem testes de app; verificação = `git hash-object` + `Select-String`/`rg`.

**Spec:** `docs/superpowers/specs/2026-09-16-fonte-de-verdade-design.md`

## Global Constraints

- Idioma dos docs vivos, commit messages e copy nova em português: **pt-BR** (`você`, `seção`, `arquivo`, `usuário`, `controle`, `tela`). Não misturar pt-PT (`tu`, `secção`, `ficheiro`, `utilizador`, `controlo`, `ecrã`).
- README público `README.md` permanece inglês.
- **Não editar** `src-tauri/src/prompt_sistema_runtime.md`. Hash no início deste ciclo: `760054df590e16461d06e78b8ad99a2b9fa0874e`. No fim deve ser o mesmo.
- **Não editar** nenhum `.rs`, `.tsx`, `.ts`, `.sql`, `schema.sql`.
- **Não** `git add` `npm-package/` (working tree sujo, fora de escopo).
- `main` = só releases. Nunca commit em `main`. Trabalho em `feat/fonte-de-verdade`. Integração em `dev`.
- Pasta pai `C:\Users\joaog\PycharmProjects\curriculum-apply\` não é git. Banners HISTÓRICO lá não entram em `git add`.
- Não copiar para o git: `prompt-construcao-tauri.md`, `prompt-execucao-*.md`, `mudanca-*-spec.md`.
- Não documentar neste ciclo: welcome, i18n EN/PT, LinkedIn rede, updater, npm wrapper, agendamento por janelas. Feedback entra só como item da sidebar.
- Cwd de todos os comandos git: `C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh`.

## File map

| Ficheiro | Responsabilidade |
|---|---|
| `docs/arquitetura-sistema-candidaturas.md` | Spec viva de sistema (perfil, pausa, schema). |
| `docs/padrao-design-claudia-rh.md` | Spec viva visual (nav, Perfil, Histórico, Configurações). |
| `CLAUDE.md` | Índice da sessão de coding + regras git/pt-BR. |
| `docs/superpowers/specs/2026-09-16-fonte-de-verdade-design.md` | Já existe (não reescrever). |
| `docs/superpowers/plans/2026-09-16-fonte-de-verdade.md` | Este plano. |
| `src-tauri/src/prompt_sistema_runtime.md` | Congelado. |
| `../<10 markdowns>` | Banner HISTÓRICO; corpo intacto. |

---

### Task 1: Alinhar `feat/fonte-de-verdade` com `dev` local (sem push)

**Files:**
- Modify: refs git apenas (`dev`, `feat/fonte-de-verdade`)
- Test: nenhum ficheiro de app

**Interfaces:**
- Consumes: `main` em `13f724f` (ou ancestral que contenha esse commit); spec commit `d919716` já em `feat/fonte-de-verdade`
- Produces: `dev` local contém `main`; `feat/fonte-de-verdade` rebased em cima desse `dev`; hash do prompt capturado

- [ ] **Step 1: Confirmar cwd e working tree**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh"
git status -sb
git hash-object src-tauri/src/prompt_sistema_runtime.md
```

Expected: branch `feat/fonte-de-verdade`; `npm-package/` modificado (não staged); hash `760054df590e16461d06e78b8ad99a2b9fa0874e`.

- [ ] **Step 2: Merge `main` em `dev` local (sem push)**

```powershell
git checkout dev
git merge main -m "chore(git): alinhar dev com main (v0.2.0)"
git merge-base --is-ancestor 13f724f dev
echo "exit=$LASTEXITCODE"
```

Expected: `exit=0` (13f724f é ancestral de `dev`). Se o merge abrir conflitos, parar e reportar BLOCKED — não resolver adivinhando.

- [ ] **Step 3: Rebase da feature em cima do `dev` atualizado**

```powershell
git checkout feat/fonte-de-verdade
git rebase dev
git log --oneline -5
```

Expected: `feat/fonte-de-verdade` contém o merge/alinhamento de `dev` e o commit da spec (`d919716` ou equivalente após rebase). Se o rebase falhar, `git rebase --abort` e reportar BLOCKED.

- [ ] **Step 4: Verificar que o prompt injetado não mudou**

```powershell
git hash-object src-tauri/src/prompt_sistema_runtime.md
git diff HEAD -- src-tauri/src/prompt_sistema_runtime.md
```

Expected: hash `760054df590e16461d06e78b8ad99a2b9fa0874e`; diff vazio.

- [ ] **Step 5: Commit só se o merge em `dev` criou commit e ainda não está na feature**

Não criar commit vazio. Se `dev` ganhou um merge commit, ele já entra via rebase — não commitar `npm-package/`. Se não houver nada para commitar nesta task, seguir em frente sem commit.

---

### Task 2: Arquitetura viva em `docs/`

**Files:**
- Create: `docs/arquitetura-sistema-candidaturas.md`
- Source (read-only): `../arquitetura-sistema-candidaturas.md`
- Source (read-only YAML): `../mudanca-perfil-spec.md` seções 4.1 e 4.2
- Test: `Select-String` sobre o ficheiro criado

**Interfaces:**
- Consumes: Task 1 (estamos em `feat/fonte-de-verdade`)
- Produces: `docs/arquitetura-sistema-candidaturas.md` em pt-BR, sem `profile.yaml` como modelo atual

- [ ] **Step 1: Copiar o original e confirmar o "fail" (profile.yaml ainda é o modelo)**

```powershell
New-Item -ItemType Directory -Force -Path docs | Out-Null
Copy-Item "..\arquitetura-sistema-candidaturas.md" "docs\arquitetura-sistema-candidaturas.md"
Select-String -Path "docs\arquitetura-sistema-candidaturas.md" -Pattern "profile\.yaml"
```

Expected: hits na seção 5.1 e na 9.1 (ainda o modelo antigo — é o ponto de partida).

- [ ] **Step 2: Substituir a seção 5.1 inteira**

Apagar desde a linha `### 5.1 Perfil do candidato` até (não inclusive) `### 5.2 Estratégia ativa`. Colar exactamente o bloco abaixo (fence exterior de 4 backticks no plano; no destino usa headings e fences yaml normais de 3 backticks):

````markdown
### 5.1 Perfil do candidato (`candidate_base.yaml` + `search_variants.yaml`)

O modelo antigo de um único `profile.yaml` foi substituído. Existem dois arquivos com responsabilidades distintas. Nenhuma variante pode afirmar um fato que não seja rastreável a `candidate_base.yaml` (regra de honestidade da seção 2).

#### 5.1.1 `candidate_base.yaml` — banco de dados pessoal

Tudo o que é verdadeiro sobre o candidato, independente de qualquer candidatura ou variante. Editado pela aba Perfil (conversa Claude Code sem `--chrome`), nunca pela sessão de execução.

```yaml
dados_pessoais:
  nome_completo: ""
  email: ""
  telefone: ""
  localizacao_atual: ""
  links:
    - tipo: ""                   # "github" | "linkedin" | "portfolio" | "outro"
      url: ""

experiencia:
  - empresa: ""
    cargo: ""
    inicio: ""
    fim: ""                      # "" ou null se atual
    descricao: ""
    conquistas: []
    tecnologias: []

projetos:
  - nome: ""
    descricao: ""
    tecnologias: []
    url: ""
    origem: ""                   # "github" | "manual" | "linkedin"

formacao:
  - instituicao: ""
    curso: ""
    inicio: ""
    fim: ""

competencias:
  - ""

idiomas:
  - idioma: ""
    nivel: ""

gaps_conhecidos:
  - competencia: ""
    contexto: ""
    como_abordar: ""

respostas_modelo:
  porque_esta_vaga: ""
  pretensao_salarial_texto: ""
  notice_period: ""

ultima_atualizacao: ""            # ISO 8601
fontes_usadas:
  - tipo: ""                      # "github" | "linkedin" | "cv_existente" | "conversa"
    referencia: ""
    consultado_em: ""
```

#### 5.1.2 `search_variants.yaml` — variantes de busca/CV

Cada variante é uma combinação de área profissional e recorte geográfico/modelo de trabalho, com peso relativo, e gera o próprio CV a partir de `candidate_base.yaml`. Não há um único CV mestre; há um por variante (`cv_gerado_path`).

```yaml
variantes:
  - id: "backend"
    nome_exibicao: "Backend"
    peso: 50
    ativa: true
    foco_competencias: []
    foco_experiencia: []
    regioes_aceitas: []
    modelos_trabalho: []
    idiomas_aplicacao: []
    cv_gerado_path: ""
    cv_gerado_em: ""

preferencias_globais:
  faixa_salarial:
    minimo: null
    moeda: ""
    flexivel: false
  setores_evitar: []
  empresas_evitar: []

red_lines:
  - "pedido de salário fora da faixa definida"
  - "qualquer campo de dados sensíveis de saúde"
  - "perguntas abertas sobre motivação sem resposta-modelo no perfil"

perguntas_pendentes:
  - pergunta: ""
    origem_vaga: ""
    variante_relacionada: ""
    data: ""
```

A soma dos pesos das variantes ativas é normalizada no uso (peso / soma dos ativos). O algoritmo de distribuição diária por peso está fora de escopo; até existir, `peso` é só ordem de prioridade aproximada.

O campo `perguntas_pendentes` é o mecanismo de crescimento orgânico da seção 9 — a sessão de execução nunca escreve direto em `preferencias_globais` nem em `red_lines`; só propõe.
````

- [ ] **Step 3: Patch seção 3.1 — acrescentar o segundo modo de sessão**

No bullet list de responsabilidades do Tauri, **adicionar** este bullet (depois do bullet do PTY):

```markdown
- Gerir o segundo modo de sessão Claude Code: a conversa da aba Perfil, invocada sem `--chrome`, com interface de chat (não PTY/terminal). Essa sessão lê e escreve `candidate_base.yaml` e `search_variants.yaml`; nunca submete candidaturas.
```

Substituir o bullet que fala em "localização do currículo mestre" por:

```markdown
- Gerir configurações: credenciais de plataformas (LinkedIn e outras) e caminho da pasta de aplicações. Os dados do candidato não se editam aqui — vivem na aba Perfil.
```

Substituir "ficheiros de perfil e estratégia" no último bullet por "arquivos `candidate_base.yaml`, `search_variants.yaml` e estratégia".

- [ ] **Step 4: Patch fluxo de disparo (seção 4, passo 1)**

Substituir:

```text
caminho para o ficheiro de perfil
```

por:

```text
conteúdo de `candidate_base.yaml` e `search_variants.yaml`
```

- [ ] **Step 5: Patch seção 3.2 e 3.5 (nomes de arquivo, não regras)**

Em 3.2, o bullet "Lê, no arranque, o perfil do candidato" passa a:

```markdown
- Lê, no arranque, `candidate_base.yaml`, `search_variants.yaml`, a estratégia do dia (se existir), e a memória das últimas execuções.
```

Em 3.5, "perfil do candidato, estratégia ativa" passa a "`candidate_base.yaml`, `search_variants.yaml`, estratégia ativa".

- [ ] **Step 6: Patch pausa — só nomes de campo**

Na seção 6.1, substituir `` `preferencias.faixa_salarial` `` por `` `preferencias_globais.faixa_salarial` ``.

Na seção 6.1, "CV mestre" / "currículo mestre" (se aparecer) por "CV da variante correspondente". **Não reescrever** as categorias de pausa, a política de captcha, nem a etiqueta da seção 7.

- [ ] **Step 7: Patch seção 9 (crescimento orgânico)**

Substituir o parágrafo 9.1 que menciona `profile.yaml` por:

```markdown
Sempre que a sessão de execução encontra uma situação que resulta em pausa total (seção 6.1) e a resolução, depois de o usuário intervir, revela uma preferência ou regra que não estava no perfil, a sessão registra uma entrada em `perguntas_pendentes` em `search_variants.yaml` — nunca escreve direto em `preferencias_globais` nem em `red_lines`. Exemplo: se o usuário resolve manualmente uma pausa sobre disponibilidade para viajar e essa pergunta tende a se repetir, a sessão propõe uma entrada nova em `perguntas_pendentes` em vez de assumir uma resposta-padrão.
```

No 9.3, substituir `` `preferencias` `` por `` `preferencias_globais` ``. O resto da garantia de não-regressão mantém-se.

- [ ] **Step 8: Patch schema seção 10 — coluna `variante_id`**

Dentro do `CREATE TABLE vagas`, depois de `match_score TEXT`, adicionar:

```sql
    , variante_id TEXT                 -- id da variante em search_variants.yaml, se associada
```

Não inventar outras colunas (`resultado`, `fonte_conexao`, `feedbacks`) — fora de escopo.

- [ ] **Step 9: Normalizar pt-BR no ficheiro inteiro (glossary mecânico)**

Substituições globais **só neste ficheiro**, palavra inteira:

| De (pt-PT) | Para (pt-BR) |
|---|---|
| secção | seção |
| Secção | Seção |
| ficheiro | arquivo |
| ficheiros | arquivos |
| Ficheiro | Arquivo |
| utilizador | usuário |
| utilizadores | usuários |
| Utilizador | Usuário |
| ecrã | tela |
| actualmente | atualmente |
| actual | atual |
| controlo | controle |
| teu controlo | seu controle |
| teu | seu |
| tua | sua |

Não substituir `profile.yaml` restante — no Step 10 trata-se. Não tocar em código YAML de campos (`localizacao_atual` fica).

- [ ] **Step 10: Verificar profile.yaml**

```powershell
Select-String -Path "docs\arquitetura-sistema-candidaturas.md" -Pattern "profile\.yaml"
```

Expected: no máximo 1–2 hits, todos com "modelo antigo" / "substituído" (a frase de 5.1). Zero hits que ensinem `profile.yaml` como ficheiro a criar ou editar. Se sobrar a linha da seção 9 antiga, o Step 7 falhou — corrigir antes de commitar.

```powershell
Select-String -Path "docs\arquitetura-sistema-candidaturas.md" -Pattern "secção|ficheiro|utilizador|\btu\b"
```

Expected: zero (glossary aplicado). `tu` dentro de palavras como `titulo` não conta — usar word boundary.

- [ ] **Step 11: Commit**

```powershell
git add docs/arquitetura-sistema-candidaturas.md
git status
git commit -m "docs(arquitetura): candidate_base + variantes como modelo vivo"
```

Não adicionar `npm-package/`.

---

### Task 3: Padrão de design vivo em `docs/`

**Files:**
- Create: `docs/padrao-design-claudia-rh.md`
- Source (read-only): `../padrao-design-claudia-rh.md`

**Interfaces:**
- Consumes: Task 2 (arquitetura já descreve Perfil/Histórico)
- Produces: design vivo com nav, seção Perfil, Histórico, Configurações sem formulário de perfil

- [ ] **Step 1: Copiar o original**

```powershell
Copy-Item "..\padrao-design-claudia-rh.md" "docs\padrao-design-claudia-rh.md"
```

- [ ] **Step 2: Substituir a seção 3 (navegação) inteira**

Apagar desde `## 3. Estrutura de navegação` até (não inclusive) `## 4. Dashboard`. Colar:

```markdown
## 3. Estrutura de navegação

Sidebar fixa à esquerda, 220px de largura, fundo `--bg-surface`, borda direita de 1px em `--border`. Do topo para baixo:

1. **Logotipo (óculos, seção 1.1) + nome** ("Claudia RH"), área de cabeçalho da sidebar, 56px de altura, logo à esquerda do nome com 10px de gap.
2. **Dashboard**
3. **Perfil** (aba conversacional — seção 5)
4. **Histórico** (tabela de vagas processadas — seção 6)
5. **Feedback** (item da sidebar; layout desta aba não é especificado neste ciclo)
6. **Pendências** — com badge numérico vermelho (`--danger`) sempre que existir pelo menos uma pendência não resolvida. O badge mostra a contagem; acima de 9, mostra "9+".
7. **Terminal**
8. Espaço flexível (empurra o item seguinte para o fundo)
9. **Configurações** + **ícone de engrenagem** (configurações rápidas/globais — tema, toggle de notificações), no fundo da sidebar, visualmente distinto dos itens de navegação principal.

Cada item de navegação principal (2–7) é uma linha de 40px de altura, ícone (Lucide ou Tabler outline, 18px) mais label, padding horizontal de 16px. O item ativo tem fundo `--accent-soft` e texto/ícone em `--accent-strong`; os restantes têm texto `--text-secondary`, mudando para `--text-primary` no hover.
```

- [ ] **Step 3: Renumerar e substituir "Vagas" por "Histórico"**

A seção atual `## 5. Vagas` (com 5.1 / 5.2 / 5.3) é **apagada** e substituída por duas seções: nova `## 5. Perfil` e `## 6. Histórico`. As seções seguintes recuam +1:

| Antes | Depois |
|---|---|
| 4 Dashboard | 4 Dashboard (inalterada no conteúdo, só glossary pt-BR) |
| — | **5 Perfil (nova)** |
| 5 Vagas | **6 Histórico** |
| 6 Badges | 7 Badges |
| 7 Terminal | 8 Terminal |
| 8 Pendências | 9 Pendências |
| 9 Configurações | 10 Configurações |
| 10 Engrenagem | 11 Engrenagem |
| 11 Referências provisórias | 12 Referências provisórias |

Atualizar referências cruzadas internas ("secção 5.2", "Vagas") para os novos números.

Colar como `## 5. Perfil`:

```markdown
## 5. Perfil

Dois estados, usando só os tokens da seção 2. Sem paleta nova.

### 5.1 Estado resumo (padrão, quando já existe pelo menos uma variante)

Cartões das seções de `candidate_base.yaml` (dados pessoais, experiência, projetos, formação, competências, idiomas) e, abaixo, um cartão por variante de `search_variants.yaml` com nome e peso visível (barra). Cada cartão tem um botão "Editar".

Botão "Atualizar perfil" abre o chat sem foco. Botão "Nova variante" abre o chat focado em criar variante.

### 5.2 Estado chat

Bolhas de mensagem, usuário à direita, Claude à esquerda. Markdown básico (listas, negrito, código). Indicador de streaming (cursor piscante) enquanto a sessão responde. Enter envia.

Entrada geral ("Atualizar perfil") ou focada ("Editar" de um cartão — o chat já recebe qual seção/variante está em foco). A reabertura nunca começa do zero se já existirem dados: o prompt de sistema inclui os YAML atuais.
```

Colar como `## 6. Histórico` (substitui as duas sub-vistas):

```markdown
## 6. Histórico

Tabela única de tudo o que foi descoberto/processado, qualquer status. Sem as duas sub-vistas antigas ("Todas as vagas" / "Histórico") como navegação principal.

Colunas: título, empresa, plataforma, status (badge, seção 7), data, variante (`variante_id` como texto discreto, não badge de cor nova), ação (abrir detalhe / link externo / pasta de arquivos quando existir candidatura).

Filtro por status como pills acima da tabela (Todas, Descoberta, Analisada, Aplicada, Pendente revisão, Bloqueada, Pulada). Filtro por variante quando houver mais de uma variante.

Clicar numa linha abre drawer de detalhe (420px, da direita): match, motivo de status, link da vaga original, pasta de arquivos gerados se `status = aplicada`.
```

- [ ] **Step 4: Patch Pendências — deixar de apontar a "Vagas → Todas as vagas"**

Na seção de Pendências (agora 9), substituir a frase que manda pendências resolvidas para `"Vagas" → "Todas as vagas"` por: "Pendências resolvidas não aparecem mais nesta lista — o registro fica no detalhe da vaga em Histórico."

- [ ] **Step 5: Patch Configurações (agora seção 10)**

Apagar os itens 1 ("Perfil do candidato") e 2 ("Red lines e respostas-modelo"). A lista fica:

```markdown
1. **Credenciais** — uma linha por plataforma configurada (LinkedIn, e espaço para adicionar outras), com campos de usuário/password geridos via keyring — a password nunca é mostrada em texto simples depois de guardada, apenas um botão "Substituir".
2. **Disparo automático** — toggle "Ativar disparo por inatividade", e, quando ativo, um input numérico para o limiar em minutos (default 15).
3. **Pasta de aplicações** — caminho onde os arquivos gerados são guardados, com botão para escolher pasta via diálogo nativo do Windows.
4. **Pré-requisitos do sistema** — bloco de estado (não editável), mostrando se a extensão Claude in Chrome está instalada e ativa, a versão do Claude Code detectada, e se a conta tem o plano necessário. Cada item com um indicador verde/vermelho e, se vermelho, uma frase curta de como resolver.
```

Zero menções a `profile.yaml` nesta seção.

- [ ] **Step 6: Glossary pt-BR** (mesma tabela da Task 2, Step 9) neste ficheiro. Incluir `"Sob o teu controlo"` → `"Sob o seu controle"` se ainda existir.

- [ ] **Step 7: Verificar**

```powershell
Select-String -Path "docs\padrao-design-claudia-rh.md" -Pattern "profile\.yaml"
Select-String -Path "docs\padrao-design-claudia-rh.md" -Pattern "^## "
Select-String -Path "docs\padrao-design-claudia-rh.md" -Pattern "secção|ficheiro|utilizador"
```

Expected:

- `profile.yaml`: zero hits
- headings incluem `## 5. Perfil` e `## 6. Histórico`; **não** incluem `## 5. Vagas`
- glossary: zero `secção` / `ficheiro` / `utilizador`

- [ ] **Step 8: Commit**

```powershell
git add docs/padrao-design-claudia-rh.md
git commit -m "docs(design): Perfil, Historico e Configuracoes sem formulario de perfil"
```

---

### Task 4: `CLAUDE.md` vivo no repo

**Files:**
- Create: `CLAUDE.md` (raiz de `claudia-rh/`, não o da pasta pai)

**Interfaces:**
- Consumes: Tasks 2–3 (paths vivos)
- Produces: índice que um agente novo segue sem abrir a pasta pai

- [ ] **Step 1: Escrever o ficheiro com este conteúdo exacto**

Criar `CLAUDE.md` na raiz de `claudia-rh/` com **todo** o bloco abaixo (não resumir, não acrescentar fases extra):

````markdown
# Claudia RH — guia da sessão de coding

Claudia RH é uma aplicação desktop Windows (Tauri) que automatiza descoberta e candidatura a vagas, orquestrando sessões do Claude Code ligadas ao Chrome via a extensão Claude in Chrome.

Este arquivo é o ponto de entrada de qualquer sessão de coding **neste repositório**. Lê-o primeiro.

## Ordem de leitura obrigatória

Só estes paths, todos dentro de `claudia-rh/`:

1. `docs/arquitetura-sistema-candidaturas.md` — spec de sistema. Se o que fores construir contradiz este documento, o documento vence — assinala a contradição em vez de a resolver em silêncio.
2. `docs/padrao-design-claudia-rh.md` — identidade visual, tokens, navegação, layout das telas.
3. `src-tauri/src/prompt_sistema_runtime.md` — texto que a app injeta na sessão de execução. Lê para perceber o que estás a orquestrar. **Não edites este arquivo sem pedido explícito.**
4. Este `CLAUDE.md`.

Os `.md` na pasta pai do workspace PyCharm (`curriculum-apply/`, um nível acima deste repo) são **históricos**. Não são fonte de verdade.

## Regras permanentes

### Idioma

Docs vivos, commit messages e copy nova em português: **pt-BR** (`você`, `seção`, `arquivo`, `usuário`). O `README.md` público permanece inglês.

### Git

```
main     → só releases (tag vX.Y.Z). Nunca commit direto.
dev      → integração. Sempre contém main + features já merged.
feat/*   → uma preocupação por branch, criada a partir de dev atualizado.
fix/*    → igual, para correções.
```

Fluxo: atualizar `dev` com `main` → `git checkout -b feat/<nome>` a partir de `dev` → review na feature → merge em `dev` → release faz merge `dev` → `main` + tag. Depois apagar a feature branch.

Não trabalhar em `main`. Não reutilizar branches leftover.

### Decisões não cobertas

Se encontrares uma decisão de implementação não coberta pela arquitetura nem pelo design, para e pergunta — não assumas em silêncio.

## Superpowers

Specs: `docs/superpowers/specs/`
Planos: `docs/superpowers/plans/`

Um ciclo = spec aprovada → plano → worktree/branch a partir de `dev` → implementação → review → merge em `dev`.

## Dívida conhecida (não "feito")

- Pré-requisitos da extensão Chrome / versão do Claude Code / plano da conta: não validados na UI.
- Janela do Chrome "sempre visível": não implementado (API OS fora de escopo imediato).
- Toast nativo Windows: `notificacoes.rs` emite eventos Tauri mas não chama `send_sync` do plugin de notificação — o toast nativo não dispara. Correção = ciclo Superpowers próprio, não este.
- Pesquisa pontual da aba Feedback: não implementada.

## O que este repo já é

App Tauri v0.2.0. Perfil = `candidate_base.yaml` + `search_variants.yaml`, aba Perfil conversacional, Histórico, Feedback, Pendências, Terminal, Configurações (credenciais / disparo / estratégia). A sessão de execução corre `claude --dangerously-skip-permissions --chrome` no PTY; a sessão de Perfil corre sem `--chrome`.
````

- [ ] **Step 2: Verificar**

```powershell
Select-String -Path "CLAUDE.md" -Pattern "profile\.yaml"
Select-String -Path "CLAUDE.md" -Pattern "placeholder"
Select-String -Path "CLAUDE.md" -Pattern "docs/arquitetura-sistema-candidaturas.md"
```

Expected: zero `profile.yaml`; zero `placeholder`; um hit no path da arquitetura.

- [ ] **Step 3: Commit**

```powershell
git add CLAUDE.md
git commit -m "docs: CLAUDE.md vivo — indice, pt-BR e fluxo git via dev"
```

---

### Task 5: Banner HISTÓRICO na pasta pai (fora do git)

**Files:**
- Modify (fora do repo): os 10 markdowns listados abaixo em `C:\Users\joaog\PycharmProjects\curriculum-apply\`

**Interfaces:**
- Consumes: Tasks 2–4 (os destinos vivos já existem, para o banner não apontar para o vazio)
- Produces: cada ficheiro começa com o banner; corpo original intacto

Banner exacto a prefixar (uma linha em branco depois):

```markdown
> **HISTÓRICO.** Este arquivo já não é fonte de verdade.
> A spec viva está em `claudia-rh/docs/` e o índice em `claudia-rh/CLAUDE.md`.
> O prompt que a app injeta está em `claudia-rh/src-tauri/src/prompt_sistema_runtime.md`.

```

Lista (não apagar nenhum; não tocar nos `.html`):

1. `arquitetura-sistema-candidaturas.md`
2. `padrao-design-claudia-rh.md`
3. `prompt-construcao-tauri.md`
4. `prompt-sistema-runtime.md`
5. `prompt-execucao-perfil.md`
6. `prompt-execucao-feedback.md`
7. `mudanca-perfil-spec.md`
8. `mudanca-feedback-spec.md`
9. `CLAUDE.md`
10. `ci-plan.md`

- [ ] **Step 1: Prefixar cada ficheiro se ainda não tiver o banner**

Para cada path `$f` na lista, a partir de `C:\Users\joaog\PycharmProjects\curriculum-apply`:

```powershell
$root = "C:\Users\joaog\PycharmProjects\curriculum-apply"
$banner = @"
> **HISTÓRICO.** Este arquivo já não é fonte de verdade.
> A spec viva está em ``claudia-rh/docs/`` e o índice em ``claudia-rh/CLAUDE.md``.
> O prompt que a app injeta está em ``claudia-rh/src-tauri/src/prompt_sistema_runtime.md``.

"@
$files = @(
  "arquitetura-sistema-candidaturas.md",
  "padrao-design-claudia-rh.md",
  "prompt-construcao-tauri.md",
  "prompt-sistema-runtime.md",
  "prompt-execucao-perfil.md",
  "prompt-execucao-feedback.md",
  "mudanca-perfil-spec.md",
  "mudanca-feedback-spec.md",
  "CLAUDE.md",
  "ci-plan.md"
)
foreach ($name in $files) {
  $path = Join-Path $root $name
  $text = Get-Content -Raw -Path $path
  if ($text -notmatch "HISTÓRICO") {
    Set-Content -Path $path -Value ($banner + $text) -NoNewline
  }
}
```

No banner escrito no disco, as backticks são simples (`` `claudia-rh/docs/` ``), não duplicadas. O here-string acima escapa para o plano; o ficheiro real usa markdown normal.

- [ ] **Step 2: Verificar que o corpo antigo sobreviveu**

```powershell
Select-String -Path "C:\Users\joaog\PycharmProjects\curriculum-apply\arquitetura-sistema-candidaturas.md" -Pattern "HISTÓRICO" | Select-Object -First 1
Select-String -Path "C:\Users\joaog\PycharmProjects\curriculum-apply\arquitetura-sistema-candidaturas.md" -Pattern "Visibilidade sobre ocultação"
```

Expected: banner na linha 1–3; a frase de princípios ainda existe mais abaixo. Se o corpo desapareceu, restaurar do git **não dá** (pasta pai não é git) — restaurar de `claudia-rh/docs/arquitetura-sistema-candidaturas.md` **não** (já está patched). Fonte de recuperação: o original está no histórico da conversa / cópia em `docs/` **antes** do patch não existe. Por isso o Step 1 tem de **prefixar**, nunca substituir. Se `Get-Content` + concatenar falhar a meio, parar.

- [ ] **Step 3: Confirmar que git em `claudia-rh` não vê estes ficheiros**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh"
git status -sb
```

Expected: `feat/fonte-de-verdade`; possivelmente `npm-package/` dirty; **não** os `.md` da pasta pai. Não há commit nesta task.

---

### Task 6: Verificação final (o "teste" deste ciclo)

**Files:**
- Test only: nenhum write

**Interfaces:**
- Consumes: Tasks 2–5
- Produces: evidência no relatório (output dos comandos)

- [ ] **Step 1: Prompt injetado intacto**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh"
git hash-object src-tauri/src/prompt_sistema_runtime.md
git diff HEAD -- src-tauri/src/prompt_sistema_runtime.md
```

Expected: `760054df590e16461d06e78b8ad99a2b9fa0874e` e diff vazio. Se o hash mudou, **reverter o ficheiro** (`git checkout -- src-tauri/src/prompt_sistema_runtime.md`) e tratar como regressão da task que o tocou.

- [ ] **Step 2: profile.yaml não é modelo atual nos docs vivos**

```powershell
Select-String -Path docs\*.md,CLAUDE.md -Pattern "profile\.yaml"
```

Expected: cada hit (se houver) contém `antigo` ou `substituído` ou `substituido`. Zero hits em `docs/padrao-design-claudia-rh.md` e `CLAUDE.md`.

- [ ] **Step 3: Código da app fora do diff**

```powershell
git diff --stat origin/main...HEAD
git diff --name-only origin/main...HEAD
```

Expected: só `docs/**`, `CLAUDE.md`, e este plano/spec. **Proibido:** `src/`, `src-tauri/src/**/*.rs`, `src-tauri/src/db/schema.sql`, `npm-package/`, `src-tauri/src/prompt_sistema_runtime.md`.

- [ ] **Step 4: Headings do design**

```powershell
Select-String -Path docs\padrao-design-claudia-rh.md -Pattern "^## "
```

Expected: inclui `## 5. Perfil` e `## 6. Histórico`; não inclui `## 5. Vagas`.

- [ ] **Step 5: Sem commit extra**

Não commitar nesta task. Se o Step 1 obrigou a reverter o prompt, commitar essa reversão na task que o partiu, não aqui.

---

### Task 7: Higiene git (`dev` + leftover + push)

**Files:**
- Modify: refs `dev`, `origin/dev`, branches locais leftover
- **Não** merge para `main`. **Não** force-push em `main`.

**Interfaces:**
- Consumes: Tasks 1–6 (feature branch completa e verificada)
- Produces: `origin/dev` contém `origin/main` e os commits desta feature; 6 leftover locais apagadas

Leftover a apagar (já ancestrais de `main`; confirmar com `merge-base --is-ancestor` antes):

- `feat/i18n-english`
- `feat/sdd`
- `feat/tests`
- `feat/welcome`
- `feature/dashboard-remodel`
- `fix/pt-br-language`

- [ ] **Step 1: Confirmar que cada leftover é ancestral de `main`**

```powershell
Set-Location "C:\Users\joaog\PycharmProjects\curriculum-apply\claudia-rh"
foreach ($b in @(
  "feat/i18n-english","feat/sdd","feat/tests","feat/welcome",
  "feature/dashboard-remodel","fix/pt-br-language"
)) {
  git merge-base --is-ancestor $b main
  "{0} ancestor-of-main exit={1}" -f $b, $LASTEXITCODE
}
```

Expected: `exit=0` para as seis. Se alguma der `exit=1`, **não apagar essa** — reportar BLOCKED com o nome.

- [ ] **Step 2: Apagar leftover locais**

```powershell
git branch -d feat/i18n-english feat/sdd feat/tests feat/welcome feature/dashboard-remodel fix/pt-br-language
git branch
```

Usar `-d` (não `-D`). Não apagar `dev` nem `feat/fonte-de-verdade` nem `main`.

- [ ] **Step 3: Merge da feature em `dev` local**

```powershell
git checkout dev
git merge feat/fonte-de-verdade -m "docs: merge feat/fonte-de-verdade — fonte de verdade unica"
git log --oneline -8
```

Expected: `dev` contém os commits de docs + spec + plano. Continua a conter `13f724f`.

- [ ] **Step 4: Push de `origin/dev` (efeito em branch partilhada)**

Isto publica em `origin/dev`. Se o remoto rejeitar ou não houver credenciais, parar e reportar BLOCKED — **não** `push --force`.

```powershell
git checkout dev
git push origin dev
git fetch origin
git merge-base --is-ancestor origin/main origin/dev
echo "origin/dev contem origin/main exit=$LASTEXITCODE"
git rev-list --left-right --count origin/main...origin/dev
```

Expected: `exit=0`; o count left (só em main) é `0`.

- [ ] **Step 5: Não mergear `main`. Voltar à feature**

```powershell
git checkout feat/fonte-de-verdade
```

Não `git push origin main`. Não criar tag. O merge `dev` → `main` é um ciclo de release, não este.

Não há commit extra nesta task para além do merge commit em `dev` (Step 3).

---

## Self-review (spec coverage)

| Requisito da spec | Task |
|---|---|
| Arquitetura viva com candidate_base + variantes | 2 |
| Design: Perfil + Histórico + Config sem formulário | 3 |
| Feedback só como item da sidebar | 3 (nav) |
| `CLAUDE.md` índice + git + pt-BR | 4 |
| Grep `profile.yaml` só como histórico | 2, 3, 4, 6 |
| Prompt hash inalterado | 1, 6 |
| Zero código app / npm-package | 2–4 commits, 6 |
| Banners pasta pai | 5 |
| Não copiar construction/execução/mudança-spec | (omissão deliberada; 6 confirma file list) |
| `origin/dev` contém `main` | 7 |
| Apagar 6 leftover | 7 |
| `variante_id` no schema da arquitetura | 2 Step 8 |
| `perguntas_pendentes` em `search_variants.yaml` | 2 Steps 2 e 7 |
| `preferencias_globais` | 2 Steps 2, 6, 7 |
| Pausa/honestidade intocadas excepto nomes | 2 Steps 6 |
| pt-BR glossary | 2 Step 9, 3 Step 6, 4 |
| Trabalho via `dev`, nunca `main` | 1, 7 |

Placeholders: nenhum TBD. Push em `origin/dev` está no Task 7 Step 4 com abort se falhar.
