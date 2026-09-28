# Emulação de busca de vagas — somente monitorização

Replica a sessão que a app lança, com o agente **proibido de se candidatar**. Serve
para ver como ele descobre, lê e avalia vagas, e — o mais útil — **que perguntas de
formulário o perfil não sabe responder**.

## Arquivos

| Arquivo | O que é |
|---|---|
| `prompt-monitor.md` | o system prompt, variante somente-leitura do de produção |
| `mcp-monitor.json` | config MCP: só o servidor `claudia` |
| `mcp-monitor-playwright.json` | config MCP: `claudia` + `playwright` |
| `monitor-chrome.cmd` | corre via extensão Claude in Chrome (caminho atual da app) |
| `monitor-playwright.cmd` | corre via Playwright MCP, perfil próprio |

## Como correr

Duplo clique no `.cmd`, ou no terminal:

```cmd
prototipo-playwright\emulacao\monitor-chrome.cmd
```

A variante Playwright abre um Chrome com perfil vazio em `perfil-monitor/` e pede
login na primeira vez. **O login é feito por ti, à mão.** Nenhum arquivo aqui pede,
lê ou guarda credenciais.

## O que difere da sessão real da app

| | App | Monitor |
|---|---|---|
| System prompt | `prompt_sistema_runtime.md` | `prompt-monitor.md` |
| `--dangerously-skip-permissions` | **sim** | **não** — cada ação pede confirmação |
| Candidaturas | submete | **nunca** |
| Gera CV e carta | sim | não |
| `register_candidatura` | chama | proibido |
| `update_profile` | pode (sessão de Perfil) | bloqueado pelo gate |
| Limite | orçamento diário | 5 vagas, 15 min |

## Três camadas impedem candidatura

1. **O prompt** diz, como regra que vence todas as outras, para não se candidatar.
2. **As permissões não estão desligadas** — nenhum clique acontece sem tu aprovares.
3. **`--session-kind autonomous`** faz o servidor MCP recusar `update_profile`.
   Verificado: devolve *"O perfil só pode ser alterado numa sessão de Perfil."*

A camada 2 é a que conta. As outras duas são instrução ao modelo, e este projeto já
aprendeu — no incidente de 25/09 da CLA-11 — que instrução no prompt não é garantia.

Se quiseres deixar correr sem confirmar cada passo, acrescenta
`--dangerously-skip-permissions` ao `.cmd`. Aí ficas só com as camadas 1 e 3, e a
regra "não se candidatar" passa a depender do modelo obedecer.

## Verificado a 2026-09-28

```
servidor MCP responde        get_search_variants -> python-backend, peso 71.5
gate de sessão bloqueia      update_profile -> recusado com a mensagem certa
14 tools carregam            via claude --mcp-config --strict-mcp-config
```

Não corri uma sessão de LinkedIn completa: isso abre browser e navega em nome do
utilizador, e é para ele ver acontecer.

## Não confundir perfis

```
prototipo-playwright/perfil-claudia/   <- Tarefa 2 da CLA-7. NÃO TOCAR.
prototipo-playwright/emulacao/perfil-monitor/  <- este. Descartável.
```

O primeiro está a ser usado por uma medição de persistência de sessão em curso.
Abri-lo por fora estraga a medição.
