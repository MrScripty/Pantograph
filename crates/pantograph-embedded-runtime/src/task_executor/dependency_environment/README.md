# dependency_environment

This directory contains helper modules for embedded runtime dependency
preflight gates.

`helpers.rs` still owns stable runtime environment key helpers used by Python
runtime metadata. Its legacy dependency-preflight request projection helpers
are retained only for transitional cleanup tests; production retired runtime
preflight must fail closed before `ModelDependencyRequest`,
`ModelDependencyResolver`, `ModelRefV2`, path repair, or Python adapter
dispatch.

Dependency-environment actions are resolved by workflow-service through the
dependency environment service; this module must not rebuild
`DependencyEnvironmentRequest`, emit `environment_ref`, or execute check/install
actions.
