# Q2 — O modo extensão contorna a restrição do Chrome 136?

**Data:** 2026-09-28
**Chrome:** 153.0.8010.54

## A restrição bate nesta máquina?

| Teste | Resultado |
|---|---|
| `--remote-debugging-port` no perfil padrão | **inconclusivo** — ver abaixo |
| `--remote-debugging-port` com `--user-data-dir` próprio | **sim, funciona** — CDP responde |

### Por que o primeiro teste é inconclusivo

O plano manda lançar o Chrome com `--remote-debugging-port=9222` e ver se a porta abre.
Corri isso e **a porta estava aberta**, mas o resultado não prova nada:

1. **Havia 26 processos Chrome a correr.** Quando o Chrome já está aberto, um segundo
   lançamento não cria instância nova — manda uma mensagem à instância existente para
   abrir uma janela, e as flags da linha de comando são descartadas. O meu
   `--remote-debugging-port` nunca chegou a ser aplicado.

2. **O que respondeu na porta não era um endpoint CDP.** `GET /json/version` devolveu
   **HTTP 404**; um endpoint CDP devolve JSON com a versão do browser. O processo à
   escuta (PID 1876, `chrome.exe`) **não tem flags de debug na linha de comando** —
   portanto a porta foi aberta por outra coisa, provavelmente a extensão do Claude
   in Chrome, que usa uma ligação de depuração.

Para fazer este teste de forma limpa era preciso o Chrome do usuário estar fechado.
**Não fechei** — são 28 processos com as sessões autenticadas e o trabalho dele
abertos, e fechá-los para um teste de protótipo não é uma troca aceitável.

### O segundo teste é limpo e é o que importa

Com `--user-data-dir` próprio numa porta livre (9333), o Chrome cria instância
separada de verdade e o CDP responde:

```json
{
   "Browser": "Chrome/153.0.8010.54",
   "Protocol-Version": "1.3",
   "V8-Version": "15.3.76.13",
   ...
}
```

**Isto é o que decide a rota.** O caminho que o roadmap propõe — perfil dedicado —
funciona nesta máquina, nesta versão do Chrome. Se a restrição do 136 existe como
documentado, o contorno documentado também existe e está confirmado.

O perfil de teste (`perfil-cdp-teste`) foi apagado depois. Os processos Chrome do
teste foram encerrados um a um pelo caminho do perfil; o Chrome pessoal do usuário
não foi tocado — verificado antes e depois.

## Modo extensão

**Não testado.** O plano (Tarefa 4, Step 3) manda pular os passos 4 e 5 se a Tarefa 2
mostrar que o perfil dedicado mantém a sessão.

A Tarefa 2 ainda não correu — depende de login manual do usuário. Mas o teste acima
já mostra que **o obstáculo técnico ao perfil dedicado não existe**: o CDP liga. O que
falta saber é se a *sessão autenticada* sobrevive ao reinício, que é outra pergunta.

Se a Tarefa 2 der "sim", o modo extensão continua desnecessário e esta secção fecha
assim. Se der "não", os passos 4 e 5 passam a ser precisos — e aí instalar a extensão
do Playwright é uma decisão do usuário, com o mesmo tipo de acesso às abas dele que a
extensão do Claude tem hoje.

## Resposta

**A pergunta não chegou a ser precisa.** O modo extensão existe para contornar uma
restrição no perfil padrão — e a rota do roadmap não usa o perfil padrão, usa um
dedicado, que funciona. A Q2 só volta a importar se a Tarefa 2 mostrar que a sessão
não persiste.

**Correção ao roadmap:** a §3 afirma que os dois caminhos que a CLA-7 listava estão
mortos. O primeiro — `launchPersistentContext` sobre o perfil do usuário — continua
morto. Mas o perfil **dedicado** não é nenhum desses dois: é uma terceira porta, e
está aberta.
