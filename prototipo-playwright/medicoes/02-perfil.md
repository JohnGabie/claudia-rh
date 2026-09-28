# Q1 — O perfil dedicado mantém a sessão entre execuções?

**Data:** 2026-09-28
**Intervalo entre fechar e reabrir:** **11 minutos**
**Chrome:** 153.0.8010.54
**Playwright MCP:** 1.64.0-alpha-1789764292000

| Página | Autenticado após reinício? |
|---|---|
| `/feed/` | **sim** — "Feed \| LinkedIn" |
| `/jobs/` | **sim** — "Vagas \| LinkedIn" |
| busca com filtro Easy Apply (`f_AL=true`) | **sim** — "Vagas de python developer \| LinkedIn" |

**3/3 autenticadas.**

## Como foi feito

1. Servidor subiu com `--browser chrome --user-data-dir ./perfil-claudia`, perfil vazio.
2. Navegou para `/feed/` e **redirecionou para `/uas/login`** — comportamento esperado
   num perfil sem sessão. Não travou em página em branco, o que já responde à
   verificação do Step 1: a restrição do Chrome 136 não bate no perfil dedicado.
3. **O usuário fez login à mão** nessa janela. Nenhum passo deste protótipo pediu, leu
   ou guardou credenciais. Confirmei a conclusão pelo título da janela, que passou de
   "Entrar no LinkedIn" para "Feed | LinkedIn".
4. Matei o servidor MCP e todo o processo do browser. Verificado: **0 processos** com
   `perfil-claudia` na linha de comando. O Chrome pessoal do usuário, num perfil
   diferente, continuou a correr sem ser tocado.
5. Esperei **11 minutos reais** — o plano exige ≥10 para a persistência ser provada em
   disco e não em cache de memória.
6. Reabri o servidor com o mesmo `--user-data-dir` e visitei as três páginas.

## O que ficou em disco entre as duas execuções

```
perfil-claudia/                          40 MB
perfil-claudia/Default/Network/Cookies   32 KB
perfil-claudia/Default/Login Data        40 KB
```

Com o browser encerrado. É isto que carrega a sessão de volta.

## Por que três páginas e não uma

O plano exige três porque uma amostra pode ser atípica — um cookie de escopo mais
largo poderia autenticar `/feed/` e falhar numa página de vagas. Não aconteceu: as
três passaram, incluindo a busca com filtro Easy Apply, que é a que o fluxo real usa.

O teste de autenticação é o URL final, não a aparência da página: o LinkedIn redireciona
pedidos não autenticados para `/login`, `/uas/login`, `/authwall` ou `/checkpoint`.
Nenhum dos três redirecionou.

## Resposta

**Sim.** O perfil dedicado mantém a sessão autenticada do LinkedIn entre execuções, com
o processo completamente encerrado e 11 minutos de intervalo.

**Consequência para a rota:** o caminho limpo existe. O roadmap (§3) dava os dois
caminhos da CLA-7 como mortos — `launchPersistentContext` sobre o perfil do usuário
(bloqueado pelo Chrome 136) e cópia do perfil (bloqueada pela App-Bound Encryption).
Ambos continuam mortos. Mas o **perfil dedicado é uma terceira porta**, e está aberta.

O custo para o usuário é fazer login uma vez, por plataforma, num browser que é do app
e não do dia-a-dia dele. Em troca, o app deixa de depender de uma extensão para navegar.

**A Q2 (modo extensão) deixa de ser decisiva** — o plano (Tarefa 4, Step 3) manda pular
os passos 4 e 5 neste caso, e evita-se instalar mais uma extensão com acesso às abas
pessoais do usuário.
