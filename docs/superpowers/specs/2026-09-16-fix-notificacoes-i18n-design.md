# Spec: Toast nativo + fugas de i18n

> Spec de design. Não é o plano de implementação.

**Data:** 2026-09-16
**Branch:** `feat/fix-notificacoes-i18n` a partir de `dev` (`0f6c922`)
**Repo:** `claudia-rh/`

## 1. Problema

`notificacoes.rs` faz poll ao SQLite a cada 30s e emite eventos Tauri (`nova-pendencia`, `nova-proposta`). O plugin `tauri_plugin_notification` está inicializado em `lib.rs`, mas **nenhum toast nativo Windows é enviado**. A aba Pendências atualiza; o sistema operativo não avisa o utilizador.

`notif.json` (`ativo`, `intervalo_minutos`) existe e a UI de Configurações grava-o. O watcher **não lê** essa config: não respeita `ativo=false` e **nunca repete** um aviso para pendências ainda por resolver.

Clique na notificação para ir às Pendências foi descrito no checklist antigo e **não existe no código**.

No frontend, depois do split `perfil/` + `dashboard/`, ainda há strings hardcoded em pt-PT/pt-BR que ignoram `useT()`:

- `src/components/dashboard/index.tsx`: `title="Editar limite"` (3×), `hoje ·`
- `src/components/perfil/index.tsx`: `A carregar perfil…`
- `src/components/perfil/CoverLettersView.tsx`: `A carregar…`

As chaves `t.common.loading` / `t.profile.loading` já existem em `en.ts` / `pt.ts`.

## 2. Objetivo

1. Com `notif.json.ativo = true`, uma pendência nova dispara toast nativo Windows (além do evento Tauri já existente).
2. Enquanto a pendência não estiver resolvida, o toast **repete** a cada `intervalo_minutos` (default 10).
3. Com `ativo = false`, zero toasts nativos. Eventos Tauri para a UI **continuam** (a aba Pendências não pode ficar cega).
4. Clique no toast: mostra a janela da app e muda para a aba Pendências.
5. As 5 strings listadas passam por i18n (`en` + `pt-BR`).
6. A lógica de “devo notificar agora?” é função pura, coberta por testes Rust, sem precisar do OS.

## 3. Fora de escopo

- Partir mais o frontend (já está em `perfil/` e `dashboard/`).
- Extrair markdown (já está em `src/lib/markdown.tsx`).
- Fundir `schema.sql` com os `ALTER`.
- Pré-requisitos Chrome / plano da conta.
- Pesquisa pontual do Feedback.
- Mudar `identifier` Tauri.
- Commitar `npm-package/`.

## 4. Regras permanentes (herdadas)

- Comentários e commits: inglês. Identificadores novos: inglês.
- Copy da UI: pt-BR.
- Não alterar `src-tauri/tauri.conf.json` `identifier`.
- Branch a partir de `dev`; merge de volta em `dev`; nunca `main`.

## 5. Desenho

### 5.1 Agendamento (puro, testável)

Novo módulo pequeno, p.ex. `src-tauri/src/notificacoes.rs` (funções `pub(crate)` no mesmo ficheiro, ou `notificacoes/schedule.rs` se o ficheiro crescer).

```rust
pub struct NotifyDecision {
    pub send_native: bool,
    pub emit_event: bool,
}

/// still_open: a pendência/proposta continua não resolvida
/// first_seen: primeira vez que este id aparece neste processo
/// last_native: último toast nativo deste id (None = nunca)
/// now, interval, native_enabled: relógio, intervalo de notif.json, flag ativo
pub fn decide_notify(
    first_seen: bool,
    still_open: bool,
    last_native: Option<std::time::Instant>,
    now: std::time::Instant,
    interval: std::time::Duration,
    native_enabled: bool,
) -> NotifyDecision
```

Regras:

| Situação | `emit_event` | `send_native` |
|---|---|---|
| `still_open=false` | false | false |
| `first_seen=true` | true | `native_enabled` |
| `first_seen=false`, `last_native=None` | false | `native_enabled` |
| `first_seen=false`, `now - last_native >= interval` | false | `native_enabled` |
| `first_seen=false`, `now - last_native < interval` | false | false |

