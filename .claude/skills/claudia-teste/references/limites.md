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
