//! Binary download and management commands.

use crate::llm::types::{BinaryStatus, DownloadProgress};
use futures_util::TryStreamExt;
use std::io::Write;
use std::path::PathBuf;
use tauri::{command, ipc::Channel, AppHandle, Manager};

/// Get the binaries directory path
fn get_binaries_dir(_app: &AppHandle) -> Result<PathBuf, String> {
    // In dev mode, binaries are in src-tauri/binaries
    // build.rs copies them to target/debug for the sidecar system
    // We check src-tauri/binaries as that's where downloads should go

    // Try to find src-tauri/binaries relative to current exe or working directory
    let candidates = [
        // Dev mode: current working directory is src-tauri
        std::env::current_dir().ok().map(|p| p.join("binaries")),
        // Dev mode: exe is in target/debug, binaries in src-tauri/binaries
        std::env::current_exe().ok().and_then(|p| {
            p.parent() // target/debug
                .and_then(|p| p.parent()) // target
                .and_then(|p| p.parent()) // src-tauri
                .map(|p| p.join("binaries"))
        }),
        // Production: binaries next to exe
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("binaries"))),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            log::debug!("Found binaries dir at: {:?}", candidate);
            return Ok(candidate);
        }
    }

    // Fallback: create in current directory
    let fallback = std::env::current_dir()
        .map_err(|e| format!("Failed to get current dir: {}", e))?
        .join("binaries");
    log::warn!("Binaries dir not found, using fallback: {:?}", fallback);
    Ok(fallback)
}

/// Get the directory for downloading binaries (uses app data dir to avoid triggering recompilation)
fn get_download_binaries_dir(app: &AppHandle) -> Result<PathBuf, String> {
    // Use app data directory for downloads - this is outside the source tree
    // and won't trigger Tauri's file watcher during dev mode
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let binaries_dir = app_data_dir.join("binaries");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&binaries_dir)
        .map_err(|e| format!("Failed to create binaries directory: {}", e))?;

    log::debug!("Download binaries dir: {:?}", binaries_dir);
    Ok(binaries_dir)
}

/// Required files for llama.cpp backend
const REQUIRED_BINARIES: &[&str] = &[
    "llama-server-x86_64-unknown-linux-gnu",
    "libllama.so",
    "libggml.so",
];

/// Check if llama.cpp binaries are available
#[command]
pub async fn check_llama_binaries(app: AppHandle) -> Result<BinaryStatus, String> {
    let binaries_dir = get_binaries_dir(&app)?;

    let mut missing = Vec::new();

    for file in REQUIRED_BINARIES {
        let path = binaries_dir.join(file);
        if !path.exists() {
            missing.push(file.to_string());
        }
    }

    // Also check for the wrapper script
    let wrapper = binaries_dir.join("llama-server-wrapper-x86_64-unknown-linux-gnu");
    if !wrapper.exists() {
        missing.push("llama-server-wrapper-x86_64-unknown-linux-gnu".to_string());
    }

    Ok(BinaryStatus {
        available: missing.is_empty(),
        missing_files: missing,
    })
}

