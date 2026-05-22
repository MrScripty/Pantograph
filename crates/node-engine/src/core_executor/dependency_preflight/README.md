# dependency_preflight

This directory contains helper modules for node-engine dependency preflight.

`input_projection.rs` owns the current graph-input projection helpers used by
preflight and model-ref assembly. It does not own dependency-planning contracts,
Pumas artifact lookup, scheduler policy, or worker load-target handoff. Those
contracts live in `pantograph-dependency-planning` and later migration slices
must replace the legacy projection helpers rather than adding compatibility
aliases around them.
