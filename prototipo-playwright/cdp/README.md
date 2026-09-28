# CDP direto — implementação de referência

Scripts descartáveis que provaram o caminho CDP sem Playwright e sem emulador.
Ficam aqui como referência para a implementação em Rust, não como código a manter.

| Script | O que prova |
|---|---|
| `cdp.mjs` | cliente JSON-RPC por WebSocket, ~40 linhas. **Sem dependências** — Node ≥22 tem `WebSocket` global |
| `t1-neutra.mjs` | os verbos de input numa página neutra: mouse, touch, key, insertText |
| `t2-linkedin-vaga.mjs` | `getFullAXTree` numa vaga real, com contagem de tokens |
| `t3-linkedin-input.mjs` | escrever em campos do LinkedIn por `insertText` e `dispatchKeyEvent` |
| `t4-coordenada-visivel.mjs` | a correção do alvo: subir do `input` invisível até ao `label[for]` |
| `t5-recorte-scroll.mjs` | `captureScreenshot` com `clip`, e `scrollIntoViewIfNeeded` antes de medir |

## O padrão que estes scripts estabelecem

```
1. Accessibility.getFullAXTree        → rótulo e backendDOMNodeId
2. DOM.scrollIntoViewIfNeeded         → obrigatório, senão a caixa vem fora do ecrã
3. resolver o elemento visível        → do input cru até ao label[for] ou ancestral > 40x20
4. DOM.getBoxModel                    → centro da caixa
5. document.elementFromPoint (guarda) → recusar se o rótulo bater no vocabulário de submissão
6. Input.dispatchMouseEvent           → clicar
7. Accessibility.getFullAXTree        → confirmar que mudou
```

**O passo 5 não é opcional.** No modal do Easy Apply o botão "Enviar candidatura"
fica no mesmo passo dos campos. Um clique por coordenada não tem elemento para
recusar em nosso nome.

**O passo 2 também não.** Sem ele, `getBoxModel` devolve uma coordenada que pode
estar fora do viewport — e aí a guarda lê o elemento errado, ou nenhum.

## Como correr

O Chrome ≥136 ignora `--remote-debugging-port` no perfil padrão. É preciso um
`--user-data-dir` próprio:

```bash
"/c/Program Files/Google/Chrome/Application/chrome.exe" \
  --remote-debugging-port=9888 \
  --user-data-dir="$TEMP/cdp-teste" --no-first-run
node t1-neutra.mjs
```

Para o Chrome dentro do emulador Android, muda só a origem da porta:

```bash
adb forward tcp:9888 localabstract:chrome_devtools_remote
```

O resto do código é idêntico. É o mesmo protocolo 1.3.
