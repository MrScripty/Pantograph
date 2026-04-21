use crate::llm::commands::version::{
    generated_history_exists, generated_history_git_dir, git_for_generated_history,
    migrate_legacy_generated_history, update_tracking_after_commit,
};
use std::path::PathBuf;

/// Initialize generated component history if it does not exist.
pub fn ensure_git_repo(generated_dir: &PathBuf) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(generated_dir)?;
    migrate_legacy_generated_history(generated_dir)?;

    let git_dir = generated_history_git_dir(generated_dir);
    if !generated_history_exists(generated_dir) {
        if let Some(parent) = git_dir.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let output = std::process::Command::new("git")
            .args(["init", "--separate-git-dir"])
            .arg(&git_dir)
            .arg(generated_dir)
            .output()?;

        if output.status.success() {
            let generated_git_marker = generated_dir.join(".git");
            if generated_git_marker.is_file() {
                std::fs::remove_file(generated_git_marker)?;
            }

            log::info!(
                "[write_gui_file] Initialized generated component history at {:?}",
                git_dir
            );

            let gitignore_path = generated_dir.join(".gitignore");
            if !gitignore_path.exists() {
                std::fs::write(&gitignore_path, "# Temporary validation files\n*.tmp\n")?;
            }

            let _ = git_for_generated_history(generated_dir)
                .args([
                    "commit",
                    "-m",
                    "Initialize generated components",
                    "--allow-empty",
                ])
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

    let stage_result = git_for_generated_history(generated_dir)
        .args(["add", path])
        .output();
    if let Err(e) = stage_result {
        log::warn!("[write_gui_file] Failed to stage file: {}", e);
        return;
    }

    let action = if is_new { "Create" } else { "Update" };
    let message = format!("{} {}", action, path);
    let commit_result = git_for_generated_history(generated_dir)
        .args(["commit", "-m", &message])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_git_repo_initializes_external_history_without_nested_marker() {
        let temp = tempfile::tempdir().expect("tempdir");
        let generated_dir = temp.path().join("src").join("generated");

        ensure_git_repo(&generated_dir).expect("ensure generated history");

        assert!(generated_history_git_dir(&generated_dir).exists());
        assert!(!generated_dir.join(".git").exists());
    }
}
