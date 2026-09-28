# Achados do protótipo — CLA-7 Fase 1

**Data:** 2026-09-28
**Medições cruas:** `medicoes/`
**Ambiente:** Chrome 153.0.8010.54, Node v24.17.0, `@playwright/mcp` 1.64.0-alpha

## As quatro perguntas

| # | Pergunta | Resposta |
|---|---|---|
| **Q1** | O perfil dedicado mantém a sessão do LinkedIn entre execuções? | **Sim.** 3/3 páginas autenticadas após 11 min e reinício completo do processo |
| **Q2** | O modo extensão contorna a restrição do Chrome 136? | **Não foi preciso testar** — o perfil dedicado resolve, e evita instalar mais uma extensão |
| **Q3** | Quanto cai o contexto? | **2× a 5×** — não os ~96× que o roadmap sugeria |
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

**Mas o argumento principal enfraqueceu, e isso tem de ficar dito.** O roadmap vendia a
migração como um corte drástico no contexto, apoiado nos 96% de screenshots da CLA-5.
Medido, o ganho de trocar screenshot por snapshot é **2× no modal do formulário**, que
é onde o agente passa mais tempo. Real, mas modesto.

Os 96% da CLA-5 nunca foram uma razão por chamada — vieram do **padrão**: 41 chamadas
visuais contra 3 de texto, naquela sessão. O agente escolheu olhar em vez de ler,
quarenta e uma vezes. Trocar a ferramenta sem mudar esse padrão não resolve isso.

### O que continua a justificar a Fase 2

1. **Remove a dependência da extensão.** É a razão original do usuário — *"desvincular
   a dependência do claude e sua extensão fortemente quebrada"*. O `pty_manager.rs:175`
   já tem código defensivo a vigiar `"Browser extension is not connected"`, e os
   eventos de 25/09 mostram três tentativas de reconexão em 30 ms seguidas de
   desistência. Isto resolve a **CLA-20** por remoção, não por correção.

2. **Desbloqueia o multi-IA.** MCP é o denominador comum dos três CLIs; controlo de
   browser não é. Enquanto o browser viver dentro do agente, o app está preso ao Claude
   por incapacidade, não por escolha. É a **CLA-8**, e era o segundo pedido do usuário.

3. **A economia real vem da arquitetura, não da ferramenta.** Com o app a conduzir,
   preencher seis campos é `fill()` seis vezes — zero bytes de contexto, nem texto nem
   pixels. O modelo é chamado só onde há julgamento. **Essa economia não foi medida
   aqui** porque exige a Fase 2; este protótipo mediu a troca de ferramenta.

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
Q3 2×-5×, Q4 sim com ressalva.

**§3, os dois caminhos dados como mortos** — continuam mortos, e a afirmação está
correta. Mas faltava uma terceira porta: o **perfil dedicado**, que não é nenhum dos
dois e está aberta.

**A frase sobre o contexto** — *"ataca de frente os 96% de screenshots da CLA-5"* está
certa na direção e otimista na magnitude. Ataca, e rende 2× a 5×. O resto depende de
mudar quem decide o próximo clique.

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

**O fluxo Easy Apply completo.** O plano pedia medir cada passo do formulário. Medi o
primeiro e fechei com Escape. Cada "Próximo" aproxima-se de um botão de submissão, e
uma candidatura acidental em nome do usuário é irreversível e chega a um recrutador
real. Confirmado depois que nada foi submetido.

**O teste da restrição do Chrome 136 no perfil padrão.** Ficou inconclusivo: havia 26
processos Chrome a correr, e um segundo lançamento reutiliza a instância e descarta as
flags. O teste limpo exigia fechar o browser do usuário, com as sessões e o trabalho
dele abertos. Não fechei. E não importa para a rota, que não usa o perfil padrão.
