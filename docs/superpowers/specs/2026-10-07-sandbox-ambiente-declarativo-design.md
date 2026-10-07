# Ambiente do Sandbox declarado num arquivo (dependências instaladas no guest)

**Estado:** APROVADA pelo usuário em 2026-10-07 ("Aprovo"), com A1, A2, A4 (conta logada), A6 (VHD como Z:) e A7. Nada implementado ainda.

**A4, resposta do usuário (2026-10-07):** "não, tudo na mesma env". As contas não ficam em pastas separadas: ele troca de conta com `/login` no mesmo Claude Code. Então a conta usada no guest é **a que está logada no PC na hora do `test-up`**. Saem o `-Conta` e o `contas.json`. O resto da proposta A4 continua valendo: copiar só o arquivo de credencial, achar o caminho em tempo de execução, montar só leitura e apagar a cópia no `test-down` com recusa pura. O risco da renovação do token continua **não verificado** e é o primeiro teste no Sandbox.
**Branch:** `feat/sandbox-ambiente` (a partir de `feat/browser-cdp-mcp`, depois do merge do laboratório).
**Bloqueia:** a T9 do plano de fechamento do browser na parte de testes em Rust dentro do Sandbox (`cargo test`, testes ao vivo).

## 1. Pedido do usuário

Contexto: o perfil `lab.config.app-build.json` monta no guest, só leitura, as toolchains do host (Node, Rust, MSVC, Windows SDK). O Rust montado é a pasta `stable` do host, que está quebrada desde o incidente de 2026-10-05 (sem a biblioteca padrão). Perguntei se apontava para o 1.95.0, consertava o stable ou subia sem Rust. Resposta, nas palavras dele:

> "Eu prefiro que instale as dependencias dentro da sandbox. Tem que instalar a claude code tbm. Faz essas configs pra isso virar igual um docker-compose.yml. a unica coisa que podemos discutir é apontar para minha conta do claude. Mas isso tem que fazer de um jeito que nao vaze path ou no git"

## 2. Leitura do pedido

1. **Dependências instaladas dentro do Sandbox**, não emprestadas do host. O guest deixa de depender das pastas de toolchain do PC (e do estado delas, como o `stable` quebrado).
2. **Claude Code também instalado no guest.**
3. **Configuração declarativa num arquivo, "igual um docker-compose.yml"**: um lugar só que diz o que cada ambiente tem e o guest monta sozinho a partir dele.
4. **Aberto para discussão: o Claude Code do guest usar a conta Claude do usuário.**
5. **Restrição:** nenhum caminho da máquina e nenhum segredo no git, em log ou em arquivo versionado.

## 3. Propostas do assistente (NÃO decididas)

### A1. Formato e lugar do arquivo

- **Opção A (recomendada):** `sandbox-lab/compose.json`, com o desenho de um docker-compose: uma lista de **ambientes** com nome (como os "services"), e cada um diz rede, memória, pastas montadas, **o que instalar** e o que rodar no fim. Os `lab.config.*.json` atuais viram ambientes desse arquivo e saem.
  Motivo do JSON: o guest roda PowerShell 5.1, que lê JSON nativamente e não lê YAML. Os perfis atuais já são JSON e o `test-up.ps1` já os lê.
- Opção B: `sandbox-lab/compose.yml` em YAML de verdade. Exige um leitor de YAML sem dependências (nem o Node nem o PowerShell 5.1 têm um embutido): um subconjunto escrito à mão no host, que converte para o `guest.config.json`. É mais código e mais um lugar para errar.

Exemplo da opção A (versões ilustrativas, a fixar no plano):

