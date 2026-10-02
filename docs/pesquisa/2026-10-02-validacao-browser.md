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

**Decisão:** o pin está correto e verificado, mas a **escolha do Chrome for Testing como browser do produto não está confirmada**. Alternativas a decidir com o usuário no portão 0.5: (A) manter o Chrome for Testing, aceitando o risco de uso e de segurança e prometendo atualizar a versão a cada release; (B) **Chromium** (open source, licença BSD, redistribuível, mesmo motor, também sem auto-update); (C) usar o Chrome do usuário com perfil dedicado (volta ao plano anterior; depende de ele ter Chrome).
