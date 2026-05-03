use crate::managed_runtime::llama_cpp_platform::{
    current_platform as current_llama_platform, current_platform_key as current_llama_platform_key,
    install_distribution as install_llama_distribution,
};

use super::contracts::{ManagedBinaryId, ReleaseAsset, ResolvedCommand};
use std::path::{Path, PathBuf};

pub(crate) trait ManagedBinaryDefinition: Sync {
    fn display_name(&self) -> &'static str;
    fn github_release_repo(&self) -> (&'static str, &'static str);
    fn default_release_version(&self) -> &'static str;
    fn release_asset(&self, version: &str) -> Result<ReleaseAsset, String>;
    fn download_url(&self, version: &str, release_asset: &ReleaseAsset) -> String;
    fn platform_key(&self) -> &'static str;
    fn executable_name(&self) -> &'static str;
    fn validate_installation(&self, install_dir: &Path) -> Vec<String>;
    fn install_distribution(&self, extracted_dir: &Path, install_dir: &Path) -> Result<(), String>;
    fn resolve_command(&self, install_dir: &Path, args: &[&str])
        -> Result<ResolvedCommand, String>;

    fn system_command(&self) -> Option<PathBuf> {
        None
    }
}

struct LlamaCppBinary;
struct OllamaBinary;

impl ManagedBinaryDefinition for LlamaCppBinary {
    fn display_name(&self) -> &'static str {
        ManagedBinaryId::LlamaCpp.display_name()
    }

    fn github_release_repo(&self) -> (&'static str, &'static str) {
        ("ggml-org", "llama.cpp")
    }

    fn default_release_version(&self) -> &'static str {
        crate::managed_runtime::llama_cpp_platform::LLAMA_CPP_RELEASE_TAG
    }

    fn release_asset(&self, version: &str) -> Result<ReleaseAsset, String> {
        Ok(current_llama_platform().release_asset(version))
    }

    fn download_url(&self, version: &str, release_asset: &ReleaseAsset) -> String {
        format!(
            "https://github.com/ggml-org/llama.cpp/releases/download/{}/{}",
            version, release_asset.archive_name
        )
    }

    fn platform_key(&self) -> &'static str {
        current_llama_platform_key()
    }

    fn executable_name(&self) -> &'static str {
        current_llama_platform().installed_server_name()
    }

    fn validate_installation(&self, install_dir: &Path) -> Vec<String> {
        current_llama_platform().validate_installation(install_dir)
    }

    fn install_distribution(&self, extracted_dir: &Path, install_dir: &Path) -> Result<(), String> {
        install_llama_distribution(extracted_dir, install_dir)
    }

    fn resolve_command(
        &self,
        install_dir: &Path,
        args: &[&str],
    ) -> Result<ResolvedCommand, String> {
        current_llama_platform().resolve_command(install_dir, args)
    }
}

impl ManagedBinaryDefinition for OllamaBinary {
    fn display_name(&self) -> &'static str {
        ManagedBinaryId::Ollama.display_name()
    }

    fn github_release_repo(&self) -> (&'static str, &'static str) {
        ("ollama", "ollama")
    }

    fn default_release_version(&self) -> &'static str {
        "retired"
    }

    fn release_asset(&self, version: &str) -> Result<ReleaseAsset, String> {
        Err(format!(
            "Ollama managed runtime support is retired; release {} is not installable by Pantograph",
            version
        ))
    }

    fn download_url(&self, _version: &str, _release_asset: &ReleaseAsset) -> String {
        String::new()
    }

    fn platform_key(&self) -> &'static str {
        "retired"
    }

    fn executable_name(&self) -> &'static str {
        "ollama"
    }

    fn validate_installation(&self, install_dir: &Path) -> Vec<String> {
        vec![install_dir.join("retired").display().to_string()]
    }

    fn install_distribution(
        &self,
        _extracted_dir: &Path,
        _install_dir: &Path,
    ) -> Result<(), String> {
        Err("Ollama managed runtime support is retired".to_string())
    }

    fn resolve_command(
        &self,
        _install_dir: &Path,
        _args: &[&str],
    ) -> Result<ResolvedCommand, String> {
        Err("Ollama managed runtime support is retired".to_string())
    }
}

static LLAMA_CPP_BINARY: LlamaCppBinary = LlamaCppBinary;
static OLLAMA_BINARY: OllamaBinary = OllamaBinary;

pub(crate) fn definition(id: ManagedBinaryId) -> &'static dyn ManagedBinaryDefinition {
    match id {
        ManagedBinaryId::LlamaCpp => &LLAMA_CPP_BINARY,
        ManagedBinaryId::Ollama => &OLLAMA_BINARY,
    }
}
