use crate::agent::create_rag_manager;
use crate::app_tasks::{AppTaskRegistry, SharedAppTaskRegistry};
use crate::config::AppConfig;
use crate::constants::paths::DATA_DIR;
use crate::llm::{
    self, InferenceGateway, RuntimeRegistry, SharedAppConfig, SharedGateway, SharedHealthMonitor,
    SharedRecoveryManager, SharedRuntimeRegistry,
};
use crate::project_root::resolve_project_root;
use crate::workflow::{self, SharedModelDependencyResolver};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::RwLock;

type AppStartupResult<T> = Result<T, Box<dyn std::error::Error>>;

fn startup_error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::other(message.into()))
}

pub fn run_app() -> AppStartupResult<()> {
    log::info!("Pantograph starting...");

    // Gateway is created in setup() where the AppHandle is available for ProcessSpawner

    // Resolve the real repo root at runtime so saved workflows survive source tree moves.
    let project_root = resolve_project_root().map_err(|error| {
        startup_error(format!(
            "failed to resolve Pantograph project root at runtime: {error}"
        ))
    })?;
    let pantograph_data_dir = project_root.join(".pantograph");
    std::fs::create_dir_all(&pantograph_data_dir).map_err(|error| {
        startup_error(format!(
            "failed to create Pantograph data directory {:?}: {error}",
            pantograph_data_dir
        ))
    })?;
    let workflow_timing_ledger_path = pantograph_data_dir.join("workflow-diagnostics.sqlite");
    let workflow_service_ledger =
        pantograph_workflow_service::SqliteDiagnosticsLedger::open(&workflow_timing_ledger_path)
            .map_err(|error| {
                startup_error(format!(
                    "failed to open workflow service diagnostics ledger {:?}: {error}",
                    workflow_timing_ledger_path
                ))
            })?;
    let workflow_service: workflow::commands::SharedWorkflowService = Arc::new(
        pantograph_workflow_service::WorkflowService::new()
            .with_diagnostics_ledger(workflow_service_ledger),
    );
    let workflow_timing_ledger =
        pantograph_workflow_service::SqliteDiagnosticsLedger::open(&workflow_timing_ledger_path)
            .map_err(|error| {
                startup_error(format!(
                    "failed to open workflow diagnostics ledger {:?}: {error}",
                    workflow_timing_ledger_path
                ))
            })?;
    let workflow_diagnostics_store: workflow::commands::SharedWorkflowDiagnosticsStore = Arc::new(
        workflow::WorkflowDiagnosticsStore::with_default_timing_ledger(workflow_timing_ledger),
    );
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
    let health_monitor: SharedHealthMonitor =
        Arc::new(llm::health_monitor::HealthMonitor::default());
    let recovery_manager: SharedRecoveryManager =
        Arc::new(llm::recovery::RecoveryManager::default());
    let app_task_registry: SharedAppTaskRegistry = Arc::new(AppTaskRegistry::new());

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
        .manage(workflow_service.clone())
        .manage(workflow_diagnostics_store)
        .manage(workflow_graph_store)
        .manage(orchestration_store)
        .manage(node_registry)
        .manage(runtime_registry)
        .manage(health_monitor)
        .manage(recovery_manager)
        .manage(app_task_registry.clone())
        .manage(shared_extensions.clone())
        .manage(model_dependency_resolver.clone())
        .setup({
            let shared_extensions = shared_extensions.clone();
            let model_dependency_resolver = model_dependency_resolver.clone();
            let workflow_service = workflow_service.clone();
            let app_task_registry = app_task_registry.clone();
            move |app| {
                let workflow_service_for_cleanup = workflow_service.clone();
                let runtime_handle = tauri::async_runtime::handle().inner().clone();
                let workflow_execution_session_cleanup_worker =
                    workflow_service_for_cleanup
                        .clone()
                        .spawn_workflow_execution_session_stale_cleanup_worker_with_handle(
                            pantograph_workflow_service::WorkflowExecutionSessionStaleCleanupWorkerConfig::default(
                            ),
                            runtime_handle,
                        )
                        .map_err(|error| {
                            startup_error(format!(
                                "failed to start workflow execution session stale cleanup worker: {error}"
                            ))
                        })?;
                let workflow_execution_session_cleanup_worker: workflow::commands::SharedWorkflowExecutionSessionStaleCleanupWorker =
                    Arc::new(workflow_execution_session_cleanup_worker);
                app.manage(workflow_execution_session_cleanup_worker);

                let app_data_dir = app.path().app_data_dir().map_err(|error| {
                    startup_error(format!("failed to resolve app data dir: {error}"))
                })?;

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

                let project_root = resolve_project_root().map_err(|error| {
                    startup_error(format!(
                        "failed to resolve Pantograph project root during setup: {error}"
                    ))
                })?;
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
                    .map_err(|error| {
                        startup_error(format!("failed to apply workflow runtime config: {error}"))
                    })?;
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
                let extension_init_task = tauri::async_runtime::spawn(async move {
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
                app_task_registry.track("executor-extension-init", extension_init_task);

                Ok(())
            }
        })
        .invoke_handler(tauri::generate_handler![
            // LLM commands
            crate::llm::send_vision_prompt,
            crate::llm::connect_to_server,
            crate::llm::start_sidecar_llm,
            crate::llm::get_llm_status,
            crate::llm::stop_llm,
            crate::llm::run_agent,
            // Docs commands
            crate::llm::get_svelte_docs_status,
            crate::llm::update_svelte_docs,
            // RAG commands
            crate::llm::get_rag_status,
            crate::llm::check_embedding_server,
            crate::llm::set_embedding_server_url,
            crate::llm::index_rag_documents,
            crate::llm::load_rag_from_disk,
            crate::llm::clear_rag_cache,
            crate::llm::search_rag,
            crate::llm::list_vector_databases,
            crate::llm::create_vector_database,
            // Config commands
            crate::llm::get_model_config,
            crate::llm::set_model_config,
            crate::llm::get_app_config,
            crate::llm::set_app_config,
            crate::llm::get_device_config,
            crate::llm::set_device_config,
            crate::llm::list_devices,
            // Server mode commands
            crate::llm::start_sidecar_inference,
            crate::llm::start_sidecar_embedding,
            crate::llm::index_docs_with_switch,
            // Backend commands
            crate::llm::list_backends,
            crate::llm::get_current_backend,
            crate::llm::switch_backend,
            crate::llm::get_backend_capabilities,
            crate::llm::get_runtime_registry_snapshot,
            crate::llm::get_runtime_debug_snapshot,
            crate::llm::reclaim_runtime_registry_runtime,
            // Managed runtime commands
            crate::llm::list_managed_runtimes,
            crate::llm::inspect_managed_runtime,
            crate::llm::refresh_managed_runtime_catalogs,
            crate::llm::install_managed_runtime,
            crate::llm::pause_managed_runtime_job,
            crate::llm::cancel_managed_runtime_job,
            crate::llm::remove_managed_runtime,
            crate::llm::select_managed_runtime_version,
            crate::llm::set_default_managed_runtime_version,
            // Chunking preview commands
            crate::llm::list_chunkable_docs,
            crate::llm::preview_doc_chunks,
            // Embedding memory mode commands
            crate::llm::get_embedding_memory_mode,
            crate::llm::set_embedding_memory_mode,
            crate::llm::is_embedding_server_ready,
            crate::llm::get_embedding_server_url,
            crate::llm::get_embedding_runtime_lifecycle_snapshot,
            // Sandbox configuration commands
            crate::llm::get_sandbox_config,
            crate::llm::set_sandbox_config,
            crate::llm::validate_component,
            // System prompt commands
            crate::llm::get_system_prompt,
            crate::llm::set_system_prompt,
            // Version commands (undo/redo for generated components)
            crate::llm::undo_component_change,
            crate::llm::redo_component_change,
            crate::llm::get_component_history,
            crate::llm::get_redo_count,
            crate::llm::list_generated_components,
            // Timeline commands
            crate::llm::get_current_commit_info,
            crate::llm::get_timeline_commits,
            crate::llm::hard_delete_commit,
            crate::llm::checkout_commit,
            // Port management commands
            crate::llm::check_port_status,
            crate::llm::resolve_conflict,
            crate::llm::find_alternate_port,
            crate::llm::get_default_port,
            // Health monitoring commands
            crate::llm::start_health_monitor,
            crate::llm::stop_health_monitor,
            crate::llm::get_health_status,
            crate::llm::check_health_now,
            crate::llm::is_health_monitor_running,
            // Recovery commands
            crate::llm::get_recovery_config,
            crate::llm::is_recovery_in_progress,
            crate::llm::get_recovery_attempt_count,
            crate::llm::trigger_recovery,
            crate::llm::reset_recovery_state,
            // Workflow commands
            crate::workflow::commands::validate_workflow_connection,
            crate::workflow::commands::get_node_definitions,
            crate::workflow::commands::get_node_definitions_by_category,
            crate::workflow::commands::get_node_definition,
            // Workflow persistence commands
            crate::workflow::commands::save_workflow,
            crate::workflow::commands::load_workflow,
            crate::workflow::commands::list_workflows,
            crate::workflow::workflow_persistence_commands::delete_workflow,
            // Headless workflow API commands
            crate::workflow::commands::workflow_get_capabilities,
            crate::workflow::commands::workflow_get_io,
            crate::workflow::commands::workflow_preflight,
            crate::workflow::commands::workflow_create_execution_session,
            crate::workflow::commands::workflow_run_execution_session,
            crate::workflow::commands::workflow_close_execution_session,
            crate::workflow::commands::workflow_get_execution_session_status,
            crate::workflow::commands::workflow_list_execution_session_queue,
            crate::workflow::commands::workflow_cleanup_stale_execution_sessions,
            crate::workflow::commands::workflow_get_scheduler_snapshot,
            crate::workflow::commands::workflow_scheduler_timeline_query,
            crate::workflow::commands::workflow_run_list_query,
            crate::workflow::commands::workflow_run_detail_query,
            crate::workflow::commands::workflow_get_diagnostics_snapshot,
            crate::workflow::commands::workflow_get_trace_snapshot,
            crate::workflow::commands::workflow_clear_diagnostics_history,
            crate::workflow::commands::workflow_cancel_execution_session_queue_item,
            crate::workflow::commands::workflow_reprioritize_execution_session_queue_item,
            crate::workflow::commands::workflow_set_execution_session_keep_alive,
            // Node-engine workflow session commands
            crate::workflow::workflow_execution_tauri_commands::create_workflow_execution_session,
            crate::workflow::workflow_execution_tauri_commands::run_workflow_execution_session,
            crate::workflow::workflow_execution_tauri_commands::get_undo_redo_state,
            crate::workflow::workflow_execution_tauri_commands::undo_workflow,
            crate::workflow::workflow_execution_tauri_commands::redo_workflow,
            crate::workflow::workflow_execution_tauri_commands::update_node_data,
            crate::workflow::workflow_execution_tauri_commands::update_node_position_in_execution,
            crate::workflow::workflow_execution_tauri_commands::add_node_to_execution,
            crate::workflow::workflow_execution_tauri_commands::remove_node_from_execution,
            crate::workflow::workflow_execution_tauri_commands::delete_selection_from_execution,
            crate::workflow::workflow_execution_tauri_commands::add_edge_to_execution,
            crate::workflow::workflow_execution_tauri_commands::get_connection_candidates,
            crate::workflow::workflow_execution_tauri_commands::connect_anchors_in_execution,
            crate::workflow::workflow_execution_tauri_commands::insert_node_and_connect_in_execution,
            crate::workflow::workflow_execution_tauri_commands::preview_node_insert_on_edge_in_execution,
            crate::workflow::workflow_execution_tauri_commands::insert_node_on_edge_in_execution,
            crate::workflow::workflow_execution_tauri_commands::remove_edge_from_execution,
            crate::workflow::workflow_execution_tauri_commands::remove_edges_from_execution,
            crate::workflow::workflow_execution_tauri_commands::create_group_in_execution,
            crate::workflow::workflow_execution_tauri_commands::ungroup_in_execution,
            crate::workflow::workflow_execution_tauri_commands::update_group_ports_in_execution,
            crate::workflow::workflow_execution_tauri_commands::get_execution_graph,
            crate::workflow::workflow_execution_tauri_commands::remove_execution,
            // Port options query commands
            crate::workflow::commands::query_port_options,
            crate::workflow::commands::get_queryable_ports,
            crate::workflow::commands::list_models_needing_review,
            crate::workflow::commands::submit_model_review,
            crate::workflow::commands::reset_model_review,
            crate::workflow::commands::get_effective_model_metadata,
            crate::workflow::commands::hydrate_puma_lib_node,
            crate::workflow::commands::run_dependency_environment_action,
            crate::workflow::commands::resolve_model_dependency_requirements,
            crate::workflow::commands::check_model_dependencies,
            crate::workflow::commands::install_model_dependencies,
            crate::workflow::commands::get_model_dependency_status,
            crate::workflow::commands::audit_dependency_pin_compliance,
            // Node group commands
            crate::workflow::groups::create_node_group,
            crate::workflow::groups::expand_node_group,
            crate::workflow::groups::collapse_node_group,
            crate::workflow::groups::update_group_ports,
            crate::workflow::groups::rename_node_group,
            crate::workflow::groups::ungroup_nodes,
            // Orchestration commands
            crate::workflow::orchestration::create_orchestration,
            crate::workflow::orchestration::get_orchestration,
            crate::workflow::orchestration::list_orchestrations,
            crate::workflow::orchestration::save_orchestration,
            crate::workflow::orchestration::delete_orchestration,
            crate::workflow::orchestration::add_orchestration_node,
            crate::workflow::orchestration::remove_orchestration_node,
            crate::workflow::orchestration::add_orchestration_edge,
            crate::workflow::orchestration::remove_orchestration_edge,
            crate::workflow::orchestration::update_orchestration_node,
            crate::workflow::orchestration::update_orchestration_node_position,
            crate::workflow::orchestration::set_orchestration_data_graph,
            crate::workflow::orchestration::register_data_graph,
            crate::workflow::orchestration::execute_orchestration,
            crate::workflow::orchestration::get_orchestration_node_types,
        ])
        .on_window_event(crate::app_lifecycle::handle_window_event)
        .run(tauri::generate_context!())
        .map_err(|error| startup_error(format!("error while running tauri application: {error}")))
}
