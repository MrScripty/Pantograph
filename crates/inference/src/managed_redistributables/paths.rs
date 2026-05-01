use std::path::{Path, PathBuf};

use super::contracts::{ManagedRedistributableArchiveKind, ManagedRedistributableId};

const MANAGED_REDISTRIBUTABLES_STATE_FILE: &str = "state.json";

pub fn managed_redistributables_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir
        .join("third-party")
        .join("managed-dependencies")
}

pub(crate) fn legacy_managed_redistributables_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("managed-dependencies")
}

pub(crate) fn managed_redistributable_version_dir(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
    version: &str,
) -> PathBuf {
    let current = managed_redistributables_dir(app_data_dir)
        .join(id.key())
        .join("versions")
        .join(version);
    if current.exists() {
        return current;
    }

    let legacy = legacy_managed_redistributables_dir(app_data_dir)
        .join(id.key())
        .join("versions")
        .join(version);
    if legacy.exists() {
        return legacy;
    }

    current
}

pub(crate) fn redistributables_state_path(app_data_dir: &Path) -> PathBuf {
    managed_redistributables_dir(app_data_dir).join(MANAGED_REDISTRIBUTABLES_STATE_FILE)
}

pub(crate) fn temp_state_path(path: &Path) -> PathBuf {
    let mut temp_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    temp_name.push(format!(".tmp-{}", uuid::Uuid::new_v4()));
    path.with_file_name(temp_name)
}

pub(crate) fn current_platform_key() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x64"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "linux-arm64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "macos-x64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "macos-arm64"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "windows-x64"
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    {
        "unsupported"
    }
}

pub(crate) fn archive_kind_for_current_platform() -> Option<ManagedRedistributableArchiveKind> {
    #[cfg(target_os = "windows")]
    {
        Some(ManagedRedistributableArchiveKind::Zip)
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        Some(ManagedRedistributableArchiveKind::TarGz)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

pub(crate) fn tool_path(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("bin/{name}.exe")
    } else {
        format!("bin/{name}")
    }
}

pub(crate) fn library_path(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("bin/{name}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib/lib{name}.dylib")
    } else {
        format!("lib/lib{name}.so")
    }
}

#[cfg(test)]
mod tests {
    use super::{managed_redistributable_version_dir, ManagedRedistributableId};

    #[test]
    fn managed_redistributable_version_dir_falls_back_to_legacy_root() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let legacy_dir = temp_dir
            .path()
            .join("managed-dependencies")
            .join("ffmpeg")
            .join("versions")
            .join("n7.1.1");
        std::fs::create_dir_all(&legacy_dir).expect("legacy dependency dir");

        assert_eq!(
            managed_redistributable_version_dir(
                temp_dir.path(),
                ManagedRedistributableId::Ffmpeg,
                "n7.1.1"
            ),
            legacy_dir
        );
    }
}

pub(crate) fn sanitize_path_segment(segment: &str) -> String {
    segment
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn current_unix_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
