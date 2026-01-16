#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent;
mod config;
mod constants;
mod hotload_sandbox;
mod llm;

use agent::create_rag_manager;
use config::AppConfig;
use constants::paths::DATA_DIR;
use llm::{
    check_embedding_server, check_llama_binaries, check_ollama_binary, clear_rag_cache,
    connect_to_server, download_llama_binaries, download_ollama_binary, get_app_config,
    get_backend_capabilities, get_current_backend, get_device_config, get_embedding_memory_mode,
    get_embedding_server_url, get_llm_status, get_model_config, get_rag_status, get_server_mode,
    get_svelte_docs_status, index_docs_with_switch, index_rag_documents, is_embedding_server_ready,
    list_backends, list_chunkable_docs, list_devices, load_rag_from_disk, preview_doc_chunks,
    run_agent, search_rag, send_vision_prompt, set_app_config, set_device_config,
    set_embedding_memory_mode, set_embedding_server_url, set_model_config, start_sidecar_embedding,
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

            // Get project data directory for docs and RAG storage
            // Use CARGO_MANIFEST_DIR (src-tauri/) and go up one level to get project root.
            // This ensures data is stored at project root regardless of the current working
            // directory (which can vary during `tauri dev`).
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            let project_root = std::path::Path::new(manifest_dir)
                .parent()
                .expect("Failed to get project root from CARGO_MANIFEST_DIR");
            let project_data_dir = project_root.join(DATA_DIR);

            // Create the data directory if it doesn't exist
            if !project_data_dir.exists() {
                std::fs::create_dir_all(&project_data_dir)
                    .expect("Failed to create data directory");
                log::info!("Created project data directory: {:?}", project_data_dir);
            }

            // Initialize RAG manager with project data directory
            let rag_manager = create_rag_manager(project_data_dir);
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
            check_ollama_binary,
            download_ollama_binary,
            // Chunking preview commands
            list_chunkable_docs,
            preview_doc_chunks,
            // Embedding memory mode commands
            get_embedding_memory_mode,
            set_embedding_memory_mode,
            is_embedding_server_ready,
            get_embedding_server_url,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Stop the inference gateway when the window closes to avoid lingering processes
                let app = window.app_handle();
                if let Some(gateway) = app.try_state::<SharedGateway>() {
                    tauri::async_runtime::block_on(async {
                        // Stop both main server and embedding server
                        gateway.stop_all().await;
                        log::info!("Stopped inference gateway and embedding server on window close");
                    });
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
