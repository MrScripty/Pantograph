use std::path::Path;

use crate::RuntimeVariantId;

use super::{
    ensure_unix_library_aliases, extract_pid_file, managed_env_path, ArchiveKind, LlamaPlatform,
    LlamaRuntimeVariant, ManagedRuntimeCommandResolutionError, ReleaseAsset, ResolvedCommand,
    LLAMA_CPU_VARIANT, LLAMA_METAL_VARIANT,
};

pub(crate) struct MacOsArm64Platform;

pub(crate) static PLATFORM: MacOsArm64Platform = MacOsArm64Platform;

impl LlamaPlatform for MacOsArm64Platform {
    fn release_asset(&self, version: &str) -> ReleaseAsset {
        ReleaseAsset {
            archive_name: format!("llama-{}-bin-macos-arm64.zip", version),
            archive_kind: ArchiveKind::Zip,
        }
    }

    fn installed_server_name(&self) -> &'static str {
        "llama-server-aarch64-apple-darwin"
    }

    fn extracted_server_file_names(&self) -> &'static [&'static str] {
        &["llama-server"]
    }

    fn catalog_runtime_variants(&self) -> &'static [LlamaRuntimeVariant] {
        &[LLAMA_CPU_VARIANT, LLAMA_METAL_VARIANT]
    }

    fn validate_installation(
        &self,
        binaries_dir: &Path,
        runtime_variant_id: &RuntimeVariantId,
    ) -> Vec<String> {
        if runtime_variant_id.as_str() == "llama_cpp.metal" {
            return validate_required_files(
                binaries_dir,
                &[
                    self.installed_server_name(),
                    "libllama.dylib",
                    "libggml-metal.dylib",
                ],
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
            &[self.installed_server_name(), "libllama.dylib"],
        )
    }

    fn resolve_command(
        &self,
        binaries_dir: &Path,
        runtime_variant_id: &RuntimeVariantId,
        args: &[&str],
    ) -> Result<ResolvedCommand, ManagedRuntimeCommandResolutionError> {
        if runtime_variant_id.as_str() == "llama_cpp.metal"
            && !binaries_dir.join("libggml-metal.dylib").exists()
        {
            return Err(
                ManagedRuntimeCommandResolutionError::missing_llamacpp_selected_variant(
                    runtime_variant_id,
                    binaries_dir.join("libggml-metal.dylib"),
                ),
            );
        }

        if runtime_variant_id.as_str() != "llama_cpp.cpu"
            && runtime_variant_id.as_str() != "llama_cpp.metal"
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
            env_overrides: vec![managed_env_path("DYLD_LIBRARY_PATH", binaries_dir)],
            pid_file,
        })
    }

    fn finalize_installation(&self, binaries_dir: &Path) -> Result<(), String> {
        ensure_unix_library_aliases(
            binaries_dir,
            &[
                "libggml.dylib",
                "libggml-base.dylib",
                "libllama.dylib",
                "libmtmd.dylib",
            ],
        )
    }

    fn is_runtime_library(&self, file_name: &str) -> bool {
        file_name.starts_with("lib")
            && (file_name.ends_with(".dylib") || file_name.contains(".dylib."))
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
    use super::{LlamaPlatform, PLATFORM};
    use crate::RuntimeVariantId;

    fn runtime_variant_id(value: &str) -> RuntimeVariantId {
        RuntimeVariantId::parse(value).expect("valid runtime variant")
    }

    #[test]
    fn catalog_runtime_variants_include_metal() {
        let variants = PLATFORM.catalog_runtime_variants();

        assert!(variants
            .iter()
            .any(|variant| variant.runtime_variant_id == "llama_cpp.metal"));
    }

    #[test]
    fn validate_installation_requires_metal_runtime_library_for_metal_variant() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(temp_dir.path().join(PLATFORM.installed_server_name()), [])
            .expect("write server");
        std::fs::write(temp_dir.path().join("libllama.dylib"), []).expect("write libllama");

        let missing =
            PLATFORM.validate_installation(temp_dir.path(), &runtime_variant_id("llama_cpp.metal"));

        assert_eq!(missing, vec!["libggml-metal.dylib".to_string()]);
    }
}
