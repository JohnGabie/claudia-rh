---
name: claudia-teste-e2e
description: >-
  Olha o Claudia RH (app Tauri) RODANDO, local ou no Windows Sandbox (CDP_HOST), por CDP: ver a tela, clicar, chamar comandos do backend e ler o que os agentes fazem.
  Use SEMPRE que o usuário falar de algo que se vê ou se confere no app aberto: testar ou conferir uma tela (Dashboard, Perfil, Histórico), comparar a UI com o banco, queixa de layout ou aparência (não gostei do layout, tá estranho, olha o que tá errado nessa tela, mexi no CSS), perguntar se o agente ou a sessão se comportou como devia, comparar agentes (Claude vs Grok), ou chamar um comando Tauri.
  QUALQUER bug do app, inclusive pedido curto como "corrige este bug", "o bug tá acontecendo de novo" ou "tem um bug no X": carregue esta skill ANTES de abrir ou editar código, porque o primeiro passo é reproduzir no app rodando.
  Use também antes de concluir qualquer mudança em UI ou em comandos Tauri.
  NÃO use para cargo test puro, sites de vagas, Linear, nem para montar Windows limpo, escolher dados ou instalar o app no Sandbox (claudia-teste-sandbox).
---

# claudia-teste-e2e

Dá a quem desenvolve o olhar de um dev sobre o app rodando. **Nunca interage com sites de vagas** e nunca clica em algo que submeta candidatura: quem interage com sites são os agentes dentro da Claudia.

Todos os scripts ficam em `.claude/skills/claudia-teste-e2e/scripts/` e rodam com Node ≥ 22.13, sem instalar nada.

## Regras duras (valem antes de tudo)

Origem: incidente de 2026-10-05. Detalhes, decisões B1 a B7 e tabela R1 a R12: `docs/superpowers/evidencias/2026-10-06-auditoria-delecao.md`.

- Teste que apaga qualquer arquivo roda **só no Windows Sandbox** (`claudia-teste-sandbox`). Nenhum caminho real em teste (raiz de disco, repositório, `AppData`, sistema).
- **Nunca o Chrome pessoal do usuário**; o Chrome do Claudia-RH pode (inclusive no smoke MCP, R11).
- `node --test` **sempre com a lista de arquivos explícita**. Não rodar `evals/rodar-evals.mjs` até a CLA-56 ser resolvida.
- **Até a Tarefa 0b do plano `docs/superpowers/plans/2026-10-06-fechar-branch-browser.md` entrar** (o código ainda não foi corrigido):
  - antes de `subir.mjs`, a **automação (disparo por ociosidade e modo autônomo) tem de estar desligada pelo usuário na tela** (B4; depois o app de teste subirá sem automação por variável de dev). Se o app instalado estiver aberto, **pare e pergunte** (R12);
  - o `subir.mjs` já passa `CLAUDIA_SEM_AUTOMACAO=1` ao `tauri dev` e informa `"automacao":"desligada nesta execução"`; **o efeito depende do app suportar CLAUDIA_SEM_AUTOMACAO (Tarefa 0b, frente c)**. Se não conseguir listar os processos, `subir.mjs` devolve `perguntar` ("não consegui verificar se o app instalado está aberto");
  - `invocar.mjs` (já aplicado no script): o argumento JSON é validado com `JSON.parse`. Comando fora do prefixo de leitura exige `--escrever`. Os prefixos `apagar_`, `excluir_`, `remover_`, `limpar_`, `desinstalar`, `instalar_`, `iniciar_`, `disparar_`, `enviar_`, `guardar_`, `configurar_`, `escrever_`, `resolver_`, `marcar_`, `renomear_`, `importar_`, `exportar_`, `abrir_`, `gerar_`, `sugerir_` e os comandos `browser_abrir`/`browser_retry` exigem também `--escrever --confirmo=<mesmo comando>`, só com autorização do usuário. **Sempre recusados:** `obter_credencial` (devolve senha) e qualquer `plugin:*` (ex.: `plugin:opener|open_url` abre o navegador pessoal) (B5);
  - `abrir_*` com argumento que seja URL, UNC (`\\`), `mailto:` ou outro esquema é sempre recusado, mesmo confirmado (abriria o navegador pessoal);
  - `subir.mjs` só reutiliza um app de dev já aberto se ele próprio o iniciou (estado em `debug-logs/subir-estado.json`, PID ainda vivo); senão devolve `perguntar` ("app de dev já aberto sem garantia de automação desligada");
  - `ver.mjs tela` recusa se `debug-logs` ou `debug-logs/prints` for link/junction ou se o caminho real sair da raiz do repo;
  - `ver.mjs tela <arq>` (já aplicado): só grava `.png` novo dentro de `debug-logs/prints/` (ex.: `debug-logs/prints/tela.png`), sem `..`, sem caminho absoluto, e recusa arquivo existente; sem argumento gera `tela-<timestamp>.png` ali (R9);
  - botões arriscados ("Iniciar procura", "Desinstalar", "Limpar", "Instalar atualização", "Gerar", "Sair") **só no Sandbox** (R10);
  - fechar o app **pela janela ou pelo PID que o próprio `subir.mjs` gravou**; nunca `taskkill /IM claudia-rh.exe`, `/IM node.exe` nem `Stop-Process -Name`.

## Fluxo

1. **Conectar:** `node scripts/subir.mjs`. Devolve JSON com `acao`: `subiu`, `reutilizar`, `perguntar` ou `abortar`. Se `perguntar`, o app já roda sem CDP: peça ao usuário antes de reiniciar.
2. **Ver:** `node scripts/ver.mjs arvore | tela [arq] | clicar "<rótulo>"`.
3. **Backend:** `node scripts/invocar.mjs <comando> [json]` e `node scripts/banco.mjs "SELECT …"`. Compare o que a UI mostra com o banco.
4. **Observar agentes:** `node scripts/observar.mjs`. Iniciar uma sessão é apertar o botão na UI do app, nunca rodar um agente por fora.

