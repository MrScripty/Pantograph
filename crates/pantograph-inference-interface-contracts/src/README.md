# Source Layout

`lib.rs` curates the DTO-only inference interface contract facade and public
re-exports. `validation.rs` owns shared contract-version checks, bounded
validation helpers, validated identifier newtypes, and the crate error type.
The crate intentionally stays behavior-light: it validates bounds, required
fields, version markers, and dependency-planning model references, but it does
not resolve Pumas facts, select schedulers, execute inference, or mutate graphs.

`DependencyEnvironmentActionIntent` is the frontend/workflow-service action
contract for descriptor-backed dependency-environment resolve/check/install
requests. It carries only graph-session identity, graph revision,
optional validation-session identity, target node id, and the typed action. The
workflow-service derives any `DependencyEnvironmentRequest` from current
validation state; graph editor and Tauri callers must not send paths, Pumas
facts, platform context, identity keys, dependency planning requests, or
environment requests. `DependencyEnvironmentActionIntentResult` is the
fail-closed response envelope used before a canonical environment request can
be derived; blocked results must carry typed diagnostics. Sidecar association
failures use explicit `DependencySidecar*` diagnostic codes so callers do not
parse messages or infer legacy path-based subjects.