```json
{
  "ambientes": {
    "testes": {
      "rede": true,
      "memoriaMB": 8192,
      "instalar": ["node", "rust", "msvc", "claude-code"],
      "montar": ["projeto:ro", "trabalho:rw"]
    },
    "app": {
      "rede": true,
      "internet": false,
      "memoriaMB": 6144,
      "instalar": ["claude-code"],
      "montar": ["deploy:ro", "trabalho:rw"]
    }
  },
  "pacotes": {
    "node":        { "versao": "22.x.y",  "arquivo": "node-v22.x.y-win-x64.zip",        "sha256": "..." },
    "rust":        { "versao": "1.95.0",  "arquivo": "rust-1.95.0-x86_64-pc-windows-msvc.msi", "sha256": "..." },
    "msvc":        { "versao": "2022",    "arquivo": "vs_buildtools-layout/", "componentes": ["VCTools", "Windows11SDK"] },
    "claude-code": { "versao": "x.y.z",   "arquivo": "claude-x.y.z-win32-x64.exe",     "sha256": "..." }
  }
}
```

Uso: `scripts\test-up.ps1 -Ambiente testes -Project <projeto do onde.mjs>`. Os nomes de montagem (`projeto`, `trabalho`, `deploy`) são resolvidos pelo script a partir da raiz do repositório. O arquivo nunca tem caminho do host.

### A2. De onde vêm os instaladores: cache no host

O Sandbox é descartado a cada vez. Baixar tudo da internet a cada boot é lento: só o MSVC Build Tools tem alguns GB (**tempo não verificado**).

- **Recomendado:** um script novo, `scripts\cache-up.ps1`, baixa **uma vez** as versões fixadas no `compose.json` para `sandbox-lab/shared/cache/` (fora do git, como `shared/tools/`), confere o SHA-256 e grava um manifesto. O guest instala a partir do cache, sem internet. Para o MSVC, um "layout offline" do instalador do Visual Studio.
- Regras de disco do `cache-up.ps1`: **só cria e escreve** dentro de `shared/cache/`. Arquivo novo vai para `.tmp` e só depois é renomeado. **Não apaga nada**: versão antiga fica até o usuário apagar à mão.
- Alternativa: instalar da internet a cada boot (`rustup`, `winget`). É mais simples, mas fica lento e depende da rede a cada teste.

### A3. Instalação no guest

Um `sandbox\instalar.ps1` roda no `LogonCommand` depois do `bootstrap.ps1` e instala, nesta ordem, o que o ambiente pede: MSVC, depois Rust (que precisa do linker do MSVC), Node e Claude Code. Grava `C:\lab\out\instalar.json` com versão, tempo e código de saída de cada pacote. O `test-up.ps1` espera esse arquivo, como já espera o `status.json`. O target do cargo fica no disco do guest, como hoje (`references/limites.md`: em pasta montada os build scripts falham).

O `guest-build.ps1` deixa de procurar `C:\hosttools` e usa o que foi instalado. O mapeamento das toolchains do host (`%USERPROFILE%\.rustup\...`, `%ProgramFiles%\nodejs`, MSVC, Windows Kits) **sai** do laboratório.

### A4. Conta do Claude no guest (ponto que você quis discutir)

| Opção | Como funciona | Vaza caminho? | Vaza segredo? | Custo |
|---|---|---|---|---|
| **1. Login a cada boot** | Você roda `claude` no guest e faz login pelo browser de lá | não | não sai do guest | login manual toda vez que sobe |
| **2. Token de longa duração (recomendada)** | Você gera uma vez, no seu PC, `claude setup-token` e salva a saída em `sandbox-lab/shared/secrets/claude-token` (pasta já ignorada no git, a mesma da chave do MCP). O guest lê o arquivo e passa o token ao Claude Code por variável de ambiente só no processo dele. | não: o nome é relativo ao laboratório | não, se: nunca entra em log, `status.json`, print ou `guest.config.json`; os scripts gravam só "token: presente/ausente" | um passo seu, uma vez; o token é revogável na conta |
| 3. Copiar a pasta `.claude` do seu usuário | — | **sim** (pasta do usuário) | **sim** (sessões, memória, configurações) | **descartada** |

Riscos da opção 2: dentro do guest o token fica disponível para qualquer processo que rode lá. O guest é descartado ao desligar, e o perfil de testes fica com a internet bloqueada pelo firewall, com exceção só da API da Anthropic (lista a definir no plano). Cada sessão do Claude Code no guest **gasta a sua cota**.

### A5. Nada de caminho nem segredo no git

