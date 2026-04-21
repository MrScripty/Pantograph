use std::path::PathBuf;

use pantograph_workflow_service::capabilities;

#[derive(Debug, Clone)]
pub struct EmbeddedRuntimeConfig {
    pub app_data_dir: PathBuf,
    pub project_root: PathBuf,
    pub workflow_roots: Vec<PathBuf>,
    pub max_loaded_sessions: Option<usize>,
}

impl EmbeddedRuntimeConfig {
    pub fn new(app_data_dir: PathBuf, project_root: PathBuf) -> Self {
        Self {
            app_data_dir,
            workflow_roots: capabilities::default_workflow_roots(&project_root),
            project_root,
            max_loaded_sessions: None,
        }
    }
}

#[cfg(feature = "standalone")]
#[derive(Debug, Clone)]
pub struct StandaloneRuntimeConfig {
    pub app_data_dir: PathBuf,
    pub project_root: PathBuf,
    pub workflow_roots: Vec<PathBuf>,
    pub max_loaded_sessions: Option<usize>,
    pub binaries_dir: PathBuf,
    pub pumas_library_path: Option<PathBuf>,
}

#[cfg(feature = "standalone")]
impl StandaloneRuntimeConfig {
    pub fn new(app_data_dir: PathBuf, project_root: PathBuf, binaries_dir: PathBuf) -> Self {
        Self {
            app_data_dir,
            workflow_roots: capabilities::default_workflow_roots(&project_root),
            project_root,
            max_loaded_sessions: None,
            binaries_dir,
            pumas_library_path: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EmbeddedRuntimeError {
    #[error("configuration error: {message}")]
    Config { message: String },

    #[error("runtime initialization error: {message}")]
    Initialization { message: String },
}
