# Spec: Fonte de verdade única (ciclo 1 Superpowers)

> Spec de design. Não é o plano de implementação. Não altera comportamento da app.

**Data:** 2026-09-16
**Repo:** `claudia-rh/` (`https://github.com/JohnGabie/claudia-rh`)
**Branch de trabalho:** `feat/fonte-de-verdade` a partir de `dev` atualizado

## 1. Problema

O código em `claudia-rh/` já usa `candidate_base.yaml` + `search_variants.yaml`, aba Perfil conversacional, e Histórico. Os documentos que a sessão de coding lê primeiro ainda descrevem `profile.yaml` e um formulário de perfil em Configurações.

Isso não é só desorganização: um agente (ou uma pessoa) que segue a ordem de leitura atual implementa o modelo errado e reintroduz bugs. Há ainda duas cópias do prompt de execução — a da pasta pai está velha; a que a app injeta está em `src-tauri/src/prompt_sistema_runtime.md`.

A pasta pai `curriculum-apply/` não é git. O git real é só `claudia-rh/`. Specs Superpowers fora do git não sobrevivem a worktrees nem a PRs.

## 2. Objetivo

Uma sessão nova aberta em `claudia-rh/` consegue responder “qual é o modelo de perfil?” lendo só ficheiros versionados, sem `profile.yaml` como modelo atual, e sem tocar no texto do prompt que a app injeta.

Sucesso mensurável:

1. `claudia-rh/docs/arquitetura-sistema-candidaturas.md` descreve `candidate_base.yaml` + `search_variants.yaml`, não `profile.yaml`.
2. `claudia-rh/docs/padrao-design-claudia-rh.md` descreve aba Perfil + Histórico + Configurações sem formulário de perfil.
3. `claudia-rh/CLAUDE.md` é o índice vivo (ordem de leitura, o que está vivo, o que está morto, regras de git).
4. Grep em `claudia-rh/docs/` e `claudia-rh/CLAUDE.md` não trata `profile.yaml` como ficheiro atual (mencionar como “modelo antigo, substituído” é permitido numa frase histórica).
5. `src-tauri/src/prompt_sistema_runtime.md` tem o mesmo conteúdo de hoje (hash inalterado).
6. `origin/dev` contém `main` (já não fica 12 commits atrás).
7. Branches locais que já são ancestral de `main` foram apagadas.

## 3. Fora de escopo (explícito)

Não entra neste ciclo:

- Qualquer alteração a `.rs`, `.tsx`, `.ts`, `.sql`, `schema.sql`, ou ao texto do prompt injetado.
- Reescrever regras de pausa, honestidade, etiqueta de plataforma.
- Documentar features que existem no código mas não fazem parte desta reconciliação: welcome wizard, i18n EN/PT, LinkedIn rede, updater, npm wrapper, agendamento por janelas. (Feedback entra só como item da sidebar, porque já tem spec própria; não se escreve capítulo novo de Feedback.)
- Partir `Perfil.tsx` / `Dashboard.tsx`.
- Corrigir o toast Windows (`notificacoes.rs` sem `send_sync`).
- Commitar as alterações soltas em `npm-package/` (working tree atual). Deixá-las intocadas.

Ciclos seguintes (não este): higiene de código, bugs conhecidos, god-files.

## 4. Regras permanentes deste projeto

Estas regras valem para este ciclo e para todos os seguintes. Copiá-las para `CLAUDE.md` vivo.

### 4.1 Idioma

Tudo o que for documento vivo, commit message, branch description e copy nova de produto em português usa **pt-BR** (`você`, `seção`, `arquivo`). Não misturar pt-PT (`tu`, `secção`, `ficheiro`) nos docs vivos.

O README público em inglês (`claudia-rh/README.md`) permanece inglês — é a cara OSS, não a spec interna.

O prompt injetado já está em pt-BR. Não o “normalizar” neste ciclo.

### 4.2 Git

```
main     → só releases (tag vX.Y.Z). Nunca commit direto.
dev      → integração. Sempre contém main + features já merged.
feat/*   → uma preocupação por branch, criada a partir de dev atualizado.
fix/*    → igual, para correções.
```

Fluxo:

1. `git checkout dev && git merge main` (dev nunca fica atrás de main).
2. `git checkout -b feat/<nome>` a partir de `dev`.
3. Trabalho + review na feature branch.
4. Merge da feature em `dev`.
5. Release: merge `dev` → `main` + tag. Depois apagar a feature branch local e remota.

Não reutilizar branches leftover. Não trabalhar em `main`. Push de `dev` depois de o atualizar neste ciclo.

## 5. Mapa de ficheiros

### 5.1 Criar / tornar vivos (dentro do git)

