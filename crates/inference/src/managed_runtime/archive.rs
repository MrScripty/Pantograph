use super::contracts::ArchiveKind;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

pub(crate) fn extract_archive(
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
    let decoder = flate2::read::GzDecoder::new(archive_file);
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
    let mut archive = zip::ZipArchive::new(archive_file)
        .map_err(|e| format!("Failed to read zip archive: {}", e))?;

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
            entry
                .read_to_string(&mut target)
                .map_err(|e| format!("Failed to read symlink target: {}", e))?;
            let target_path = Path::new(target.trim())
                .file_name()
                .ok_or_else(|| format!("Invalid symlink target in {}", entry.name()))?;
            std::os::unix::fs::symlink(target_path, &output_path)
                .map_err(|e| format!("Failed to create symlink {:?}: {}", output_path, e))?;
            continue;
        }

        let mut output = fs::File::create(&output_path)
            .map_err(|e| format!("Failed to create {:?}: {}", output_path, e))?;
        io::copy(&mut entry, &mut output)
            .map_err(|e| format!("Failed to extract {:?}: {}", output_path, e))?;
    }

    Ok(())
}

#[cfg(unix)]
fn zip_entry_is_symlink(entry: &zip::read::ZipFile<'_>) -> bool {
    entry
        .unix_mode()
        .map(|mode| (mode & 0o170000) == 0o120000)
        .unwrap_or(false)
}
