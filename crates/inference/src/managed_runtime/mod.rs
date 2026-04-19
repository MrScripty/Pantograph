pub mod llama_cpp_platform;
pub mod ollama_platform;

mod archive;
mod contracts;
mod definitions;
mod operations;
mod paths;

pub use contracts::{
    BinaryStatus, DownloadProgress, ManagedBinaryCapability, ManagedBinaryId,
    ManagedBinaryInstallState, ResolvedCommand,
};
pub use operations::{
    binary_capability, check_binary_status, download_binary, list_binary_capabilities,
    remove_binary, resolve_binary_command,
};
pub use paths::managed_runtime_dir;

pub(crate) use contracts::{ArchiveKind, ReleaseAsset};
pub(crate) use paths::{extract_pid_file, prepend_env_path};
