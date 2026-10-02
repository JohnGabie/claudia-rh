# Validação: Motor do browser com CDP

Fase 0 — validação de limitações técnicas e requisitos antes da implementação.

## 0.1: Fonte do download, licença e fixação da versão

**Hipótese:** É possível baixar Chrome for Testing de fontes confiáveis, fixar uma versão específica com hash, e a licença permite tanto baixar para o usuário quanto redistribuir no instalador.

**O que foi feito:**
1. Consultado https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json
2. Identificada a versão estável: `154.0.8037.92`
3. Baixado o zip de https://storage.googleapis.com/chrome-for-testing-public/154.0.8037.92/win64/chrome-win64.zip
4. Calculado o SHA-256: `b897ef3601c947ac0620c784556dec719ac602b0159ce105927acf645ee0f598`
5. Verificado tamanho: 197 MB (bem acima do mínimo de 50 MB)
6. Consultada a licença do repositório `GoogleChromeLabs/chrome-for-testing` (Apache 2.0). **Erro do primeiro registro (corrigido pelo Opus/conversa principal):** essa licença cobre o código e o painel do repositório, **não** o binário do Chrome. A conclusão "permitido" que estava aqui foi retirada.
7. Hash conferido de forma independente com `Get-FileHash`: igual ao acima.

**Resultado:**
- Versão fixada: `154.0.8037.92`
- SHA-256: `b897ef3601c947ac0620c784556dec719ac602b0159ce105927acf645ee0f598` (conferido duas vezes)
- Base URL: `https://storage.googleapis.com/chrome-for-testing-public`
- Tamanho do arquivo: 205 614 867 bytes (197 MB)

**Licença: não estabelecida pelas fontes lidas.** O que as páginas da Google dizem (lido em 2026-10-02 em `developer.chrome.com/blog/chrome-for-testing/`):

- "Chrome for Testing is a Chrome flavor that specifically targets web app testing and automation use cases."
- "Chrome for Testing has been created purely for browser automation and testing purposes. **It should only be used to consume trustworthy content.**"
- É um "versioned binary" **sem auto-update**; a marca tem "the word 'Test' in a black rectangle".
- A página diz que o conteúdo do **site** é CC BY 4.0 e as amostras de código Apache 2.0. **Nada** sobre licença ou redistribuição do binário.

(a) Baixar da Google na máquina do usuário: **não confirmado** por nenhuma fonte; a Google descreve o uso pretendido como teste e automação.
(b) Redistribuir no nosso instalador: **não confirmado**; o plano não faz isso.

**Dois problemas de produto, mais importantes que a licença:**
1. A Google restringe o uso pretendido a "conteúdo confiável". O browser da Claudia abre LinkedIn, sites de empresas e vagas de terceiros, que não são conteúdo confiável nesse sentido.
2. **Sem auto-update:** correções de segurança do Chrome passam a ser responsabilidade nossa (subir a versão fixada a cada release). Um browser desatualizado, logado nas contas do usuário, é risco real.

**Os Termos de Serviço do Google Chrome não foram lidos** (o plano pedia; só o blog do Chrome for Testing foi lido). Falta isso para fechar a questão de licença. mas a **escolha do Chrome for Testing como browser do produto não está confirmada**. Alternativas a decidir com o usuário no portão 0.5: (A) manter o Chrome for Testing, aceitando o risco de uso e de segurança e prometendo atualizar a versão a cada release; (B) **Chromium** (open source, redistribuível, mesmo motor; **ressalvas:** os builds agregam código de terceiros sob várias licenças, não só BSD, não há canal estável com correções de segurança como o do Chrome, e também não tem auto-update); (C) usar o Chrome do usuário com perfil dedicado (volta ao plano anterior; depende de ele ter Chrome).


## 0.2: A sessão do LinkedIn sobrevive no Chrome for Testing

**Hipótese:** o perfil `prototipo-playwright/perfil-claudia` (criado pelo Chrome 153, logado no LinkedIn) continua autenticado no Chrome for Testing 154.

**O que foi feito:** cópia do perfil para `%TEMP%\cft-profile-0.2` (o original não foi aberto); Chrome for Testing na porta 9777; navegação **só de leitura** a `https://www.linkedin.com/feed/` pelo cliente CDP de referência; nada foi clicado, digitado nem enviado. O agente relata ter encerrado só o processo da porta 9777, mas o comando exato não ficou registrado nas notas.

