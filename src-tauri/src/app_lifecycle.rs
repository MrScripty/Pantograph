use tauri::{Manager, Window, WindowEvent};

use crate::app_tasks::SharedAppTaskRegistry;
use crate::llm::runtime_registry::stop_all_and_sync_runtime_registry;
use crate::llm::{
    SharedGateway, SharedHealthMonitor, SharedRecoveryManager, SharedRuntimeRegistry,
};
use crate::workflow;
use crate::workflow::runtime_shutdown::invalidate_loaded_session_runtimes;

pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { .. } = event {
        shutdown_window_runtime(window);
    }
}

fn shutdown_window_runtime(window: &Window) {
    let app = window.app_handle();
    let gateway = app
        .try_state::<SharedGateway>()
        .map(|state| state.inner().clone());
    let workflow_session_cleanup_worker = app
        .try_state::<workflow::commands::SharedWorkflowSessionStaleCleanupWorker>()
        .map(|state| state.inner().clone());
    let app_task_registry = app
        .try_state::<SharedAppTaskRegistry>()
        .map(|state| state.inner().clone());
    let health_monitor = app
        .try_state::<SharedHealthMonitor>()
        .map(|state| state.inner().clone());
    let recovery_manager = app
        .try_state::<SharedRecoveryManager>()
        .map(|state| state.inner().clone());
    let runtime_registry = app
        .try_state::<SharedRuntimeRegistry>()
        .map(|state| state.inner().clone());

    tauri::async_runtime::block_on(async {
        if let Some(app_task_registry) = app_task_registry {
            app_task_registry.shutdown().await;
        }

        if let Some(health_monitor) = health_monitor {
            health_monitor.stop();
        }

        if let Some(recovery_manager) = recovery_manager {
            recovery_manager.stop_auto_recovery_task();
        }

        if let Some(workflow_session_cleanup_worker) = workflow_session_cleanup_worker {
            workflow_session_cleanup_worker.shutdown().await;
        }

        if let Some(gateway) = gateway {
            invalidate_loaded_session_runtimes(&app);
            if let Some(runtime_registry) = runtime_registry {
                stop_all_and_sync_runtime_registry(gateway.as_ref(), runtime_registry.as_ref())
                    .await;
            } else {
                gateway.stop_all().await;
            }
            log::info!("Stopped inference gateway and embedding server on window close");
        }
    });
}
