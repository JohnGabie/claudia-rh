# Anexar currículo — Plano de implementação (Parte 1: upload; Parte 2: o resto)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** **`browser_upload`**: o agente consegue anexar o currículo (e a cover letter) certo num formulário de candidatura, só de documentos da biblioteca do usuário, sem nunca derrubar o Chrome. Depois: PDF dos currículos gerados, a aba Currículos completa e o MCP de documentos e variantes.

**Architecture:** o foco é a tool `browser_upload`. Um módulo Rust novo `documentos` dá a ela o que anexar (tabelas `documentos` e `preferidos`, importação segura, regra de escolha, resolução segura do arquivo); na Parte 1 o arquivo anexável é o currículo que o usuário importa. O PDF dos currículos gerados (pelo `PrintToPdf` do WebView2 do app) é da Parte 2. A tool `browser_upload` vive em `mcp/tools/browser.rs`, recebe uma `ref` e no máximo um `id` de documento (nunca um caminho), e anexa por CDP (`DOM.setFileInputFiles`, com `Page.setInterceptFileChooserDialog` para o botão que abre o seletor).

**Tech Stack:** Rust (Tauri 2.11, rusqlite, tokio, sha2, CDP próprio em `browser/cdp.rs`), `webview2-com` 0.38 + `windows`/`windows-core` 0.61 (já no grafo via `wry`), React/TS no front.

**Spec:** `docs/superpowers/specs/2026-10-04-anexar-curriculo-design.md` (aprovada pelo usuário em 2026-10-04).

## Como este plano é executado (pedido do usuário)

Duas partes, cada uma com **execução e teste**:

- **Parte 1 — `browser_upload`** (Tarefas 1 a 8): o mínimo para o agente anexar um currículo de verdade: a biblioteca, a importação do currículo do usuário, a regra de escolha e a tool. Sem geração de PDF: o anexo da Parte 1 é o PDF/DOCX importado. Termina num **portão**: rodadas de teste completas + aprovação do usuário.
- **Parte 2 — O resto** (Tarefas 9 a 16): PDF dos currículos gerados (WebView2), aba Currículos com grade e página A4, tags, preferido por variante, MCP de documentos/variantes/cover letter, migração. **Só começa depois do portão da Parte 1.** A Tarefa 9 já está detalhada; os passos de código das Tarefas 10 a 16 são escritos numa revisão deste plano depois do portão, com o que a Parte 1 ensinar; aqui ficam as tarefas, arquivos, interfaces e critérios de aceite.

## Global Constraints

- **Nenhum passo de instalação novo** (CLAUDE.md): nada de Python, Node, binário externo; só crates Rust.
- **O agente nunca passa nem recebe caminho de arquivo**: só `ref` e `id` de documento.
- **O Chrome nunca falha por causa de um upload** (regra dura do usuário): toda falha é erro da tool; a aba continua usável.
- **Só PDF e DOCX, até 10 MB**, tipo real conferido (`%PDF-` ou ZIP `PK\x03\x04`).
- Arquivos anexáveis só dentro de `<dados>/documentos/` (`<dados>` = `app_data_dir`, onde fica `claudia_rh.db`).
- Upload só com **exatamente uma** vaga em `candidatando` (mesma regra da guarda; o acúmulo de vagas é o bug CLA-35, fora deste plano).
- Currículo nunca vai para log ou diagnóstico: só nome, formato, tamanho e `sha256`.
- `prompt_sistema_runtime.md` só é editado com **autorização explícita** do usuário (Tarefa 7 fica bloqueada até ela).
- Comentários e commits em inglês; UI e docs em pt-BR; i18n pt e en.
- Testes: `CARGO_TARGET_DIR` fora do `target/` do repo; exit code real; **mutação** para cada regra nova (colar a linha `FAILED`).
- Nunca testar em site real sem pedido; nunca enviar candidatura.

## Review Focus

1. **Arquivo do documento apagado ou editado fora do app** depois de registrado → o upload recusa com mensagem clara ("o arquivo mudou/sumiu, gere ou importe de novo"), nunca anexa um arquivo diferente. (Tarefa 2, testes `missing_file_is_refused`, `edited_file_is_refused`.)
2. **Usuário importa o mesmo PDF duas vezes ou um arquivo com nome estranho** (`..\x.pdf`, acentos, espaços, 200 caracteres) → vira um nome seguro dentro da pasta, sem sobrescrever outro. (Tarefa 3, `import_sanitizes_names_and_never_overwrites`.)
3. **Site que aceita só PDF e o preferido é DOCX** → a página recusa ou o input ignora; a tool tem de dizer que a página não mostrou o arquivo, não "anexou". (Tarefa 5, `upload_reports_when_the_page_did_not_take_the_file`.)
4. **Botão de anexo que não abre seletor** (é outro botão, ou a página trava) → timeout curto, interceptação desligada, aba usável. (Tarefa 5, `a_chooser_that_never_opens_times_out_and_the_tab_still_works`.)
5. **Nenhum documento disponível** (biblioteca vazia, ou sem preferido e sem variante) → recusa explicando o que falta ("marque um currículo como preferido"), sem anexar nada. (Tarefa 4, `nothing_to_pick_returns_none`; Tarefa 5, `upload_with_an_empty_library_is_refused_with_a_hint`.)

---

# PARTE 1 — `browser_upload`

## Estrutura de arquivos da Parte 1

| Arquivo | Responsabilidade |
|---|---|
| `src-tauri/src/documentos/mod.rs` (novo) | modelo `Documento`, pasta, CRUD no banco |
| `src-tauri/src/documentos/seguranca.rs` (novo) | `resolve_attachable`: caminho dentro da pasta, existe, `sha256` confere |
| `src-tauri/src/documentos/importar.rs` (novo) | importar arquivo do usuário com validação |
| `src-tauri/src/documentos/escolha.rs` (novo) | regra de escolha (preferido da variante → geral → gerado da variante) |
| `src-tauri/src/documentos/fonte.rs` (novo) | `DbDocumentSource`: o que `browser_upload` consulta |
| `src-tauri/src/db/schema.sql` | tabelas `documentos` e `preferidos` |
| `src-tauri/src/commands/documentos.rs` (novo) | comandos Tauri da Parte 1 |
| `src-tauri/src/mcp/tools/browser.rs` | `browser_upload` + trait `DocumentSource` |
| `src-tauri/src/mcp/server.rs` | registra `browser_upload` |
| `src/components/perfil/CurriculosView.tsx` | bloco mínimo "Para anexar": importar e marcar preferido |
| `src-tauri/tests/fixtures/upload-form.html` (novo) | página de teste com os três tipos de campo |
| `src-tauri/tests/browser_live.rs` | testes live do upload |

---

### Tarefa 1: Biblioteca no banco (`documentos`, `preferidos`)

**Files:**
- Modify: `src-tauri/src/db/schema.sql`
- Create: `src-tauri/src/documentos/mod.rs`
- Modify: `src-tauri/src/lib.rs` (`mod documentos;`)

**Interfaces:**
- Produces:
  - `pub fn docs_dir(data_dir: &Path) -> PathBuf` → `data_dir.join("documentos")`
  - `pub struct Documento { pub id: i64, pub tipo: String, pub nome: String, pub origem: String, pub arquivo: String, pub arquivo_html: Option<String>, pub formato: String, pub idioma: Option<String>, pub tags: String, pub sha256: String, pub tamanho: i64, pub vaga_id: Option<i64>, pub variante_id: Option<String>, pub criado_em: String }` (`serde::Serialize`)
  - `pub struct NovoDocumento<'a> { pub tipo: &'a str, pub nome: &'a str, pub origem: &'a str, pub arquivo: &'a str, pub arquivo_html: Option<&'a str>, pub formato: &'a str, pub idioma: Option<&'a str>, pub sha256: &'a str, pub tamanho: i64, pub variante_id: Option<&'a str> }`
  - `pub fn inserir(conn: &Connection, d: &NovoDocumento) -> rusqlite::Result<i64>`
  - `pub fn obter(conn: &Connection, id: i64) -> rusqlite::Result<Option<Documento>>`
  - `pub fn listar(conn: &Connection, tipo: &str) -> rusqlite::Result<Vec<Documento>>` (mais novo primeiro)
  - `pub fn definir_preferido(conn: &Connection, id: i64, variante_id: Option<&str>) -> Result<(), String>` (o tipo vem do próprio documento)
  - `pub fn preferido(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Option<i64>`

- [ ] **Step 1: Schema.** Acrescentar ao fim de `schema.sql`:

```sql
-- Documents the agent may attach (CVs, cover letters). `arquivo` is a bare file name inside
-- <app data>/documentos/: never a path.
CREATE TABLE IF NOT EXISTS documentos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tipo TEXT NOT NULL CHECK (tipo IN ('curriculo', 'carta')),
    nome TEXT NOT NULL,
    origem TEXT NOT NULL CHECK (origem IN ('gerado', 'importado')),
    arquivo TEXT NOT NULL,
    arquivo_html TEXT,
    formato TEXT NOT NULL CHECK (formato IN ('pdf', 'docx')),
    idioma TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    sha256 TEXT NOT NULL,
    tamanho INTEGER NOT NULL,
    vaga_id INTEGER REFERENCES vagas(id),
    variante_id TEXT,
    criado_em TEXT NOT NULL DEFAULT (datetime('now'))
);

-- One preferred document per (tipo, variante). variante_id '' = the general preference.
CREATE TABLE IF NOT EXISTS preferidos (
    tipo TEXT NOT NULL,
    variante_id TEXT NOT NULL DEFAULT '',
    documento_id INTEGER NOT NULL REFERENCES documentos(id) ON DELETE CASCADE,
    PRIMARY KEY (tipo, variante_id)
);
```

