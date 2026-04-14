#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Force linker to include workflow-nodes' inventory::submit!() statics
extern crate workflow_nodes;

mod agent;
mod config;
mod constants;
mod hotload_sandbox;
mod llm;
mod project_root;
mod workflow;

use agent::create_rag_manager;
use config::AppConfig;
use constants::paths::DATA_DIR;
use llm::{
    check_embedding_server, check_health_now, check_port_status, checkout_commit, clear_rag_cache,
    connect_to_server, create_vector_database, find_alternate_port, get_app_config,
    get_backend_capabilities, get_component_history, get_current_backend, get_current_commit_info,
    get_default_port, get_device_config, get_embedding_memory_mode,
    get_embedding_runtime_lifecycle_snapshot, get_embedding_server_url, get_health_status,
    get_llm_status, get_model_config, get_rag_status, get_recovery_attempt_count,
    get_recovery_config, get_redo_count, get_sandbox_config, get_svelte_docs_status,
    get_system_prompt, get_timeline_commits, hard_delete_commit, index_docs_with_switch,
    index_rag_documents, install_managed_runtime, is_embedding_server_ready,
    is_health_monitor_running, is_recovery_in_progress, list_backends, list_chunkable_docs,
    list_devices, list_generated_components, list_managed_runtimes, list_vector_databases,
    load_rag_from_disk, preview_doc_chunks, redo_component_change, remove_managed_runtime,
    reset_recovery_state, resolve_conflict, run_agent, search_rag, send_vision_prompt,
    set_app_config, set_device_config, set_embedding_memory_mode, set_embedding_server_url,
    set_model_config, set_sandbox_config, set_system_prompt, start_health_monitor,
    start_sidecar_embedding, start_sidecar_inference, start_sidecar_llm, stop_health_monitor,
    stop_llm, switch_backend, trigger_recovery, undo_component_change, update_svelte_docs,
    validate_component, InferenceGateway, RuntimeRegistry, SharedAppConfig, SharedGateway,
    SharedRuntimeRegistry,
};
use project_root::resolve_project_root;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::RwLock;
use workflow::{ExecutionManager, SharedModelDependencyResolver};

