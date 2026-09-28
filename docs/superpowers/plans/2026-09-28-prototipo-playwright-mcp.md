# Protótipo Playwright MCP — Plano de Implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Responder às quatro perguntas em aberto da CLA-7 com números medidos, e produzir uma recomendação de seguir ou parar.

**Architecture:** Protótipo standalone. As quatro perguntas são sobre o `@playwright/mcp` e sobre o Chrome — nenhuma é sobre o claudia-rh. Por isso as Tarefas 1 a 4 não tocam em nenhum arquivo do app: rodam `npx @playwright/mcp` direto, numa pasta descartável. Só a Tarefa 5 integra, e o que ela integra é reversível em uma linha. A Tarefa 6 escreve os achados.

**Tech Stack:** `@playwright/mcp` (Node, via `npx`), Chrome 153 já instalado, Node já presente na máquina.

**Spec:** `docs/superpowers/specs/2026-09-27-browser-como-mcp-roadmap.md` (§3 e Fase 1)

## Natureza deste plano

**Este plano não entrega software. Entrega respostas.**

O código escrito aqui é descartável e fica marcado como tal — é o que a CLA-7 exige (*"Protótipo feito aqui é descartável e fica marcado como tal"*). Por isso as tarefas não seguem o ciclo TDD normal: o "teste" de cada tarefa é uma **observação medida**, e o critério de aceitação é um número ou um sim/não, não um assert verde.

Se no fim as respostas forem desfavoráveis, o resultado correto é apagar tudo e fechar a CLA-7 com "não compensa". Isso é sucesso, não falha.

## Global Constraints

- **Comentários de código: inglês.** Sempre. (`CLAUDE.md`)
- **Commit messages: inglês.** (`CLAUDE.md`)
- **Identificadores novos: inglês.** (`CLAUDE.md`)
- **Docs e achados: pt-BR.** (`CLAUDE.md`)
- **NUNCA mudar o `identifier` Tauri** em `src-tauri/tauri.conf.json`. (`CLAUDE.md`)
- **Nenhum arquivo de `src-tauri/src/` é modificado nas Tarefas 1–4.** Só a Tarefa 5 toca no app.
- **`prompt_sistema_runtime.md` não é tocado em tarefa nenhuma.** É protegido por `CLAUDE.md` e não faz parte deste protótipo.
- **Todo o artefato do protótipo vive em `prototipo-playwright/`**, na raiz do repo, e essa pasta entra no `.gitignore` exceto pelos arquivos de medição.
- **As credenciais do LinkedIn são do usuário.** Nenhuma tarefa pede, lê, guarda ou digita senha. Os passos de login são executados pelo usuário, à mão, e o plano marca-os como tal.

## Review Focus

Modos de falha que tornariam os achados **falsos sem parecerem falsos** — cada um tem verificação na tarefa que o possui:

1. **Medir uma página que não carregou.** Um snapshot de página em branco é minúsculo e faria a economia parecer espetacular. Tarefa 3 verifica que a página tem conteúdo real antes de medir.
2. **Medir uma vaga simples em vez de uma real.** Uma página de descrição estática não representa o fluxo: o custo vem do formulário Easy Apply, com múltiplos passos. Tarefa 3 mede o fluxo com modal aberto, não só a descrição.
3. **Testar persistência de sessão rápido demais.** Fechar e reabrir no mesmo minuto pode passar por cache em memória e não prova nada. Tarefa 2 exige encerrar o processo e reabrir depois de um intervalo real.
4. **Confundir `npx` frio com quente.** A primeira execução baixa o pacote; as seguintes não. Tarefa 1 mede as duas separadamente e reporta as duas.
5. **Concluir de uma amostra só.** Uma página pode ser atípica. Tarefas 2 e 3 usam três vagas diferentes, não uma.

---

## Estrutura de arquivos

| Arquivo | Responsabilidade |
|---|---|
| `prototipo-playwright/.gitignore` | ignora perfil, `node_modules` e tudo menos as medições |
| `prototipo-playwright/README.md` | marca a pasta como descartável e diz como apagá-la |
| `prototipo-playwright/mcp-config-teste.json` | config MCP só do protótipo, isolada da do app |
| `prototipo-playwright/medicoes/` | os números crus de cada tarefa, em arquivos datados |
| `prototipo-playwright/ACHADOS.md` | as quatro respostas e a recomendação |

