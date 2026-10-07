---
name: claudia-teste-sandbox
description: >-
  Testa o Claudia RH (app Tauri) num Windows LIMPO e isolado (Windows Sandbox, laboratório em sandbox-lab/ do repositório): monta o ambiente, escolhe os dados (cenário novo, mock ou cópia dos dados do usuário), publica ou instala o app e liga o CDP do app dentro do guest.
  Use SEMPRE que o pedido envolver máquina limpa ou PC novo ("funciona num PC novo?", "testa do zero", "como se fosse a primeira vez"), Windows Sandbox, testar o instalador (NSIS, setup, pré-requisitos como WebView2), primeira abertura e welcome, desinstalação, build ou instalação do zero, ou testar com os dados reais do usuário sem mexer neles. Use também para preparar o Sandbox antes de olhar o app lá dentro.
  NÃO use para olhar, clicar ou tirar print do app já rodando (local ou no Sandbox já montado), reproduzir bug no app aberto, mexer em CSS ou UI: isso é a claudia-teste-e2e. NÃO use para cargo test, tsc, Linear nem sites de vagas.
---

# claudia-teste-sandbox

Mapa do laboratório Windows Sandbox para testar o app num Windows limpo. Diz o que existe, quando usar cada peça e quais regras valem; **não é roteiro**: você desenha o teste conforme a pergunta.

Comandos abaixo partem da **raiz do repositório** (`node .claude/skills/claudia-teste-sandbox/scripts/...`). Scripts em Node ≥ 22.13, sem dependências npm. Contrato: exit 0 ok, 1 falha, 2 erro de uso; a **última linha do stdout é JSON**.

## 0. Regras duras (valem antes de tudo)

