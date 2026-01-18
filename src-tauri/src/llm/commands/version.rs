//! Component versioning commands for undo/redo functionality.
//!
//! Uses an isolated git repository in src/generated/ to track component changes.
//! This allows undo/redo without affecting the main project's git repository.
//!
//! Navigation is non-destructive: we use `git checkout` to move between commits
//! while keeping all commits intact. Two tracking files in .git/ track position:
//! - PANTOGRAPH_HEAD: Current checkout position (commit hash)
//! - PANTOGRAPH_TIP: Latest commit in history (for redo limit)

use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use tauri::command;

use super::shared::get_project_root;

/// Result of an undo/redo operation
#[derive(Debug, Serialize)]
pub struct VersionResult {
    pub success: bool,
    pub message: String,
    /// Path of the affected file (if any)
    pub affected_file: Option<String>,
}

/// Entry in the component history
#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    pub hash: String,
    pub message: String,
    pub timestamp: Option<String>,
}

/// Read a tracking file from .git directory, returning None if not found
fn read_tracking_file(generated_dir: &PathBuf, filename: &str) -> Option<String> {
    let path = generated_dir.join(".git").join(filename);
    std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Write a tracking file to .git directory
fn write_tracking_file(generated_dir: &PathBuf, filename: &str, value: &str) -> Result<(), String> {
    let path = generated_dir.join(".git").join(filename);
    std::fs::write(&path, value).map_err(|e| format!("Failed to write {}: {}", filename, e))
}

/// Get the current HEAD commit hash from git
fn get_git_head(generated_dir: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(generated_dir)
        .output()
        .map_err(|e| format!("Failed to get HEAD: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err("Failed to get HEAD commit".to_string())
    }
}

/// Get the current position (PANTOGRAPH_HEAD or fallback to git HEAD)
fn get_current_position(generated_dir: &PathBuf) -> Result<String, String> {
    // Try PANTOGRAPH_HEAD first, fallback to git HEAD
    if let Some(pos) = read_tracking_file(generated_dir, "PANTOGRAPH_HEAD") {
        return Ok(pos);
    }
    get_git_head(generated_dir)
}

/// Get the commit message for a given commit hash
fn get_commit_message(generated_dir: &PathBuf, commit: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%s", commit])
        .current_dir(generated_dir)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Get the parent commit of a given commit (returns None if at root)
fn get_parent_commit(generated_dir: &PathBuf, commit: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", &format!("{}^", commit)])
        .current_dir(generated_dir)
        .output()
        .ok()?;

    if output.status.success() {
        let parent = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !parent.is_empty() {
            return Some(parent);
        }
    }
    None
}

/// Checkout files from a specific commit (non-destructive)
fn checkout_commit_files(generated_dir: &PathBuf, commit: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["checkout", commit, "--", "."])
        .current_dir(generated_dir)
        .output()
        .map_err(|e| format!("Failed to checkout: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Checkout failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Sync the working directory to exactly match a commit's state.
/// This handles both file content updates AND file deletions.
/// git checkout alone doesn't delete files that were added in later commits.
fn sync_working_dir_to_commit(generated_dir: &PathBuf, commit: &str) -> Result<(), String> {
    // Get list of files that should exist at this commit
    let output = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", commit])
        .current_dir(generated_dir)
        .output()
        .map_err(|e| format!("Failed to list files at commit: {}", e))?;

    let target_files: HashSet<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(String::from)
        .collect();

    // Walk the generated directory and delete files not in target commit
    fn collect_files(dir: &PathBuf, base: &PathBuf, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    // Skip .git directory
                    if path.file_name().map(|n| n == ".git").unwrap_or(false) {
                        continue;
                    }
                    collect_files(&path, base, files);
                } else if path.is_file() {
                    files.push(path);
                }
            }
        }
    }

    let mut current_files: Vec<PathBuf> = Vec::new();
    collect_files(generated_dir, generated_dir, &mut current_files);

    // Delete files that shouldn't exist at the target commit
    for file_path in current_files {
        if let Ok(rel_path) = file_path.strip_prefix(generated_dir) {
            let rel_str = rel_path.to_string_lossy().replace('\\', "/");

            // Skip protected files
            if rel_str == ".gitkeep" || rel_str == ".gitignore" {
                continue;
            }

            // Delete if not in target commit
            if !target_files.contains(&rel_str) {
                if let Err(e) = std::fs::remove_file(&file_path) {
                    log::warn!("Failed to delete file {:?}: {}", file_path, e);
                } else {
                    log::debug!("Deleted file not in target commit: {}", rel_str);
                }
            }
        }
    }

    // Clean up empty directories (except the root)
    fn remove_empty_dirs(dir: &PathBuf, is_root: bool) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            for entry in &entries {
                let path = entry.path();
                if path.is_dir() {
                    if path.file_name().map(|n| n == ".git").unwrap_or(false) {
                        continue;
                    }
                    remove_empty_dirs(&path, false);
                }
            }
            // After processing children, check if this dir is now empty
            if !is_root {
                if let Ok(remaining) = std::fs::read_dir(dir) {
                    if remaining.count() == 0 {
                        let _ = std::fs::remove_dir(dir);
                    }
                }
            }
        }
    }
    remove_empty_dirs(generated_dir, true);

    // Now checkout to restore file contents
    checkout_commit_files(generated_dir, commit)?;

    Ok(())
}

