# Propostas para cortar a dependência do Claude (Tarefa 5.1): APLICADAS no commit beaeb33

> O texto abaixo é o registro histórico de quando as propostas ainda não estavam aplicadas. A sessão de execução hoje roda com `--no-chrome` e o prompt manda usar as ferramentas `browser_*` do app.

O código das Fases 0 a 4 está pronto e testado, mas a sessão de execução **ainda** sobe com `--chrome` e o prompt ainda manda o agente usar a extensão. Remover `--chrome` sem trocar o prompt deixaria o agente sem browser. Estes dois arquivos mandam parar e perguntar:

- `src-tauri/src/prompt_sistema_runtime.md`: "Não edite sem pedido explícito."
- `docs/arquitetura-sistema-candidaturas.md`: "assinale a contradição em vez de a resolver em silêncio."

Por isso este documento só **propõe**. Aplicar é decisão do usuário.

## 1. Mudanças de código (sem texto de produto; só depois de decidir o prompt)

| Onde | Hoje | Proposta |
|---|---|---|
| `commands/sessao.rs:250` | `args.push("--chrome")` | remover; o teste da linha 287 passa a afirmar a ausência |
| `commands/perfil.rs:726` | `"--chrome"` na sessão de perfil | remover; atualizar o comentário das linhas 653-655 |
| `commands/welcome.rs:19` | checa o registro do native messaging host | trocar `nativehost_chrome` por `browser_pronto` (lido de `StatusState`, estágio `ready`); tirar o passo da extensão de `Welcome.tsx` |
| `mcp/mod.rs:23` | comentário "The `claude --chrome` run" | "the autonomous execution run" |
| `README.md:95-96, 175, 193-214` | extensão Claude in Chrome como pré-requisito | remover o passo; o browser é baixado pelo app |

## 2. Proposta para `prompt_sistema_runtime.md`

Linhas que mencionam o mecanismo antigo: **3** (cita `--chrome`), **56** (captcha "que a extensão Claude in Chrome não conseguiu passar") e **86-88** (seção "Queda de ligação da extensão Chrome", com o `/chrome`).

- **Linha 3:** trocar `claude --dangerously-skip-permissions --chrome` por `claude --dangerously-skip-permissions` (ou o equivalente do CLI usado).
- **Nova seção "Browser"** (substitui qualquer instrução de usar as ferramentas da extensão):

  > Você controla um browser próprio da Claudia pelas ferramentas `browser_open`, `browser_read`, `browser_click`, `browser_type`, `browser_key`, `browser_crop` e `browser_click_at`. Use `browser_open` para abrir uma URL e `browser_read` para ver os elementos, cada um com uma ref (`e1`, `e2`, …) que vale só até a página mudar. Use `browser_read` com `scope: "dialog"` para ler só o modal. Clique e escreva pelas refs. Use `browser_crop` ou `browser_click_at` só quando a árvore não mostrar o que precisa.
  >
  > **Enviar uma candidatura** (o botão "Enviar candidatura", "Submit application" e equivalentes) só funciona com `confirm_submit` igual ao ID da vaga que está em `candidatando`, dentro do orçamento diário e **uma vez por vaga**. Se a ferramenta responder `RECUSADO`, leia o motivo e corrija; nunca tente contornar. Depois de enviar, registre com `register_candidatura`.
  >
  > Um site em que a conta não está logada não deve ser trabalhado: registre o motivo e passe à próxima fonte.

- **Linha 56:** trocar "que a extensão Claude in Chrome não conseguiu passar automaticamente" por "que o browser da Claudia não conseguiu passar".
- **Linhas 86-88:** substituir a seção da extensão por:

  > ### Queda do browser
  > Se uma ferramenta `browser_*` responder que a conexão fechou, que o browser não está pronto ou que a ref está obsoleta, espere alguns segundos e chame `browser_read` de novo. O app religa o browser sozinho. Se continuar falhando depois de três tentativas, registre uma pendência e passe para a próxima vaga.

## 3. Contradições com `docs/arquitetura-sistema-candidaturas.md`

