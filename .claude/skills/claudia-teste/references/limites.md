# Limites testados da skill

Cada limite aqui foi **medido** contra o app rodando, não deduzido. Data e versão abaixo.

## 1. Ler a aba Terminal por CDP

- **Veredito:** DOM legível, com uma ressalva (abaixo).
- **Evidência:**
  - No app (aba Terminal aberta): `.xterm` presente, `.xterm-rows` presente, **0 canvas**, 23 linhas no DOM, 4 nós de acessibilidade com "terminal". O texto estava vazio porque nenhuma sessão tinha rodado.
  - Num `Terminal` do mesmo `@xterm/xterm` criado dentro da página do app, escrever `linha-um` e `linha-dois 123` e ler `.xterm-rows > div` devolveu `linha-um|linha-dois 123 |||`, também com 0 canvas.
- **Fechado na Tarefa 6:** emiti o evento `pty-output` pelo canal do Tauri (`plugin:event|emit`). O listener do próprio app o escreveu no xterm real, e `observar.mjs` leu o marcador de volta pelo DOM. Isso exercita o caminho real (backend -> evento -> xterm -> DOM) sem iniciar sessão do agente nem gastar tokens. Ainda não vi o texto de uma sessão `claude` de verdade; a Tarefa 9 cobre se o usuário iniciar uma.
- **Consequência:** `observar.mjs` lê o Terminal por `.xterm-rows > div` e devolve `indisponivel` com o motivo quando o terminal está vazio ou sem esse renderer. Nunca finge ler.
- **Data / app:** 2026-10-01, `tauri dev`, Edg/154.0.4258.48.

## 2. Clicar na UI do app

- **Veredito:** ambos os mecanismos funcionam (DOM `.click()` e mouse CDP).
- **Evidência:** a sonda clicou em History, que só troca a vista. Medido pela árvore de acessibilidade inteira: o Dashboard tem 23 nós úteis e `heading:Dashboard`; depois do clique ficou `heading:Jobs` com 1.424 nós.
  - A) DOM `.click()`: árvore mudou.
  - B) `Input.dispatchMouseEvent` no centro do botão (110, 200), com `elementFromPoint` confirmando que o elemento sob o ponto é o botão: árvore mudou.
- **Ressalva sobre o método:** a primeira sonda usava o primeiro `h1/h2` da página como oráculo e deu "nada" nos dois. Estava errada: aquele seletor não pegava o heading que muda. Um oráculo fraco dá falso negativo tão facilmente quanto falso positivo. A skill compara a árvore inteira, não um seletor.
- **Consequência:** `ver.mjs` clica por mouse CDP, com a guarda `elementFromPoint` e o rótulo conferidos antes, por ser o mais fiel a um usuário.
- **Data / app:** 2026-10-01, `tauri dev`, Edg/154.0.4258.48.

## Verificação contra o app rodando (2026-10-01)

| Passo | Comando | Resultado | Veredito |
|---|---|---|---|
| Conectar | `subir.mjs` | `{"acao":"reutilizar","porta":9333}` | OK |
| Ver | `ver.mjs arvore` | Dashboard, Profile, History, Feedback, Notifications 9+, Terminal | OK |
| Clicar | `ver.mjs clicar "Notifications"` | `{"clicou":"Notifications","arvoreMudou":true}` | OK |
| Backend vs arquivo | `invocar.mjs obter_modo_autonomo` e `modo_autonomo.json` | `true` e `{"skip_permissions":true}` | bate |
| Recusa | `ver.mjs clicar "Apagar"`, `banco.mjs "DELETE …"`, `invocar.mjs configurar_…` sem flag | todos recusados, exit 2; vagas 305 antes e depois | OK |
| Observar | `observar.mjs` | terminal vazio -> `indisponivel` com motivo; sessões lidas do banco; texto lido após evento `pty-output` | OK, ver limite 1 |

### CLA-25 reproduzida pela UI, sem depender dos olhos do usuário

- **O que a UI mostra:** o Dashboard tem os controles de modo ("Search jobs now" e a lista Active/Inactive).
- **O que o backend sabe:** `obter_config_disparo` devolve só `ativo`, `limiar_minutos`, `limite_diario`, `limite_tempo_minutos` e `janelas`. Em disco só existem `modo_autonomo.json` (outra coisa) e `search_variants.yaml`. **Nenhum campo de modo de busca persistido.**
- **O código confirma:** `src/components/dashboard/index.tsx:229`, `handleModoDragEnd = () => { /* ... sem backend por agora */ }`, e nenhum comando Tauri que guarde os modos.
- **Limite do que provei:** não movi um slider. Mostrei que não há onde o valor possa ser guardado. Mover o controle e reler o estado confirmaria a divergência de ponta a ponta.

## Evals de disparo (2026-10-01)

**Resultado: 16/16 nas duas rodadas, com a descrição atual, no Sonnet.** Zero execuções inválidas (nenhuma saiu como `INVALIDO` nem com `session limit`).

Medido com `node .claude/skills/claudia-teste/evals/rodar-evals.mjs sonnet`, duas vezes em sequência. Critério: a **primeira** chamada de ferramenta do modelo é `Skill(claudia-teste)`. Descrição inalterada entre as rodadas (mesmo hash do frontmatter).

| Caso | Esperado | Rodada 1 | Rodada 2 |
|---|---|---|---|
| testa o Dashboard rodando | disparar | disparou | disparou |
| o badge de notificações bate com o banco? | disparar | disparou | disparou |
| tem um bug no Perfil, reproduz | disparar | disparou | disparou |
| chama o comando obter_modo_autonomo e vê o retorno | disparar | disparou | disparou |
| o agente parou onde devia? | disparar | disparou | disparou |
| compara Claude e Grok na mesma tarefa | disparar | disparou | disparou |
| **não gostei do layout do Dashboard** | disparar | disparou | disparou |
| **o bug do Perfil tá acontecendo de novo** | disparar | disparou | disparou |
| **olha o que tá errado nessa tela** | disparar | disparou | disparou |
| **só mexi no CSS, confere se ficou bom** | disparar | disparou | disparou |
| **corrige este bug no Perfil** | disparar | disparou | disparou |
| roda cargo test | não disparar | não disparou | não disparou |
| candidata nessa vaga do LinkedIn | não disparar | não disparou | não disparou |
| registra uma issue no Linear | não disparar | não disparou | não disparou |
| explica o pty_manager.rs | não disparar | não disparou | não disparou |
| qual a cor do tema? | não disparar | não disparou | não disparou |

Em negrito, os 5 que tinham falhado numa corrida anterior. Aquelas falhas coincidiram com o limite de uso da conta se esgotando durante a corrida; com o limite renovado, os 5 disparam nas duas rodadas.

**Sobre corridas anteriores:** os números 5/16, 6/16, 7/16 e 11/16 foram descartados. Vinham de um runner com defeito (`shell: true` cortava o prompt em vários argumentos), do `run_eval.py` oficial (que não funciona no Windows, `WinError 10038`), ou de execuções falhadas contadas como "não disparou".

**Limites desta medição**

- Só o Sonnet. O Haiku não foi remedido de forma válida.
- Duas repetições por prompt, não dez. Um prompt de fronteira pode oscilar.
- Mede o disparo, não se o fluxo seguinte presta. A skill disparar e depois seguir o fluxo certo é outra verificação.
