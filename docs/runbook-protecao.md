# Runbook de proteção e de ativação

Comandos para ativar, conferir e desfazer as proteções do `origin` (`JohnGabie/claudia-rh`) e do clone local. Spec de referência: `docs/superpowers/specs/2026-10-19-regras-agentes-design.md` (10.8, 10.9 e P15); agente: `agents/versao.md` ("Proteção de branch").

Este documento foi **escrito, não executado**. Os únicos comandos rodados ao escrevê-lo foram GET de leitura (seção 1).

## 0. Avisos

- **Nada que mude estado roda sem o "pode aplicar" do usuário.** Vale para as seções 2, 3, 4, 6, 7 e 9 e para a execução real do pacote de recuperação (8.2, que grava objetos no `.git` e a ref `refs/heads/recovery`). Agente nenhum aplica, desliga ou "contorna" proteção por conta própria. A seção 1, a seção 5 (conferência) e o `-WhatIf` da seção 8 são só leitura.
- **O administrador também fica sujeito às regras** (`enforce_admins` = true, ruleset de tag sem lista de bypass). Depois de ativar, nem o dono empurra direto em `main`/`dev` nem move tag `v*`. Emergência = desligar a regra (seção 6), agir, religar (seção 2 ou 4), tudo com aviso ao usuário.
- **Ordem recomendada:**
  1. PR de sincronização do `dev` (seção 9, tarefa T14): o `dev` local está muito à frente do `origin/dev` e, com a proteção ativa, esses commits só chegam por PR. Fazer antes evita ter de desligar a regra depois. O PR de sincronização não depende do hook (o hook, passo 2, vem depois); se o hook estiver ativo, ele aceita a branch `sync/*` por ref nomeada.
  2. Hook local (seção 7) e primeiras execuções (seção 8), em qualquer ordem.
  3. Repositório só com merge commit (seção 3).
  4. Proteção de `dev`, depois de `main` (seção 2).
  5. Ruleset de tags (seção 4).
  6. Conferir tudo (seção 5).
- **Conferir o estado atual só lendo:** seção 1. Antes de afirmar em qualquer relatório que a proteção está (ou não) ativa, rode os GET da seção 1/5.
- Todos os comandos usam o `gh` já autenticado como dono do repositório. Os blocos são PowerShell; o JSON vai por here-string (`@'...'@`, o `'@` de fechamento na coluna 0) e `--input -`, sem arquivo temporário.

## 1. Pré-checagem (só leitura)

### 1.1 Nomes exatos dos checks

O contexto que o GitHub usa na proteção é o `name` do check-run (o `name:` de cada job em `.github/workflows/ci.yml`). Para listar a partir do último commit do `dev` no `origin`:

```powershell
$sha = gh api repos/JohnGabie/claudia-rh/branches/dev --jq .commit.sha
gh api "repos/JohnGabie/claudia-rh/commits/$sha/check-runs" --jq '.check_runs[].name'
```

**Nomes conferidos em 2026-10-20** (commit `98da02b4ea59ec377d5e1ce4c551e7f161769408`, ponta do `origin/dev` naquele dia, workflow `ci`, push em `dev`, os 5 com conclusão `success`, todos do app GitHub Actions, `app.id` = 15368; GET terminou com código de saída 0):

```
windows (exe)
npm ci (build)
cargo test
npm install (fresh)
repo guard (lab files, keys, machine paths)
```

São exatamente os 5 jobs da spec. Os mesmos nomes aparecem nos PRs porque o `ci.yml` também dispara em `pull_request`. **Não verificado:** que os nomes de um check-run de PR são idênticos aos de push (o esperado, pois vêm do mesmo job); confirmar no primeiro PR (T14) com `gh pr checks`.

### 1.2 Estado atual de proteção e de rulesets

```powershell
gh api repos/JohnGabie/claudia-rh/branches/main/protection
gh api repos/JohnGabie/claudia-rh/branches/dev/protection
gh api repos/JohnGabie/claudia-rh/rulesets
gh api repos/JohnGabie/claudia-rh --jq '{private,default_branch,allow_squash_merge,allow_rebase_merge,allow_merge_commit}'
```

