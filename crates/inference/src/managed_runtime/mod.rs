pub mod llama_cpp_platform;
pub mod ollama_platform;

mod archive;
mod catalog;
mod contracts;
mod definitions;
mod operations;
mod paths;
mod state;

pub use contracts::{
    BinaryStatus, DownloadProgress, ManagedBinaryCapability, ManagedBinaryId,
    ManagedBinaryInstallState, ManagedRuntimeCatalogVersion, ManagedRuntimeJobArtifactStatus,
    ManagedRuntimeJobState, ManagedRuntimeJobStatus, ManagedRuntimeReadinessState,
    ManagedRuntimeSelectionState, ManagedRuntimeSnapshot, ManagedRuntimeVersionStatus,
    ResolvedCommand,
};
pub use operations::{
    binary_capability, cancel_binary_download, check_binary_status, download_binary,
    list_binary_capabilities, list_managed_runtime_snapshots, managed_runtime_snapshot,
    pause_binary_download, refresh_managed_runtime_catalog, refresh_managed_runtime_catalogs,
    remove_binary, resolve_binary_command, select_managed_runtime_version,
    set_default_managed_runtime_version,
};
pub use paths::managed_runtime_dir;
pub use state::{
    load_managed_runtime_state, save_managed_runtime_state, ManagedRuntimeHistoryEventKind,
    ManagedRuntimeInstallHistoryEntry, ManagedRuntimePersistedJobArtifact,
    ManagedRuntimePersistedRuntime, ManagedRuntimePersistedState, ManagedRuntimePersistedVersion,
};

pub(crate) use contracts::{ArchiveKind, ReleaseAsset};
pub(crate) use paths::{extract_pid_file, prepend_env_path};