/// Extract file path from commit message (format: "Create/Update path.svelte")
fn extract_affected_file(message: &str) -> Option<String> {
    message.split_whitespace().last().map(String::from)
}

/// Undo the last component change (non-destructive)
#[command]
pub async fn undo_component_change() -> Result<VersionResult, String> {
    let project_root = get_project_root()?;
    let generated_dir = project_root.join("src").join("generated");

    // Check if git repo exists
    if !generated_dir.join(".git").exists() {
        return Ok(VersionResult {
            success: false,
            message: "No version history available".to_string(),
            affected_file: None,
        });
    }

    // Get current position
    let current = get_current_position(&generated_dir)?;

    // Get the commit message before undoing (to know what was undone)
    let current_message = get_commit_message(&generated_dir, &current)
        .unwrap_or_else(|| "Unknown".to_string());

    // Get parent commit
    let parent = match get_parent_commit(&generated_dir, &current) {
        Some(p) => p,
        None => {
            return Ok(VersionResult {
                success: false,
                message: "Nothing to undo - at the beginning of history".to_string(),
                affected_file: None,
            });
        }
    };

    // Sync working directory to parent commit (handles both updates and deletions)
    sync_working_dir_to_commit(&generated_dir, &parent)?;

    // Update PANTOGRAPH_HEAD to parent
    write_tracking_file(&generated_dir, "PANTOGRAPH_HEAD", &parent)?;

    // Ensure PANTOGRAPH_TIP is set (preserve it if already set)
    if read_tracking_file(&generated_dir, "PANTOGRAPH_TIP").is_none() {
        // First undo ever - set TIP to where we were
        write_tracking_file(&generated_dir, "PANTOGRAPH_TIP", &current)?;
    }

    let affected_file = extract_affected_file(&current_message);

    Ok(VersionResult {
        success: true,
        message: format!("Undone: {}", current_message),
        affected_file,
    })
}

/// Redo the last undone component change (non-destructive)
#[command]
pub async fn redo_component_change() -> Result<VersionResult, String> {
    let project_root = get_project_root()?;
    let generated_dir = project_root.join("src").join("generated");

    // Check if git repo exists
    if !generated_dir.join(".git").exists() {
        return Ok(VersionResult {
            success: false,
            message: "No version history available".to_string(),
            affected_file: None,
        });
    }

    // Get current position and tip
    let current = get_current_position(&generated_dir)?;
    let tip = match read_tracking_file(&generated_dir, "PANTOGRAPH_TIP") {
        Some(t) => t,
        None => {
            // No TIP means we've never undone, so nothing to redo
            return Ok(VersionResult {
                success: false,
                message: "Nothing to redo".to_string(),
                affected_file: None,
            });
        }
    };

    // If current == tip, nothing to redo
    if current == tip {
        return Ok(VersionResult {
            success: false,
            message: "Nothing to redo - already at the latest".to_string(),
            affected_file: None,
        });
    }

    // Find the next commit toward the tip
    // git rev-list --ancestry-path returns commits from current to tip (exclusive of current)
    // The last one in the list is the immediate child (next commit toward tip)
    let output = Command::new("git")
        .args(["rev-list", "--ancestry-path", &format!("{}..{}", current, tip)])
        .current_dir(&generated_dir)
        .output()
        .map_err(|e| format!("Failed to find redo path: {}", e))?;

    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    let commits: Vec<&str> = output_str.trim().lines().collect();

    if commits.is_empty() {
        return Ok(VersionResult {
            success: false,
            message: "Nothing to redo - no path to tip".to_string(),
            affected_file: None,
        });
    }

    // Last commit in the list is the immediate child (closest to current)
    // Safe to use expect here since we checked is_empty() above
    let Some(last_commit) = commits.last() else {
        return Ok(VersionResult {
            success: false,
            message: "Nothing to redo - no path to tip".to_string(),
            affected_file: None,
        });
    };
    let next_commit = last_commit.trim().to_string();

    // Get the commit message for the next commit
    let next_message = get_commit_message(&generated_dir, &next_commit)
        .unwrap_or_else(|| "Unknown".to_string());

    // Sync working directory to next commit (handles both updates and new files)
    sync_working_dir_to_commit(&generated_dir, &next_commit)?;

    // Update PANTOGRAPH_HEAD to next commit
    write_tracking_file(&generated_dir, "PANTOGRAPH_HEAD", &next_commit)?;

    let affected_file = extract_affected_file(&next_message);

    Ok(VersionResult {
        success: true,
        message: format!("Redone: {}", next_message),
        affected_file,
    })
}

