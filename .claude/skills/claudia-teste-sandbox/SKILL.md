---
name: claudia-teste-sandbox
description: >-
  Testa o Claudia RH (app Tauri) num Windows LIMPO e isolado (Windows Sandbox, laboratório em SANDBOX_LAB_DIR): monta o ambiente, escolhe os dados (cenário novo, mock ou cópia dos dados do usuário), publica ou instala o app e liga o CDP do app dentro do guest.
  Use SEMPRE que o pedido envolver máquina limpa ou PC novo ("funciona num PC novo?", "testa do zero", "como se fosse a primeira vez"), Windows Sandbox, testar o instalador (NSIS, setup, pré-requisitos como WebView2), primeira abertura e welcome, desinstalação, build ou instalação do zero, ou testar com os dados reais do usuário sem mexer neles. Use também para preparar o Sandbox antes de olhar o app lá dentro.
  NÃO use para olhar, clicar ou tirar print do app já rodando (local ou no Sandbox já montado), reproduzir bug no app aberto, mexer em CSS ou UI: isso é a claudia-teste-e2e. NÃO use para cargo test, tsc, Linear nem sites de vagas.
---

# claudia-teste-sandbox

Mapa do laboratório Windows Sandbox para testar o app num Windows limpo. Diz o que existe, quando usar cada peça e quais regras valem; **não é roteiro**: você desenha o teste conforme a pergunta.

Comandos abaixo partem da **raiz do repositório** (`node .claude/skills/claudia-teste-sandbox/scripts/...`). Scripts em Node ≥ 22.13, sem dependências npm. Contrato: exit 0 ok, 1 falha, 2 erro de uso; a **última linha do stdout é JSON**.

## 1. Pré-requisitos

- `SANDBOX_LAB_DIR` aponta para o laboratório (precisa de `scripts\app-stage.ps1`). Ausente ou incompleto: **pare e diga ao usuário o que falta**; não invente caminho nem crie laboratório.
- Perfis `app*` exigem `LAB_APP_PROJECT` (projeto, read-only no guest em `C:\labproject`) e `LAB_WORK_DIR` (pasta de trabalho). Use o **mesmo** `LAB_WORK_DIR` no `publicar.mjs`, no `test-up.ps1` e no `banco-guest.mjs`.
- Windows Sandbox habilitado (`wsb` no PATH). Só existe **um** Sandbox por vez.

## 2. O laboratório em uma tela

| Peça (em `%SANDBOX_LAB_DIR%`) | Uso |
|---|---|
| `scripts\test-up.ps1 -Config <perfil>` / `scripts\test-down.ps1` | sobe / derruba (descarta tudo do guest); id em `wsb list` |
| `lab.config.app.json` | app sem rede; controle só por `wsb exec` (winapp não alcança o guest) |
| `lab.config.app-winapp.json` | rede ligada para winapp e CDP; internet bloqueada por `sandbox\app\guest-firewall.ps1` (rodar como System) |
| `lab.config.app-build.json` | build dentro do guest com toolchains do host read-only |
| `sandbox\app\guest-run.ps1 [-Exe] [-NoStart]` | no guest: copia `appdata`/`localappdata` publicados e inicia o exe (`-NoStart`: só os dados) |
| `guest-observe.ps1`, `guest-click.ps1`, `guest-chrome-probe.ps1` | tela + árvore UIA, clique e Chrome no guest sem winapp |

Pastas no guest: `C:\lab` (shared, RO; `C:\lab\tools\sqlite\sqlite3.exe`), `C:\labsandbox` (scripts do guest, RO), `C:\appdeploy` (deploy, RO), `C:\labwork` (= `<work-dir>\guest-rw`, gravável). Controle: `wsb exec --id <id> -r ExistingLogin|System -c "<comando>"` (não devolve a saída do processo: grave em `C:\labwork`; o `wsb exec` sempre sai com 0 no host e o código do guest aparece só como texto localizado no stdout, que os scripts desta skill interpretam: `references/limites.md`). Documentação completa: `%SANDBOX_LAB_DIR%\.claude\skills\sandbox-lab\SKILL.md`. Comportamentos que afetam o teste: `references/limites.md`.

