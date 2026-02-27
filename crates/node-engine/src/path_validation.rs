//! Shared path validation for untrusted file path inputs.
//!
//! All external path strings must be resolved through this module before any
//! filesystem read/write operation.

use std::path::{Component, Path, PathBuf};

/// Resolve an untrusted path string and ensure it stays within `allowed_root`.
///
/// - Rejects empty paths.
/// - Rejects explicit parent traversal segments (`..`).
/// - Allows absolute paths only when they resolve inside `allowed_root`.
/// - Checks symlink escapes by canonicalizing either the target (if it exists)
///   or the nearest existing ancestor (for paths that do not exist yet).
pub fn resolve_path_within_root(input_path: &str, allowed_root: &Path) -> Result<PathBuf, String> {
    let raw = input_path.trim();
    if raw.is_empty() {
        return Err("path is empty".to_string());
    }

    let input = Path::new(raw);
    if input
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("path traversal ('..') is not allowed".to_string());
    }

    let canonical_root = allowed_root.canonicalize().map_err(|e| {
        format!(
            "failed to resolve allowed root '{}': {e}",
            allowed_root.display()
        )
    })?;

    let candidate = if input.is_absolute() {
        input.to_path_buf()
    } else {
        canonical_root.join(input)
    };

    if candidate.exists() {
        let canonical_candidate = candidate
            .canonicalize()
            .map_err(|e| format!("failed to canonicalize path '{}': {e}", candidate.display()))?;
        if canonical_candidate.starts_with(&canonical_root) {
            return Ok(canonical_candidate);
        }
        return Err(format!(
            "path '{}' resolves outside allowed root '{}'",
            canonical_candidate.display(),
            canonical_root.display()
        ));
    }

    let existing_ancestor = nearest_existing_ancestor(&candidate).ok_or_else(|| {
        format!(
            "path '{}' has no existing ancestor to validate",
            candidate.display()
        )
    })?;
    let canonical_ancestor = existing_ancestor.canonicalize().map_err(|e| {
        format!(
            "failed to canonicalize ancestor '{}': {e}",
            existing_ancestor.display()
        )
    })?;
    if !canonical_ancestor.starts_with(&canonical_root) {
        return Err(format!(
            "path '{}' escapes allowed root '{}'",
            candidate.display(),
            canonical_root.display()
        ));
    }

    Ok(candidate)
}

fn nearest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut cursor = Some(path);
    while let Some(current) = cursor {
        if current.exists() {
            return Some(current.to_path_buf());
        }
        cursor = current.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolves_relative_path_inside_root() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("a")).expect("mkdir");
        std::fs::write(root.join("a/file.txt"), "ok").expect("write");

        let resolved = resolve_path_within_root("a/file.txt", root).expect("valid path");
        assert!(resolved.starts_with(root));
    }

    #[test]
    fn rejects_parent_dir_traversal() {
        let dir = tempdir().expect("tempdir");
        let err = resolve_path_within_root("../etc/passwd", dir.path()).expect_err("must reject");
        assert!(err.contains("traversal"));
    }

    #[test]
    fn rejects_absolute_path_outside_root() {
        let dir = tempdir().expect("tempdir");
        let err = resolve_path_within_root("/tmp/definitely-outside-root", dir.path())
            .expect_err("must reject");
        assert!(err.contains("outside") || err.contains("escapes"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = tempdir().expect("tempdir");
        let outside = tempdir().expect("tempdir");
        symlink(outside.path(), root.path().join("link")).expect("symlink");

        let err = resolve_path_within_root("link/secret.txt", root.path()).expect_err("reject");
        assert!(err.contains("escapes") || err.contains("outside"));
    }
}
