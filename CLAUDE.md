# Claudia RH

App desktop Windows (Tauri v2: Rust + React/TypeScript) que automatiza candidaturas com Claude Code + Chrome.

Este arquivo é o ponto de entrada de qualquer sessão de coding **neste repositório**.

## Ordem de leitura

1. `docs/arquitetura-sistema-candidaturas.md` — spec de sistema. Se o que você for construir contradiz este documento, o documento vence — assinale a contradição em vez de a resolver em silêncio.
2. `docs/padrao-design-claudia-rh.md` — identidade visual, tokens, navegação, layout.
3. `src-tauri/src/prompt_sistema_runtime.md` — texto que a app injeta na sessão de execução. **Não edite sem pedido explícito.**
4. Este `CLAUDE.md`.
5. `.claude/QUALIDADE.md` — roadmap de qualidade/refactor (quando existir).

## Convenções

- **Comentários de código: inglês.** Sempre.
- **Commit messages: inglês.**
- **Identificadores novos: inglês.** Código legado em português não se renomeia em massa.
- **Copy da UI: pt-BR.** Docs vivos (`docs/`, este ficheiro) também em pt-BR.
- **README público: inglês.**

## Git

```
main     → só releases (tag vX.Y.Z). Nunca commit direto.
dev      → integração. Sempre contém main + features já merged.
feat/*   → uma preocupação por branch, criada a partir de dev atualizado.
fix/*    → igual, para correções.
```

Fluxo: atualizar `dev` com `main` → `git checkout -b feat/<nome>` a partir de `dev` → review → merge em `dev` → release faz merge `dev` → `main` + tag.

## Regras duras

- **NUNCA mudar o `identifier` Tauri** em `src-tauri/tauri.conf.json` (`io.github.johngabie.claudia-rh`). Determina pasta de dados do usuário e identidade do instalador. Teste `identifier_is_frozen` garante isto. Ver `src-tauri/src/migration.rs`.
- Se uma decisão não está na arquitectura nem no design, pare e pergunte.

## Superpowers

Specs: `docs/superpowers/specs/`
Planos: `docs/superpowers/plans/`

Um ciclo = spec aprovada → plano → branch a partir de `dev` → implementação → **teste no app rodando (`claudia-teste-e2e`)** → review → merge em `dev`.

O passo de teste é obrigatório:

- Roda o app de verdade (`tauri dev` com CDP) e exercita o que mudou. `cargo test` e `tsc` não bastam.
- O resultado entra no ledger ou na descrição do PR, com evidência: o que foi aberto, o que foi chamado, o que a tela mostrou.
- Mudança só de backend, sem efeito visível: chama o comando por `invoke` e confere o banco.
- Bug corrigido só fecha depois de reproduzido antes e confirmado depois.

Tasks, bugs e o que entra em cada versão vivem no Linear (time `CLA`), não em listas neste ficheiro. Como registar e quando pode fechar: skill `linear-claudia`.

## Diagnóstico

Em `npm run tauri dev`, lê `debug-logs/events.jsonl` e `debug-logs/pty-tail.log` na raiz do repo **antes** de perguntar o que aconteceu. Em produção (Windows): `%APPDATA%\io.github.johngabie.claudia-rh\diagnostics\`.

## Build

- Frontend: `npm run build` (`tsc` + vite). Typecheck: `npx tsc --noEmit`.
- Backend: em `src-tauri/`, `cargo check` / `cargo test` / `cargo clippy`.
- App: `npm run tauri dev`.

## Teste em máquina limpa

| Quero… | Skill |
|---|---|
| ver, clicar ou conferir o app rodando, tirar print, reproduzir bug | `claudia-teste-e2e` (local ou no Sandbox) |
| montar um Windows limpo, escolher os dados, publicar ou instalar o app, ligar o CDP no guest | `claudia-teste-sandbox` |

A `claudia-teste-sandbox` usa um laboratório Windows Sandbox mantido fora do repositório, localizado pela variável `SANDBOX_LAB_DIR`. Cenários de dados: `novo`, `mock` e `copia` (este só com autorização do usuário na rodada). Falha do app num Windows limpo é resultado do teste: registre no Linear, não contorne no laboratório.

## Dívida conhecida

- Toast nativo Windows: `notificacoes.rs` emite eventos Tauri mas não chama o plugin de notificação — o toast do sistema não dispara.
- Pré-requisitos Chrome / plano da conta: não validados na UI.
- Pesquisa pontual da aba Feedback: não implementada.

## O que este repo já é

v0.2.0. Perfil = `candidate_base.yaml` + `search_variants.yaml` (módulo `src/components/perfil/`). Dashboard em `src/components/dashboard/`. MCP embutido em `src-tauri/src/mcp/`. Sessão de execução = `claude --chrome` no PTY; sessão de Perfil = sem `--chrome`.
