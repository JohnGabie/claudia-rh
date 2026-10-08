# sandbox-lab

Laboratório Windows Sandbox do claudia-rh: um Windows limpo e descartável para testar o app, controlado por scripts e por IA.

- **Como usar:** skill `claudia-teste-sandbox` (`.claude/skills/claudia-teste-sandbox/SKILL.md`). Referência completa do laboratório: `.claude/skills/claudia-teste-sandbox/references/laboratorio.md`.
- **Objetivos do laboratório genérico (O1–O10):** `OBJECTIVES.md`.
- **Onde fica:** sempre o `sandbox-lab/` do checkout principal. De uma worktree, a skill usa este laboratório e monta a worktree como projeto (`onde.mjs`).

## O que está no git e o que não está

| Versionado | Fora do git (ignorado) |
|---|---|
| `scripts/`, `sandbox/` (sem os gerados), `compose.json`, `sample-project/`, `shared/db/`, `shared/extensions/`, `shared/mcp.json`, `shared/test-page.html`, `shared/tools.sha256` | `shared/tools/` (~592 MB), `shared/cache/` (instaladores do guest), `shared/secrets/` (chave do MCP), `shared/out/`, `evidence/`, `sandbox/sandbox.generated.wsb`, `sandbox/guest.config.json`, `.mcp.json`, e `.sandbox-work/` na raiz do repositório |

## Ferramentas

`shared/tools/` (Chrome for Testing, Python portátil, `uv`, wheels do `windows-mcp`, `sqlite`) não está no git. A lista de arquivos com tamanho e SHA-256 está em `shared/tools.sha256`. Se faltar algo, o `test-up.ps1` para antes de subir o Sandbox e diz o quê. Para obter: copiar da pasta antiga do laboratório para `shared/tools/` (destino novo, sem `/MIR`) e conferir com `node .claude/skills/claudia-teste-sandbox/scripts/conferir-copia.mjs --manifesto sandbox-lab/shared/tools.sha256 --copia sandbox-lab/shared/tools`. Um script que baixa as ferramentas é trabalho futuro (Linear).

## Ferramentas do PC, só leitura (ambiente `testes`)

O ambiente `testes` não instala MSVC, Rust nem Node no guest (levava cerca de 3 h): o `test-up.ps1` acha na hora, no PC, a pasta de cada ferramenta declarada em `hostTools` no `compose.json` (por nome; nenhum caminho fica no arquivo) e a monta **só leitura** em `C:\hosttools\{rust,node,msvc,winkits}`: `rust` pelo `rustup which rustc` rodado na pasta do projeto (recusa toolchain sem libstd), `node` em `ProgramFiles\nodejs`, `msvc` no BuildTools do Visual Studio 2022 (ou `vswhere`), `winkits` em `ProgramFilesX86\Windows Kits\10`. Se algo faltar ou a pasta for recusada, o `test-up.ps1` para antes de subir. Para um "PC novo de verdade" use `-Ambiente testes-limpo`, que instala tudo no guest a partir do cache e leva cerca de 3 h.

## Cache de instaladores

`shared/cache/` guarda os instaladores dos pacotes que o guest instala (Claude Code no `testes`; Rust, Node, MSVC bootstrapper e layout só no `testes-limpo`), baixados uma vez pelo `scripts\cache-up.ps1` e nunca apagados. Tamanho aproximado por pacote:
- Rust 1.95.0: 304 MB
- Node 22.23.3: 36 MB
- Claude Code 2.1.285: 244 MB
- MSVC 17.14: **não verificado** (bootstrapper ~4.3 MB + layout ~?? GB)

Uso: `scripts\cache-up.ps1` (sem parâmetros) baixa os pacotes para `shared\cache\` (só cria; nunca apaga); `scripts\cache-up.ps1 -SoPlano` mostra o plano sem baixar. Se o `test-up.ps1` disser que o cache está incompleto, **pare e avise o usuário** para rodar `cache-up.ps1` primeiro.

## Regras

- **Nunca** automatize o desktop do host: winapp sempre com `--on sandbox`; MCP só o `windows-sandbox` (roda no guest).
- Só existe um Sandbox por vez: suba com `scripts\test-up.ps1`, derrube com `scripts\test-down.ps1`.
- `shared\` é read-only no guest (exceto `shared\out\`). Nunca escreva em `shared\db\backup.sqlite`.
- A chave do MCP fica em `shared\secrets\` e nunca entra no git. Depois de cada boot, `scripts\mcp-sync.ps1` atualiza o IP no `.mcp.json` (que usa `${LAB_MCP_AUTH_KEY}`, nunca a chave).
- Regras do repositório (`CLAUDE.md` da raiz) valem sobre este arquivo.
