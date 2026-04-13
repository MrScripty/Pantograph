use crate::state::RuntimeRegistryStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeObservation {
    pub runtime_id: String,
    pub display_name: String,
    pub backend_keys: Vec<String>,
    pub model_id: Option<String>,
    pub status: RuntimeRegistryStatus,
    pub runtime_instance_id: Option<String>,
    pub last_error: Option<String>,
}