| Destino | Origem | O que acontece |
|---|---|---|
| `docs/arquitetura-sistema-candidaturas.md` | `../arquitetura-sistema-candidaturas.md` | copiar + patch da seção 6 desta spec + normalizar pt-BR |
| `docs/padrao-design-claudia-rh.md` | `../padrao-design-claudia-rh.md` | copiar + patch da seção 7 + pt-BR |
| `CLAUDE.md` | índice novo | ordem de leitura, regras 4.1–4.2, checklist sem contradições |
| `docs/superpowers/specs/` | esta spec | já existe a partir deste ficheiro |
| `docs/superpowers/plans/` | (vazio até o plano) | pasta reservada |

### 5.2 Não tocar no corpo

- `src-tauri/src/prompt_sistema_runtime.md` — é o prompt que `include_str!` injeta. Única fonte operacional. Conteúdo congelado neste ciclo.

### 5.3 Pasta pai `curriculum-apply/` (não é git)

Cada `.md` da raiz da pasta pai ganha um banner no topo, conteúdo original intacto abaixo:

```markdown
> **HISTÓRICO.** Este ficheiro já não é fonte de verdade.
> A spec viva está em `claudia-rh/docs/` e o índice em `claudia-rh/CLAUDE.md`.
> O prompt que a app injeta está em `claudia-rh/src-tauri/src/prompt_sistema_runtime.md`.
```

Ficheiros que recebem o banner (não se apagam):

- `arquitetura-sistema-candidaturas.md`
- `padrao-design-claudia-rh.md`
- `prompt-construcao-tauri.md`
- `prompt-sistema-runtime.md`
- `prompt-execucao-perfil.md`
- `prompt-execucao-feedback.md`
- `mudanca-perfil-spec.md`
- `mudanca-feedback-spec.md`
- `CLAUDE.md`
- `ci-plan.md`

Não copiar para o git, neste ciclo: `prompt-construcao-tauri.md` (obra das fases, já feita), os `prompt-execucao-*.md` (instruções de patch, executadas por este ciclo), `mudanca-*-spec.md` (histórico da decisão; a arquitetura patched substitui). Os HTML de review (`notification-placement-review.html`, `pendencias-before-after.html`) ficam na pasta pai sem banner — não são spec.

## 6. Patch da arquitetura

Aplicar o mapa de `prompt-execucao-perfil.md` / `mudanca-perfil-spec.md` seção 7, **exceto** qualquer edição ao prompt injetado.

1. **Seção 5.1** — substituir `Perfil do candidato (profile.yaml)` por duas subseções:
   - 5.1.1 `candidate_base.yaml` — banco pessoal, schema da seção 4.1 de `mudanca-perfil-spec.md`.
   - 5.1.2 `search_variants.yaml` — variantes com peso, schema da seção 4.2.
   Renumera o resto do capítulo 5 se necessário, sem mudar o significado das seções que não são perfil.
2. **Todas as referências** a `profile.yaml` no resto do documento — apontar ao par de ficheiros correto (banco vs variante, conforme o contexto).
3. **Seção 3.1 (Tauri)** — acrescentar o segundo modo de sessão Claude Code: conversa de Perfil, **sem** `--chrome`, UI de chat (não PTY).
4. **Seção 10 (SQLite)** — coluna `variante_id TEXT` em `vagas`.
5. **Seção 9 (crescimento orgânico)** — `perguntas_pendentes` vive em `search_variants.yaml`, não em `profile.yaml`. Continua a regra: a sessão de execução nunca escreve direto em `preferencias_globais` nem em `red_lines`.
6. **Preferências** — o nome atual é `preferencias_globais` (não `preferencias`). Ajustar só onde o documento antigo usa o nome velho.
7. **Não alterar** seções de pausa (6.x), honestidade, visibilidade do Chrome, etiqueta de plataforma — só nomes de ficheiro/campo se aparecerem lá.

Fonte do schema YAML: copiar os blocos de `mudanca-perfil-spec.md` seções 4.1 e 4.2, não inventar campos.

## 7. Patch do padrão de design

1. **Navegação (seção 3)** — sidebar viva, nesta ordem:
   1. Dashboard
   2. Perfil
   3. Histórico
   4. Feedback (item da sidebar; sem capítulo de layout novo neste ciclo)
   5. Pendências (badge inalterado)
   6. Terminal
   7. Configurações no fundo + engrenagem
2. **Renomear “Vagas” → “Histórico”.** Tabela única com filtro por status (e, se já fizer sentido no texto, filtro por variante). Sem as duas sub-vistas antigas como navegação principal.
3. **Nova seção Perfil** — dois estados, tokens da seção 2 (não inventar paleta):
   - **Resumo:** cartões das seções de `candidate_base.yaml` + cartão por variante (nome, peso visível). Cada cartão tem “Editar”.
   - **Chat:** bolhas utilizador/Claude, markdown básico, indicador de streaming. Entrada geral (“Atualizar perfil”) ou focada (“Editar” de um cartão).
