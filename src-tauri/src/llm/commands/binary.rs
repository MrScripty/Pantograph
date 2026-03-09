//! Binary download and management commands.

use crate::llm::managed_binaries::{check_binary_status, download_binary, ManagedBinaryId};
use crate::llm::paths::{get_binaries_dir, get_managed_binaries_dir};
use crate::llm::types::{BinaryStatus, DownloadProgress};
use futures_util::TryStreamExt;
use std::io::Write;
use tauri::{command, ipc::Channel, AppHandle};

/// Check if llama.cpp binaries are available.
#[command]
pub async fn check_llama_binaries(app: AppHandle) -> Result<BinaryStatus, String> {
    check_binary_status(&app, ManagedBinaryId::LlamaCpp).await
}

/// Download llama.cpp binaries from GitHub releases.
#[command]
pub async fn download_llama_binaries(
    app: AppHandle,
    channel: Channel<DownloadProgress>,
) -> Result<(), String> {
    download_binary(&app, ManagedBinaryId::LlamaCpp, channel).await
}

/// Check if Ollama binary is available in our managed location.
#[command]
pub async fn check_ollama_binary(app: AppHandle) -> Result<BinaryStatus, String> {
    // First check system PATH (already installed by user).
    if which::which("ollama").is_ok() {
        return Ok(BinaryStatus {
            available: true,
            missing_files: vec![],
        });
    }

    let binaries_dir = get_binaries_dir(&app)?;
    let ollama_path = binaries_dir.join("ollama");

    if ollama_path.exists() {
        Ok(BinaryStatus {
            available: true,
            missing_files: vec![],
        })
    } else {
        Ok(BinaryStatus {
            available: false,
            missing_files: vec!["ollama".to_string()],
        })
    }
}

/// Download Ollama binary from GitHub releases.
#[command]
pub async fn download_ollama_binary(
    app: AppHandle,
    channel: Channel<DownloadProgress>,
) -> Result<(), String> {
    let binaries_dir = get_managed_binaries_dir(&app)?;

    std::fs::create_dir_all(&binaries_dir)
        .map_err(|e| format!("Failed to create binaries directory: {}", e))?;

    let release_tag = "v0.14.1";
    let archive_name = "ollama-linux-amd64.tar.zst";
    let download_url = format!(
        "https://github.com/ollama/ollama/releases/download/{}/{}",
        release_tag, archive_name
    );

    channel
        .send(DownloadProgress {
            status: "Downloading Ollama...".to_string(),
            current: 0,
            total: 0,
            done: false,
            error: None,
        })
        .ok();

    log::info!("Downloading Ollama from: {}", download_url);

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
    let temp_path = binaries_dir.join(archive_name);
    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream
        .try_next()
        .await
        .map_err(|e| format!("Download error: {}", e))?
    {
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {}", e))?;
        downloaded += chunk.len() as u64;

        channel
            .send(DownloadProgress {
                status: "Downloading...".to_string(),
                current: downloaded,
                total: total_size,
                done: false,
                error: None,
            })
            .ok();
    }
    drop(file);

    channel
        .send(DownloadProgress {
            status: "Extracting...".to_string(),
            current: total_size,
            total: total_size,
            done: false,
            error: None,
        })
        .ok();

    log::info!("Extracting Ollama archive to: {:?}", binaries_dir);

    let file =
        std::fs::File::open(&temp_path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let decoder =
        zstd::Decoder::new(file).map_err(|e| format!("Failed to create zstd decoder: {}", e))?;
    let mut archive = tar::Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|e| format!("Failed to read tar entries: {}", e))?
    {
        let mut entry = entry.map_err(|e| format!("Failed to read tar entry: {}", e))?;
        let path = entry
            .path()
            .map_err(|e| format!("Failed to get entry path: {}", e))?;
        let path_str = path.to_string_lossy();

        if path_str.ends_with("/ollama") || path_str == "ollama" {
            let dest_path = binaries_dir.join("ollama");
            let mut dest_file = std::fs::File::create(&dest_path)
                .map_err(|e| format!("Failed to create ollama binary: {}", e))?;

            std::io::copy(&mut entry, &mut dest_file)
                .map_err(|e| format!("Failed to extract ollama: {}", e))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                let mut perms = dest_file
                    .metadata()
                    .map_err(|e| format!("Failed to get metadata: {}", e))?
                    .permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&dest_path, perms)
                    .map_err(|e| format!("Failed to set permissions: {}", e))?;
            }

            log::info!("Extracted ollama binary to: {:?}", dest_path);
        }
    }

    let _ = std::fs::remove_file(&temp_path);

    let ollama_path = binaries_dir.join("ollama");
    if !ollama_path.exists() {
        return Err("Failed to extract ollama binary from archive".to_string());
    }

    channel
        .send(DownloadProgress {
            status: "Complete".to_string(),
            current: total_size,
            total: total_size,
            done: true,
            error: None,
        })
        .ok();

    log::info!("Ollama binary downloaded and extracted successfully");
    Ok(())
}
