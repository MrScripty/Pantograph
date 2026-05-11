use std::path::Path;

use super::{
    ensure_unix_library_aliases, extract_pid_file, managed_env_path, ArchiveKind, LlamaPlatform,
    LlamaRuntimeVariant, ManagedRuntimeCommandResolutionError, ReleaseAsset, ResolvedCommand,
    LLAMA_CPU_VARIANT, LLAMA_CUDA_VARIANT,
};
use crate::RuntimeVariantId;

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

    fn extracted_server_file_names(&self) -> &'static [&'static str] {
        &["llama-server"]
    }

    fn runtime_variant_install_subdir(&self, relative_path: &Path) -> Option<&'static str> {
        relative_path
            .components()
            .any(|component| component.as_os_str() == "cuda")
            .then_some("cuda")
    }

    fn validate_installation(
        &self,
        binaries_dir: &Path,
        runtime_variant_id: &RuntimeVariantId,
    ) -> Vec<String> {
        if runtime_variant_id.as_str() == "llama_cpp.cuda" {
            return validate_required_files(
                binaries_dir,
                &["cuda/llama-server", "cuda/libllama.so", "cuda/libggml.so"],
            );
        }

        if runtime_variant_id.as_str() != "llama_cpp.cpu" {
            return vec![format!(
                "unsupported llama.cpp runtime variant '{}'",
                runtime_variant_id
            )];
        }

        validate_required_files(
            binaries_dir,
            &[self.installed_server_name(), "libllama.so", "libggml.so"],
        )
    }

    fn resolve_command(
        &self,
        binaries_dir: &Path,
        runtime_variant_id: &RuntimeVariantId,
        args: &[&str],
    ) -> Result<ResolvedCommand, ManagedRuntimeCommandResolutionError> {
        let (executable_path, library_dir) = if runtime_variant_id.as_str() == "llama_cpp.cuda" {
            let cuda_executable = binaries_dir.join("cuda/llama-server");
            if !cuda_executable.exists() {
                return Err(
                    ManagedRuntimeCommandResolutionError::missing_llamacpp_selected_variant(
                        runtime_variant_id,
                        cuda_executable,
                    ),
                );
            }
            (cuda_executable, binaries_dir.join("cuda"))
        } else if runtime_variant_id.as_str() == "llama_cpp.cpu" {
            (
                binaries_dir.join(self.installed_server_name()),
                binaries_dir.to_path_buf(),
            )
        } else {
            return Err(ManagedRuntimeCommandResolutionError::platform(format!(
                "unsupported llama.cpp runtime variant '{}'",
                runtime_variant_id
            )));
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
            env_overrides: vec![managed_env_path("LD_LIBRARY_PATH", &library_dir)],
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

fn validate_required_files(binaries_dir: &Path, required_files: &[&str]) -> Vec<String> {
    required_files
        .iter()
        .filter(|relative_path| !binaries_dir.join(relative_path).exists())
        .map(|relative_path| (*relative_path).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::DeviceResolutionDiagnosticCode;

    use super::{LlamaPlatform, PLATFORM};
    use crate::managed_runtime::ManagedRuntimeCommandResolutionError;
    use crate::RuntimeVariantId;

    fn runtime_variant_id(value: &str) -> RuntimeVariantId {
        RuntimeVariantId::parse(value).expect("valid runtime variant")
    }

    #[test]
    fn resolve_command_rejects_selected_cuda_variant_without_cuda_runtime() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let cpu_server = temp_dir.path().join(PLATFORM.installed_server_name());
        std::fs::write(&cpu_server, []).expect("write cpu server");

        let error = PLATFORM
            .resolve_command(
                temp_dir.path(),
                &runtime_variant_id("llama_cpp.cuda"),
                &["--device", "CUDA0"],
            )
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
        assert_eq!(
            diagnostic
                .runtime_variant_id
                .as_ref()
                .map(RuntimeVariantId::as_str),
            Some("llama_cpp.cuda")
        );
        assert_eq!(requested_device, None);
        assert!(missing_path.ends_with("cuda/llama-server"));
    }

    #[test]
    fn validate_installation_checks_selected_cuda_variant_files() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let cpu_server = temp_dir.path().join(PLATFORM.installed_server_name());
        std::fs::write(&cpu_server, []).expect("write cpu server");
        std::fs::write(temp_dir.path().join("libllama.so"), []).expect("write cpu libllama");
        std::fs::write(temp_dir.path().join("libggml.so"), []).expect("write cpu libggml");

        let missing =
            PLATFORM.validate_installation(temp_dir.path(), &runtime_variant_id("llama_cpp.cuda"));

        assert!(missing.contains(&"cuda/llama-server".to_string()));
        assert!(missing.contains(&"cuda/libllama.so".to_string()));
        assert!(missing.contains(&"cuda/libggml.so".to_string()));
    }

    #[test]
    fn resolve_command_uses_selected_cuda_runtime_variant() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let cuda_dir = temp_dir.path().join("cuda");
        std::fs::create_dir_all(&cuda_dir).expect("create cuda dir");
        let cuda_server = cuda_dir.join("llama-server");
        std::fs::write(&cuda_server, []).expect("write cuda server");

        let resolved = PLATFORM
            .resolve_command(
                temp_dir.path(),
                &runtime_variant_id("llama_cpp.cuda"),
                &["--device=CUDA0"],
            )
            .expect("resolve CUDA runtime command");

        assert_eq!(resolved.executable_path, cuda_server);
        assert!(resolved
            .env_overrides
            .iter()
            .any(|(key, value)| key == "LD_LIBRARY_PATH" && value == cuda_dir.as_os_str()));
    }

    #[test]
    fn resolve_command_does_not_infer_cuda_variant_from_device_arg() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let cpu_server = temp_dir.path().join(PLATFORM.installed_server_name());
        std::fs::write(&cpu_server, []).expect("write cpu server");

        let resolved = PLATFORM
            .resolve_command(
                temp_dir.path(),
                &runtime_variant_id("llama_cpp.cpu"),
                &["--device=CUDA0"],
            )
            .expect("resolve CPU runtime command");

        assert_eq!(resolved.executable_path, cpu_server);
    }
}
