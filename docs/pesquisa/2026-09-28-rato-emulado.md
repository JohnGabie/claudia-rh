# Pesquisa: rato emulado por coordenadas

**Data:** 2026-09-28
**Origem:** sugestão do usuário — *"a gente podia adicionar um mouse, como o Claude Code faz (…) através de um print, você conseguiria dizer vai na posição X e Y ali e clica"*
**Estado:** provado, com o ciclo visual completo

## Funciona

O ciclo que o usuário descreveu, executado de ponta a ponta:

1. Abri o modal do Easy Apply e fotografei o viewport (958×888)
2. **Olhei para a imagem** e escolhi um alvo: o **terceiro** currículo da lista,
   `cv_joao_gabriel_yby_mara.pdf`, cujo botão estava em ~**(410, 708)**
3. `page.mouse.move(410, 708)` → `down()` → `up()`
4. A seleção passou do índice 0 para o índice **2**

```
antes                              depois
  [0] ● cv_fullstack_engineer        [0] ○ cv_fullstack_engineer
  [1] ○ cv_joao_gabriel_hire_feed    [1] ○ cv_joao_gabriel_hire_feed
  [2] ○ cv_joao_gabriel_yby_mara     [2] ● cv_joao_gabriel_yby_mara   ← o alvo visual
```

**Acertou o índice 2, não o 1.** Se a coordenada estivesse errada, o vizinho mais
próximo seria o 1 — chegar ao 2 é o que prova que veio da imagem e atingiu o pretendido.

Restaurado ao original, modal fechado, **nada submetido**.

## Como, já que a ferramenta não existe

O Playwright MCP **não tem** ferramenta de coordenadas. Nenhuma das 25 aceita `x`/`y`:
o `browser_click` só aceita `element` e `target`.

O caminho é `browser_run_code_unsafe`, que corre código Playwright com o objeto `page`:

```js
async (page) => {
  await page.mouse.move(410, 708);
  await page.mouse.down();
  await page.mouse.up();
}
```

`page.mouse` despacha `Input.dispatchMouseEvent` por CDP — eventos **trusted**, como os
de uma pessoa. É o mesmo mecanismo que a ferramenta `computer` do Claude in Chrome usa.

## O preço, e é alto

A descrição da própria ferramenta:

> *"Unsafe: executes arbitrary JavaScript in the Playwright server process and is
> **RCE-equivalent**."*

Não é um aviso decorativo. Expor isto ao agente é dar-lhe execução arbitrária de código
no processo que controla o browser autenticado do usuário.

**Isto colide de frente com a [CLA-30](https://linear.app/claudia-rh/issue/CLA-30)**, que
quer confinar a sessão. A capacidade que resolve o problema de endereçamento é
precisamente a mais perigosa do conjunto.

Duas saídas possíveis, nenhuma testada:

1. **O app faz o clique, não o agente.** O agente pede *"clicar em (410, 708)"* como
   dados; o app valida e executa. `browser_run_code_unsafe` nunca chega à lista de
   ferramentas do agente.
2. **Uma tool MCP própria** no servidor do claudia-rh, `click_at(x, y)`, que faz só isso
   e nada mais.

A segunda é provavelmente a certa: mantém a superfície estreita e deixa o registo do
que foi clicado passar pelo `dispatch` do app, que já tem log e notificação.

## Onde isto encaixa

Agora há quatro formas de atingir um elemento, e a tabela completa do estudo:

| Estratégia | Funciona neste modal | Custo |
|---|---|---|
| `browser_click` com `ref` da árvore | **não** | — |
| `.click()` no DOM por `aria-label` | **não** — vazio | — |
| `.click()` no DOM por texto do rótulo | **não** | — |
| `.click()` no DOM por **índice** | sim | leitura da árvore |
| **`page.mouse` por coordenada** | **sim** | recorte: 390–777 tok |

As duas que funcionam são complementares, não alternativas:

- **Índice** serve quando a árvore e o DOM concordam na ordem. É barato e não precisa
  de imagem. Parte se o site reordenar.
- **Coordenada** serve quando nada mais identifica o elemento. Precisa de ver — um
  recorte de 390 tokens chega — e é imune a reordenação, porque não depende de
  identidade nenhuma.

O desenho completo que sai dos quatro documentos desta pesquisa:

```
ESTRUTURA   ← árvore de acessibilidade   rótulos, ordem, estado
VER         ← recorte de ecrã            390–777 tok, quando a árvore não chega
ESCREVER    → DOM por índice             barato, frágil a reordenação
FALLBACK    → rato por coordenada        quando nada mais identifica o alvo
VERIFICAR   ← árvore outra vez           única fonte de verdade
```

## A guarda que este caminho exige

Um clique por coordenada **não tem elemento para recusar**. As guardas anteriores
liam o rótulo do elemento antes de clicar; aqui não há elemento até se clicar.

A solução usada foi ler primeiro o que ocupa aquele pixel:

```js
const el = document.elementFromPoint(410, 708);
const ctx = el?.closest("label,li,fieldset,button")?.innerText;
if (/enviar|submit|revisar|próximo|continuar/i.test(ctx)) recusar();
```

No modal do Easy Apply isto não é teórico: **"Enviar candidatura" está no mesmo passo**,
a poucas centenas de píxeis. Uma coordenada errada por 200 px numa vertical é uma
candidatura submetida em nome do usuário.

## O que continua por testar

- **Escrever texto por teclado emulado** (`page.keyboard.type`) — só o clique foi testado.
- **Coordenadas após scroll.** A posição muda; a imagem tem de ser recente.
- **Fiabilidade da leitura visual** em elementos pequenos ou sobrepostos. Um rádio de
  24×20 px tem pouca margem.
