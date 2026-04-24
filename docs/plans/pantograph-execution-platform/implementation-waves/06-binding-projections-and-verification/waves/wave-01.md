# Wave 01: Base API And Support-Tier Freeze

## Objective

Freeze the native Rust base API, support tiers, artifact names, and exact host
verification commands before binding workers start.

## Workers

No parallel workers. The host owns this wave.

## Required Work

- Run `../../../08-stage-start-implementation-gate.md` for Stage `06`.
- Confirm Stage `01` through Stage `05` outputs are integrated and gated.
- Freeze the Rust API surface to project.
- Reconcile support tiers with the binding-platform plan.
- Record exact C#, Python, and BEAM language-native smoke or acceptance
  commands. Unsupported lanes must be labeled unsupported or experimental.
- Assign one owner for any generated artifacts or package metadata.

## Write Set

- Stage-start report or implementation notes only.

## Verification

- Start outcome is `ready` or `ready_with_recorded_assumptions`.
- Every supported host lane has a real artifact-loading test command.
