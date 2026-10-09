# Pesquisa: quem já resolveu o problema do browser

**Data:** 2026-09-28
**Origem:** *"a gente também não pensou se alguém já tentou resolver esse problema"*
**Estado:** levantamento; nenhum candidato foi instalado nem testado

> Devia ter feito isto antes de medir. Passei o dia a descobrir problemas que
> projetos existentes já tratam — em particular o endereçamento de elementos, que
> me derrotou quatro vezes.

## Os dois que importam

### `mcp-chrome` — usa o teu browser, com os teus logins

[hangwin/mcp-chrome](https://github.com/hangwin/mcp-chrome) · extensão Chrome + servidor MCP

Ataca as **duas razões** que motivaram tudo isto:

| A razão | Como resolve |
|---|---|
| *"dependência do Claude; trocar conta obriga a mexer no plugin"* | é uma extensão própria, independente de qualquer conta de IA |
| *"não posso usar outras IAs"* | **model-agnostic** — qualquer cliente MCP |

E resolve de passagem um problema que o Playwright me deu: usa **o browser que já está
aberto, com as sessões autenticadas do usuário**. A documentação do projeto contrasta
isto explicitamente com o Playwright, que *"requires re-login"*.

Isso elimina o perfil dedicado — e com ele a preocupação de um browser que só faz
automação ser mais fácil de sinalizar que o browser real de uma pessoa.

**20+ ferramentas**, incluindo screenshots por elemento (o que medi a 390 tokens),
extração de conteúdo, busca semântica e preenchimento de formulários.

**Ressalvas:** o próprio projeto diz estar *"in its early stages and under intensive
development"*. E não diz nada sobre deteção de bots — não é um problema que resolva,
é um que não aborda.

### `agent-browser` — resolve o endereçamento que me derrotou

[vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser) · CLI para agentes

**Screenshots anotados:** sobrepõe números `[N]` aos elementos interativos, cada um
ligado a uma referência `@eN`. O modelo vê a imagem com os números e diz *"clica no
@e3"*.

Isto dissolve as quatro estratégias que falharam no meu teste:

| A minha tentativa | Porque falhou | O que o anotado faz |
|---|---|---|
| `ref` da árvore | `browser_click` não reage no modal | o ref vem do próprio sistema |
| `aria-label` | vazio no LinkedIn | não depende de rótulo |
| texto do rótulo | não localiza | não depende de texto |
| índice | funciona, mas parte se reordenarem | o número está **na imagem**, é o que se vê |
| coordenada X,Y | funciona, exige `run_code_unsafe` (RCE) | sem código arbitrário |

A documentação diz que serve precisamente para *"botões de ícone sem rótulo, elementos
canvas, e layouts complexos onde o texto não chega"* — a descrição exata do que
encontrei.

Mais: as referências **persistem entre interações**, sem re-consultar o DOM. E tem
`--if-changed` e `--threshold` para não reenviar screenshots que não mudaram — o
problema de contexto que medi, já tratado.

Reutiliza o perfil do Chrome do usuário em **snapshot read-only**, sem tocar no original.

Funciona com qualquer agente que corra um CLI e leia JSON — não é específico de nenhuma IA.

## Outros, menos diretos

- [OpenChrome](https://github.com/shaun0927/openchrome) — servidor MCP para Chrome real,
  a partir de Claude Code, Codex CLI, OpenCode ou qualquer cliente MCP
- [BrowserMCP](https://github.com/browsermcp/mcp) — MCP + extensão, para VS Code, Claude,
  Cursor, Windsurf
- [browser-control](https://github.com/echelon-agents-ai/browser-control) — MCP + extensão,
  com *"vision-first control, multi-agent tabs, vault-filled logins e **human handoff**"*.
  O human handoff mapeia sobre as regras de pausa que o claudia-rh já tem.

## Prior art no domínio inteiro

Existem projetos que fazem o que o claudia-rh faz, ponta a ponta. Não são componentes a
usar, mas é útil saber que existem:

- [ApplyPilot](https://github.com/Pickle-Pixel/ApplyPilot) — pipeline de 6 fases,
  descoberta em 5+ boards, pontuação contra o currículo, adaptação por vaga, submissão
- [AI Job Agent](https://github.com/AkbarDevop/ai-job-agent) — LinkedIn, Greenhouse,
  Lever, Jobvite, Ashby; construído dentro do Claude Code
- [Auto Job Applier LinkedIn](https://github.com/GodsScion/Auto_job_applier_linkedIn) —
  corre inteiramente local, com a conta do próprio usuário

## O que isto muda

O trabalho do dia mediu bem o problema e chegou tarde à pergunta certa.

**O que a medição deu, e continua válido:**
- o servidor MCP do claudia-rh é consumido por outra IA sem alteração nenhuma (Grok, provado)
- um recorte de ecrã custa 390–777 tokens e é legível
- um registo extraído custa ~480 tokens
- o LinkedIn custa 1 433 tok/vaga contra 140 do Greenhouse
- navegar, ver e abrir o formulário não dispara defesa nenhuma

**O que deixa de ser preciso construir:** o endereçamento de elementos, o rato por
coordenadas, a gestão de perfil, e provavelmente o `click_at(x,y)` que eu ia recomendar.

## Recomendação

**Avaliar `mcp-chrome` primeiro**, porque ataca as duas razões originais de uma vez e
elimina o perfil dedicado. Uma tarde chega para saber: instalar, apontar o Grok e o
Claude ao mesmo servidor, e ver se conduz o LinkedIn autenticado.

Se o endereçamento no modal do Easy Apply voltar a falhar — é onde tudo emperrou —
então `agent-browser` entra, porque é o único candidato que trata disso de frente.

**Não construir nada antes disso.** O que eu ia recomendar construir já existe, em duas
implementações, e uma delas é da Vercel.

## O que não foi feito

Nenhum destes projetos foi instalado, corrido ou testado. Isto é leitura de
documentação e READMEs. Maturidade, manutenção real e comportamento no LinkedIn estão
por verificar — e o `mcp-chrome` avisa, ele próprio, que está em fase inicial.
