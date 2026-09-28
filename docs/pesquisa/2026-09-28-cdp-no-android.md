> **Atualiza** `2026-09-28-android-emulador.md`, que concluiu que o Android não
> servia para conteúdo web. Essa conclusão estava certa sobre a árvore do
> Android e **errada sobre o Android**. Havia um caminho que eu não tinha visto.

# Pesquisa: CDP no Chrome do Android

**Data:** 2026-09-28
**Origem:** ideia do usuário — *"tenta usar o chrome do android studio"*
**Como:** emulador `bruna_pixel7_api36` (Android 16), Chrome 133, sessão do LinkedIn autenticada pelo usuário dentro do emulador

## O caminho

```
adb forward tcp:9555 localabstract:chrome_devtools_remote
```

Uma linha. O Chrome do Android expõe o mesmo socket de DevTools que o Chrome do desktop, e o `adb` traz essa porta para a máquina.

```json
{
  "Android-Package": "com.android.chrome",
  "Browser": "Chrome/133.0.6943.137",
  "Protocol-Version": "1.3"
}
```

**Isto resolve o que matou a tentativa anterior.** A árvore de acessibilidade do Android devolvia `FrameLayout label="Web View"` sem filhos, cega para a página. O CDP fala com o Chrome diretamente e vê tudo — não depende da árvore do Android para nada.

E resolve as candidaturas externas pela mesma razão: é o mesmo Chrome, o mesmo protocolo, quer a vaga fique no LinkedIn ou salte para o Greenhouse.

## Leitura: muito melhor que o desktop

Vaga real, autenticada, com a árvore estabilizada:

| | Desktop via **Playwright** | Desktop via **CDP puro** | **Android via CDP** |
|---|---|---|---|
| árvore da vaga | 13 528 tok | 10 467 tok | **799 tok** |
| árvore do modal Easy Apply | 20 320 tok | 4 313 tok | **2 378 tok** |
| campos com nome utilizável | 0 de 20 | **12 de 12** | **12 de 12** |

> **Correção (mesma data, depois de um teste independente).** A primeira versão
> desta tabela tinha só duas colunas e afirmava que o desktop dava **0 de 20**
> campos com nome. Isso media o *snapshot do Playwright MCP*, não a
> acessibilidade da página. O `Accessibility.getFullAXTree` do CDP calcula o
> nome acessível pelo algoritmo completo — elemento `label`, `aria-labelledby`,
> conteúdo de texto — e não apenas o atributo `aria-label`, que de facto está
> vazio.
>
> **O desktop por CDP puro nomeia os 12 campos, tal como o Android.** Incluindo
> o truque do estado no rótulo (`Desmarcar seleção de X` / `Selecionar X`), que
> eu julgava exclusivo do móvel.
>
> Isto reduz a vantagem do Android de 20 320 → 2 378 para **4 313 → 2 378**. O
> emulador continua a ganhar em tokens e no sandbox, mas deixa de ser o único
> sítio onde a árvore presta.

O 16.9× não é só o ecrã ser estreito. É a versão móvel do LinkedIn não carregar os cards de recomendação, as promoções Premium e o rodapé gigante que enchiam 93% da árvore no desktop.

### Os campos têm nome, e isso é o principal

No desktop, o modal do Easy Apply tinha `aria-label` vazio e IDs do React (`«re»`, `«rf»`). Nada era endereçável por nome — só por índice ou coordenada.

No Android, a mesma árvore devolve:

```
combobox "E-mail *"
combobox "Phone country code *"
textbox  "Telefone*"
radio    "Selecionar resume cv_joao_gabriel_hire_feed.pdf"
radio    "Desmarcar seleção de resume cv_fullstack_engineer.pdf"
button   "Enviar candidatura"
checkbox "Siga a empresa Hired para conhecer as novidades."
```

**12 de 12 com nome.** E repare no detalhe: o rótulo do rádio **codifica o estado** — *"Desmarcar seleção de X"* quando X está selecionado, *"Selecionar X"* quando não está. Não é preciso ler a propriedade `checked`.