| Linha | Frase atual | Proposta |
|---|---|---|
| 9 | "orquestra sessões do Claude Code ligadas ao Chrome" | "orquestra sessões de uma IA (Claude Code, Grok, Gemini, OpenCode) ligadas a um browser próprio, por tools MCP" |
| 47-49 | "### 3.2 Sessão de execução (Claude Code + extensão Claude in Chrome)… flag `--chrome`" | "Sessão de execução (agente + browser da Claudia)… sem `--chrome`; o browser é do app" |
| 41, 107 | "invocada sem `--chrome`" | "invocada sem acesso ao browser" |
| 90 | "flag `--chrome` (que ativa a integração com a extensão…)" | remover a menção; o MCP do app já fornece as tools de browser |
| 242-243 | captcha/diálogos: "a sessão não os c…" | manter; o app recusa diálogos nativos nunca aceitá-los |
| 270-272 | "### 6.4 Queda de ligação da extensão Chrome" | "### 6.4 Queda do browser: o app vigia e religa (até 5 vezes em 10 minutos); depois disso mostra o erro e o usuário reinicia" |
| 282, 303 | "navegação feita pela própria sessão através da extensão" | "através das tools `browser_*`" |

Também vale uma seção nova: **browser próprio** (Chrome for Testing, perfil persistente em `%LOCALAPPDATA%`, só sessão por cookies), **guarda de envio** e **Activity log do modo desenvolvedor**.

## 4. Decisões que dependem de você

1. Posso editar o `prompt_sistema_runtime.md` conforme a seção 2? (Sem isso o `--chrome` não pode sair.)
2. Posso atualizar `docs/arquitetura-sistema-candidaturas.md` conforme a seção 3?
3. Quando um Chrome de outra versão (de uma atualização) segura o perfil, o app o fecha pedindo `Browser.close`; ao sair do app o Chrome da Claudia é encerrado. Aceita esses dois comportamentos?

## 5. Itens que a revisão final do Opus acrescentou (necessários antes de aplicar a Fase 5)

a. **Prompt, linha 56** (login "com as credenciais já configuradas"): contradiz a decisão "só sessão, nenhuma senha passa pelo app" e a regra "site deslogado é pulado". Trocar por: "Se a conta do site não está logada no browser da Claudia, registre o motivo e passe à próxima fonte; o usuário faz o login na janela do browser."
b. **`commands/perfil.rs:676-701`** (sessão de perfil): o prompt diz "Chrome com a sessão autenticada" e manda "fechar as abas", mas não existe tool para fechar abas. Reescrever para o browser da Claudia e remover o "fechar as abas" (ou acrescentar `browser_close_tab` como ferramenta nova).
c. **Passar `--no-chrome`**, não só remover `--chrome`: se o usuário ligou o Chrome por padrão no Claude Code, as tools da extensão continuariam carregando junto com as `browser_*`.
d. **Só iniciar sessão com o browser `ready`.** `sessao.rs` e o `idle_watcher` não esperam o browser. Na primeira abertura o download leva minutos, e a regra "3 tentativas e vira pendência" geraria uma pendência para cada vaga. Esperar o estágio `ready` (com limite e mensagem) antes de `claude`.
e. **Welcome:** além de `browser_pronto`, um passo "entre no LinkedIn na janela da Claudia" (a sessão por cookies é o que dá a "memória das contas"; sem isso o agente só pula tudo).

## 6. Pontos que a mesma revisão deixou como issues (não bloqueiam a Fase 5)

- **Instância única do app** (`tauri-plugin-single-instance`): uma segunda instância que sai roda `stop()` e derruba o Chrome da primeira. O vigia já se recupera (corrigido), mas o ideal é impedir a segunda instância.
- **A porta de debug não tem autenticação:** qualquer processo local, ou o próprio agente pelo shell, pode ler o `state.json`, conectar por CDP e contornar a guarda. A guarda protege contra erro e manipulação do **modelo**, não é contenção. Documentar na spec §5.5.
- **Eventos do browser no Activity log** (download, lançamento, queda, relançamento: spec §5.6 "Evento do browser") não são gravados: `service.rs` não toca `activity_log`.
- Desinstalador: a caixa nativa está em inglês e vem do repositório do Tauri, não da versão travada; confirmar com o instalador real e alinhar a spec §5.7 com o corte da Tarefa 4.2.
- `guard.rs` diz "nesta sessão", mas a reivindicação é por (vaga, dia); a tabela `submit_claims` é criada fora das migrações.
- `prototipo-playwright/` versiona caminhos absolutos pessoais (`mcp-config-teste.json`, `cdp/t1-neutra.mjs:125`, `cdp/t5-recorte-scroll.mjs:4`) num repositório público; podar ou mover para `docs/pesquisa`.