/// Download llama.cpp binaries from GitHub releases
#[command]
pub async fn download_llama_binaries(
    app: AppHandle,
    channel: Channel<DownloadProgress>,
) -> Result<(), String> {
    let binaries_dir = get_binaries_dir(&app)?;

    // Ensure binaries directory exists
    std::fs::create_dir_all(&binaries_dir)
        .map_err(|e| format!("Failed to create binaries directory: {}", e))?;

    // llama.cpp release info
    let release_tag = "b4967";
    let archive_name = format!("llama-{}-bin-ubuntu-x64.zip", release_tag);
    let download_url = format!(
        "https://github.com/ggerganov/llama.cpp/releases/download/{}/{}",
        release_tag, archive_name
    );

    channel
        .send(DownloadProgress {
            status: "Downloading llama.cpp binaries...".to_string(),
            current: 0,
            total: 0,
            done: false,
            error: None,
        })
        .ok();

    log::info!("Downloading llama.cpp from: {}", download_url);

    // Download the archive
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
    let temp_path = binaries_dir.join(&archive_name);

    // Download with progress
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

    // Extract the archive
    log::info!("Extracting archive to: {:?}", binaries_dir);

    let file =
        std::fs::File::open(&temp_path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read archive entry: {}", e))?;

        let name = file.name().to_string();

        // We're interested in specific files from the archive
        // The archive structure is: build/bin/llama-server, build/lib/libllama.so, etc.
        let extract_name: Option<String> =
            if name.ends_with("llama-server") && !name.contains("llama-server-") {
                Some("llama-server-x86_64-unknown-linux-gnu".to_string())
            } else if name.ends_with("libllama.so") {
                Some("libllama.so".to_string())
            } else if name.ends_with("libggml.so") {
                Some("libggml.so".to_string())
            } else if name.ends_with("libggml-base.so") {
                Some("libggml-base.so".to_string())
            } else if name.ends_with("libggml-cpu.so") {
                // Skip CPU-specific variants, use the base one
                None
            } else if name.contains("libggml-") && name.ends_with(".so") {
                // Extract other ggml libraries (vulkan, cuda, etc.)
                std::path::Path::new(&name)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            } else {
                None
            };

        if let Some(ref dest_name) = extract_name {
            let dest_path = binaries_dir.join(dest_name);
            let mut dest_file = std::fs::File::create(&dest_path)
                .map_err(|e| format!("Failed to create file {}: {}", dest_name, e))?;

            std::io::copy(&mut file, &mut dest_file)
                .map_err(|e| format!("Failed to extract {}: {}", dest_name, e))?;

            // Make executable
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

            log::info!("Extracted: {} -> {:?}", name, dest_path);
        }
    }

    // Create the wrapper script
    let wrapper_path = binaries_dir.join("llama-server-wrapper-x86_64-unknown-linux-gnu");
    let wrapper_content = r#"#!/bin/bash
# Wrapper script for llama-server that sets up LD_LIBRARY_PATH

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export LD_LIBRARY_PATH="${SCRIPT_DIR}:${LD_LIBRARY_PATH}"

# Check for CUDA device request
for arg in "$@"; do
    if [[ "$arg" == CUDA* ]]; then
        if [[ -d "${SCRIPT_DIR}/cuda" ]]; then
            export LD_LIBRARY_PATH="${SCRIPT_DIR}/cuda:${LD_LIBRARY_PATH}"
        fi
        break
    fi
done

exec "${SCRIPT_DIR}/llama-server-x86_64-unknown-linux-gnu" "$@"
"#;

    std::fs::write(&wrapper_path, wrapper_content)
        .map_err(|e| format!("Failed to write wrapper script: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&wrapper_path)
            .map_err(|e| format!("Failed to get wrapper metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&wrapper_path, perms)
            .map_err(|e| format!("Failed to set wrapper permissions: {}", e))?;
    }

    // Clean up the archive
    let _ = std::fs::remove_file(&temp_path);

    channel
        .send(DownloadProgress {
            status: "Complete".to_string(),
            current: total_size,
            total: total_size,
            done: true,
            error: None,
        })
        .ok();

    log::info!("llama.cpp binaries downloaded and extracted successfully");
    Ok(())
}

/// Check if Ollama binary is available in our managed location
#[command]
pub async fn check_ollama_binary(app: AppHandle) -> Result<BinaryStatus, String> {
    // First check system PATH (already installed by user)
    if which::which("ollama").is_ok() {
        return Ok(BinaryStatus {
            available: true,
            missing_files: vec![],
        });
    }

    // Check our managed binaries directory
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

/// Download Ollama binary from GitHub releases
#[command]
pub async fn download_ollama_binary(
    app: AppHandle,
    channel: Channel<DownloadProgress>,
) -> Result<(), String> {
    let binaries_dir = get_download_binaries_dir(&app)?;

    // Ensure binaries directory exists
    std::fs::create_dir_all(&binaries_dir)
        .map_err(|e| format!("Failed to create binaries directory: {}", e))?;

    // Ollama release info - uses tar.zst format
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

    // Download the archive
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

    // Download with progress
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

    // Extract the tar.zst archive
    log::info!("Extracting Ollama archive to: {:?}", binaries_dir);

    let file =
        std::fs::File::open(&temp_path).map_err(|e| format!("Failed to open archive: {}", e))?;

    // Decompress zstd
    let decoder =
        zstd::Decoder::new(file).map_err(|e| format!("Failed to create zstd decoder: {}", e))?;

    // Extract tar
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

        // We're looking for the main ollama binary
        // The archive structure is typically: bin/ollama
        if path_str.ends_with("/ollama") || path_str == "ollama" {
            let dest_path = binaries_dir.join("ollama");

            // Extract to destination
            let mut dest_file = std::fs::File::create(&dest_path)
                .map_err(|e| format!("Failed to create ollama binary: {}", e))?;

            std::io::copy(&mut entry, &mut dest_file)
                .map_err(|e| format!("Failed to extract ollama: {}", e))?;

            // Make executable
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

    // Clean up the archive
    let _ = std::fs::remove_file(&temp_path);

    // Verify extraction
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
