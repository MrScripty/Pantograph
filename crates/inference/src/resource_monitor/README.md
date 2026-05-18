# Inference Resource Monitor

This directory owns platform and runtime-adapter resource observation helpers
for the `inference` crate.

The public boundary is `resource_monitor::RuntimeResourceMonitor`. Callers
start a monitor at the execution shell, finish it at the same boundary, and
receive an `InferenceExecutionResourceObservation`. Scheduler policy,
candidate ranking, terminal event projection, and workflow diagnostics live in
their owning crates.

Platform-specific selection is isolated in `platform.rs` plus the platform
files. Business logic should call `default_runtime_resource_monitor()` or an
explicit monitor implementation instead of using `cfg()` directly.

The first implementation is process RSS through the existing `sysinfo`
dependency. It reports host RAM only through
`InferenceResourceObservationSourceKind::OsProcessRss`; GPU/device memory must
come from runtime-native telemetry such as PyTorch CUDA or MPS counters.
