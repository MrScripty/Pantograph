# Plan: Pantograph Phase 3 KV Cache Implementation

## Status
Active

Last updated: 2026-04-19

## Current Source-of-Truth Summary

This document is the dedicated source of truth for roadmap Phase 3.

The current codebase has partial KV-cache implementation, but it is not a
completed workflow primitive:

- `crates/inference/src/kv_cache/` already owns a real store with memory and
  disk persistence, model-fingerprint validation, metadata, markers, and
  codec-based truncation hooks.
- `crates/node-engine/src/core_executor.rs` already exposes working
  `kv-cache-save` and `kv-cache-load` executor paths backed by that store.
- `crates/workflow-nodes/src/storage/kv_cache_{save,load,truncate}.rs` still
  contain placeholder task runtime logic and TODO comments rather than the
  real backend-owned execution path.
- Truncation and partial reuse are not operational end to end because concrete
  runtime codecs and reuse integration are not yet wired into supported
  inference backends.
- The roadmap is internally stale: its top summary still says Phase 3 is not
  started, while the Phase 3 section says complete.

Phase 3 therefore needs a real implementation plan that finishes KV cache as a
backend-owned workflow primitive and reconciles the stale source of truth.

## Objective

Implement a standards-compliant, backend-owned KV-cache system that:

- allows compatible inference nodes to consume and emit explicit KV-cache
  artifacts in workflows
- validates KV reuse against the same model and runtime fingerprint before
  reuse is allowed
- supports prefix reuse, marker-based truncation, and partial reruns where the
  runtime/backend can prove compatibility
- integrates KV artifacts with kept-alive workflow sessions and Phase 6
  workflow-session memory through indirect references instead of parallel state
  systems
- surfaces machine-consumable hit, miss, fallback, and invalidation facts in
  diagnostics

The implementation must keep business logic in backend Rust, keep Tauri and the
frontend as read-only consumers of backend-owned facts, and leave immediate
touched files and directories in a standards-compliant state.

## Scope

### In Scope

- backend-owned KV-cache contracts, handles, compatibility rules, and runtime
  adapter contracts
- explicit graph-node support for KV-cache input/output on compatible
  inference nodes
- model-fingerprint validation strong enough to prevent cross-model or
  cross-runtime misuse
- supported runtime/backend capture, restore, and truncation paths
- session-memory integration for kept-alive reruns and partial workflow reuse
- diagnostics for KV reuse and invalidation outcomes
- immediate standards refactors in touched backend, workflow-node, transport,
  and README boundaries
- roadmap and documentation reconciliation for Phase 3

### Out of Scope

- distributed or multi-host cache synchronization
- broad durable history/versioning beyond bounded cache retention
- pretending unsupported runtimes support KV reuse when they do not
- moving KV lifecycle ownership into Tauri or the frontend
- codebase-wide compliance sweeps outside the immediate touched areas

## Inputs

### Problem

Pantograph currently has a usable KV-cache store but not a usable KV-cache
workflow system. The missing pieces are the ones that matter for actual
workflow execution:

- inference nodes do not yet expose an explicit KV input/output contract in the
  graph
- workflow-node task implementations still present placeholder behavior instead
  of the real backend executor path
- runtime reuse is not yet integrated with supported inference backends as a
  stable workflow primitive
- the current implementation does not yet tie KV reuse into Phase 6 workflow
  session memory and partial rerun semantics
- diagnostics do not yet expose KV hit, miss, truncation, or invalidation
  reasons as a first-class backend-owned contract

### Constraints

- Backend Rust owns KV semantics, compatibility, capture, restore, truncation,
  reuse, and diagnostics truth.
- Tauri remains a transport and composition layer only.
- The frontend may expose explicit KV ports and render diagnostics, but it must
  not infer KV validity or cache lifecycle locally.
- KV artifacts are narrower than general workflow memory and must not replace
  Phase 6 node memory.
- KV reuse is allowed only when compatibility can be proven by backend-owned
  fingerprints and runtime support.
- Unsupported runtimes must fail deterministically or degrade cleanly instead
  of silently pretending reuse happened.
- Existing workflow, runtime, and diagnostics contracts should remain additive
  unless a documented breaking change is explicitly approved.

### Assumptions

