use crate::managed_runtime::llama_cpp_platform::{
    current_platform as current_llama_platform, install_distribution as install_llama_distribution,
};
use crate::managed_runtime::ollama_platform::{
    current_platform as current_ollama_platform,
    install_distribution as install_ollama_distribution,
};
use flate2::read::GzDecoder;
use futures_util::TryStreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub mod llama_cpp_platform;
pub mod ollama_platform;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ManagedBinaryId {
    LlamaCpp,
    Ollama,
}

impl ManagedBinaryId {
    fn install_dir_name(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama-cpp",
            Self::Ollama => "ollama",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryStatus {
    pub available: bool,
    pub missing_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub status: String,
    pub current: u64,
    pub total: u64,
    pub done: bool,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArchiveKind {
    TarGz,
    TarZst,
    Zip,
}

#[derive(Clone, Debug)]
pub(crate) struct ReleaseAsset {
    pub(crate) archive_name: String,
    pub(crate) archive_kind: ArchiveKind,
}

#[derive(Clone, Debug)]
pub struct ResolvedCommand {
    pub executable_path: PathBuf,
    pub working_directory: PathBuf,
    pub args: Vec<OsString>,
    pub env_overrides: Vec<(OsString, OsString)>,
    pub pid_file: Option<PathBuf>,
}

trait ManagedBinaryDefinition: Sync {
    fn display_name(&self) -> &'static str;
    fn release_asset(&self) -> Result<ReleaseAsset, String>;
    fn download_url(&self, release_asset: &ReleaseAsset) -> String;
    fn validate_installation(&self, install_dir: &Path) -> Vec<String>;
    fn install_distribution(&self, extracted_dir: &Path, install_dir: &Path) -> Result<(), String>;
    fn resolve_command(&self, install_dir: &Path, args: &[&str]) -> Result<ResolvedCommand, String>;

    fn system_command(&self) -> Option<PathBuf> {
        None
    }
}

struct LlamaCppBinary;
struct OllamaBinary;

impl ManagedBinaryDefinition for LlamaCppBinary {
    fn display_name(&self) -> &'static str {
        "llama.cpp"
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

    fn resolve_command(&self, install_dir: &Path, args: &[&str]) -> Result<ResolvedCommand, String> {
        current_llama_platform().resolve_command(install_dir, args)
    }
}

impl ManagedBinaryDefinition for OllamaBinary {
    fn display_name(&self) -> &'static str {
        "Ollama"
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

    fn resolve_command(&self, install_dir: &Path, args: &[&str]) -> Result<ResolvedCommand, String> {
        current_ollama_platform().resolve_command(install_dir, args)
    }

    fn system_command(&self) -> Option<PathBuf> {
        which::which("ollama").ok()
    }
}

static LLAMA_CPP_BINARY: LlamaCppBinary = LlamaCppBinary;
static OLLAMA_BINARY: OllamaBinary = OllamaBinary;

static TRANSITION_LOCKS: Lazy<Mutex<HashMap<ManagedBinaryId, Arc<tokio::sync::Mutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn definition(id: ManagedBinaryId) -> &'static dyn ManagedBinaryDefinition {
    match id {
        ManagedBinaryId::LlamaCpp => &LLAMA_CPP_BINARY,
        ManagedBinaryId::Ollama => &OLLAMA_BINARY,
    }
}

fn transition_lock(id: ManagedBinaryId) -> Arc<tokio::sync::Mutex<()>> {
    let mut locks = TRANSITION_LOCKS
        .lock()
        .expect("managed binary transition locks poisoned");
    locks
        .entry(id)
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

pub fn managed_runtime_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("runtimes")
}

fn managed_install_dir(app_data_dir: &Path, id: ManagedBinaryId) -> PathBuf {
    managed_runtime_dir(app_data_dir).join(id.install_dir_name())
}

pub async fn check_binary_status(
    app_data_dir: &Path,
    id: ManagedBinaryId,
) -> Result<BinaryStatus, String> {
    let definition = definition(id);

    if definition.system_command().is_some() {
        return Ok(BinaryStatus {
            available: true,
            missing_files: Vec::new(),
        });
    }

    let install_dir = managed_install_dir(app_data_dir, id);
    let missing_files = definition.validate_installation(&install_dir);
    Ok(BinaryStatus {
        available: missing_files.is_empty(),
        missing_files,
    })
}

pub async fn download_binary<F>(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    mut on_progress: F,
) -> Result<(), String>
where
    F: FnMut(DownloadProgress),
{
    let lock = transition_lock(id);
    let _guard = lock.lock().await;
    let definition = definition(id);
    let runtime_root = managed_runtime_dir(app_data_dir);
    let install_dir = managed_install_dir(app_data_dir, id);
    let release_asset = definition.release_asset()?;
    let download_url = definition.download_url(&release_asset);

    fs::create_dir_all(&runtime_root)
        .map_err(|e| format!("Failed to create runtime directory: {}", e))?;

    on_progress(DownloadProgress {
        status: format!("Downloading {} binaries...", definition.display_name()),
        current: 0,
        total: 0,
        done: false,
        error: None,
    });

    log::info!(
        "Downloading {} from: {}",
        definition.display_name(),
        download_url
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    let temp_path = runtime_root.join(format!(
        ".{}-{}",
        uuid::Uuid::new_v4(),
        release_asset.archive_name
    ));
    let mut file =
        fs::File::create(&temp_path).map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut downloaded = 0_u64;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream
        .try_next()
        .await
        .map_err(|e| format!("Download error: {}", e))?
    {
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {}", e))?;
        downloaded += chunk.len() as u64;

        on_progress(DownloadProgress {
            status: "Downloading...".to_string(),
            current: downloaded,
            total: total_size,
            done: false,
            error: None,
        });
    }
    drop(file);

    on_progress(DownloadProgress {
        status: "Extracting...".to_string(),
        current: total_size,
        total: total_size,
        done: false,
        error: None,
    });

    let extract_dir =
        runtime_root.join(format!(".{}-extract-{}", id.install_dir_name(), uuid::Uuid::new_v4()));
    let staging_dir =
        runtime_root.join(format!(".{}-staging-{}", id.install_dir_name(), uuid::Uuid::new_v4()));
    fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extraction directory: {}", e))?;
    fs::create_dir_all(&staging_dir)
        .map_err(|e| format!("Failed to create staging directory: {}", e))?;

    let extraction_result =
        extract_archive(&temp_path, &extract_dir, release_asset.archive_kind)
            .and_then(|_| definition.install_distribution(&extract_dir, &staging_dir));

    let _ = fs::remove_dir_all(&extract_dir);
    let _ = fs::remove_file(&temp_path);

    if let Err(error) = extraction_result {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(error);
    }

    let missing = definition.validate_installation(&staging_dir);
    if let Some(first_missing) = missing.first() {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(format!(
            "{} extraction completed but runtime file is still missing: {}",
            definition.display_name(),
            first_missing
        ));
    }

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir)
            .map_err(|e| format!("Failed to replace existing install: {}", e))?;
    }
    if let Some(parent) = install_dir.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create install directory: {}", e))?;
    }
    fs::rename(&staging_dir, &install_dir)
        .map_err(|e| format!("Failed to finalize install: {}", e))?;

