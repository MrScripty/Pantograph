use tauri::{AppHandle, Manager};

use super::commands::SharedWorkflowService;

pub fn invalidate_loaded_session_runtimes(app: &AppHandle) {
    let Some(workflow_service) = app.try_state::<SharedWorkflowService>() else {
        return;
    };

    match workflow_service.invalidate_all_session_runtimes() {
        Ok(invalidated) if !invalidated.is_empty() => {
            log::info!(
                "invalidated {} loaded workflow execution session runtime(s) before producer stop",
                invalidated.len()
            );
        }
        Ok(_) => {}
        Err(error) => {
            log::warn!(
                "failed to invalidate loaded workflow execution session runtimes before producer stop: {}",
                error
            );
        }
    }
}
