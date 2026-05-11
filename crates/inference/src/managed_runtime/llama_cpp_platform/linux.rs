use std::path::Path;

use super::{
    ensure_unix_library_aliases, extract_pid_file, find_option_value, prepend_env_path,
    ArchiveKind, LlamaPlatform, LlamaRuntimeVariant, ManagedRuntimeCommandResolutionError,
    ReleaseAsset, ResolvedCommand, LLAMA_CPU_VARIANT, LLAMA_CUDA_VARIANT,
};

pub(crate) struct LinuxPlatform;

pub(crate) static PLATFORM: LinuxPlatform = LinuxPlatform;

impl LlamaPlatform for LinuxPlatform {
    fn release_asset(&self, version: &str) -> ReleaseAsset {
        ReleaseAsset {
            archive_name: format!("llama-{}-bin-ubuntu-x64.tar.gz", version),
            archive_kind: ArchiveKind::TarGz,
        }
    }

    fn catalog_runtime_variants(&self) -> &'static [LlamaRuntimeVariant] {
        &[LLAMA_CPU_VARIANT, LLAMA_CUDA_VARIANT]
    }

    fn installed_server_name(&self) -> &'static str {
        "llama-server-x86_64-unknown-linux-gnu"
    }

    fn validate_installation(&self, binaries_dir: &Path) -> Vec<String> {
        let mut missing = Vec::new();
        if !binaries_dir.join(self.installed_server_name()).exists() {
            missing.push(self.installed_server_name().to_string());
        }
        if !binaries_dir.join("libllama.so").exists() {
            missing.push("libllama.so".to_string());
        }
        if !binaries_dir.join("libggml.so").exists() {
            missing.push("libggml.so".to_string());
        }
        missing
    }

    fn resolve_command(
        &self,
        binaries_dir: &Path,
        args: &[&str],
    ) -> Result<ResolvedCommand, ManagedRuntimeCommandResolutionError> {
        let device = find_option_value(args, "--device").unwrap_or_default();
        let use_cuda = device.starts_with("CUDA");

        let (executable_path, library_dir) = if use_cuda {
            let cuda_executable = binaries_dir.join("cuda/llama-server");
            if !cuda_executable.exists() {
                return Err(
                    ManagedRuntimeCommandResolutionError::missing_llamacpp_cuda_variant(
                        device,
                        cuda_executable,
                    ),
                );
            }
            (cuda_executable, binaries_dir.join("cuda"))
        } else {
            (
                binaries_dir.join(self.installed_server_name()),
                binaries_dir.to_path_buf(),
            )
        };

        if !executable_path.exists() {
            return Err(ManagedRuntimeCommandResolutionError::platform(format!(
                "llama.cpp server binary not found at {}",
                executable_path.display()
            )));
        }

        let (args, pid_file) = extract_pid_file(args);

        Ok(ResolvedCommand {
            executable_path,
            working_directory: binaries_dir.to_path_buf(),
            args,
            env_overrides: vec![prepend_env_path("LD_LIBRARY_PATH", &library_dir, ":")],
            pid_file,
        })
    }

    fn finalize_installation(&self, binaries_dir: &Path) -> Result<(), String> {
        ensure_unix_library_aliases(
            binaries_dir,
            &["libggml.so", "libggml-base.so", "libllama.so", "libmtmd.so"],
        )?;

        let cuda_dir = binaries_dir.join("cuda");
        if cuda_dir.exists() {
            ensure_unix_library_aliases(
                &cuda_dir,
                &["libggml.so", "libggml-base.so", "libllama.so", "libmtmd.so"],
            )?;
        }

        Ok(())
    }

    fn is_runtime_library(&self, file_name: &str) -> bool {
        file_name.starts_with("lib") && (file_name.contains(".so") || file_name.ends_with(".so"))
    }
}

#[cfg(test)]
mod tests {
    use crate::DeviceResolutionDiagnosticCode;

    use super::{LlamaPlatform, PLATFORM};
    use crate::managed_runtime::ManagedRuntimeCommandResolutionError;

    #[test]
    fn resolve_command_rejects_cuda_device_without_cuda_runtime_variant() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let cpu_server = temp_dir.path().join(PLATFORM.installed_server_name());
        std::fs::write(&cpu_server, []).expect("write cpu server");

        let error = PLATFORM
            .resolve_command(temp_dir.path(), &["--device", "CUDA0"])
            .expect_err("missing CUDA runtime variant should fail");

        let ManagedRuntimeCommandResolutionError::MissingRuntimeVariant {
            diagnostic,
            requested_device,
            missing_path,
        } = error
        else {
            panic!("unexpected error: {error}");
        };

        assert_eq!(
            diagnostic.code,
            DeviceResolutionDiagnosticCode::MissingRuntimeVariant
        );
        assert_eq!(requested_device.as_deref(), Some("CUDA0"));
        assert!(missing_path.ends_with("cuda/llama-server"));
    }

    #[test]
    fn resolve_command_uses_cuda_runtime_variant_when_requested() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let cuda_dir = temp_dir.path().join("cuda");
        std::fs::create_dir_all(&cuda_dir).expect("create cuda dir");
        let cuda_server = cuda_dir.join("llama-server");
        std::fs::write(&cuda_server, []).expect("write cuda server");

        let resolved = PLATFORM
            .resolve_command(temp_dir.path(), &["--device=CUDA0"])
            .expect("resolve CUDA runtime command");

        assert_eq!(resolved.executable_path, cuda_server);
        let cuda_library_path = cuda_dir.to_string_lossy();
        assert!(resolved
            .env_overrides
            .iter()
            .any(|(key, value)| key == "LD_LIBRARY_PATH"
                && value
                    .to_string_lossy()
                    .starts_with(cuda_library_path.as_ref())));
    }
}
