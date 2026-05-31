# dependency_environment

This directory contains helper modules for embedded runtime dependency
preflight gates.

`helpers.rs` still owns stable runtime environment key helpers used by Python
runtime metadata. Its legacy dependency-preflight request projection helper
has been deleted; production retired runtime preflight must fail closed before
`ModelDependencyRequest`, `ModelDependencyResolver`, `ModelRefV2`, path repair,
or Python adapter dispatch. The preflight gate returns only success/diagnostic
failure, so it cannot inject a legacy `model_ref` payload into Python runtime
inputs. The old private `build_model_dependency_request` helper must not be
recreated as a Python-runtime compatibility bridge.

Dependency-environment actions are resolved by workflow-service through the
dependency environment service; this module must not rebuild
`DependencyEnvironmentRequest`, emit `environment_ref`, or execute check/install
actions.
