# win-sandbox-lab

Ambiente de teste descartavel no Windows Sandbox, controlado por IA (winapp CLI + Windows-MCP).

- Objetivos, layout e criterios de aprovacao: **`OBJECTIVES.md`** - leia primeiro.
- Como operar o lab (test-up/test-down, MCP, winapp, evidencias, banco): skill **`.claude/skills/sandbox-lab/SKILL.md`**.
- Skills do winapp: `.claude/skills/winapp-sandbox`, `.claude/skills/winapp-ui-automation`.

Regras:
- **Nunca** automatize o desktop do host: winapp sempre com `--on sandbox`; MCP so o `windows-sandbox` (roda no guest).
- So existe um Sandbox por vez: suba com `scripts\test-up.ps1`, derrube com `scripts\test-down.ps1`.
- `shared\` e read-only no guest (exceto `shared\out\`). Nunca escreva em `shared\db\backup.sqlite`.
- Segredos em `shared\secrets\` (gitignored). Apos cada boot rode `scripts\mcp-sync.ps1` para atualizar o IP no `.mcp.json`.
