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

**Resultado: inconclusivo.** Não há medição confiável do disparo da descrição. O que se sabe e o que não se sabe:

**Sabido**
- O `run_eval.py` oficial não funciona no Windows: `WinError 10038` em `select()` sobre pipes, e ele registra 0 disparos em tudo. Os "5/16" iniciais eram isso.
- O runner em Node (`evals/rodar-evals.mjs`) decide pela **primeira** chamada de ferramenta do modelo: disparou se for `Skill(claudia-teste)`.
- Lançar `claude` com `shell: true` corta prompts de várias palavras em argumentos separados, e o modelo nunca vê a pergunta real. Foi preciso `shell: false`.
- Rodados **sozinhos**, `tem um bug no Perfil, reproduz`, `o agente parou onde devia?` e `não gostei do layout do Dashboard` dispararam `Skill(claudia-teste)` e seguiram para `subir.mjs`, que é o fluxo da skill.
- Com o runner corrigido, o Sonnet deu **11/16**: os 5 casos "não deve disparar" corretos, os 6 primeiros "deve disparar" corretos.

**Não sabido, e por quê**
- Os 5 "deve disparar" que falharam nessa corrida (layout, bug recorrente, "tá errado nessa tela", CSS, "corrige este bug") podem ser descrição fraca **ou** o limite de uso da conta se esgotando durante a corrida. Repetições posteriores falharam em 100% com `You've hit your session limit` e `exit=1`, e o runner as lia como "não disparou". Não consigo separar as duas causas.
- Não medi a descrição antiga com o runner válido, então **não sei se a reescrita da descrição melhorou algo**.
- Os resultados do Haiku (5/16, tudo "sem ferramenta") não são confiáveis e foram descartados.

**O que fazer:** rodar `node .claude/skills/claudia-teste/evals/rodar-evals.mjs sonnet` com a conta fora do limite, de preferência 2-3 vezes por prompt. O runner agora marca execução falhada como `INVALIDO` e sai com código 3, em vez de contá-la como "não disparou".