- KV cache is meaningful first for text-generation inference paths and should
  not be forced onto non-sequential inference nodes such as diffusion unless a
  backend proves a compatible equivalent artifact.
- Initial concrete runtime support should land incrementally, starting with the
  runtime/backend combination that can genuinely capture and restore KV state
  with a stable codec path.
- Phase 6 workflow-session memory remains the owner of logical workflow state;
  KV cache can be referenced from that state but should not become a second
  session-memory owner.
- Explicit graph-node ports are the correct workflow-level affordance because
  they make cache flow visible and auditable in the graph.

### Dependencies

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-6-incremental-graph-execution.md`
- `crates/inference/src/kv_cache/`
- `crates/node-engine/src/core_executor.rs`
- `crates/workflow-nodes/src/storage/`
- `crates/workflow-nodes/src/processing/`
- `crates/pantograph-workflow-service/src/capabilities.rs`
- `crates/pantograph-workflow-service/src/technical_fit.rs`
- `crates/pantograph-embedded-runtime/src/lib.rs`
- `src-tauri/src/main.rs`
- standards in
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`

### Affected Structured Contracts

- backend KV-cache DTOs in `crates/inference/src/kv_cache/types.rs`
- executor input/output contracts for `kv-cache-save`, `kv-cache-load`, and
  `kv-cache-truncate`
- graph-node port contracts for compatible inference nodes
- workflow runtime requirements / required extension shaping
- Phase 6 workflow-session node-memory indirect state references when they
  point at KV artifacts
- diagnostics payloads for KV reuse outcomes

### Affected Persisted Artifacts

- cached KV metadata and on-disk cache entries under the app data directory
- roadmap Phase 3 wording and status
- touched `README.md` files in:
  - `crates/inference/src/kv_cache/`
  - `crates/workflow-nodes/src/storage/`
  - `crates/workflow-nodes/src/processing/` if inference-node ports change
  - `crates/node-engine/src/` or any new extracted subdirectory
  - `crates/pantograph-embedded-runtime/src/` if runtime KV helpers are
    extracted there

### Concurrency and Lifecycle Review

- KV store access is shared mutable state and must have one owner for cache
  mutation semantics, bounded retention, and update ordering.
- Capture/restore/truncate paths may overlap with kept-alive execution,
  reclaim, restore, or partial rerun paths; the plan must define a single
  lifecycle owner and prohibit split ownership between node-engine,
  embedded-runtime, and Tauri.
- Cache cleanup or eviction work must have explicit start/stop ownership, no
  polling loops in the frontend, and deterministic shutdown behavior.
- Tests must isolate durable cache directories and any global cache store
  state per test.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Duplicate execution ownership remains split between workflow-node task stubs and `core_executor` handlers | High | Unify on one backend execution owner before adding new behavior |
| Overly weak fingerprints allow invalid reuse across model/runtime changes | High | Freeze a strict compatibility contract before runtime integration |
| Phase 6 node memory and Phase 3 KV references diverge into parallel state systems | High | Store KV artifacts as explicit indirect references under the Phase 6 session-memory model |
| Oversized touched files absorb more logic and drift further from standards | High | Extract focused helper modules before adding Phase 3 behavior |
| Diagnostics overclaim reuse when unsupported runtimes fall back | Medium | Emit explicit hit/miss/fallback reasons from backend-owned execution paths |
| Placeholder READMEs and banned filler language hide the real ownership boundaries | Medium | Rewrite touched READMEs in the same slices that settle the code boundaries |

## Definition of Done

- A dedicated Phase 3 plan exists and is reconciled with the roadmap.
- Compatible inference nodes expose explicit KV input/output ports and the
  graph enforces typed connectivity for them.
- KV reuse is blocked unless backend-owned compatibility checks prove the same
  model/runtime fingerprint.
- At least one supported runtime performs real capture, restore, and marker or
  token truncation through a concrete backend codec.
- Workflow-session reruns can preserve and reuse compatible KV artifacts
  through Phase 6 node-memory references.
- Backend diagnostics expose cache hit, miss, invalidation, and fallback
  reasons through stable machine-consumable fields.
- Immediate touched files and directories are left in standards-compliant
  shape, including README updates and decomposition where thresholds are
  exceeded.

## Ownership and Lifecycle Note