Evento Tauri só na **primeira** vista do id neste processo (comportamento atual da UI). Toast nativo na primeira vista e depois a cada intervalo enquanto aberta.

Testes unitários (`#[cfg(test)]` no mesmo módulo), pelo menos:

- primeira vista + ativo → emit + native
- primeira vista + inativo → emit, sem native
- repetir depois do intervalo → só native
- dentro do intervalo → nada
- resolvida (sai do set) → nada; se voltar a abrir, trata-se como first_seen

### 5.2 Envio nativo

Usar o plugin já ligado:

```rust
use tauri_plugin_notification::NotificationExt;

app.notification()
    .builder()
    .title(title)
    .body(body)
    .show()
```

Títulos/corpos (pt-BR, constantes no Rust neste ciclo — o backend não tem i18n; a UI sim):

- Pendência: título `"Claudia RH — Pendência"`, corpo com categoria/descrição se a query as devolver; senão `"Tens uma pendência por resolver."`
- Proposta de perfil: título `"Claudia RH — Perfil"`, corpo `"Há uma pergunta nova para o teu perfil."`

Falhas de `.show()` não rebentam o loop: log `eprintln!` e segue.

`notificacoes::start` passa a receber `Arc<Mutex<NotifConfig>>` (já existe `NotifState` em `lib.rs`). Lê `ativo` e `intervalo_minutos` a cada iteração do loop (para o toggle nas Configurações fazer efeito sem restart).

Alargar as queries para trazer `categoria`/`descricao` da pendência quando for barato; se não, corpo genérico. Não inventar colunas.

### 5.3 Clique → aba Pendências

No setup da app, quando o plugin permitir listener de clique:

1. `show` + `set_focus` na janela `main`
2. `app.emit("navigate-to-pendencias", ())`

No frontend (`App.tsx`): `listen("navigate-to-pendencias", () => setView("pendencias"))`.

Se a API do plugin v2 não expuser clique de forma fiável no Windows, documentar no relatório da task e ficar só com (1) show+focus. Não bloquear o resto do ciclo por isso.

### 5.4 i18n

Adicionar chaves em `en.ts` e `pt.ts` (espelhadas):

- `dashboard.editLimit` → en `"Edit limit"` / pt `"Editar limite"`
- `dashboard.todayTotal` → en `"today"` / pt `"hoje"` (o `·` e o número ficam no JSX)

Substituir os hardcodes pelos `t.*` já existentes ou por estas chaves. `perfil/index.tsx` e `CoverLettersView.tsx` usam `t.profile.loading` / `t.common.loading` (já definidas).

Não traduzir o resto do Dashboard neste ciclo.

## 6. Ficheiros

| Ficheiro | Papel |
|---|---|
| `src-tauri/src/notificacoes.rs` | `decide_notify` + loop + `.show()` |
| `src-tauri/src/lib.rs` | passar `notif_arc` a `start`; listener de clique se existir |
| `src/App.tsx` | `listen("navigate-to-pendencias")` |
| `src/i18n/en.ts`, `src/i18n/pt.ts` | chaves novas |
| `src/components/dashboard/index.tsx` | `t.dashboard.editLimit` / `todayTotal` |
| `src/components/perfil/index.tsx` | `t.profile.loading` |
| `src/components/perfil/CoverLettersView.tsx` | `t.common.loading` |

## 7. Verificação

```text
# testes da decisão
cd src-tauri
cargo test decide_notify -- --nocapture

# i18n: zero hardcodes listados
rg -n "Editar limite|hoje ·|A carregar" src/components

# plugin ainda inicializado
rg -n "tauri_plugin_notification::init" src-tauri/src/lib.rs

# identifier intocado
git diff origin/dev -- src-tauri/tauri.conf.json
```

Critério humano: com a app a correr e `notif.json.ativo=true`, inserir uma linha em `pendencias` (resolvida=0) → toast Windows em ≤30s. Desligar o toggle em Configurações → novos toasts param; a aba Pendências continua a atualizar.

## 8. Como o Superpowers continua

Depois desta spec aprovada: `writing-plans` → implementação na branch `feat/fix-notificacoes-i18n` → review → merge em `dev`.
