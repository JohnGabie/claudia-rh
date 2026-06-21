use crate::pty_manager;
use tauri::AppHandle;

#[tauri::command]
pub fn iniciar_pty(
    app: AppHandle,
    comando: String,
    args: Vec<String>,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    pty_manager::iniciar(app, comando, args, rows, cols)
}

#[tauri::command]
pub fn escrever_pty(input: String) -> Result<(), String> {
    pty_manager::escrever(input)
}

#[tauri::command]
pub fn redimensionar_pty(rows: u16, cols: u16) -> Result<(), String> {
    pty_manager::redimensionar(rows, cols)
}

#[tauri::command]
pub fn parar_pty() {
    pty_manager::parar()
}