- [ ] **Step 2: Testes que falham** (em `documentos/mod.rs`, `#[cfg(test)] mod tests`, banco em memória com `crate::db::apply`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    pub(crate) fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        c
    }

    pub(crate) fn novo<'a>(tipo: &'a str, arquivo: &'a str, variante: Option<&'a str>) -> NovoDocumento<'a> {
        NovoDocumento { tipo, nome: "CV", origem: "gerado", arquivo, arquivo_html: None, formato: "pdf", idioma: Some("pt"), sha256: "00", tamanho: 1, variante_id: variante }
    }

    #[test]
    fn insert_get_and_list_newest_first() {
        let c = mem();
        let a = inserir(&c, &novo("curriculo", "a.pdf", None)).unwrap();
        let b = inserir(&c, &novo("curriculo", "b.pdf", None)).unwrap();
        inserir(&c, &novo("carta", "c.pdf", None)).unwrap();
        assert_eq!(obter(&c, a).unwrap().unwrap().arquivo, "a.pdf");
        let l = listar(&c, "curriculo").unwrap();
        assert_eq!(l.iter().map(|d| d.id).collect::<Vec<_>>(), vec![b, a], "only CVs, newest first");
        assert!(obter(&c, 999).unwrap().is_none());
    }

    #[test]
    fn one_preferred_per_type_and_variant_the_last_mark_wins() {
        let c = mem();
        let a = inserir(&c, &novo("curriculo", "a.pdf", None)).unwrap();
        let b = inserir(&c, &novo("curriculo", "b.pdf", None)).unwrap();
        definir_preferido(&c, a, None).unwrap();
        definir_preferido(&c, b, None).unwrap();
        assert_eq!(preferido(&c, "curriculo", None), Some(b));
        definir_preferido(&c, a, Some("backend")).unwrap();
        assert_eq!(preferido(&c, "curriculo", Some("backend")), Some(a));
        assert_eq!(preferido(&c, "curriculo", None), Some(b), "the general mark is separate from the variant's");
        assert_eq!(preferido(&c, "carta", None), None);
    }

    #[test]
    fn marking_an_unknown_document_is_an_error() {
        let c = mem();
        assert!(definir_preferido(&c, 42, None).is_err());
    }
}
```

- [ ] **Step 3: Rodar e ver falhar.** `cargo test --lib documentos` → FAIL (não compila: funções inexistentes).

- [ ] **Step 4: Implementar** `documentos/mod.rs`:

```rust
//! The library of documents the agent may attach. Files live in <app data>/documentos/; the database
//! holds only bare file names, so a row can never point outside that folder.
pub mod escolha;
pub mod fonte;
pub mod importar;
pub mod seguranca;

use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

pub fn docs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("documentos")
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Documento {
    pub id: i64,
    pub tipo: String,
    pub nome: String,
    pub origem: String,
    pub arquivo: String,
    pub arquivo_html: Option<String>,
    pub formato: String,
    pub idioma: Option<String>,
    pub tags: String,
    pub sha256: String,
    pub tamanho: i64,
    pub vaga_id: Option<i64>,
    pub variante_id: Option<String>,
    pub criado_em: String,
}

pub struct NovoDocumento<'a> {
    pub tipo: &'a str,
    pub nome: &'a str,
    pub origem: &'a str,
    pub arquivo: &'a str,
    pub arquivo_html: Option<&'a str>,
    pub formato: &'a str,
    pub idioma: Option<&'a str>,
    pub sha256: &'a str,
    pub tamanho: i64,
    pub variante_id: Option<&'a str>,
}

const COLS: &str = "id, tipo, nome, origem, arquivo, arquivo_html, formato, idioma, tags, sha256, tamanho, vaga_id, variante_id, criado_em";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Documento> {
    Ok(Documento {
        id: r.get(0)?,
        tipo: r.get(1)?,
        nome: r.get(2)?,
        origem: r.get(3)?,
        arquivo: r.get(4)?,
        arquivo_html: r.get(5)?,
        formato: r.get(6)?,
        idioma: r.get(7)?,
        tags: r.get(8)?,
        sha256: r.get(9)?,
        tamanho: r.get(10)?,
        vaga_id: r.get(11)?,
        variante_id: r.get(12)?,
        criado_em: r.get(13)?,
    })
}

