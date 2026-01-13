#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod llm;

use llm::{connect_to_server, get_llm_status, send_vision_prompt, start_sidecar_llm, stop_llm, LlamaServer};
use std::sync::Arc;
use tokio::sync::RwLock;

fn main() {
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
