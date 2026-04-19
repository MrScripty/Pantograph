use crate::managed_runtime::llama_cpp_platform::{
    current_platform as current_llama_platform, install_distribution as install_llama_distribution,
};
use crate::managed_runtime::ollama_platform::{
    current_platform as current_ollama_platform,
    install_distribution as install_ollama_distribution,
};

use super::contracts::{ManagedBinaryId, ReleaseAsset, ResolvedCommand};
use std::path::{Path, PathBuf};

pub(crate) trait ManagedBinaryDefinition: Sync {
    fn display_name(&self) -> &'static str;
    fn release_asset(&self) -> Result<ReleaseAsset, String>;
    fn download_url(&self, release_asset: &ReleaseAsset) -> String;
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

    fn release_asset(&self) -> Result<ReleaseAsset, String> {
        Ok(current_llama_platform().release_asset())
    }

    fn download_url(&self, release_asset: &ReleaseAsset) -> String {
        format!(
            "https://github.com/ggml-org/llama.cpp/releases/download/{}/{}",
            crate::managed_runtime::llama_cpp_platform::LLAMA_CPP_RELEASE_TAG,
            release_asset.archive_name
        )
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

    fn release_asset(&self) -> Result<ReleaseAsset, String> {
        Ok(current_ollama_platform().release_asset())
    }

    fn download_url(&self, release_asset: &ReleaseAsset) -> String {
        format!(
            "https://github.com/ollama/ollama/releases/download/{}/{}",
            crate::managed_runtime::ollama_platform::OLLAMA_RELEASE_TAG,
            release_asset.archive_name
        )
    }

    fn validate_installation(&self, install_dir: &Path) -> Vec<String> {
        current_ollama_platform().validate_installation(install_dir)
    }

    fn install_distribution(&self, extracted_dir: &Path, install_dir: &Path) -> Result<(), String> {
        install_ollama_distribution(extracted_dir, install_dir)
    }

    fn resolve_command(
        &self,
        install_dir: &Path,
        args: &[&str],
    ) -> Result<ResolvedCommand, String> {
        current_ollama_platform().resolve_command(install_dir, args)
    }

    fn system_command(&self) -> Option<PathBuf> {
        which::which("ollama").ok()
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
