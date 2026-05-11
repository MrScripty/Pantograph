use crate::managed_runtime::{
    extract_pid_file as extract_pid_file_impl, prepend_env_path as prepend_env_path_impl,
    ArchiveKind, ManagedRuntimeCommandResolutionError, ReleaseAsset, ResolvedCommand,
};
use crate::RuntimeVariantId;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const LLAMA_CPP_RELEASE_TAG: &str = "b8248";

pub(crate) struct LlamaRuntimeVariant {
    pub(crate) runtime_variant_id: &'static str,
    pub(crate) display_suffix: Option<&'static str>,
}

pub(crate) const LLAMA_CPU_VARIANT: LlamaRuntimeVariant = LlamaRuntimeVariant {
    runtime_variant_id: "llama_cpp.cpu",
    display_suffix: None,
};

pub(crate) const LLAMA_CUDA_VARIANT: LlamaRuntimeVariant = LlamaRuntimeVariant {
    runtime_variant_id: "llama_cpp.cuda",
    display_suffix: Some("CUDA"),
};

#[cfg(any(test, target_os = "macos"))]
pub(crate) const LLAMA_METAL_VARIANT: LlamaRuntimeVariant = LlamaRuntimeVariant {
    runtime_variant_id: "llama_cpp.metal",
    display_suffix: Some("Metal"),
};

pub(crate) trait LlamaPlatform: Sync {
    fn release_asset(&self, version: &str) -> ReleaseAsset;
    fn catalog_runtime_variants(&self) -> &'static [LlamaRuntimeVariant] {
        &[LLAMA_CPU_VARIANT]
    }
    fn extracted_server_file_names(&self) -> &'static [&'static str];
    fn runtime_variant_install_subdir(&self, _relative_path: &Path) -> Option<&'static str> {
        None
    }
    fn installed_server_name(&self) -> &'static str;
    fn validate_installation(
        &self,
        binaries_dir: &Path,
        runtime_variant_id: &RuntimeVariantId,
    ) -> Vec<String>;
    fn resolve_command(
        &self,
        binaries_dir: &Path,
        runtime_variant_id: &RuntimeVariantId,
        args: &[&str],
    ) -> Result<ResolvedCommand, ManagedRuntimeCommandResolutionError>;
    fn finalize_installation(&self, binaries_dir: &Path) -> Result<(), String>;
    fn is_runtime_library(&self, file_name: &str) -> bool;
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux;
#[cfg(any(test, all(target_os = "macos", target_arch = "aarch64")))]
mod macos_arm64;
#[cfg(any(test, all(target_os = "macos", target_arch = "x86_64")))]
mod macos_x64;
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
mod windows;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use linux::PLATFORM as CURRENT_PLATFORM;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use macos_arm64::PLATFORM as CURRENT_PLATFORM;
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
use macos_x64::PLATFORM as CURRENT_PLATFORM;
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
use windows::PLATFORM as CURRENT_PLATFORM;

pub(crate) fn current_platform() -> &'static dyn LlamaPlatform {
    &CURRENT_PLATFORM
}

pub(crate) fn current_platform_key() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x86_64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "macos-arm64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "macos-x86_64"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "windows-x86_64"
    }
}

pub(crate) fn install_distribution(
    extracted_dir: &Path,
    binaries_dir: &Path,
) -> Result<(), String> {
    let platform = current_platform();
    fs::create_dir_all(binaries_dir)
        .map_err(|e| format!("Failed to create binaries directory: {}", e))?;

    let mut installed_server = false;
    copy_relevant_entries(
        extracted_dir,
        extracted_dir,
        binaries_dir,
        platform,
        &mut installed_server,
    )?;

    if !installed_server {
        return Err(format!(
            "Failed to find {} in extracted llama.cpp archive",
            platform.installed_server_name()
        ));
    }

    platform.finalize_installation(binaries_dir)
}

fn copy_relevant_entries(
    root: &Path,
    current: &Path,
    binaries_dir: &Path,
    platform: &dyn LlamaPlatform,
    installed_server: &mut bool,
) -> Result<(), String> {
    for entry in
        fs::read_dir(current).map_err(|e| format!("Failed to read {:?}: {}", current, e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let source_path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to read file type for {:?}: {}", source_path, e))?;

        if file_type.is_dir() {
            copy_relevant_entries(root, &source_path, binaries_dir, platform, installed_server)?;
            continue;
        }

        let relative_path = source_path
            .strip_prefix(root)
            .map_err(|e| format!("Failed to relativize {:?}: {}", source_path, e))?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let file_name_ref: &str = file_name.as_ref();

        let variant_subdir = platform.runtime_variant_install_subdir(relative_path);

        let destination = if platform
            .extracted_server_file_names()
            .contains(&file_name_ref)
        {
            if let Some(subdir) = variant_subdir {
                binaries_dir.join(subdir).join(file_name_ref)
            } else {
                *installed_server = true;
                binaries_dir.join(platform.installed_server_name())
            }
        } else if platform.is_runtime_library(file_name_ref) {
            let dest_dir = variant_subdir
                .map(|subdir| binaries_dir.join(subdir))
                .unwrap_or_else(|| binaries_dir.to_path_buf());
            dest_dir.join(file_name_ref)
        } else {
            continue;
        };

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {:?}: {}", parent, e))?;
        }

        copy_entry(&source_path, &destination, &file_type)?;
    }

    Ok(())
}

