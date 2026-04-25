# 05: Composition, Factoring, And Migration

## Purpose

Support higher-level graph usability without hiding primitive runtime facts or
breaking existing persisted workflows unnecessarily.

## Implementation Readiness Status

Ready for stage-start preflight after stages `01` through `04` are complete
and their stage-end refactor gates have been recorded.

## Implementation Notes

### 2026-04-24 Stage-Start Report

- Selected stage: Stage `05`, composition, factoring, and migration.
- Current branch: `main`.
- Stage base: `ce63ffb5`, the Stage `04` closeout commit.
- Prior-stage gates: Stage `01`, Stage `02`, Stage `03`, and Stage `04`
  coordination ledgers record completed implementation and stage-end refactor
  gate outcomes of `not_warranted`.
- Git status before implementation: unrelated asset changes only:
  deleted `assets/3c842e69-080c-43ad-a9f0-14136e18761f.jpg`, deleted
  `assets/grok-image-6c435c73-11b8-4dcf-a8b2-f2735cc0c5d3.png`, deleted
  `assets/grok-image-e5979483-32c2-4cf5-b32f-53be66170132.png`,
  untracked `assets/banner_3.jpg`, `assets/banner_3.png`,
  `assets/github_social.jpg`, and `assets/reject/`.
- Dirty-file overlap: none. Stage `05` implementation must not touch
  `assets/`.
