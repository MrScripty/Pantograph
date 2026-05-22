# dependency_environment

This directory contains helper modules for the embedded runtime dependency
environment task executor.

`helpers.rs` owns the current input projection, mode parsing, environment-ref
manifest emission, and stable key helper functions used by the legacy
dependency-environment execution path. It is not the shared dependency
environment contract owner. Migration slices must move successful execution to
`pantograph-dependency-planning` contracts and remove these legacy helpers
instead of wrapping them as compatibility behavior.