fn copy_entry(source: &Path, destination: &Path, file_type: &fs::FileType) -> Result<(), String> {
    #[cfg(unix)]
    if file_type.is_symlink() {
        let link_target = fs::read_link(source)
            .map_err(|e| format!("Failed to read symlink {:?}: {}", source, e))?;
        let file_name = link_target
            .file_name()
            .ok_or_else(|| format!("Invalid symlink target for {:?}", source))?;

        if destination.exists() {
            let _ = fs::remove_file(destination);
        }

        std::os::unix::fs::symlink(file_name, destination)
            .map_err(|e| format!("Failed to create symlink {:?}: {}", destination, e))?;
        return Ok(());
    }

    fs::copy(source, destination)
        .map_err(|e| format!("Failed to copy {:?} to {:?}: {}", source, destination, e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let file_name = destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if file_name.contains("llama-server")
            || file_name.ends_with(".so")
            || file_name.contains(".so.")
            || file_name.ends_with(".dylib")
            || file_name.contains(".dylib.")
        {
            let mut permissions = fs::metadata(destination)
                .map_err(|e| format!("Failed to read metadata for {:?}: {}", destination, e))?
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(destination, permissions).map_err(|e| {
                format!("Failed to update permissions for {:?}: {}", destination, e)
            })?;
        }
    }

    Ok(())
}

pub(crate) fn extract_pid_file(args: &[&str]) -> (Vec<OsString>, Option<PathBuf>) {
    extract_pid_file_impl(args)
}

#[cfg(test)]
pub(crate) fn find_option_value<'a>(args: &'a [&'a str], option_name: &str) -> Option<&'a str> {
    let mut index = 0;
    while index < args.len() {
        let arg = args[index];
        if arg == option_name {
            return args.get(index + 1).copied();
        }
        if let Some(value) = arg.strip_prefix(&format!("{}=", option_name)) {
            return Some(value);
        }
        index += 1;
    }
    None
}

pub(crate) fn prepend_env_path(key: &str, prefix: &Path, separator: &str) -> (OsString, OsString) {
    prepend_env_path_impl(key, prefix, separator)
}

#[cfg(unix)]
pub(crate) fn ensure_unix_library_aliases(directory: &Path, bases: &[&str]) -> Result<(), String> {
    for base in bases {
        ensure_unix_alias(directory, base)?;
    }
    Ok(())
}

#[cfg(unix)]
fn ensure_unix_alias(directory: &Path, base: &str) -> Result<(), String> {
    if directory.join(base).exists() {
        return Ok(());
    }

    let mut candidates = Vec::new();
    for entry in
        fs::read_dir(directory).map_err(|e| format!("Failed to read {:?}: {}", directory, e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy().to_string();
        if file_name.starts_with(&format!("{}.", base)) {
            candidates.push(file_name);
        }
    }

    candidates.sort();
    let Some(best_candidate) = candidates.pop() else {
        return Ok(());
    };

    create_unix_alias(directory, base, &best_candidate)?;

    if let Some(major_alias) = major_alias_name(base, &best_candidate) {
        create_unix_alias(directory, &major_alias, &best_candidate)?;
    }

    Ok(())
}

#[cfg(unix)]
fn create_unix_alias(directory: &Path, alias_name: &str, target_name: &str) -> Result<(), String> {
    let alias_path = directory.join(alias_name);
    if alias_path.exists() {
        return Ok(());
    }

    std::os::unix::fs::symlink(target_name, &alias_path)
        .map_err(|e| format!("Failed to create symlink {:?}: {}", alias_path, e))?;
    Ok(())
}

fn major_alias_name(base: &str, candidate: &str) -> Option<String> {
    let suffix = candidate.strip_prefix(base)?;
    let suffix = suffix.strip_prefix('.')?;
    let major = suffix.split('.').next()?;
    if major.chars().all(|c| c.is_ascii_digit()) {
        Some(format!("{}.{}", base, major))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_pid_file, find_option_value, major_alias_name};

    #[test]
    fn test_extract_pid_file_strips_split_flag() {
        let (args, pid_file) =
            extract_pid_file(&["-m", "model.gguf", "--pid-file", "/tmp/llama.pid"]);
        assert_eq!(args, vec!["-m", "model.gguf"]);
        assert_eq!(pid_file.unwrap().to_string_lossy(), "/tmp/llama.pid");
    }

    #[test]
    fn test_find_option_value_supports_equals_syntax() {
        let value = find_option_value(&["--device=CUDA0"], "--device");
        assert_eq!(value, Some("CUDA0"));
    }

    #[test]
    fn test_major_alias_name_extracts_soname() {
        let alias = major_alias_name("libllama.so", "libllama.so.0.0.8248");
        assert_eq!(alias.as_deref(), Some("libllama.so.0"));
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn install_distribution_uses_platform_archive_server_names_and_variant_subdir() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let extracted_dir = temp_dir.path().join("extracted");
        let cpu_dir = extracted_dir.join("bin");
        let cuda_dir = extracted_dir.join("cuda");
        std::fs::create_dir_all(&cpu_dir).expect("create cpu dir");
        std::fs::create_dir_all(&cuda_dir).expect("create cuda dir");
        std::fs::write(cpu_dir.join("llama-server"), []).expect("write cpu server");
        std::fs::write(cpu_dir.join("libllama.so"), []).expect("write cpu lib");
        std::fs::write(cuda_dir.join("llama-server"), []).expect("write cuda server");
        std::fs::write(cuda_dir.join("libllama.so"), []).expect("write cuda lib");

        let install_dir = temp_dir.path().join("install");
        super::install_distribution(&extracted_dir, &install_dir).expect("install distribution");

        assert!(install_dir
            .join(super::current_platform().installed_server_name())
            .exists());
        assert!(!install_dir.join("llama-server").exists());
        assert!(install_dir.join("cuda").join("llama-server").exists());
        assert!(install_dir.join("cuda").join("libllama.so").exists());
    }
}
