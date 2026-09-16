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
fix/*   → igual, para correções.
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
