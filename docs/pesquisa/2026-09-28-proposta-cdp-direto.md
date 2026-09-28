# Proposta: o app controla o Chrome direto, por CDP

**Data:** 2026-09-28
**Estado:** proposta de desenho. Nada implementado.
**Substitui:** a recomendação anterior de avaliar `mcp-chrome` ou `agent-browser`

> As duas propostas anteriores violavam a regra que acabou de entrar no `CLAUDE.md`:
> pediam ao usuário que instalasse mais alguma coisa. Esta não pede nada.

## O que foi verificado

Consultei o protocolo do Chrome 153 instalado nesta máquina (`/json/protocol`, 1.6 MB).
Tudo que o protótipo provou hoje existe como comando CDP nativo:

| Capacidade provada hoje | Comando CDP | Como foi provado |
|---|---|---|
| navegar | `Page.navigate` | 8 plataformas, nenhuma bloqueou |
| ler a estrutura | `Accessibility.getFullAXTree` | 20 320 tok o modal inteiro |
| **recorte de tela** | `Page.captureScreenshot` com `clip: {x,y,width,height,scale}` | **390 tok, legível** |
| **rato por coordenada** | `Input.dispatchMouseEvent` | acertou o alvo lido da imagem |
| teclado | `Input.dispatchKeyEvent`, `Input.insertText` | não testado |
| clicar/escrever no DOM | `Runtime.evaluate` | funcionou por índice |
| posição de um elemento | `DOM.getBoxModel` | via `getBoundingClientRect` |

**O Playwright é um invólucro sobre isto. A extensão do Claude também.** Eu estava
medindo a embalagem, não o conteúdo.

E o CDP com `--user-data-dir` próprio foi testado nesta máquina: porta 9333, Chrome
153.0.8010.54, protocolo 1.3, resposta completa.

## O que o app economiza em passos

`welcome.rs` checa três coisas hoje, e uma delas é:

```
Software\Google\Chrome\NativeMessagingHosts\com.anthropic.claude_code_browser_extension
```

| Passo de hoje | Com CDP direto |
|---|---|
| Instalar Claude Code + acertar PATH | vira "instalar **uma** IA qualquer" |
| **Instalar a extensão Claude in Chrome** | **some** |
| **Registrar o native messaging host** | **some** |
| Abrir o Claudia RH | fica |
| Preencher o perfil | fica |

Dois passos a menos, e o primeiro deixa de ser específico do Claude. Nenhum passo novo.

## O desenho

```
Claudia RH  ──lança──►  Chrome (--remote-debugging-port, perfil do usuário)
     │                       ▲
     │                       │ CDP, em Rust (chromiumoxide)
     └── servidor MCP ───────┘
              ▲
              │ qualquer CLI: claude, grok, gemini, opencode
```

O app **deixa de consumir** controle de browser e passa a **fornecê-lo**, como mais
algumas tools no servidor MCP que ele já tem — o mesmo que o Grok consumiu hoje sem
precisar de alteração nenhuma.

As tools novas seriam poucas, e cada uma mapeia direto num comando CDP:

| Tool | CDP por baixo |
|---|---|
| `browser_ir_para(url)` | `Page.navigate` |
| `browser_ler_pagina()` | `Accessibility.getFullAXTree` |
| `browser_recortar(x,y,w,h)` | `Page.captureScreenshot` com `clip` |
| `browser_clicar(x,y)` | `Input.dispatchMouseEvent` |
| `browser_escrever(texto)` | `Input.insertText` |

### Por que isso resolve as duas razões do usuário

> *"Toda vez que eu tenho mais de uma conta e preciso trocar, tenho que trocar no
> plugin. Isso desestabiliza se o token estoura."*

O browser passa a ser do app. Trocar de conta de IA não toca no browser.

> *"Eu não posso usar outras IAs, que foi uma das reclamações que mais recebi."*

`--chrome` era a única flag sem equivalente em outro agente (ver
`2026-09-28-agentes-cli-e-portabilidade.md`). Virando tool MCP, some o bloqueio.

### E a segurança fica melhor, não pior

O caminho que eu tinha proposto para o rato exigia `browser_run_code_unsafe`, que a
própria ferramenta descreve como **"RCE-equivalent"**. Isso contradizia de frente o
confinamento da CLA-30.

Com as tools acima, o agente não executa código arbitrário: pede uma ação nomeada, o app
valida e executa. E cada clique passa pelo `dispatch` do app, que já tem log e
notificação para a UI.

**A guarda contra submissão acidental vira código do app, não instrução de prompt.** Isso
importa: o botão "Enviar candidatura" fica no mesmo passo do formulário, a poucas
centenas de pixels dos campos.

## O que isso custa

**É trabalho de verdade.** Não é uma tarde. Precisa de:

- um cliente CDP em Rust ([`chromiumoxide`](https://github.com/mattsse/chromiumoxide),
  que tem `DispatchMouseEventParams`) ou WebSocket na mão
- gerenciar o ciclo de vida do Chrome: lançar, detectar que morreu, religar
- as tools novas no servidor MCP
- e o prompt do agente muda: ele deixa de "dirigir o browser" e passa a "pedir ações"

**E tem risco de perfil.** Hoje a extensão roda no Chrome real do usuário. Lançar o
Chrome nós mesmos com `--remote-debugging-port` exige ou o perfil dedicado (testado: a
sessão do LinkedIn sobrevive ao reinício, 3/3 páginas autenticadas depois de 11 min) ou
descobrir como reusar o perfil padrão — que o Chrome 136+ bloqueia para debugging.

O perfil dedicado funciona, mas custa um login por plataforma. E um browser que só faz
automação é, em tese, mais fácil de sinalizar do que o browser real de uma pessoa —
embora nenhuma das 8 plataformas testadas hoje tenha bloqueado nada.

## O que não foi verificado

- **`chromiumoxide` dentro de um app Tauri.** Nada garante que se comporte bem.
- **O esforço real.** A estimativa acima é leitura de documentação, não experiência.
- **Teclado.** `Input.insertText` existe no protocolo; não foi testado.
- **Lançar o Chrome nós mesmos** pode trazer problemas que não apareceram ao conectar
  num Chrome já rodando.

## Recomendação

**Um spike antes de qualquer decisão.** Meio dia, código descartável, uma pergunta:

> Um binário Rust com `chromiumoxide` consegue abrir o LinkedIn autenticado, ler a
> árvore, recortar a tela e clicar por coordenada — tudo que o protótipo fez hoje via
> Playwright?

Se sim, o desenho acima é viável e vira um ciclo Superpowers normal. Se não, o motivo
vai dizer o que fazer — e o protótipo Playwright continua no repositório como referência
do que é possível.

**Não começar a implementação antes disso.** A medição de hoje mostrou cinco heurísticas
que falharam em silêncio; a lição é que este domínio pune quem assume.