    on_progress(DownloadProgress {
        status: "Complete".to_string(),
        current: total_size,
        total: total_size,
        done: true,
        error: None,
    });

    log::info!(
        "{} binaries downloaded and extracted successfully",
        definition.display_name()
    );
    Ok(())
}

pub fn resolve_binary_command(
    app_data_dir: &Path,
    id: ManagedBinaryId,
    args: &[&str],
) -> Result<ResolvedCommand, String> {
    let definition = definition(id);

    if let Some(executable_path) = definition.system_command() {
        let (args, pid_file) = extract_pid_file(args);
        let working_directory = executable_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        return Ok(ResolvedCommand {
            executable_path,
            working_directory,
            args,
            env_overrides: Vec::new(),
            pid_file,
        });
    }

    let install_dir = managed_install_dir(app_data_dir, id);
    let missing = definition.validate_installation(&install_dir);
    if let Some(first_missing) = missing.first() {
        return Err(format!(
            "{} binaries are not installed for the current platform (missing {})",
            definition.display_name(),
            first_missing
        ));
    }

    definition.resolve_command(&install_dir, args)
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

pub(crate) fn prepend_env_path(
    key: &str,
    prefix: &Path,
    separator: &str,
) -> (OsString, OsString) {
    let mut value = prefix.as_os_str().to_os_string();
    if let Some(existing) = std::env::var_os(key) {
        if !existing.is_empty() {
            value.push(separator);
            value.push(existing);
        }
    }

    (OsString::from(key), value)
}

fn extract_archive(
    archive_path: &Path,
    destination: &Path,
    archive_kind: ArchiveKind,
) -> Result<(), String> {
    match archive_kind {
        ArchiveKind::TarGz => extract_tar_gz_archive(archive_path, destination),
        ArchiveKind::TarZst => extract_tar_zst_archive(archive_path, destination),
        ArchiveKind::Zip => extract_zip_archive(archive_path, destination),
    }
}

fn extract_tar_gz_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let archive_file =
        fs::File::open(archive_path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let decoder = GzDecoder::new(archive_file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(destination)
        .map_err(|e| format!("Failed to unpack tar.gz archive: {}", e))
}

fn extract_tar_zst_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let archive_file =
        fs::File::open(archive_path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let decoder = zstd::Decoder::new(archive_file)
        .map_err(|e| format!("Failed to create zstd decoder: {}", e))?;
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(destination)
        .map_err(|e| format!("Failed to unpack tar.zst archive: {}", e))
}

fn extract_zip_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let archive_file =
        fs::File::open(archive_path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(archive_file).map_err(|e| format!("Failed to read zip archive: {}", e))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;
        let relative_path = entry
            .enclosed_name()
            .map(|path| path.to_path_buf())
            .ok_or_else(|| format!("Archive entry has invalid path: {}", entry.name()))?;
        let output_path = destination.join(relative_path);

        if entry.is_dir() {
            fs::create_dir_all(&output_path)
                .map_err(|e| format!("Failed to create {:?}: {}", output_path, e))?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {:?}: {}", parent, e))?;
        }

        #[cfg(unix)]
        if zip_entry_is_symlink(&entry) {
            let mut target = String::new();
            entry.read_to_string(&mut target)
                .map_err(|e| format!("Failed to read symlink target: {}", e))?;
            let target_path = Path::new(target.trim())
                .file_name()
                .ok_or_else(|| format!("Invalid symlink target in {}", entry.name()))?;
            std::os::unix::fs::symlink(target_path, &output_path)
                .map_err(|e| format!("Failed to create symlink {:?}: {}", output_path, e))?;
            continue;
        }

        let mut output =
            fs::File::create(&output_path).map_err(|e| format!("Failed to create {:?}: {}", output_path, e))?;
        io::copy(&mut entry, &mut output)
            .map_err(|e| format!("Failed to extract {:?}: {}", output_path, e))?;
    }

    Ok(())
}

#[cfg(unix)]
fn zip_entry_is_symlink(entry: &zip::read::ZipFile<'_>) -> bool {
    entry.unix_mode()
        .map(|mode| (mode & 0o170000) == 0o120000)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::extract_pid_file;

    #[test]
    fn extract_pid_file_strips_split_flag() {
        let args = ["--host", "127.0.0.1", "--pid-file", "/tmp/pid", "--port", "8080"];
        let (sanitized, pid_file) = extract_pid_file(&args);

        assert_eq!(
            sanitized,
            vec!["--host", "127.0.0.1", "--port", "8080"]
        );
        assert_eq!(pid_file.as_deref(), Some(std::path::Path::new("/tmp/pid")));
    }

    #[test]
    fn extract_pid_file_strips_inline_flag() {
        let args = ["--pid-file=/tmp/pid", "--port", "8080"];
        let (sanitized, pid_file) = extract_pid_file(&args);

        assert_eq!(sanitized, vec!["--port", "8080"]);
        assert_eq!(pid_file.as_deref(), Some(std::path::Path::new("/tmp/pid")));
    }
}