/// Get the version history for components
#[command]
pub async fn get_component_history(
    path: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<HistoryEntry>, String> {
    let project_root = get_project_root()?;
    let generated_dir = project_root.join("src").join("generated");

    // Check if git repo exists
    if !generated_dir.join(".git").exists() {
        return Ok(vec![]);
    }

    let limit_str = limit.unwrap_or(20).to_string();
    let mut args = vec!["log", "--oneline", "-n", &limit_str, "--format=%H|%s|%cr"];

    // If a path is specified, show history for that file only
    let path_owned;
    if let Some(p) = &path {
        args.push("--");
        path_owned = p.clone();
        args.push(&path_owned);
    }

    let output = Command::new("git")
        .args(&args)
        .current_dir(&generated_dir)
        .output()
        .map_err(|e| format!("Failed to get history: {}", e))?;

    let history = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() >= 2 {
                Some(HistoryEntry {
                    hash: parts[0].to_string(),
                    message: parts[1].to_string(),
                    timestamp: parts.get(2).map(|s| s.to_string()),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(history)
}

/// Check how many redo steps are available
#[command]
pub async fn get_redo_count() -> Result<u32, String> {
    let project_root = get_project_root()?;
    let generated_dir = project_root.join("src").join("generated");

    if !generated_dir.join(".git").exists() {
        return Ok(0);
    }

    // Get current position and tip
    let current = match get_current_position(&generated_dir) {
        Ok(c) => c,
        Err(_) => return Ok(0),
    };

    let tip = match read_tracking_file(&generated_dir, "PANTOGRAPH_TIP") {
        Some(t) => t,
        None => return Ok(0), // No tip means nothing to redo
    };

    if current == tip {
        return Ok(0);
    }

    // Count commits between current and tip
    let output = Command::new("git")
        .args(["rev-list", "--count", &format!("{}..{}", current, tip)])
        .current_dir(&generated_dir)
        .output()
        .map_err(|e| format!("Failed to count redo commits: {}", e))?;

    let count_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let count: u32 = count_str.parse().unwrap_or(0);

    Ok(count)
}

/// Update tracking files after a new commit (called from write.rs)
pub fn update_tracking_after_commit(generated_dir: &PathBuf) {
    // Get the new HEAD commit
    if let Ok(new_head) = get_git_head(generated_dir) {
        // Update both HEAD and TIP to the new commit
        // This "forks" history if we were in a rewound state
        let _ = write_tracking_file(generated_dir, "PANTOGRAPH_HEAD", &new_head);
        let _ = write_tracking_file(generated_dir, "PANTOGRAPH_TIP", &new_head);
    }
}

/// Information about a generated component file
#[derive(Debug, Serialize)]
pub struct GeneratedComponentInfo {
    /// Relative path from src/generated/ (e.g., "modals/MyModal.svelte")
    pub path: String,
    /// Full file content
    pub content: String,
}

/// List all .svelte files in the generated directory
/// Used to restore workspace on app startup
#[command]
pub async fn list_generated_components() -> Result<Vec<GeneratedComponentInfo>, String> {
    let project_root = get_project_root()?;
    let generated_dir = project_root.join("src").join("generated");

    if !generated_dir.exists() {
        return Ok(vec![]);
    }

    let mut components = Vec::new();
    collect_svelte_files(&generated_dir, &generated_dir, &mut components)?;

    Ok(components)
}

/// Recursively collect .svelte files from a directory
fn collect_svelte_files(
    base_dir: &PathBuf,
    current_dir: &PathBuf,
    components: &mut Vec<GeneratedComponentInfo>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(current_dir)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();

        // Skip hidden files/directories (like .git, .gitkeep)
        if path
            .file_name()
            .map(|n| n.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }

        if path.is_dir() {
            collect_svelte_files(base_dir, &path, components)?;
        } else if path.extension().map(|e| e == "svelte").unwrap_or(false) {
            // Get relative path from base_dir
            let relative_path = path
                .strip_prefix(base_dir)
                .map_err(|e| format!("Failed to get relative path: {}", e))?
                .to_string_lossy()
                .replace('\\', "/"); // Normalize for cross-platform

            // Read file content
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read file {}: {}", relative_path, e))?;

            components.push(GeneratedComponentInfo {
                path: relative_path,
                content,
            });
        }
    }

    Ok(())
}
