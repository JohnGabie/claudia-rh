# Pesquisa: recortes de ecrã como terceira superfície

**Data:** 2026-09-28
**Origem:** sugestão do usuário — *"não tem como tirar print do navegador somente nessas partes que você está com dificuldade de acessar?"*
**Estado:** medido e verificado visualmente

> Esta sugestão corrige uma conclusão minha anterior. Eu tinha medido *snapshot
> inteiro* contra *screenshot de viewport inteiro* e concluído que a percepção
> textual era 13× mais cara. A comparação estava incompleta: faltava o recorte.

## O resultado

Modal do Easy Apply aberto, três formas de perceber a mesma informação:

| Forma | Dimensões | **Tokens** | vs snapshot |
|---|---|---|---|
| snapshot inteiro do modal | 71 117 chars | **20 320** | — |
| screenshot do viewport | 958×888 | **1 135** | 18× mais barato |
| **recorte do modal** | 800×728 | **777** | **26× mais barato** |
| **recorte do grupo de currículos** | 753×388 | **390** | **52× mais barato** |
| recorte de um rádio só | 24×20 | 1 | inútil — pequeno demais |

O custo de uma imagem é `(largura × altura) / 750`. Recortar não é uma otimização
marginal: é linear na área, e a área cai depressa.

## O recorte é legível?

Verifiquei em vez de assumir. O recorte de **390 tokens** do grupo de currículos mostra:

- os seis PDFs, com nome completo
- a data de carregamento de cada um
- **qual está selecionado** — o rádio verde preenchido no topo

Três coisas que a árvore de acessibilidade dá pior ou não dá: ela lista os nomes e o
`[checked]`, mas não as datas nem a relação visual entre eles.

*(Nota lateral: as datas — 25/09, 18/09, 17/09, 16/09 — são os PDFs que a sessão gerou
por fora e carregou. É a [CLA-27](https://linear.app/claudia-rh/issue/CLA-27) vista do
lado do LinkedIn.)*

## Como se pede

O parâmetro é **`target`**, não `ref`. A minha primeira tentativa passou `ref` — que o
servidor ignora em silêncio, devolvendo o viewport inteiro com ar de recorte. Sétima
falha silenciosa do estudo.

```js
browser_take_screenshot({ element: "grupo de currículos", target: "e390" })
```

O `target` aceita o ref do snapshot **ou** um seletor CSS. O seletor `[role=dialog]`
falhou com *"does not match any elements"* — o modal do LinkedIn não expõe esse papel
no DOM, embora a árvore de acessibilidade o reporte como `dialog`. Mais uma assimetria
entre as duas superfícies.

## O desenho que sai disto

Três superfícies, cada uma para o que faz melhor:

```
ESTRUTURA   ← árvore de acessibilidade   rótulos, ordem, estado; barata em blocos pequenos
VER         ← recorte de ecrã            layout, estado visual, o que a árvore não exprime
ESCREVER    → DOM por índice             a única que funciona (ver teste comportamental)
```

E o que isto faz à conclusão da Q3 da CLA-7:

**Antes:** *"snapshot é 13× mais caro que screenshot — trocar a ferramenta piora tudo."*

**Agora:** a comparação certa não é snapshot-inteiro contra viewport-inteiro. É:

| | Tokens |
|---|---|
| extração dirigida pelo app (registo JSON) | ~480 |
| **recorte da zona que interessa** | **390–777** |
| screenshot de viewport | 1 135 |
| snapshot inteiro | 20 320 |

**As duas abordagens baratas custam o mesmo, e são complementares.** O registo extraído
serve para gravar na base de dados; o recorte serve para o modelo decidir quando os
rótulos não chegam.

Nada disto muda a conclusão estrutural: **só compensa se o app conduzir.** O que muda é
que, com o app a conduzir, ele tem uma ferramenta barata a mais do que eu julgava.

## O que continua por testar

- **Recorte durante o preenchimento** — fotografar o campo antes e depois de escrever
  seria uma verificação visual, e as falhas silenciosas deste estudo mostram que faz
  falta.
- **Custo real de tokenização de imagem.** A fórmula `pixels/750` é a da documentação;
  não a medi contra faturação real.
- **Recortes noutras plataformas.** Só LinkedIn.
