//! Desktop projection of the production application-configuration owner.

pub use pantograph_app_config::{
    AppConfig, DeviceConfig, DeviceInfo, EmbeddingMemoryMode, ImportValidationMode, ModelConfig,
    SandboxConfig,
};

/// Backend-owned runtime status contract exposed to the GUI.
pub type ServerModeInfo = inference::ServerModeInfo;