- Standards reviewed through the execution-platform standards map:
  `PLAN-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
  `CODING-STANDARDS.md`, `DOCUMENTATION-STANDARDS.md`,
  `TESTING-STANDARDS.md`, `CONCURRENCY-STANDARDS.md`,
  `TOOLING-STANDARDS.md`, `DEPENDENCY-STANDARDS.md`,
  `SECURITY-STANDARDS.md`, `RELEASE-STANDARDS.md`,
  `COMMIT-STANDARDS.md`, `languages/rust/RUST-API-STANDARDS.md`, and
  `languages/rust/RUST-TOOLING-STANDARDS.md`.
- Intended Wave `02` write sets:
  - `composition-contracts`: `crates/pantograph-node-contracts/`.
  - `workflow-nodes-factoring`: `crates/workflow-nodes/`.
  - `runtime-lineage`: `crates/pantograph-embedded-runtime/` lineage
    modules.
- Host-owned shared files: workspace manifests and lockfiles, public facade
  exports, saved workflow fixtures shared across workers, ADR files, release
  notes, and migration fixture integration.
- Forbidden write set for this stage unless the plan is updated first:
  host binding implementation, GUI redesign, and credential/session
  attribution logic.
- Start outcome: `ready_with_recorded_assumptions`.
- Recorded assumptions:
  - Wave `02` may be executed serially by the host in this shared workspace
    when subagents are not explicitly authorized; the recorded worker write
    sets and reports still apply.
  - Stage `05` may introduce composed-node contract metadata, migration
    records, fixture tests, and release notes without adding a new dependency.
    If a dependency becomes necessary, update this plan with dependency
    owner, transitive-cost, feature, audit, and release-artifact impact before
    editing manifests.
  - Existing GUI grouping behavior remains backend-owned graph state. Stage
    `05` may formalize composed-node contracts and migration behavior, but it
    must not redesign GUI grouping interactions.

### 2026-04-24 Wave 01 Inventory And Upgrade Policy Freeze

- Built-in workflow node descriptors are discovered through
  `node_engine::NodeRegistry::with_builtins()` and projected into canonical
  contracts by `workflow_nodes::builtin_node_contracts()`.
- Existing built-in workflow node type ids inventoried from
  `crates/workflow-nodes/src/`: `agent-tools`, `audio-generation`,
  `audio-input`, `audio-output`, `boolean-input`, `component-preview`,
  `conditional`, `dependency-environment`, `depth-estimation`,
  `diffusion-inference`, `embedding`, `expand-settings`, `human-input`,
  `image-input`, `image-output`, `json-filter`, `kv-cache-load`,
  `kv-cache-save`, `kv-cache-truncate`, `linked-input`,
  `llamacpp-inference`, `llm-inference`, `masked-text-input`, `merge`,
  `model-provider`, `number-input`, `ollama-inference`, `onnx-inference`,
  `point-cloud-output`, `process`, `puma-lib`, `pytorch-inference`,
  `read-file`, `reranker`, `selection-input`, `text-input`, `text-output`,
  `tool-executor`, `tool-loop`, `unload-model`, `validator`,
  `vector-input`, `vector-output`, `vision-analysis`, and `write-file`.
- Existing workflow-node port id inventory found 122 declared `PORT_*`
  constants. High-risk compatibility families are model identity and
  dependency ports (`model_path`, `model_id`, `model_type`,
  `task_type_primary`, `backend_key`, `recommended_backend`,
  `platform_context`, `selected_binding_ids`, `dependency_bindings`,
  `dependency_requirements_id`, `dependency_requirements`,
  `inference_settings`, `environment_ref`, and `model_ref`), inference
  controls (`prompt`, `system_prompt`, `context`, `tools`, `temperature`,
  `max_tokens`, `kv_cache_in`, `kv_cache_out`, `stream`, `steps`,
  `cfg_scale`, `seed`, `width`, `height`, `duration`,
  `num_inference_steps`, `guidance_scale`, `top_k`, and
  `return_documents`), and generated-output ports (`response`, `tool_calls`,
  `has_tool_calls`, `image`, `audio`, `embedding`, `results`,
  `top_document`, `scores`, `depth_map`, `point_cloud`,
  `duration_seconds`, `sample_rate`, and `model_used`).
- Existing saved workflow artifacts inventoried:
  `src/templates/workflows/gguf-reranker-workflow.json`,
  `crates/pantograph-embedded-runtime/src/lib_tests/graph_fixtures.rs`, and
  inline workflow-service graph/session fixtures under
  `crates/pantograph-workflow-service/src/graph/` and
  `crates/pantograph-workflow-service/src/workflow/tests/fixtures/`.
- Existing migration behavior found in
  `crates/pantograph-workflow-service/src/graph/canonicalization.rs`:
  legacy `system-prompt` nodes are canonicalized to `text-input` and legacy
  `prompt` handles are rewritten to `text`.
- Existing graph grouping behavior found in
  `crates/pantograph-workflow-service/src/graph/group_mutation.rs`:
  user-selected nodes can be collapsed into a `node-group` graph node carrying
  internal nodes, edges, and boundary port mappings in graph data.
- Classification freeze:
  - Keep as primitive contract families: simple input/output nodes,
    `conditional`, `merge`, storage nodes, `process`, `json-filter`,
    `validator`, `dependency-environment`, `puma-lib`, `model-provider`,
    `expand-settings`, specialized model-producing primitives
    (`llamacpp-inference`, `ollama-inference`, `pytorch-inference`,
    `onnx-inference`, `diffusion-inference`, `audio-generation`,
    `embedding`, `reranker`, `depth-estimation`), and direct
    non-model utilities.
  - Compose as stable external authoring contracts with primitive internal
    trace preservation: `node-group` and `tool-loop`.
  - Split only with explicit migration records and typed rejection fallback:
    `vision-analysis` and any future inference node whose direct model
    execution cannot be represented with Stage `03` managed runtime context
    and Stage `04` primitive model/license ledger attribution.
- Migration output semantics freeze:
  - `upgraded`: graph artifacts are rewritten to current node and port
    contracts with a migration record containing source and target contract
    digests or versions, changed node/port ids, and diagnostics-lineage
    behavior.
  - `regenerated`: volatile projections such as `derived_graph` are discarded
    and rebuilt from canonical graph state.
  - `typed_rejection`: artifacts that cannot be upgraded without silent
    behavior changes fail with actionable typed diagnostics.
- Temporary compatibility projections may exist only inside migration code.
  They must not remain as supported public node, port, GUI, or binding
  semantics after migration completes.
- Wave `02` non-overlap decision: `composition-contracts` owns canonical
  contract metadata and migration error types; `workflow-nodes-factoring`
  owns concrete descriptor changes and workflow-node README coverage; and
  `runtime-lineage` owns embedded-runtime composed-parent lineage projection.
- Wave `01` outcome: complete. Source implementation may begin with the
  `composition-contracts` slice.

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
