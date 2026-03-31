use std::path::{Path, PathBuf};

fn looks_like_project_root(path: &Path) -> bool {
    path.join("Cargo.toml").is_file() && path.join("src-tauri").join("Cargo.toml").is_file()
}

fn find_project_root_from(seed: &Path) -> Option<PathBuf> {
    let start = if seed.is_file() { seed.parent()? } else { seed };

    for candidate in start.ancestors() {
        if looks_like_project_root(candidate) {
            return Some(candidate.to_path_buf());
        }
    }

    None
}

pub fn resolve_project_root() -> Result<PathBuf, String> {
    let mut seeds = Vec::new();

    if let Some(path) = std::env::var_os("PANTOGRAPH_PROJECT_ROOT") {
        seeds.push(PathBuf::from(path));
    }

    if let Ok(exe_path) = std::env::current_exe() {
        seeds.push(exe_path);
    }

    if let Ok(current_dir) = std::env::current_dir() {
        seeds.push(current_dir);
    }

    seeds.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    for seed in seeds {
        if let Some(project_root) = find_project_root_from(&seed) {
            return Ok(project_root);
        }
    }

    Err("Failed to resolve Pantograph project root from runtime paths".to_string())
}