Esperado **antes** de ativar (conferido em 2026-10-20):

- `branches/main/protection` e `branches/dev/protection`: erro HTTP 404, `{"message":"Branch not protected"...}`. O `gh` sai com código 1; é o resultado normal de "sem proteção".
- `rulesets`: `[]`.
- repositório: `{"allow_merge_commit":true,"allow_rebase_merge":true,"allow_squash_merge":true,"default_branch":"main","private":false}`.

## 2. Proteção de `main` e de `dev`

Regras (spec 10.8/10.9): PR obrigatório com 0 aprovações; os 5 checks obrigatórios; branch atualizada antes do merge (`strict`); sem force push; sem apagar a branch; `enforce_admins` = true.

Plano gratuito com repositório público: pela documentação oficial do GitHub, a proteção de branch e os rulesets estão disponíveis em repositórios públicos no plano gratuito. **Não verificado na prática**: a resposta da API ao aplicar é a prova; se vier erro 403 ("Upgrade to GitHub Pro or make this repository public"), pare e avise o usuário, sem tentar outro caminho.

Notas sobre o JSON:

- `checks[].app_id` = `15368` é o app GitHub Actions (conferido na seção 1.1); fixa a origem do check, e um check "verde" de outro app não vale.
- `required_approving_review_count` = 0: o PR é obrigatório, mas a aprovação é a do usuário na conversa ("pode fechar"), não um review do GitHub.
- `restrictions` = `null` (repositório de conta pessoal não tem restrição por equipe/usuário).
- Efeito do `strict`: se o `dev` andar depois de o PR abrir, o PR precisa ser atualizado e o CI roda de novo antes do merge.

### 2.1 `dev`

```powershell
$body = @'
{
  "required_status_checks": {
    "strict": true,
    "checks": [
      { "context": "npm ci (build)", "app_id": 15368 },
      { "context": "npm install (fresh)", "app_id": 15368 },
      { "context": "repo guard (lab files, keys, machine paths)", "app_id": 15368 },
      { "context": "cargo test", "app_id": 15368 },
      { "context": "windows (exe)", "app_id": 15368 }
    ]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": {
    "required_approving_review_count": 0,
    "dismiss_stale_reviews": false,
    "require_code_owner_reviews": false
  },
  "restrictions": null,
  "required_linear_history": false,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_conversation_resolution": false
}
'@
$body | gh api -X PUT repos/JohnGabie/claudia-rh/branches/dev/protection --input -
```

### 2.2 `main`

Mesmo JSON, outra branch:

```powershell
$body = @'
{
  "required_status_checks": {
    "strict": true,
    "checks": [
      { "context": "npm ci (build)", "app_id": 15368 },
      { "context": "npm install (fresh)", "app_id": 15368 },
      { "context": "repo guard (lab files, keys, machine paths)", "app_id": 15368 },
      { "context": "cargo test", "app_id": 15368 },
      { "context": "windows (exe)", "app_id": 15368 }
    ]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": {
    "required_approving_review_count": 0,
    "dismiss_stale_reviews": false,
    "require_code_owner_reviews": false
  },
  "restrictions": null,
  "required_linear_history": false,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_conversation_resolution": false
}
'@
$body | gh api -X PUT repos/JohnGabie/claudia-rh/branches/main/protection --input -
```

**Não verificado:** se `enforce_admins` e `strict` atrapalham o fluxo de release (`dev` → `main` por PR, depois a tag `v*.*.*` que dispara o `release.yml`). A tag não passa por regra de branch; o PR de release segue as mesmas regras de qualquer PR.

## 3. Repositório: só merge commit

```powershell
gh api -X PATCH repos/JohnGabie/claudia-rh -F allow_squash_merge=false -F allow_rebase_merge=false -F allow_merge_commit=true
```

Squash e rebase geram commits diferentes dos da branch, e o `dev` local divergiria do `origin/dev`; por isso só merge commit (`gh pr merge --merge`).

## 4. Tags `v*`: ruleset que impede mover e apagar

