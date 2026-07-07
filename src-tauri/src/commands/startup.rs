use tauri::command;

const APP_NAME: &str = "ClaudiaRH";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[command]
pub fn obter_iniciar_com_sistema() -> Result<bool, String> {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run = hkcu.open_subkey(RUN_KEY).map_err(|e| e.to_string())?;
        Ok(run.get_value::<String, _>(APP_NAME).is_ok())
    }
    #[cfg(not(windows))]
    Ok(false)
}

#[command]
pub fn configurar_iniciar_com_sistema(ativo: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run = hkcu
            .open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
            .map_err(|e| e.to_string())?;
        if ativo {
            let exe = std::env::current_exe()
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .to_string();
            run.set_value(APP_NAME, &exe).map_err(|e| e.to_string())?;
        } else {
            let _ = run.delete_value(APP_NAME);
        }
    }
    #[cfg(not(windows))]
    let _ = ativo;
    Ok(())
}
