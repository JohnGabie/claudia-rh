use tauri_plugin_updater::UpdaterExt;

#[derive(serde::Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub body: String,
}

#[tauri::command]
pub async fn verificar_atualizacao(app: tauri::AppHandle) -> Result<Option<UpdateInfo>, String> {
    let updater = match app.updater() {
        Ok(u) => u,
        Err(_) => return Ok(None),
    };
    let update = updater.check().await.map_err(|e| e.to_string())?;
    Ok(update.map(|u| UpdateInfo {
        version: u.version,
        body: u.body.clone().unwrap_or_default(),
    }))
}

#[tauri::command]
pub async fn instalar_atualizacao(app: tauri::AppHandle) -> Result<(), String> {
    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => return Err(e.to_string()),
    };
    let update = updater.check().await.map_err(|e| e.to_string())?;
    if let Some(update) = update {
        update
            .download_and_install(|_, _| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
        app.restart();
    }
    Ok(())
}