- O `compose.json` só aceita nomes (`projeto`, `trabalho`, `deploy`, `cache`) e caminhos do guest (`C:\lab...`). Uma função pura nova (`Get-LabComposeRefusal`) recusa caminho absoluto do host, `%USERPROFILE%` ou variável de ambiente. Ela tem teste puro no PC e roda no CI.
- O `conferir-repo.mjs`, que já procura caminho de máquina, passa a procurar também padrões de token no texto versionado (prefixo dos tokens da Anthropic).
- `shared/cache/` entra no `.gitignore` e na lista de proibidos do `proibidos.mjs`.

### Decisões do usuário (2026-10-07)

- **A1:** `compose.json` (opção A).
- **A2:** cache no PC (`cache-up.ps1` só cria, nunca apaga).
- **A4:** nenhuma das opções da tabela. O usuário quer usar **a conta que já está logada no PC dele** e tem **mais de uma conta**. Proposta (NÃO decidida), a detalhar antes do plano:
  - Na subida, o `test-up.ps1` copia **só o arquivo de credencial** da conta escolhida, nunca a pasta `.claude` inteira: nada de sessões, memória nem configurações. O caminho é achado em tempo de execução pela API do Windows (`GetFolderPath('UserProfile')`), nunca escrito em arquivo versionado. A cópia vai para `.sandbox-work/` (fora do git), é montada só leitura no guest e é apagada pelo `test-down.ps1` através de uma função de recusa pura (só aquele arquivo, dentro de `.sandbox-work/`).
  - Para escolher a conta: `-Conta <nome>`, mapeado num arquivo local fora do git (`sandbox-lab/shared/secrets/contas.json`: nome → pasta de configuração daquela conta). **Em aberto:** como o usuário separa as contas hoje (pastas de configuração diferentes? trocando o login?).
  - **Risco, não verificado:** se o Claude Code do guest renovar o token, a credencial do PC pode deixar de valer e o PC pede login de novo. Precisa de teste no Sandbox antes de adotar. Se acontecer, o recuo é o token de longa duração (opção 2).

### A6. Disco Z: no guest para tudo o que os testes criam e apagam (pedido do usuário em 2026-10-07)