Nada em `src-tauri/` ou `src/` é criado ou modificado, exceto a Tarefa 5.

---

### Task 1: Ambiente do protótipo e custo do `npx` (responde Q4)

**Files:**
- Create: `prototipo-playwright/.gitignore`
- Create: `prototipo-playwright/README.md`
- Create: `prototipo-playwright/medicoes/01-npx.md`

**Interfaces:**
- Consumes: nada.
- Produces: a pasta `prototipo-playwright/` e o fato medido de quanto custa `npx @playwright/mcp` a frio e a quente. A Tarefa 5 reutiliza o caminho da pasta.

- [ ] **Step 1: Criar a pasta e marcá-la como descartável**

```bash
mkdir -p prototipo-playwright/medicoes
```

`prototipo-playwright/README.md`:

```markdown
# Protótipo descartável — Playwright MCP

Esta pasta existe para responder às quatro perguntas em aberto da CLA-7.
**Não faz parte do app.** Nada aqui é importado por `src/` ou `src-tauri/`.

Para apagar quando o estudo fechar:

    rm -rf prototipo-playwright/

Os achados ficam registrados na CLA-7 antes de apagar. Ver `ACHADOS.md`.
```

`prototipo-playwright/.gitignore`:

```gitignore
# Tudo é descartável exceto as medições e a documentação.
*
!.gitignore
!README.md
!ACHADOS.md
!mcp-config-teste.json
!medicoes/
!medicoes/*.md
```

- [ ] **Step 2: Medir `npx` a frio**

Limpar o cache do npx para garantir que a medição é mesmo a frio:

```bash
npm cache verify >/dev/null 2>&1
rm -rf ~/.npm/_npx 2>/dev/null
```

Medir:

```bash
cd prototipo-playwright
START=$(date +%s)
npx -y @playwright/mcp@latest --help > /dev/null 2>&1
END=$(date +%s)
echo "npx frio: $((END-START))s"
du -sh ~/.npm/_npx 2>/dev/null | cut -f1
```

Expected: um tempo em segundos e um tamanho em disco. Ambos vão para o arquivo de medição.

- [ ] **Step 3: Medir `npx` a quente**

```bash
cd prototipo-playwright
START=$(date +%s)
npx -y @playwright/mcp@latest --help > /dev/null 2>&1
END=$(date +%s)
echo "npx quente: $((END-START))s"
```

Expected: substancialmente menor que o frio. Se não for, o cache não está sendo usado e isso é um achado por si só.

- [ ] **Step 4: Confirmar que não baixou browser**

O requisito do usuário é "não adicionar nenhum setup a mais". Verificar que `--browser chrome` usa o Chrome instalado:

```bash
du -sh ~/.cache/ms-playwright 2>/dev/null || echo "sem browsers baixados"
ls "$LOCALAPPDATA/ms-playwright" 2>/dev/null || echo "sem browsers baixados (Windows)"
```

Expected: nenhum browser baixado, ou apenas o que já existisse antes. Se o Playwright baixou um Chromium de ~150 MB, o requisito "sem setup a mais" não está atendido e isso vai para os achados.

- [ ] **Step 5: Escrever a medição**

`prototipo-playwright/medicoes/01-npx.md`:

```markdown
# Q4 — A dependência de `npx` é aceitável?

**Data:** <preencher com a data real>
**Node:** <saída de `node --version`>

| Medida | Valor |
|---|---|
| `npx` a frio (primeira execução) | <N>s |
| `npx` a quente | <N>s |
| Disco em `~/.npm/_npx` | <N> MB |
| Browser baixado pelo Playwright | <sim/não, tamanho> |

## Resposta

<Uma frase: aceitável ou não, e por quê.>
```

- [ ] **Step 6: Commit**

```bash
git add prototipo-playwright/
git commit -m "prototype: measure npx cold/warm start cost for playwright mcp

Throwaway prototype for CLA-7. Answers Q4 of the four open questions."
```

---

### Task 2: Perfil dedicado persistente (responde Q1)

**Files:**
- Create: `prototipo-playwright/medicoes/02-perfil.md`

**Interfaces:**
- Consumes: a pasta `prototipo-playwright/` da Tarefa 1.
- Produces: o fato de a sessão autenticada do LinkedIn sobreviver ou não ao reinício do processo. Este é o resultado que decide se a rota tem caminho limpo — a Tarefa 6 depende dele.

