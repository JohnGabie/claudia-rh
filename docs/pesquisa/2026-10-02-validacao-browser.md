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
6. Consultada licença em https://raw.githubusercontent.com/GoogleChromeLabs/chrome-for-testing/main/LICENSE

**Resultado:**
- Versão fixada: `154.0.8037.92`
- SHA-256: `b897ef3601c947ac0620c784556dec719ac602b0159ce105927acf645ee0f598`
- Base URL: `https://storage.googleapis.com/chrome-for-testing-public`
- Tamanho do arquivo: 197 MB
- Licença: Apache License 2.0

**Conclusão sobre licença:**

Chrome for Testing é distribuído sob a **Apache License 2.0**. Conforme a licença:

(a) **Baixar na máquina do usuário é permitido?** SIM. A Apache 2.0 explicitamente permite: "You are granted a perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable copyright license to reproduce, prepare Derivative Works of, publicly display, publicly perform, sublicense, and distribute the Work and such Derivative Works in Source or Object form."

(b) **Redistribuir no instalador é permitido?** SIM. A Apache 2.0 permite redistribuição em qualquer meio ("in any medium, with or without modifications, and in Source or Object form"), desde que: (i) se forneça cópia da Apache License aos destinatários; (ii) se mantenham avisos de copyright e atribuição; (iii) se modifiquem os arquivos, declarem as mudanças.

**Decisão:** Hipótese confirmada. Proceder para Fase 1 — implementação das fundações.
