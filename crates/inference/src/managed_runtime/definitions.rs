use crate::managed_runtime::llama_cpp_platform::{
    current_platform as current_llama_platform, current_platform_key as current_llama_platform_key,
    install_distribution as install_llama_distribution,
};

use super::contracts::{
    ManagedBinaryId, ManagedRuntimeCommandResolutionError, ReleaseAsset, ResolvedCommand,
};
use crate::RuntimeVariantId;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ManagedRuntimeVariantDefinition {
    pub(crate) runtime_variant_id: RuntimeVariantId,
    pub(crate) display_suffix: Option<&'static str>,
}

pub(crate) trait ManagedBinaryDefinition: Sync {
    fn display_name(&self) -> &'static str;
    fn github_release_repo(&self) -> (&'static str, &'static str);
    fn default_release_version(&self) -> &'static str;
    fn release_asset(&self, version: &str) -> Result<ReleaseAsset, String>;
    fn download_url(&self, version: &str, release_asset: &ReleaseAsset) -> String;
    fn default_runtime_variant_id(&self) -> RuntimeVariantId;
    fn catalog_runtime_variants(&self) -> Vec<ManagedRuntimeVariantDefinition>;
    fn platform_key(&self) -> &'static str;
    fn executable_name(&self) -> &'static str;
    fn validate_installation(&self, install_dir: &Path) -> Vec<String>;
    fn install_distribution(&self, extracted_dir: &Path, install_dir: &Path) -> Result<(), String>;
    fn resolve_command(
        &self,
        install_dir: &Path,
        args: &[&str],
    ) -> Result<ResolvedCommand, ManagedRuntimeCommandResolutionError>;

    fn system_command(&self) -> Option<PathBuf> {
        None
    }
}

struct LlamaCppBinary;

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

    fn default_runtime_variant_id(&self) -> RuntimeVariantId {
        RuntimeVariantId::parse("llama_cpp.cpu").expect("static runtime variant id is valid")
    }

    fn catalog_runtime_variants(&self) -> Vec<ManagedRuntimeVariantDefinition> {
        current_llama_platform()
            .catalog_runtime_variants()
            .iter()
            .map(|variant| ManagedRuntimeVariantDefinition {
                runtime_variant_id: RuntimeVariantId::parse(variant.runtime_variant_id)
                    .expect("static runtime variant id is valid"),
                display_suffix: variant.display_suffix,
            })
            .collect()
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
    ) -> Result<ResolvedCommand, ManagedRuntimeCommandResolutionError> {
        current_llama_platform().resolve_command(install_dir, args)
    }
}

static LLAMA_CPP_BINARY: LlamaCppBinary = LlamaCppBinary;

pub(crate) fn definition(id: ManagedBinaryId) -> &'static dyn ManagedBinaryDefinition {
    match id {
        ManagedBinaryId::LlamaCpp => &LLAMA_CPP_BINARY,
    }
}
