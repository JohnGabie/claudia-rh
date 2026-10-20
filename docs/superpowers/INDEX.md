# Índice de specs e planos

Criado em 2026-10-19 (spec `2026-10-19-regras-agentes-design.md`, P6). Regras de uso em `CONVENCAO.md`.

**Como ler.** Cada linha é um arquivo existente. "Estado" vem do cabeçalho do próprio arquivo; onde o cabeçalho não diz, está escrito **a confirmar** (o arquiteto confirma ao tocar na área). Os nomes de área são propostas e o usuário os ajusta quando a área for migrada para uma spec viva.

Nenhuma spec foi migrada ainda: todas continuam com o nome datado. Quando uma área for migrada, a spec viva (`specs/<área>.md`) entra aqui e as antigas passam a `absorvida por <área>`.

Estados: `viva` (é a spec vigente da área), `histórica` (valeu na época, o código seguiu em frente), `absorvida por <área>`, `rascunho` (não aprovada), `a confirmar`.

## Processo e agentes

| Arquivo | Tipo | Estado |
|---|---|---|
| `specs/2026-09-16-fonte-de-verdade-design.md` | spec | histórica (ciclo 1: fonte única) |
| `plans/2026-09-16-fonte-de-verdade.md` | plano | histórico |
| `specs/2026-09-24-skill-linear-claudia-design.md` | spec | a confirmar (skill `linear-claudia` existe no repo) |
| `specs/2026-10-19-regras-agentes-design.md` | spec | aprovada em 2026-10-19 (mensagem "blz bora"); em implementação; revisão 2 confirmada em 2026-10-19 |
| `plans/2026-10-19-regras-agentes.md` | plano | em execução |

## Sessão Claude (arranque, prompt, ambiente)

| Arquivo | Tipo | Estado |
|---|---|---|
| `specs/2026-09-17-sessao-spawn-design.md` | spec | a confirmar |
| `plans/2026-09-17-sessao-spawn.md` | plano | a confirmar |
| `specs/2026-09-25-entrega-do-prompt-design.md` | spec | parte 1 em `feat/fix-sessao-spawn`; parte 2 a confirmar |
| `specs/2026-09-25-isolar-ambiente-sessao-design.md` | spec | a confirmar (CLA-29) |

## Diagnóstico e notificações

| Arquivo | Tipo | Estado |
|---|---|---|
| `specs/2026-09-17-diagnostico-logs-design.md` | spec | a confirmar |
| `plans/2026-09-17-diagnostico-logs.md` | plano | a confirmar |
| `specs/2026-09-16-fix-notificacoes-i18n-design.md` | spec | a confirmar (o toast nativo consta como dívida em `docs/estado-do-projeto.md`) |
| `plans/2026-09-16-fix-notificacoes-i18n.md` | plano | a confirmar |

## Perfil

| Arquivo | Tipo | Estado |
|---|---|---|
| `specs/2026-09-25-perfil-escrita-segura-design.md` | spec | a confirmar (branch `feat/perfil-escrita-segura` já integrada em `dev`) |
| `plans/2026-09-25-perfil-escrita-segura.md` | plano | a confirmar |

## Browser e MCP

| Arquivo | Tipo | Estado |
|---|---|---|
| `specs/2026-09-27-browser-como-mcp-roadmap.md` | spec de rota | histórica: "superado pela decisão CDP direto"; as fases 3 e 4 seguem no Linear |
| `plans/2026-09-28-prototipo-playwright-mcp.md` | plano | histórico: "CONCLUÍDO" (resposta da CLA-7: CDP direto) |
| `specs/2026-10-02-browser-cdp-motor-design.md` | spec | implementada em `feat/browser-cdp-mcp`; candidata a **spec viva `browser`** |
| `plans/2026-10-02-browser-cdp-motor.md` | plano | a confirmar |
| `specs/2026-10-03-mcp-leitura-robusta-design.md` | spec | implementada; complementa o motor (candidata a absorção por `browser`) |
| `specs/2026-10-08-pendencias-finais-browser-design.md` | spec | aprovada e implementada; guarda de envio, eventos do browser, diálogos (candidata a absorção por `browser`) |
| `plans/2026-10-06-fechar-branch-browser.md` | plano | a confirmar |
| `plans/2026-10-02-propostas-pos-corte.md` | plano | histórico: propostas aplicadas no commit `beaeb33` |
| `evidencias/2026-10-feat-browser-cdp-mcp.md` | evidência | estado de 2026-10-09; o que foi provado e o que falta |
| `evidencias/2026-10-06-auditoria-delecao.md` | evidência | histórica; origem da regra "teste que apaga roda só no Sandbox" |
| `evidencias/linear-pendente.md` | evidência | histórica (issues criadas em 2026-10-07) |

## Documentos (currículos e cover letters)

| Arquivo | Tipo | Estado |
|---|---|---|
| `specs/2026-10-04-anexar-curriculo-design.md` | spec | aprovada em 2026-10-04 e implementada; o conteúdo por variante saiu para `feat/curriculo-por-variante` (branch parada) |
| `plans/2026-10-04-anexar-curriculo.md` | plano | a confirmar |

## Sites

| Arquivo | Tipo | Estado |
|---|---|---|
| `specs/2026-10-14-lista-de-sites-design.md` | spec | aprovada em 2026-10-14. O cabeçalho ainda diz "Nenhum código foi escrito"; a informação de que foi implementada vem do git (`feat/lista-de-sites-impl` está em `dev`), não do cabeçalho |
| `plans/2026-10-14-lista-de-sites.md` | plano | a confirmar |
| `specs/2026-10-15-sites-fluxo-guiado-design.md` | spec | o cabeçalho diz "rascunho", o plano diz "aprovada em 2026-10-15": **a confirmar** |
| `plans/2026-10-15-sites-fluxo-guiado.md` | plano | a confirmar |
| `specs/2026-10-16-vigia-campanha-banidos-notas.md` | notas | rascunho, não aprovada (vigia de navegação, campanha, níveis, banidos) |

## Teste e Sandbox

| Arquivo | Tipo | Estado |
|---|---|---|
| `specs/2026-10-01-skill-claudia-teste-design.md` | spec | a confirmar (a skill foi renomeada para `claudia-teste-e2e`) |
| `plans/2026-10-01-skill-claudia-teste.md` | plano | a confirmar |
| `specs/2026-10-05-teste-maquina-limpa-design.md` | spec | a confirmar; candidata a **spec viva `sandbox`** |
| `plans/2026-10-05-teste-maquina-limpa.md` | plano | a confirmar |
| `specs/2026-10-07-laboratorio-no-repo-design.md` | spec | aprovada e implementada (integrada em `e242f28`); candidata a absorção por `sandbox` |
| `plans/2026-10-07-laboratorio-no-repo.md` | plano | a confirmar |
| `specs/2026-10-07-sandbox-ambiente-declarativo-design.md` | spec | aprovada em 2026-10-07 e implementada; o adendo 9 (2026-10-08) vale para o ambiente `testes`; candidata a absorção por `sandbox` |
| `plans/2026-10-07-sandbox-ambiente-declarativo.md` | plano | a confirmar |

## Branches

O estado das branches não fica neste índice, porque muda a cada commit. Ele vem do relatório do agente de versão, gerado na hora (`agents/versao.md`, "Saúde das branches").

## Fora do índice

`docs/pesquisa/` (pesquisas e decisões de browser, lista de sites) e `docs/superpowers/specs/assets/` (mock visual da tela Sites) são material de apoio, não specs.