**Nota de segurança:** o login é feito **pelo usuário, à mão**. Nenhum passo deste plano pede, lê, guarda ou digita credenciais.

- [ ] **Step 1: Subir o Playwright MCP com perfil dedicado**

```bash
cd prototipo-playwright
npx -y @playwright/mcp@latest \
  --browser chrome \
  --user-data-dir ./perfil-claudia \
  --port 8931
```

Isto abre um Chrome com um perfil próprio, vazio, em `prototipo-playwright/perfil-claudia/`.

**Verificação:** o Chrome abre e não trava numa página em branco. Se travar, a restrição do Chrome 136 está batendo mesmo com `--user-data-dir` próprio — o que contradiz a documentação e é um achado grave. Registrar e parar.

- [ ] **Step 2: PASSO DO USUÁRIO — login manual no LinkedIn**

Peça ao usuário para, na janela que abriu:

1. Navegar para `https://www.linkedin.com/login`
2. Entrar com as credenciais dele
3. Resolver qualquer 2FA ou captcha que apareça
4. Confirmar que chegou ao feed

**Não automatize este passo.** Não peça as credenciais. Espere o usuário confirmar que está logado.

- [ ] **Step 3: Encerrar o processo por completo**

```bash
# Ctrl+C no terminal do npx, depois confirmar que morreu:
tasklist 2>/dev/null | grep -i "chrome.exe" | head -3
```

Expected: o Chrome do protótipo fechou. O Chrome pessoal do usuário, se estiver aberto, continua — são processos diferentes com perfis diferentes.

- [ ] **Step 4: Esperar um intervalo real**

Esperar **no mínimo 10 minutos** antes de reabrir. Reabrir no mesmo minuto pode passar por cache em memória e não provaria persistência em disco.

Se houver outra tarefa para fazer no intervalo, fazer. O importante é que o tempo passe de verdade.

- [ ] **Step 5: Reabrir e verificar a sessão**

```bash
cd prototipo-playwright
npx -y @playwright/mcp@latest \
  --browser chrome \
  --user-data-dir ./perfil-claudia \
  --port 8931
```

Navegar para `https://www.linkedin.com/feed/` e observar.

Expected, se Q1 for **sim**: chega ao feed, autenticado, sem pedir login.
Expected, se Q1 for **não**: redireciona para a página de login.

- [ ] **Step 6: Repetir com três páginas diferentes**

Não concluir de uma amostra. Confirmar a sessão em três lugares que exigem autenticação:

1. `https://www.linkedin.com/feed/`
2. `https://www.linkedin.com/jobs/`
3. Uma vaga concreta com botão *Easy Apply*

Expected: os três autenticados, ou os três não. Se der misto, registrar exatamente qual falhou — isso aponta para cookie de escopo diferente, não para falha de persistência.

- [ ] **Step 7: Escrever a medição**

`prototipo-playwright/medicoes/02-perfil.md`:

```markdown
# Q1 — O perfil dedicado mantém a sessão entre execuções?

**Data:** <data real>
**Intervalo entre fechar e reabrir:** <N> minutos
**Chrome:** 153.0.8010.54

| Página | Autenticado após reinício? |
|---|---|
| `/feed/` | <sim/não> |
| `/jobs/` | <sim/não> |
| vaga com Easy Apply | <sim/não> |

## Resposta

<Uma frase. Se for "não", a rota da CLA-7 depende do modo extensão (Q2) e
a Tarefa 4 passa a ser decisiva em vez de confirmatória.>
```

- [ ] **Step 8: Commit**

```bash
git add prototipo-playwright/medicoes/02-perfil.md
git commit -m "prototype: measure dedicated profile session persistence

Answers Q1. The profile directory itself is gitignored."
```

---

### Task 3: Custo de contexto — snapshot vs screenshot (responde Q3)

**Files:**
- Create: `prototipo-playwright/medicoes/03-contexto.md`

**Interfaces:**
- Consumes: o perfil autenticado da Tarefa 2 (se Q1 deu sim; se deu não, fazer login manual outra vez antes desta tarefa).
- Produces: a razão medida entre percepção textual e percepção por pixels, comparável com a baseline de 2026-09-16 registrada na CLA-5.

- [ ] **Step 1: Escolher três vagas reais e confirmar que carregaram**

Abrir três vagas distintas do LinkedIn com *Easy Apply*. Antes de medir qualquer uma, confirmar que a página tem conteúdo real — um snapshot de página em branco é minúsculo e faria a economia parecer espetacular sem ser verdade.

