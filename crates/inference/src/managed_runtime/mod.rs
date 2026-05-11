pub mod llama_cpp_platform;

mod archive;
mod catalog;
mod contracts;
mod definitions;
mod neutral_contracts;
mod operations;
mod paths;
mod state;

pub use contracts::{
    BinaryStatus, DownloadProgress, ManagedBinaryCapability, ManagedBinaryId,
    ManagedBinaryInstallState, ManagedRuntimeCatalogVersion, ManagedRuntimeCommandResolutionError,
    ManagedRuntimeJobArtifactStatus, ManagedRuntimeJobState, ManagedRuntimeJobStatus,
    ManagedRuntimeReadinessState, ManagedRuntimeSelectionState, ManagedRuntimeSnapshot,
    ManagedRuntimeVersionStatus, ResolvedCommand,
};
pub use neutral_contracts::{
    list_managed_runtime_dependency_statuses, managed_runtime_dependency_status,
    resolve_runtime_sidecar_dependency_command,
};
pub use operations::{
    binary_capability, cancel_binary_download, check_binary_status, download_binary,
    list_binary_capabilities, list_managed_runtime_snapshots, managed_runtime_snapshot,
    pause_binary_download, refresh_managed_runtime_catalog, refresh_managed_runtime_catalogs,
    remove_binary, remove_binary_version, resolve_binary_command, select_managed_runtime_version,
    set_default_managed_runtime_version,
};
pub use paths::managed_runtime_dir;
pub use state::{
    load_managed_runtime_state, reconcile_interrupted_managed_runtime_jobs,
    save_managed_runtime_state, ManagedRuntimeHistoryEventKind, ManagedRuntimeInstallHistoryEntry,
    ManagedRuntimePersistedJobArtifact, ManagedRuntimePersistedRuntime,
    ManagedRuntimePersistedState, ManagedRuntimePersistedVersion,
};

pub(crate) use contracts::{ArchiveKind, ReleaseAsset};
pub(crate) use paths::{extract_pid_file, prepend_env_path};
