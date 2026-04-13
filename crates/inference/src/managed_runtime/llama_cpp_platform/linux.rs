use std::path::Path;

use super::{
    ArchiveKind, LLAMA_CPP_RELEASE_TAG, LlamaPlatform, ReleaseAsset, ResolvedCommand,
    ensure_unix_library_aliases, extract_pid_file, find_option_value, prepend_env_path,
};

pub(crate) struct LinuxPlatform;

pub(crate) static PLATFORM: LinuxPlatform = LinuxPlatform;

impl LlamaPlatform for LinuxPlatform {
    fn release_asset(&self) -> ReleaseAsset {
        ReleaseAsset {
            archive_name: format!("llama-{}-bin-ubuntu-x64.tar.gz", LLAMA_CPP_RELEASE_TAG),
            archive_kind: ArchiveKind::TarGz,
        }
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
    ) -> Result<ResolvedCommand, String> {
        let device = find_option_value(args, "--device").unwrap_or_default();
        let use_cuda = device.starts_with("CUDA");

        let (executable_path, library_dir) =
            if use_cuda && binaries_dir.join("cuda/llama-server").exists() {
                (
                    binaries_dir.join("cuda/llama-server"),
                    binaries_dir.join("cuda"),
                )
            } else {
                (
                    binaries_dir.join(self.installed_server_name()),
                    binaries_dir.to_path_buf(),
                )
            };

        if !executable_path.exists() {
            return Err(format!(
                "llama.cpp server binary not found at {}",
                executable_path.display()
            ));
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
