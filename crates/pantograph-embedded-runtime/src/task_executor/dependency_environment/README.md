# dependency_environment

This directory contains helper modules for embedded runtime dependency
preflight gates.

`helpers.rs` owns input projection for Python-backed dependency preflight and
stable runtime environment key helpers used by Python runtime metadata.
Dependency-environment actions are resolved by workflow-service through the
dependency environment service; this module must not rebuild
`DependencyEnvironmentRequest`, emit `environment_ref`, or execute check/install
actions.
