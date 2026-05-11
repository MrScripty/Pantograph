use std::path::Path;

use crate::RuntimeVariantId;

use super::{
    extract_pid_file, prepend_env_path, ArchiveKind, LlamaPlatform, LlamaRuntimeVariant,
    ManagedRuntimeCommandResolutionError, ReleaseAsset, ResolvedCommand, LLAMA_CPU_VARIANT,
    LLAMA_CUDA_VARIANT,
};

pub(crate) struct WindowsPlatform;

pub(crate) static PLATFORM: WindowsPlatform = WindowsPlatform;

impl LlamaPlatform for WindowsPlatform {
    fn release_asset(&self, version: &str) -> ReleaseAsset {
        ReleaseAsset {
            archive_name: format!("llama-{}-bin-win-x64.zip", version),
            archive_kind: ArchiveKind::Zip,
        }
    }

    fn catalog_runtime_variants(&self) -> &'static [LlamaRuntimeVariant] {
        &[LLAMA_CPU_VARIANT, LLAMA_CUDA_VARIANT]
    }

    fn installed_server_name(&self) -> &'static str {
        "llama-server-x86_64-pc-windows-msvc.exe"
    }

    fn validate_installation(&self, binaries_dir: &Path) -> Vec<String> {
        let mut missing = Vec::new();
        if !binaries_dir.join(self.installed_server_name()).exists() {
            missing.push(self.installed_server_name().to_string());
        }

        let has_llama_dll = std::fs::read_dir(binaries_dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy().to_ascii_lowercase();
                name.ends_with(".dll") && name.contains("llama")
            });

        if !has_llama_dll {
            missing.push("llama runtime DLL".to_string());
        }

        missing
    }

    fn resolve_command(
        &self,
        binaries_dir: &Path,
        runtime_variant_id: &RuntimeVariantId,
        args: &[&str],
    ) -> Result<ResolvedCommand, ManagedRuntimeCommandResolutionError> {
        if runtime_variant_id.as_str() != "llama_cpp.cpu"
            && runtime_variant_id.as_str() != "llama_cpp.cuda"
        {
            return Err(ManagedRuntimeCommandResolutionError::platform(format!(
                "unsupported llama.cpp runtime variant '{}'",
                runtime_variant_id
            )));
        }

        let executable_path = binaries_dir.join(self.installed_server_name());
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
            env_overrides: vec![prepend_env_path("PATH", binaries_dir, ";")],
            pid_file,
        })
    }

    fn finalize_installation(&self, _binaries_dir: &Path) -> Result<(), String> {
        Ok(())
    }

    fn is_runtime_library(&self, file_name: &str) -> bool {
        file_name.to_ascii_lowercase().ends_with(".dll")
    }
}