4. **Configurações** — remover “Perfil do candidato” e “Red lines e respostas-modelo”. Manter credenciais, disparo, pasta de aplicações, pré-requisitos.
5. Badges de status: se a tabela de Histórico mostrar variante, usar `variante_id` como informação discreta, não como badge de cor nova.

## 8. `CLAUDE.md` vivo (no repo)

Substitui o `CLAUDE.md` da pasta pai como ponto de entrada. Estrutura:

1. Uma frase: o que é o Claudia RH.
2. **Ordem de leitura obrigatória** (só paths dentro do repo):
   1. `docs/arquitetura-sistema-candidaturas.md`
   2. `docs/padrao-design-claudia-rh.md`
   3. `src-tauri/src/prompt_sistema_runtime.md` (ler para perceber o que a app injeta; **não editar** sem pedido explícito)
   4. Este `CLAUDE.md`
3. Aviso: ficheiros em `../` (pasta pai do workspace PyCharm) são históricos.
4. Regras 4.1 (pt-BR) e 4.2 (git).
5. Checklist de fases **sem contradições**. Em concreto: apagar ou reformular a linha da Fase 4 que diz que `disparar_sessao` é placeholder — a Fase 5 já o tornou real. Manter os itens em aberto (pré-requisitos Chrome, toast sem `send_sync`) como dívida conhecida, não como “feito”.
6. Onde Superpowers grava: `docs/superpowers/specs/`, `docs/superpowers/plans/`.

Não copiar o texto inteiro do `CLAUDE.md` da pasta pai se ele contradisser os docs vivos. O checklist pode ser resumido; o valor é a ordem de leitura e as regras.

## 9. Higiene git (neste ciclo)

Estado observado em 2026-09-16:

- `main` == `origin/main` == `13f724f` (tag `v0.2.0` no ancestral).
- `dev` local em `8f283e0`, 12 commits à frente de `origin/dev`, 5 commits atrás de `main`.
- Feature branches locais já ancestrais de `main`: `feat/i18n-english`, `feat/sdd`, `feat/tests`, `feat/welcome`, `feature/dashboard-remodel`, `fix/pt-br-language`.

Ações, nesta ordem, na branch de trabalho depois dos docs (ou num commit próprio de git-hygiene na mesma PR para `dev`):

1. Em `dev`: merge de `main` (fast-forward ou merge commit; o resultado deve conter `13f724f`).
2. Push de `dev` para `origin/dev`.
3. Apagar as 6 branches locais listadas acima (já estão em `main`; não há trabalho único nelas).
4. Não apagar `dev`. Não force-push em `main`. Não incluir `npm-package/` no commit.

A própria spec e o plano Superpowers vivem em `feat/fonte-de-verdade`, merged para `dev` no fim do ciclo.

## 10. Verificação

Não há testes de app neste ciclo (zero código). Verificar assim:

```text
# 1. Prompt injetado intacto
git diff HEAD -- src-tauri/src/prompt_sistema_runtime.md
# esperado: vazio

# 2. Docs vivos não ensinam profile.yaml como modelo atual
rg -n "profile\.yaml" docs CLAUDE.md
# esperado: zero hits, OU só frases do tipo "substituído / modelo antigo"

# 3. Código da app não entra no diff
git diff --stat
# esperado: só docs/, CLAUDE.md, e banners da pasta pai (pasta pai está fora do git)

# 4. Git
git rev-list --left-right --count origin/main...origin/dev
# depois do push: origin/dev contém origin/main
git branch
# as 6 leftover não aparecem
```

Critério de aceitação humano: abrir `claudia-rh/CLAUDE.md` e conseguir apontar o modelo de perfil correto em menos de um minuto, sem abrir a pasta pai.

## 11. Como o Superpowers continua depois

Este ciclo acaba quando a spec (este ficheiro) + o plano + o merge em `dev` estiverem feitos.

Ciclos seguintes, cada um com spec → plano → worktree → implementação → review, **sempre a partir de `dev`**:

1. Bugs bobos conhecidos (toast Windows, fugas de i18n) — `systematic-debugging` + TDD.
2. God-files (`Perfil.tsx`, `Dashboard.tsx`) — split com testes de caracterização.
3. Schema SQLite único (`schema.sql` a incluir colunas que hoje só existem em `ALTER`).
4. Documentar o app que realmente corre (welcome, i18n, LinkedIn rede, updater), se ainda fizer falta.

Não juntar esses ciclos a este.