Pedido: "Tem como mudar para que os arquivos do sandbox sejam outra letra e não C? poderia ser uma aleatória com Z". Objetivo: um teste que apaga e rode por engano no PC mira em `Z:\`, que não existe no host (hoje o host tem C:, D: e G:), e falha sem tocar em nada.

Investigação ao vivo no Sandbox (2026-10-07):
- **MappedFolder em Z:** só pela documentação, que não fala de outra letra. O guest só tem C:. Mesmo que funcionasse, seria uma pasta do host: o que fosse apagado ali seria apagado no host. **Descartada.**
- **`subst Z:`:** funciona para a conta que o criou, mas `fs.realpathSync.native` (usado pelo `tempDir()` e pelo `novoCenarioDir()`) devolve o caminho real em `C:\`. As travas não enxergam o subst. Dá uma falsa sensação de segurança. **Descartada.**
- **VHD montado como letra (verificado, com a letra V:):** o `diskpart` como System leva cerca de 7 s; a letra vale para todas as contas, sem reparse point, e o `realpathSync.native` continua na letra. Node e PowerShell criam e apagam normalmente.

Proposta (NÃO decidida):
- O `bootstrap.ps1` (ou o `test-up.ps1` via `wsb exec -r System`, se o LogonCommand não tiver admin; **não verificado**) cria `C:\labdata\z.vhdx`, monta como **Z: fixo** e cria `Z:\tmp`. Antes de formatar, seleciona o vdisk pelo arquivo e confere a letra.
- `util.mjs tempDir()`, `comum.mjs novoCenarioDir()` e `travas.sandbox.ps1` usam a raiz fixa `Z:\claudia-teste-sandbox` (não `os.tmpdir()`) e recusam caminho cujo real path não comece com `Z:\`. `testes-no-guest.mjs` passa `TEMP=Z:\tmp`. Revisar `cenario-copia.mjs:87` (usa `TEMP`).
- O `test-up.ps1` recusa subir se o host tiver Z:.
- É uma trava a mais: não substitui a de dois fatores (`exigirSandbox()`).

### A7. Correção no `travas.sandbox.ps1`

O teste "a source that is a drive root is refused" passa `Q:\` ao `app-stage.ps1`. Isso viola a regra "nunca passar raiz de disco, nem para testar a recusa". Proposta: remover esse caso e cobrir a recusa de raiz de disco só no teste puro `travas.test.ps1`.

## 4. Impacto

- **Laboratório:** `test-up.ps1` (lê o ambiente do `compose.json`), novo `cache-up.ps1`, novo `sandbox\instalar.ps1`, `guest-build.ps1` (sem `C:\hosttools`), os `lab.config.*.json` saem, `travas.ps1` (recusa do compose), `.gitignore`, `proibidos.mjs`, `conferir-repo.mjs`, README e skill `claudia-teste-sandbox` (seções 2 e 9).
- **App:** nenhum.
- **Dados do usuário:** nenhum, além do token, se for a opção 2, que fica em `shared/secrets/`.
- **Disco do host:** o `cache-up.ps1` grava alguns GB em `sandbox-lab/shared/cache/` (**tamanho não verificado**). Hoje há 250 GB livres.
- **Segurança:** token no guest (A4). Firewall do guest com exceção da Anthropic.

## 5. Critérios de teste

- **Puros (PC):** `Get-LabComposeRefusal` recusa caminho do host, variável e nome desconhecido, e aceita os nomes válidos. Leitura do `compose.json` gera o `guest.config.json` esperado. Detector de token no `conferir-repo`. Todos com prova de mutação.
- **Sandbox:** subir `testes`; `instalar.json` com todos os pacotes em exit 0; `rustc --version` = 1.95.0, `node --version`, `cl.exe` encontrado, `claude --version`; `cargo test --lib` do projeto passa dentro do guest; com a opção 2, `claude -p "diga ok"` responde, e o token não aparece em nenhum arquivo de `C:\lab\out` nem `C:\labwork` (busca pelo prefixo).
- **Cache:** rodar o `cache-up.ps1` duas vezes; a segunda não baixa nada e não apaga nada (listagem antes e depois).

## 6. Fora do escopo

- Docker de verdade ou container Linux (o app é Windows e o Sandbox é a máquina limpa).
- Instalar o app pelo `compose.json` (continua com `publicar.mjs` e o instalador).
- Atualização automática das versões fixadas.

## 7. Pontos em aberto e não verificado

- **Não verificado:** tempo de instalação do MSVC Build Tools e do Rust no guest a cada boot; se o layout offline do Visual Studio instala sem rede no Sandbox; se o instalador nativo do Claude Code roda no Sandbox (conta `WDAGUtilityAccount`).
- Se o MSVC for lento demais a cada boot, alternativa a decidir: instalar o MSVC no guest uma vez e guardar a pasta instalada no cache. Isso é diferente de emprestar a do host e é **não verificado** para o MSVC.
- Versão exata de Node e de Claude Code a fixar.
- **Enquanto isto não existe:** posso subir o Sandbox hoje no perfil atual para os testes de Node e PowerShell, que não precisam de Rust, e deixar os de Rust para depois desta spec.

## 8. Adendo de 2026-10-07 (plano) — APROVADO pelo usuário em 2026-10-07 ("Aprovo")

Origem: ao escrever o plano, apareceu um ponto em que a spec, ao pé da letra, quebrava o uso normal (D1), e o usuário decidiu também como montar o cache do MSVC. Decisões do usuário em 2026-10-07: D1 = "Cenários só no Sandbox"; MSVC = layout montado dentro de um Sandbox; D2 = todas aprovadas. Este adendo descreve **como** isso fica. O texto abaixo só vale depois de o usuário aprovar este trecho.

### 8.1 D1: cenários só no Sandbox (substitui a parte de `novoCenarioDir()` da A6)

- `cenario-novo.mjs`, `cenario-mock.mjs` e `cenario-copia.mjs` passam a rodar **só dentro do guest**: `novoCenarioDir()` recusa fora do Sandbox (os mesmos dois fatores de `exigirSandbox()`) e grava em `Z:\claudia-teste-sandbox\<cenario>`, com o caminho real começando em `Z:\`. Nada de cenário no PC.
- O host dispara um cenário com `node .claude/skills/claudia-teste-sandbox/scripts/cenario-no-guest.mjs --cenario novo|mock|copia [--autorizado] [--manter-automacao]`, que roda no guest `C:\labsandbox\app\guest-cenario.ps1` pelo `wsb exec`. O resultado volta por `C:\labwork\run\cenario.json`. O Node usado é o que o guest instalou, então os ambientes `app` e `app-winapp` passam a instalar `node`.
- **`copia` com os dados reais:** o `test-up.ps1` só monta os dados reais com a chave `-DadosReais`. Ela é passada **só com autorização do usuário naquela rodada**. A pasta é achada em tempo de execução: `[Environment]::GetFolderPath('ApplicationData')` + `io.github.johngabie.claudia-rh`. Nunca fica escrita num arquivo versionado, e nunca é `%LOCALAPPDATA%`.
  - A pasta é montada **só leitura** em `C:\labdadosreais`.
  - Uma função pura recusa qualquer outra pasta, links no caminho e nomes 8.3.
  - Dentro do guest, o `cenario-copia.mjs` usa `C:\labdadosreais` como origem e continua exigindo `--autorizado`.
  - A cópia dos dados reais fica só no `Z:` do guest e some no `test-down`.
- **Publicar:** o `publicar.mjs --app <exe|instalador|pasta>` publica só o app (sem `--cenario`). O `app-stage.ps1` perde `-AppData` e `-LocalAppData`. No guest, `guest-run.ps1 -Cenario Z:\claudia-teste-sandbox\<cenario>` copia `appdata` e `localappdata` do cenário para o perfil do guest; ele só aceita uma pasta debaixo de `Z:\claudia-teste-sandbox\`.

### 8.2 MSVC: layout offline montado dentro de um Sandbox

- O PC não instala nem executa nada do Visual Studio. No host, o `cache-up.ps1` só **baixa** o bootstrapper de versão fixa (`vs_buildtools-<versão>.exe`, SHA-256 do `compose.json`), como os outros pacotes.
- Um ambiente novo, `cache-msvc` (rede ligada), monta só leitura o cache (via `shared\` → `C:\lab\cache`) e, gravável, `.sandbox-work\cache-saida\` em `C:\labcachesaida`. O passo dentro do guest é feito pelo `sandbox\layout-msvc.ps1`, rodado como System:
  - confere o SHA-256 do bootstrapper;
  - monta o layout em `C:\labdata\msvc-layout` (disco do guest);
  - grava a lista de arquivos com tamanho e SHA-256 (`layout.sha256`);
  - copia tudo para uma pasta nova `C:\labcachesaida\<id>\msvc-<versão>\`;
  - grava o resultado em `C:\lab\out\layout-msvc.json`.
- No host, o `cache-up.ps1 -ImportarMsvc <id>`:
  - recusa uma origem que não seja exatamente `.sandbox-work\cache-saida\<id>\msvc-<versão>` ou que tenha link;
  - confere **no host** o SHA-256 e o tamanho de cada arquivo contra o `layout.sha256` (falta, sobra ou diferença → para sem copiar);
  - copia arquivo por arquivo para uma pasta nova `.tmp` em `shared\cache\`, sem sobrescrever;
  - confere de novo e só então renomeia para `msvc-<versão>`.
- Regra de disco no host: o `cache-up.ps1` continua **só criando**. Nunca apaga nem sobrescreve, nem no `shared\cache\`, nem no `.sandbox-work\cache-saida\`. Limpar a `cache-saida` é tarefa manual do usuário.
- **Não verificado:** se o layout montado no guest depende de algo da máquina; quanto tempo leva e quanto ocupa; se o `--layout` precisa de administrador (por isso o passo roda como System).
