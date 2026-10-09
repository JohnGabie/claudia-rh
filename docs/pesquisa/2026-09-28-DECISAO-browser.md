# Decisão: o app controla o Chrome por CDP direto

**Data:** 2026-09-28
**Estado:** decidido pelo usuário. Substitui a extensão Claude in Chrome.
**Provado:** candidatura real submetida de ponta a ponta, só com CDP.

> Este documento é o resumo executivo de um dia de pesquisa. Os oito documentos
> `2026-09-28-*.md` nesta pasta têm as medições cruas e os erros do caminho.

## A decisão

```
Claudia RH ──lança──► Chrome (perfil dedicado, --remote-debugging-port)
     │                     ▲
     │                     │ CDP em Rust
     └── servidor MCP ─────┘
              ▲
              │ claude, grok, gemini, opencode — qualquer cliente MCP
```

**O browser é do app, não do agente.** Deixa de ser consumido e passa a ser
fornecido, como tools no servidor MCP que o app já tem.

**Perfil separado do Chrome pessoal e do browser do Claude.** Login uma vez por
plataforma, num browser que é do app.

## Porque, nas razões do usuário

> *"Toda vez que eu tenho mais de uma conta e preciso trocar, tenho que trocar
> no plugin. Isso desestabiliza se o token estoura."*

O browser deixa de estar preso a uma conta de IA. Trocar de agente não toca nele.

> *"Eu não posso usar outras IAs, que foi uma das reclamações que mais recebi
> quando lancei a Claudia-rh."*

`--chrome` era a **única** flag sem equivalente noutro agente. Virando tool MCP,
o bloqueio some. Provado: o Grok consumiu o servidor MCP do app sem alteração
nenhuma — handshake em 0.2s, 14 tools descobertas, `get_candidate_profile`
respondeu.

E há a razão que só apareceu ao contar: **tira dois passos do setup.** A extensão
Claude in Chrome e o registro do native messaging host deixam de existir. O
`welcome.rs:19` hoje verifica uma chave de registro que passa a ser irrelevante.

## O que ficou provado

**Candidatura real submetida**, 2026-09-28, vaga 4449864350, Full Stack Engineer
na iFindTech Ltd:

```
✓ "Sua candidatura na iFindTech Ltd foi enviada."
```

O fluxo completo, só com CDP:

```
navegar → abrir modal → preencher telefone → Avançar
       → selecionar currículo → Avaliar → Enviar candidatura
```

Sem Playwright, sem extensão, sem emulador. **~40 linhas de cliente WebSocket e
zero dependências** — o Node 22+ tem `WebSocket` global.

### Os números

| | Playwright | **CDP direto** |
|---|---|---|
| árvore da vaga | 13 528 tok | **10 467 tok** |
| árvore do modal Easy Apply | 20 320 tok | **4 313 tok** |
| campos com nome utilizável | 0 de 20 | **12 de 12** |
| recorte de tela | não testado | **390 tok**, legível |

O `aria-label` vazio que eu atribuía ao LinkedIn era limitação do snapshot do
Playwright. O `Accessibility.getFullAXTree` calcula o nome pelo algoritmo
completo — elemento `label`, `aria-labelledby`, conteúdo de texto.

### Todos os verbos funcionam

`Input.dispatchMouseEvent` · `dispatchTouchEvent` · `dispatchKeyEvent` ·
`insertText` · `Page.captureScreenshot` com `clip` ·
`Accessibility.getFullAXTree` · `DOM.getBoxModel` · `DOM.resolveNode`

## O padrão, e as armadilhas que ele evita

```
1. ativar a aba                       ← /json/activate + poll por visibilityState
2. Accessibility.getFullAXTree        ← rótulo + backendDOMNodeId
3. DOM.scrollIntoViewIfNeeded         ← obrigatório
4. resolver o elemento visível        ← do input cru até ao label[for]
5. DOM.getBoxModel                    ← centro
6. elementFromPoint                   ← guarda: recusar "enviar|submit|revisar"
7. Input.dispatchMouseEvent           ← clicar
8. getFullAXTree                      ← confirmar que mudou
```

Cinco armadilhas, todas encontradas por falharem em silêncio:

