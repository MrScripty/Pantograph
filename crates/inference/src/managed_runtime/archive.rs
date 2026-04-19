use super::contracts::ArchiveKind;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

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
    extract_tar_archive_entries(&mut archive, destination, "tar.gz")
}

fn extract_tar_zst_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let archive_file =
        fs::File::open(archive_path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let decoder = zstd::Decoder::new(archive_file)
        .map_err(|e| format!("Failed to create zstd decoder: {}", e))?;
    let mut archive = tar::Archive::new(decoder);
    extract_tar_archive_entries(&mut archive, destination, "tar.zst")
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
        let output_path = validated_output_path(destination, &relative_path)?;

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

fn extract_tar_archive_entries<R: Read>(
    archive: &mut tar::Archive<R>,
    destination: &Path,
    archive_label: &str,
) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|e| format!("Failed to create {:?}: {}", destination, e))?;

    let entries = archive
        .entries()
        .map_err(|e| format!("Failed to read {} archive entries: {}", archive_label, e))?;

    for entry in entries {
        let mut entry =
            entry.map_err(|e| format!("Failed to read {} archive entry: {}", archive_label, e))?;
        let relative_path = entry
            .path()
            .map_err(|e| format!("Failed to inspect {} entry path: {}", archive_label, e))?
            .to_path_buf();
        let _ = validated_output_path(destination, &relative_path)?;
        entry.unpack_in(destination).map_err(|e| {
            format!(
                "Failed to unpack {} archive entry {:?}: {}",
                archive_label, relative_path, e
            )
        })?;
    }

    Ok(())
}

fn validated_output_path(destination: &Path, relative_path: &Path) -> Result<PathBuf, String> {
    if relative_path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "Archive entry escapes extraction root: {}",
            relative_path.display()
        ));
    }

    Ok(destination.join(relative_path))
}

#[cfg(unix)]
fn zip_entry_is_symlink(entry: &zip::read::ZipFile<'_>) -> bool {
    entry
        .unix_mode()
        .map(|mode| (mode & 0o170000) == 0o120000)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::validated_output_path;
    use std::path::Path;

    #[test]
    fn validated_output_path_accepts_normal_relative_entries() {
        let output = validated_output_path(Path::new("/tmp/runtime"), Path::new("bin/server"))
            .expect("valid output path");

        assert_eq!(output, Path::new("/tmp/runtime").join("bin/server"));
    }

    #[test]
    fn validated_output_path_rejects_parent_dir_escapes() {
        let error = validated_output_path(Path::new("/tmp/runtime"), Path::new("../server"))
            .expect_err("parent dir should be rejected");

        assert!(error.contains("escapes extraction root"));
    }
}
