# Spec de rota: o browser como superfície MCP

> **Spec de rota, não de implementação.** Ordena backlog existente para chegar a um objetivo. Não autoriza código: cada fase mantém seu próprio ciclo spec → plano → review.

> **Status: superado pela decisão CDP direto** (`docs/pesquisa/2026-09-28-DECISAO-browser.md`). As Fases 3 e 4 (CLA-27, CLA-30, CLA-8) continuam abertas no Linear.

**Data:** 2026-09-27
**Issues atravessadas:** CLA-29, CLA-7, CLA-20, CLA-28, CLA-5, CLA-27, CLA-30, CLA-8

---

## 1. Objetivo

**Tirar o controle do browser de dentro do agente e colocá-lo numa superfície MCP, para que o app deixe de depender do Claude Code e da sua extensão de Chrome.**

Formulado pelo usuário em 2026-09-25:

> *"a razão disso é desvincular a dependência do claude e sua extensão fortemente quebrada, add um mcp"*

Uma frase, e resolve ou desbloqueia oito issues. Este documento diz em que ordem.

## 2. Por que agora — o que foi medido em 2026-09-25

Quatro medições independentes que convergem no mesmo ponto.

**A extensão é o acoplamento.** MCP é o denominador comum dos três CLIs agênticos (Claude Code, Gemini CLI, Grok Build). Controle de browser não é — só o Claude tem. Enquanto o browser viver dentro do agente, o app está preso a um fornecedor por incapacidade, não por escolha.

**O custo está nos pixels.** Numa sessão de 172 turnos: 18.093.941 tokens de cache read contra 82.697 de output. Dessas, **17 screenshots ocupam 96% de tudo que entra no contexto**; todo o texto de página extraído soma 1%. O Playwright MCP percebe a página por árvore de acessibilidade — texto — e inverte essa proporção.

**A extensão falha e leva trabalho junto.** `pty_manager.rs:175` já tem código defensivo procurando `"Browser extension is not connected"` na saída. Os eventos de 2026-09-25 mostram `chrome_reconnect` três vezes em 30 ms seguido de `chrome_failed`.

**O agente já contornou a lacuna sozinho.** No workspace: 11 scripts e 63 pacotes npm que ninguém autorizou, incluindo um `html2pdf.js` com Puppeteer — a peça que a CLA-27 diz faltar — e um `db_write.js` que escreve no banco sem passar pelas tools.

## 3. O que está verificado e o que não está

Essa separação é o que impede a rota de virar wishful thinking.

### Verificado

| Fato | Fonte |
|---|---|
| Chrome ≥136 ignora `--remote-debugging-port` no perfil padrão; trava numa página em branco, sem erro | docs Chrome for Developers |
| A máquina do usuário roda **Chrome 153** | `chrome.exe` em disco |
| Copiar o perfil quebra a descriptografia de cookies (App-Bound Encryption) | issue #217 do projeto de pesquisa |
| `@playwright/mcp` existe, é oficial da Microsoft, e serve qualquer cliente MCP | `microsoft/playwright-mcp` |
| `--browser chrome` usa o Chrome instalado, sem baixar browsers | flags confirmadas |
| Percepção por árvore de acessibilidade é o modo padrão | docs Playwright MCP |
| Grok Build (2026-05) tem MCP nativo, headless `-p`, e lê `.mcp.json` / `claude_desktop_config.json` sem reconfiguração | x.ai |
| Gemini CLI suporta MCP stdio e modo não-interativo | docs google-gemini/gemini-cli |

### Por verificar — e cada um pode mudar a rota

1. **O perfil dedicado mantém a sessão do LinkedIn entre execuções?** Se não, o perfil persistente não serve e sobra o modo extensão — que é sair de uma extensão para entrar em outra.
2. **O modo extensão contorna a restrição do Chrome 136?** A documentação não afirma isso. Opera em outra camada, mas é suposição até alguém ver.
3. **Quanto cai o contexto, em números?** A promessa é grande; o valor é desconhecido.
4. **A dependência de `npx` é aceitável?** `@playwright/mcp` roda em Node. O app não distribui Node.

**Nenhuma fase depois da 1 deve começar antes dessas quatro terem resposta.**

## 4. As fases

### Fase 0 — Enxergar (CLA-29)

Voltar a gravar transcripts, limpando as variáveis `CLAUDE_CODE_*` herdadas no spawn.

**Por que é primeiro:** os transcripts pararam em 2026-09-16. Toda a medição dessa rota — o contexto antes, o contexto depois — sai desses arquivos. Sem eles, a Fase 1 produz uma impressão em vez de um número.

Já tem spec escrita (`2026-09-25-isolar-ambiente-sessao-design.md`) e é a mudança mais barata do backlog.

**Saída:** um `.jsonl` novo em disco depois de uma sessão.

---

### Fase 1 — Provar (CLA-7, protótipo descartável)

Declarar `@playwright/mcp` como servidor MCP adicional, ao lado do servidor `claudia` que o app já escreve em `commands/perfil.rs:447`. Uma sessão que abre o LinkedIn, autentica uma vez num perfil dedicado, e lê uma vaga.

**Responde às quatro perguntas em aberto da §3.** Nada além disso. O código é descartável e fica marcado como tal.

**Saída:** os quatro números/respostas, e uma recomendação de seguir ou parar.

**Ponto de parada honesto:** se a sessão não sobreviver ao reinício autenticada, e o modo extensão não contornar o Chrome 136, essa rota não tem caminho limpo. Melhor saber isso com um protótipo do que no meio da Fase 2.

---

### Fase 2 — Mover (CLA-7, implementação)

