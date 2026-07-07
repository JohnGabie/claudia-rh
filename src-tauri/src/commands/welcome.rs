use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Serialize)]
pub struct SetupStatus {
    pub claude_instalado: bool,
    pub nativehost_chrome: bool,
    pub perfil_preenchido: bool,
}

#[tauri::command]
pub fn verificar_setup(app: AppHandle) -> SetupStatus {
    let claude_instalado = std::process::Command::new(super::claude_program())
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let nativehost_chrome = {
        #[cfg(windows)]
        {
            use winreg::enums::HKEY_CURRENT_USER;
            use winreg::RegKey;
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            hkcu.open_subkey(
                r"Software\Google\Chrome\NativeMessagingHosts\com.anthropic.claude_code_browser_extension",
            )
            .is_ok()
        }
        #[cfg(not(windows))]
        {
            let home = std::env::var("HOME").unwrap_or_default();
            std::path::Path::new(&home)
                .join(".claude/chrome/chrome-native-host")
                .exists()
        }
    };

    let perfil_preenchido = app
        .path()
        .app_data_dir()
        .ok()
        .and_then(|d| std::fs::read_to_string(d.join("candidate_base.yaml")).ok())
        .map(|c| c.trim().len() > 50)
        .unwrap_or(false);

    SetupStatus { claude_instalado, nativehost_chrome, perfil_preenchido }
}

#[tauri::command]
pub fn welcome_necessario(app: AppHandle) -> bool {
    let Ok(data_dir) = app.path().app_data_dir() else { return true };
    !data_dir.join("welcome_seen").exists()
}

#[tauri::command]
pub fn marcar_welcome_concluido(app: AppHandle) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::write(data_dir.join("welcome_seen"), "1").map_err(|e| e.to_string())
}