## 3. Específico do app

- Dados em `%APPDATA%\io.github.johngabie.claudia-rh` (perfil, banco, configs, `diagnostics\`); logs do plugin em `%LOCALAPPDATA%\io.github.johngabie.claudia-rh\logs`. Lista conferida no código: `references/pastas-de-dados.md`.
- `debug-logs/` só existe em build de debug (`tauri dev`); no guest, o diagnóstico é `diagnostics\` do `%APPDATA%`.
- Browser próprio baixado na primeira abertura e Activity log: **não encontrados no código desta versão**; confira na versão em teste antes de afirmar algo sobre eles.
- O welcome confere Claude Code instalado, native host do Chrome para a extensão do Claude e perfil preenchido (`verificar_setup`): num Windows limpo, a ausência é esperada e é parte do que se testa.

## 4. Cenários de dados

Cada script recria `--saida` (padrão `%TEMP%\claudia-teste-sandbox\<cenario>`; com o disco do `%TEMP%` cheio, passe `--saida` em outro disco) e grava `cenario.json`. Só recria pastas de cenário (com `cenario.json` ou marcador `.cenario`); recusa pasta não vazia sem relação, a raiz do repositório e seus ancestrais, e raiz de disco.

| Cenário | Comando | Quando faz sentido |
|---|---|---|
| `novo` | `node .claude/skills/claudia-teste-sandbox/scripts/cenario-novo.mjs [--saida <p>]` | primeira abertura, welcome, criação de perfil, instalador num PC novo |
| `mock` | `node .claude/skills/claudia-teste-sandbox/scripts/cenario-mock.mjs [--saida <p>]` | telas com dados sem expor ninguém: perfil e banco fictícios, automação desligada |
| `copia` | `node .claude/skills/claudia-teste-sandbox/scripts/cenario-copia.mjs --autorizado [--origem <p>] [--saida <p>] [--manter-automacao]` | bug que só aparece com os dados reais; migração sobre dados reais |

**`copia` exige pedir autorização ao usuário naquela rodada**, dizendo o que será copiado; autorização de outra rodada não vale. Sem `--autorizado` sai com exit 2 sem ler nada. Comportamento:
- copia só `%APPDATA%\io.github.johngabie.claudia-rh` e só lê a origem; **nunca** `%LOCALAPPDATA%` (perfil do browser, cookies);
- recusa `--saida` e `--origem` sobrepostas;
- consolida o WAL do SQLite na cópia e confere integridade (até 3 tentativas; se falhar, peça ao usuário para fechar o app);
- desliga disparo e modo autônomo na cópia, salvo `--manter-automacao` (só com motivo e com o usuário ciente).

## 5. O que publicar

`node .claude/skills/claudia-teste-sandbox/scripts/publicar.mjs --app <exe|instalador|pasta> --cenario <pasta> [--work-dir <p>]` (padrão `LAB_WORK_DIR`, senão `%TEMP%\claudia-teste-sandbox\work`). O app vai para `C:\appdeploy\app`, os dados para `C:\appdeploy\appdata`.

- **exe de `tauri build --no-bundle`**: rápido, mas não instala pré-requisitos; sem WebView2 no guest o app não abre.
- **instalador** (`tauri build`, NSIS): cuida de pré-requisitos como o WebView2 (o bootstrapper precisa de rede). Silencioso: `wsb exec --id <id> -r ExistingLogin -c "C:\appdeploy\app\<instalador>.exe /S"`. Instale os dados antes da primeira abertura com `guest-run.ps1 -NoStart` (com o instalador publicado, o `guest-run.json` registra o instalador como `exe`: inofensivo).
- **`npm ci` falha com EPERM** se qualquer processo (ex.: um servidor vite de outra worktree) segura arquivos em `node_modules` (como `@esbuild\win32-x64\esbuild.exe`), e deixa o `node_modules` pela metade: pare o processo antes, ou use uma worktree separada e `CARGO_TARGET_DIR` próprio. O `tauri build` também pode regravar `src-tauri/Cargo.toml` (só fim de linha), sujando o `git status`.
- Build dentro do guest: perfil `lab.config.app-build.json`; target do cargo no disco do guest (`references/limites.md`).

## 6. Observar o app no guest

1. `node .claude/skills/claudia-teste-sandbox/scripts/cdp-guest.mjs [--id <id>] [--iniciar] [--exe-path <exe no guest>] [--porta 9333] [--porta-host 19333] [--timeout 90]` → `{host, porta, browser}`. O WebView2 só escuta CDP no `127.0.0.1` do guest; o script cria `netsh portproxy` e regra de firewall no guest. Com `--iniciar`, sobe o app com CDP (`--exe-path` para o app instalado; sem ele, `guest-run.ps1`). Falha do `wsb exec` e código de saída não zero do processo no guest aparecem na hora (lidos da linha localizada "... código: N" do stdout, pois o `wsb exec` sempre sai com 0). Exige perfil com rede.
   - **CDP no guest não funciona hoje.** A porta de depuração remota não abre, nem com build de debug. A variável de ambiente `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` chega ao processo e a política `AdditionalBrowserArguments` por app pode ser aplicada, mas o WebView2 do guest não responde em ambos os casos; a causa está em aberto. Use os degraus 2 e 3 da escada de prints em vez de CDP (experimental: `cdp-guest.mjs` monta portproxy + firewall corretamente, e falha com timeout).
2. Use `CDP_HOST`/`CDP_PORT` com a skill **`claudia-teste-e2e`** (tela, árvore, cliques, comandos, agentes).
3. Banco: `node .claude/skills/claudia-teste-sandbox/scripts/banco-guest.mjs [--id <id>] [--work-dir <p>] [--timeout 30]` → `{db}`, snapshot consistente (`.tmp` → rename) para usar como `CLAUDIA_DB`. Repita depois de cada ação que muda dados.
4. Prints: escada de prints da `claudia-teste-e2e` (inclui degraus sem CDP e sem rede).

Fluxo típico, **exemplo e não obrigação**: cenário → `publicar.mjs` → `test-up.ps1 -Config lab.config.app-winapp.json` → `guest-firewall.ps1` (System) → instalar ou `guest-run.ps1` → `cdp-guest.mjs` → observar com `claudia-teste-e2e` → prints → copiar evidências de `C:\labwork` → `test-down.ps1`.

## 7. Regras

- **Nunca acione envio de candidatura** nem ação com efeito externo real; não faça login em contas reais no guest.
- **Falha do app num Windows limpo é resultado do teste**: registre com evidência (print, `diagnostics\`, `C:\labwork\run\guest-run.json`); não contorne instalando coisas no guest nem mudando o laboratório.
- Achado fora do escopo do teste vira task no Linear pela skill `linear-claudia`.
- Nunca aja no desktop do host: UI só no guest (`winapp ... --on sandbox`, `wsb exec`).
- Derrube o Sandbox com `test-down.ps1` ao terminar; o guest some com tudo que não foi copiado para `C:\labwork`.

## 8. Melhorias no ferramental

Quando notar algo que tornaria o teste mais fácil, rápido ou confiável (no laboratório, nos scripts e cenários desta skill ou nesta skill), **sugira ao usuário com o motivo e o custo**. Com o "sim" dele, implemente (o laboratório tem processo próprio, fora deste repositório). Se não for o momento, registre no Linear (`linear-claudia`). **Nunca mude o ferramental por conta própria no meio de um teste**: um teste com ferramenta alterada no caminho não prova nada.

## 9. Testes

Você desenha os testes a partir da pergunta e decide se vale escrever testes automáticos. Os testes dos scripts desta skill rodam sem Sandbox:

```
node --test ".claude/skills/claudia-teste-sandbox/scripts/test/*.test.mjs"
```

Use o glob; a forma `node --test <pasta>/` falha no Node 24.

## Saída

Diga o cenário, o perfil, o que foi publicado (exe ou instalador), o que foi feito, o que foi visto (print indicando o degrau da escada, banco) e o veredito com evidência. Sem Sandbox disponível, diga o que faltou em vez de simular.

## Mais detalhe

- `references/limites.md`: comportamentos do laboratório que afetam o teste e evals de disparo
- `references/pastas-de-dados.md`: o que fica em `%APPDATA%` e `%LOCALAPPDATA%` do app
