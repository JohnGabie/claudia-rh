# Q3 — Quanto cai o contexto?

**Data:** 2026-09-28 · **corrigido depois da revisão de contexto fresco**
**Playwright MCP:** 1.64.0-alpha-1789764292000
**Dados crus:** `medicoes/raw2/` (a primeira tentativa está em `medicoes/raw/`, e está errada)

## Resposta

**Não cai. Sobe — entre 10× e 16×.**

Trocar screenshot por snapshot de acessibilidade, numa página de vaga do LinkedIn,
**aumenta** o custo de contexto por chamada de percepção.

## A medição

| Vaga | Snapshot | Screenshot | Razão |
|---|---|---|---|
| 1 | 45 968 chars ≈ **13 134 tok** | 929×861 px = **1 067 tok** | **12.3× pior** |
| 2 | 52 093 chars ≈ **14 884 tok** | 929×861 px = **1 067 tok** | **13.9× pior** |
| 3 | 51 276 chars ≈ **14 651 tok** | 929×861 px = **1 067 tok** | **13.7× pior** |

Páginas esperadas até a árvore de acessibilidade parar de crescer, não medidas logo
após o `navigate`.

### A conclusão não depende da taxa de tokenização

| chars/token | Razão medida |
|---|---|
| 3.0 (conservador) | 14.4× · 16.3× · 16.0× pior |
| 3.5 (usado acima) | 12.3× · 13.9× · 13.7× pior |
| 4.5 (generoso) | 9.6× · 10.9× · 10.7× pior |

Mesmo na hipótese mais favorável ao snapshot, ele perde por dez vezes.

## Porquê — a assimetria que torna isto estrutural

**O custo de uma imagem depende da área, não do conteúdo.** Um screenshot de viewport
929×861 custa `(929 × 861) / 750 ≈ 1 067 tokens`, tenha a página o que tiver dentro.
É um teto fixo.

**O custo de um snapshot depende da complexidade.** Uma página de vaga do LinkedIn tem
centenas de nós — nav, rodapé, lista de idiomas, cards de recomendação, promoções
Premium. Tudo entra na árvore.

E há um detalhe que agrava: **93% de cada snapshot é boilerplate.** Comparei as vagas
duas a duas — 214 de 231 linhas são byte a byte iguais entre vagas diferentes. O que
distingue duas vagas são ~2.4 KB; o resto é o chrome do LinkedIn, pago outra vez em
cada chamada.

## Três erros na primeira medição

Esta secção fica registada porque os números errados chegaram a ser escritos, e a forma
como falharam é instrutiva.

**1. O modal Easy Apply nunca abriu.** O script clicava no primeiro `ref` que continha
"Candidatura simplificada" — e essa string aparece **181 vezes** na página, quase toda
nos badges dos cards de vagas recomendadas. Clicou num badge, marcou `clicked = true`, e
mediu a mesma página. O que foi rotulado "modal Easy Apply aberto" tinha **zero nós
`dialog`**, o mesmo URL e o mesmo número de campos que o estado "antes".

Faltava a asserção: verificar que um `dialog` apareceu depois do clique. Na
re-medição, com essa asserção, o modal continuou a não abrir — e **parei em vez de
tentar mais refs às cegas**, que é exatamente o caminho para uma candidatura acidental.

**O fluxo Easy Apply continua por medir.** A conclusão acima não depende dele: um
formulário só acrescenta nós à árvore, e o screenshot mantém o teto fixo — a razão só
pioraria.

**2. As páginas foram medidas antes de carregar.** O script snapshotava ~1 s depois do
`navigate`. O sinal estava à vista e passou despercebido: as três vagas deram 231, 231
e 237 linhas. Três páginas diferentes com essa variância não é convergência, é a
assinatura de todas terem parado no mesmo esqueleto. Medidas depois de estabilizarem,
dão 46 KB, 52 KB e 51 KB — **três vezes mais** que os 15 KB iniciais.

O guard `loaded()` não apanhou isto: procurava `apply|candidat|vaga|job`, palavras que
aparecem no nav mesmo com o corpo por renderizar. Apanhava página em branco, que era o
que o plano pedia; não apanhava página a meio.

**3. Bytes não são contexto.** O erro que inverteu o sinal. O screenshot foi medido em
**base64** — uma inflação de 4:3 sobre o PNG, e os bytes do PNG não têm relação nenhuma
com o custo em tokens, que é função dos píxeis. O snapshot foi medido em bytes, que
esses sim acompanham tokens.

Dois erros a empurrar na mesma direção produziram um "5×" que era o inverso da verdade.

## O que isto faz à baseline da CLA-5

A CLA-5 registou *"17 screenshots = 2.3 MB = 96% do contexto"*. Com o custo real de
imagem, 17 screenshots de viewport custam ~18 000 tokens — não 96% de uma sessão que
leu 18 milhões de tokens de cache.

**A baseline da CLA-5 precisa de ser re-derivada antes de a Fase 2 se apoiar nela.**
Está em bytes de transcript, não em tokens faturados, e as duas coisas divergem muito
em imagens. Não a corrijo aqui: os dados estão no Linear, não neste repo.

Fica, isso sim, desfeita a afirmação que eu próprio escrevi na primeira versão — de que
os 96% vinham do padrão de chamadas e não da razão por chamada. Essa reconciliação
usava 41 chamadas num sítio e 17 noutro, e explicava uma parte da diferença como se
explicasse toda. **Não se sustenta, e não era minha para afirmar** — o número está numa
issue que este protótipo não auditou.

## O que a rota perde e o que ganha com isto

**Perde:** o argumento de que a percepção textual é mais barata. É falso nesta forma.
Trocar a ferramenta e manter o padrão — o modelo a olhar para a página a cada passo —
**piora** o contexto em dez vezes ou mais.

**Ganha em clareza:** a Fase 2 só compensa se o app passar a conduzir. Preencher seis
campos com `fill()` custa **zero** tokens — nem texto, nem píxeis. A economia não vem
de a percepção ser mais barata; vem de **deixar de haver percepção** nos passos
determinísticos.

Isto deixa de ser um detalhe de implementação e passa a ser a condição de a Fase 2
valer a pena.