Verificação por vaga: o snapshot menciona o título da vaga e o nome da empresa. Se não mencionar, a página não carregou; recarregar antes de medir.

- [ ] **Step 2: Medir o snapshot de acessibilidade**

Para cada vaga, capturar o snapshot e medir o tamanho em bytes do que volta:

```bash
# A chamada é feita via a tool browser_snapshot do Playwright MCP.
# Gravar a resposta crua num arquivo e medir:
wc -c < medicoes/raw/vaga1-snapshot.txt
```

Registrar os três valores.

- [ ] **Step 3: Medir o screenshot da mesma vaga**

Para cada vaga, no mesmo estado de página, capturar screenshot e medir o base64 que entra no contexto:

```bash
wc -c < medicoes/raw/vaga1-screenshot.b64
```

Registrar os três valores.

- [ ] **Step 4: Medir o fluxo, não só a descrição**

A descrição da vaga não representa o custo real — o gasto vem do formulário. Para **uma** das três vagas, abrir o modal *Easy Apply* e medir snapshot e screenshot em cada passo do formulário, sem submeter.

**Não submeter a candidatura.** Este é um protótipo de medição, não uma candidatura real. Fechar o modal no fim.

Registrar o total acumulado dos dois modos ao longo do fluxo.

- [ ] **Step 5: Comparar com a baseline**

A baseline está na CLA-5, medida do transcript `84b2943e` de 2026-09-16:

```
17 screenshots = 2.3 MB = 96% do contexto
texto de página extraído = 24 KB = 1%
```

Calcular a razão nova e pôr lado a lado.

- [ ] **Step 6: Escrever a medição**

`prototipo-playwright/medicoes/03-contexto.md`:

```markdown
# Q3 — Quanto cai o contexto?

**Data:** <data real>

## Por vaga, só a descrição

| Vaga | Snapshot (bytes) | Screenshot (bytes) | Razão |
|---|---|---|---|
| 1 | <N> | <N> | <N>× |
| 2 | <N> | <N> | <N>× |
| 3 | <N> | <N> | <N>× |

## Fluxo Easy Apply completo (vaga <N>, sem submeter)

| Modo | Total acumulado |
|---|---|
| Snapshots | <N> KB |
| Screenshots | <N> KB |

## Comparação com a baseline de 2026-09-16

| | Antes (CLA-5) | Agora |
|---|---|---|
| % do contexto em imagens | 96% | <N>% |

## Resposta

<Uma frase com o número. Se a economia for menor que 5×, o argumento
principal da rota enfraquece e isso tem que ficar dito.>
```

- [ ] **Step 7: Commit**

```bash
git add prototipo-playwright/medicoes/03-contexto.md
git commit -m "prototype: measure accessibility snapshot vs screenshot context cost

Answers Q3. Compares against the 2026-09-16 baseline from CLA-5."
```

---

### Task 4: A restrição do Chrome 136 e o modo extensão (responde Q2)

**Files:**
- Create: `prototipo-playwright/medicoes/04-chrome136.md`

**Interfaces:**
- Consumes: nada das tarefas anteriores.
- Produces: confirmação de que a restrição bate nesta máquina, e se o modo extensão a contorna. Só é decisiva se a Tarefa 2 tiver dado "não".

- [ ] **Step 1: Confirmar que a restrição bate mesmo**

O roadmap afirma, com base em documentação, que o Chrome ≥136 ignora `--remote-debugging-port` no perfil padrão. Confirmar na máquina:

```bash
"/c/Program Files/Google/Chrome/Application/chrome.exe" --remote-debugging-port=9222 &
sleep 5
curl -s -m 3 http://localhost:9222/json/version || echo "porta NAO abriu — restricao confirmada"
```

Expected: a porta não abre. Se abrir, a restrição não se aplica como documentado e **toda a §3 do roadmap precisa ser revista** — registrar como achado de alta prioridade.

Fechar esse Chrome antes de seguir.

- [ ] **Step 2: Confirmar que com `--user-data-dir` próprio funciona**

```bash
"/c/Program Files/Google/Chrome/Application/chrome.exe" \
  --remote-debugging-port=9222 \
  --user-data-dir="$PWD/prototipo-playwright/perfil-cdp-teste" &
sleep 5
curl -s -m 3 http://localhost:9222/json/version | head -3
```

Expected: responde com a versão. Isto confirma que o contorno documentado funciona.