| Armadilha | Consequência |
|---|---|
| **aba congelada** | o Chrome mantém um renderer visível; numa aba escondida os eventos de input são engolidos sem erro |
| **`role=dialog` ausente no DOM** | o modal do LinkedIn só o reporta na árvore; qualquer seletor com esse escopo não encontra nada |
| **`input[type=radio]` com `opacity:0`** | 13×13 px, caixa por baixo do botão de download; `elementFromPoint` devolve o elemento errado |
| **`getBoxModel` sem scroll** | devolve coordenadas de viewport sem validar visibilidade; a guarda lê o elemento errado |
| **o botão muda de nome** | `Avançar` no passo 1, **`Avaliar`** no 2; sem detecção de passo repetido o script cicla em silêncio |

**A guarda do passo 6 não é opcional.** No modal do Easy Apply o botão "Enviar
candidatura" fica no mesmo passo dos campos. Um clique por coordenada não tem
elemento para recusar em nosso nome.

## Android: opção documentada, não usada

O usuário decidiu não usar. Fica registrado porque foi medido e funciona.

**Como:** `adb forward tcp:9555 localabstract:chrome_devtools_remote`. O Chrome
do Android expõe o mesmo socket de DevTools. **O código CDP é idêntico** — muda
só a origem da porta.

**O que ganha:**

| | |
|---|---|
| árvore da vaga | 799 tok (vs 10 467 no desktop) |
| árvore do modal | 2 378 tok (vs 4 313) |
| sandbox real | o agente não alcança o host — responde à CLA-30 |
| toque | evento de sistema, não `click()` sintético |

**O que custa:** ~5.5 GB, ~40s-2min de arranque, login por plataforma dentro do
emulador, e a licença do SDK proíbe embutir (permite download na primeira
execução, como o Flutter faz).

**Se for retomado**, o que falta: refazer a medição de escrita com a aba ativada
— todas as falhas registradas foram contra uma aba congelada, e os números de
escrita de lá não provam nada.

## O que fica por fazer

**Fase 2 — o app conduz.** Em ordem de dependência:

1. **Cliente CDP em Rust.** [`chromiumoxide`](https://github.com/mattsse/chromiumoxide)
   tem `DispatchMouseEventParams`, ou WebSocket na mão. A referência em
   JavaScript está em `prototipo-playwright/cdp/candidatura-completa.mjs` — é
   literalmente o que o Rust tem de reproduzir.
2. **Ciclo de vida do Chrome:** lançar com perfil dedicado, detectar que morreu,
   religar.
3. **Tools MCP novas:** `browser_ir_para`, `browser_ler_pagina`,
   `browser_recortar`, `browser_clicar`, `browser_escrever`. Cada uma mapeia num
   comando CDP, e cada chamada passa pelo `dispatch` do app — que já tem log e
   notificação.
4. **A guarda contra submissão vira código do app**, não instrução de prompt.
   Este projeto já aprendeu, no incidente de 25/09 da CLA-11, que instrução em
   prompt não é garantia.
5. **`agent_program()`** substitui `claude_program()`, com um perfil por agente.
   A tabela de equivalência de flags está em
   `2026-09-28-agentes-cli-e-portabilidade.md`.
6. **Remover `--chrome`** dos quatro call sites e a verificação do native
   messaging host em `welcome.rs`.

**O que não precisa ser construído**, e quase foi: endereçamento de elementos,
rato por coordenadas, gestão de perfil, e a tool `click_at(x,y)` que eu ia
recomendar. O CDP dá tudo.

## Nota sobre o método

Este conjunto de documentos foi corrigido três vezes no mesmo dia, sempre por
erro meu, sempre apanhado por teste independente:

1. *"o desktop dá 0 de 20 campos com nome"* — media o Playwright, não a página.
2. *"a escrita falha por pipeline de input partido"* — eu é que nunca ativava a aba.
3. *"o Android não serve para conteúdo web"* — serve, por CDP; a árvore do
   Android é que não expõe o WebView.

E a recomendação de avaliar `mcp-chrome` e `agent-browser` foi descartada pelo
próprio usuário, com a regra que virou dura no `CLAUDE.md`: **nenhum passo de
instalação novo.**

Oito heurísticas falharam em silêncio ao longo do dia. Nenhuma deu erro; todas
devolveram algo que parecia uma resposta. É a lição que a Fase 2 tem de carregar:
**validar que se apanhou a coisa certa, não que se apanhou alguma coisa.**