```powershell
$body = @'
{
  "name": "protect-release-tags",
  "target": "tag",
  "enforcement": "active",
  "conditions": {
    "ref_name": { "include": ["refs/tags/v*"], "exclude": [] }
  },
  "rules": [
    { "type": "update" },
    { "type": "deletion" }
  ]
}
'@
$body | gh api -X POST repos/JohnGabie/claudia-rh/rulesets --input -
```

Guarde o `id` que a resposta devolve (precisa dele na seção 6). Se perder, `gh api repos/JohnGabie/claudia-rh/rulesets --jq '.[] | [.id,.name] | @tsv'`.

- **Criar tag continua possível ao usuário:** o ruleset não tem a regra `creation`, só `update` e `deletion`. É o que permite lançar: o `release.yml` roda em tags `v*.*.*` e publica (release e npm). O agente de versão nunca cria tag `v*` sem o usuário mandar.
- Sem lista de bypass: nem o dono move ou apaga uma tag `v*` publicada. Corrigir um release errado = nova tag (ex.: `v0.2.1`), ou desligar o ruleset (seção 6) por decisão do usuário.
- O **hook local também recusa mover tag** no `origin` (qualquer atualização de tag existente, mesmo para o mesmo commit) e recusa apagar (seção 7). Duas camadas: a do servidor vale para qualquer máquina, a do hook protege este PC antes de o push sair.
- **Não verificado:** a resposta da API para ruleset de tag em repositório público no plano gratuito (a documentação oficial diz que rulesets existem nesse caso); e se o padrão `refs/tags/v*` do ruleset (mais amplo que o `v*.*.*` do `release.yml`) cobre os formatos que o hook aceita (`v` + dígitos e pontos, sufixo opcional). Cobre: todo nome que começa com `v`.

## 5. Conferir depois de aplicar (só leitura)

```powershell
# Proteção: dev e main
gh api repos/JohnGabie/claudia-rh/branches/dev/protection --jq '{strict: .required_status_checks.strict, checks: [.required_status_checks.checks[].context], enforce_admins: .enforce_admins.enabled, aprovacoes: .required_pull_request_reviews.required_approving_review_count, force_push: .allow_force_pushes.enabled, apagar: .allow_deletions.enabled}'
gh api repos/JohnGabie/claudia-rh/branches/main/protection --jq '{strict: .required_status_checks.strict, checks: [.required_status_checks.checks[].context], enforce_admins: .enforce_admins.enabled, aprovacoes: .required_pull_request_reviews.required_approving_review_count, force_push: .allow_force_pushes.enabled, apagar: .allow_deletions.enabled}'

# Repositório
gh api repos/JohnGabie/claudia-rh --jq '{allow_squash_merge,allow_rebase_merge,allow_merge_commit}'

# Ruleset de tags
gh api repos/JohnGabie/claudia-rh/rulesets --jq '.[] | {id,name,target,enforcement}'
```

O que esperar:

- **dev e main:** `strict: true`; `checks` com os 5 nomes da seção 1.1; `enforce_admins: true`; `aprovacoes: 0`; `force_push: false`; `apagar: false`.
- **repositório:** `allow_squash_merge: false`, `allow_rebase_merge: false`, `allow_merge_commit: true`.
- **rulesets:** um item `protect-release-tags`, `target: "tag"`, `enforcement: "active"`.
- Prova de efeito (decisão do usuário, não faz parte da conferência automática): um push direto de teste em `dev` deve ser recusado pelo servidor, e um PR sem os 5 checks verdes deve ficar com o merge bloqueado (`gh pr checks <pr>`; o botão de merge fica desabilitado na página do PR). **Não verificado até alguém rodar.**

## 6. Desfazer cada item

**Desligar é decisão do usuário, só com o "pode desligar" dele, e deve ser religado assim que a emergência acabar** (volte à seção 2, 3 ou 4). Enquanto desligado, a estabilidade depende só das regras locais e do cuidado de quem age.