Fechar e apagar `perfil-cdp-teste` depois.

- [ ] **Step 3: DECISÃO — o modo extensão é necessário?**

Se a **Tarefa 2 deu "sim"** (perfil dedicado mantém sessão): o modo extensão é desnecessário. Registrar isso e **pular os passos 4 e 5**.

Se a **Tarefa 2 deu "não"**: o modo extensão é a única porta restante. Continuar.

- [ ] **Step 4: PASSO DO USUÁRIO — instalar a extensão do Playwright**

Só se o passo 3 disse para continuar.

Peça ao usuário para instalar a *Playwright Extension* da Chrome Web Store no Chrome dele.

Avise antes: isto dá à extensão acesso às abas do browser pessoal, com as sessões autenticadas dele. É a mesma classe de permissão que a extensão do Claude tem hoje. A decisão é dele.

- [ ] **Step 5: Testar o modo extensão**

```bash
cd prototipo-playwright
npx -y @playwright/mcp@latest --extension
```

Verificar se consegue ler uma aba já aberta e autenticada do LinkedIn.

Expected, se contornar a restrição: lê a aba com a sessão real do usuário.
Expected, se não contornar: falha a conectar, ou conecta e trava.

- [ ] **Step 6: Escrever a medição**

`prototipo-playwright/medicoes/04-chrome136.md`:

```markdown
# Q2 — O modo extensão contorna a restrição do Chrome 136?

**Data:** <data real>
**Chrome:** 153.0.8010.54

## A restrição bate nesta máquina?

| Teste | Resultado |
|---|---|
| `--remote-debugging-port` no perfil padrão | <porta abriu? sim/não> |
| `--remote-debugging-port` com `--user-data-dir` próprio | <porta abriu? sim/não> |

## Modo extensão

<Testado / Não testado porque a Tarefa 2 resolveu com perfil dedicado>

| Teste | Resultado |
|---|---|
| Lê aba autenticada do Chrome pessoal | <sim/não> |

## Resposta

<Uma frase.>
```

- [ ] **Step 7: Commit**

```bash
git add prototipo-playwright/medicoes/04-chrome136.md
git commit -m "prototype: confirm Chrome 136 restriction and test extension mode

Answers Q2. Extension mode only tested if Task 2 showed dedicated
profiles do not persist the session."
```

---

### Task 5: Integração mínima e reversível

**Files:**
- Create: `prototipo-playwright/mcp-config-teste.json`
- Create: `prototipo-playwright/medicoes/05-integracao.md`
- Modify: nenhum arquivo do app.

**Interfaces:**
- Consumes: as respostas das Tarefas 1–4.
- Produces: confirmação de que os dois servidores MCP coexistem numa sessão — o `claudia` e o `playwright` — que é a única coisa que as tarefas anteriores não provam.

**Por que esta tarefa não modifica `src-tauri/`:** o app monta o `mcp-config` em tempo de execução (`commands/perfil.rs:447`). Testar a coexistência não exige mudar esse código — exige um arquivo de config paralelo, passado à mão numa sessão de teste. Se a coexistência funcionar, a mudança no app é de uma linha e pertence à Fase 2, não a este protótipo.

- [ ] **Step 1: Escrever a config de teste**

`prototipo-playwright/mcp-config-teste.json`:

```json
{
  "mcpServers": {
    "claudia": {
      "command": "<caminho do executável do app>",
      "args": ["--mcp-serve"]
    },
    "playwright": {
      "command": "npx",
      "args": [
        "-y", "@playwright/mcp@latest",
        "--browser", "chrome",
        "--user-data-dir", "<caminho absoluto>/prototipo-playwright/perfil-claudia"
      ]
    }
  }
}
```

O bloco `claudia` deve copiar exatamente o que o app gera hoje. Obter o formato real inspecionando o arquivo que o app escreve, em vez de adivinhar:

```bash
cat "$APPDATA/io.github.johngabie.claudia-rh/mcp-config.json"
```

- [ ] **Step 2: Rodar uma sessão com as duas configs**

```bash
cd prototipo-playwright
claude --mcp-config ./mcp-config-teste.json --strict-mcp-config \
  "Liste as tools MCP disponíveis, agrupadas por servidor. Não faça mais nada."
```

Expected: a resposta lista as 14 tools do `claudia` **e** as tools do `playwright`. Se só aparecer um dos dois, os servidores não coexistem e isso é um bloqueio da Fase 2.

