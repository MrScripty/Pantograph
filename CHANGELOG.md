# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog.

## [Unreleased]

### Added
- Source-directory documentation coverage with `README.md` files across all active source trees.
- Tooling hooks and quality-gate scripts for linting, type checking, and tests.
- Tauri path-boundary regression tests for workflow loading, sandbox validation, and agent file tools.

### Changed
- Root project `README.md` reorganized around install, usage, development, and contribution workflows.
- Accessibility interaction semantics improved by replacing suppressed non-semantic handlers with button-based interactions.

### Fixed
- Launcher contract behavior aligned with CLI standards and expected error handling paths.

### Security
- Canonical path validation enforced at file-boundary entry points to block traversal and symlink escape paths.