**Resultado:** `Browser = Chrome/154.0.8037.92`; URL `https://www.linkedin.com/feed/`; título `Feed | LinkedIn`. Sem `/login`, `/authwall` ou `/checkpoint`.

**Decisão:** **passou, com o alcance estreito que a evidência dá:** cookies de uma cópia de perfil criada pelo Chrome 153, abertos pelo mesmo usuário do Windows, são lidos pelo Chrome for Testing 154. **Não** prova que a sessão sobrevive a reiniciar o Chrome for Testing, nem que funciona num perfil na pasta de dados do próprio app, nem o primeiro login feito numa janela dele. Isso é coberto pela Tarefa 6.3-a/c. **Limpeza:** a cópia do perfil (`cft-profile-0.2`, 194 MB, com cookies de sessão) e os perfis de tema foram apagados do `%TEMP%` ao fim; o Opus apontou que tinham ficado para trás.

## 0.3: O tema laranja sem clique do usuário

**Hipótese:** dá para gravar `#D97757` (inteiro com sinal `-2525353`) no `Preferences` de um perfil novo, antes da primeira abertura, e o Chrome o mostra.

**O que foi feito:** três patches testados, cada um num perfil novo, com screenshot do sistema. O Haiku não pôde escolher cor pela interface, então testou chaves candidatas.

**Resultado:**
- **Candidato 2 funcionou**: `{"autogenerated":{"theme":{"color":-2525353}},"extensions":{"theme":{"id":"autogenerated_theme_id"}}}`. Screenshot recortado em `docs/pesquisa/2026-10-02-tema-laranja-cft.png` (conferido pela conversa principal e pelo Opus): barra de abas, barra de endereço e nova guia em salmão/laranja. O Chrome monta uma paleta a partir da cor, então a tela não mostra o hex exato `#D97757`.
- Candidatos 1 e 3: o Haiku registrou "falhou", mas **nenhum dos dois screenshots mostra o Chrome for Testing** (só o Chrome pessoal do usuário, com abas dele). Ficam "não comprovados", sem efeito, porque o 2 funciona. Os screenshots originais foram apagados do `%TEMP%` por capturarem abas pessoais; só o recorte do candidato 2 foi guardado. Próxima vez: capturar só a janela do Chrome for Testing.
- Política de registro: não aplicada (qualquer chave sob `Google\Chrome` mudaria também o Chrome pessoal do usuário).

**Decisão:** **passou.** `src-tauri/browser-theme.json` gravado com o candidato 2.

**Achado novo na mesma imagem (importante para o produto):** o Chrome for Testing mostra uma **faixa fixa** no topo: *"O Chrome for Testing v154.0.8037.92 é destinado apenas a testes automatizados. Para a navegação normal, use uma versão padrão do Chrome. Baixe o Chrome"*, além de um botão "Fazer login no Chromium?". O usuário veria isso toda vez que abrisse o browser da Claudia. Soma-se ao aviso da Google de "conteúdo confiável" e à falta de auto-update (seção 0.1). Ver o portão 0.5.

## 0.4: O que o desinstalador do Tauri já apaga

**Hipótese:** a caixa nativa do desinstalador já cobre a opção "remover meus dados".

**O que foi feito:** leitura (sem executar nada) do `installer.nsi` do repositório do Tauri (`crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi`). **Ressalva:** foi lido o repositório no GitHub (branch `dev` segundo as notas do agente), não a versão exata usada por este projeto, e o passo do plano de procurar o `installer.nsi` no registro local do cargo não foi feito (o arquivo não está lá). Não se verificou se uma instalação "por máquina" muda o que `$APPDATA` aponta. A confirmação final vem do instalador gerado (Fase 6).

**Resultado (linhas citadas pelo Haiku, conferidas só pelo agente):**
- (a) Existe a caixa "Delete the application data" (`un.ConfirmShow`, linhas 426-458).
- (b) Marcada, apaga `$APPDATA\${BUNDLEID}` e `$LOCALAPPDATA\${BUNDLEID}` (linhas 869-885), exceto em modo de atualização.
- (c) Hooks `NSIS_HOOK_PREINSTALL/POSTINSTALL/PREUNINSTALL/POSTUNINSTALL`; configuração em `bundle.windows.nsis.installerHooks`.
- (d) `$DeleteAppDataCheckboxState` é legível nos hooks; o binário ainda existe no `PREUNINSTALL`.