pub fn inserir(conn: &Connection, d: &NovoDocumento) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO documentos (tipo, nome, origem, arquivo, arquivo_html, formato, idioma, sha256, tamanho, variante_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![d.tipo, d.nome, d.origem, d.arquivo, d.arquivo_html, d.formato, d.idioma, d.sha256, d.tamanho, d.variante_id],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn obter(conn: &Connection, id: i64) -> rusqlite::Result<Option<Documento>> {
    conn.query_row(&format!("SELECT {COLS} FROM documentos WHERE id = ?1"), [id], row).optional()
}

pub fn listar(conn: &Connection, tipo: &str) -> rusqlite::Result<Vec<Documento>> {
    let mut st = conn.prepare(&format!("SELECT {COLS} FROM documentos WHERE tipo = ?1 ORDER BY id DESC"))?;
    let rows = st.query_map([tipo], row)?;
    rows.collect()
}

pub fn definir_preferido(conn: &Connection, id: i64, variante_id: Option<&str>) -> Result<(), String> {
    let doc = obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("documento {id} não existe"))?;
    conn.execute(
        "INSERT INTO preferidos (tipo, variante_id, documento_id) VALUES (?1, ?2, ?3)
         ON CONFLICT (tipo, variante_id) DO UPDATE SET documento_id = excluded.documento_id",
        params![doc.tipo, variante_id.unwrap_or(""), id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn preferido(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Option<i64> {
    conn.query_row(
        "SELECT documento_id FROM preferidos WHERE tipo = ?1 AND variante_id = ?2",
        params![tipo, variante_id.unwrap_or("")],
        |r| r.get(0),
    )
    .ok()
}
```

Criar `escolha.rs`, `fonte.rs`, `importar.rs`, `seguranca.rs` vazios (só um comentário de uma linha) para compilar; as próximas tarefas os preenchem.

- [ ] **Step 5: Rodar e ver passar.** `cargo test --lib documentos` → PASS.

- [ ] **Step 6: Mutação.** Trocar `ON CONFLICT ... DO UPDATE` por `DO NOTHING` → `one_preferred_per_type_and_variant_the_last_mark_wins` tem de FALHAR (colar a linha). Restaurar.

- [ ] **Step 7: Commit.** `git commit -m "feat(documentos): library tables and store (one preferred per type and variant)"`

---

### Tarefa 2: Resolver o arquivo com segurança

**Files:**
- Modify: `src-tauri/src/documentos/seguranca.rs`

**Interfaces:**
- Consumes: `documentos::{obter, docs_dir, Documento}`
- Produces:
  - `pub struct Anexavel { pub id: i64, pub nome: String, pub formato: String, pub tamanho: u64, pub sha256: String, pub path: PathBuf }`
  - `pub fn sha256_file(path: &Path) -> std::io::Result<String>` (hex minúsculo)
  - `pub fn resolve_attachable(conn: &Connection, data_dir: &Path, id: i64) -> Result<Anexavel, String>`

- [ ] **Step 1: Testes que falham.**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::documentos::{inserir, NovoDocumento};

    fn setup(name: &str) -> (rusqlite::Connection, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("claudia-docs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(crate::documentos::docs_dir(&dir)).unwrap();
        let c = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        (c, dir)
    }

    fn add(c: &rusqlite::Connection, dir: &Path, arquivo: &str, bytes: &[u8]) -> i64 {
        let p = crate::documentos::docs_dir(dir).join(arquivo);
        if !arquivo.contains("..") && !arquivo.contains('/') && !arquivo.contains('\\') {
            std::fs::write(&p, bytes).unwrap();
        }
        let sha = if p.exists() { sha256_file(&p).unwrap() } else { "00".into() };
        inserir(c, &NovoDocumento { tipo: "curriculo", nome: "CV", origem: "importado", arquivo, arquivo_html: None, formato: "pdf", idioma: None, sha256: &sha, tamanho: bytes.len() as i64, variante_id: None }).unwrap()
    }

    #[test]
    fn a_registered_file_resolves_inside_the_folder() {
        let (c, dir) = setup("ok");
        let id = add(&c, &dir, "cv.pdf", b"%PDF-1.7 hello");
        let a = resolve_attachable(&c, &dir, id).unwrap();
        assert!(a.path.ends_with("cv.pdf"));
        assert_eq!(a.tamanho, 14);
    }

    #[test]
    fn a_name_that_escapes_the_folder_is_refused() {
        let (c, dir) = setup("escape");
        std::fs::write(dir.join("segredo.pdf"), b"%PDF-1.7 x").unwrap();
        for bad in ["..\\segredo.pdf", "../segredo.pdf", "C:\\Windows\\win.ini", "sub/cv.pdf"] {
            let id = add(&c, &dir, bad, b"");
            let e = resolve_attachable(&c, &dir, id).unwrap_err();
            assert!(e.contains("fora da pasta"), "{bad}: {e}");
        }
    }

    #[test]
    fn missing_file_is_refused() {
        let (c, dir) = setup("missing");
        let id = add(&c, &dir, "cv.pdf", b"%PDF-1.7 x");
        std::fs::remove_file(crate::documentos::docs_dir(&dir).join("cv.pdf")).unwrap();
        assert!(resolve_attachable(&c, &dir, id).unwrap_err().contains("sumiu"));
    }

    #[test]
    fn edited_file_is_refused() {
        let (c, dir) = setup("edited");
        let id = add(&c, &dir, "cv.pdf", b"%PDF-1.7 original");
        std::fs::write(crate::documentos::docs_dir(&dir).join("cv.pdf"), b"%PDF-1.7 changed").unwrap();
        assert!(resolve_attachable(&c, &dir, id).unwrap_err().contains("mudou"));
    }

    #[test]
    fn unknown_id_is_refused() {
        let (c, dir) = setup("unknown");
        assert!(resolve_attachable(&c, &dir, 77).unwrap_err().contains("não existe"));
    }
}
```

- [ ] **Step 2: Rodar e ver falhar.** `cargo test --lib documentos::seguranca` → FAIL.

- [ ] **Step 3: Implementar.**

```rust
//! The only way from a document id to a file on disk. The agent never gives a path; this checks that
//! the stored name stays inside <app data>/documentos/, that the file is there, and that it is the
//! same file the user registered (sha256).
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct Anexavel {
    pub id: i64,
    pub nome: String,
    pub formato: String,
    pub tamanho: u64,
    pub sha256: String,
    pub path: PathBuf,
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn is_bare_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', ':']) && name != "." && name != ".." && !name.starts_with("..")
}

pub fn resolve_attachable(conn: &Connection, data_dir: &Path, id: i64) -> Result<Anexavel, String> {
    let doc = super::obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
    if !is_bare_name(&doc.arquivo) {
        return Err(format!("o documento {id} aponta para fora da pasta de documentos: recusado"));
    }
    let base = std::fs::canonicalize(super::docs_dir(data_dir)).map_err(|_| "a pasta de documentos não existe".to_string())?;
    let path = base.join(&doc.arquivo);
    let real = std::fs::canonicalize(&path).map_err(|_| format!("o arquivo de \"{}\" sumiu: gere ou importe de novo", doc.nome))?;
    if !real.starts_with(&base) {
        return Err(format!("o documento {id} aponta para fora da pasta de documentos: recusado"));
    }
    let meta = std::fs::metadata(&real).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err(format!("o documento {id} aponta para fora da pasta de documentos: recusado"));
    }
    let sha = sha256_file(&real).map_err(|e| e.to_string())?;
    if sha != doc.sha256 {
        return Err(format!("o arquivo de \"{}\" mudou depois de registrado: gere ou importe de novo", doc.nome));
    }
    Ok(Anexavel { id, nome: doc.nome, formato: doc.formato, tamanho: meta.len(), sha256: sha, path: real })
}
```

- [ ] **Step 4: Rodar e ver passar.** `cargo test --lib documentos::seguranca` → PASS.

- [ ] **Step 5: Mutações** (uma por vez, colar o FAILED, restaurar):
  - remover o `if !is_bare_name(...)` **e** o `if !real.starts_with(&base)` → `a_name_that_escapes_the_folder_is_refused` falha;
  - remover a comparação de `sha` → `edited_file_is_refused` falha.

- [ ] **Step 6: Commit.** `git commit -m "feat(documentos): resolve a document id to a file only inside the folder and only if unchanged"`

---

### Tarefa 3: Importar o currículo do usuário

**Files:**
- Modify: `src-tauri/src/documentos/importar.rs`

**Interfaces:**
- Consumes: `docs_dir`, `inserir`, `obter`, `NovoDocumento`, `seguranca::sha256_file`
- Produces: `pub const MAX_BYTES: u64 = 10 * 1024 * 1024;` e `pub fn importar(conn: &Connection, data_dir: &Path, tipo: &str, origem: &Path, nome: Option<&str>) -> Result<Documento, String>`

- [ ] **Step 1: Testes que falham.**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn setup(name: &str) -> (rusqlite::Connection, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("claudia-imp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        (c, dir)
    }

    fn file(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn a_real_pdf_is_copied_registered_and_resolvable() {
        let (c, dir) = setup("pdf");
        let src = file(&dir, "Meu CV.pdf", b"%PDF-1.7 conteudo");
        let d = importar(&c, &dir, "curriculo", &src, None).unwrap();
        assert_eq!((d.origem.as_str(), d.formato.as_str(), d.nome.as_str()), ("importado", "pdf", "Meu CV"));
        assert!(src.exists(), "the user's original is never moved");
        crate::documentos::seguranca::resolve_attachable(&c, &dir, d.id).unwrap();
    }

    #[test]
    fn a_docx_is_accepted_by_its_zip_header() {
        let (c, dir) = setup("docx");
        let src = file(&dir, "cv.docx", b"PK\x03\x04 resto");
        assert_eq!(importar(&c, &dir, "curriculo", &src, Some("Word")).unwrap().formato, "docx");
    }

    #[test]
    fn a_fake_pdf_is_refused() {
        let (c, dir) = setup("fake");
        let src = file(&dir, "cv.pdf", b"MZ not a pdf");
        assert!(importar(&c, &dir, "curriculo", &src, None).unwrap_err().contains("não é um PDF"));
    }

    #[test]
    fn other_extensions_are_refused() {
        let (c, dir) = setup("ext");
        let src = file(&dir, "cv.exe", b"%PDF-1.7");
        assert!(importar(&c, &dir, "curriculo", &src, None).unwrap_err().contains("PDF ou DOCX"));
    }

    #[test]
    fn a_file_over_ten_megabytes_is_refused() {
        let (c, dir) = setup("big");
        let mut big = b"%PDF-1.7".to_vec();
        big.resize((MAX_BYTES + 1) as usize, b'x');
        let src = file(&dir, "cv.pdf", &big);
        assert!(importar(&c, &dir, "curriculo", &src, None).unwrap_err().contains("10 MB"));
    }

    #[test]
    fn import_sanitizes_names_and_never_overwrites() {
        let (c, dir) = setup("names");
        let src = file(&dir, "cv.pdf", b"%PDF-1.7 a");
        let a = importar(&c, &dir, "curriculo", &src, Some("..\\..\\x")).unwrap();
        let b = importar(&c, &dir, "curriculo", &src, Some("..\\..\\x")).unwrap();
        assert_ne!(a.arquivo, b.arquivo, "the same import twice gets two files");
        for d in [&a, &b] {
            assert!(!d.arquivo.contains(['\\', '/', ':']) && !d.arquivo.starts_with(".."), "{}", d.arquivo);
            crate::documentos::seguranca::resolve_attachable(&c, &dir, d.id).unwrap();
        }
    }

    #[test]
    fn a_wrong_type_is_refused() {
        let (c, dir) = setup("tipo");
        let src = file(&dir, "cv.pdf", b"%PDF-1.7");
        assert!(importar(&c, &dir, "foto", &src, None).is_err());
    }
}
```

- [ ] **Step 2: Rodar e ver falhar.**

- [ ] **Step 3: Implementar.**

```rust
//! Bringing the user's own CV into the library: copied (never moved or referenced), checked by
//! extension AND content, at most 10 MB, stored under a generated bare name.
use super::{docs_dir, inserir, obter, Documento, NovoDocumento};
use rusqlite::Connection;
use std::path::Path;

pub const MAX_BYTES: u64 = 10 * 1024 * 1024;

pub fn importar(conn: &Connection, data_dir: &Path, tipo: &str, origem: &Path, nome: Option<&str>) -> Result<Documento, String> {
    if tipo != "curriculo" && tipo != "carta" {
        return Err(format!("tipo inválido: {tipo}"));
    }
    let ext = origem.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
    let formato = match ext.as_str() {
        "pdf" => "pdf",
        "docx" => "docx",
        _ => return Err("só é possível importar PDF ou DOCX".into()),
    };
    let meta = std::fs::metadata(origem).map_err(|_| "não foi possível ler o arquivo escolhido".to_string())?;
    if meta.len() > MAX_BYTES {
        return Err("o arquivo passa de 10 MB".into());
    }
    let bytes = std::fs::read(origem).map_err(|_| "não foi possível ler o arquivo escolhido".to_string())?;
    let ok = match formato {
        "pdf" => bytes.starts_with(b"%PDF-"),
        _ => bytes.starts_with(b"PK\x03\x04"),
    };
    if !ok {
        return Err(if formato == "pdf" { "o arquivo não é um PDF de verdade".into() } else { "o arquivo não é um DOCX de verdade".into() });
    }
    let dir = docs_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let arquivo = format!("importado-{nanos}.{formato}");
    let dest = dir.join(&arquivo);
    if dest.exists() {
        return Err("conflito de nome ao importar; tente de novo".into());
    }
    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
    let sha = super::seguranca::sha256_file(&dest).map_err(|e| e.to_string())?;
    let stem = origem.file_stem().and_then(|s| s.to_str()).unwrap_or("Currículo");
    let nome = nome.unwrap_or(stem).trim();
    let nome = if nome.is_empty() { "Currículo" } else { nome };
    let id = inserir(conn, &NovoDocumento { tipo, nome, origem: "importado", arquivo: &arquivo, arquivo_html: None, formato, idioma: None, sha256: &sha, tamanho: bytes.len() as i64, variante_id: None })
        .map_err(|e| {
            let _ = std::fs::remove_file(&dest);
            e.to_string()
        })?;
    obter(conn, id).map_err(|e| e.to_string())?.ok_or_else(|| "falha ao registrar o documento".into())
}
```

O `nome` exibido pode ter qualquer texto; o **arquivo** sempre é `importado-<nanos>.<formato>`.

- [ ] **Step 4: Rodar e ver passar.**

- [ ] **Step 5: Mutações:** remover a checagem de cabeçalho → `a_fake_pdf_is_refused` falha; remover o limite → `a_file_over_ten_megabytes_is_refused` falha.

- [ ] **Step 6: Commit.** `git commit -m "feat(documentos): import the user's own PDF/DOCX safely (copy, real type, 10 MB, generated name)"`

---

### Tarefa 4: Regra de escolha

**Files:**
- Modify: `src-tauri/src/documentos/escolha.rs`

**Interfaces:**
- Consumes: `preferido`, tabela `documentos`
- Produces:
  - `pub struct Escolha { pub id: i64, pub regra: &'static str }` — `regra` ∈ `"preferido da variante"`, `"preferido geral"`, `"gerado mais recente da variante"`
  - `pub fn escolher(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Option<Escolha>`

- [ ] **Step 1: Testes que falham.**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::documentos::{definir_preferido, inserir, NovoDocumento};

    fn mem() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::apply(&c).unwrap();
        c
    }
    fn doc(c: &rusqlite::Connection, origem: &str, variante: Option<&str>) -> i64 {
        inserir(c, &NovoDocumento { tipo: "curriculo", nome: "CV", origem, arquivo: "x.pdf", arquivo_html: None, formato: "pdf", idioma: None, sha256: "0", tamanho: 1, variante_id: variante }).unwrap()
    }

    #[test]
    fn the_variant_preference_wins_over_everything() {
        let c = mem();
        let geral = doc(&c, "importado", None);
        let var = doc(&c, "importado", None);
        doc(&c, "gerado", Some("backend"));
        definir_preferido(&c, geral, None).unwrap();
        definir_preferido(&c, var, Some("backend")).unwrap();
        let e = escolher(&c, "curriculo", Some("backend")).unwrap();
        assert_eq!((e.id, e.regra), (var, "preferido da variante"));
    }

    #[test]
    fn without_a_variant_preference_the_general_one() {
        let c = mem();
        let geral = doc(&c, "importado", None);
        doc(&c, "gerado", Some("backend"));
        definir_preferido(&c, geral, None).unwrap();
        assert_eq!(escolher(&c, "curriculo", Some("backend")).unwrap().regra, "preferido geral");
        assert_eq!(escolher(&c, "curriculo", None).unwrap().id, geral);
    }

    #[test]
    fn without_preferences_the_newest_generated_for_that_variant() {
        let c = mem();
        doc(&c, "gerado", Some("backend"));
        let novo = doc(&c, "gerado", Some("backend"));
        doc(&c, "gerado", Some("devops"));
        doc(&c, "importado", None);
        let e = escolher(&c, "curriculo", Some("backend")).unwrap();
        assert_eq!((e.id, e.regra), (novo, "gerado mais recente da variante"));
    }

    #[test]
    fn nothing_to_pick_returns_none() {
        let c = mem();
        doc(&c, "importado", None);
        doc(&c, "gerado", Some("devops"));
        assert!(escolher(&c, "curriculo", Some("backend")).is_none(), "no preference, nothing generated for backend");
        assert!(escolher(&c, "curriculo", None).is_none(), "no variant and no general preference");
        assert!(escolher(&c, "carta", None).is_none());
    }
}
```

- [ ] **Step 2: Rodar e ver falhar.**

- [ ] **Step 3: Implementar.**

```rust
//! Which document to attach (spec section 4). The user's marks always win over what was generated.
use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq)]
pub struct Escolha {
    pub id: i64,
    pub regra: &'static str,
}

pub fn escolher(conn: &Connection, tipo: &str, variante_id: Option<&str>) -> Option<Escolha> {
    if let Some(v) = variante_id {
        if let Some(id) = super::preferido(conn, tipo, Some(v)) {
            return Some(Escolha { id, regra: "preferido da variante" });
        }
    }
    if let Some(id) = super::preferido(conn, tipo, None) {
        return Some(Escolha { id, regra: "preferido geral" });
    }
    let v = variante_id?;
    conn.query_row(
        "SELECT id FROM documentos WHERE tipo = ?1 AND origem = 'gerado' AND variante_id = ?2 ORDER BY id DESC LIMIT 1",
        params![tipo, v],
        |r| r.get(0),
    )
    .ok()
    .map(|id| Escolha { id, regra: "gerado mais recente da variante" })
}
```

- [ ] **Step 4: Rodar e ver passar.**

- [ ] **Step 5: Mutações:** trocar a ordem dos dois primeiros blocos → `the_variant_preference_wins_over_everything` falha; tirar `AND origem = 'gerado'` → `nothing_to_pick_returns_none` ou `without_preferences_...` falha (colar qual).

- [ ] **Step 6: Commit.** `git commit -m "feat(documentos): selection rule - variant preference, general preference, newest generated for the variant"`

---

### Tarefa 5: Tool `browser_upload` (com a regra "o Chrome nunca falha")

**Files:**
- Modify: `src-tauri/src/mcp/tools/browser.rs`
- Create: `src-tauri/tests/fixtures/upload-form.html`
- Modify: `src-tauri/tests/browser_live.rs`

**Interfaces:**
- Produces (em `mcp/tools/browser.rs`):
  - `pub struct Attachment { pub id: i64, pub nome: String, pub formato: String, pub tamanho: u64, pub sha256: String, pub path: std::path::PathBuf, pub regra: String }`
  - `pub trait DocumentSource: Send + Sync { fn for_upload(&self, vaga_id: i64, tipo: &str, documento: Option<i64>) -> Result<Attachment, String>; }`
  - `pub struct NoDocuments;` (implementa recusando: "a biblioteca de documentos não está disponível")
  - `BrowserTools::with_documents(self, docs: Arc<dyn DocumentSource>) -> Self`
  - `pub async fn upload(&self, reference: &str, tipo: Option<&str>, documento: Option<i64>) -> Result<String, String>`

- [ ] **Step 1: Fixture** `src-tauri/tests/fixtures/upload-form.html`:

```html
<!doctype html>
<html lang="pt-BR"><head><meta charset="utf-8"><title>Upload</title></head>
<body>
  <h1>Candidatura de teste</h1>
  <label for="cv">Currículo</label>
  <input id="cv" type="file" onchange="document.getElementById('nome-cv').textContent = this.files[0] ? this.files[0].name : ''">
  <span id="nome-cv"></span>

  <!-- LinkedIn pattern: the real input is hidden, a button opens the chooser -->
  <input id="escondido" type="file" style="display:none" onchange="document.getElementById('nome-esc').textContent = this.files[0] ? this.files[0].name : ''">
  <button id="anexar" type="button" onclick="document.getElementById('escondido').click()">Carregar currículo</button>
  <span id="nome-esc"></span>

  <!-- only PDF: the page itself rejects anything else -->
  <input id="so-pdf" type="file" accept=".pdf" aria-label="Só PDF" onchange="var f=this.files[0]; document.getElementById('nome-pdf').textContent = (f && f.name.toLowerCase().endsWith('.pdf')) ? f.name : 'rejeitado'">
  <span id="nome-pdf"></span>

  <button id="morto" type="button">Botão que não abre nada</button>
  <button id="enviar" type="button" onclick="window.__submitted = true">Enviar candidatura</button>
  <script>window.__submitted = false;</script>
</body></html>
```

- [ ] **Step 2: Testes live que falham** (em `tests/browser_live.rs`; use os helpers existentes `launch`, `tools`, `Ctx::candidatando`, `fixture_file`, `agent_client`, `eval`, `ref_of`):

```rust
struct FakeDocs {
    path: std::path::PathBuf,
    formato: &'static str,
}

impl claudia_rh_lib::mcp::tools::browser::DocumentSource for FakeDocs {
    fn for_upload(&self, _vaga: i64, _tipo: &str, _doc: Option<i64>) -> Result<claudia_rh_lib::mcp::tools::browser::Attachment, String> {
        Ok(claudia_rh_lib::mcp::tools::browser::Attachment {
            id: 1,
            nome: "CV Teste".into(),
            formato: self.formato.into(),
            tamanho: std::fs::metadata(&self.path).unwrap().len(),
            sha256: "abc".into(),
            path: self.path.clone(),
            regra: "preferido geral".into(),
        })
    }
}

fn fake_doc(ext: &'static str) -> Arc<FakeDocs> {
    let p = std::env::temp_dir().join(format!("claudia-upload-{}.{ext}", std::process::id()));
    std::fs::write(&p, if ext == "pdf" { &b"%PDF-1.7 teste"[..] } else { &b"PK\x03\x04 teste"[..] }).unwrap();
    Arc::new(FakeDocs { path: p, formato: ext })
}

async fn text_of(l: &Live, id: &str) -> String {
    let a = agent_client(l).await;
    eval(&a, &format!("document.getElementById('{id}').textContent")).await.as_str().unwrap_or("").to_string()
}

#[tokio::test]
#[ignore]
async fn upload_into_a_visible_file_input_and_the_page_sees_it() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let out = t.upload(&ref_of(&list, "button", "Currículo"), None, None).await.unwrap();
    assert!(out.contains("CV Teste") && out.contains("preferido geral"), "{out}");
    assert!(text_of(&l, "nome-cv").await.ends_with(".pdf"));
}

#[tokio::test]
#[ignore]
async fn upload_through_a_button_that_opens_the_chooser() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    t.upload(&ref_of(&list, "button", "Carregar currículo"), None, None).await.unwrap();
    assert!(text_of(&l, "nome-esc").await.ends_with(".pdf"));
}