O botão de submeter também vem identificado sem ambiguidade: `button "Enviar candidatura"`. A guarda de segurança fica trivial.

## Escrita: não resolvida

Três formas tentadas para mudar o currículo selecionado, todas falharam:

| Tentativa | Resultado |
|---|---|
| `DOM.getBoxModel` → `Input.dispatchMouseEvent` | resolveu a caixa (287, 667), clicou, **nada mudou** |
| mesma caixa → `Input.dispatchTouchEvent` | **nada mudou** |
| `DOM.resolveNode` → `Runtime.callFunctionOn(click)` | clicou no `INPUT` e **desmarcou os dois** — grupo ficou sem seleção |

A terceira é a mais informativa: o clique **registou** (o nó respondeu `clicou: INPUT`) e produziu efeito, mas o efeito errado.

### Três causas, e a minha hipótese principal estava errada

Dois testes independentes — um no Chrome do desktop, outro no próprio emulador — isolaram o que se passava. **A escrita não estava partida; eu é que estava a escrever na aba errada.**

**1. A aba alvo não estava visível.** Esta é a causa principal, e é minha.

O Chrome no Android mantém **um** renderer visível. Uma sondagem encontrou 13 abas nesse emulador: 1 `visible`, 11 em `<TIMEOUT>` — congeladas ou descartadas, incluindo a do LinkedIn. E os meus scripts escolhiam a aba por URL e **nunca a ativavam**:

```js
const p = alvos.find((t) => t.type === "page" && /jobs\/view/.test(t.url || ""));
```

Verificado: `t20-android-vaga.mjs`, `t21-android-modal.mjs` e `t25-resolve.mjs` têm **zero** chamadas a `/json/activate` ou `Page.bringToFront`.

Um A/B controlado numa página neutra, mesma coordenada, mesma sessão, reproduz exatamente a tabela de falhas acima:

| Verbo | aba **visível** | aba **escondida** |
|---|---|---|
| `Input.dispatchTouchEvent` | OK | **nada — o comando nem responde** |
| `Input.dispatchMouseEvent` | OK | OK *(caminho diferente)* |
| `adb shell input tap` | OK | nada |
| `JS .click()` | OK | **OK** |
| `adb shell input text` | OK | **nada** |
| `Input.insertText` | OK | OK |

Touch morto, `.click()` vivo — o meu sintoma, explicado.

A correção, verificada na mesma aba que antes falhava:

