# Padrões e armadilhas do CDP

Todas falharam em silêncio quando descobertas. Não dão erro; devolvem algo que parece resposta.

1. **A porta é de outro processo.** A 9222 era o Chrome pessoal do usuário. Confirmar sempre com `validarVersaoApp` (`Edg/` mais página `localhost:1420`).
2. **A 1420 presa por um vite antigo** derruba o `tauri dev` com "Port 1420 is already in use". `subir.mjs` não a libera sozinho (ver ledger da Tarefa 3); resolva à mão.
3. **Aba escondida engole eventos de input** sem erro. `/json/activate/<id>` e `Page.bringToFront` antes de clicar.
4. **`getBoxModel` sem `scrollIntoViewIfNeeded`** devolve coordenada fora do ecrã.
5. **Clique por coordenada não tem elemento para recusar.** Ler `elementFromPoint` antes e comparar com o rótulo pretendido.
6. **A árvore de acessibilidade calcula o nome pelo algoritmo completo** (`label`, `aria-labelledby`, texto), não só `aria-label`.
7. **Um oráculo fraco dá falso negativo.** Verificar "a tela mudou" pelo primeiro `h1/h2` deu "nada" num clique que funcionava. Comparar a árvore inteira.
8. **`process.exit` com o WebSocket aberto** aborta o Node 24 no Windows (`UV_HANDLE_CLOSING`). Fechar o socket e sair depois.
9. **Escapes em heredoc de shell** corrompem regex e strings com `\`. Escrever arquivos JS pela ferramenta de escrita, não por `cat <<EOF`.

Referência validada: `prototipo-playwright/cdp/` na branch `feat/exec-prototipo-playwright`.