#[tokio::test]
#[ignore]
async fn upload_refuses_a_submit_button_and_never_clicks_it() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.upload(&ref_of(&list, "button", "Enviar candidatura"), None, None).await.unwrap_err();
    assert!(e.starts_with("RECUSADO"), "{e}");
    let a = agent_client(&l).await;
    assert_eq!(eval(&a, "window.__submitted").await, false);
}

#[tokio::test]
#[ignore]
async fn upload_without_an_active_vaga_is_refused() {
    let l = launch().await;
    let none = Arc::new(Ctx { status: "candidatando", active: None, claimed: Mutex::new(HashSet::new()) });
    let t = tools(&l, none).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.upload(&ref_of(&list, "button", "Currículo"), None, None).await.unwrap_err();
    assert!(e.contains("candidatando"), "{e}");
}

#[tokio::test]
#[ignore]
async fn upload_reports_when_the_page_did_not_take_the_file() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("docx"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let out = t.upload(&ref_of(&list, "button", "Só PDF"), None, None).await.unwrap();
    assert!(out.contains("a página não mostrou o nome do arquivo"), "{out}");
}

#[tokio::test]
#[ignore]
async fn a_chooser_that_never_opens_times_out_and_the_tab_still_works() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)).with_documents(fake_doc("pdf"));
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.upload(&ref_of(&list, "button", "Botão que não abre nada"), None, None).await.unwrap_err();
    assert!(e.contains("seletor de arquivo"), "{e}");
    // The tab is still usable, and the chooser is no longer intercepted: a later real chooser works.
    let list = t.read(None).await.unwrap();
    t.upload(&ref_of(&list, "button", "Carregar currículo"), None, None).await.unwrap();
    assert!(text_of(&l, "nome-esc").await.ends_with(".pdf"));
}

