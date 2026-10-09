# Laboratório do Sandbox dentro do claudia-rh

> **Status: APROVADA (respostas do usuário em 2026-10-07, ver "Decisões do usuário") e implementada (integrada em `e242f28`).** Nenhum código antes da aprovação foi escrito.

Data: 2026-10-07. Branch: `feat/laboratorio-no-repo` (criada a partir de `feat/browser-cdp-mcp`, commit `595bada`).

## Decisões do usuário (2026-10-07)

Respostas aos pontos em aberto. Valem sobre as propostas abaixo. **A spec continua aguardando a aprovação final do usuário antes de qualquer código ou cópia.**

| # | Ponto | Decisão |
|---|---|---|
| L1 | Lugar | `sandbox-lab/` na raiz do claudia-rh. Scripts e configs versionados; ferramentas, chave, saídas e evidências ignoradas. |
| L2 | Ferramentas pesadas (~592 MB) | Copiar uma vez agora para a pasta ignorada, com uma lista de SHA-256 versionada para conferência. O script de download vira issue para depois. |
| L3 | Variáveis de ambiente | Nenhuma. A skill acha tudo pela raiz do repositório. |
| L4 | Worktrees | Laboratório do checkout principal; projeto testado é o da worktree onde o teste roda. |
| L5 | Chave do MCP (`shared/secrets`) | Copiar a mesma chave para a pasta ignorada. Nunca entra no git. |
| L6 | Quem faz a cópia | Um agente, com travas: destino novo e vazio, sem `/MIR`, `/MOV` nem `/PURGE`, nada apagado, conferência por hash de cada arquivo, original intacto. |
| L7 | Skills | Juntar a `sandbox-lab` como referência da `claudia-teste-sandbox`; trazer as `winapp-*` para `.claude/skills/` com a licença MIT. |
| L8 | Saídas antigas (`shared/out`, `evidence`) | Não copiar. A pasta nova começa limpa; as antigas ficam no lugar original. |
| L9 | Branch | Entra na `feat/browser-cdp-mcp`, como pré-requisito para fechar essa branch. |

## 1. Pedido do usuário

Nas palavras dele:

> "Eu acho que nao era isso voce estava usando caminhos meus como pycharm/ etc... usar somente claudia-rh entendeu?"

E a resposta dele à proposta de trazer o laboratório para o repositório:

> "Sim, para dentro do claudia-rh"

## 2. Leitura do pedido

- **"caminhos meus como pycharm/ etc"**: hoje a skill `claudia-teste-sandbox` depende de um laboratório que mora fora do repositório, numa pasta pessoal do usuário. Ela acha essa pasta pela variável `SANDBOX_LAB_DIR`, e o app e a pasta de trabalho por `LAB_APP_PROJECT` e `LAB_WORK_DIR`. Essas variáveis apontam para caminhos da máquina dele.
- **"usar somente claudia-rh"**: tudo o que o teste em Sandbox precisa deve estar dentro do repositório `claudia-rh`, achado a partir da raiz do repositório. Sem variável de caminho e sem caminho pessoal.
- **"para dentro do claudia-rh"**: o laboratório vem para dentro do repositório. Scripts e configurações passam a ser versionados. O que é pesado ou secreto fica no repositório, mas fora do git.

Nada além disso foi pedido. Todo o resto abaixo é proposta.

## 3. O que existe hoje (só leitura, 2026-10-07)

A pasta atual do laboratório fica fora do repositório, irmã do workspace. **Não é repositório git**: não tem histórico nem backup.

| Parte | Tamanho | Observação |
|---|---|---|
| `shared/tools/` | ~592 MB, 2.599 arquivos | ferramentas pré-baixadas para o guest (detalhe abaixo) |
| `scripts/` | KB | `test-up.ps1`, `test-down.ps1`, `app-stage.ps1`, `mcp-sync.ps1`, `validate-config.ps1` |
| `sandbox/` | KB | modelo do `.wsb`, `bootstrap.ps1`, `bootstrap-mcp.ps1`, `serve-lab.ps1`, `app/guest-*.ps1`; mais dois arquivos gerados (`sandbox.generated.wsb`, `guest.config.json`) |
| `lab.config.json`, `lab.config.app.json`, `lab.config.app-winapp.json`, `lab.config.app-build.json` | KB | perfis do Sandbox |
| `.claude/skills/` | KB | `sandbox-lab`, `winapp-sandbox`, `winapp-ui-automation` e `WINAPP-LICENSE.txt` (MIT, Microsoft) |
| `CLAUDE.md`, `OBJECTIVES.md`, `.mcp.json`, `.gitignore` | KB | |
| `sample-project/` | KB | projeto de exemplo do perfil padrão |
| `shared/db/`, `shared/extensions/`, `shared/mcp.json`, `shared/test-page.html` | KB | banco de exemplo com hash conferido, extensão de exemplo, config do MCP do guest |
| `shared/out/` | KB | logs do último boot (gerados) |
| `shared/secrets/` | 1 arquivo | **não foi aberto**. É a chave do MCP do guest. |
| `evidence/` | vazia | |

