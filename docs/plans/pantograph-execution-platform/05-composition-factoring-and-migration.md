# 05: Composition, Factoring, And Migration

## Purpose

Support higher-level graph usability without hiding primitive runtime facts or
breaking existing persisted workflows unnecessarily.

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

The first implementation wave should avoid breaking persisted workflows unless
the break is explicit and migrated.

Required migration work:

- inventory existing node type ids and port ids
- classify coarse nodes as keep, split, or compose
- preserve stable ids where feasible
- add compatibility projections for existing persisted graphs
- define regeneration or migration rules for affected graph artifacts
- preserve diagnostics meaning across contract upgrades

## Affected Structured Contracts And Persisted Artifacts

- Node type ids, port ids, composed-node contracts, internal primitive graph
  mappings, saved workflow graphs, migration records, compatibility
  projections, and diagnostics lineage projections.

## Standards Compliance Notes

- Architecture compliance requires composition to preserve primitive execution
  facts and avoid hiding model/license usage behind presentation-only nodes.
- Rust API compliance requires explicit migration states, compatibility
  decisions, composed-parent lineage types, and typed errors for unmigratable
  persisted graphs.
- Documentation and release compliance require migration notes or changelog
  entries for user-visible contract changes, especially removed or renamed
  nodes and ports.
- Testing compliance requires saved-workflow fixture migration tests,
  diagnostics lineage tests, model/license attribution tests through composed
  nodes, and compatibility tests for preserved ids.
- Tooling compliance requires schema-backed or fixture validation for persisted
  workflow artifacts touched by migrations.

## Risks And Mitigations

- Risk: factoring improves authoring but loses trace fidelity. Mitigation:
  require primitive trace mapping and composed-parent lineage.
- Risk: persisted workflows silently change behavior. Mitigation: require
  explicit migration records or compatibility projections.
- Risk: large nodes remain because decomposition is too disruptive. Mitigation:
  classify keep/split/compose and defer only with documented compatibility
  rationale.

## Tasks

- Define composed-node external contract rules.
- Define primitive trace mapping for composed nodes.
- Classify existing coarse inference nodes as keep, split, or compose.
- Ensure composed nodes preserve model/license attribution for internal
  primitive model execution.
- Define migration rules for persisted workflows affected by node factoring.

## Verification

- Composed nodes remain inspectable in diagnostics.
- Model/license usage points to primitive model execution and composed parent
  context.
- Graph authoring remains practical without forcing users into low-level-only
  primitive graphs.
- Persisted workflow compatibility is either preserved or explicitly migrated.
- Migration fixtures prove saved workflows either preserve behavior or fail
  with actionable typed migration diagnostics.

## Completion Criteria

- Composition improves usability without reducing diagnostics quality.
- Existing workflow migration or compatibility strategy is documented.
- The stage-start implementation gate in
  `08-stage-start-implementation-gate.md` is recorded before source edits.
- The stage-end refactor gate in `09-stage-end-refactor-gate.md` is completed
  or explicitly recorded as not warranted for this stage.

## Re-Plan Triggers

- Existing node ids or port ids cannot be preserved or migrated safely.
- Composed nodes cannot retain primitive model/license attribution.
- Compatibility projections would require duplicating canonical semantics in
  GUI or bindings.