- `crates/inference/src/kv_cache/` owns low-level KV artifact contracts,
  storage, metadata, and codec-facing manipulation helpers.
- `node-engine` owns executor-time KV use-case behavior and the typed node
  input/output contract consumed during workflow execution.
- `pantograph-embedded-runtime` owns runtime-specific capture/restore/truncate
  orchestration for supported backends, but it must consume backend-owned KV
  contracts instead of inventing adapter-local cache semantics.
- `pantograph-workflow-service` owns workflow capability and runtime-requirement
  shaping for graphs that explicitly request KV behavior.
- Tauri may wire the shared store and forward diagnostics, but it must not own
  KV validity, eviction, or reuse policy.
- Any cache cleanup worker or retention task introduced by implementation must
  have one composition-root start owner, one stop owner, and deterministic
  shutdown semantics.

## Public Facade Preservation Note

- Preserve existing workflow, runtime, and diagnostics facades by growing KV
  contracts additively where possible.
- Prefer extracting focused helper modules behind current facades over API-
  breaking rewrites of `core_executor`, `WorkflowService`, or runtime entry
  surfaces.
- If a graph-node contract must change, make it append-only and keep older
  workflows loadable by treating missing KV fields as explicit "KV disabled".

## Milestones

### Milestone 1: Reconcile Source Of Truth And Freeze KV Contracts

**Goal:** Freeze one backend-owned KV-cache contract model before more code is
added.

**Tasks:**
- [x] Reconcile roadmap Phase 3 status and add this dedicated Phase 3 plan as
      the source of truth.
- [x] Define backend-owned concepts for:
      `kv artifact`,
      `kv handle`,
      `kv compatibility`,
      `kv usage mode`,
      `kv truncation marker`.
- [x] Define a stable `KvCacheHandle`-style executable contract that inference
      nodes can consume and emit explicitly in the graph.
- [x] Define strict compatibility fields so KV can be reused only for the same
      model/runtime fingerprint and tokenizer-relevant configuration.
- [x] Decide how Phase 6 workflow-session memory stores indirect references to
      KV artifacts without becoming a second cache owner.
- [x] Rewrite the placeholder READMEs in `crates/inference/src/kv_cache/` and
      `crates/workflow-nodes/src/storage/` so the ownership model is explicit.

**Verification:**
- Source-of-truth review against plan, architecture, and documentation
  standards
- Contract review for additive compatibility and executable-boundary clarity

**Status:** Complete

### Milestone 2: Refactor Immediate Backend Boundaries Before Behavior Growth

**Goal:** Make the immediate insertion areas standards-compliant before adding
end-to-end KV behavior.

**Tasks:**
- [x] Extract KV executor handlers out of `crates/node-engine/src/core_executor.rs`
      into focused KV modules before adding more Phase 3 logic there.
- [x] Runtime-side extraction in `crates/pantograph-embedded-runtime/src/lib.rs`
      was not required for this milestone because no new KV runtime-state or
      branching behavior was added in the embedded runtime layer.
- [x] Capability/requirement extraction from `technical_fit.rs` and
      `capabilities.rs` was not required for this milestone because no new
      KV-specific runtime-policy shaping landed yet.
- [x] Eliminate split execution ownership between the placeholder
      `workflow-nodes` storage task runtime bodies and the real backend
      `node-engine` executor path. One owner must remain.
- [x] Update or add READMEs for any new extracted source directories/modules.

**Verification:**
- `cargo check` or focused crate tests for touched backend packages
- README review for every touched `src/` directory
- Decomposition review for touched files that exceed size/responsibility
  thresholds

**Status:** Complete

### Milestone 3: Add Explicit Workflow-Graph KV Ports And Validation

**Goal:** Make KV usage explicit and typed in the workflow graph.

**Tasks:**
- [x] Add a typed KV port/data contract for graph connectivity instead of
      treating KV artifacts as generic JSON.
- [x] Extend compatible inference-node descriptors with explicit optional KV
      input and KV output ports.
- [x] Keep incompatible nodes out of the first rollout; do not add KV ports to
      runtimes that cannot prove text-generation-style KV reuse semantics.
- [x] Update graph compatibility checks so KV ports only connect to KV ports.
- [x] Ensure graph load/save and structured workflow artifacts treat missing KV
      settings as "disabled" for backward compatibility.