```powershell
# Proteção de dev e de main (desliga tudo da branch)
gh api -X DELETE repos/JohnGabie/claudia-rh/branches/dev/protection
gh api -X DELETE repos/JohnGabie/claudia-rh/branches/main/protection

# Repositório: volta a aceitar squash e rebase
gh api -X PATCH repos/JohnGabie/claudia-rh -F allow_squash_merge=true -F allow_rebase_merge=true -F allow_merge_commit=true

# Ruleset de tags: obtém o id pelo nome e apaga
$id = (gh api repos/JohnGabie/claudia-rh/rulesets | ConvertFrom-Json) | Where-Object { $_.name -eq 'protect-release-tags' } | ForEach-Object { $_.id }
if ("$id" -match '^\d+$') { gh api -X DELETE "repos/JohnGabie/claudia-rh/rulesets/$id" } else { "Ruleset protect-release-tags nao encontrado (ou mais de um): id=[$id]. Nada apagado." }
```

Para afrouxar só uma regra sem derrubar a proteção inteira, a API tem sub-recursos (por exemplo `DELETE .../branches/<b>/protection/enforce_admins`), mas o caminho mais simples e menos sujeito a erro é o DELETE acima seguido de religar com o JSON da seção 2.

Depois de desfazer, confirme com a seção 1.2 (404 nas duas branches e `rulesets` vazio).

## 7. Hook local `pre-push` (decisão do usuário, uma vez por clone)

O hook protege o `origin` no PC: recusa `--all`, `--tags`, `--mirror`, branches de trabalho, `recovery*`, tags internas, force push, deleção e atualização de tag `v*`. Lista e regras: cabeçalho de `scripts/dev/git-hooks/pre-push.ps1`. Ele protege a máquina; não substitui as regras do servidor.

```powershell
# ver o que seria feito (nada é gravado)
powershell -NoProfile -File scripts/dev/install-hooks.ps1 -WhatIf

# ativar (uma vez por clone); o instalador também confere
powershell -NoProfile -File scripts/dev/install-hooks.ps1
```

- **Conferência = o próprio instalador.** Exit 0: instalado (ou já estava) e conferido por um push de teste a um repositório temporário. Exit 1: conflito com outro hook que ele não instalou, ou o hook NÃO está em vigor porque `core.hooksPath` aponta para outra pasta (ele mostra as duas opções: (1) `git config --unset core.hooksPath` ou (2) apontar o `core.hooksPath` para a pasta comum; o instalador nunca mexe na configuração, é decisão do usuário), ou a conferência real falhou (`CHECK FAILED`): não confiar no hook. Exit 2: problema de ambiente.
- **Quando `scripts/dev/git-hooks` mudar** (merge de uma nova versão), rode o `install-hooks.ps1` de novo: as cópias instaladas não se atualizam sozinhas.
- **Desligar é decisão do usuário:** remover à mão os dois arquivos (`pre-push` e `pre-push.ps1`) da pasta de hooks comum (`git rev-parse --git-common-dir`, subpasta `hooks`), ou ajustar o `core.hooksPath` se for ele que está em vigor. O agente não faz.
- Vale para todas as worktrees e branches do clone, porque a pasta de hooks é comum (não depende de a branch ter `scripts/dev/git-hooks`). O hook aceita no máximo **uma branch de trabalho por push** ao público. Outro clone precisa do mesmo comando.
- **Válvula de escape (só o usuário):** `CLAUDIA_ALLOW_PUBLIC_PUSH=1` libera o push ao repositório público **inteiro**, deleções incluídas, com um aviso. Use só de propósito, na mesma linha do comando (`$env:CLAUDIA_ALLOW_PUBLIC_PUSH = '1'; git push ...; Remove-Item Env:CLAUDIA_ALLOW_PUBLIC_PUSH`), e nunca deixe a variável definida na sessão. **Agente nenhum a usa sem pedido explícito do usuário.**
- Ativar e desligar o hook é decisão do usuário, como a proteção de branch.

## 8. Primeiras execuções (decisão do usuário)

### 8.1 Invariante do backup (só leitura)

```powershell
powershell -NoProfile -File scripts/dev/verify-backup.ps1
```

