# Spec: contas do browser da Claudia

> Spec de design. Não é o plano de implementação. Ciclo 2 de 3. Depende do motor do browser (`2026-10-02-browser-cdp-motor-design.md`).

**Data:** 2026-10-02
**Branch:** `feat/browser-contas` (a partir da branch do ciclo 1, depois que ela estiver integrada)
**Issue:** a criar no Linear, ligada à CLA-31

## 1. Problema

O browser da Claudia guarda as sessões do usuário (cookies), mas o app não sabe **em quais sites há sessão** nem **em quais a IA pode agir**. Resultado hoje: a procura de vagas gasta tempo em site onde não há login, e o usuário não tem um lugar para ver e controlar isso.

## 2. Objetivo

Um lugar simples para as contas que o usuário tem, onde ele vê se cada uma está logada e decide se a IA pode usá-la. A procura de vagas **pula** o que não está logado e permitido, sem travar a rodada. O browser abre quando a IA é chamada; fora disso o usuário o abre pelo botão.

Princípio: **só sessão**. O usuário faz login uma vez na janela do browser da Claudia, como num browser normal. Nenhuma senha passa pelo app. Quando a sessão expira, o site volta a "precisa de login" e o usuário loga de novo.

## 3. Modelo de dados

Tabela `contas_browser` (no banco principal do app):

| Campo | Significado |
|---|---|
| `chave` | identificador estável (`linkedin`, `indeed`, …; para site do usuário, gerado do domínio) |
| `nome` | rótulo exibido |
| `url_login` | para onde o botão "Abrir" leva |
| `dominios` | domínios que pertencem à conta (usado para recusar navegação em `browser_open`) |
| `regra_login` | como saber se há sessão (ver seção 4) |
| `ia_pode_usar` | permissão explícita do usuário, desligada por padrão |
| `embutida` | se veio com o app ou foi adicionada pelo usuário |
| `estado` | `logada`, `precisa_login`, `desconhecido` |
| `verificada_em` | instante da última checagem |

Contas embutidas começam com `ia_pode_usar = falso`. O usuário liga uma a uma.

## 4. Como saber se está logado

A regra é um **dado** da conta, não código espalhado:

- **Conta embutida:** `cookie_sessao`. O app lê os cookies do domínio pelo CDP (`Network.getCookies`) e confere se existe o cookie de sessão daquele site (LinkedIn: `li_at`, a confirmar na primeira tarefa do plano). Só **o nome** do cookie é lido para decidir; o valor não é guardado nem registrado.
- **Conta adicionada pelo usuário:** começa em `manual`: o usuário clica "já estou logado". O estado vale até a próxima checagem, que é a heurística "apareceu cookie novo de sessão depois do login" ou, na falta dela, continua manual.
- A lista inicial de contas embutidas e a regra de cada uma só entram **depois de testada** na primeira tarefa do plano. Só entra o que foi verificado de verdade, começando pelo LinkedIn.

**O verde não pode mentir.** A sessão expira. O app reconfere antes de cada rodada de procura (o browser já foi aberto pela chamada da IA), e quando uma navegação cai numa página de login. Com o browser fechado não há como ler os cookies pelo CDP: o estado fica como estava, marcado com a hora da última checagem, e a tela mostra `desconhecido` se essa checagem for antiga demais; nunca `logada` sem checagem recente. Se a checagem falha por qualquer motivo, o estado vira `desconhecido` (amarelo), nunca `logada`.

## 5. Comportamento

1. **Abrir o browser:** o browser da Claudia **só roda quando a IA é chamada** (decisão de 2026-10-03; ver "Ciclo de vida" na seção 3.6 da arquitetura). Por isso a tela de contas tem um botão **"Abrir o browser"**, sempre visível no topo, que lança o browser quando ele não está aberto e o traz para a frente quando já está. Sem ele o usuário não teria como logar fora de uma sessão. O botão já existe nas Configurações (comando `browser_abrir`, entregue no ciclo 1); este ciclo o leva para a tela de contas e acrescenta o foco da janela.
2. **Abrir uma conta:** botão por conta. Abre o browser se preciso (mesmo comando), foca a janela e abre `url_login`. O usuário loga na janela.
3. **Estado visível:** verde (`logada`), vermelho ou cinza (`precisa_login`), amarelo (`desconhecido`).
4. **Adicionar site:** o usuário informa nome e URL. A conta nasce com regra `manual` e `ia_pode_usar` desligado.
5. **Remover:** só contas adicionadas pelo usuário. As embutidas podem ser desligadas, não apagadas. Remover **não** limpa os cookies do perfil; isso é uma ação separada e explícita.
6. **Permissão:** o interruptor "a IA pode usar" é a autorização explícita. Mudar para ligado exige um clique, sem automação.

## 6. Integração com a procura de vagas

A regra "pular o que não está logado" vale **no código**, não no prompt:

- `browser_open(url)` resolve o domínio da URL para uma conta. Se a conta existe e está `precisa_login`, `desconhecido` ou com `ia_pode_usar` desligado, a tool **recusa** com texto claro ("conta LinkedIn não está logada; pule esta fonte"), e o agente segue para a próxima.
- Domínios sem conta cadastrada (sites de empresa, onde o agente só lê a vaga) **não são bloqueados**: só se bloqueia o que é conta gerenciada.
- `browser_contas()` devolve ao agente quais contas estão logadas e permitidas, para ele planejar a rodada.
- A recusa fica no activity log do modo desenvolvedor, e o item pulado entra no resumo da rodada com o motivo.

Isso também alimenta a condição 3 da guarda de envio do ciclo 1 (site logado e permitido).

## 7. Interface

Uma lista de contas dentro do app. **Onde ela mora na navegação** é decidido no plano, depois de ler `docs/padrao-design-claudia-rh.md` seção de navegação, porque o design vence e não vou propor aqui uma aba que contradiga o documento. Segue os tokens existentes (`--accent` `#D97757` como destaque; verde, âmbar e vermelho só como estado semântico, nunca como decoração).

Também entra neste ciclo a **linha do tempo por vaga** no modo desenvolvedor, em cima do activity log e da lista simples que o ciclo 1 já entrega (ver seção 5.6 do motor): agrupa as ações, os eventos e os prints de uma candidatura, do início ao envio.

## 8. Testes

- **Unidade:** cada regra de login contra cookies simulados; transição de estados (`logada` → `desconhecido` quando a checagem falha); o verde nunca aparece sem checagem bem-sucedida; resolução de domínio para conta; permissão desligada bloqueia.
- **Integração:** Chrome real (headless) com cookies plantados no perfil de teste: `li_at` presente dá `logada`, ausente dá `precisa_login`; `browser_open` recusa e permite conforme o estado.
- **Persistência:** as contas e permissões sobrevivem a reiniciar o app.
- **Botão e ciclo de vida:** com o app aberto, o browser está fechado (estado `installed`); o botão o abre e fica desabilitado enquanto ele roda; iniciar uma sessão o abre sozinho; fechar a janela à mão sem sessão ativa não a reabre.
- **Manual curto:** logar de verdade numa conta no browser da Claudia, ver o verde, deixar a sessão expirar e ver voltar.

## 9. Fora do escopo

- Guardar senha ou fazer login automático. Descartado por segurança.
- A IA configurar contas e perfil a partir do que vê logada (ciclo 3).
- Várias contas do mesmo site; limpeza de cookies por conta.
- Detectar captcha ou parede de login no meio da rodada (ideia da observabilidade, fora do ciclo 1).
