# Pesquisa: navegação e leitura, por plataforma

**Data:** 2026-09-28
**Motivo:** [CLA-7](https://linear.app/claudia-rh/issue/CLA-7) e [CLA-26](https://linear.app/claudia-rh/issue/CLA-26) — o LinkedIn é o caso hostil; os outros são mais fáceis?
**Como:** Playwright MCP, **perfil dedicado** (`prototipo-playwright/perfil-claudia`), nunca o Chrome pessoal do usuário

> Documento **durável**. Os dados crus estão em `prototipo-playwright/medicoes/raw2/plat-*.yml`,
> numa pasta descartável — se ela for apagada, este resumo é o que fica.

## Ponto de partida: o sistema é LinkedIn na prática

Da base de dados, 2026-09-28:

| Plataforma | Vagas | Candidaturas |
|---|---|---|
| **linkedin** | **207** | **21** |
| linkedin_rede | 12 | 0 |
| programathor | 5 | 0 |
| wellfound / indeed / gupy | 4 cada | 0 |
| greenhouse | 3 | 0 |
| weworkremotely / himalayas / geekhunter | 2 cada | 0 |
| outras 8 | 1 cada | 1 (linkedin_externo) |

**207 das 257 vagas e 21 das 22 candidaturas vêm do LinkedIn.** Tudo o resto foi
descoberto e praticamente nunca usado. Isso significa que toda a dificuldade medida
até agora — extração frágil, árvore pesada — foi medida na plataforma mais difícil, que
é também a única que produz resultado.

## O teste

Oito plataformas, a página de busca de cada uma, no perfil dedicado.

| Plataforma | Snapshot | Tokens | Vagas na página | Custo por vaga | Bloqueio |
|---|---|---|---|---|---|
| **greenhouse** | 24 523 ch | 7 007 | 50 | **140 tok** | livre |
| **remoteok** | 83 076 ch | 23 736 | 69 | **344 tok** | livre |
| **weworkremotely** | 73 268 ch | 20 934 | 57 | **367 tok** | livre |
| indeed-br | 62 211 ch | 17 775 | 33 | 539 tok | livre |
| wellfound | 67 537 ch | 19 297 | 28 | 689 tok | livre |
| himalayas | 63 426 ch | 18 122 | 24 | 755 tok | livre |
| **linkedin** | 35 103 ch | 10 030 | 7 | **1 433 tok** | livre |
| programathor | 28 476 ch | 8 136 | 1 | 8 136 tok | livre |

Referência: um screenshot de viewport custa **1 067 tokens**, igual em todas.

## Os dois achados

**Nenhuma das oito bloqueou.** Sem captcha, sem cloudflare, sem muro de login — nem o
LinkedIn, já autenticado no perfil dedicado. O medo do anti-bot não se materializou numa
sessão de leitura. Isso não diz nada sobre volume ou ritmo: oito páginas lidas uma vez
não é o mesmo que uma sessão a percorrer vagas durante uma hora.

**O LinkedIn é 10× pior que o Greenhouse por vaga descoberta.** 1 433 tokens contra 140.
A página de busca do LinkedIn devolve **7 vagas** por 10 030 tokens; a do Greenhouse
devolve **50** por 7 007. A diferença é estrutural: o LinkedIn carrega as vagas por
scroll e enche a página de promoções e recomendações; o Greenhouse serve uma lista.

O Programathor é o caso extremo do lado errado — 1 vaga por 8 136 tokens, porque a
página que visitei é de filtros por cidade, não de resultados.

## O que isto sugere para o desenho

Um desenho misto é defensável, e os números apoiam a intuição do usuário:

- **Descoberta** nas plataformas baratas. RemoteOK e WeWorkRemotely dão ~350 tok/vaga e
  não exigem login. Greenhouse é ainda mais barato, mas é um ATS por empresa, não um
  board — serve para empresas específicas, não para varrer.
- **LinkedIn para o que só ele tem**: a rede, as vagas que não estão noutro lado, o
  Easy Apply. Pagar 1 433 tok/vaga faz sentido quando a vaga não existe mais barato.

Isto liga à [CLA-26](https://linear.app/claudia-rh/issue/CLA-26), que pede fontes novas:
os números dizem quais valem a pena primeiro.

## A ressalva que vale mais que a tabela

**Escrevi três heurísticas de contagem e as três falharam em silêncio.**

1. A primeira exigia que o caminho começasse por `/job` — deu **0** para RemoteOK
   (`/remote-jobs/`), WeWorkRemotely e Programathor (`/jobs-development/`).
2. A segunda exigia um slug com 4+ palavras — deu **0** para LinkedIn
   (`/jobs/view/4467095900`) e Greenhouse (`/gitlab/jobs/8556658002`), que usam IDs
   numéricos puros.
3. Só a terceira, aceitando slug **ou** ID numérico, deu números plausíveis em todas.

Nenhuma das três deu erro. Todas devolveram um número que parecia uma medição.

Somando ao estudo da CLA-7, onde uma extração de descrição apanhou a promoção do
Premium em vez do texto da vaga: **quatro heurísticas escritas, quatro falhas
silenciosas.**

Esse é o custo real do desenho "o app conduz". Não é o contexto — esse está medido e é
favorável. É que **cada plataforma precisa do seu extrator, e um extrator partido
devolve dados plausíveis em vez de um erro.** Para uma pessoa só a manter isto, é o
risco que importa.

**Qualquer extrator precisa de validação de que apanhou a coisa certa**, não apenas de
que apanhou alguma coisa.

## O que não foi testado

- **Ritmo e volume.** Oito páginas, uma vez cada. Não diz nada sobre uma sessão longa.
- **Candidatura**, em nenhuma plataforma. Só leitura.
- **Páginas de vaga individuais** fora do LinkedIn — só as de busca.
- **Login nas outras plataformas.** Nenhuma exigiu para ler a lista; várias exigirão
  para candidatar.
