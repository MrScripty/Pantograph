# UniFFI Capability Estimate Fixture

Headless run 37101137330 at PR #11 head
`8ac413c52b2501bc3c69310a38eb6f13e81bb2d4` passed workflow-service and frontend
HTTP adapter tests, then reached UniFFI adapter-mode qualification. The
frontend-http test configuration failed compilation at `lib_tests.rs:320`:
`WorkflowRuntimeRequirements::estimated_peak_ram_mb` no longer exists.

The canonical contract publishes typed `resource_estimates`. Host capabilities
call `estimate_memory_requirements`, which emits available zero-byte RAM/VRAM
estimates when the graph has no required models. Preserve the old zero-RAM
assertion by locating PeakRamBytes and comparing the complete available estimate,
including its zero value and empty diagnostics. An absent, unavailable, or
nonzero estimate fails the assertion. Existing limits and no-model assertions
remain unchanged; no production, feature, or memory policy change is needed.

Rustfmt and staged whitespace pass. No local Cargo build was attempted under the
shared disk constraint. Independent source review and fresh hosted UniFFI test
execution remain required. The separate unused shutdown Result warning in
`runtime.rs:209` is recorded for lifecycle/error-contract review rather than
suppressed. Native metadata, C# generation/package/quickstart and aggregate lint,
Clippy/audit qualification remain open.

Independent integrator source review accepted staged tree
`1910522f10c8fddcbe156635e0db2663e220f5fc` against capabilities.rs:313–370.
This preserves full available zero-byte RAM semantics; exact-head hosted
frontend-http feature compilation and test execution remain required. The
preceding no-default UniFFI configuration already passed all 12 tests.
