# Q4 — A dependência de `npx` é aceitável?

**Data:** 2026-09-28
**Node:** v24.17.0
**npx:** 11.13.0
**Chrome:** 153.0.8010.54

| Medida | Valor |
|---|---|
| `npx` a frio (primeira execução, cache limpo) | **3s** |
| `npx` a quente (3 execuções) | **1490ms, 1501ms, 1481ms** — média 1.49s |
| Disco: entrada `@playwright/mcp` no cache npx | **19 MB** |
| Browser baixado pelo Playwright | **não** — ver ressalva |

## Correção ao plano

O plano manda limpar `~/.npm/_npx`. **No Windows esse caminho não existe** — o cache
está em `%LOCALAPPDATA%\npm-cache\_npx` (via `npm config get cache`). A primeira
medição que fiz deu 2s e foi na verdade **a quente**, porque o cache real nunca foi
tocado.

A medição a frio acima é legítima: removi a entrada `9833c18b2d85bc59` do cache real
antes de medir, e ela voltou a aparecer com 19 MB depois.

## Ressalva sobre os 19 MB vs 628 MB

O diretório `_npx` inteiro tem **628 MB em 17 entradas** — cache acumulado de outros
pacotes, sem relação com este estudo. O `@playwright/mcp` custa **19 MB**, não 628.

## Ressalva sobre "não baixou browser"

Existe `%LOCALAPPDATA%\ms-playwright` com **1.6 GB** de browsers. Verifiquei as datas:

```
2025-09-29  chromium-1169, firefox-1482, webkit-2158, ffmpeg-1011, winldd-1007
2026-06-05  chromium-1223, chromium_headless_shell-1223
```

São de meses antes desta medição — vieram do plugin Playwright que o usuário já tem
instalado, não desta execução.

**Mas `--help` não abre browser nenhum.** Esta medição não prova que `--browser chrome`
não baixa nada ao arrancar de verdade. A Tarefa 2 é a primeira que lança um browser;
é lá que isto se confirma ou cai.

Numa máquina sem o plugin Playwright, o resultado pode ser outro — e essa é a máquina
do usuário final, não esta.

## Resposta

**Aceitável, com uma condição por verificar.** O custo recorrente é 1.5s por arranque
e 19 MB em disco, o que é baixo para uma sessão que dura minutos. O `npx` frio de 3s
acontece uma vez.

A condição: o app não distribui Node. Adotar isto significa exigir Node na máquina do
usuário, ou empacotá-lo. Essa decisão é da Fase 2, não deste protótipo — mas fica dito
que a dependência existe e não é gratuita.

## Correção da revisão: "a frio" ainda é morno

A revisão apanhou que limpei apenas `_npx`, não o `_cacache` do npm. Os tarballs de
19 MB continuavam locais, portanto os **3s são "cache de resolução limpo, cache de
pacotes quente"** — não o número de uma máquina nova.

Numa máquina sem nada, o primeiro arranque será mais lento. Não o corrigi porque o
custo de decisão é baixo (3s contra talvez 15s, uma vez) e a ressalva que importa para
a Q4 é outra e já está escrita: **o app não distribui Node.**