- [ ] **Step 3: Confirmar que o servidor do claudia continua funcional**

Coexistir não basta — o servidor do app tem que continuar respondendo:

```bash
cd prototipo-playwright
claude --mcp-config ./mcp-config-teste.json --strict-mcp-config \
  "Chame get_candidate_profile e diga apenas o nome do candidato. Não altere nada."
```

Expected: devolve o nome. Se falhar, os dois servidores interferem entre si.

- [ ] **Step 4: Escrever a medição**

`prototipo-playwright/medicoes/05-integracao.md`:

```markdown
# Coexistência dos dois servidores MCP

**Data:** <data real>

| Teste | Resultado |
|---|---|
| Tools do `claudia` visíveis | <sim/não, quantas> |
| Tools do `playwright` visíveis | <sim/não, quantas> |
| `get_candidate_profile` responde | <sim/não> |

## Resposta

<Uma frase: a Fase 2 tem caminho ou está bloqueada.>
```

- [ ] **Step 5: Commit**

```bash
git add prototipo-playwright/mcp-config-teste.json prototipo-playwright/medicoes/05-integracao.md
git commit -m "prototype: verify claudia and playwright MCP servers coexist

No app files modified. Config is passed by hand to a test session."
```

---

### Task 6: Achados e recomendação

**Files:**
- Create: `prototipo-playwright/ACHADOS.md`
- Modify: nenhum arquivo de código.

**Interfaces:**
- Consumes: as cinco medições das Tarefas 1–5.
- Produces: a recomendação de seguir ou parar, registrada na CLA-7. É a entrega real deste plano.

- [ ] **Step 1: Consolidar as quatro respostas**

`prototipo-playwright/ACHADOS.md`:

```markdown
# Achados do protótipo — CLA-7 Fase 1

**Data:** <data real>
**Medições cruas:** `medicoes/`

## As quatro perguntas

| # | Pergunta | Resposta |
|---|---|---|
| Q1 | O perfil dedicado mantém a sessão do LinkedIn entre execuções? | <> |
| Q2 | O modo extensão contorna a restrição do Chrome 136? | <ou: não foi preciso testar> |
| Q3 | Quanto cai o contexto? | <N>× |
| Q4 | A dependência de `npx` é aceitável? | <> |

## Coexistência MCP

<uma linha>

## Recomendação

<Seguir para a Fase 2 / Parar / Seguir com ressalvas — e por quê, em um parágrafo.>

## O que mudaria no roadmap

<Se alguma medição contradiz a §3 do roadmap, dizer exatamente qual linha
e o que ela passa a dizer.>
```

- [ ] **Step 2: Registrar na CLA-7**

Adicionar comentário na CLA-7 com o conteúdo do `ACHADOS.md`, em pt-BR, seguindo a skill `linear-claudia`.

**Não fechar a CLA-7.** Fechar issue tem regra própria e é decisão do usuário — a skill `linear-claudia` é explícita nisso. O comentário informa; quem fecha é ele.

- [ ] **Step 3: Commit**

```bash
git add prototipo-playwright/ACHADOS.md
git commit -m "prototype: consolidate findings and recommendation for CLA-7

Four questions answered with measured numbers. Recommendation recorded
in the issue; closing it is the user's call."
```

- [ ] **Step 4: Apresentar ao usuário**

Mostrar as quatro respostas e a recomendação. Perguntar se quer:

- **Seguir para a Fase 2** — o browser vira MCP no app de verdade
- **Parar** — fechar a CLA-7 com "não compensa" e apagar `prototipo-playwright/`
- **Investigar mais** — alguma resposta ficou ambígua

**Não apagar a pasta sem ele mandar.** As medições cruas são a única prova dos achados.

---

## Dependência externa deste plano

**A Tarefa 3 compara com a baseline da CLA-5**, que veio de transcripts de 2026-09-16. Essa comparação funciona porque os números antigos já estão registrados na issue.

**A CLA-29 não é pré-requisito deste protótipo.** O roadmap punha-a como Fase 0 porque assumia que a medição sairia de transcripts de sessões novas. Este plano mede direto — bytes de snapshot contra bytes de screenshot — e não precisa de transcript nenhum.

A CLA-29 continua valendo por si (auditoria, medição de sessões futuras), mas não bloqueia esta fase. **Isso é uma correção ao roadmap, e a Tarefa 6 deve registrá-la.**