#[tokio::test]
#[ignore]
async fn upload_with_an_empty_library_is_refused_with_a_hint() {
    let l = launch().await;
    let t = tools(&l, Ctx::candidatando(7)); // NoDocuments
    t.open(&fixture_file("upload-form.html")).await.unwrap();
    let list = t.read(None).await.unwrap();
    let e = t.upload(&ref_of(&list, "button", "Currículo"), None, None).await.unwrap_err();
    assert!(e.contains("documentos"), "{e}");
    t.read(None).await.unwrap(); // tab still fine
}
```

Se o papel AX do `<input type=file>` não for `button` no Chrome for Testing 154, ajustar `ref_of` para o papel real que `browser_read` mostrar (ler a saída de `browser_read` da fixture primeiro e anotar no teste). Se `Ctx`/`tools` não forem públicos o suficiente, ajustar apenas a visibilidade dentro do arquivo de teste.

- [ ] **Step 3: Rodar e ver falhar** (não compila: `with_documents`/`upload` não existem). Comando live do `CLAUDE.md` com `--test browser_live upload`.

- [ ] **Step 4: Implementar em `mcp/tools/browser.rs`.** Acrescentar perto de `ActionSink`:

```rust
/// What browser_upload attaches: always resolved by the app from the library, never a path from the agent.
#[derive(Debug, Clone)]
pub struct Attachment {
    pub id: i64,
    pub nome: String,
    pub formato: String,
    pub tamanho: u64,
    pub sha256: String,
    pub path: std::path::PathBuf,
    pub regra: String,
}

pub trait DocumentSource: Send + Sync {
    fn for_upload(&self, vaga_id: i64, tipo: &str, documento: Option<i64>) -> Result<Attachment, String>;
}

pub struct NoDocuments;
impl DocumentSource for NoDocuments {
    fn for_upload(&self, _: i64, _: &str, _: Option<i64>) -> Result<Attachment, String> {
        Err("a biblioteca de documentos não está disponível: gere ou importe um currículo no app".into())
    }
}

