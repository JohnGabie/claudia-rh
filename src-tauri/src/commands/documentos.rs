//! Tauri commands of the document library (spec section 6). All async: disk and hashing work runs
//! off the main thread.
use crate::documentos;
use crate::documentos::biblioteca::{self, DocumentoDetalhe, DocumentoView, Tag, VarianteResumo};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

fn data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

/// Tauri runs sync commands on the main thread: a 10 MB hash there freezes the window. Every document
/// command is async and does its disk work here.
async fn bloqueante<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn listar_documentos(app: AppHandle, tipo: String) -> Result<Vec<DocumentoView>, String> {
    let dir = data_dir(&app)?;
    bloqueante(move || biblioteca::listar_view(&biblioteca::open_conn(&dir)?, &tipo)).await
}

#[tauri::command]
pub async fn obter_documento(app: AppHandle, id: i64) -> Result<DocumentoDetalhe, String> {
    let dir = data_dir(&app)?;
    bloqueante(move || biblioteca::detalhe(&biblioteca::open_conn(&dir)?, &dir, id)).await
}

#[tauri::command]
pub async fn importar_documento(app: AppHandle, tipo: String, caminho_origem: String, nome: Option<String>) -> Result<DocumentoView, String> {
    let dir = data_dir(&app)?;
    bloqueante(move || {
        let conn = biblioteca::open_conn(&dir)?;
        let d = documentos::importar::importar(&conn, &dir, &tipo, std::path::Path::new(&caminho_origem), nome.as_deref())?;
        biblioteca::ver(&conn, d.id)
    })
    .await
}

#[tauri::command]
pub async fn renomear_documento(app: AppHandle, id: i64, nome: String) -> Result<(), String> {
    let dir = data_dir(&app)?;
    bloqueante(move || biblioteca::renomear(&biblioteca::open_conn(&dir)?, id, &nome)).await
}

#[tauri::command]
pub async fn atualizar_tags_documento(app: AppHandle, id: i64, tags: Vec<Tag>) -> Result<(), String> {
    let dir = data_dir(&app)?;
    bloqueante(move || biblioteca::gravar_tags(&biblioteca::open_conn(&dir)?, id, &tags)).await
}

#[tauri::command]
pub async fn definir_preferido(app: AppHandle, id: i64, variante_id: Option<String>) -> Result<(), String> {
    let dir = data_dir(&app)?;
    bloqueante(move || documentos::definir_preferido(&biblioteca::open_conn(&dir)?, id, variante_id.as_deref())).await
}

#[tauri::command]
pub async fn remover_preferido(app: AppHandle, tipo: String, variante_id: Option<String>) -> Result<(), String> {
    let dir = data_dir(&app)?;
    bloqueante(move || biblioteca::remover_preferido(&biblioteca::open_conn(&dir)?, &tipo, variante_id.as_deref())).await
}

/// Does not check the sha256: opening a file the user edited outside is legitimate; attaching one is
/// not (the upload still goes through `resolve_attachable`).
#[tauri::command]
pub async fn abrir_documento(app: AppHandle, id: i64) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let path = bloqueante(move || {
        let conn = biblioteca::open_conn(&dir)?;
        let d = documentos::obter(&conn, id).map_err(|e| e.to_string())?.ok_or_else(|| format!("o documento {id} não existe"))?;
        biblioteca::dentro_da_pasta(&dir, &d.arquivo).ok_or_else(|| format!("o arquivo de \"{}\" sumiu: gere ou importe de novo", d.nome))
    })
    .await?;
    app.opener().open_path(path.to_string_lossy().to_string(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apagar_documento(app: AppHandle, id: i64) -> Result<(), String> {
    let dir = data_dir(&app)?;
    bloqueante(move || biblioteca::apagar(&biblioteca::open_conn(&dir)?, &dir, id)).await
}

#[tauri::command]
pub async fn listar_variantes(app: AppHandle) -> Result<Vec<VarianteResumo>, String> {
    let vs = crate::commands::perfil::ler_search_variants(app)?;
    Ok(vs.into_iter().map(|v| VarianteResumo { id: v.id, nome_exibicao: v.nome_exibicao, ativa: v.ativa }).collect())
}

#[cfg(test)]
mod tests {
    /// The old commands took a path from the UI and deleted or opened it (spec 1.6). They must stay
    /// out of the handler for good.
    #[test]
    fn the_old_path_commands_are_gone() {
        let lib = include_str!("../lib.rs");
        for old in ["listar_curriculos", "abrir_curriculo", "apagar_curriculo", "listar_cover_letters", "abrir_cover_letter", "apagar_cover_letter"] {
            assert!(!lib.contains(old), "{old} took a path from the UI and must stay unregistered");
        }
    }
}
