# CDP direto — implementação de referência

Prova que o app pode controlar o Chrome sem Playwright, sem extensão e sem
emulador. **Uma candidatura real foi submetida por este caminho** a 2026-09-28.

Descartável: é referência para a implementação em Rust, não código a manter.
A decisão está em `docs/pesquisa/2026-09-28-DECISAO-browser.md`.

| Script | O que faz |
|---|---|
| `cdp.mjs` | cliente JSON-RPC por WebSocket, ~40 linhas, **zero dependências** (Node ≥22 tem `WebSocket` global) |
| `candidatura-completa.mjs` | **o fluxo inteiro** — navegar, abrir modal, preencher, avançar, submeter |
| `t1-neutra.mjs` | os verbos de input numa página neutra |
| `t2-linkedin-vaga.mjs` | `getFullAXTree` numa vaga real, com contagem de tokens |
| `t3-linkedin-input.mjs` | escrever por `insertText` e `dispatchKeyEvent` |
| `t4-coordenada-visivel.mjs` | a correção do alvo: do `input` invisível ao `label[for]` |
| `t5-recorte-scroll.mjs` | `captureScreenshot` com `clip` |

## Correr

```bash
"/c/Program Files/Google/Chrome/Application/chrome.exe" \
  --remote-debugging-port=9888 \
  --user-data-dir="<caminho>/prototipo-playwright/perfil-claudia" --no-first-run

node candidatura-completa.mjs 4449864350             # lê e para antes de enviar
node candidatura-completa.mjs 4449864350 --submeter  # envia de verdade
```

**Sem `--submeter` nada é enviado.** O script para no último ecrã e reporta o que
seria submetido.

O Chrome ≥136 ignora `--remote-debugging-port` no perfil padrão — o
`--user-data-dir` próprio é obrigatório, não é preferência.

Para o Chrome dentro de um emulador Android, muda só a origem da porta; o resto
do código é idêntico:

```bash
adb forward tcp:9888 localabstract:chrome_devtools_remote
```

## As cinco armadilhas

Todas falharam em silêncio. Estão no código, comentadas onde importam.

1. **Aba congelada** — o Chrome mantém um renderer visível. Numa aba escondida
   os eventos de input são engolidos sem erro. `/json/activate` e depois *poll*
   por `visibilityState`; uma aba congelada revive 3–5 s depois.
2. **`role=dialog` ausente no DOM** — o modal do LinkedIn só o reporta na árvore
   de acessibilidade. Qualquer `querySelector('[role=dialog] …')` devolve nada.
3. **`input[type=radio]` com `opacity:0`** — 13×13 px e a caixa fica por baixo do
   botão de download. Clicar no `label[for]`, não no input.
4. **`getBoxModel` sem `scrollIntoViewIfNeeded`** — devolve coordenadas de
   viewport sem validar visibilidade. Sem o scroll, a guarda lê o elemento errado.
5. **O botão muda de nome entre passos** — `Avançar` no primeiro, **`Avaliar`**
   no segundo. Sem detecção de passo repetido, o script cicla sem dar erro.

## A guarda

Antes de qualquer clique por coordenada:

```js
const el = document.elementFromPoint(x, y);
const rot = el?.closest("label,li,button")?.innerText ?? "";
if (/enviar|submit|revisar|próximo|continuar/i.test(rot)) recusar();
```

**Não é opcional.** No modal do Easy Apply o botão "Enviar candidatura" fica no
mesmo passo dos campos, a poucas centenas de píxeis. Um clique por coordenada não
tem elemento para recusar em nosso nome.

Na implementação em Rust isto passa a ser código do app, não instrução de prompt.
