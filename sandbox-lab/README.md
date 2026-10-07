# sandbox-lab

Laboratório Windows Sandbox do claudia-rh: um Windows limpo e descartável para testar o app, controlado por scripts e por IA.

- **Como usar:** skill `claudia-teste-sandbox` (`.claude/skills/claudia-teste-sandbox/SKILL.md`). Referência completa do laboratório: `.claude/skills/claudia-teste-sandbox/references/laboratorio.md`.
- **Objetivos do laboratório genérico (O1–O10):** `OBJECTIVES.md`.
- **Onde fica:** sempre o `sandbox-lab/` do checkout principal. De uma worktree, a skill usa este laboratório e monta a worktree como projeto (`onde.mjs`).

## O que está no git e o que não está

| Versionado | Fora do git (ignorado) |
|---|---|
| `scripts/`, `sandbox/` (sem os gerados), `lab.config*.json`, `sample-project/`, `shared/db/`, `shared/extensions/`, `shared/mcp.json`, `shared/test-page.html`, `shared/tools.sha256` | `shared/tools/` (~592 MB), `shared/secrets/` (chave do MCP), `shared/out/`, `evidence/`, `sandbox/sandbox.generated.wsb`, `sandbox/guest.config.json`, `.mcp.json`, e `.sandbox-work/` na raiz do repositório |

## Ferramentas

`shared/tools/` (Chrome for Testing, Python portátil, `uv`, wheels do `windows-mcp`, `sqlite`) não está no git. A lista de arquivos com tamanho e SHA-256 está em `shared/tools.sha256`. Se faltar algo, o `test-up.ps1` para antes de subir o Sandbox e diz o quê. Para obter: copiar da pasta antiga do laboratório para `shared/tools/` (destino novo, sem `/MIR`) e conferir com `node .claude/skills/claudia-teste-sandbox/scripts/conferir-copia.mjs --manifesto sandbox-lab/shared/tools.sha256 --copia sandbox-lab/shared/tools`. Um script que baixa as ferramentas é trabalho futuro (Linear).

## Regras

- **Nunca** automatize o desktop do host: winapp sempre com `--on sandbox`; MCP só o `windows-sandbox` (roda no guest).
- Só existe um Sandbox por vez: suba com `scripts\test-up.ps1`, derrube com `scripts\test-down.ps1`.
- `shared\` é read-only no guest (exceto `shared\out\`). Nunca escreva em `shared\db\backup.sqlite`.
- A chave do MCP fica em `shared\secrets\` e nunca entra no git. Depois de cada boot, `scripts\mcp-sync.ps1` atualiza o IP no `.mcp.json` (que usa `${LAB_MCP_AUTH_KEY}`, nunca a chave).
- Regras do repositório (`CLAUDE.md` da raiz) valem sobre este arquivo.
