# Spec: Entrega do prompt sem argv — arquivo agora, MCP depois

> Spec de design. Não é o plano de implementação.

**Data:** 2026-09-25
**Branch:** parte 1 já existe em `feat/fix-sessao-spawn`; parte 2 em `feat/prompt-via-mcp` a partir de `dev`
**Repo:** `claudia-rh/`

## 1. Problema

A sessão de execução nunca inicia. O usuário clica, vê o banner, e o verde fica infinito.

Console real da webview, capturado em 2026-09-25:

```
CreateProcessW `"...\Anthropic.ClaudeCode_...\claude.exe
  --dangerously-skip-permissions --chrome
  --mcp-config ...\mcp-config.json
  --system-prompt "# Prompt de sistema — sessão de execução
  ...  dados_pessoais: nome_completo: ... cpf: ... endereco: ...
```

O corpo inteiro do prompt vai dentro da linha de comando.

| Medida | Valor |
|---|---|
| Template `prompt_sistema_runtime.md` | **17 264** chars (medido 2026-09-25) |
| Prompt montado, com os YAMLs substituídos | **30 147** chars (medido na spec de 2026-09-17) |
| Logo, a parte que vem dos dados | **~12 900** chars |
| Limite do `CreateProcessW` | **32 767**, incluindo o NUL |

`src-tauri/src/commands/sessao.rs:62` faz `args.push("--system-prompt")` e empurra o corpo como argumento seguinte.

### Dois defeitos que se escondem um ao outro

O spawn falha. E `sessao.rs` emite o banner na linha 67 e `session-started` na **71**, antes de chamar `pty_manager::iniciar_claude` na **73**. A UI promete uma sessão viva antes de existir processo. Um defeito impede o arranque; o outro esconde a falha.

### O problema não é o limite, é o crescimento

As regras são fixas. **Os dados crescem.** `prompt.rs:21-27` substitui cinco marcadores:

| Marcador | Origem | Tamanho |
|---|---|---|
| `{{CANDIDATE_BASE_YAML}}` | `candidate_base.yaml` | cresce com a carreira |
| `{{SEARCH_VARIANTS_YAML}}` | `search_variants.yaml` | cresce com as buscas |
| `{{STRATEGY_MD}}` | `strategy.md` | cresce |
| `{{RECENT_MEMORY_SUMMARY}}` | SQL sobre `candidaturas`/`vagas`/`pendencias` | cresce com o uso |
| `{{DB_PATH}}` | caminho | fixo e curto |

Com ~2 600 chars de folga, acrescentar duas experiências ao currículo quebra outra vez. Qualquer correção que só levante o teto adia o defeito em vez de o encerrar.

### Vazamento de PII na linha de comando

O erro acima despejou CPF, endereço, telefone e data de nascimento no console. Não é acidente do log: a argv **contém** esses dados, e argv é legível por qualquer processo da máquina que liste argumentos de processo. O app rodar localmente reduz o risco, não o elimina — um arquivo em `AppData` tem ACL do sistema de arquivos; argv não tem nada.

## 2. Estudo

### 2.1 O limite do Windows

[CreateProcessW](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw): *"The maximum length of this string is 32,767 characters, including the Unicode terminating null character."* A spec de 2026-09-17 já registrou isto e concluiu o mesmo: nunca passar o corpo do prompt na argv.

Agravante: o `portable-pty` / ConPTY ainda precisa *quotar* cada argumento antes de entregar, o que consome margem além dos 30 147 medidos.

### 2.2 O MCP já existe e já é configurado sozinho

`src-tauri/src/mcp/server.rs` expõe **9 tools**, e todas escrevem:

```
register_vaga · register_candidatura · update_profile · update_vaga_status
create_pendencia · close_pendencia · close_pendencias_vaga
list_pendencias · get_pendencia_vaga
```

`update_profile` é o ponto importante: **o Claude já grava o perfil de volta por MCP.** O que não existe é o caminho inverso — nenhuma tool lê o perfil. O app empurra por argv e recebe por MCP. É um cano de sentido único com a ponta de retorno já soldada.

`write_mcp_config` (`src-tauri/src/commands/perfil.rs:447`) escreve o `mcp-config.json` a cada spawn, e o `--mcp-config` já é passado. **Acrescentar tools não custa configuração nenhuma ao usuário** — restrição declarada explicitamente pelo autor e satisfeita por construção.

### 2.3 Alternativas descartadas

| Caminho | Por que não |
|---|---|
| stdin | A sessão é interativa num PTY; stdin é o terminal do usuário. Usá-lo para o prompt quebra a interatividade. |
| Variável de ambiente | O bloco de ambiente no Windows também tem teto, e é legível por outros processos. Troca um problema pelo mesmo problema. |
| Encurtar o template à mão | Não resolve: o que cresce são os dados, não as regras. |

## 3. Objetivo

1. A sessão volta a iniciar.
2. O prompt deixa de crescer com o perfil — acrescentar experiências ao currículo não pode voltar a quebrar o spawn.
3. CPF e endereço saem da linha de comando.
4. Zero configuração nova para o usuário.
5. Se o spawn falhar, a UI diz que falhou em vez de ficar verde.

## 4. Fora de escopo

- Tirar o perfil do **contexto do modelo**. Impossível e não pretendido: para escrever um currículo o Claude precisa dos dados. O que se resolve aqui é dado em repouso (argv, arquivo) e escopo de acesso.
- Criptografar `candidate_base.yaml` em disco.
- Reescrever as regras do `prompt_sistema_runtime.md`. A edição autorizada é cirúrgica — ver 5.3.
- Mudar quem conduz o browser. Isso é o estudo `CLA-7`.

## 5. Desenho

### 5.1 Parte 1 — `--system-prompt-file` (já escrita)

Existe em `feat/fix-sessao-spawn`, com teste que fixa o comportamento:

```rust
assert!(args.contains(&"--system-prompt-file".to_string()));
assert!(!args.iter().any(|a| a == "--system-prompt"));
```

O prompt é gravado em `data_dir/workspace/.claude-system-prompt.txt` e a argv leva só o caminho. A mesma branch inverte a ordem do ciclo de vida: spawn primeiro, `session-started` só se o spawn retornar `Ok`.

**Ação: merge de `feat/fix-sessao-spawn` em `dev`.** São 19 commits, que trazem junto todo o trabalho de diagnóstico (`events.jsonl`, export de zip, watchdog) preso na mesma branch.

Isto encerra o objetivo 1, o 3 (parcialmente — sai da argv, passa a ficar num arquivo com ACL) e o 5. **Não encerra o 2.**

### 5.2 Parte 2 — tools de leitura no MCP

Quatro tools novas, uma por marcador que cresce:

| Tool | Substitui | Retorna |
|---|---|---|
| `get_candidate_profile` | `{{CANDIDATE_BASE_YAML}}` | conteúdo de `candidate_base.yaml` |
| `get_search_variants` | `{{SEARCH_VARIANTS_YAML}}` | conteúdo de `search_variants.yaml` |
| `get_strategy` | `{{STRATEGY_MD}}` | conteúdo de `strategy.md` |
| `get_memory_summary` | `{{RECENT_MEMORY_SUMMARY}}` | o resumo que `build_memory_summary` já calcula |

`{{DB_PATH}}` permanece no template: é curto, fixo, e não é dado pessoal.

`montar_prompt_sistema` (`prompt.rs:4`) deixa de ler os três arquivos e de correr o SQL. Passa a substituir só `{{DB_PATH}}`. O prompt montado fica **fixo em ~17 k**, e deixa de depender do tamanho da carreira do usuário.

**Ganho além do tamanho:** hoje `build_memory_summary` corre uma vez, no spawn, e o Claude carrega um retrato congelado. Como tool, ele relê no meio da sessão — os contadores de candidaturas do dia passam a estar certos depois da décima candidatura, e não só na primeira.

`update_profile` já existe e continua como está. O par fica simétrico.

### 5.3 A edição do `prompt_sistema_runtime.md`

O `CLAUDE.md` marca este arquivo como **não editar sem pedido explícito**. A autorização foi dada nominalmente pelo autor em 2026-09-25, para este escopo e só este.

A edição é substituir quatro blocos de dados por instrução de como obtê-los. Onde hoje está o YAML inteiro, passa a estar a instrução de chamar `get_candidate_profile` antes de avaliar uma vaga ou gerar material.

**Este é o risco real da parte 2, e não é de engenharia, é de comportamento.** Se os dados saem do prompt e o modelo não chama a tool, ele inventa ou pergunta — e inventar dados de currículo é pior do que o bug que estamos consertando. A instrução precisa deixar claro que o perfil **não está** no prompt e que trabalhar sem o chamar produz candidatura falsa.

Mitigação que o desenho adota: as tools retornam erro explícito e legível quando o arquivo não existe, em vez de string vazia. Hoje `prompt.rs:10` usa `unwrap_or_default()` — um `candidate_base.yaml` ausente vira silêncio, e o Claude recebe um perfil vazio sem saber. Uma tool que falha alto é melhor do que um `{{ }}` que se substitui por nada.

### 5.4 Ordem

A parte 1 não depende da parte 2 e deve ir primeiro: é código pronto e testado, e devolve o app ao ar hoje. A parte 2 assenta sobre ela, porque com o prompt em arquivo o tamanho deixa de ser urgente e a mudança de comportamento pode ser validada sem pressa.

## 6. Arquivos

**Parte 1** — nenhum arquivo novo; é um merge.

**Parte 2:**

| Arquivo | Mudança |
|---|---|
| `src-tauri/src/mcp/server.rs` | 4 tools de leitura; erro explícito quando a origem não existe |
| `src-tauri/src/prompt.rs` | `montar_prompt_sistema` substitui só `{{DB_PATH}}`; `build_memory_summary` passa a ser chamado pela tool |
| `src-tauri/src/prompt_sistema_runtime.md` | 4 blocos de dados → instrução de chamar as tools (**editado sob autorização de 2026-09-25**) |
| `docs/arquitetura-sistema-candidaturas.md` | a arquitetura descreve o prompt como portador dos dados; passa a descrever o MCP |

A última linha não é opcional: o `CLAUDE.md` manda que, se o código contradiz a arquitetura, o documento vence. Mudar um sem o outro cria a contradição que esse repo já pagou caro para remover.

## 7. Verificação

```text
# parte 1 — o critério é o da spec de 2026-09-17
clicar "procurar vagas" → em ≤10 s, ou há texto do Claude no Terminal,
ou há uma linha de erro explícita. Nunca banner sozinho com verde infinito.

# parte 2 — tamanho deixa de depender dos dados
teste unitário: montar_prompt_sistema com um candidate_base.yaml de 50 KB
produz o mesmo tamanho que com um de 5 KB

# parte 2 — PII fora da argv e fora do arquivo de prompt
teste unitário: o prompt montado não contém o e-mail nem o CPF do perfil

# parte 2 — o modelo realmente chama
sessão real: o Claude invoca get_candidate_profile antes de gerar
o primeiro currículo. Se gerar sem chamar, a instrução do 5.3 falhou.

# parte 2 — falha alto
renomear candidate_base.yaml e chamar a tool → erro legível, não string vazia
```

O terceiro teste é o que impede regressão do vazamento: se alguém voltar a injetar o YAML no prompt, ele quebra.

## 8. Superpowers

Parte 1: merge de `feat/fix-sessao-spawn` em `dev`, sem ciclo novo — o ciclo dela já existe (spec de 2026-09-17).

Parte 2: esta spec aprovada → `writing-plans` → `feat/prompt-via-mcp` a partir de `dev` → review → merge em `dev`.
