# dependency_preflight

This directory contains helper modules for node-engine dependency preflight.

`input_projection.rs` owns the current graph-input projection helpers used by
preflight and model-ref assembly. It does not own dependency-planning contracts,
Pumas artifact lookup, scheduler policy, or worker load-target handoff. Those
contracts live in `pantograph-dependency-planning` and later migration slices
must replace the legacy projection helpers rather than adding compatibility
aliases around them.

`planning_projection.rs` owns the temporary node-engine projection from graph
inputs into `pantograph-dependency-planning` request contracts. It requires a
typed `pumas_model_ref` identity and rejects path-shaped identity fields; it
must not convert the shared request back into `ModelDependencyRequest` or
`ModelRefV2`.
