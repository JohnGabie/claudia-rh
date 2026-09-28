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

| | Desktop (hoje) | **Android via CDP** | |
|---|---|---|---|
| árvore da vaga | 13 528 tok | **799 tok** | **16.9× menor** |
| árvore do modal Easy Apply | 20 320 tok | **2 378 tok** | **8.5× menor** |
| campos com nome utilizável | **0 de 20** | **12 de 12** | — |

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

A terceira é a mais informativa: o clique **registou** (o nó respondeu `clicou: INPUT`) e produziu efeito, mas o efeito errado. Provavelmente o LinkedIn usa um controlo próprio em que o `input[type=radio]` cru não é o alvo da interação — há um wrapper que gere a seleção.

**O que funcionou:** abrir o modal, por `Runtime.evaluate` com um `.click()` no botão encontrado por `aria-label`. Esse caminho é fiável.

**O que não funcionou:** mudar um campo específico dentro do modal.

Isto é a mesma classe de problema do desktop — e ali a solução foi o índice, que aqui não testei. A leitura melhorou muito; a escrita continua por resolver.

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

## O que continua por testar

- **Escrita por índice** dentro do modal, que foi o que funcionou no desktop.
- **Candidatura externa** por este caminho — deve funcionar (mesmo Chrome), não foi medido.
- **`Input.insertText`** para campos de texto.
- **Sessão longa**, ritmo, e se o LinkedIn trata um emulador de forma diferente.

## Nota sobre o método

Foram precisas cinco tentativas para chegar aqui, e duas conclusões minhas foram revertidas pelo caminho — primeiro *"o Android não serve"*, depois *"o clique é que é o problema"*. A ideia de usar o Chrome do Android por CDP foi do usuário, não minha.