const UPLOAD_STEP: Duration = Duration::from_secs(5);
```

No struct `BrowserTools` acrescentar `docs: Arc<dyn DocumentSource>,` iniciado em `new` com `Arc::new(NoDocuments)`, e:

```rust
pub fn with_documents(mut self, docs: Arc<dyn DocumentSource>) -> Self {
    self.docs = docs;
    self
}
```

O método (junto das outras tools):

```rust
pub async fn upload(&self, reference: &str, tipo: Option<&str>, documento: Option<i64>) -> Result<String, String> {
    let started = Instant::now();
    self.agent_active.store(true, Ordering::SeqCst);
    let (c, _, _) = match self.tab().await {
        Ok(t) => t,
        Err(e) => return self.finish("browser_upload", started, Err(e), None).await,
    };
    let r = async {
        let tipo = tipo.unwrap_or("curriculo");
        if tipo != "curriculo" && tipo != "carta" {
            return Err(format!("tipo inválido: {tipo} (use curriculo ou carta)"));
        }
        let vaga = self.guard.active_vaga().ok_or_else(|| {
            "RECUSADO: anexar exige exatamente uma vaga em 'candidatando' (a vaga sendo candidatada).".to_string()
        })?;
        let att = self.docs.for_upload(vaga, tipo, documento)?;
        let n = self.resolve_ref(&c, reference).await?;
        let file = att.path.to_string_lossy().to_string();
        let (shot, x, y) = self.locate(&c, &n, true).await?;
        if Self::is_file_input(&c, n.backend_node_id).await {
            tokio::time::timeout(UPLOAD_STEP, c.call("DOM.setFileInputFiles", json!({ "files": [file], "backendNodeId": n.backend_node_id })))
                .await
                .map_err(|_| "o campo de arquivo não respondeu a tempo".to_string())??;
        } else {
            // A button that opens the chooser: it must not be a submit (the upload never submits).
            let mut probe = Probe::default();
            probe.add_name(&n.name);
            Self::probe_node(&c, n.backend_node_id, &mut probe).await;
            if let Some(hit) = Self::node_at_point(&c, x, y).await {
                if hit != n.backend_node_id {
                    Self::probe_node(&c, hit, &mut probe).await;
                }
            }
            let url = Self::page_url(&c).await;
            if self.run_guard(&probe, &url, None).is_err() {
                return Err("RECUSADO: esse elemento envia a candidatura; browser_upload só anexa arquivos.".into());
            }
            Self::upload_via_chooser(&c, x, y, &file).await?;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
        let file_name = att.path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        let shown = Self::eval(&c, &format!("document.body ? document.body.innerText.includes({}) : false", serde_json::to_string(&file_name).unwrap_or_default()))
            .await
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let kb = (att.tamanho + 1023) / 1024;
        let mut message = format!("Anexou \"{}\" ({}, {} KB) pela regra: {}.", att.nome, att.formato, kb, att.regra);
        if !shown {
            message.push_str(" Atenção: a página não mostrou o nome do arquivo; confira com browser_read ou browser_text antes de seguir (o site pode ter recusado o formato).");
        }
        Ok(Outcome {
            message,
            target: Some(n.name),
            role: Some(n.role),
            // Never the content: name, format, size and a short hash only.
            text: Some(format!("{} | {} | {} B | sha256 {}", att.nome, att.formato, att.tamanho, &att.sha256[..att.sha256.len().min(12)])),
            shot_before: shot,
            ..Default::default()
        })
    }
    .await;
    self.finish("browser_upload", started, r, Some(&c)).await
}

async fn is_file_input(c: &CdpClient, backend: i64) -> bool {
    let Ok(d) = c.call("DOM.describeNode", json!({ "backendNodeId": backend })).await else { return false };
    let node = &d["node"];
    if node["nodeName"].as_str() != Some("INPUT") {
        return false;
    }
    let attrs = node["attributes"].as_array().cloned().unwrap_or_default();
    attrs.chunks(2).any(|kv| kv.first().and_then(|k| k.as_str()) == Some("type") && kv.get(1).and_then(|v| v.as_str()).map(|v| v.eq_ignore_ascii_case("file")) == Some(true))
}

/// Clicks a button that opens the file chooser, intercepting the chooser so no native dialog ever
/// appears. The interception is ALWAYS turned off again, success or not: the tab must stay usable.
async fn upload_via_chooser(c: &CdpClient, x: f64, y: f64, file: &str) -> Result<(), String> {
    let mut events = c.subscribe();
    let _ = c.call("Page.enable", json!({})).await;
    c.call("Page.setInterceptFileChooserDialog", json!({ "enabled": true })).await?;
    let result = async {
        Self::mouse_click(c, x, y).await?;
        let backend = tokio::time::timeout(UPLOAD_STEP, async {
            loop {
                match events.recv().await {
                    Ok(ev) if ev["method"] == "Page.fileChooserOpened" => {
                        return ev["params"]["backendNodeId"].as_i64();
                    }
                    Ok(_) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return None,
                }
            }
        })
        .await
        .map_err(|_| "o elemento não abriu o seletor de arquivo (tempo esgotado)".to_string())?
        .ok_or_else(|| "o seletor de arquivo abriu sem um campo de arquivo associado".to_string())?;
        tokio::time::timeout(UPLOAD_STEP, c.call("DOM.setFileInputFiles", json!({ "files": [file], "backendNodeId": backend })))
            .await
            .map_err(|_| "o campo de arquivo não respondeu a tempo".to_string())??;
        Ok(())
    }
    .await;
    let _ = c.call("Page.setInterceptFileChooserDialog", json!({ "enabled": false })).await;
    result
}
```

Confirmar no código os nomes/assinaturas reais de `Probe`, `probe_node`, `node_at_point`, `run_guard`, `mouse_click`, `locate` e `CdpClient::subscribe` (são os usados em `click`, linhas ~954–986); se algum diferir, usar o nome real **sem mudar a lógica**. O formato dos eventos em `subscribe()` deve ser conferido em `browser/cdp.rs` (campo `method`/`params`); se for outro, ajustar só a leitura do evento.

- [ ] **Step 5: Rodar e ver passar** os 7 testes live de upload e a suíte live inteira (`--skip icon`).

- [ ] **Step 6: Mutações** (uma por vez, colar o FAILED, restaurar):
  - remover a chamada `Page.setInterceptFileChooserDialog` com `enabled: false` → `a_chooser_that_never_opens_times_out_and_the_tab_still_works` tem de falhar (a segunda tentativa trava ou não recebe o evento; se não falhar, o teste não protege a regra: reforçar até falhar);
  - remover o bloco do guard (`run_guard`) → `upload_refuses_a_submit_button_and_never_clicks_it` falha;
  - remover o `active_vaga().ok_or_else` → `upload_without_an_active_vaga_is_refused` falha.

- [ ] **Step 7: Commit.** `git commit -m "feat(mcp): browser_upload attaches a library document by ref; chooser interception always released; never submits"`

---

### Tarefa 6: Ligar a biblioteca ao app e ao MCP

**Files:**
- Modify: `src-tauri/src/documentos/fonte.rs`
- Create: `src-tauri/src/commands/documentos.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` (registrar comandos; remover `debug_pdf_prova`)
- Modify: `src-tauri/src/mcp/server.rs` (`browser_upload` + `with_documents`)
- Modify: `src/components/perfil/CurriculosView.tsx`, `src/i18n/pt.ts`, `src/i18n/en.ts`

**Interfaces:**
- Consumes: `escolha::escolher`, `seguranca::resolve_attachable`, `importar::importar`, `DocumentSource`
- Produces:
  - `pub struct DbDocumentSource { pub data_dir: PathBuf }` implementando `DocumentSource`
  - comandos Tauri: `listar_documentos(tipo: String) -> Vec<Documento>`, `importar_documento(tipo: String, caminho_origem: String, nome: Option<String>) -> Documento`, `definir_preferido(id: i64, variante_id: Option<String>)`

- [ ] **Step 1: Teste que falha para `DbDocumentSource`** (em `fonte.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::tools::browser::DocumentSource;

    fn setup() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("claudia-fonte-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        crate::db::apply(&c).unwrap();
        c.execute("INSERT INTO vagas (id, titulo, empresa, plataforma, url, descoberta_em, status, variante_id) VALUES (7, 't', 'e', 'p', 'u', datetime('now'), 'candidatando', 'backend')", []).unwrap();
        dir
    }

    #[test]
    fn picks_by_the_vagas_variant_and_resolves_safely() {
        let dir = setup();
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let src = dir.join("meu.pdf");
        std::fs::write(&src, b"%PDF-1.7 x").unwrap();
        let d = crate::documentos::importar::importar(&c, &dir, "curriculo", &src, Some("Meu")).unwrap();
        crate::documentos::definir_preferido(&c, d.id, Some("backend")).unwrap();
        let a = DbDocumentSource { data_dir: dir.clone() }.for_upload(7, "curriculo", None).unwrap();
        assert_eq!((a.id, a.regra.as_str()), (d.id, "preferido da variante"));
    }

    #[test]
    fn an_explicit_document_must_be_of_the_right_type() {
        let dir = setup();
        let c = rusqlite::Connection::open(dir.join("claudia_rh.db")).unwrap();
        let src = dir.join("carta.pdf");
        std::fs::write(&src, b"%PDF-1.7 x").unwrap();
        let carta = crate::documentos::importar::importar(&c, &dir, "carta", &src, None).unwrap();
        let e = DbDocumentSource { data_dir: dir }.for_upload(7, "curriculo", Some(carta.id)).unwrap_err();
        assert!(e.contains("não é um curriculo"), "{e}");
    }

    #[test]
    fn nothing_to_attach_explains_what_to_do() {
        let dir = setup();
        let e = DbDocumentSource { data_dir: dir }.for_upload(7, "curriculo", None).unwrap_err();
        assert!(e.contains("preferido"), "{e}");
    }
}
```

(Se o `INSERT INTO vagas` precisar de outras colunas `NOT NULL`, ver `schema.sql` e preencher.)

- [ ] **Step 2: Implementar `fonte.rs`.**

```rust
//! What browser_upload asks: "which file for this vaga?". Answered from the library only.
use crate::mcp::tools::browser::{Attachment, DocumentSource};
use std::path::PathBuf;

pub struct DbDocumentSource {
    pub data_dir: PathBuf,
}