Origem: incidente de 2026-10-05 (um teste passou `C:\` a uma função de apagar). Detalhes, decisões B1 a B7 e tabela R1 a R12: `docs/superpowers/evidencias/2026-10-06-auditoria-delecao.md`; regras gerais no `CLAUDE.md` do repositório.

- Teste que apaga qualquer arquivo roda **só no Windows Sandbox**. No PC do usuário: só compilar, clippy, `tsc`, build e testes que não tocam o disco.
- **Nenhum caminho real em teste** (raiz de disco, repositório, pasta acima dele, `AppData`, sistema), nem para provar a recusa: a recusa se testa na função de validação pura.
- **Nunca o Chrome pessoal do usuário** (nem perfil, nem cópia). O Chrome do Claudia-RH pode.
- `node --test` **sempre com a lista de arquivos explícita**, nunca glob nem pasta.
- A guarda "estou no Sandbox" dos testes e do `travas.sandbox.ps1` usa **dois fatores**: identidade `WDAGUtilityAccount` (do SO, nunca `$env:USERNAME`) **e** o arquivo `C:\labsandbox\bootstrap.ps1`, que só existe montado no guest.

## 1. Pré-requisitos

- O laboratório fica em `sandbox-lab/` do **checkout principal** do repositório; a pasta de trabalho, em `.sandbox-work/` do mesmo checkout. Descubra os caminhos com `node .claude/skills/claudia-teste-sandbox/scripts/onde.mjs` (só lê o git) → `{raizPrincipal, projeto, lab, trabalho, deploy, guestRw, appStage, testUp}`. De uma worktree, `projeto` é a worktree e o resto é do checkout principal.
- Sem variável de ambiente de caminho: `SANDBOX_LAB_DIR`, `LAB_APP_PROJECT` e `LAB_WORK_DIR` não existem mais.
- Ferramentas (`sandbox-lab/shared/tools/`, conferidas por `sandbox-lab/shared/tools.sha256`) e a chave do MCP (`sandbox-lab/shared/secrets/`) ficam fora do git. Se o `onde.mjs` falhar ou o `test-up.ps1` disser que falta ferramenta: **pare e diga ao usuário o que falta** (`sandbox-lab/README.md`, seção Ferramentas); não baixe nada nem crie laboratório.
- Windows Sandbox habilitado (`wsb` no PATH). Só existe **um** Sandbox por vez.

## 2. O laboratório em uma tela

| Peça (em `lab` do `onde.mjs`) | Uso |
|---|---|
| `scripts\test-up.ps1 -Config <perfil> -Project <projeto>` / `scripts\test-down.ps1` | sobe / derruba (descarta tudo do guest); id em `wsb list`; `<projeto>` vem do `onde.mjs` |
| `lab.config.app.json` | app sem rede; controle só por `wsb exec` (winapp não alcança o guest) |
| `lab.config.app-winapp.json` | rede ligada para winapp e CDP; internet bloqueada por `sandbox\app\guest-firewall.ps1` (rodar como System) |
| `lab.config.app-build.json` | build dentro do guest com toolchains do host read-only |
| `sandbox\app\guest-run.ps1 [-Exe] [-NoStart]` | no guest: copia `appdata`/`localappdata` publicados e inicia o exe (`-NoStart`: só os dados) |
| `guest-observe.ps1`, `guest-click.ps1`, `guest-chrome-probe.ps1` | tela + árvore UIA, clique e Chrome no guest sem winapp |

Pastas no guest: `C:\lab` (shared, RO; `C:\lab\tools\sqlite\sqlite3.exe`), `C:\labsandbox` (scripts do guest, RO), `C:\appdeploy` (deploy, RO), `C:\labwork` (= `.sandbox-work\guest-rw`, gravável). Controle: `wsb exec --id <id> -r ExistingLogin|System -c "<comando>"` (não devolve a saída do processo: grave em `C:\labwork`; o `wsb exec` sempre sai com 0 no host e o código do guest aparece só como texto localizado no stdout, que os scripts desta skill interpretam: `references/limites.md`). Documentação completa: `references/laboratorio.md`. Comportamentos que afetam o teste: `references/limites.md`.

## 3. Específico do app

- Dados em `%APPDATA%\io.github.johngabie.claudia-rh` (perfil, banco, configs, `diagnostics\`); logs do plugin em `%LOCALAPPDATA%\io.github.johngabie.claudia-rh\logs`. Lista conferida no código: `references/pastas-de-dados.md`.
- `debug-logs/` só existe em build de debug (`tauri dev`); no guest, o diagnóstico é `diagnostics\` do `%APPDATA%`.
- Browser próprio baixado na primeira abertura e Activity log: **não encontrados no código desta versão**; confira na versão em teste antes de afirmar algo sobre eles.
- O welcome confere Claude Code instalado, native host do Chrome para a extensão do Claude e perfil preenchido (`verificar_setup`): num Windows limpo, a ausência é esperada e é parte do que se testa.

## 4. Cenários de dados

Em andamento: o cenário de dados falsos completos está em spec (CLA-53, branch `feat/dados-falsos-teste`).

Cada script recria `--saida` (padrão `%TEMP%\claudia-teste-sandbox\<cenario>`; `--saida` só dentro de `<tmp>\claudia-teste-sandbox\`, nunca a própria pasta base, outro disco, link ou pasta real) e grava `cenario.json`. Só recria pastas de cenário (com `cenario.json` ou marcador `.cenario`); recusa pasta não vazia sem relação, a raiz do repositório e seus ancestrais, e raiz de disco.

| Cenário | Comando | Quando faz sentido |
|---|---|---|
| `novo` | `node .claude/skills/claudia-teste-sandbox/scripts/cenario-novo.mjs [--saida <p>]` | primeira abertura, welcome, criação de perfil, instalador num PC novo |
| `mock` | `node .claude/skills/claudia-teste-sandbox/scripts/cenario-mock.mjs [--saida <p>]` | telas com dados sem expor ninguém: perfil e banco fictícios, automação desligada |
| `copia` | `node .claude/skills/claudia-teste-sandbox/scripts/cenario-copia.mjs --autorizado [--origem <p>] [--saida <p>] [--manter-automacao]` | bug que só aparece com os dados reais; migração sobre dados reais |

**`copia` exige pedir autorização ao usuário naquela rodada**, dizendo o que será copiado; autorização de outra rodada não vale. Sem `--autorizado` sai com exit 2 sem ler nada. Comportamento:
- copia só `%APPDATA%\io.github.johngabie.claudia-rh` e só lê a origem; **nunca** `%LOCALAPPDATA%` (perfil do browser, cookies);
- recusa `--saida` e `--origem` sobrepostas;
- consolida o WAL do SQLite na cópia e confere integridade (até 3 tentativas; se falhar, peça ao usuário para fechar o app);
- desliga disparo e modo autônomo na cópia, salvo `--manter-automacao` (só com motivo e com o usuário ciente);
- não reproduz links simbólicos da origem (ex.: `diagnostics\latest`): não aparecem na cópia; o app recria o link ao abrir. Ficam de fora também `*.db-wal` e `*.db-shm` (consolidados no banco).

Verificado ao vivo de ponta a ponta: origem inalterada (listagem idêntica antes e depois), automação desligada no guest antes e depois da primeira abertura, firewall aplicado antes da primeira abertura, contagens do banco no guest iguais às da cópia no host.

## 5. O que publicar

`node .claude/skills/claudia-teste-sandbox/scripts/publicar.mjs --app <exe|instalador|pasta> --cenario <pasta>`. A pasta de trabalho é sempre `.sandbox-work/` do checkout principal (não há `--work-dir`). O `-Clean` só roda se `.sandbox-work\deploy` não existir ou tiver o marcador `.lab-deploy`, sem junction nem link no caminho ou dentro dele; a mesma trava está no `publicar.mjs` e no `app-stage.ps1`. O app vai para `C:\appdeploy\app`, os dados para `C:\appdeploy\appdata`. O `app-stage.ps1` recusa origem que seja raiz de disco, o próprio deploy, uma pasta que o contenha ou algo dentro dele, e copia com `robocopy /XJ` (não segue junction). Só o `app-stage.ps1` mostra o que foi publicado (total de arquivos e MB); o `publicar.mjs` aplica a trava do `-Clean` e **não confere o conteúdo** do deploy.

- **exe de `tauri build --no-bundle`**: rápido, mas não instala pré-requisitos; sem WebView2 no guest o app não abre.
- **instalador** (`tauri build`, NSIS): cuida de pré-requisitos como o WebView2 (o bootstrapper precisa de rede). Silencioso: `wsb exec --id <id> -r ExistingLogin -c "C:\appdeploy\app\<instalador>.exe /S"`. Instale os dados antes da primeira abertura com `guest-run.ps1 -NoStart` (com o instalador publicado, o `guest-run.json` registra o instalador como `exe`: inofensivo).
- **`npm ci` falha com EPERM** se qualquer processo (ex.: um servidor vite de outra worktree) segura arquivos em `node_modules` (como `@esbuild\win32-x64\esbuild.exe`), e deixa o `node_modules` pela metade: pare o processo antes, ou use uma worktree separada e `CARGO_TARGET_DIR` próprio. O `tauri build` também pode regravar `src-tauri/Cargo.toml` (só fim de linha), sujando o `git status`.
- Build dentro do guest: perfil `lab.config.app-build.json`; target do cargo no disco do guest (`references/limites.md`).

## 6. Observar o app no guest

1. `node .claude/skills/claudia-teste-sandbox/scripts/cdp-guest.mjs [--id <id>] [--iniciar] [--exe-path <exe no guest>] [--porta 9333] [--porta-host 19333] [--timeout 90]` → `{host, porta, browser}`. O WebView2 só escuta CDP no `127.0.0.1` do guest; o script cria `netsh portproxy` e regra de firewall no guest. Com `--iniciar`, sobe o app com CDP (`--exe-path` para o app instalado; sem ele, `guest-run.ps1`). Falha do `wsb exec` e código de saída não zero do processo no guest aparecem na hora (lidos da linha localizada "... código: N" do stdout, pois o `wsb exec` sempre sai com 0). Exige perfil com rede.
   - **CDP no guest não funciona hoje.** A porta de depuração remota não abre, nem com build de release nem com o de debug. A variável `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` chega ao ambiente do shell que inicia o app, mas o `msedgewebview2.exe` sobe sem `--remote-debugging-port`. A hipótese de que o WebView2 reaproveitaria um navegador já em execução foi **testada e refutada** (todos os `claudia-rh.exe` e `msedgewebview2.exe` encerrados antes de iniciar: nada mudou), então encerrar instâncias não ajuda. A causa é desconhecida; o caminho restante é um opt-in no app para abrir a porta em builds de teste, que precisa de spec própria. `cdp-guest.mjs` monta portproxy e firewall corretamente e falha com timeout.
   - **Sem CDP, use UIA** (verificada ao vivo, com o app instalado): ver o passo 2.
2. Com CDP (hoje indisponível no guest), use `CDP_HOST`/`CDP_PORT` com a skill **`claudia-teste-e2e`** (tela, árvore, cliques, comandos, agentes). **Sem CDP, UIA pelo winapp** (perfil com rede; `-a claudia-rh` é o app):
   - `winapp ui inspect --on sandbox -a claudia-rh -d 30`: árvore, inclusive o conteúdo web (URL da página, botões de navegação, textos). **O `-d 30` é necessário**: com a profundidade padrão, mesmo com `-i`, só aparece a barra de título nativa. Elementos fora da área visível vêm marcados `[offscreen]`.
   - `winapp ui search --on sandbox "<texto>" -a claudia-rh`: acha elementos e devolve o id.
   - `winapp ui invoke --on sandbox <id> -a claudia-rh` (por nome: `winapp ui invoke --on sandbox "<nome>" -a claudia-rh -w <hwnd>`, verificado só com `-w`): clica por id ou por nome. **Só navegação**; nunca botões de envio, exclusão ou busca.
   - `winapp ui screenshot --on sandbox -a claudia-rh -w <hwnd> -o <arq>`: print só da janela do app; sem `-w` o print inclui também uma janela minúscula extra.
   - Limites: sem `invoke` de comandos Tauri e sem avaliação de JavaScript; só o que a árvore de acessibilidade expõe.
   - A página do app instalado (release) tem origem `http://tauri.localhost/`.
3. Banco: `node .claude/skills/claudia-teste-sandbox/scripts/banco-guest.mjs [--id <id>] [--timeout 30]` → `{db}`, snapshot consistente (`.tmp` → rename) para usar como `CLAUDIA_DB`. Repita depois de cada ação que muda dados.
4. Prints: escada de prints da `claudia-teste-e2e` (inclui degraus sem CDP e sem rede).

Fluxo típico, **exemplo e não obrigação**: cenário → `publicar.mjs` → `test-up.ps1 -Config lab.config.app-winapp.json -Project <projeto do onde.mjs>` → `guest-firewall.ps1` (System) → instalar ou `guest-run.ps1` → `cdp-guest.mjs` → observar com `claudia-teste-e2e` → prints → copiar evidências de `C:\labwork` → `test-down.ps1`.

## 7. Regras

- **Nunca acione envio de candidatura** nem ação com efeito externo real; não faça login em contas reais no guest.
- **Falha do app num Windows limpo é resultado do teste**: registre com evidência (print, `diagnostics\`, `C:\labwork\run\guest-run.json`); não contorne instalando coisas no guest nem mudando o laboratório.
- Achado fora do escopo do teste vira task no Linear pela skill `linear-claudia`.
- Nunca aja no desktop do host: UI só no guest (`winapp ... --on sandbox`, `wsb exec`).
- Derrube o Sandbox com `test-down.ps1` ao terminar; o guest some com tudo que não foi copiado para `C:\labwork`.

## 8. Melhorias no ferramental

Quando notar algo que tornaria o teste mais fácil, rápido ou confiável (no laboratório, nos scripts e cenários desta skill ou nesta skill), **sugira ao usuário com o motivo e o custo**. Com o "sim" dele, implemente (o laboratório está em `sandbox-lab/` e segue o processo deste repositório: spec, plano, revisão). Se não for o momento, registre no Linear (`linear-claudia`). **Nunca mude o ferramental por conta própria no meio de um teste**: um teste com ferramenta alterada no caminho não prova nada.

## 9. Testes

Você desenha os testes a partir da pergunta e decide se vale escrever testes automáticos.

- **Puros (rodam no PC; não tocam o disco):** `scripts/test/proibidos.test.mjs`, `manifesto.test.mjs`, `caminhos-lab.test.mjs`, `onde.test.mjs`, `testes-no-guest.test.mjs`, `motivo-recusa.test.mjs`, `sandbox.test.mjs`, e `sandbox-lab/scripts/test/travas.test.ps1`.
- **O CI (job `repo-guard`, `.github/workflows/ci.yml`) roda só:** `conferir-repo.mjs` (leitura via `git ls-files`: arquivos proibidos e caminho de máquina em `sandbox-lab/`, `.claude/skills/`, `docs/` e `CLAUDE.md`), `proibidos.test.mjs`, `manifesto.test.mjs`, `caminhos-lab.test.mjs`, `testes-no-guest.test.mjs` e `travas.test.ps1`. `onde.test.mjs`, `motivo-recusa.test.mjs` e `sandbox.test.mjs` são puros mas **não** estão no CI.
- **Sandbox (criam ou apagam pastas):** `comum.test.mjs`, `novo.test.mjs`, `mock.test.mjs`, `copia.test.mjs`, `publicar.test.mjs` e `sandbox-lab/scripts/test/travas.sandbox.ps1`. Rodam **só no guest**: `test-up.ps1 -Config lab.config.app-build.json -Project <projeto>` e depois `node .claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs --node <arquivos>` ou `--ps sandbox-lab/scripts/test/travas.sandbox.ps1`. Fora do Sandbox, `tempDir()` e o cabeçalho do `.sandbox.ps1` recusam antes de criar qualquer pasta.

```
node --test <arquivo1>.test.mjs <arquivo2>.test.mjs
```

Glob e `node --test <pasta>/` não são permitidos (a forma de pasta falha no Node 24 e o glob incluiria testes de Sandbox).

## Saída

Diga o cenário, o perfil, o que foi publicado (exe ou instalador), o que foi feito, o que foi visto (print indicando o degrau da escada, banco) e o veredito com evidência. Sem Sandbox disponível, diga o que faltou em vez de simular.

## Mais detalhe

- `references/limites.md`: comportamentos do laboratório que afetam o teste e evals de disparo
- `references/pastas-de-dados.md`: o que fica em `%APPDATA%` e `%LOCALAPPDATA%` do app
