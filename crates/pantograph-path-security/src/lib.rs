//! Shared allowed-root path validation for filesystem boundaries.
//!
//! External path strings and persisted path records must be resolved here
//! before file or process access.

use std::fmt;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AllowedRootPathError {
    EmptyPath,
    ParentTraversal,
    RootResolutionFailed {
        root: PathBuf,
        message: String,
    },
    PathResolutionFailed {
        path: PathBuf,
        message: String,
    },
    NoExistingAncestor {
        path: PathBuf,
    },
    OutsideRoot {
        path: PathBuf,
        allowed_root: PathBuf,
    },
}

impl fmt::Display for AllowedRootPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPath => formatter.write_str("path is empty"),
            Self::ParentTraversal => formatter.write_str("path traversal ('..') is not allowed"),
            Self::RootResolutionFailed { root, message } => write!(
                formatter,
                "failed to resolve allowed root '{}': {}",
                root.display(),
                message
            ),
            Self::PathResolutionFailed { path, message } => write!(
                formatter,
                "failed to canonicalize path '{}': {}",
                path.display(),
                message
            ),
            Self::NoExistingAncestor { path } => write!(
                formatter,
                "path '{}' has no existing ancestor to validate",
                path.display()
            ),
            Self::OutsideRoot { path, allowed_root } => write!(
                formatter,
                "path '{}' resolves outside allowed root '{}'",
                path.display(),
                allowed_root.display()
            ),
        }
    }
}

impl std::error::Error for AllowedRootPathError {}

/// Resolve an untrusted path string and ensure it stays within `allowed_root`.
///
/// This trims string input before parsing so boundary DTOs cannot smuggle
/// whitespace-only paths through as relative file names.
pub fn resolve_external_path_within_root(
    input_path: &str,
    allowed_root: &Path,
) -> Result<PathBuf, AllowedRootPathError> {
    let raw = input_path.trim();
    if raw.is_empty() {
        return Err(AllowedRootPathError::EmptyPath);
    }

    resolve_path_within_root(Path::new(raw), allowed_root)
}

/// Resolve a path and ensure it stays within `allowed_root`.
///
/// - Rejects empty paths.
/// - Rejects explicit parent traversal segments (`..`).
/// - Allows absolute paths only when they resolve inside `allowed_root`.
/// - Checks symlink escapes by canonicalizing either the target when it exists
///   or the nearest existing ancestor when the final path does not exist yet.
pub fn resolve_path_within_root(
    input_path: &Path,
    allowed_root: &Path,
) -> Result<PathBuf, AllowedRootPathError> {
    if input_path.as_os_str().is_empty() {
        return Err(AllowedRootPathError::EmptyPath);
    }

    if input_path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(AllowedRootPathError::ParentTraversal);
    }

    let canonical_root = allowed_root.canonicalize().map_err(|error| {
        AllowedRootPathError::RootResolutionFailed {
            root: allowed_root.to_path_buf(),
            message: error.to_string(),
        }
    })?;

    let candidate = if input_path.is_absolute() {
        input_path.to_path_buf()
    } else {
        canonical_root.join(input_path)
    };

    if candidate.exists() {
        let canonical_candidate = candidate.canonicalize().map_err(|error| {
            AllowedRootPathError::PathResolutionFailed {
                path: candidate.clone(),
                message: error.to_string(),
            }
        })?;
        if canonical_candidate.starts_with(&canonical_root) {
            return Ok(canonical_candidate);
        }
        return Err(AllowedRootPathError::OutsideRoot {
            path: canonical_candidate,
            allowed_root: canonical_root,
        });
    }

    let existing_ancestor = nearest_existing_ancestor(&candidate).ok_or_else(|| {
        AllowedRootPathError::NoExistingAncestor {
            path: candidate.clone(),
        }
    })?;
    let canonical_ancestor = existing_ancestor.canonicalize().map_err(|error| {
        AllowedRootPathError::PathResolutionFailed {
            path: existing_ancestor.clone(),
            message: error.to_string(),
        }
    })?;
    if !canonical_ancestor.starts_with(&canonical_root) {
        return Err(AllowedRootPathError::OutsideRoot {
            path: candidate,
            allowed_root: canonical_root,
        });
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

        let resolved = resolve_external_path_within_root("a/file.txt", root).expect("valid path");
        assert!(resolved.starts_with(root.canonicalize().expect("canonical root")));
    }

    #[test]
    fn rejects_blank_string_paths() {
        let dir = tempdir().expect("tempdir");
        let error =
            resolve_external_path_within_root("  ", dir.path()).expect_err("blank is invalid");

        assert_eq!(error, AllowedRootPathError::EmptyPath);
    }

    #[test]
    fn rejects_parent_dir_traversal() {
        let dir = tempdir().expect("tempdir");
        let error = resolve_external_path_within_root("../etc/passwd", dir.path())
            .expect_err("must reject");

        assert_eq!(error, AllowedRootPathError::ParentTraversal);
    }

    #[test]
    fn rejects_absolute_path_outside_root() {
        let dir = tempdir().expect("tempdir");
        let error = resolve_external_path_within_root("/tmp/definitely-outside-root", dir.path())
            .expect_err("must reject");

        assert!(matches!(error, AllowedRootPathError::OutsideRoot { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = tempdir().expect("tempdir");
        let outside = tempdir().expect("tempdir");
        symlink(outside.path(), root.path().join("link")).expect("symlink");

        let error =
            resolve_external_path_within_root("link/secret.txt", root.path()).expect_err("reject");
        assert!(matches!(error, AllowedRootPathError::OutsideRoot { .. }));
    }
}
