# Pesquisa: Android emulado como superfície de automação

**Data:** 2026-09-28
**Origem:** proposta do usuário — sandbox real, toque em vez de clique sintético, e qualquer IA consegue usar
**Como:** emulador `bruna_pixel7_api36` (Android 16) ligado nesta máquina, LinkedIn aberto no Chrome do Android, medido e desligado

## O que estava certo na proposta

**O sandbox é real, e responde a um problema que eu próprio documentei.** A CLA-30 existe porque a sessão escreveu 11 scripts e instalou 63 pacotes npm **na máquina do usuário**. Num emulador isso é impossível: o agente não alcança o sistema de arquivos do host, não vê o Chrome real, não instala nada fora da máquina virtual. Confinar o CDP exige construir guardas; o emulador já é a guarda.

**O toque é um evento de sistema**, não um `click()` sintético num DOM. Não há `isTrusted` para ninguém inspecionar.

**A árvore é melhor estruturada.** Cada elemento vem com tudo de uma vez:

```
@e39 EditText text="linkedin.com/jobs/view/..." id="com.android.chrome:id/url_bar" at=210,144 size=460x131
@e43 ImageButton label="See 16 tabs" id="com.android.chrome:id/tab_switcher_button" at=828,136 size=126x147
```

`ref` + rótulo + **coordenadas** + tamanho, numa linha. No web eu precisei de três superfícies (árvore para o rótulo, `getBoxModel` para a posição, `dispatchMouseEvent` para clicar). Aqui vem tudo junto.

**A visão móvel é mais barata:**

| | Tokens |
|---|---|
| screenshot desktop (958×888) | 1 135 |
| recorte desktop (753×388) | 390 |
| **screenshot móvel (270×600)** | **216** |

**5.3× mais barato que o desktop** — e eu li a vaga inteira a partir dele: título, empresa, "1 week ago · Over 200 applicants", botão Apply, e o início da descrição. Um ecrã estreito cabe mais informação por pixel.

## O que mata a ideia, na forma testada

**A árvore de acessibilidade do Android não expõe o conteúdo web do Chrome.**

Com a vaga do LinkedIn totalmente renderizada no ecrã — título, botão Apply, descrição, tudo visível no screenshot — a árvore devolveu:

```
@e29 FrameLayout label="Web View" at=0,136 size=1080x2201 focused
```

**Sem um único filho.** Toda a página, reduzida a um retângulo sem conteúdo.

E o pior: **na primeira leitura funcionou.** O modal de login apareceu completo, com `Dialog`, `Button text="Continue with google"`, `TextView` com o texto real. Depois de dispensar o modal, nunca mais voltou. Confirmei que não era corrida — duas leituras seguidas, ambas vazias.

Isto é a **oitava falha silenciosa** deste estudo: sem erro, sem aviso, uma árvore que parece uma resposta e não é.

Para conteúdo web dentro do Android, o agente veria "Web View" e não saberia que há uma vaga com um botão Apply.

## O que ficou por testar, e é onde a ideia sobreviveria

**A app nativa do LinkedIn.** A limitação acima é do WebView, não do Android. Uma app nativa tem views nativas, e a árvore expõe-as corretamente — é assim que os leitores de ecrã funcionam.

Não consegui testar: a imagem deste emulador é `google_apis`, **sem Play Store**. Só tem YouTube, Gmail, Maps, Chrome. Instalar o LinkedIn exigiria uma imagem `google_apis_playstore` ou fazer sideload do APK.

**É esse o teste que decide a proposta**, e continua por fazer.

## Os custos, com números reais desta máquina

| | |
|---|---|
| imagem de sistema (android-36, Google APIs) | 4.3 GB |
| emulador | 1.1 GB |
| platform-tools (adb) | 17 MB |
| **mínimo** | **~5.5 GB** |

Android Studio (3.3 GB), `sources` e `build-tools` não são precisos — são ferramentas de quem programa, não do app.

**A licença proíbe embutir.** [Termos do Android SDK](https://developer.android.com/studio/terms), secção 3.4: *"you may not copy (…), modify, adapt, **redistribute** (…) the SDK or any part of the SDK"*, e a 3.1 concede licença **não-sublicenciável**.

Há uma saída legal e usada por outros (Flutter, React Native): o app **descarrega** do Google na primeira execução e o usuário aceita a licença da Google dentro da nossa interface. O licenciado passa a ser o usuário. Mas são ~5.5 GB antes de o app fazer seja o que for.

**Outros custos observados ao correr:**

- **Arranque: ~2 minutos** para o boot completo. Não é algo que se ligue e desligue por sessão.
- **Login de novo.** O Chrome do emulador é um perfil limpo — apareceu o muro de "Sign in to see who you already know". Todas as contas teriam de ser autenticadas dentro do emulador.
- **Configuração frágil.** O `ANDROID_AVD_HOME` desta máquina aponta para `D:\Android\avd`, e o AVD que eu via em `~/.android/avd` era invisível ao emulador. Numa máquina de usuário, cada variação dessas é um caminho de falha.

## Conclusão

**Para conteúdo web: não funciona.** A árvore não expõe o WebView, e sem árvore o Android perde a única vantagem estrutural que tinha sobre o CDP. Sobra a visão — que é mais barata (216 vs 390 tokens), mas não justifica 5.5 GB, dois minutos de arranque e logins novos.

**Para a app nativa: por testar, e é a única forma que sobrevive.** Se a app do LinkedIn expuser uma árvore limpa, a proposta volta com força: sandbox real, toque de sistema, árvore com coordenadas incluídas, e visão barata.

**O que fazer a seguir, se valer a pena:** uma imagem `google_apis_playstore`, instalar o LinkedIn, autenticar, e medir a árvore de um ecrã de vaga. Meio dia. Se a árvore vier limpa, a decisão muda.

## O que continua verdadeiro sem isto

O padrão que eu já tinha proposto para o CDP não depende do Android:

```
LER       ← Accessibility.getFullAXTree   ✓ testado, dá os rótulos
LOCALIZAR ← DOM.getBoxModel               coordenadas do elemento
CLICAR    → Input.dispatchMouseEvent      ✓ testado, acertou o alvo visual
```

É a mesma combinação que torna o `mobile-mcp` bom, disponível no Chrome do usuário, sem instalar nada.
