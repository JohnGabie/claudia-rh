# win-sandbox-lab — Objetivos de teste

Ambiente de teste descartável: build no host, execução num Windows Sandbox limpo,
controlado por IA (winapp CLI + Windows-MCP). Nada toca o sistema/banco reais.

## Layout

```
win-sandbox-lab/
├─ compose.json           ← ambientes nomeados (exemplo, app, app-winapp, cache-msvc, testes) com suas dependências e montagens
├─ OBJECTIVES.md          ← este arquivo
├─ scripts/
│  ├─ test-up.ps1         ← lê compose.json → gera sandbox/sandbox.generated.wsb → sobe o Sandbox
│  ├─ test-down.ps1       ← derruba o Sandbox e apaga a cópia da credencial do Claude
│  ├─ cache-up.ps1        ← baixa instaladores para shared/cache (só cria, nunca apaga)
│  ├─ app-stage.ps1       ← publica exe para os ambientes app e app-winapp
│  ├─ mcp-sync.ps1        ← atualiza o IP do guest no .mcp.json
│  └─ validate-config.ps1 ← valida compose.json sem subir o Sandbox (estático)
├─ sandbox/
│  ├─ sandbox.template.wsb
│  ├─ bootstrap.ps1       ← roda no LogonCommand dentro do Sandbox
│  ├─ bootstrap-mcp.ps1   ← sobe o Windows-MCP no guest
│  ├─ instalar.ps1        ← instala MSVC, Rust, Node, Claude Code no guest (ambiente testes)
│  ├─ layout-msvc.ps1     ← monta o layout offline do MSVC (ambiente cache-msvc)
│  └─ app/                ← scripts do guest para os ambientes
├─ shared/                ← mapeado no Sandbox em C:\lab (somente leitura, exceto out/)
│  ├─ extensions/lab-ext/ ← extensão MV3 de exemplo (marcador visível na página)
│  ├─ db/backup.sqlite    ← backup SQLite de exemplo (hash conhecido)
│  ├─ cache/              ← instaladores baixados uma vez (Rust, Node, Claude Code, MSVC bootstrapper)
│  ├─ tools/              ← Chrome for Testing portátil, Python/uv + windows-mcp pré-baixados
│  └─ out/                ← ÚNICA pasta gravável: logs, status.json, evidências do guest
├─ .mcp.json              ← Claude Code → Windows-MCP (HTTP) dentro do Sandbox
├─ .claude/skills/        ← skills do winapp (winapp-ui-automation etc.)
└─ evidence/              ← screenshots/relatórios gerados no host durante os testes
```

Dentro do Sandbox: `C:\lab` = `shared\` (read-only), `C:\lab\out` = `shared\out` (gravável),
`C:\labproject` = projeto montado (read-only). Disco Z do guest: `Z:\claudia-teste-sandbox\<cenario>` para dados de teste, `Z:\tmp` para temporário.

## Objetivos

| ID  | Objetivo | Critério de aprovação |
|-----|----------|-----------------------|
| O1  | Config estática válida | `validate-config.ps1` passa: JSON válido, todos os caminhos existem, `.wsb` gerado é XML válido, `.mcp.json` válido |
| O2  | Sobe com 1 comando | `scripts\test-up.ps1` sobe o Sandbox sem interação manual; `shared\out\status.json` aparece com `bootstrap: "ok"` |
| O3  | Projeto montado | `C:\labproject` existe no guest, read-only (escrita falha) |
| O4  | Banco restaurado | `C:\labdata\app.sqlite` no guest tem o mesmo SHA-256 do backup no boot e a consulta de verificação retorna as linhas esperadas; o `backup.sqlite` do host continua com o hash original no fim |
| O5  | Extensões carregadas | Chrome (for Testing) no guest abre com `lab-ext` carregada; a página de teste mostra o marcador da extensão |
| O6  | MCP acessível | Windows-MCP rodando no guest via HTTP com auth key; do host, listar tools funciona |
| O7  | Skills instaladas | `.claude/skills/` contém as skills do winapp com `SKILL.md` válido (frontmatter name/description) |
| O8  | Controle do Chrome pela IA | Navegar, digitar e clicar no Chrome do guest (winapp `--on sandbox` ou MCP); screenshot comprova |
| O9  | Validação independente | O2–O8 reconferidos com evidência nova (arquivos, hashes, screenshots), sem depender de relatórios anteriores |
| O10 | Isolamento / descarte | Após `test-down.ps1`, nenhum processo/arquivo do guest persiste; host intacto (sem Chrome/Python novos fora de `shared/tools`, banco original intacto) |
