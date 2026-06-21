use keyring::{Entry, Error};

const PREFIX: &str = "claudia-rh";

// Stores the username for a service so obter_credencial(servico) works without knowing it.
fn meta_entry(servico: &str) -> Result<Entry, String> {
    Entry::new(&format!("{}:{}:meta", PREFIX, servico), "utilizador")
        .map_err(|e| e.to_string())
}

fn cred_entry(servico: &str, utilizador: &str) -> Result<Entry, String> {
    Entry::new(&format!("{}:{}", PREFIX, servico), utilizador)
        .map_err(|e| e.to_string())
}

/// Stores a credential in the Windows Credential Manager.
/// The password never touches any file or log — only the keyring.
#[tauri::command]
pub fn guardar_credencial(
    servico: String,
    utilizador: String,
    password: String,
) -> Result<(), String> {
    meta_entry(&servico)?
        .set_password(&utilizador)
        .map_err(|e| e.to_string())?;
    cred_entry(&servico, &utilizador)?
        .set_password(&password)
        .map_err(|e| e.to_string())
}

/// Returns the stored password for a service. Used internally by Phase 5
/// to inject credentials as environment variables — never passed as CLI arguments
/// or written to any terminal output.
#[tauri::command]
pub fn obter_credencial(servico: String) -> Result<String, String> {
    let utilizador = meta_entry(&servico)?
        .get_password()
        .map_err(|e| e.to_string())?;
    cred_entry(&servico, &utilizador)?
        .get_password()
        .map_err(|e| e.to_string())
}

/// Returns the stored username (not password) for a service, or None if no
/// credential exists. Safe to expose to the frontend.
#[tauri::command]
pub fn obter_utilizador_credencial(servico: String) -> Result<Option<String>, String> {
    match meta_entry(&servico)?.get_password() {
        Ok(u) => Ok(Some(u)),
        Err(Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Removes a credential from the Windows Credential Manager.
#[tauri::command]
pub fn apagar_credencial(servico: String) -> Result<(), String> {
    let utilizador = match meta_entry(&servico)?.get_password() {
        Ok(u) => u,
        Err(Error::NoEntry) => return Ok(()),
        Err(e) => return Err(e.to_string()),
    };
    let _ = meta_entry(&servico).and_then(|e| e.delete_credential().map_err(|e2| e2.to_string()));
    let _ = cred_entry(&servico, &utilizador)
        .and_then(|e| e.delete_credential().map_err(|e2| e2.to_string()));
    Ok(())
}
