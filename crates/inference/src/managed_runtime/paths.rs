use super::contracts::ManagedBinaryId;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub fn managed_runtime_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("runtimes")
}

pub(crate) fn managed_install_dir(app_data_dir: &Path, id: ManagedBinaryId) -> PathBuf {
    managed_runtime_dir(app_data_dir).join(id.install_dir_name())
}

pub(crate) fn managed_version_install_dir(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    version: &str,
) -> PathBuf {
    managed_install_dir(app_data_dir, id)
        .join("versions")
        .join(version)
}

pub(crate) fn extract_pid_file(args: &[&str]) -> (Vec<OsString>, Option<PathBuf>) {
    let mut sanitized = Vec::with_capacity(args.len());
    let mut pid_file = None;
    let mut index = 0;

    while index < args.len() {
        let arg = args[index];

        if arg == "--pid-file" {
            if let Some(path) = args.get(index + 1) {
                pid_file = Some(PathBuf::from(path));
            }
            index += 2;
            continue;
        }

        if let Some(path) = arg.strip_prefix("--pid-file=") {
            pid_file = Some(PathBuf::from(path));
            index += 1;
            continue;
        }

        sanitized.push(OsString::from(arg));
        index += 1;
    }

    (sanitized, pid_file)
}

pub(crate) fn prepend_env_path(key: &str, prefix: &Path, separator: &str) -> (OsString, OsString) {
    let mut value = prefix.as_os_str().to_os_string();
    if let Some(existing) = std::env::var_os(key) {
        if !existing.is_empty() {
            value.push(separator);
            value.push(existing);
        }
    }

    (OsString::from(key), value)
}

#[cfg(test)]
mod tests {
    use super::{extract_pid_file, managed_install_dir, managed_version_install_dir};
    use crate::managed_runtime::ManagedBinaryId;

    #[test]
    fn extract_pid_file_strips_split_flag() {
        let args = [
            "--host",
            "127.0.0.1",
            "--pid-file",
            "/tmp/pid",
            "--port",
            "8080",
        ];
        let (sanitized, pid_file) = extract_pid_file(&args);

        assert_eq!(sanitized, vec!["--host", "127.0.0.1", "--port", "8080"]);
        assert_eq!(pid_file.as_deref(), Some(std::path::Path::new("/tmp/pid")));
    }

    #[test]
    fn extract_pid_file_strips_inline_flag() {
        let args = ["--pid-file=/tmp/pid", "--port", "8080"];
        let (sanitized, pid_file) = extract_pid_file(&args);

        assert_eq!(sanitized, vec!["--port", "8080"]);
        assert_eq!(pid_file.as_deref(), Some(std::path::Path::new("/tmp/pid")));
    }

    #[test]
    fn managed_version_install_dir_nests_version_under_runtime_root() {
        let app_data_dir = std::path::Path::new("/tmp/pantograph");
        let runtime_root = managed_install_dir(app_data_dir, ManagedBinaryId::LlamaCpp);

        let version_dir =
            managed_version_install_dir(app_data_dir, ManagedBinaryId::LlamaCpp, "b8248");

        assert_eq!(version_dir, runtime_root.join("versions").join("b8248"));
    }
}
