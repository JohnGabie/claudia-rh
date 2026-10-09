# Pesquisa: portabilidade entre agentes CLI

**Data:** 2026-09-28
**Motivo:** [CLA-8](https://linear.app/claudia-rh/issue/CLA-8) — desvincular o app do Claude Code
**Estado:** verificado por teste na máquina do usuário, não só por documentação

> Este documento é **durável**. Não faz parte do protótipo descartável em
> `prototipo-playwright/`. As conclusões aqui sobrevivem ao estudo da CLA-7.

## Por que isto importa

Duas razões do usuário, nas palavras dele:

> *"Ele cria uma dependência muito grande do Claude. Toda vez que eu tenho mais de uma
> conta e preciso trocar, tenho que trocar no plugin. Isso desestabiliza na hora de
> trocar se o token estoura."*

> *"Eu não posso usar outras IAs, que foi uma das reclamações que mais recebi quando
> lancei a Claudia-rh."*

A segunda é **feedback de usuários reais do projeto open source**, não dívida técnica
interna. Isso muda a natureza da prioridade.

## O teste que fecha a questão

Feito a 2026-09-28, com o Grok que já estava instalado na máquina:

```
$ grok mcp add teste-claudia claudia-rh.exe -- --mcp-serve --data-dir ... --session-kind autonomous
$ grok mcp doctor

teste-claudia (stdio)
  ✓ command found
  ✓ server started (0.2s)
  ✓ handshake OK (protocol 2025-11-25)
  ✓ 14 tools discovered

$ grok -p "Chame get_candidate_profile e diga apenas o primeiro nome"
  → John
```

**O servidor MCP do app não precisou de mudança nenhuma para ser consumido por outra
IA.** A configuração de teste foi removida a seguir.

## O que está instalado nesta máquina

| CLI | Estado | Notas |
|---|---|---|
| `claude` | 2.1.220 | |
| `grok` | **1.0.41** | MCP, skills, plugins, `-p` headless, `--output-format`, `agent` |
| `opencode` | não instalado | |
| `gemini` | não instalado | |

O Grok já tinha um `chrome-devtools-mcp` configurado, com `--screenshot-format=jpeg
--screenshot-quality=60 --screenshot-max-width=1280 --screenshot-max-height=768`.
É uma **terceira via de controlo de browser**, ao lado do Playwright MCP e da extensão
do Claude.

## Superfície de flags: o que é portável e o que não é

Extraído de `commands/sessao.rs`, `perfil.rs` e `linkedin.rs`:

| Portável (cliente MCP qualquer) | Só Claude Code |
|---|---|
| `--mcp-config` | **`--chrome`** |
| `--mcp-serve`, `--data-dir` | `--dangerously-skip-permissions` |
| `--notify-port`, `--session-kind` | `--system-prompt-file` |
| | `--output-format stream-json`, `--print`, `--verbose`, `--include-partial-messages` |

As da direita, exceto a primeira, têm equivalente direto:

| | Claude Code | Grok Build | Gemini CLI |
|---|---|---|---|
| headless | `--print` | `-p` / `--single` | `-p` / `--prompt` |
| formato de saída | `--output-format` | `--output-format` | `--output-format` |
| prompt de ficheiro | `--system-prompt-file` | `--prompt-file` | `-p` |
| MCP | `--mcp-config` (por invocação) | `grok mcp add` (persistente) | `~/.gemini/settings.json` |
| skills | `.claude/skills/` | sim | — |

**A diferença que exige trabalho real** é como a config MCP chega: o Claude aceita um
ficheiro por invocação, o Grok e o Gemini leem de config persistente. O app escreve
config em runtime (`commands/perfil.rs:447`) e passaria a escrever no formato de cada
agente.

## Três afirmações minhas que a pesquisa derrubou

Registadas porque foram ditas com confiança e estavam erradas.

**"Skills são um conceito do Claude Code, logo o prompt não é portável."**
Falso. `SKILL.md` é markdown com frontmatter YAML — formato aberto, não proprietário.
O [OpenCode lê `.claude/skills/` diretamente](https://github.com/ivan-gerasin/claude2opencode),
sem conversão, e o [Grok Build suporta skills out of the box](https://x.ai/news/grok-build-cli).
Isto **retira a CLA-24 do caminho crítico** do multi-IA.

**"O Gemini pode não ter CLI para PC."**
Tem. O [Gemini CLI é open source](https://github.com/google-gemini/gemini-cli), corre
em Windows via `gemini.cmd`, configura MCP em `~/.gemini/settings.json`, e tem headless
`-p` com `--output-format json`.

**"A abstração do CLI é trabalho significativo."**
Menor do que eu disse. As interfaces são quase iguais — ver a tabela acima.

## O único bloqueio real

**`--chrome`.** É a única flag sem equivalente noutro agente, porque o browser vive
*dentro* do agente Claude.

É por isso que a [CLA-7](https://linear.app/claudia-rh/issue/CLA-7) é o portão da CLA-8
— **não pelo argumento de contexto**, que a medição de 2026-09-28 derrubou, mas porque
enquanto o browser estiver dentro do agente, trocar de agente significa perder o browser.

E é exatamente a queixa das contas: o plugin está preso a uma conta Claude; o browser
está preso ao plugin.

## Candidatos a controlo de browser

| | Estado |
|---|---|
| **Extensão Claude in Chrome** | atual; boa a navegar, presa ao Claude, percepção por screenshot |
| **Playwright MCP** | medido na CLA-7: perfil dedicado persiste sessão, coexiste com o servidor do app |
| **chrome-devtools-mcp** | **já instalado no Grok do usuário**; por avaliar |

---

*Fontes: [Grok Build](https://x.ai/news/grok-build-cli) · [Gemini CLI](https://github.com/google-gemini/gemini-cli) · [OpenCode](https://opencode.ai/) · [claude2opencode](https://github.com/ivan-gerasin/claude2opencode) · [Docs MCP do Gemini CLI](https://geminicli.com/docs/tools/mcp-server/)*
