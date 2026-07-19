mod commands;
mod db;
mod db_watcher;
mod idle_watcher;
mod notificacoes;
mod prompt;
mod pty_manager;

use commands::cover_letter::{gerar_cover_letter, listar_cover_letters, abrir_cover_letter, apagar_cover_letter};
use commands::linkedin::{iniciar_busca_linkedin_rede, listar_vagas_linkedin_rede, obter_status_linkedin_rede};
use commands::updater::{instalar_atualizacao, verificar_atualizacao};
use commands::startup::{obter_iniciar_com_sistema, configurar_iniciar_com_sistema};
use commands::prompts::{abrir_ficheiro_prompt, abrir_pasta_dados};
use commands::curriculos::{gerar_curriculo, listar_curriculos, abrir_curriculo, apagar_curriculo};
use commands::credenciais::{
    apagar_credencial, guardar_credencial, obter_credencial, obter_utilizador_credencial,
};
use commands::estado::{
    abrir_pasta, atividade_recente, candidaturas_hoje, contar_pendencias, contar_propostas,
    ignorar_proposta, listar_candidaturas, listar_pendencias, listar_propostas, listar_vagas,
    pular_pendencia, resolver_pendencia, resumo_memoria_recente, vaga_atual_sessao,
    vagas_analisadas_hoje, vagas_analisadas_total, tempo_sessoes_hoje,
};
use commands::feedback::{
    agregar_dados_feedback, gerar_feedback, listar_feedbacks, marcar_resultado_candidatura,
    sugerir_feedback,
};
use commands::notif::{configurar_notif, load_notif_config, obter_config_notif};
use commands::perfil::{
    enviar_mensagem_perfil, escrever_para_perfil_chrome, guardar_candidato_base, guardar_estrategia,
    guardar_search_variants, guardar_pesos_variantes, guardar_variante_unica, iniciar_sessao_perfil, iniciar_sessao_perfil_chrome, interromper_perfil, ler_candidato_base,
    ler_estrategia, ler_search_variants, remover_ultima_troca_perfil,
};
use commands::pty::{escrever_pty, iniciar_pty, parar_pty, redimensionar_pty};
use commands::sessao::{
    configurar_disparo, configurar_limite_diario, configurar_modo_autonomo,
    disparar_sessao, obter_config_disparo, obter_modo_autonomo,
    registar_pausa_sessao, registar_retoma_sessao,
};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

pub struct DbState(pub Arc<Mutex<Connection>>);

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct JanelaAgendamento {
    pub dia_semana: u8,   // 0=Dom, 1=Seg, …, 6=Sab
    pub inicio: String,   // "09:00"
    pub fim: String,      // "17:00"
    pub ativo: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IdleConfig {
    pub ativo: bool,
    pub limiar_minutos: u32,
    #[serde(default = "default_limite_diario")]
    pub limite_diario: u32,
    #[serde(default)]
    pub limite_tempo_minutos: u32,
    #[serde(default)]
    pub janelas: Vec<JanelaAgendamento>,
}

fn default_limite_diario() -> u32 { 10 }

pub struct IdleState(pub Arc<Mutex<IdleConfig>>);

#[derive(Serialize, Deserialize, Clone)]
pub struct NotifConfig {
    pub ativo: bool,
    pub intervalo_minutos: u32,
}

pub struct NotifState(pub Arc<Mutex<NotifConfig>>);

fn load_idle_config(data_dir: &std::path::Path) -> IdleConfig {
    let path = data_dir.join("disparo.json");
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<IdleConfig>(&content) {
            return cfg;
        }
    }
    IdleConfig { ativo: true, limiar_minutos: 15, limite_diario: 10, limite_tempo_minutos: 0, janelas: vec![] }
}

#[tauri::command]
fn minimize_window(window: tauri::Window) {
    let _ = window.minimize();
}

#[tauri::command]
fn maximize_window(window: tauri::Window) {
    if window.is_maximized().unwrap_or(false) {
        let _ = window.unmaximize();
    } else {
        let _ = window.maximize();
    }
}

