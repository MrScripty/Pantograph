use crate::llm::commands::version::update_tracking_after_commit;
use std::path::PathBuf;

/// Initialize git repo in generated folder if not exists (for versioning).
pub fn ensure_git_repo(generated_dir: &PathBuf) -> Result<(), std::io::Error> {
    let git_dir = generated_dir.join(".git");
    if !git_dir.exists() {
        std::fs::create_dir_all(generated_dir)?;
        let output = std::process::Command::new("git")
            .args(["init"])
            .current_dir(generated_dir)
            .output()?;

        if output.status.success() {
            log::info!("[write_gui_file] Initialized git repo in src/generated/");

            let gitignore_path = generated_dir.join(".gitignore");
            if !gitignore_path.exists() {
                std::fs::write(&gitignore_path, "# Temporary validation files\n*.tmp\n")?;
            }

            let _ = std::process::Command::new("git")
                .args(["add", "."])
                .current_dir(generated_dir)
                .output();
            let _ = std::process::Command::new("git")
                .args([
                    "commit",
                    "-m",
                    "Initialize generated components",
                    "--allow-empty",
                ])
                .current_dir(generated_dir)
                .output();
        } else {
            log::warn!(
                "[write_gui_file] Failed to initialize git repo: {:?}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    Ok(())
}

/// Commit the file change to git (for undo/redo support).
pub fn commit_change(generated_dir: &PathBuf, path: &str, is_new: bool) {
    if let Err(e) = ensure_git_repo(generated_dir) {
        log::warn!("[write_gui_file] Failed to ensure git repo: {}", e);
        return;
    }

    let stage_result = std::process::Command::new("git")
        .args(["add", path])
        .current_dir(generated_dir)
        .output();
    if let Err(e) = stage_result {
        log::warn!("[write_gui_file] Failed to stage file: {}", e);
        return;
    }

    let action = if is_new { "Create" } else { "Update" };
    let message = format!("{} {}", action, path);
    let commit_result = std::process::Command::new("git")
        .args(["commit", "-m", &message])
        .current_dir(generated_dir)
        .output();

    match commit_result {
        Ok(output) if output.status.success() => {
            log::info!("[write_gui_file] Git committed: {}", message);
            update_tracking_after_commit(generated_dir);
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::debug!("[write_gui_file] Git commit notice: {}", stderr);
        }
        Err(e) => {
            log::warn!("[write_gui_file] Failed to commit: {}", e);
        }
    }
}