impl DocumentSource for DbDocumentSource {
    fn for_upload(&self, vaga_id: i64, tipo: &str, documento: Option<i64>) -> Result<Attachment, String> {
        let conn = crate::mcp::tools::open_db(&self.data_dir)?;
        let (id, regra) = match documento {
            Some(id) => {
                let d = super::obter(&conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
                if d.tipo != tipo {
                    return Err(format!("o documento {id} não é um {tipo}"));
                }
                (id, "escolhido pelo agente".to_string())
            }
            None => {
                let variante: Option<String> = conn
                    .query_row("SELECT variante_id FROM vagas WHERE id = ?1", [vaga_id], |r| r.get(0))
                    .ok()
                    .flatten();
                let e = super::escolha::escolher(&conn, tipo, variante.as_deref()).ok_or_else(|| {
                    format!("não há {tipo} para anexar: marque um como preferido no app (Perfil → Currículos) ou gere um para a variante da vaga")
                })?;
                (e.id, e.regra.to_string())
            }
        };
        let a = super::seguranca::resolve_attachable(&conn, &self.data_dir, id)?;
        Ok(Attachment { id: a.id, nome: a.nome, formato: a.formato, tamanho: a.tamanho, sha256: a.sha256, path: a.path, regra })
    }
}
```

Se `open_db` for `pub(crate)` em outro módulo, usar o caminho real.

- [ ] **Step 3: Rodar** `cargo test --lib documentos::fonte` → PASS. Mutação: ignorar `vaga.variante_id` (passar `None`) → `picks_by_the_vagas_variant_and_resolves_safely` falha.

- [ ] **Step 4: Comandos Tauri** em `commands/documentos.rs`:

```rust
use crate::documentos::{self, Documento};
use tauri::{AppHandle, Manager};

fn data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

fn conn(app: &AppHandle) -> Result<rusqlite::Connection, String> {
    crate::mcp::tools::open_db(&data_dir(app)?)
}

#[tauri::command]
pub fn listar_documentos(app: AppHandle, tipo: String) -> Result<Vec<Documento>, String> {
    documentos::listar(&conn(&app)?, &tipo).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn importar_documento(app: AppHandle, tipo: String, caminho_origem: String, nome: Option<String>) -> Result<Documento, String> {
    documentos::importar::importar(&conn(&app)?, &data_dir(&app)?, &tipo, std::path::Path::new(&caminho_origem), nome.as_deref())
}

#[tauri::command]
pub fn definir_preferido(app: AppHandle, id: i64, variante_id: Option<String>) -> Result<(), String> {
    documentos::definir_preferido(&conn(&app)?, id, variante_id.as_deref())
}
```

Registrar os três no `invoke_handler` de `lib.rs`.

- [ ] **Step 5: MCP.** Em `mcp/server.rs`:

```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BrowserUploadArgs {
    /// Ref de browser_read do campo de arquivo ou do botão que abre o seletor de arquivo
    reference: String,
    /// "curriculo" (padrão) ou "carta"
    tipo: Option<String>,
    /// Opcional: id de um documento da biblioteca. Sem ele, a Claudia escolhe (preferido da variante da vaga, depois o preferido geral, depois o gerado da variante). Nunca um caminho de arquivo.
    documento: Option<i64>,
}
```

e a tool:

```rust
#[tool(
    description = "Anexa o currículo (ou a carta) da biblioteca da Claudia ao campo de arquivo indicado pela ref de browser_read, ou ao botão que abre o seletor de arquivo. A Claudia escolhe o arquivo; você nunca passa um caminho. Só funciona com a vaga ativa em 'candidatando'. Nunca envia a candidatura."
)]
async fn browser_upload(&self, Parameters(a): Parameters<BrowserUploadArgs>) -> Result<CallToolResult, rmcp::ErrorData> {
    self.browser_reply("browser_upload", self.browser.upload(&a.reference, a.tipo.as_deref(), a.documento).await)
}
```

Onde o `BrowserTools` é criado (≈ linha 462): `BrowserTools::new(state_file, guard, sink).with_documents(Arc::new(crate::documentos::fonte::DbDocumentSource { data_dir: cfg.data_dir.clone() }))`. A tool só fica disponível para a sessão de execução: seguir o mesmo filtro por `SessionKind` que as outras `browser_*` já usam (conferir em `server.rs`).

- [ ] **Step 6: Bloco mínimo no front** (`CurriculosView.tsx`), abaixo da lista atual: título "Para anexar", lista de `listar_documentos("curriculo")` com nome, origem, formato e botão estrela "Usar como preferido" (`definir_preferido({ id })`), e botão "Adicionar o meu" que usa `open` de `@tauri-apps/plugin-dialog` com filtros `pdf`/`docx` e chama `importar_documento({ tipo: "curriculo", caminhoOrigem, nome: null })`. Mostrar o erro do backend quando houver. Textos em `pt.ts`/`en.ts` (`profile.attach.title`, `profile.attach.importMine`, `profile.attach.makePreferred`, `profile.attach.preferred`, `profile.attach.empty`). Este bloco é provisório: a Parte 2 o substitui pela aba Currículos.

- [ ] **Step 7: Verificar.** `cargo test --lib` (exit 0), `npx tsc --noEmit` (exit 0), suíte live inteira (`--skip icon`, exit 0), smoke MCP (`tests/mcp-browser-smoke.mjs`) TUDO OK.

- [ ] **Step 8: Commit.** `git commit -m "feat: wire the document library to the app and the MCP (browser_upload, import, preferred)"`

---

### Tarefa 7: Ensinar o agente (BLOQUEADA até autorização explícita do usuário)

**Files:** `src-tauri/src/prompt_sistema_runtime.md`, `src-tauri/src/commands/prompts.rs` (teste do prompt)

- [ ] **Step 1: Pedir autorização ao usuário**, mostrando o texto exato a acrescentar. Sem um "pode editar o prompt", **pular esta tarefa** e registrar que o agente ainda não conhece a tool.
- [ ] **Step 2: Texto proposto** (na seção das ferramentas do browser, depois do bullet de `browser_scroll`):

```markdown
- `browser_upload` anexa o currículo (ou, com `tipo: "carta"`, a carta) ao campo de arquivo ou ao botão que abre o seletor, pela ref de `browser_read`. **Não escolha arquivo nem passe caminho:** a Claudia escolhe pela variante da vaga e pelas marcas do usuário, e diz qual usou. Depois, confira com `browser_read` ou `browser_text` que o nome do arquivo apareceu. Se a tool disser que não há currículo para anexar, registre uma pendência e não siga com a candidatura.
```

- [ ] **Step 3: Teste do prompt** em `commands/prompts.rs`: acrescentar `"browser_upload"` à lista do teste `the_shipped_runtime_prompt_no_longer_sends_the_agent_to_the_extension` (que exige cada tool no prompt). Rodar → PASS. Mutação: tirar a linha do prompt → o teste falha.
- [ ] **Step 4: Commit.** `git commit -m "docs(prompt): teach browser_upload (approved by the user)"`

---

### Tarefa 8: Rodada de testes da Parte 1 e portão

Seguir o `CLAUDE.md` (Processo de desenvolvimento, seções 3 a 6).

- [ ] **Step 1: Unitários e regressão:** `cargo test --lib`, `npx tsc --noEmit`, `npm run build` — exit 0.
- [ ] **Step 2: Integração:** suíte live inteira (`--skip icon`) e smoke MCP — exit 0 / TUDO OK.
- [ ] **Step 3: Rodada de agente** (Haiku faz, Opus observa): o agente, pelo MCP do binário, abre `upload-form.html` (Chrome de teste, pasta temporária, `CLAUDIA_ALLOW_LOCAL_URLS`), lê, anexa nos três campos, tenta anexar no "Enviar candidatura" e no "Botão que não abre nada". O Opus cruza transcript, Activity log (linhas `browser_upload` com nome/formato/tamanho, **sem conteúdo**) e prints.
- [ ] **Step 4: Smoke de uso real no app** (`/claudia-teste`): Perfil → Currículos → bloco "Para anexar": importar um PDF próprio, marcar preferido; conferir no banco (`banco.mjs "SELECT id, nome, origem, formato FROM documentos"`) e na pasta `documentos/`. Prints.
- [ ] **Step 5: Upload num site real — só se o usuário autorizar a rodada** (ex.: LinkedIn Easy Apply até o passo do currículo, **sem enviar**). Sem autorização, registrar como "não testado em site real".
- [ ] **Step 6: Review Opus** do diff da Parte 1 (segurança do caminho, robustez do upload, guarda). Corrigir ou registrar os achados.
- [ ] **Step 7: Portão.** Relatório ao usuário: o que foi feito, evidências (exit codes, FAILED das mutações, prints), o que não foi verificado. **A Parte 2 só começa com a aprovação dele.**

---

# PARTE 2 — O resto (depois do portão da Parte 1)

A Tarefa 9 (PDF) vem detalhada abaixo. Os passos de código das Tarefas 10 a 16 são escritos numa revisão deste plano **depois** do portão, com o que a Parte 1 ensinar (papel AX do input de arquivo, nomes reais dos helpers). Aqui ficam o escopo, os arquivos, as interfaces e os critérios de aceite, todos da spec.

### Tarefa 9: PDF dos currículos gerados pelo WebView2 do app

Começa por uma prova do caminho. **Se o PDF sair ruim (texto cortado, cores erradas, página errada), parar e voltar ao usuário** antes do Step 7.

**Files:**
- Create: `src-tauri/src/pdf.rs`
- Modify: `src-tauri/Cargo.toml` (dependências Windows), `src-tauri/src/lib.rs` (`mod pdf;`, comando de prova), `src-tauri/src/commands/curriculos.rs` (`gerar_curriculo` gera o PDF e registra); `src-tauri/capabilities/` não muda (a janela invisível não ganha capability: sem IPC)

**Interfaces:**
- Produces: `pub async fn html_to_pdf(app: &tauri::AppHandle, html: &std::path::Path, pdf: &std::path::Path) -> Result<(), String>`

- [ ] **Step 1: Dependências.** Em `src-tauri/Cargo.toml`, sob `[target.'cfg(windows)'.dependencies]` (criar a seção se não existir; não mexer em `windows-sys`):

```toml
webview2-com = "0.38"
windows = { version = "0.61", features = ["Win32_Foundation"] }
windows-core = "0.61"
```

Rodar `cargo tree -i webview2-com` e confirmar **uma só versão** (a do `wry`). Se aparecerem duas, alinhar a versão à do `wry`.

- [ ] **Step 2: Escrever `src-tauri/src/pdf.rs`.**

```rust
//! HTML -> PDF with the app's OWN WebView2 (the Edge engine Tauri already runs on), in a hidden window.
//! No browser of ours, no new runtime: the same HTML the user previews becomes the PDF.
use std::path::Path;
use std::time::Duration;

