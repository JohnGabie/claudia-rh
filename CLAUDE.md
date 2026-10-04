# Claudia RH

App desktop Windows (Tauri v2: Rust + React/TypeScript) que automatiza candidaturas com um agente de IA e um browser próprio.

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
- **Nenhum passo de instalação novo para o usuário.** O que o app não conseguir empacotar, baixar ou configurar sozinho, não entra. Isso vale para extensões de browser, runtimes (Node, Python), binários externos e servidores que precisam ser instalados à parte.
- **Neste projeto seguimos estritamente spec driven development.** Sem spec aprovada pelo usuário, não se escreve código. Ver "Spec driven e rodadas de teste" abaixo.
- Se uma decisão não está na arquitectura nem no design, pare e pergunte.

### Por que a regra dos passos existe

O setup chegou a ter uns 12 passos: instalar o Claude Code, acertar o PATH, instalar a extensão Claude in Chrome, registrar o native messaging host, abrir o app, criar o perfil. A extensão e o native host saíram: o browser agora é baixado pelo app. Cada passo é uma chance do usuário desistir antes de ver o produto funcionando.

Toda solução que "resolve" um problema adicionando um passo está trocando uma dor por outra — e a dor do setup é paga por **todo** usuário, enquanto a dor técnica é paga uma vez por quem constrói.

Na prática, isso descarta de saída: extensões de browser de terceiros (mesmo boas, mesmo open source), qualquer coisa que rode por `npx`, e servidores MCP externos que o usuário precise instalar. Se a capacidade for mesmo necessária, o caminho é embutir — não pedir.

Exceção única: algo que venha **dentro do instalador do Claudia RH**, com a nossa cara e a nossa responsabilidade. Aí não é um passo novo, é parte do produto.

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

## Spec driven e rodadas de teste

O objetivo de tudo isto é a **maturação do projeto**: cada mudança deixa o produto mais previsível, mais testado e mais fácil de confiar.

### Spec primeiro, sempre

1. **Toda mudança começa numa spec** em `docs/superpowers/specs/` (data no nome). A spec contém o que o usuário pediu, separado do que o assistente propõe.
2. **O usuário aprova a spec antes de qualquer código.** Escrever a spec e já lançar a implementação não vale: a aprovação é um passo próprio. O que o usuário não pediu fica marcado como proposta e só entra se ele aceitar.
3. Depois da aprovação: plano em `docs/superpowers/plans/` → implementação → testes → review. Se a implementação descobrir que a spec está errada ou incompleta, **para e atualiza a spec** em vez de seguir por conta própria.
4. A spec é a fonte da verdade do que foi pedido: o código e os testes respondem a ela, não o contrário. Decisões novas entram na spec com data.

### Rodadas de teste (todas, a cada mudança relevante)

| Rodada | O que é | Quando |
|---|---|---|
| **Unitários** | `cargo test --lib`, `npx tsc --noEmit` | sempre, antes de cada commit |
| **Integração** | os testes live com Chrome real (`src-tauri/tests/browser_live.rs` e afins, `#[ignore]`, com `CLAUDIA_TEST_CHROME`), smoke MCP (`tests/mcp-browser-smoke.mjs`) e validação do Activity log | ao mexer em browser, MCP, banco ou comandos |
| **Regressão** | a suíte inteira roda de novo depois de cada correção. Bug corrigido ganha um teste que falha antes e passa depois, e esse teste fica | sempre que um bug é corrigido |
| **Automáticos com IA** | `/claudia-teste`: o agente roda o app de verdade (`tauri dev` com CDP), abre a tela, clica, chama os comandos e confere o banco; e rodadas de agentes de verdade contra páginas de teste, com um revisor lendo o Activity log | ao mexer em tela, comando ou ferramenta do agente, e antes de dar qualquer coisa por concluída |

Regras que valem para todas:

- **Mutação:** teste de regra nova só vale se, ao quebrar o código de propósito, o teste **nomeado** falhar. Cole a linha `FAILED` como prova. Teste que não consegue falhar não protege nada.
- **Exit code real:** confira o código de saída do comando, sem `grep`/`tail` escondendo falha.
- **Evidência, não afirmação:** diga o que foi executado e o que foi visto. O que não foi verificado vai escrito como "não verificado".
- **Nunca em site real sem pedido:** os testes automáticos usam páginas locais (`src-tauri/tests/fixtures/`), Chrome de teste e pastas temporárias. Login, candidatura e envio reais só quando o usuário pedir.
- **Dados do usuário são intocáveis:** testes não escrevem no banco real, no perfil do browser do app nem em `%LOCALAPPDATA%\io.github.johngabie.claudia-rh`. Antes de qualquer coisa destrutiva, backup.
- **Review por outro modelo:** mudanças de segurança ou de ferramenta do agente passam por uma revisão independente (Opus) antes do merge, e os achados ou viram correção ou ficam registrados.
- Falha achada fora do escopo vira task no Linear (skill `linear-claudia`), não fica só na memória.

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

v0.2.0. Perfil = `candidate_base.yaml` + `search_variants.yaml` (módulo `src/components/perfil/`). Dashboard em `src/components/dashboard/`. MCP embutido em `src-tauri/src/mcp/`. Browser próprio em `src-tauri/src/browser/` (Chrome for Testing, CDP, ferramentas `browser_*` no MCP); Activity log (sempre ligado) em `src-tauri/src/activity_log/`. Sessão de execução = `claude --no-chrome` no PTY, só com o browser pronto.
