use crate::{NotifConfig, NotifState};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Manager, State};

#[derive(Serialize)]
pub struct ConfigNotif {
    pub ativo: bool,
    pub intervalo_minutos: u32,
}

impl From<&NotifConfig> for ConfigNotif {
    fn from(c: &NotifConfig) -> Self {
        ConfigNotif { ativo: c.ativo, intervalo_minutos: c.intervalo_minutos }
    }
}

#[tauri::command]
pub fn obter_config_notif(state: State<'_, NotifState>) -> Result<ConfigNotif, String> {
    let cfg = state.0.lock().map_err(|e| e.to_string())?;
    Ok(ConfigNotif::from(&*cfg))
}

#[tauri::command]
pub fn configurar_notif(
    state: State<'_, NotifState>,
    app: AppHandle,
    ativo: bool,
    intervalo_minutos: u32,
) -> Result<(), String> {
    {
        let mut cfg = state.0.lock().map_err(|e| e.to_string())?;
        cfg.ativo = ativo;
        cfg.intervalo_minutos = intervalo_minutos;
    }
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    salvar_config_notif(&data_dir, ativo, intervalo_minutos)
}

fn salvar_config_notif(
    data_dir: &Path,
    ativo: bool,
    intervalo_minutos: u32,
) -> Result<(), String> {
    let payload = serde_json::json!({ "ativo": ativo, "intervalo_minutos": intervalo_minutos });
    let content = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    std::fs::write(data_dir.join("notif.json"), content).map_err(|e| e.to_string())
}

pub fn load_notif_config(data_dir: &Path) -> NotifConfig {
    let path = data_dir.join("notif.json");
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<NotifConfig>(&content) {
            return cfg;
        }
    }
    NotifConfig { ativo: true, intervalo_minutos: 10 }
}