fn main() {
    // Initialize logging - shows logs in terminal when running in dev mode
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("Pantograph starting...");

    // Gateway is created in setup() where the AppHandle is available for ProcessSpawner

    // Create the execution manager for node-engine based workflows
    let execution_manager: workflow::SharedExecutionManager = Arc::new(ExecutionManager::new());
    let workflow_service: workflow::commands::SharedWorkflowService =
        Arc::new(pantograph_workflow_service::WorkflowService::new());
    let workflow_diagnostics_store: workflow::commands::SharedWorkflowDiagnosticsStore =
        Arc::new(workflow::WorkflowDiagnosticsStore::default());

    // Resolve the real repo root at runtime so saved workflows survive source tree moves.
    let project_root =
        resolve_project_root().expect("Failed to resolve Pantograph project root at runtime");
    let orchestrations_path = project_root.join(".pantograph/orchestrations");
    let workflow_graph_store: workflow::commands::SharedWorkflowGraphStore = Arc::new(
        pantograph_workflow_service::FileSystemWorkflowGraphStore::new(project_root.clone()),
    );

    let mut orchestration_store =
        node_engine::OrchestrationStore::with_persistence(&orchestrations_path);

    // Load existing orchestrations from disk
    match orchestration_store.load_from_disk() {
        Ok(count) => {
            if count > 0 {
                log::info!(
                    "Loaded {} orchestrations from {:?}",
                    count,
                    orchestrations_path
                );
            }
        }
        Err(e) => {
            log::warn!("Failed to load orchestrations from disk: {}", e);
        }
    }

    let orchestration_store: workflow::SharedOrchestrationStore =
        Arc::new(RwLock::new(orchestration_store));

    // Create the shared node-engine registry (includes port options providers via inventory)
    let node_registry: workflow::commands::SharedNodeRegistry =
        Arc::new(node_engine::NodeRegistry::with_builtins());
    let runtime_registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());

    // Create shared executor extensions (populated async in .setup())
    let shared_extensions: workflow::commands::SharedExtensions =
        Arc::new(RwLock::new(node_engine::ExecutorExtensions::new()));

    // Dependency resolver used by execution preflight and workflow dependency commands.
    let model_dependency_resolver: SharedModelDependencyResolver = Arc::new(
        workflow::model_dependencies::TauriModelDependencyResolver::new(
            shared_extensions.clone(),
            project_root.clone(),
        ),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(execution_manager)
        .manage(workflow_service.clone())
        .manage(workflow_diagnostics_store)
        .manage(workflow_graph_store)
        .manage(orchestration_store)
        .manage(node_registry)
        .manage(runtime_registry)
        .manage(shared_extensions.clone())
        .manage(model_dependency_resolver.clone())
        .setup({
            let shared_extensions = shared_extensions.clone();
            let model_dependency_resolver = model_dependency_resolver.clone();
            let workflow_service = workflow_service.clone();
            move |app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");

            // Clean up any lingering sidecar processes from previous runs
            if let Err(err) = inference::LlamaServer::cleanup_stale_sidecar(&app_data_dir) {
                log::warn!("Failed to clean up stale sidecar: {}", err);
            }

            // Create process spawner and inference gateway
            let spawner = llm::process_tauri::create_spawner(app.handle().clone());
            let gateway: SharedGateway = Arc::new(InferenceGateway::new(spawner));
            tauri::async_runtime::block_on(async { gateway.init().await });
            app.manage(gateway);

            let dependency_event_app = app.handle().clone();
            model_dependency_resolver.set_activity_emitter(Arc::new(move |event| {
                let _ = dependency_event_app.emit("dependency-activity", &event);
            }));

            let project_root =
                resolve_project_root().expect("Failed to resolve Pantograph project root");
            let project_data_dir = project_root.join(DATA_DIR);

            // Create the data directory if it doesn't exist
            if !project_data_dir.exists() {
                match std::fs::create_dir_all(&project_data_dir) {
                    Ok(()) => {
                        log::info!("Created project data directory: {:?}", project_data_dir);
                    }
                    Err(e) => {
                        log::error!("Failed to create data directory {:?}: {}. Some features may not work.", project_data_dir, e);
                    }
                }
            }

            // Initialize RAG manager with project data directory
            let rag_manager = create_rag_manager(project_data_dir);
            app.manage(rag_manager);

            let kv_cache_dir = app_data_dir.join("kv_cache");
            let config = tauri::async_runtime::block_on(async {
                match AppConfig::load(&app_data_dir).await {
                    Ok(config) => {
                        log::info!("Loaded app configuration");
                        config
                    }
                    Err(e) => {
                        log::warn!("Failed to load config, using defaults: {}", e);
                        AppConfig::default()
                    }
                }
            });
            workflow_service
                .set_loaded_runtime_capacity_limit(config.workflow.max_loaded_sessions)
                .expect("failed to apply workflow runtime config");
            let shared_config: SharedAppConfig = Arc::new(RwLock::new(config));
            app.manage(shared_config);

            // Initialize executor extensions (PumasApi etc.) asynchronously.
            // Prefer the sibling Pumas release build dir when available, then fall back
            // to the launcher root.
            let pumas_launcher_root = project_root
                .parent()
                .map(|parent| parent.join("Pumas-Library"))
                .filter(|p| p.exists());
            let pumas_release_dir = pumas_launcher_root
                .as_ref()
                .map(|root| root.join("rust").join("target").join("release"))
                .filter(|p| p.exists());
            if let Some(ref p) = pumas_release_dir {
                log::info!("Detected sibling Pumas release dir at {:?}", p);
            } else if let Some(ref p) = pumas_launcher_root {
                log::info!("Detected sibling Pumas-Library at {:?}", p);
            }
            let pumas_library_path = pumas_release_dir.or(pumas_launcher_root);

            // Register the dependency resolver synchronously to avoid startup races
            // where model execution can happen before async extension setup finishes.
            tauri::async_runtime::block_on(async {
                let resolver_trait: Arc<dyn node_engine::ModelDependencyResolver> =
                    model_dependency_resolver.clone();
                let mut ext = shared_extensions.write().await;
                ext.set(
                    node_engine::extension_keys::MODEL_DEPENDENCY_RESOLVER,
                    resolver_trait,
                );
            });

            let ext_init = shared_extensions.clone();
            tauri::async_runtime::spawn(async move {
                let mut ext = ext_init.write().await;
                workflow_nodes::setup_extensions_with_path(
                    &mut ext,
                    pumas_library_path.as_deref(),
                )
                .await;

                // Initialize KV cache store for cache save/load/truncate nodes
                let kv_store = std::sync::Arc::new(inference::kv_cache::KvCacheStore::new(
                    kv_cache_dir,
                    inference::kv_cache::StoragePolicy::MemoryAndDisk,
                ));
                ext.set(node_engine::extension_keys::KV_CACHE_STORE, kv_store);
                log::info!("Initialized KV cache store");
            });

            Ok(())
        }})
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
            list_vector_databases,
            create_vector_database,
            // Config commands
            get_model_config,
            set_model_config,
            get_app_config,
            set_app_config,
            get_device_config,
            set_device_config,
            list_devices,
            // Server mode commands
            start_sidecar_inference,
            start_sidecar_embedding,
            index_docs_with_switch,
            // Backend commands
            list_backends,
            get_current_backend,
            switch_backend,
            get_backend_capabilities,
            // Managed runtime commands
            list_managed_runtimes,
            install_managed_runtime,
            remove_managed_runtime,
            // Chunking preview commands
            list_chunkable_docs,
            preview_doc_chunks,
            // Embedding memory mode commands
            get_embedding_memory_mode,
            set_embedding_memory_mode,
            is_embedding_server_ready,
            get_embedding_server_url,
            get_embedding_runtime_lifecycle_snapshot,
            // Sandbox configuration commands
            get_sandbox_config,
            set_sandbox_config,
            validate_component,
            // System prompt commands
            get_system_prompt,
            set_system_prompt,
            // Version commands (undo/redo for generated components)
            undo_component_change,
            redo_component_change,
            get_component_history,
            get_redo_count,
            list_generated_components,
            // Timeline commands
            get_current_commit_info,
            get_timeline_commits,
            hard_delete_commit,
            checkout_commit,
            // Port management commands
            check_port_status,
            resolve_conflict,
            find_alternate_port,
            get_default_port,
            // Health monitoring commands
            start_health_monitor,
            stop_health_monitor,
            get_health_status,
            check_health_now,
            is_health_monitor_running,
            // Recovery commands
            get_recovery_config,
            is_recovery_in_progress,
            get_recovery_attempt_count,
            trigger_recovery,
            reset_recovery_state,
            // Workflow commands
            workflow::commands::validate_workflow_connection,
            workflow::commands::get_node_definitions,
            workflow::commands::get_node_definitions_by_category,
            workflow::commands::get_node_definition,
            // Workflow persistence commands
            workflow::commands::save_workflow,
            workflow::commands::load_workflow,
            workflow::commands::list_workflows,
            // Headless workflow API commands
            workflow::commands::workflow_run,
            workflow::commands::workflow_get_capabilities,
            workflow::commands::workflow_get_io,
            workflow::commands::workflow_preflight,
            workflow::commands::workflow_create_session,
            workflow::commands::workflow_run_session,
            workflow::commands::workflow_close_session,
            workflow::commands::workflow_get_session_status,
            workflow::commands::workflow_list_session_queue,
            workflow::commands::workflow_get_scheduler_snapshot,
            workflow::commands::workflow_get_diagnostics_snapshot,
            workflow::commands::workflow_get_trace_snapshot,
            workflow::commands::workflow_clear_diagnostics_history,
            workflow::commands::workflow_cancel_session_queue_item,
            workflow::commands::workflow_reprioritize_session_queue_item,
            workflow::commands::workflow_set_session_keep_alive,
            // Node-engine workflow commands (Phase 5)
            workflow::commands::execute_workflow_v2,
            workflow::commands::create_workflow_session,
            workflow::commands::run_workflow_session,
            workflow::commands::get_undo_redo_state,
            workflow::commands::undo_workflow,
            workflow::commands::redo_workflow,
            workflow::commands::update_node_data,
            workflow::commands::update_node_position_in_execution,
            workflow::commands::add_node_to_execution,
            workflow::commands::remove_node_from_execution,
            workflow::commands::add_edge_to_execution,
            workflow::commands::get_connection_candidates,
            workflow::commands::connect_anchors_in_execution,
            workflow::commands::insert_node_and_connect_in_execution,
            workflow::commands::preview_node_insert_on_edge_in_execution,
            workflow::commands::insert_node_on_edge_in_execution,
            workflow::commands::remove_edge_from_execution,
            workflow::commands::get_execution_graph,
            workflow::commands::remove_execution,
            // Port options query commands
            workflow::commands::query_port_options,
            workflow::commands::get_queryable_ports,
            workflow::commands::list_models_needing_review,
            workflow::commands::submit_model_review,
            workflow::commands::reset_model_review,
            workflow::commands::get_effective_model_metadata,
            workflow::commands::hydrate_puma_lib_node,
            workflow::commands::run_dependency_environment_action,
            workflow::commands::resolve_model_dependency_requirements,
            workflow::commands::check_model_dependencies,
            workflow::commands::install_model_dependencies,
            workflow::commands::get_model_dependency_status,
            workflow::commands::audit_dependency_pin_compliance,
            // Node group commands
            workflow::groups::create_node_group,
            workflow::groups::expand_node_group,
            workflow::groups::collapse_node_group,
            workflow::groups::update_group_ports,
            workflow::groups::rename_node_group,
            workflow::groups::ungroup_nodes,
            // Orchestration commands
            workflow::orchestration::create_orchestration,
            workflow::orchestration::get_orchestration,
            workflow::orchestration::list_orchestrations,
            workflow::orchestration::save_orchestration,
            workflow::orchestration::delete_orchestration,
            workflow::orchestration::add_orchestration_node,
            workflow::orchestration::remove_orchestration_node,
            workflow::orchestration::add_orchestration_edge,
            workflow::orchestration::remove_orchestration_edge,
            workflow::orchestration::update_orchestration_node,
            workflow::orchestration::update_orchestration_node_position,
            workflow::orchestration::set_orchestration_data_graph,
            workflow::orchestration::register_data_graph,
            workflow::orchestration::execute_orchestration,
            workflow::orchestration::get_orchestration_node_types,
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
