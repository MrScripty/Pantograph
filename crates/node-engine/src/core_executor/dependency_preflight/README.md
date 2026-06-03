# dependency_preflight

This directory contains the remaining retired helper modules from node-engine
dependency preflight.

Production runtime launch must not enter this directory as a successful
execution path. The old node-engine preflight enforcement helper has been
deleted instead of retained as a second readiness authority. The remaining
facade rejects retired model-reference input shapes and re-exports path-free
projection helpers used by cleanup tests and canonical dependency-planning
contract work. It must not perform resolver lookup, `ModelDependencyRequest`
construction, path repair, runtime-host dispatch, compatibility acceptance, or
`ModelRefV2` output. The old `build_model_ref_v2` constructor has been deleted
and must not be recreated as a compatibility bridge. The old
`build_model_dependency_request` constructor has also been deleted; canonical
dependency planning must use `planning_projection.rs` and shared
`pantograph-dependency-planning` contracts instead of rebuilding
`ModelDependencyRequest`.

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
