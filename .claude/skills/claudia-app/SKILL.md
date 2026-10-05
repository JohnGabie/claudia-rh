---
name: claudia-app
description: >-
  Olha o Claudia RH (app Tauri) RODANDO, local ou no Windows Sandbox (CDP_HOST), por CDP: ver a tela, clicar, chamar comandos do backend e ler o que os agentes fazem.
  Use SEMPRE que o usuário falar de algo que se vê ou se confere no app aberto: testar ou conferir uma tela (Dashboard, Perfil, Histórico), comparar a UI com o banco, queixa de layout ou aparência (não gostei do layout, tá estranho, olha o que tá errado nessa tela, mexi no CSS), perguntar se o agente ou a sessão se comportou como devia, comparar agentes (Claude vs Grok), ou chamar um comando Tauri.
  QUALQUER bug do app, inclusive pedido curto como "corrige este bug", "o bug tá acontecendo de novo" ou "tem um bug no X": carregue esta skill ANTES de abrir ou editar código, porque o primeiro passo é reproduzir no app rodando.
  Use também antes de dar por concluída qualquer mudança em UI ou em comandos Tauri.
  NÃO use para cargo test puro, sites de vagas, Linear, nem para montar Windows limpo, escolher dados ou instalar o app no Sandbox (claudia-sandbox).
---

# claudia-app

Dá a quem desenvolve o olhar de um dev sobre o app rodando. **Nunca interage com sites de vagas** e nunca clica em algo que submeta candidatura: quem interage com sites são os agentes dentro da Claudia.

Todos os scripts ficam em `.claude/skills/claudia-app/scripts/` e rodam com Node ≥ 22, sem instalar nada.

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
| Sandbox | `CDP_HOST`/`CDP_PORT` impressos por `node claudia-sandbox/scripts/cdp-guest.mjs`; banco: `CLAUDIA_DB` impresso por `node claudia-sandbox/scripts/banco-guest.mjs` (snapshot; repetir após cada ação) |

Com `CDP_HOST` remoto, `subir.mjs` nunca sobe `tauri dev`: ou reutiliza o CDP do app no guest, ou aborta (exit 3).

## Modo reconhecimento

Quando a queixa é vaga, não suponha. Conecte, abra a tela citada, leia a árvore, tire screenshot e **descreva o que vê**. Só depois pergunte ou proponha. Nesse modo nada é alterado. Se o bug depende de dados ou sequência, pergunte como chegar lá.

## Bug citado

Primeiro reproduza no app rodando. Só feche depois de reproduzido antes e confirmado depois.

## O que nunca fazer

- Escrever direto no banco ou nos YAMLs do perfil. Escrita só por `invocar.mjs --escrever`, avisando antes.
- Editar `prompt_sistema_runtime.md` ou o `identifier` Tauri.
- Reiniciar o app do usuário sem perguntar, ou matar processo que não subiu.
- Clicar em rótulo destrutivo, de envio, ou sem nome (o `ver.mjs` recusa).
- Dar teste por passado porque o `cargo test` ficou verde.

## Prints

Comandos relativos à pasta da skill (`.claude/skills/claudia-app/`), como no resto deste arquivo; scripts da `claudia-sandbox` com o caminho a partir da raiz do repositório.

Escada, do mais fiel ao mais robusto; usar o primeiro que funcionar e dizer qual foi usado:

| # | Local (host) | Sandbox | Mostra |
|---|---|---|---|
| 1 | `node scripts/ver.mjs tela <arq>` | idem, com `CDP_HOST`/`CDP_PORT` | só a página do WebView2 |
| 2 | `pwsh -NoProfile -File scripts/print-janela.ps1 -Processo claudia-rh -Saida <arq>` | `winapp ui screenshot --on sandbox -a claudia-rh -o <arq>` | janela nativa inteira, inclusive diálogos nativos que o CDP não vê |
| 3 | `pwsh -NoProfile -File scripts/print-janela.ps1 -TelaInteira -Saida <arq>` | `winapp target screenshot sandbox -o <arq>` | tela inteira |
| 4 | — | sem rede: `C:\labsandbox\app\guest-observe.ps1` via `wsb exec` (PNG em `C:\labwork\run`) | tela inteira + árvore UIA |

`scripts/print-janela.ps1`: PowerShell + `System.Drawing`, sem dependências; captura o retângulo da janela principal do processo (DPI-aware) ou a tela inteira; só leitura (não foca, não clica). Execute com `pwsh -NoProfile -File scripts/print-janela.ps1 ...`. Prints vão para `debug-logs/prints/` por padrão (já ignorada pelo git) e podem conter dados pessoais: não anexar a issues públicas sem revisar.

**Regra:** toda afirmação sobre a UI vem com print; se nenhum degrau da escada funcionar, dizer quais falharam e por quê.

## Saída

Cada teste devolve: o que foi feito, o que foi visto (árvore ou screenshot) e o veredito, com evidência.

## Mais detalhe

- `references/limites.md` — o que foi medido e o que não funciona
- `references/padroes-cdp.md` — armadilhas
- `references/comandos.md` — comandos Tauri (gerado por `scripts/gerar-comandos.mjs`)