- [x] Touch frontend graph/node code only if needed to render or order the new
      ports; keep the frontend declarative and backend-contract-driven.
- [x] Update `processing/README.md` if inference-node port contracts change.

**Verification:**
- Focused Rust tests for descriptor and graph-port compatibility
- Focused frontend typecheck/tests only if node rendering is touched
- Backward-compatibility load check for workflows without KV fields

**Status:** Complete

### Milestone 4: Integrate Real Runtime Capture, Restore, And Truncation

**Goal:** Turn the store into a real inference-runtime reuse primitive.

**Tasks:**
- [x] Implement concrete runtime adapter and codec support for at least one
      supported backend that can genuinely capture and restore KV state.
- [x] Route `kv-cache-save`, `kv-cache-load`, and `kv-cache-truncate` through
      the real backend-owned execution path rather than placeholder task logic.
- [x] Implement real marker/token truncation through backend codecs where
      supported; unsupported runtimes must return explicit unsupported reasons.
- [x] Ensure both load-time validation and consume-time validation enforce the
      same fingerprint rules.
- [x] Add bounded retention and eviction policy semantics to the real store,
      including explicit failure behavior when entries are missing or invalid.
- [x] Keep dependency growth minimal and justify any runtime-specific codec
      dependency additions per dependency standards.

**Verification:**
- Focused `cargo test` for `crates/inference`
- Focused `cargo test` for `crates/node-engine`

**Current note:**
- llama.cpp now owns the first real capture/restore path and explicit
  unsupported truncation reporting.
- PyTorch now has backend-owned KV runtime/model identity plus worker snapshot
  save/restore/clear/truncate primitives for `dllm`-style live caches.
- `pytorch-inference` now both restores compatible `kv_cache_in` handles and
  emits fresh `kv_cache_out` handles through the shared KV store contract for
  `dllm` execution.
- The remaining PyTorch gap is broader session/partial-rerun integration, not
  the per-node explicit KV input/output contract.
- Cross-layer acceptance path from inference execution to saved KV handle to
  later compatible reuse
- Re-run affected suites to detect durable cache-state leakage

**Status:** Complete

### Milestone 5: Integrate KV With Workflow Sessions And Partial Reruns

**Goal:** Make KV reuse useful for kept-alive workflows and suffix-only reruns.

**Tasks:**
- [x] Extend Phase 6 workflow-session memory to carry indirect KV references
      for compatible inference nodes.
- [x] Define how repeated kept-alive invocations reuse compatible KV when only
      downstream suffix inputs change.
- [x] Define invalidation rules for:
      model change,
      runtime/backend change,
      tokenizer/config change,
      upstream prompt-prefix change,
      graph edit breaking prefix compatibility.
- [x] Reuse KV artifacts only through backend-owned partial-rerun semantics;
      do not let the frontend or Tauri decide reuse.
- [x] Ensure reclaim/restore and checkpoint paths keep logical ownership of KV
      references aligned with the workflow session.

**Verification:**
- Focused `cargo test` for session-memory and workflow execution paths
- Cross-layer acceptance for kept-alive rerun with unchanged prefix and changed
  suffix
- Replay/idempotency checks for restore/retry flows that include KV references

**Status:** Complete

Latest landed slice:
- Bound workflow-session node-memory projection now preserves compatible
  `kv_cache_out` handles as backend-owned indirect references with inspection
  metadata instead of dropping those runtime-restorable artifacts between runs.
- Backend node preparation now projects a preserved KV handle reference back
  into typed `kv_cache_in` inputs when rerunning a bound session without an
  explicit override, so reuse handoff stays in Rust execution paths.
- Backend rerun preparation now also refuses to project preserved KV handles
  from node-memory snapshots whose status is already `invalidated`, so
  upstream-prefix or graph-edit invalidation does not silently reuse stale KV.
- Embedded keep-alive checkpoint and scheduler-reclaim coverage now asserts
  that workflow-session-owned KV references survive backend restore paths and
  remain isolated per session instead of cross-wiring across retained
  executors.
- Graph memory-impact analysis for KV-capable inference nodes now emits
  explicit backend-owned invalidation reasons for model, runtime/backend,
  tokenizer-or-config, upstream-prefix, and graph-topology compatibility
  breaks instead of leaving later rerun policy to infer them from generic
  change tags.
