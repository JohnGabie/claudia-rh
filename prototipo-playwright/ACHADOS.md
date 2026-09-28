# Achados do protótipo — CLA-7 Fase 1

**Data:** 2026-09-28
**Medições cruas:** `medicoes/`
**Ambiente:** Chrome 153.0.8010.54, Node v24.17.0, `@playwright/mcp` 1.64.0-alpha

## As quatro perguntas

| # | Pergunta | Resposta |
|---|---|---|
| **Q1** | O perfil dedicado mantém a sessão do LinkedIn entre execuções? | **Sim.** 3/3 páginas autenticadas após 11 min e reinício completo do processo |
| **Q2** | O modo extensão contorna a restrição do Chrome 136? | **Não foi preciso testar** — o perfil dedicado resolve, e evita instalar mais uma extensão |
| **Q3** | Quanto cai o contexto? | **Não cai — sobe 10× a 16×.** O roadmap estava errado no sinal |
| **Q4** | A dependência de `npx` é aceitável? | **Sim**, com a ressalva de que o app não distribui Node |

## Coexistência MCP

Os dois servidores carregam na mesma sessão: 14 tools do `claudia` e 25 do
`playwright`. O servidor do app continua funcional. Nenhum arquivo do app foi
modificado — a mudança necessária é acrescentar um bloco à config que ele já escreve.

## Recomendação

**Seguir para a Fase 2 — mas com a justificação trocada.**

A rota é tecnicamente viável: o caminho do perfil dedicado funciona, os servidores
coexistem, e o custo de `npx` é baixo. Nenhum dos obstáculos que podiam matar a ideia
sobreviveu ao teste.

**Mas o argumento principal não enfraqueceu — inverteu-se, e isso muda o que a Fase 2
tem de fazer.**

A primeira versão deste documento dizia que a troca rendia "2× a 5×". Estava errada no
sinal. Uma revisão de contexto fresco encontrou três faltas na medição; re-medi e
confirmei as três. Medido em **tokens**, com as páginas totalmente carregadas:

| Vaga | Snapshot | Screenshot | Razão |
|---|---|---|---|
| 1 | ≈ **13 134 tok** | **1 067 tok** | **12.3× pior** |
| 2 | ≈ **14 884 tok** | **1 067 tok** | **13.9× pior** |
| 3 | ≈ **14 651 tok** | **1 067 tok** | **13.7× pior** |

Aguenta em todo o intervalo plausível: de **9.6×** a **16.3×**. Não há corner onde o
snapshot ganhe.

**A assimetria é estrutural.** Uma imagem custa pela área — `(929×861)/750 ≈ 1 067
tokens`, teto fixo. Um snapshot custa pela complexidade, e **93% do dele é boilerplate**
(214 de 231 linhas idênticas entre vagas diferentes), pago outra vez em cada chamada.

**Consequência dura: trocar a ferramenta e manter o padrão piora o contexto dez vezes.**
A Fase 2 só compensa se o app passar a conduzir — e isso deixa de ser detalhe de
implementação para ser a condição de a migração valer a pena.

### O que eu afirmei sobre a CLA-5 e retiro

A primeira versão dizia que os 96% vinham do padrão de chamadas, não da razão por
chamada. **Retiro** — essa reconciliação usava 41 chamadas num sítio e 17 noutro, e
explicava parte da diferença como se explicasse toda.

Fica no lugar uma dúvida legítima: 17 screenshots de viewport custam ~18 000 tokens,
não 96% de uma sessão de 18 milhões. **A baseline da CLA-5 precisa de ser re-derivada**
antes de alguém se apoiar nela — está em bytes de transcript, não em tokens faturados.
Este protótipo não a auditou.

### O que continua a justificar a Fase 2

1. **Remove a dependência da extensão.** É a razão original do usuário — *"desvincular
   a dependência do claude e sua extensão fortemente quebrada"*. O `pty_manager.rs:175`
   já tem código defensivo a vigiar `"Browser extension is not connected"`, e os
   eventos de 25/09 mostram três tentativas de reconexão em 30 ms seguidas de
   desistência. Isto resolve a **CLA-20** por remoção, não por correção.

2. **Desbloqueia o multi-IA.** MCP é o denominador comum dos três CLIs; controlo de
   browser não é. Enquanto o browser viver dentro do agente, o app está preso ao Claude
   por incapacidade, não por escolha. É a **CLA-8**, e era o segundo pedido do usuário.