**Decisão:** a caixa nativa cobre a **opção 2** da spec (apagar os dados, o que inclui a pasta do browser em `%LOCALAPPDATA%`), **sem código nosso**. O modo `--cleanup` da Tarefa 4.2 só é necessário para a opção 1 ("só o browser"), que exige uma página própria de nsDialogs (80 a 130 linhas de NSIS segundo o Haiku, estimativa não verificada). Proposta ao usuário no portão: adotar só a caixa nativa na primeira versão.


## 0.6: O Chrome for Testing dispara captcha onde não deveria?

**Hipótese:** o Chrome for Testing, com a porta de debug aberta, é tratado como bot por sites de vagas e por detectores, mais do que um Chrome normal.

**O que foi feito (2026-10-02):** o Chrome for Testing 154 e o Chrome normal 154 foram abertos **com janela visível e perfil novo**, lado a lado, na mesma rede, cada um com `--remote-debugging-port` e dirigido por CDP (como o app fará). Cada um visitou, **sem logar em nada** e só em leitura, com 9 a 14 s de espera por página: `bot.sannysoft.com`, `nowsecure.nl` (Cloudflare), Google Search, LinkedIn Jobs (público), Indeed DK, Glassdoor, Jobindex, Himalayas, Wellfound, o detector de pontuação do reCAPTCHA v3 (`antcpt.com/score_detector`) e `browserscan.net/bot-detection`. O script procurou textos de desafio ("verify you are human", "just a moment", "unusual traffic", "access denied", etc.) e iframes de captcha, e salvou um screenshot da página de cada visita. Um primeiro endereço do reCAPTCHA (`recaptcha-demo.appspot.com`) deu 404 por erro nosso e foi substituído.

**Resultado:**
- **Nenhum desafio, bloqueio ou texto de captcha** em nenhum dos 12 sites, em nenhum dos dois browsers. LinkedIn (público), Indeed, Glassdoor, Jobindex, Himalayas e Wellfound carregaram o conteúdo real das vagas nos dois. Os iframes que o script marcou em LinkedIn e Himalayas são componentes invisíveis da própria página (confirmado nos screenshots).
- **Pontuação reCAPTCHA v3: 0,9 nos dois** (1,0 = humano; abaixo de 0,3 vira desafio difícil).
- **BrowserScan: "Normal"** no Chrome for Testing.
- **Sannysoft:** tudo verde nos dois; `navigator.webdriver` ausente ("missing (passed)"), `window.chrome` presente, `PHANTOM_UA` ok.
- **Única diferença visível:** `navigator.languages` no Chrome for Testing de perfil novo é só `pt-BR`; no Chrome normal é `pt-BR,pt,en-US,en`. Uma lista de um idioma é um sinal fraco de automação. Corrigir gravando `intl.accept_languages` no `Preferences` do perfil (junto do tema, na Tarefa 1.6).

**Limites do teste:** perfil novo e deslogado, poucas visitas, nenhuma interação, rede doméstica. Não mede o que os sites fazem com **muitas candidaturas seguidas**, com cliques em velocidade de máquina, nem com a conta do usuário logada. O LinkedIn real (sessão logada, candidatura enviada) já tinha funcionado por CDP em 2026-09-28 com o Chrome 153. A Tarefa 6.3 repete isso com o binário final.

**Decisão:** hipótese **não confirmada**: sem sinal de captcha extra para o Chrome for Testing. Não é argumento para trocar de binário; o motivo para reavaliar continua sendo a faixa fixa e o uso pretendido (CLA-34). Acrescentar `intl.accept_languages` ao tema (Tarefa 1.6).

## Baseline do clippy (antes da Tarefa 1.1)

**3 warnings**, todos já existentes e sem relação com o browser (2026-10-02, antes de acrescentar o módulo e as dependências): `associated function new is never used`, `function write_both is never used` e `name LASTINPUTINFO contains a capitalized acronym`. Saída completa guardada em `.superpowers/sdd/.../clippy-baseline.txt`. Os arquivos novos de `browser/` não podem somar warnings a este número.