`shared/tools/` por subpasta:

| Subpasta | Tamanho | Arquivos | O que é |
|---|---|---|---|
| `chrome` | 433,6 MB | 306 | Chrome for Testing `154.0.8037.92` (a mesma versão do `src-tauri/browser-pin.json`). Não é o Chrome pessoal do usuário. |
| `python` | 58,9 MB | 2.195 | CPython 3.14.7 portátil |
| `wheels` | 44,8 MB | 92 | pacotes Python para instalar o `windows-mcp` 0.8.7 sem rede |
| `uv` | 40,8 MB | 2 | `uv.exe`, `uvx.exe` |
| `sqlite` | 14,2 MB | 4 | `sqlite3.exe` e ferramentas |

Não existe script que baixa essas ferramentas. Foram baixadas à mão. A origem exata de cada uma não está registrada.

O `.gitignore` atual do laboratório já exclui: `shared/secrets/`, `shared/tools/`, `shared/out/`, `evidence/`, `sandbox/sandbox.generated.wsb`, `sandbox/guest.config.json`.

### 3.1 Caminhos pessoais e segredos nos arquivos que seriam versionados

Conferido por busca de texto, sem abrir `shared/secrets/` nem `shared/tools/`:

- **Scripts, perfis, skills, `.mcp.json`, `shared/mcp.json`: nenhum caminho pessoal escrito.** Os perfis usam variáveis (`%LAB_APP_PROJECT%`, `%LAB_WORK_DIR%`) e, no perfil de build, `%USERPROFILE%`, `%ProgramFiles%` e `%ProgramFiles(x86)%` para as toolchains do host. São variáveis, não caminhos escritos.
- **`.mcp.json`: sem segredo.** Usa `${LAB_MCP_AUTH_KEY}`, expandido do ambiente.
- **`sandbox/sandbox.generated.wsb` tem caminhos pessoais** (a pasta do laboratório, o repositório e uma pasta de trabalho em outro disco). É gerado a cada `test-up` e já está no `.gitignore`. Continua ignorado.
- **`sandbox/guest.config.json`**: só caminhos do guest. Gerado e ignorado.
- **Riscos de segredo indireto:**
  - `scripts/mcp-sync.ps1 -Literal` grava a chave do MCP **em texto puro** no `.mcp.json`. Se o `.mcp.json` for versionado, a chave vai para o git.
  - `scripts/mcp-sync.ps1` (sem `-Literal`) reescreve o `.mcp.json` com o IP do guest a cada boot. Se versionado, suja o `git status` toda vez.
  - `sandbox/serve-lab.ps1` serve toda a pasta `C:\lab` por HTTP no guest, e a chave fica em `C:\lab\secrets`. Só dentro do guest, em `localhost`. Fica registrado; não muda nesta spec.
- **O app em teste é montado inteiro no guest.** No último uso, `LAB_APP_PROJECT` apontava para a raiz do repositório. Com o laboratório dentro do repositório, a raiz montada passaria a conter as ferramentas, a chave e a pasta de trabalho. E o `guest-build.ps1` copia o projeto para o disco do guest excluindo só `node_modules`, `target`, `dist` e `.git`: copiaria também os ~592 MB e a chave.

### 3.2 Onde os scripts apagam

