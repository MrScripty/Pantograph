#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent;
mod llm;

use llm::{connect_to_server, get_llm_status, run_agent, send_vision_prompt, start_sidecar_llm, stop_llm, LlamaServer};
use std::sync::Arc;
use tokio::sync::RwLock;

fn main() {
    // Initialize logging - shows logs in terminal when running in dev mode
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("Pantograph starting...");

    let llama_server = Arc::new(RwLock::new(LlamaServer::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(llama_server)
        .invoke_handler(tauri::generate_handler![
            send_vision_prompt,
            connect_to_server,
            start_sidecar_llm,
            get_llm_status,
            stop_llm,
            run_agent,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
