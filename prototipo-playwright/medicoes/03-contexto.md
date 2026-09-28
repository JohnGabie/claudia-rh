# Q3 — Quanto cai o contexto?

**Data:** 2026-09-28
**Playwright MCP:** 1.64.0-alpha-1789764292000
**Vagas:** três reais do LinkedIn, busca `developer` + filtro Easy Apply, Brasil

## Por vaga, só a descrição

| Vaga | Snapshot (bytes) | Screenshot (bytes) | Razão |
|---|---|---|---|
| 1 | 14 943 | 74 556 | **5.0×** |
| 2 | 15 137 | 77 064 | **5.1×** |
| 3 | 15 339 | 75 060 | **4.9×** |
| **total** | **44.4 KB** | **221.4 KB** | **5.0×** |

As três carregaram de verdade — verificado por conteúdo no snapshot, não por
ausência de erro. Uma página em branco snapshota minúsculo e faria a economia
parecer espetacular sem ser verdade.

## O fluxo Easy Apply — onde o custo real está

| Estado | Snapshot | Screenshot | Razão |
|---|---|---|---|
| descrição da vaga | 14 657 B | 74 556 B | **5.1×** |
| modal Easy Apply aberto | 46 246 B | 93 264 B | **2.0×** |

**Esta é a medição que importa, e contraria a expectativa.**

Ao abrir o formulário, o snapshot **triplica** (14.7 → 46.2 KB) porque a árvore de
acessibilidade ganha todos os campos, rótulos, opções e estados. O screenshot cresce
pouco (74.6 → 93.3 KB) porque um PNG de viewport é quase do mesmo tamanho tenha o que
tiver dentro.

Resultado: **no ponto em que o agente passa mais tempo, a vantagem cai para 2×.**

### Nota de segurança

O modal foi aberto e medido no **primeiro passo apenas**, e fechado com Escape sem
avançar nem submeter. Não percorri o formulário até ao fim: cada "Próximo" aproxima-se
de um botão de submissão, e uma candidatura acidental em nome do usuário é irreversível
e chega a um recrutador real. Confirmado depois: o browser saiu para `/jobs/`, nenhuma
candidatura enviada.

O plano pedia medir cada passo do formulário. **Não cumpri esse passo como escrito** —
a troca entre fidelidade da medição e risco de candidatura acidental não a justifica.
A medição do primeiro passo já mostra o efeito, que é o snapshot crescer com a
complexidade do formulário enquanto o screenshot não.

## Comparação com a baseline de 2026-09-16

| | Antes (CLA-5) | Medido agora |
|---|---|---|
| Imagens no contexto | 2.3 MB (17 screenshots) | — |
| Texto de página | 24 KB (3 chamadas) | — |
| % do contexto em imagens | **96%** | — |
| Razão por chamada | ~96× *(inferida do agregado)* | **2× a 5× (medida)** |

**A diferença entre 96× e 5× não é contradição — é a pergunta errada.**

Os 96% da CLA-5 não vêm de cada screenshot ser 96× mais caro que cada leitura de texto.
Vêm do **padrão de chamadas**: naquela sessão o agente fez 41 chamadas visuais
(`browser_batch` 23×, `computer` 18×) contra 3 de texto. Ele *escolheu* olhar em vez de
ler, quarenta e uma vezes.

Trocar a ferramenta sem trocar o padrão dá uma economia de 2× a 5×. Real, mas modesta.

## Resposta

**Entre 2× e 5×, conforme o que se mede — e 2× é o número do fluxo que importa.**

O argumento "snapshots são texto, logo muito mais baratos" **não se sustenta na forma
forte**. Um formulário complexo produz uma árvore de acessibilidade grande.

**O que continua de pé, e é o argumento real da rota:** com o app a conduzir, os passos
determinísticos deixam de precisar de percepção nenhuma. Preencher seis campos é
`fill()` seis vezes — zero bytes de contexto, nem texto nem pixels. O modelo é chamado
só onde há julgamento.

Essa economia não é de 2× nem de 5×: é de eliminar a chamada. Mas **não foi medida aqui**,
porque exige o app a conduzir, que é a Fase 2. Este protótipo mede a troca de
ferramenta, não a troca de arquitetura.

**Correção ao roadmap:** a secção que diz *"a percepção é por árvore de acessibilidade —
texto, não pixels — ataca de frente os 96% de screenshots da CLA-5"* está certa na
direção e otimista na magnitude. Ataca, mas rende 2× a 5×, não 96×. O resto da economia
depende de mudar quem decide o próximo clique.