| Onde | O quê |
|---|---|
| `scripts/app-stage.ps1` | com `-Clean`, `Remove-Item <work-dir>\deploy -Recurse -Force`, depois `robocopy /MIR`. É o B3 da auditoria de 2026-10-06. |
| `scripts/test-up.ps1` | apaga tudo dentro de `shared\out` (menos `.gitkeep`) a cada subida. |
| `sandbox/app/guest-run.ps1`, `guest-build.ps1`, `bootstrap-mcp.ps1` | `robocopy` (um com `/MIR`) **dentro do guest**. Não alcançam o host. |
| `.claude/skills/claudia-teste-sandbox/scripts/publicar.mjs` | sempre chama `app-stage.ps1` com `-Clean`. |

## 4. Propostas do assistente (NÃO DECIDIDAS)

Cada proposta traz opções e uma recomendação. Nada disto foi aprovado.

### P1. Onde o laboratório fica e o que entra no git

**Opções de lugar:**

- **A. `sandbox-lab/` na raiz do repositório.** Fica à vista, separado do app e das skills.
- **B. Dentro da skill** (`.claude/skills/claudia-teste-sandbox/lab/`). Junta tudo num lugar, mas põe ~592 MB ignorados dentro de `.claude/skills/`, que o Claude Code varre à procura de skills.
- **C. `tools/sandbox-lab/`.** Igual a A, um nível abaixo.

**Recomendação: A, `sandbox-lab/`.** Nome em inglês, como pede a convenção para identificadores novos.

**O que é versionado:** `scripts/`, `sandbox/` (sem os gerados), os quatro `lab.config*.json`, `OBJECTIVES.md`, `sample-project/`, `shared/db/`, `shared/extensions/`, `shared/mcp.json`, `shared/test-page.html`, o `.gitignore` do laboratório.

**O que fica ignorado:** `shared/tools/`, `shared/secrets/`, `shared/out/`, `evidence/`, `sandbox/sandbox.generated.wsb`, `sandbox/guest.config.json` e **o `.mcp.json`** (é gerado pelo `mcp-sync.ps1` com o IP do boot).

**Garantia de que `shared/secrets` nunca entra no git**, em camadas:

1. **Duas regras de ignore**, de propósito redundantes: o `.gitignore` dentro de `sandbox-lab/` e uma linha explícita `sandbox-lab/shared/secrets/` no `.gitignore` da raiz. A regra `*.key` que já existe na raiz cobre também o arquivo atual.
2. **Tirar a opção `-Literal` do `mcp-sync.ps1`.** É a única forma de a chave ir parar num arquivo fora de `shared/secrets/`.
3. **Uma função pura de conferência** (nome provisório `arquivosProibidos`): recebe a lista de arquivos versionados, em texto, e devolve os que não podem estar ali (qualquer coisa em `shared/secrets/` ou `shared/tools/`, `*.key`, `.mcp.json` do laboratório, `sandbox.generated.wsb`, `guest.config.json`). O teste dela usa listas escritas no próprio teste. Não lê disco.
4. **Um passo no CI** (`.github/workflows/ci.yml`) que roda `git ls-files` e passa a saída para essa função. Só lê.
5. **Hook de pre-commit: não recomendado agora.** O git não instala hook sozinho. Cada máquina teria de configurar, e isso é um passo a mais que se esquece. O CI pega o mesmo erro.

