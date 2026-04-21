use std::sync::{Arc, Mutex};

use tauri::async_runtime::JoinHandle;

pub type SharedAppTaskRegistry = Arc<AppTaskRegistry>;

pub struct AppTaskRegistry {
    tasks: Mutex<Vec<OwnedAppTask>>,
}

struct OwnedAppTask {
    name: &'static str,
    handle: JoinHandle<()>,
}

impl AppTaskRegistry {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
        }
    }

    pub fn track(&self, name: &'static str, handle: JoinHandle<()>) {
        match self.tasks.lock() {
            Ok(mut tasks) => {
                tasks.push(OwnedAppTask { name, handle });
            }
            Err(error) => {
                log::error!("Failed to track app task '{name}': {error}");
                handle.abort();
            }
        }
    }

    pub async fn shutdown(&self) {
        let tasks = match self.tasks.lock() {
            Ok(mut tasks) => std::mem::take(&mut *tasks),
            Err(error) => {
                log::error!("Failed to acquire app task registry for shutdown: {error}");
                return;
            }
        };

        for task in tasks {
            task.handle.abort();
            match task.handle.await {
                Ok(()) => {
                    log::debug!("App task '{}' finished before shutdown", task.name);
                }
                Err(error) => {
                    log::debug!("App task '{}' stopped during shutdown: {error}", task.name);
                }
            }
        }
    }
}