#[tauri::command]
fn close_window(window: tauri::Window) {
    let _ = window.close();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            // Database
            let conn = db::init(&data_dir.join("claudia_rh.db"))?;
            let conn_arc = Arc::new(Mutex::new(conn));
            app.manage(DbState(Arc::clone(&conn_arc)));

            // Idle config
            let idle_cfg = load_idle_config(&data_dir);
            let idle_arc = Arc::new(Mutex::new(idle_cfg));
            app.manage(IdleState(Arc::clone(&idle_arc)));

            // Notification config
            let notif_cfg = load_notif_config(&data_dir);
            let notif_arc = Arc::new(Mutex::new(notif_cfg));
            app.manage(NotifState(Arc::clone(&notif_arc)));

            if let Some(window) = app.get_webview_window("main") {
                // Close button → hide to tray instead of quitting
                let w = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                });
            }

            // System tray icon
            let tray_menu = MenuBuilder::new(app)
                .text("show", "Abrir Claudia RH")
                .separator()
                .text("quit", "Sair")
                .build()?;

            if let Some(icon) = app.default_window_icon() {
                let tray = TrayIconBuilder::with_id("main")
                    .icon(icon.clone())
                    .tooltip("Claudia RH")
                    .menu(&tray_menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(w) = app.get_webview_window("main") {
                                if w.is_visible().unwrap_or(false) {
                                    let _ = w.hide();
                                } else {
                                    let _ = w.show();
                                    let _ = w.set_focus();
                                }
                            }
                        }
                    })
                    .build(app)?;
                // Keep the tray alive for the process lifetime
                std::mem::forget(tray);
            }

            // Ensure all prompt files exist on disk
            commands::prompts::ensure_all_prompts(&data_dir);

            // Start background tasks
            idle_watcher::start(app.handle().clone(), idle_arc, Arc::clone(&conn_arc));
            db_watcher::start(app.handle().clone(), Arc::clone(&conn_arc));
            notificacoes::start(app.handle().clone(), Arc::clone(&conn_arc));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            iniciar_pty,
            escrever_pty,
            redimensionar_pty,
            parar_pty,
            minimize_window,
            maximize_window,
            close_window,
            listar_vagas,
            listar_pendencias,
            resumo_memoria_recente,
            candidaturas_hoje,
            atividade_recente,
            contar_pendencias,
            resolver_pendencia,
            pular_pendencia,
            listar_candidaturas,
            contar_propostas,
            listar_propostas,
            ignorar_proposta,
            vaga_atual_sessao,
            vagas_analisadas_hoje,
            vagas_analisadas_total,
            tempo_sessoes_hoje,
            abrir_pasta,
            ler_candidato_base,
            guardar_candidato_base,
            ler_search_variants,
            guardar_search_variants,
            guardar_pesos_variantes,
            guardar_variante_unica,
            ler_estrategia,
            guardar_estrategia,
            iniciar_sessao_perfil,
            iniciar_sessao_perfil_chrome,
            enviar_mensagem_perfil,
            escrever_para_perfil_chrome,
            interromper_perfil,
            remover_ultima_troca_perfil,
            guardar_credencial,
            obter_credencial,
            obter_utilizador_credencial,
            apagar_credencial,
            disparar_sessao,
            obter_config_disparo,
            configurar_disparo,
            configurar_limite_diario,
            obter_modo_autonomo,
            configurar_modo_autonomo,
            registar_pausa_sessao,
            registar_retoma_sessao,
            obter_config_notif,
            configurar_notif,
            agregar_dados_feedback,
            gerar_feedback,
            listar_feedbacks,
            marcar_resultado_candidatura,
            sugerir_feedback,
            gerar_curriculo,
            listar_curriculos,
            abrir_curriculo,
            apagar_curriculo,
            gerar_cover_letter,
            listar_cover_letters,
            abrir_cover_letter,
            apagar_cover_letter,
            abrir_ficheiro_prompt,
            abrir_pasta_dados,
            obter_iniciar_com_sistema,
            configurar_iniciar_com_sistema,
            iniciar_busca_linkedin_rede,
            listar_vagas_linkedin_rede,
            obter_status_linkedin_rede,
            verificar_atualizacao,
            instalar_atualizacao,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