```js
await fetch(`http://localhost:${PORT}/json/activate/${alvo.id}`);  // ou cdp("Page.bringToFront")
// depois esperar por document.visibilityState === "visible"
```

Uma aba congelada revive 3–5 s depois da ativação — tem de ser *poll*, não `sleep` fixo.

**2. A coordenada apontava para o elemento errado.** Causa adicional e independente, encontrada no desktop: o `input[type=radio]` do LinkedIn tem `opacity: 0` e 13×13 px, e a caixa dele fica **por baixo do botão de download**. `elementFromPoint` devolvia `BUTTON "Baixar resume…"`.

A correção: subir do `input` até ao `label[for]` visível. Ativar a aba **não** resolve esta — são problemas diferentes que calharam ao mesmo tempo.

**3. `hw.keyboard=no` no AVD.** O teclado do utilizador, e nada a ver com o CDP.

`D:\Android\avd\bruna_pixel7_api36.avd\config.ini`, linha 77. Com `no`, o Android regista `BuiltInKeyboardId=-2` e trata o teclado como não-alfabético: teclas de navegação passam, alfanuméricas em campos de texto não.

**Não era regressão desse dia** — o `config.ini` não era tocado desde 01/09/2026. O que mudou foi o contexto: mais abas abertas, e eu a falar com a errada.

### A premissa que eu dei aos agentes estava invertida

Eu disse que `Input.dispatchMouseEvent` passava pelo pipeline de input do Android, e que o teclado partido explicava o clique falhado. **Não passa.** Os verbos de input do CDP são injetados diretamente no renderer do Chrome, não no `InputDispatcher` do Android.

Prova: `adb shell input tap` e `input text` funcionavam **antes e depois** de qualquer correção, no mesmo AVD. Os dois sintomas nunca estiveram ligados.

### E uma armadilha de segurança que saiu daí

`DOM.getBoxModel` devolve coordenadas relativas ao viewport **sem validar se o elemento lá está**. O botão "Enviar candidatura" dava `y = 966` com `innerHeight = 905` — fora do ecrã. A guarda `elementFromPoint` devolvia `NADA` e recusava o clique: **decisão certa pelo motivo errado.**

É preciso `DOM.scrollIntoViewIfNeeded` antes de qualquer `getBoxModel`. Sem isso, uma coordenada fora do viewport pode cair noutro elemento em vez de em nada — e aí a guarda lê o rótulo errado.

## O que isto custa

| | |
|---|---|
| imagem de sistema + emulador + adb | ~5.5 GB |
| arranque | ~40 s a 2 min |
| login | uma vez por plataforma, **dentro** do emulador |
| licença | proíbe embutir; permite descarregar na primeira execução |

E ganha o que nenhuma outra opção dá: **um sandbox de verdade.** A [CLA-30](https://linear.app/claudia-rh/issue/CLA-30) existe porque a sessão escreveu 11 scripts e instalou 63 pacotes npm na máquina do usuário. Dentro do emulador isso é impossível — o agente não alcança o host.

## A consequência que vale mais que os números

**O código CDP é o mesmo** apontado ao Chrome do desktop ou ao do emulador. Mesmo protocolo, mesmos comandos, mesma versão 1.3.

Constrói-se uma vez e o usuário escolhe:

```
Claudia RH ──CDP──► Chrome do desktop          rápido, sem custo, sem sandbox
           └─CDP──► Chrome no emulador Android  5.5 GB, sandbox real, árvore 17× menor
```

Não é uma decisão que tenha de ser tomada agora. É a mesma implementação a servir as duas.

## Alterações feitas ao ambiente do utilizador

Durante o diagnóstico, dois valores foram mudados em
`D:\Android\avd\bruna_pixel7_api36.avd\config.ini`. **Original guardado em
`config.ini.bak-20260928`**, verificado.

| | antes | depois | porquê |
|---|---|---|---|
| `hw.keyboard` | `no` | `yes` | o teclado do utilizador passa a funcionar |
| `hw.ramSize` | `2G` | `4096` | com 2.5 GB e 13 abas o Chrome descartava as de segundo plano — era isso que as tornava indirigíveis |

Ambos sobrevivem a reboot: verificado em boot frio, o `hardware-qemu.ini` mostra
`hw.keyboard = true` e `hw.ramSize = 4096`.

**Nenhum passo manual recorrente.** A ativação da aba é código — duas linhas num
script, não estado do emulador.

## O que continua por testar

- **Repetir a medição da escrita com a aba ativada.** Todas as falhas de escrita
  registadas acima foram medidas contra uma aba escondida. Os números de leitura
  continuam válidos (a leitura funciona em abas congeladas); os de escrita não
  provam nada.
- **Candidatura externa** por este caminho — deve funcionar, é o mesmo Chrome.
- **Sessão longa**, ritmo, e se o LinkedIn trata um emulador de forma diferente.

## Nota sobre o método

Este documento foi corrigido duas vezes no mesmo dia. Primeiro porque afirmava
que o desktop dava 0 de 20 campos com nome — media o Playwright, não a página.
Depois porque atribuía a falha de escrita a um pipeline de input partido, quando
a causa era eu nunca ativar a aba.

As duas correções vieram de testes independentes. Nenhuma teria aparecido se eu
tivesse confiado na primeira explicação que encaixava nos sintomas.

## Nota sobre o método

Foram precisas cinco tentativas para chegar aqui, e duas conclusões minhas foram revertidas pelo caminho — primeiro *"o Android não serve"*, depois *"o clique é que é o problema"*. A ideia de usar o Chrome do Android por CDP foi do usuário, não minha.
