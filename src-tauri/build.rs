use std::fs;
use std::io::Read;
use std::path::Path;

fn main() {
    // Ensure wrapper script is always copied from the canonical source (binaries/)
    // to the target directory. This prevents stale copies from causing issues.
    copy_binaries_to_target();

    tauri_build::build();
}

/// Quick hash of first 8KB + last 8KB + file size for fast comparison
fn quick_file_hash(path: &Path) -> Option<u64> {
    let metadata = fs::metadata(path).ok()?;
    let size = metadata.len();

    let mut file = fs::File::open(path).ok()?;
    let mut hasher: u64 = size;

    // Read first 8KB
    let mut buf = [0u8; 8192];
    let n = file.read(&mut buf).ok()?;
    for byte in &buf[..n] {
        hasher = hasher.wrapping_mul(31).wrapping_add(*byte as u64);
    }

    // Read last 8KB if file is large enough
    if size > 16384 {
        use std::io::Seek;
        file.seek(std::io::SeekFrom::End(-8192)).ok()?;
        let n = file.read(&mut buf).ok()?;
        for byte in &buf[..n] {
            hasher = hasher.wrapping_mul(31).wrapping_add(*byte as u64);
        }
    }

    Some(hasher)
}

/// Check if two files are identical using quick hash
fn files_match(src: &Path, dst: &Path) -> bool {
    if !dst.exists() {
        return false;
    }

    // Quick check: sizes must match
    let src_size = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
    let dst_size = fs::metadata(dst).map(|m| m.len()).unwrap_or(1);
    if src_size != dst_size {
        return false;
    }

    // Compare hashes
    match (quick_file_hash(src), quick_file_hash(dst)) {
        (Some(h1), Some(h2)) => h1 == h2,
        _ => false,
    }
}

fn copy_binaries_to_target() {
    let binaries_dir = Path::new("binaries");

    // Get the target directory - during build, OUT_DIR is something like
    // target/debug/build/<crate>-<hash>/out
    // We need to go up to target/debug/ to place the wrapper where Tauri expects it
    let out_dir = std::env::var("OUT_DIR").unwrap_or_else(|_| "target/debug".to_string());
    let out_path = Path::new(&out_dir);

    // Navigate from OUT_DIR (target/debug/build/<crate>-<hash>/out) to target/debug/
    let target_dir = out_path
        .ancestors()
        .nth(3)
        .unwrap_or(Path::new("target/debug"));

    // Copy the wrapper script
    let wrapper_name = "llama-server-wrapper-x86_64-unknown-linux-gnu";
    let src_wrapper = binaries_dir.join(wrapper_name);
    let dst_wrapper = target_dir.join(wrapper_name);

    if src_wrapper.exists() {
        if files_match(&src_wrapper, &dst_wrapper) {
            // Already up to date, skip
        } else if let Err(e) = fs::copy(&src_wrapper, &dst_wrapper) {
            println!("cargo:warning=Failed to copy wrapper script: {}", e);
        } else {
            println!("cargo:warning=Copied wrapper script to {:?}", dst_wrapper);
            // Make sure it's executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = fs::metadata(&dst_wrapper) {
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o755);
                    let _ = fs::set_permissions(&dst_wrapper, perms);
                }
            }
        }
    }

    // Copy the cuda directory if it exists
    let cuda_src = binaries_dir.join("cuda");
    let cuda_dst = target_dir.join("cuda");

    if cuda_src.exists() && cuda_src.is_dir() {
        let mut copied = 0;
        let mut skipped = 0;
        if let Err(e) = copy_dir_if_changed(&cuda_src, &cuda_dst, &mut copied, &mut skipped) {
            println!("cargo:warning=Failed to copy cuda directory: {}", e);
        } else if copied > 0 {
            println!("cargo:warning=Copied {} files to {:?} ({} unchanged)", copied, cuda_dst, skipped);
        }
        // Silent if all files were skipped (already up to date)
    }

    // Trigger rebuild when binaries change
    println!("cargo:rerun-if-changed=binaries/llama-server-wrapper-x86_64-unknown-linux-gnu");
    println!("cargo:rerun-if-changed=binaries/cuda/llama-server");
    println!("cargo:rerun-if-changed=binaries/cuda/libggml-cuda.so");
}

fn copy_dir_if_changed(src: &Path, dst: &Path, copied: &mut usize, skipped: &mut usize) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_if_changed(&src_path, &dst_path, copied, skipped)?;
        } else {
            // Skip if files match
            if files_match(&src_path, &dst_path) {
                *skipped += 1;
                continue;
            }

            fs::copy(&src_path, &dst_path)?;
            *copied += 1;

            // Make shared libraries and executables executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.ends_with(".so")
                    || name_str.contains(".so.")
                    || name_str == "llama-server"
                {
                    if let Ok(metadata) = fs::metadata(&dst_path) {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o755);
                        let _ = fs::set_permissions(&dst_path, perms);
                    }
                }
            }
        }
    }

    Ok(())
}