O browser passa a ser MCP. O agente continua dirigindo — o laço agêntico continua sendo do CLI, que é onde funciona — mas através de tools servidas pelo app, como `register_vaga` já é hoje.

Isso preserva o `prompt_sistema_runtime.md` e as 14 tools existentes, em vez de jogá-las fora.

**Cai por consequência, sem trabalho próprio:**

* **CLA-20** — a extensão que cai deixa de existir. O watcher de `pty_manager.rs:175` fica sem objeto.
* **CLA-28, em parte** — a linha 88 do prompt manda executar `/chrome` e escolher *"Reconnect extension"*, um menu interativo numa sessão sem ninguém para interagir. Some junto com a extensão.
* **CLA-5, em parte** — a percepção textual ataca os 96%. A divisão Opus/Haiku vira otimização, não necessidade.

**Saída:** uma candidatura completa sem a extensão do Claude, com o contexto medido contra a baseline da Fase 0.

---

### Fase 3 — Fechar as lacunas antes de fechar a porta (CLA-27, depois CLA-30)

**CLA-27 primeiro.** Expor `gerar_curriculo` e `gerar_cover_letter` como tools MCP. Hoje o agente gera PDFs próprios — três candidaturas produziram arquivos de 4.919, 71.029 e 135.985 bytes, 27× de variação — e faz upload deles no LinkedIn, porque não existe tool. Os templates do app têm cuidados de ATS que esses arquivos não têm.

Falta uma peça mesmo depois de expor: `gerar_curriculo` produz **HTML**, não PDF (`curriculos.rs:849`). O `html2pdf.js` que o agente escreveu mostra o que falta, e como.

**CLA-30 depois, nunca antes.** Os 11 scripts não são travessura — são um inventário de tools em falta. Confinar antes de fornecê-las garante que a sessão para de trabalhar.

Ordem dentro da CLA-30: ler os onze, perguntar de cada um *por que o agente precisou disso*, fechar essas lacunas, e só então restringir.

**Saída:** o agente consegue fazer o trabalho todo através de tools; nenhum script improvisado é necessário.

---

### Fase 4 — Desvincular (CLA-8)

Com o browser em MCP e as lacunas fechadas, trocar de agente vira um adaptador fino em vez de uma reescrita.

`claude_program()` (`commands/mod.rs:30`) já é o ponto único por onde passam os 8 call sites. Passa a resolver *qual agente*, e cada um traz suas flags.

**Grok Build primeiro, não Gemini** — pela compatibilidade declarada de config MCP com o Claude Code, que é menos trabalho de integração.

**O que continua sendo trabalho real:** o parsing de `--output-format stream-json` em `perfil.rs` e `cover_letter.rs` é específico do Claude Code. É aí que mora o adaptador, e é por isso que essa fase é a última.

**Saída:** uma candidatura completa conduzida por um agente que não é o Claude.

## 5. Mapa das dependências

```
Fase 0  CLA-29  ──┬─→ Fase 1  CLA-7 protótipo  ──→ Fase 2  CLA-7 impl.
                  │                                    │
                  └─→ (medição da CLA-5)               ├─→ CLA-20  some
                                                       ├─→ CLA-28  parcial
                                                       ├─→ CLA-5   parcial
                                                       │
                                                       └─→ Fase 3  CLA-27 ─→ CLA-30
                                                                              │
                                                                              └─→ Fase 4  CLA-8
```

## 6. Fora dessa rota

Issues ativas que **não** dependem disso e podem andar em paralelo, por quem estiver corrigindo bugs:

| | |
|---|---|
| **CLA-25** | a escolha de modo de busca não sai do React (`dashboard/index.tsx:229`) |
| **CLA-26** | faltam fontes de busca — bloqueada pela CLA-25, não por essa rota |
| **CLA-14, CLA-15, CLA-16, CLA-18** | contadores, alertas, feedbacks |
| **CLA-17** | pendências que fecham por abandono |
| **CLA-6** | módulo de e-mail |

**CLA-12 merece nota.** Está marcada `Urgent` por candidaturas duplicadas, atribuídas a retry sem idempotência em `register_candidatura`. Mas o `db_write.js` encontrado no workspace insere sem passar por tool nenhuma, e `candidaturas` não tem constraint. **A causa pode ser a CLA-30, não a que está escrita na issue.** Vale testar essa hipótese antes de construir a defesa.

## 7. Riscos

**O protótipo pode matar a rota.** É para isso que ele serve. Ver §3, pontos 1 e 2.

**Node em runtime.** Se `npx` não for aceitável para distribuição, a Fase 2 precisa de outra forma de servir o Playwright MCP. Não invalida o desenho — invalida a implementação mais fácil dele.

**Risco de conta, inalterado.** Automação deixa marcas observáveis; os portais restringem automação nos termos de uso; o risco recai na conta do usuário, que é o ativo mais difícil de repor nesse app. Com perfil dedicado, um bloqueio atinge esse perfil e não a sessão pessoal — é separação de risco, não eliminação. Continua sendo decisão consciente, não técnica.

**Sequência ignorada.** O erro mais provável dessa rota é fazer a CLA-30 antes da CLA-27, ou a CLA-8 antes da CLA-7. Os dois produzem trabalho que se joga fora.

## 8. Como saber que chegamos

```text
uma candidatura completa, submetida com:
  · sem a extensão do Claude in Chrome envolvida
  · CV gerado a partir dos templates do app, não improvisado
  · zero scripts escritos pelo agente no workspace
  · contexto por candidatura medido e comparado com 2026-09-16
  · e repetida com um agente que não é o Claude
```

O último ponto é o objetivo. Os quatro primeiros são o que torna ele possível.
