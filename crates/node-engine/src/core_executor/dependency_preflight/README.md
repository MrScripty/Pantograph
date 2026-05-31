# dependency_preflight

This directory contains retired helper modules for node-engine dependency
preflight.

Production runtime launch must not enter this directory as a successful
execution path. If old runtime preflight is reached, the guardrail in
`dependency_preflight.rs` fails closed before resolver lookup,
`ModelDependencyRequest` construction, path repair, runtime-host dispatch, or
`ModelRefV2` output. Its public helper now returns only success/diagnostic
failure and cannot hand a legacy model-ref payload back to callers. The
remaining helpers are retained temporarily for diagnostic tests and the planned
legacy-contract deletion slice. The old `build_model_ref_v2` constructor has
been deleted and must not be recreated as a compatibility bridge.

`input_projection.rs` contains legacy graph-input projection helpers that are
no longer allowed to feed successful node-engine runtime execution. It does
not own dependency-planning contracts, Pumas artifact lookup, scheduler policy,
or worker load-target handoff. Those contracts live in
`pantograph-dependency-planning` and later migration slices must delete or
replace these legacy projection helpers rather than adding compatibility
aliases around them.

`planning_projection.rs` owns the temporary node-engine projection from graph
inputs into `pantograph-dependency-planning` request contracts. It requires a
typed `pumas_model_ref` identity and rejects path-shaped identity fields; it
must not convert the shared request back into `ModelDependencyRequest` or
`ModelRefV2`.
