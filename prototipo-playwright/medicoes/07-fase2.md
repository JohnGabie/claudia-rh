# Fase 2 — quanto custa se o app conduzir

**Data:** 2026-09-28
**Dados crus:** `medicoes/raw2/vaga*-fase2.json`, `t7-fase2.json`

## A pergunta que faltava

As medições anteriores compararam *snapshot inteiro* contra *screenshot inteiro*. Essa
é a comparação de hoje — o modelo a perceber a página a cada passo. Não é como o
sistema seria construído.

Com o app a conduzir, **a árvore de acessibilidade é lida por código, não pelo modelo.**
Os 13 000 tokens nunca entram no contexto. O que entra é o registo que o app extrai.

## O resultado

| Vaga | Registo extraído | Snapshot inteiro | Screenshot |
|---|---|---|---|
| 1 | **469 tok** | 13 109 tok | 1 067 tok |
| 2 | **489 tok** | 14 881 tok | 1 067 tok |
| 3 | 76 tok *(falhou)* | 14 679 tok | 1 067 tok |

Nas duas que funcionaram: **~480 tokens por vaga** — 28× menos que o snapshot e
**2.2× menos que um screenshot**.

E o registo é **estruturado**. Os campos mapeiam diretamente nos parâmetros de
`register_vaga`:

```json
{
  "titulo": "Full-Stack Developer (Remote)",
  "empresa": "Hired",
  "easyApply": true,
  "descricao": "... 1472 chars ...",
  "url": "https://www.linkedin.com/jobs/view/4467095900/"
}
```

Um screenshot custa 1 067 tokens e devolve píxeis que o modelo tem de interpretar antes
de poder escrever na base de dados. Isto custa 469 e devolve os campos já separados.

## A falha da terceira vaga importa mais que as duas que funcionaram

A vaga 3 devolveu como descrição:

> *"Receba conselhos alimentados por IA sobre esta vaga e mais recursos exclusivos com o Premium."*

**Apanhou a promoção do LinkedIn Premium em vez do texto da vaga.** A página tinha 2 nós
de texto com mais de 60 caracteres, contra 4 nas outras duas — a descrição está
colapsada atrás de um "ver mais" e não entra na árvore até ser expandida.

Isto não é um bug de escrita rápida. É a natureza do problema.

## O que a sondagem do DOM mostrou

Antes desta medição tentei extrair por seletores CSS, com nomes de classe do tipo
`.job-details-jobs-unified-top-card__job-title`. **Devolveu todos os campos vazios** — e
o script reportou "63× mais barato", porque estava a medir um objeto vazio.

Sondando o DOM a sério:

```
h1                              : []      — a página não tem h1
classes com job|apply|company   : []      — nenhuma reconhecível
texto de botão "Candidatura…"   : presente
```

**Os nomes de classe não são utilizáveis; o texto e os papéis de acessibilidade são.**
É por isso que a extração acima usa `heading level=1`, `paragraph`, e o texto do botão,
em vez de seletores.

## O que isto custa manter

Duas em três funcionaram com uma heurística de vinte linhas escrita numa tarde. A
terceira precisa de expandir um "ver mais" antes de ler, e isso é mais um passo que o
app tem de conhecer.

**E o LinkedIn muda.** Uma extração por texto e papéis é mais resistente que uma por
classes — mas não é imune. Cada mudança de layout deles é manutenção aqui.

Isso não invalida a Fase 2; muda a natureza do custo. Deixa de ser "quanto custa o
contexto" e passa a ser **"quem mantém os extratores quando o site muda"**.

## Resposta

**A Fase 2 tem o número que a justifica: ~480 tokens por vaga contra 1 067 de um
screenshot, com dados estruturados em vez de píxeis.**

A condição continua a ser a que a correção da Q3 estabeleceu: **o app tem de conduzir.**
Se o modelo continuar a olhar para a página, o Playwright MCP é 13× pior que a extensão
de hoje. Se o app conduzir e o modelo receber registos, é 2× melhor e muito mais útil.

O custo escondido é a manutenção dos extratores, e uma vaga em três já mostrou como
falha — silenciosamente, devolvendo texto plausível que é a promoção errada.

**Qualquer extrator precisa de validação de que extraiu a coisa certa**, não apenas de
que extraiu alguma coisa. A vaga 3 passaria num teste que só verificasse "a descrição
não está vazia".
