#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Force linker to include workflow-nodes' inventory::submit!() statics.
extern crate workflow_nodes;

mod agent;
mod app_lifecycle;
mod app_setup;
mod app_tasks;
mod config;
mod constants;
mod hotload_sandbox;
mod llm;
mod project_root;
mod workflow;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    if let Err(error) = app_setup::run_app() {
        log::error!("Pantograph failed: {error}");
        std::process::exit(1);
    }
}