O que esperar **hoje**: o script não faz fetch nem push, só compara as refs dos dois remotes. Enquanto o PR de sincronização (seção 9) não entrar, o `origin/dev` e o `backup/dev` podem diferir; as linhas por ref mostram `OK`, `OK-ANCESTRAL`, `ATRASADO`, `DIVERGE`, `FALTA` ou `TAG-DIFERE` (significados no cabeçalho do script). Exit 0 = tudo OK ou OK-ANCESTRAL; exit 1 = alguma ref fora da invariante; exit 2 = um remote não respondeu. Resultado exato hoje: **não verificado** (não rodado ao escrever este documento). Depois do PR de sincronização e de um `git push backup --all`, o esperado é exit 0.

### 8.2 Pacote de recuperação

Primeiro só listar o que entraria (nada é gravado):

```powershell
powershell -NoProfile -File scripts/dev/recovery-pack.ps1 -WhatIf
```

Leia a lista. Só então, por decisão do usuário, a execução real (grava um commit na branch órfã `recovery`, sem push e sem apagar nada; se a varredura de segredos achar algo, não grava e avisa):

```powershell
powershell -NoProfile -File scripts/dev/recovery-pack.ps1
```

E, depois dela, o push (só ao `backup`; o `origin` recusa `recovery*`):

```powershell
git push backup recovery
```

## 9. PR de sincronização do `dev` (T14)

Objetivo: levar ao `origin/dev` os commits que só existem no `dev` local, por PR, sem push direto em `dev`. Cada passo depende do "pode aplicar" do usuário; o merge depende também do "pode fechar".

```powershell
# 1. branch de sincronização a partir do dev local (troque AAAA-MM-DD pela data)
git switch -c sync/dev-AAAA-MM-DD dev

# 1.1 varredura de segredos ANTES do push (só leitura); exit 1 ou 2: PARE e avise o usuário
powershell -NoProfile -File scripts/dev/scan-secrets.ps1 -Range origin/dev..sync/dev-AAAA-MM-DD
# Avisos (caminhos pessoais, e-mails) não bloqueiam: repasse a lista ao usuário e só siga com o OK dele.

# 2. publicar só essa branch (a lista do hook aceita sync/*)
git push origin sync/dev-AAAA-MM-DD

# 3. abrir o PR contra dev
gh pr create --base dev --head sync/dev-AAAA-MM-DD --title "sync: publish local dev" --body "Synchronization PR (T14): brings the local dev history to origin/dev."

# 4. acompanhar o CI: os 5 checks devem ficar verdes
gh pr checks sync/dev-AAAA-MM-DD --watch
```

5. Mostrar ao usuário o resumo e esperar o "pode fechar" na conversa. Sem ele, não segue.

```powershell
# 6. merge: conferir os checks de novo e fazer o merge commit
gh pr checks sync/dev-AAAA-MM-DD
gh pr merge sync/dev-AAAA-MM-DD --merge

# 7. avançar o dev local só por fast-forward
git fetch origin dev:dev

# 8. backup e conferência da invariante
git push backup --all
git push backup --tags
powershell -NoProfile -File scripts/dev/verify-backup.ps1
```

Regras do passo a passo:

- **Nunca** `--admin` (contorna a proteção) e **nunca** `--delete-branch`/`-d` (apaga a branch remota sem o usuário decidir).
- `git fetch origin dev:dev` só avança por fast-forward e não troca de branch. Se recusar (o `dev` local divergiu do `origin/dev`, ou o `dev` está aberto em alguma worktree), pare e avise o usuário; nunca `git pull` sem `--ff-only`, nunca `reset`.
- Se a varredura do passo 1.1 der exit 1 ou 2, não empurre.
- Se o `strict` estiver ativo e o `dev` andar durante o PR, atualize a branch e espere o CI de novo antes do merge.
- Com o `dev` à frente do `origin/dev` em muitos commits, o PR será grande (não há limite de tamanho de PR); é PR de sincronização de histórico já revisado. O `dev` também não pode ficar atrás de `main`: se `main` tiver algo que o `dev` não tem, faça antes o PR `main` → `dev` (com o OK do usuário).
- Remover a branch `sync/dev-AAAA-MM-DD` depois (local e remota) é decisão do usuário, com a tag `archive/<branch>` antes, como em `agents/versao.md`.
