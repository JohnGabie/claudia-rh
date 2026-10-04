//! Tauri commands of the document library (attach-CV plan, part 1): list, import, mark preferred.
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
