use std::path::Path;

use super::{find_executable, OllamaPlatform, OLLAMA_RELEASE_TAG};
use crate::managed_runtime::{extract_pid_file, prepend_env_path, ArchiveKind, ReleaseAsset, ResolvedCommand};

pub(crate) struct LinuxPlatform;

pub(crate) static PLATFORM: LinuxPlatform = LinuxPlatform;

impl OllamaPlatform for LinuxPlatform {
    fn release_asset(&self) -> ReleaseAsset {
        ReleaseAsset {
            archive_name: format!("ollama-linux-amd64.tar.zst"),
            archive_kind: ArchiveKind::TarZst,
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
        let (args, pid_file) = extract_pid_file(args);

        Ok(ResolvedCommand {
            executable_path,
            working_directory: working_directory.clone(),
            args,
            env_overrides: vec![prepend_env_path(
                "LD_LIBRARY_PATH",
                &working_directory,
                ":",
            )],
            pid_file,
        })
    }
}
