use crate::managed_runtime::{ReleaseAsset, ResolvedCommand};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const OLLAMA_RELEASE_TAG: &str = "v0.14.1";

pub(crate) trait OllamaPlatform: Sync {
    fn release_asset(&self) -> ReleaseAsset;
    fn executable_name(&self) -> &'static str;
    fn validate_installation(&self, install_dir: &Path) -> Vec<String>;
    fn resolve_command(&self, install_dir: &Path, args: &[&str]) -> Result<ResolvedCommand, String>;
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod macos_arm64;
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
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

pub(crate) fn current_platform() -> &'static dyn OllamaPlatform {
    &CURRENT_PLATFORM
}

pub(crate) fn install_distribution(extracted_dir: &Path, install_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(install_dir)
        .map_err(|e| format!("Failed to create binaries directory: {}", e))?;
    copy_entries(extracted_dir, extracted_dir, install_dir)
}

pub(crate) fn find_executable(install_dir: &Path, file_name: &str) -> Option<PathBuf> {
    find_first_path(install_dir, &|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == file_name)
    })
}

fn copy_entries(root: &Path, current: &Path, install_dir: &Path) -> Result<(), String> {
    let mut entries = fs::read_dir(current)
        .map_err(|e| format!("Failed to read {:?}: {}", current, e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read directory entry: {}", e))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let source_path = entry.path();
        let relative_path = source_path
            .strip_prefix(root)
            .map_err(|e| format!("Failed to relativize {:?}: {}", source_path, e))?;
        let destination = install_dir.join(relative_path);
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to read file type for {:?}: {}", source_path, e))?;

        if file_type.is_dir() {
            fs::create_dir_all(&destination)
                .map_err(|e| format!("Failed to create {:?}: {}", destination, e))?;
            copy_entries(root, &source_path, install_dir)?;
            continue;
        }

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

        if destination.exists() {
            let _ = fs::remove_file(destination);
        }

        std::os::unix::fs::symlink(&link_target, destination)
            .map_err(|e| format!("Failed to create symlink {:?}: {}", destination, e))?;
        return Ok(());
    }

    fs::copy(source, destination)
        .map_err(|e| format!("Failed to copy {:?} to {:?}: {}", source, destination, e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let permissions = fs::metadata(source)
            .map_err(|e| format!("Failed to read metadata for {:?}: {}", source, e))?
            .permissions();
        fs::set_permissions(destination, permissions)
            .map_err(|e| format!("Failed to set permissions for {:?}: {}", destination, e))?;

        let mut permissions = fs::metadata(destination)
            .map_err(|e| format!("Failed to read metadata for {:?}: {}", destination, e))?
            .permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        fs::set_permissions(destination, permissions)
            .map_err(|e| format!("Failed to update permissions for {:?}: {}", destination, e))?;
    }

    Ok(())
}

fn find_first_path(root: &Path, predicate: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    if !root.exists() {
        return None;
    }

    let mut entries = fs::read_dir(root).ok()?.collect::<Result<Vec<_>, _>>().ok()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_first_path(&path, predicate) {
                return Some(found);
            }
            continue;
        }

        if predicate(&path) {
            return Some(path);
        }
    }

    None
}