const LOAD_TIMEOUT: Duration = Duration::from_secs(15);
const PRINT_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn html_to_pdf(app: &tauri::AppHandle, html: &Path, pdf: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        win::print(app, html, pdf).await
    }
    #[cfg(not(windows))]
    {
        let _ = (app, html, pdf);
        Err("gerar PDF só está disponível no Windows".into())
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tauri::{webview::PageLoadEvent, WebviewUrl, WebviewWindowBuilder};
    use tokio::sync::oneshot;
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Environment6, ICoreWebView2_7};
    use webview2_com::PrintToPdfCompletedHandler;
    use windows::Win32::Foundation::BOOL;
    use windows_core::{Interface, HSTRING};

    type Slot<T> = Arc<Mutex<Option<oneshot::Sender<T>>>>;

    fn send<T>(slot: &Slot<T>, v: T) {
        if let Some(tx) = slot.lock().ok().and_then(|mut g| g.take()) {
            let _ = tx.send(v);
        }
    }

    pub async fn print(app: &tauri::AppHandle, html: &Path, pdf: &Path) -> Result<(), String> {
        let url = tauri::Url::from_file_path(html).map_err(|_| "caminho do HTML inválido".to_string())?;
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let label = format!("pdf-{nanos}");
        let (loaded_tx, loaded_rx) = oneshot::channel::<()>();
        let loaded: Slot<()> = Arc::new(Mutex::new(Some(loaded_tx)));
        let loaded_cb = loaded.clone();
        // Hidden, A4-ish size, no capability for this label: the page has no IPC to the app.
        let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
            .visible(false)
            .inner_size(794.0, 1123.0)
            .on_page_load(move |_w, p| {
                if p.event() == PageLoadEvent::Finished {
                    send(&loaded_cb, ());
                }
            })
            .build()
            .map_err(|e| format!("não foi possível abrir a janela de impressão: {e}"))?;

        let result = async {
            tokio::time::timeout(LOAD_TIMEOUT, loaded_rx)
                .await
                .map_err(|_| "o HTML não carregou a tempo".to_string())?
                .map_err(|_| "a janela de impressão fechou".to_string())?;
            let (tx, rx) = oneshot::channel::<Result<(), String>>();
            let slot: Slot<Result<(), String>> = Arc::new(Mutex::new(Some(tx)));
            let out = HSTRING::from(pdf.as_os_str());
            let slot_wv = slot.clone();
            win.with_webview(move |wv| {
                let slot_cb = slot_wv.clone();
                let started: windows_core::Result<()> = unsafe {
                    (|| {
                        let core = wv.controller().CoreWebView2()?;
                        let core7: ICoreWebView2_7 = core.cast()?;
                        let env6: ICoreWebView2Environment6 = wv.environment().cast()?;
                        let s = env6.CreatePrintSettings()?;
                        s.SetShouldPrintBackgrounds(true)?;
                        s.SetShouldPrintHeaderAndFooter(false)?;
                        s.SetPageWidth(8.27)?; // A4, inches
                        s.SetPageHeight(11.69)?;
                        s.SetMarginTop(0.0)?;
                        s.SetMarginBottom(0.0)?;
                        s.SetMarginLeft(0.0)?;
                        s.SetMarginRight(0.0)?;
                        let handler = PrintToPdfCompletedHandler::create(Box::new(move |hr: windows_core::Result<()>, ok: BOOL| {
                            send(&slot_cb, match (hr, ok.as_bool()) {
                                (Ok(()), true) => Ok(()),
                                (Ok(()), false) => Err("o WebView2 não gerou o PDF".to_string()),
                                (Err(e), _) => Err(format!("erro do WebView2 ao gerar o PDF: {e}")),
                            });
                            Ok(())
                        }));
                        core7.PrintToPdf(&out, &s, &handler)
                    })()
                };
                if let Err(e) = started {
                    send(&slot_wv, Err(format!("não foi possível iniciar a impressão: {e}")));
                }
            })
            .map_err(|e| e.to_string())?;
            tokio::time::timeout(PRINT_TIMEOUT, rx)
                .await
                .map_err(|_| "a geração do PDF demorou demais".to_string())?
                .map_err(|_| "a impressão foi interrompida".to_string())?
        }
        .await;
        let _ = win.destroy();
        result
    }
}
```

Os tipos exatos do callback (`windows_core::Result<()>`, `BOOL`) vêm de `webview2-com-macros` 0.8.1 (`completed_callback`). Se o compilador discordar, siga a mensagem dele; **não mude a lógica**.

- [ ] **Step 3: Comando de prova (temporário, removido no Step 7).** Em `src-tauri/src/lib.rs`: `mod pdf;` e

```rust
#[tauri::command]
async fn debug_pdf_prova(app: tauri::AppHandle, html: String, pdf: String) -> Result<(), String> {
    crate::pdf::html_to_pdf(&app, std::path::Path::new(&html), std::path::Path::new(&pdf)).await
}
```

registrado no `invoke_handler` **só com `#[cfg(debug_assertions)]`**.

- [ ] **Step 4: Compilar.** `cargo build` (com `CARGO_TARGET_DIR` do scratchpad). Esperado: compila.

- [ ] **Step 5: Prova no app rodando** (skill `claudia-teste`): gerar um currículo pela tela (Perfil → Currículos, um template em pt) para ter um HTML real; depois `node .claude/skills/claudia-teste/scripts/invocar.mjs debug_pdf_prova '{"html":"<caminho do cv_*.html>","pdf":"<scratchpad>\\prova.pdf"}' --escrever`. Conferir: o arquivo começa com `%PDF-`; abrir o PDF e tirar print da página 1 (e da 2, se houver); comparar com a folha do HTML. **Nenhuma janela pode aparecer na tela.**

- [ ] **Step 6: Portão técnico.** Mostrar ao usuário os prints do PDF lado a lado com o HTML. Se a qualidade não servir (margens, cores, quebras), **parar** e decidir com o usuário antes do Step 7.

- [ ] **Step 7: `gerar_curriculo` gera o PDF e registra** (depois do portão técnico). Remover `debug_pdf_prova`. Tornar o comando `async` e, depois de gravar o HTML (código atual), acrescentar:

```rust
let data = app.path().app_data_dir().map_err(|e| e.to_string())?;
let docs = crate::documentos::docs_dir(&data);
std::fs::create_dir_all(&docs).map_err(|e| e.to_string())?;
let pdf_name = file_name.replace(".html", ".pdf");
let pdf_path = docs.join(&pdf_name);
crate::pdf::html_to_pdf(&app, &path, &pdf_path).await?;
let bytes = std::fs::read(&pdf_path).map_err(|e| e.to_string())?;
if !bytes.starts_with(b"%PDF-") {
    return Err("o PDF gerado é inválido".into());
}
let sha = crate::documentos::seguranca::sha256_file(&pdf_path).map_err(|e| e.to_string())?;
let conn = crate::mcp::tools::open_db(&data)?;
crate::documentos::inserir(&conn, &crate::documentos::NovoDocumento {
    tipo: "curriculo",
    nome: &format!("{template_nome} ({idioma_str})"),
    origem: "gerado",
    arquivo: &pdf_name,
    arquivo_html: Some(&path.to_string_lossy()),
    formato: "pdf",
    idioma: Some(idioma_str),
    sha256: &sha,
    tamanho: bytes.len() as i64,
    variante_id: None,
}).map_err(|e| e.to_string())?;
```

(`template_nome` e `idioma_str` já existem no comando; mover o cálculo de `template_nome` para antes deste bloco.) A variante do currículo gerado entra na Parte 2 (o assistente de geração ganha o seletor de variante).

- [ ] **Step 8: Commit.**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/pdf.rs src-tauri/src/lib.rs src-tauri/src/commands/curriculos.rs
git commit -m "feat(pdf): generated CVs become PDFs via the app's own WebView2 and join the library"
```

---

### Tarefa 10: Variante da vaga escolhida pelo agente + variante principal
- **Files:** `mcp/tools/vagas.rs` (`register_vaga`/`update_vaga_status` aceitam `variante_id`, validado contra `search_variants.yaml`), nova tool `set_vaga_variante`, `commands/perfil.rs` (`principal: true` em no máximo uma variante), `documentos/fonte.rs` (sem variante na vaga → variante principal).
- **Aceite:** testes de validação (variante inexistente recusada; duas principais impossível); o resultado do upload diz a variante e a regra.

### Tarefa 11: Comandos completos da biblioteca
- **Files:** `commands/documentos.rs` (`obter_documento` com `preview_html`, `renomear_documento`, `atualizar_tags_documento`, `remover_preferido`, `abrir_documento`, `apagar_documento(id, forcar)` só dentro da pasta e por id, `listar_variantes`), `gerar_curriculo`/`gerar_cover_letter` com `variante_id` e tags automáticas (variante, template, cor, idioma); remoção de `listar_curriculos`, `apagar_curriculo`, `abrir_curriculo`, `listar_cover_letters`, `abrir_cover_letter`, `apagar_cover_letter`.
- **Aceite:** o contrato da seção 6 da spec, campo a campo; apagar fora da pasta é impossível (teste + mutação).

### Tarefa 12: Migração dos currículos e cartas antigos
- **Files:** `documentos/migrar.rs`; chamada na abertura do app.
- **Aceite:** cada `curriculos/*.html` e carta existente vira documento `gerado` com PDF; idempotente; **nada é apagado** (teste conferindo que os originais continuam lá).

### Tarefa 13: Aba Currículos (sidebar, sub-abas, grade, página A4, Voltar)
- **Files:** `Sidebar.tsx`, `App.tsx` (view `documentos`), `src/components/documentos/` (aba, grade, cartão com miniatura em `<iframe sandbox srcdoc>` carregada por IntersectionObserver, página do documento com folha A4 e **← Voltar**, tags editáveis, preferido geral e por variante, renomear, abrir, excluir com confirmação), Perfil sem as telas internas de currículos/cartas (substitui o bloco provisório da Tarefa 6), seletor de preferido nas variantes, i18n.
- **Aceite:** smoke `/claudia-teste` nos dois idiomas, estados vazio/erro/com dados, prints.

### Tarefa 14: MCP de documentos, variantes e cover letter
- **Files:** `mcp/server.rs`, `mcp/tools/` (`list_variantes`, `get_variante`, `update_variante` só Perfil com a mesma validação/backup de `guardar_search_variants`, `list_documentos`, `get_documento` com texto só de gerados, `save_cover_letter` renderizando HTML A4 + PDF, `set_documento_preferido` só Perfil).
- **Aceite:** nenhuma tool aceita ou devolve caminho (teste); escrita recusada na sessão de execução (teste por `SessionKind`).

### Tarefa 15: Onde o documento aparece (Dashboard, Histórico, Pendências, Activity log, Welcome)
- **Files:** `candidaturas.documento_id`, `Vagas.tsx`/dashboard, histórico, `Pendencias.tsx` (categoria "sem currículo/carta para anexar"), `ActivityLogScreen.tsx` (linhas de upload), `Welcome.tsx`.
- **Aceite:** smoke no app com prints.

### Tarefa 16: Rodada de testes da Parte 2 e entrega
- As mesmas rodadas da Tarefa 8, mais review Opus do branch inteiro. Integração (merge/push) é decisão do usuário.
