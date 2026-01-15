#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent;
mod config;
mod constants;
mod llm;

use agent::create_rag_manager;
use config::AppConfig;
use llm::{
    check_embedding_server, check_llama_binaries, clear_rag_cache, connect_to_server,
    download_llama_binaries, get_app_config, get_backend_capabilities, get_current_backend,
    get_device_config, get_llm_status, get_model_config, get_rag_status, get_server_mode,
    get_svelte_docs_status, index_docs_with_switch, index_rag_documents, list_backends,
    list_devices, load_rag_from_disk, run_agent, search_rag, send_vision_prompt, set_app_config,
    set_device_config, set_embedding_server_url, set_model_config, start_sidecar_embedding,
    start_sidecar_inference, start_sidecar_llm, stop_llm, switch_backend, update_svelte_docs,
    InferenceGateway, LlamaServer, SharedAppConfig, SharedGateway,
};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

fn main() {
    // Initialize logging - shows logs in terminal when running in dev mode
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("Pantograph starting...");

    // Create the inference gateway - single entry point for all inference operations
    let gateway: SharedGateway = Arc::new(InferenceGateway::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(gateway)
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");

            if let Err(err) = LlamaServer::cleanup_stale_sidecar(&app_data_dir) {
                log::warn!("Failed to clean up stale sidecar: {}", err);
            }

            // Initialize RAG manager
            let rag_manager = create_rag_manager(app_data_dir.clone());
            app.manage(rag_manager);

            // Load app configuration
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let config = match AppConfig::load(&app_data_dir).await {
                    Ok(config) => {
                        log::info!("Loaded app configuration");
                        config
                    }
                    Err(e) => {
                        log::warn!("Failed to load config, using defaults: {}", e);
                        AppConfig::default()
                    }
                };

                let shared_config: SharedAppConfig = Arc::new(RwLock::new(config));
                app_handle.manage(shared_config);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // LLM commands
            send_vision_prompt,
            connect_to_server,
            start_sidecar_llm,
            get_llm_status,
            stop_llm,
            run_agent,
            // Docs commands
            get_svelte_docs_status,
            update_svelte_docs,
            // RAG commands
            get_rag_status,
            check_embedding_server,
            set_embedding_server_url,
            index_rag_documents,
            load_rag_from_disk,
            clear_rag_cache,
            search_rag,
            // Config commands
            get_model_config,
            set_model_config,
            get_app_config,
            set_app_config,
            get_device_config,
            set_device_config,
            list_devices,
            // Server mode commands
            get_server_mode,
            start_sidecar_inference,
            start_sidecar_embedding,
            index_docs_with_switch,
            // Backend commands
            list_backends,
            get_current_backend,
            switch_backend,
            get_backend_capabilities,
            // Binary download commands
            check_llama_binaries,
            download_llama_binaries,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Stop the inference gateway when the window closes to avoid lingering processes
                let app = window.app_handle();
                if let Some(gateway) = app.try_state::<SharedGateway>() {
                    tauri::async_runtime::block_on(async {
                        gateway.stop().await;
                        log::info!("Stopped inference gateway on window close");
                    });
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