**Caminhos pessoais nos versionados:** uma segunda função pura (nome provisório `achaCaminhoPessoal`) recebe um texto e diz se há caminho de máquina (`<letra>:\Users\`, a pasta Users do macOS, pasta de projetos da IDE, pasta de usuário). Teste com textos fixos. O mesmo passo do CI aplica a função aos arquivos versionados de `sandbox-lab/` e `.claude/skills/`.

**Como os perfis deixam de usar variável:** `projectPath` e as pastas extras (`deploy`, `guest-rw`) passam a ser caminhos **relativos** (o `test-up.ps1` já resolve caminho relativo a partir da pasta do laboratório). O perfil de build continua usando `%USERPROFILE%` e `%ProgramFiles%` para achar as toolchains do host: são variáveis padrão do Windows, não caminho pessoal escrito. Ver P4 para o caso de worktree.

**Proteger o guest de ver o que não precisa:** o `guest-build.ps1` passa a excluir também `sandbox-lab\shared`, `.sandbox-work` e `.claude` ao copiar o projeto.

### P2. As ferramentas pesadas (`shared/tools`)

**Opções:**

- **A. Script reprodutível que baixa.** Um manifesto versionado diz, para cada ferramenta, a URL, a versão e o SHA-256. O script baixa, confere o hash e extrai na pasta ignorada. O Chrome for Testing pode reaproveitar a versão e o hash do `src-tauri/browser-pin.json`.
- **B. Cópia única à mão** da pasta atual para a pasta ignorada. Junto, um **manifesto versionado só de conferência**: lista de arquivos com tamanho e SHA-256. Não baixa nada.
- **C. As duas, em duas etapas.** B agora; A num ciclo próprio.

**Recomendação: C.** B agora porque não mexe em rede nem escreve fora da pasta ignorada, e o laboratório volta a funcionar logo. A origem de `python`, `uv`, `wheels` e `sqlite` não está registrada: o script de download (A) precisa de pesquisa e de spec própria. Registrar A no Linear.

**Onde ficam:** `sandbox-lab/shared/tools/`, ignorada. O guest continua vendo em `C:\lab\tools\`. Nenhum caminho do guest muda.

**Se faltarem:** o `validate-config.ps1` e o `test-up.ps1` param antes de subir o Sandbox, com uma mensagem que diz **qual ferramenta falta** e **como obtê-la** (copiar à mão, ou o script de A quando existir). Nunca baixam nada sozinhos e nunca sobem um Sandbox pela metade. A comparação "o que falta" é uma função pura (manifesto + lista de arquivos encontrados → lista do que falta ou difere).

### P3. Pasta de trabalho do Sandbox dentro do repositório

**Proposta:** `.sandbox-work/` na raiz do repositório, ignorada. Dentro dela:

- `deploy/`: o que vai para `C:\appdeploy` (somente leitura no guest);
- `guest-rw/`: o que vira `C:\labwork` (gravável no guest).

**Relação com o `-Clean` (B3).** O `deploy/` só é apagado se **todas** as condições forem verdadeiras:

1. o caminho, resolvido de verdade (sem atalho, sem junction), é **exatamente** `<raiz>/.sandbox-work/deploy`. Lista branca, não lista negra;
2. nem a pasta nem nenhuma pasta acima dela até a raiz do repositório é ponto de nova análise (junction, link simbólico);
3. existe dentro dela o marcador `.lab-deploy`, criado pelo próprio `publicar` na primeira vez.

Faltou uma: não apaga, e diz qual faltou. A mesma trava vale nos dois lados: `publicar.mjs` e `app-stage.ps1`. A decisão (apaga ou não) é uma função pura que recebe os fatos já lidos e devolve o motivo da recusa, como pede o B1.

**`--work-dir`:** sai. Uma pasta de trabalho só. Se o usuário quiser manter a opção, ela só aceita subpastas de `.sandbox-work/`.

**`shared\out` do `test-up.ps1`:** recebe a mesma trava (caminho exato dentro do laboratório, sem junction). Opcional, não recomendado agora: mover `shared/out` e `evidence/` para dentro de `.sandbox-work/`, para que tudo o que é gerado fique numa pasta só. Mexe em mais scripts; fica como ponto em aberto.

**Espaço em disco:** a pasta de trabalho passa a ficar no mesmo disco do repositório. No último uso ela estava em outro disco. O disco virtual do Sandbox já fica no C: de qualquer jeito (limite 10). O app publicado soma dezenas de MB, não GB. Continua valendo conferir o espaço livre antes do `test-up`.

### P4. Como a skill acha tudo pela raiz do repositório

**Opções:**

- **A. Sem variável nenhuma.** Tudo derivado da raiz do repositório.
- **B. Raiz do repositório por padrão, variáveis só como sobrescrita opcional.**

**Recomendação: A.** O pedido foi "usar somente claudia-rh". Uma sobrescrita opcional é a porta por onde o caminho pessoal volta.

**O problema da worktree.** Os agentes trabalham em worktrees (`.claude/worktrees/<nome>/`). Cada worktree tem a sua cópia dos arquivos versionados, mas **não** tem as pastas ignoradas: ficaria sem as ferramentas, sem a chave e sem a pasta de trabalho. Copiar 592 MB por worktree não faz sentido.

**Proposta:**

- **Laboratório** (scripts, ferramentas, chave) e **pasta de trabalho** = sempre os do **checkout principal** do repositório. A skill acha o checkout principal pelo próprio git (`git rev-parse --git-common-dir`, só leitura). Nenhum caminho escrito.
- **Projeto em teste** (o que hoje é `LAB_APP_PROJECT`) = a raiz do checkout **atual** (worktree ou principal). A skill passa esse caminho ao `test-up.ps1` por um parâmetro novo (nome provisório `-Project`), que tem prioridade sobre o `projectPath` do perfil.
- Consequência aceita: mudança nos scripts do laboratório feita numa worktree só vale depois de integrada. Isso combina com a regra da seção 8 da skill ("nunca mude o ferramental no meio de um teste").

**O que muda nos scripts (só descrição):**

- `comum.mjs`: `labDir()` deixa de ler `SANDBOX_LAB_DIR`. Passa a calcular `<checkout principal>/sandbox-lab` e a conferir que `scripts/app-stage.ps1` existe. Ganha uma função para a pasta de trabalho (`<checkout principal>/.sandbox-work`). As duas viram funções puras sobre texto de caminho, com a leitura do git separada.
- `publicar.mjs`: sem `LAB_WORK_DIR`, sem `--work-dir` (ou restrito, P3). Cria o marcador `.lab-deploy`. Trava do `-Clean` (P3).
- `banco-guest.mjs`: sem `LAB_WORK_DIR` e sem `--work-dir`; lê de `<pasta de trabalho>/guest-rw/db/`.
- `cdp-guest.mjs`, `cenario-*.mjs`: sem mudança de caminho (não usam o laboratório no host).
- `sandbox-lab/scripts/test-up.ps1`: parâmetro `-Project`; perfis com caminhos relativos; trava em `shared\out`; mensagem clara se faltar ferramenta.
- `sandbox-lab/scripts/app-stage.ps1`: `-WorkDir` com padrão na pasta de trabalho do repositório, sem `LAB_WORK_DIR`; trava do `-Clean`.
- `sandbox-lab/scripts/mcp-sync.ps1`: sem `-Literal`.
- `sandbox-lab/sandbox/app/guest-build.ps1`: exclusões a mais na cópia (P1).
- Testes que hoje passam `SANDBOX_LAB_DIR` (`publicar.test.mjs`, `comum.test.mjs`) mudam junto. Ver seção 8.

**Docs que mudam junto:** `CLAUDE.md` (a regra "Nada de caminho absoluto" deixa de citar `SANDBOX_LAB_DIR`; o parágrafo de "Teste em máquina limpa" passa a dizer que o laboratório está em `sandbox-lab/`), `SKILL.md` da skill (seções 1, 2, 5 e 6) e `references/limites.md` ("a partir de `%SANDBOX_LAB_DIR%`"). Specs e planos antigos ficam como estão: são histórico.

### P5. Migração: copiar, nunca mover

1. **Antes de copiar**, commit das regras de ignore (P1) na branch. Assim, nada pesado ou secreto aparece no `git status` em momento nenhum.
2. **Conferir espaço livre** no disco do repositório (~650 MB com folga).
3. **Copiar** da pasta atual do laboratório para `sandbox-lab/` no checkout principal. Cópia **sem** espelhamento e sem opção de mover ou apagar (nada de `/MIR`, `/MOV`, `/PURGE`, `Move-Item`, `Remove-Item`). O destino tem de não existir ou estar vazio. A origem só é lida.
4. **Não copiar** o que é gerado: `shared/out/`, `evidence/`, `sandbox/sandbox.generated.wsb`, `sandbox/guest.config.json`, `.mcp.json`. Eles se recriam no primeiro `test-up`.
5. **A chave (`shared/secrets/`)**: opções:
   - **a)** copiar o arquivo como está (fica ignorado pelas duas regras);
   - **b)** não copiar e gerar uma chave nova. Não foi achado no laboratório o script que gera a chave (o `validate-config.ps1` diz "gerado pelo setup do MCP"): **não verificado** como gerar.
   - **Recomendação: a)**, conferindo com `git check-ignore` **antes** de qualquer `git add`.
6. **`git add` só com a lista explícita de arquivos.** Nunca `git add -A` nem `git add sandbox-lab/` nesta migração. Depois, `git status --ignored` para ver que `tools/` e `secrets/` aparecem como ignorados.
7. **Conferir que a cópia ficou completa**, só lendo os dois lados: para cada arquivo da origem (menos os do item 4), o mesmo caminho relativo existe na cópia, com o mesmo tamanho e o mesmo SHA-256. Resultado esperado: zero diferenças; contagem da origem e da cópia batendo (~2.599 em `tools/` e ~40 no resto). A comparação das duas listas é função pura. Para a chave, compara-se só o hash; o conteúdo não é exibido.
8. **O original fica onde está.** Só sai se o usuário decidir depois, à mão. Nenhum script e nenhum agente apaga a pasta antiga.

Quem copia: um agente, depois da aprovação, com o comando mostrado ao usuário antes. Ou o próprio usuário. Ponto em aberto.

### P6. O `CLAUDE.md` e a skill do laboratório dentro do claudia-rh

O laboratório traz um `CLAUDE.md` próprio, a skill `sandbox-lab` e duas skills do winapp. O `.gitignore` da raiz só versiona skills em `.claude/skills/` da raiz. Skills em `sandbox-lab/.claude/skills/` não são achadas quando o Claude Code roda na raiz do repositório (**não verificado**: não foi testado se o Claude Code acha skills em pastas `.claude/` de subpastas).

**Opções:**

- **A. Juntar.** O conteúdo da `sandbox-lab/SKILL.md` vira uma referência da skill existente (`.claude/skills/claudia-teste-sandbox/references/laboratorio.md`). As skills `winapp-sandbox` e `winapp-ui-automation` vão para `.claude/skills/` da raiz, com o `WINAPP-LICENSE.txt` ao lado. O `CLAUDE.md` do laboratório vira `sandbox-lab/README.md`.
- **B. Manter como está**, dentro de `sandbox-lab/.claude/` e `sandbox-lab/CLAUDE.md`, usados só quando alguém abre o Claude Code dentro de `sandbox-lab/`.

**Recomendação: A.** Um ponto de entrada só (o `CLAUDE.md` da raiz), uma skill só para o Sandbox. Regras:

- Onde o texto do laboratório e o do repositório divergem, **vale o do repositório** (regras duras do `CLAUDE.md`).
- O `README.md` do laboratório diz o que é e aponta para a skill `claudia-teste-sandbox`.
- As skills do winapp entram no repositório: medir de novo os evals de disparo da `claudia-teste-sandbox` e da `claudia-teste-e2e`, porque descrições novas podem roubar o disparo (seção "Evals de disparo" de `limites.md`). Os evals continuam manuais e só com autorização (B6, CLA-56).
- O `OBJECTIVES.md` (objetivos O1 a O10 do laboratório genérico) fica em `sandbox-lab/`, sem mudança.

### P7. Limites conhecidos e o que não muda

Não muda nada disto:

- **CDP não abre no guest.** A porta de depuração do WebView2 não responde (release e debug). Trazer o laboratório para o repositório não resolve isso.
- **UIA pelo winapp continua sendo o caminho** para olhar o app no guest (`-d 30`, só navegação).
- Só um Sandbox por vez. `wsb exec` não devolve a saída e sempre sai com 0 no host. Disco virtual no C:.
- Caminhos **do guest** (`C:\lab`, `C:\labsandbox`, `C:\labproject`, `C:\appdeploy`, `C:\labwork`) ficam iguais. São do Sandbox, não da máquina do usuário.
- Cenários (`novo`, `mock`, `copia`), a autorização do `copia` e as regras da seção 0 da skill.
- O Chrome do guest continua sendo o Chrome for Testing. Nunca o Chrome pessoal do usuário.

### P8. Testes

- **No PC do usuário, só funções puras**, com dados escritos no próprio teste, sem tocar o disco: `arquivosProibidos`, `achaCaminhoPessoal`, a decisão do `-Clean` (motivo da recusa), "o que falta" das ferramentas, a comparação das listas da cópia, o cálculo dos caminhos do laboratório e da pasta de trabalho a partir de um texto de raiz.
- **Todo teste que cria ou apaga pasta roda só no Windows Sandbox.** Isso inclui o `publicar.test.mjs` de hoje (cria pastas temporárias e um `app-stage.ps1` falso) e qualquer teste da trava do `-Clean` com pastas reais. No guest, esses testes usam pastas em `C:\labdata`, nunca as pastas mapeadas do host.
- `node --test` sempre com a lista de arquivos explícita.
- Prova de mutação para cada trava nova: quebrar a trava, ver o teste nomeado falhar, restaurar.
- `comum.test.mjs` continua proibido até a Tarefa 0b (B1).

## 5. Impacto

- **Dados do usuário:** nenhum. Nada toca `%APPDATA%` nem `%LOCALAPPDATA%` do app.
- **Telas e app:** nenhum. Só ferramental de teste.
- **Ferramentas do agente:** nenhuma.
- **Segurança:** melhora (menos variáveis de caminho, trava do `-Clean`, conferência de segredo no CI). Risco novo: a chave e ~592 MB passam a morar dentro da pasta do repositório. Mitigado por P1.
- **Repositório:** ~70 arquivos pequenos versionados a mais. Os ~592 MB não entram no git.
- **IDE:** a pasta ignorada com 2,6 mil arquivos pode ser indexada pelo PyCharm. O `tsconfig.json` só inclui `src`; Rust e Vite não olham `sandbox-lab/` (**não verificado** para o Vite).

## 6. Critérios de aceite

1. `git ls-files` não lista nada de `sandbox-lab/shared/tools/`, `sandbox-lab/shared/secrets/`, nem `*.key`, nem o `.mcp.json` do laboratório. Conferido pela função pura no CI.
2. Nenhum arquivo versionado de `sandbox-lab/` ou `.claude/skills/` contém caminho de máquina (função pura no CI).
3. Nenhum script da skill lê `SANDBOX_LAB_DIR`, `LAB_APP_PROJECT` ou `LAB_WORK_DIR` (busca de texto).
4. A partir de uma worktree, a skill acha o laboratório e a pasta de trabalho do checkout principal e monta a worktree como projeto. Verificado num Sandbox real.
5. O `-Clean` recusa, no Sandbox, cada caso: caminho diferente do exato, junction, sem marcador. Apaga só o caso válido.
6. Cópia da migração com zero diferenças de caminho, tamanho e hash. Pasta original intacta (mesma listagem antes e depois).
7. Fluxo típico da skill de ponta a ponta no Sandbox (cenário `mock` → `publicar` → `test-up` → app aberto → UIA → `test-down`) funcionando com o laboratório novo.

## 7. Fora do escopo

- Script que baixa as ferramentas (P2-A): ciclo próprio.
- Resolver o CDP no guest.
- Apagar a pasta antiga do laboratório.
- Mudar os scripts do guest além da exclusão no `guest-build.ps1`.
- Reescrever specs e planos antigos que citam `SANDBOX_LAB_DIR`.

## 8. Pontos em aberto (precisam do usuário)

1. Lugar: `sandbox-lab/` na raiz (recomendado) ou outro?
2. Ferramentas: cópia à mão agora e script de download depois (recomendado), ou já o script?
3. Sem variável nenhuma (recomendado) ou com sobrescrita opcional?
4. Laboratório e pasta de trabalho sempre do checkout principal, projeto do checkout atual (recomendado para worktrees). De acordo?
5. Chave do MCP: copiar (recomendado) ou gerar uma nova?
6. Quem faz a cópia: um agente, com o comando mostrado antes, ou o próprio usuário?
7. Juntar a skill `sandbox-lab` na `claudia-teste-sandbox` e trazer as skills do winapp para a raiz (recomendado), ou manter tudo dentro de `sandbox-lab/`?
8. Mover `shared/out` e `evidence/` para `.sandbox-work/` agora, ou depois?
9. A branch saiu de `feat/browser-cdp-mcp`, não de `dev`, porque depende das regras de deleção de lá. Integrar depois dela?

## 9. Não verificado

- Se o Claude Code acha skills e `CLAUDE.md` em subpastas `.claude/` de um repositório.
- Como gerar uma chave nova do MCP (o script não foi achado).
- A origem e as URLs de `python`, `uv`, `wheels` e `sqlite`.
- O conteúdo de `shared/db/backup.sqlite` (o `OBJECTIVES.md` diz que é exemplo; não foi aberto).
- Montar no guest uma pasta (`.sandbox-work/guest-rw`) que fica dentro de outra pasta montada como somente leitura (a raiz do repositório em `C:\labproject`). O laboratório já faz isso com `shared` e `shared\out`, mas não com o projeto.
- Se o Vite ou outra ferramenta de build varre `sandbox-lab/`.
- Nada foi executado: nenhum teste, script ou cópia. Só leitura de arquivos e listagem.