3. **A economia só existe se o app conduzir — e agora isso é obrigatório, não
   desejável.** Com o app a conduzir, preencher seis campos é `fill()` seis vezes:
   **zero** tokens, nem texto nem píxeis. A economia não vem de a percepção ser mais
   barata (não é, é 10× mais cara); vem de **deixar de haver percepção** nos passos
   determinísticos. **Não foi medida aqui** — exige a Fase 2.

4. **Resolve a CLA-27 por desaparecimento.** Se o app conduz o browser, o upload passa
   a ser o app a enviar o arquivo que o app gerou. Os PDFs improvisados deixam de
   existir em vez de serem corrigidos.

### O custo, dito por inteiro

- **Login uma vez por plataforma**, num browser que é do app e não do dia-a-dia.
- **Node em runtime.** O app não o distribui hoje. Exigi-lo na máquina do usuário ou
  empacotá-lo é uma decisão da Fase 2 e não é gratuita.
- **Os browsers do Playwright.** Esta máquina já tem 1.6 GB em `ms-playwright`, de
  meses antes — do plugin que o usuário já tinha. Numa máquina limpa, `--browser chrome`
  deve usar o Chrome instalado sem baixar nada, mas **isso não foi provado aqui**:
  todas as medições correram numa máquina que já os tinha.

## O que muda no roadmap

**§3, "Por verificar"** — as quatro perguntas têm resposta. Q1 sim, Q2 dispensada,
**Q3 negativa (10×-16× pior)**, Q4 sim com ressalva.

**§3, os dois caminhos dados como mortos** — continuam mortos, e a afirmação está
correta. Mas faltava uma terceira porta: o **perfil dedicado**, que não é nenhum dos
dois e está aberta.

**A frase sobre o contexto** — *"a percepção é por árvore de acessibilidade — texto,
não pixels — ataca de frente os 96% de screenshots da CLA-5"* — **está errada, não
otimista.** Medido, a percepção textual custa dez a dezasseis vezes mais que a visual
nestas páginas. Esta linha tem de ser reescrita antes de alguém a citar.

**Fase 0 (CLA-29) não é pré-requisito.** O roadmap punha-a primeiro assumindo que a
medição sairia de transcripts de sessões novas. Não saiu — este protótipo mediu bytes
diretamente. A CLA-29 continua a valer por si (auditoria), mas não bloqueia. A relação
no Linear já foi corrigida de `blocks` para `related`.

## Achado colateral, para a CLA-30

O teste de coexistência bateu numa negação de permissão, e a saída revelou o mecanismo
que a CLA-30 procura: `--allowedTools "mcp__claudia__get_candidate_profile"` libertou
exatamente uma tool e nada mais.

Hoje o app passa `--dangerously-skip-permissions`, que liberta tudo — e foi isso que
permitiu à sessão escrever 11 scripts e instalar 63 pacotes npm. **Confinar não precisa
de infraestrutura nova.** Precisa de trocar uma flag que liberta tudo por uma lista do
que é preciso.

## O que não foi feito, e porquê

**O fluxo Easy Apply — nem o primeiro passo.** A primeira tentativa julgou ter aberto o
modal e não abriu: clicou num badge de card recomendado, porque "Candidatura
simplificada" aparece **181 vezes** na página. Na re-medição, com asserção de que um nó
`dialog` apareceu, o modal continuou a não abrir — e **parei em vez de tentar mais refs
às cegas.** Cada clique errado numa página de vaga é um caminho possível para uma
candidatura acidental em nome do usuário, que é irreversível e chega a um recrutador
real.

A conclusão da Q3 não depende disto: um formulário só acrescenta nós à árvore enquanto
o screenshot mantém o teto fixo, portanto a razão só pioraria.

**Nada foi submetido** — verificado depois: o browser saiu para `/jobs/`.

**O teste da restrição do Chrome 136 no perfil padrão.** Ficou inconclusivo: havia 26
processos Chrome a correr, e um segundo lançamento reutiliza a instância e descarta as
flags. O teste limpo exigia fechar o browser do usuário, com as sessões e o trabalho
dele abertos. Não fechei. E não importa para a rota, que não usa o perfil padrão.
