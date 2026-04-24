# 05: Composition, Factoring, And Migration

## Purpose

Support higher-level graph usability without hiding primitive runtime facts or
breaking existing persisted workflows unnecessarily.

## Implementation Readiness Status

Ready for stage-start preflight after stages `01` through `04` are complete
and their stage-end refactor gates have been recorded.

## Required Direction

- Primitive nodes own narrow, coherent responsibilities.
- Composed nodes present stable external contracts.
- Composition must remain diagnosable in terms of internal primitive behavior.
- Model/license usage must point to the primitive model execution and preserve
  composed-parent context.
- Large nodes with unrelated knobs or mixed responsibilities are decomposition
  candidates.

## Composition Rules

Composed nodes must define:

- stable external node type id
- stable external ports
- mapping from external ports to internal primitive graph behavior
- diagnostics parent/child trace mapping
- model/license attribution behavior for internal model-producing primitives
- compatibility and migration rules for persisted graphs

## Migration Strategy

The first implementation wave should cleanly upgrade persisted workflows that
are affected by node factoring. The final system does not preserve old graph
contract behavior through indefinite backward-compatibility shims.

Required migration work:

- inventory existing node type ids and port ids
- classify coarse nodes as keep, split, or compose
- preserve stable ids only when they still describe the upgraded semantics
- define one-time migration or regeneration rules for affected graph artifacts
- remove replaced compatibility surfaces after migration
- preserve diagnostics meaning across contract upgrades

## Implementation Decisions

### Ownership Decision

- `crates/pantograph-node-contracts` owns composed-node contract semantics,
  external port mapping, compatibility decisions, and contract upgrade
  metadata.
- `crates/workflow-nodes` owns factoring concrete coarse nodes into primitive
  and composed registrations.
- `crates/pantograph-embedded-runtime` owns composed-parent lineage projection
  during execution.
- `crates/pantograph-workflow-service` owns saved-workflow migration use cases
  and calls registry/runtime APIs rather than duplicating composition rules.

### Migration Decision

- Existing saved workflows are upgraded, regenerated, or rejected with typed
  diagnostics. Compatibility is not preserved by default.
- A node or port split requires an explicit migration record with typed failure
  diagnostics for unmigratable graphs. Temporary compatibility projections are
  allowed only inside the migration path and must not remain as supported public
  semantics after the upgrade completes.
- Migration artifacts carry source contract version or digest, target contract
  version or digest, changed node/port ids, and diagnostics-lineage behavior.
- Silent behavior changes are forbidden. If behavior cannot be upgraded cleanly,
  the migration must fail with actionable diagnostics and release notes.

### Trace Decision

- Model/license usage events point to the primitive node that performed direct
  model execution and include composed-parent lineage when the primitive is
  nested under a composed node.
- Composed nodes may simplify authoring but cannot collapse primitive execution
  facts into presentation-only summaries.

## Affected Structured Contracts And Persisted Artifacts

- Node type ids, port ids, composed-node contracts, internal primitive graph
  mappings, saved workflow graphs, migration records, temporary migration
  projections, and diagnostics lineage projections.

## Standards Compliance Notes

- Architecture compliance requires composition to preserve primitive execution
  facts and avoid hiding model/license usage behind presentation-only nodes.
- Rust API compliance requires explicit migration states, compatibility
  decisions, composed-parent lineage types, and typed errors for unmigratable
  persisted graphs.
- Documentation and release compliance require migration notes or changelog
  entries for user-visible contract changes, especially removed or renamed
  nodes and ports. Notes must state that old public graph/session surfaces are
  removed after the upgrade rather than supported indefinitely.
- Testing compliance requires saved-workflow fixture upgrade tests,
  diagnostics lineage tests, model/license attribution tests through composed
  nodes, and rejection tests for graph artifacts that cannot be upgraded.
- Tooling compliance requires schema-backed or fixture validation for persisted
  workflow artifacts touched by migrations.

## Risks And Mitigations

- Risk: factoring improves authoring but loses trace fidelity. Mitigation:
  require primitive trace mapping and composed-parent lineage.
- Risk: persisted workflows silently change behavior. Mitigation: require
  explicit migration records and typed upgrade failures.
- Risk: large nodes remain because decomposition is too disruptive. Mitigation:
  classify keep/split/compose and defer only with documented upgrade rationale.

## Tasks

- Inventory existing node type ids and port ids before changing contracts.
- Define composed-node external contract rules.
- Define primitive trace mapping for composed nodes.
- Classify existing coarse inference nodes as keep, split, or compose.
- Ensure composed nodes preserve model/license attribution for internal
  primitive model execution.
- Define migration rules for persisted workflows affected by node factoring.
- Remove temporary compatibility projections after migrated artifacts are
  regenerated or upgraded.

## Intended Write Set

- Primary:
  - `crates/pantograph-node-contracts/`
  - `crates/workflow-nodes/`
- Adjacent only if required by integration:
  - `crates/pantograph-embedded-runtime/`
  - `crates/pantograph-workflow-service/`
  - saved workflow fixtures or migration fixtures
- Forbidden for this stage unless the plan is updated first:
  - host binding implementation
  - GUI redesign
  - credential/session attribution logic

## Verification Commands

Expected stage verification:

```bash
cargo test -p pantograph-node-contracts
cargo test -p workflow-nodes
cargo test -p pantograph-embedded-runtime
cargo check --workspace --all-features
```

If saved-workflow migration use cases are touched, also run:

```bash
cargo test -p pantograph-workflow-service
```

Stage completion also requires the Rust baseline verification from
`RUST-TOOLING-STANDARDS.md` unless the stage-start report records an existing
repo-owned equivalent:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Verification

- Composed nodes remain inspectable in diagnostics.
- Model/license usage points to primitive model execution and composed parent
  context.
- Graph authoring remains practical without forcing users into low-level-only
  primitive graphs.
- Persisted workflows are upgraded, regenerated, or rejected with actionable
  typed migration diagnostics.
- Migration fixtures prove saved workflows either upgrade cleanly or fail with
  actionable typed migration diagnostics.

## Completion Criteria

- Composition improves usability without reducing diagnostics quality.
- Existing workflow upgrade strategy is documented, including removal of old
  compatibility surfaces after migration.
- The stage-start implementation gate in
  `08-stage-start-implementation-gate.md` is recorded before source edits.
- The stage-end refactor gate in `09-stage-end-refactor-gate.md` is completed
  or explicitly recorded as not warranted for this stage.

## Re-Plan Triggers

- Existing node ids or port ids cannot be upgraded safely.
- Composed nodes cannot retain primitive model/license attribution.
- Temporary migration projections would require duplicating canonical semantics
  in GUI or bindings.
