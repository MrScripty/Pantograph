use std::path::Path;

use super::{find_executable, OllamaPlatform, OLLAMA_RELEASE_TAG};
use crate::llm::managed_binaries::{prepend_env_path, ArchiveKind, ReleaseAsset, ResolvedCommand};

pub(crate) struct MacOsX64Platform;

pub(crate) static PLATFORM: MacOsX64Platform = MacOsX64Platform;

impl OllamaPlatform for MacOsX64Platform {
    fn release_asset(&self) -> ReleaseAsset {
        ReleaseAsset {
            archive_name: "Ollama-darwin.zip".to_string(),
            archive_kind: ArchiveKind::Zip,
        }
    }

    fn executable_name(&self) -> &'static str {
        "ollama"
    }

    fn validate_installation(&self, install_dir: &Path) -> Vec<String> {
        if find_executable(install_dir, self.executable_name()).is_some() {
            Vec::new()
        } else {
            vec![self.executable_name().to_string()]
        }
    }

    fn resolve_command(&self, install_dir: &Path, args: &[&str]) -> Result<ResolvedCommand, String> {
        let executable_path = find_executable(install_dir, self.executable_name()).ok_or_else(|| {
            format!(
                "Managed Ollama binary not found under {} for release {}",
                install_dir.display(),
                OLLAMA_RELEASE_TAG
            )
        })?;
        let working_directory = executable_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| install_dir.to_path_buf());
        let (args, pid_file) = crate::llm::managed_binaries::extract_pid_file(args);

        Ok(ResolvedCommand {
            executable_path,
            working_directory: working_directory.clone(),
            args,
            env_overrides: vec![prepend_env_path(
                "DYLD_LIBRARY_PATH",
                &working_directory,
                ":",
            )],
            pid_file,
        })
    }
}
