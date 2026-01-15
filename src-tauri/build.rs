use std::fs;
use std::path::Path;

fn main() {
    // Ensure wrapper script is always copied from the canonical source (binaries/)
    // to the target directory. This prevents stale copies from causing issues.
    copy_binaries_to_target();

    tauri_build::build();
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
        if let Err(e) = fs::copy(&src_wrapper, &dst_wrapper) {
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
        if let Err(e) = copy_dir_recursive(&cuda_src, &cuda_dst) {
            println!("cargo:warning=Failed to copy cuda directory: {}", e);
        } else {
            println!("cargo:warning=Copied cuda directory to {:?}", cuda_dst);
        }
    }

    // Trigger rebuild when binaries change
    println!("cargo:rerun-if-changed=binaries/llama-server-wrapper-x86_64-unknown-linux-gnu");
    println!("cargo:rerun-if-changed=binaries/cuda/llama-server");
    println!("cargo:rerun-if-changed=binaries/cuda/libggml-cuda.so");
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
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