- Bound-session acceptance coverage now freezes the suffix-only reuse rule:
  explicit graph wiring from upstream `kv_cache_out` to downstream
  `kv_cache_in` lets the demand engine keep the prefix node cached while only
  rerunning the downstream suffix consumer after suffix-input edits.
- The combined node-memory, demand-engine, and checkpoint/reclaim coverage now
  closes Milestone 5: KV reuse decisions stay in backend Rust execution paths,
  and logical KV-reference ownership remains aligned with the workflow session
  across reruns, invalidation, checkpoint, restore, and reclaim.

### Milestone 6: Add Diagnostics, Runtime Requirements, And Source-Of-Truth Close-Out

**Goal:** Expose KV behavior as observable backend facts and reconcile the
phase documentation.

**Tasks:**
- [x] Surface backend-owned diagnostics for cache hit, miss, invalidation,
      unsupported-runtime fallback, truncation outcome, and reuse source.
- [x] Update runtime requirement extraction so workflows that explicitly enable
      KV usage declare the `kv_cache` extension requirement.
- [x] Ensure diagnostics and preflight consumers use the same canonical
      extension and compatibility contract.
- [x] Reconcile the roadmap so Phase 3 status and milestone wording match the
      landed implementation.
- [x] Close the plan with a completion summary and touched README updates.

**Verification:**
- Focused diagnostics tests for stable machine-consumable KV fields
- Preflight/runtime-requirement tests for `kv_cache` extension shaping
- Source-of-truth review for roadmap, plan, and touched READMEs

**Status:** Complete

Latest landed slice:
- `node-engine` now emits backend-owned structured KV execution diagnostics for
  restore hit/miss/invalidation, capture saved/unsupported, and truncate
  outcomes, and those facts now flow through workflow trace plus Tauri
  diagnostics without moving reuse decisions into adapter code.
- Immediate preflight/diagnostics fixtures now use the canonical `kv_cache`
  extension name so runtime requirement shaping and diagnostics examples no
  longer drift across underscore versus hyphen spellings.

## Re-Plan Triggers

- A second runtime/backend needs materially different KV semantics that do not
  fit the frozen handle/codec contract.
- Phase 6 session-memory integration requires a broader persistence or
  checkpoint model than currently approved.
- Supported backends cannot provide a concrete truncation/capture codec and the
  rollout must pivot to a different first backend.
- Graph-level explicit KV ports prove insufficient and a different explicit
  workflow contract is required.

## Recommendations

- Recommendation 1: Deliver runtime integration incrementally by backend.
  Start with the first backend that can prove real capture, restore, and
  truncation. This keeps the contract honest and avoids fake parity across
  runtimes that do not yet support the feature.
- Recommendation 2: Keep KV handles as explicit typed artifacts in the graph.
  This makes reuse auditable, preserves backend ownership of validation, and
  gives partial-rerun/session-memory work a clean integration seam.
- Recommendation 3: Unify on one execution owner for KV nodes before feature
  growth. Leaving placeholder workflow-node runtime logic beside real executor
  handlers would violate single-owner and lifecycle standards.

## Standards Review Passes

### Pass 1: Plan And Architecture Standards Review

Checked against:
- `PLAN-STANDARDS.md`
- `ARCHITECTURE-PATTERNS.md`
- `CODING-STANDARDS.md`

Resulting corrections:
- added explicit objective, scope, inputs, risks, definition of done, milestone
  verification, ownership note, facade-preservation note, and re-plan triggers
- froze backend ownership of KV semantics and prohibited Tauri/frontend from
  becoming reuse-policy owners
- required one explicit contract model for KV handles and compatibility instead
  of allowing ad hoc JSON payloads or runtime-local cache semantics

### Pass 2: Decomposition And Immediate File-Health Review

Checked against:
- `CODING-STANDARDS.md`
- `DOCUMENTATION-STANDARDS.md`

Immediate standards issues already identified and folded into the plan:
- `crates/node-engine/src/core_executor.rs` exceeds decomposition thresholds and
  must not absorb more KV behavior inline
- `crates/pantograph-embedded-runtime/src/lib.rs` exceeds decomposition
  thresholds and requires extraction before meaningful KV runtime growth there
