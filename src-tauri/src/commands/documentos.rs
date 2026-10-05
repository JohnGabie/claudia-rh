//! Tauri commands of the document library (spec section 6). Wave 0 fixes the signatures; wave 1 (B2)
//! fills the bodies marked NOT_YET.
use crate::documentos::biblioteca::{self, DocumentoDetalhe, DocumentoView, Tag, VarianteResumo};
use crate::documentos;
use tauri::{AppHandle, Manager};

const NOT_YET: &str = "não implementado ainda (Parte 2, onda 1)";

fn data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn listar_documentos(app: AppHandle, tipo: String) -> Result<Vec<DocumentoView>, String> {
    biblioteca::listar_view(&biblioteca::open_conn(&data_dir(&app)?)?, &tipo)
}

#[tauri::command]
pub fn obter_documento(_app: AppHandle, _id: i64) -> Result<DocumentoDetalhe, String> {
    Err(NOT_YET.into())
}

#[tauri::command]
pub fn importar_documento(app: AppHandle, tipo: String, caminho_origem: String, nome: Option<String>) -> Result<DocumentoView, String> {
    let dir = data_dir(&app)?;
    let conn = biblioteca::open_conn(&dir)?;
    let d = documentos::importar::importar(&conn, &dir, &tipo, std::path::Path::new(&caminho_origem), nome.as_deref())?;
    biblioteca::ver(&conn, d.id)
}

#[tauri::command]
pub fn renomear_documento(_app: AppHandle, _id: i64, _nome: String) -> Result<(), String> {
    Err(NOT_YET.into())
}

#[tauri::command]
pub fn atualizar_tags_documento(_app: AppHandle, _id: i64, _tags: Vec<Tag>) -> Result<(), String> {
    Err(NOT_YET.into())
}

#[tauri::command]
pub fn definir_preferido(app: AppHandle, id: i64, variante_id: Option<String>) -> Result<(), String> {
    documentos::definir_preferido(&biblioteca::open_conn(&data_dir(&app)?)?, id, variante_id.as_deref())
}

#[tauri::command]
pub fn remover_preferido(_app: AppHandle, _tipo: String, _variante_id: Option<String>) -> Result<(), String> {
    Err(NOT_YET.into())
}

#[tauri::command]
pub fn abrir_documento(_app: AppHandle, _id: i64) -> Result<(), String> {
    Err(NOT_YET.into())
}

#[tauri::command]
pub fn apagar_documento(_app: AppHandle, _id: i64) -> Result<(), String> {
    Err(NOT_YET.into())
}

#[tauri::command]
pub fn listar_variantes(_app: AppHandle) -> Result<Vec<VarianteResumo>, String> {
    Err(NOT_YET.into())
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
