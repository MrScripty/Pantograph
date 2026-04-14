use std::path::Path;

use super::{
    ensure_unix_library_aliases, extract_pid_file, prepend_env_path, ArchiveKind, LlamaPlatform,
    ReleaseAsset, ResolvedCommand, LLAMA_CPP_RELEASE_TAG,
};

pub(crate) struct MacOsArm64Platform;

pub(crate) static PLATFORM: MacOsArm64Platform = MacOsArm64Platform;

impl LlamaPlatform for MacOsArm64Platform {
    fn release_asset(&self) -> ReleaseAsset {
        ReleaseAsset {
            archive_name: format!("llama-{}-bin-macos-arm64.zip", LLAMA_CPP_RELEASE_TAG),
            archive_kind: ArchiveKind::Zip,
        }
    }

    fn installed_server_name(&self) -> &'static str {
        "llama-server-aarch64-apple-darwin"
    }

    fn validate_installation(&self, binaries_dir: &Path) -> Vec<String> {
        let mut missing = Vec::new();
        if !binaries_dir.join(self.installed_server_name()).exists() {
            missing.push(self.installed_server_name().to_string());
        }
        if !binaries_dir.join("libllama.dylib").exists() {
            missing.push("libllama.dylib".to_string());
        }
        missing
    }

    fn resolve_command(
        &self,
        binaries_dir: &Path,
        args: &[&str],
    ) -> Result<ResolvedCommand, String> {
        let executable_path = binaries_dir.join(self.installed_server_name());
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
            env_overrides: vec![prepend_env_path("DYLD_LIBRARY_PATH", binaries_dir, ":")],
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