Defina `CDP_PORT` com a porta devolvida por `subir.mjs` (padrão 9333). `CDP_HOST` (padrão `127.0.0.1`) escolhe a máquina; `CLAUDIA_DB` escolhe o banco lido por `banco.mjs` e `observar.mjs`.

## Alvo

| Alvo | Como |
|---|---|
| Local (`tauri dev` no host) | padrão; `node scripts/subir.mjs` |
| Sandbox | UIA (abaixo) ou, se o CDP abrir, `CDP_HOST`/`CDP_PORT` impressos por `node .claude/skills/claudia-teste-sandbox/scripts/cdp-guest.mjs --iniciar`; banco: `CLAUDIA_DB` impresso por `node .claude/skills/claudia-teste-sandbox/scripts/banco-guest.mjs` (snapshot; repetir após cada ação) |

CDP no guest não funciona hoje: a porta de depuração não abre, nem com build de debug (a hipótese do navegador já em execução foi refutada; ver `claudia-teste-sandbox`, seção 6). No Sandbox, veja e clique por **UIA**, sem CDP:

- `winapp ui inspect --on sandbox -a claudia-rh -d 30` (o conteúdo web só aparece com `-d` em torno de 30; a profundidade padrão, mesmo com `-i`, não o mostra);
- `winapp ui search --on sandbox "<texto>" -a claudia-rh`;
- `winapp ui invoke --on sandbox <id> -a claudia-rh`; por nome, com `-w <hwnd>` (verificado só assim) (só navegação; nunca envio, exclusão ou busca).

Sem `invoke` de comandos Tauri e sem JavaScript por UIA. Prints pelos degraus 2 e 3 da escada abaixo.

Com `CDP_HOST` remoto, `subir.mjs` nunca sobe `tauri dev`: ou reutiliza o CDP do app no guest, ou aborta (exit 3).

## Modo reconhecimento

Quando a queixa é vaga, não suponha. Conecte, abra a tela citada, leia a árvore, tire screenshot e **descreva o que vê**. Só depois pergunte ou proponha. Nesse modo nada é alterado. Se o bug depende de dados ou sequência, pergunte como chegar lá.

## Bug citado

Primeiro reproduza no app rodando. Só feche depois de reproduzido antes e confirmado depois.

## O que nunca fazer

- Escrever direto no banco ou nos YAMLs do perfil. Escrita só por `invocar.mjs --escrever`, avisando antes.
- Editar `prompt_sistema_runtime.md` ou o `identifier` Tauri.
- Reiniciar o app do usuário sem perguntar, ou matar processo que não subiu (nem por nome de imagem: ver regras duras).
- Clicar em rótulo destrutivo, de envio, ou sem nome (o `ver.mjs` recusa).
- Dar teste por passado porque o `cargo test` ficou verde.

## Prints

Comandos relativos à pasta da skill (`.claude/skills/claudia-teste-e2e/`), como no resto deste arquivo; scripts da `claudia-teste-sandbox` com o caminho a partir da raiz do repositório.

Escada, do mais fiel ao mais robusto; usar o primeiro que funcionar e dizer qual foi usado:

| # | Local (host) | Sandbox | Mostra |
|---|---|---|---|
| 1 | `node scripts/ver.mjs tela <arq>` | idem, com `CDP_HOST`/`CDP_PORT` | só a página do WebView2 |
| 2 | `pwsh -NoProfile -File scripts/print-janela.ps1 -Processo claudia-rh -Saida <arq>` | `winapp ui screenshot --on sandbox -a claudia-rh -w <hwnd> -o <arq>` (sem `-w`, sai também uma janela minúscula extra) | janela nativa inteira, inclusive diálogos nativos que o CDP não vê |
| 2b | — | UIA: `winapp ui inspect --on sandbox -a claudia-rh -d 30` | árvore de acessibilidade do conteúdo web (texto, não imagem) |
| 3 | `pwsh -NoProfile -File scripts/print-janela.ps1 -TelaInteira -Saida <arq>` | `winapp target screenshot sandbox -o <arq>` | tela inteira; no Sandbox é o desktop do guest e, nos perfis `app*`, o Chrome do laboratório pode ficar na frente e cobrir o app |
| 4 | — | sem rede: `C:\labsandbox\app\guest-observe.ps1` via `wsb exec` (PNG em `C:\labwork\run`) | tela inteira + árvore UIA |

`scripts/print-janela.ps1`: PowerShell + `System.Drawing`, sem dependências; captura o retângulo da janela principal do processo (DPI-aware) ou a tela inteira (com vários monitores, `-TelaInteira` captura o retângulo de toda a tela virtual, com áreas pretas onde não há monitor); só leitura (não foca, não clica). Execute com `pwsh -NoProfile -File scripts/print-janela.ps1 ...`. Prints vão para `debug-logs/prints/` (já ignorada pelo git), só ali e sem sobrescrever e podem conter dados pessoais: não anexar a issues públicas sem revisar.

**Regra:** toda afirmação sobre a UI vem com print; se nenhum degrau da escada funcionar, dizer quais falharam e por quê.

## Saída

Cada teste devolve: o que foi feito, o que foi visto (árvore ou screenshot) e o veredito, com evidência.

## Mais detalhe

- `references/limites.md` — o que foi medido e o que não funciona
- `references/padroes-cdp.md` — armadilhas
- `references/comandos.md` — comandos Tauri (gerado por `scripts/gerar-comandos.mjs`)