- `crates/pantograph-workflow-service/src/technical_fit.rs` exceeds the soft
  decomposition threshold and should not absorb more KV-specific policy inline
- `crates/inference/src/kv_cache/store.rs` exceeds 500 lines and must be
  reviewed for extraction before major Phase 3 growth
- `crates/workflow-nodes/src/storage/README.md` and
  `crates/inference/src/kv_cache/README.md` use banned placeholder language and
  need standards-compliant rewrites

Resulting corrections:
- added an explicit refactor milestone before new feature growth
- required README rewrites and new module READMEs in the same slices as
  extraction work

### Pass 3: Concurrency And Lifecycle Standards Review

Checked against:
- `CONCURRENCY-STANDARDS.md`
- `TESTING-STANDARDS.md`

Resulting corrections:
- required one owner for KV mutation, retention, and cleanup semantics
- required explicit lifecycle ownership for any cleanup or retention worker
- required per-test durable cache isolation and replay/idempotency verification
  for restore and kept-alive flows
- required the plan to keep shared mutable cache state behind backend-owned
  synchronization boundaries rather than scattering state across app layers

### Pass 4: Dependency, Interop, And Frontend Standards Review

Checked against:
- `DEPENDENCY-STANDARDS.md`
- `INTEROP-STANDARDS.md`
- `FRONTEND-STANDARDS.md`

Resulting corrections:
- required dependency additions to stay narrow and runtime-specific rather than
  adding heavy generic dependencies to core crates
- required the KV handle and diagnostics payloads to be executable structured
  contracts across Rust, Tauri, and frontend boundaries
- required frontend changes, if any, to stay declarative and contract-driven
  with no local KV validity logic or polling-based ownership

### Pass 5: Standards-Compliance Conclusion

If implemented as written, this plan will:

- keep KV business logic in backend Rust
- preserve one execution owner and one lifecycle owner for KV reuse behavior
- leave touched files and directories in a healthier, standards-compliant state
- provide cross-layer verification instead of relying on local unit tests only
- integrate KV as a specialized execution artifact under the broader
  workflow-session memory model rather than creating a competing state system

Re-plan before implementation if:
- the first supported runtime cannot provide a real codec/capture path
- Phase 3 needs broader persistence semantics than the current bounded store
  model
- graph contracts need breaking changes rather than additive KV fields

## Completion Summary

### Completed

- Backend-owned KV artifact, handle, compatibility, and usage-mode contracts
  are frozen in `crates/inference/src/kv_cache`.
- KV execution ownership lives in `node-engine` through
  `core_executor::kv_cache`, while `workflow-nodes` stays descriptor-only.
- Workflow graphs now expose a first-class `kv_cache` port type with matching
  validation and metadata across backend and Tauri transport surfaces.
- llama.cpp and PyTorch `dllm` execution paths now support backend-owned KV
  capture/restore through typed handles, with truncation delegated through
  backend-compatible paths and explicit unsupported behavior where codecs do
  not yet exist.
- Workflow capability extraction and immediate diagnostics/preflight fixtures
  now use the canonical `kv_cache` extension contract.
- Structured KV execution diagnostics now flow from backend execution through
  workflow trace and Tauri diagnostics without shifting reuse-policy ownership
  into adapters.
- Workflow-session reuse is now fully covered: bound node-memory projection,
  rerun injection, explicit suffix-only graph-wired reuse, invalidation
  taxonomy, and checkpoint/reclaim ownership all stay backend-owned.
- The roadmap and touched module READMEs are reconciled with the landed Phase 3
  implementation state.

### Deviations

- None.

### Follow-Ups

- Future work, if any, belongs in later roadmap phases rather than reopening
  the completed Phase 3 contract unless a new backend/runtime demands a
  contract change.

### Verification Summary

- `cargo test -p node-engine events::tests -- --nocapture`
- `cargo test -p pantograph-workflow-service trace::tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml translated_task_progress_detail_updates_backend_diagnostics_projection -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml node_progress_detail_is_exposed_in_diagnostics_snapshot -- --nocapture`

### Traceability Links

- Module README updated:
  `crates/inference/src/kv_cache/README.md`,
  `crates/node-engine/src/core_executor/README.md`
- ADR added/updated: none
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: not applicable in
  local repository history
