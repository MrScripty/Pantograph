# Milestone 5d: Inference Interface Resolution And Validation

**Goal:** Add the canonical backend resolver/validator that turns a connected
Pumas model reference into the typed input/output interface for a generic
inference node. The graph editor, workflow validators, scheduler task
materialization, and pre-dispatch runtime checks must all consume this same
contract.

This milestone is inserted after Milestone 5c task-level orchestration and
before the remaining Milestone 5b production runtime-host wiring. Milestone 6
PyTorch/diffusers execution must consume this generic system rather than
defining an image-only inference-node interface.

**Tasks:**

- [x] Add the dedicated `pantograph-inference-interface-contracts` crate as a
      DTO-only shared contract owner. It must not import workflow-service,
      scheduler, runtime-host, Pumas lookup, frontend rendering policy, or
      worker execution code.
- [x] Add the new crate README and any affected source-directory README/ADR
      updates in the same slices that introduce or change public boundaries.
      The contract crate README must document purpose, dependencies, invariants,
      API consumer contract, structured producer contract, and revisit triggers.
- [x] Verify contract-crate dependency direction before broad implementation.
      It may depend on stable path-free model-reference DTOs only when doing so
      does not pull in Pumas lifecycle, lookup, storage, runtime wiring,
      workflow-service, scheduler, runtime-host, inference execution, or
      frontend policy.
- [x] Define resolver and validator request contracts separately. Resolver
      input is model-focused (`PumasModelRef`, optional task kind, optional
      runtime/device constraints). Validator input consumes a descriptor
      fingerprint plus draft node bindings, literals, and selected options.
- [x] Define the shared `InferenceInterfaceDescriptor` contract with typed
      input/output ports, task kind, grouped value categories, typed defaults,
      typed option sets, availability, runtime conditions, capability
      fingerprint, and bounded diagnostics. The contract must be versioned,
      path-free, `serde(deny_unknown_fields)`, and covered by serde fixtures.
- [x] Define the grouped value categories: scalar, artifact, reference, and
      constraint. Defaults must be typed small scalar/default-marker values
      unless a later artifact-reference default contract is explicitly added.
- [x] Define typed bounded option sets with per-option availability and
      diagnostics. Do not use `serde_json::Value`, metadata bags, or
      runtime-specific option blobs.
- [x] Define availability as coarse status plus additive typed reason codes,
      with bounded diagnostics attached to the descriptor, port, option, drift,
      or validation event that owns the availability decision. Public enums
      should be `#[non_exhaustive]` where future variants are expected, and UI
      behavior should key primarily from the coarse status.
- [x] Define the minimal authored interface snapshot persisted directly in
      inference node data. It must preserve graph shape and drift diagnostics
      only: descriptor fingerprint, authored port summaries, labels when
      needed, value types, requirements, default markers or small scalar
      defaults, and last known availability status/reasons. It must not contain
      Pumas package facts, executable paths, runtime load targets, worker
      settings, scheduler decisions, full runtime API schemas, or large media.
- [ ] Replace inference-node use of `node.data.definition` as a semantic
      dynamic-port source. For inference nodes, `node.data.definition` may
      remain only as a generated projection from the authored snapshot/current
      descriptor during staged integration, and must not be accepted as an
      executable fallback when descriptor validation fails.
- [ ] Retire the current static all-port `llm-inference` interface. Keep only
      bootstrap/control ports required before model resolution, such as
      `pumas_model_ref`, optional task kind, optional runtime/device
      constraints, and diagnostics. All task/model-specific prompt, image,
      mask, sampler, result, and runtime-condition ports must come from the
      descriptor/authored snapshot.
- [x] Define authored-versus-current drift report contracts in
      `pantograph-inference-interface-contracts`. Drift reports must identify
      added/removed ports, type changes, requirement/default/option changes,
      availability changes, task/runtime-condition changes, severity, and
      blocking diagnostics.
- [ ] Define workflow graph/service-owned typed graph patch operations and
      update proposal contracts for "update to current interface." The
      inference-interface contract crate may provide drift types, but graph
      patch operations must live with graph mutation ownership.
- [ ] Define live draft validation session contracts with backend-issued
      validation session ids, monotonic client graph revisions, event sequence
      numbers, descriptor/drift/diagnostic/update-proposal events, and a
      backend-owned validation summary.
- [x] Define the graph validation summary contract with status, executable
      boolean, typed enqueue-disabled reasons, diagnostics count, and blocking
      diagnostics count. Frontend must not infer enqueue permission from raw
      diagnostics.
- [ ] Add a workflow-service resolver boundary that combines Pumas model and
      selected-artifact facts, inference capability facts, runtime availability
      facts, and optional graph-authored constraints into one descriptor. It
      must return typed unavailable/not-implemented diagnostics when facts or
      runtime support are missing rather than guessing from names or paths.
- [ ] Implement the first thin vertical slice before broad horizontal
      expansion: connected model ref input resolves a descriptor, projects
      authored visible ports, produces a backend validation summary, and gates
      frontend submit plus backend admission without invoking retired
      `inference_settings`, `expand-settings`, static all-port, or model-path
      paths.
- [ ] Ensure runtime/device constraints narrow interface validation without
      becoming scheduler decisions. Explicit invalid constraints block enqueue
      and may include typed advisory alternatives when they can be computed
      safely; alternatives must not become fallback execution choices.
- [ ] Wire graph editor draft validation to request and render the descriptor
      for unsaved graph state through the live validation session model. The
      editor renders saved authored ports immediately, overlays pending/stale/
      invalid validation state from backend events, disables submit/enqueue
      from the latest backend summary, and does not block editing while
      validation runs.
- [ ] Reuse existing graph-session/event transport patterns for live validation
      only when they preserve backend ownership and event-driven UI updates.
      Workflow-service must snapshot draft graph state under lock, release the
      lock before Pumas/inference fact resolution, and publish events only for
      the current validation session id and client graph revision.
- [ ] Wire graph editor drift presentation and update preview. The editor must
      show authored-current diffs visually on nodes/ports/edges, keep invalid
      edges visible, preview backend-proposed typed patch operations, and apply
      them only after user confirmation.
- [ ] Wire `PortOptionsProvider`, selection-input, and option-cache consumers
      only as descriptor-backed presentation plumbing where reused. Typed
      option identity, defaults, availability, and diagnostics remain owned by
      `pantograph-inference-interface-contracts`, and cache keys/invalidation
      must include descriptor fingerprint or validation session/revision.
- [ ] Wire workflow save validation to consume the same descriptor contract.
      Required inputs, optional defaults, valid options, connected upstream
      output types, and explicit runtime/device constraints must be validated
      before a workflow can be saved or submitted as executable.
- [ ] Wire workflow submit and scheduler queue admission to consume the
      backend validation summary. The frontend submit button and backend queue
      admission must both fail closed while inference validation is pending,
      stale, unavailable, unresolved, or blocked; raw diagnostics must not be
      the enqueue authority.
- [ ] Update scheduler task graph projection and materialization so generic
      inference tasks store path-free model refs, task kind, typed constraints,
      descriptor fingerprint, and bindings only. Workflow-service must
      materialize final runtime-host inputs from upstream task results, graph
      literal values, and descriptor defaults after scheduler input readiness.
- [ ] Align the runtime-host input contract with descriptor materialization.
      Runtime-host execution requests must include scheduler-selected handoff
      plus typed, path-free materialized inputs and artifact/result references;
      they must not receive graph paths, Pumas package facts, or scheduler-owned
      payload guesses.
- [ ] Add pre-dispatch descriptor revalidation before scheduler dispatch
      selection. If Pumas facts, selected artifact state, runtime capability,
      or descriptor fingerprint changed since draft/save validation, fail the
      task with typed diagnostics rather than falling back to stale ports.
- [ ] Delete or rewrite retired inference-node port discovery and validation
      surfaces replaced by this resolver. Do not keep compatibility aliases,
      legacy model-path support checks, planned-inference validation branches,
      or image-only graph-editor port tables as successful paths.
- [ ] Delete or rewrite retired `inference_settings` JSON canonicalization and
      `expand-settings` frontend-owned dynamic port sync. If any of that UI
      remains, it must consume backend descriptors as presentation plumbing and
      cannot remain an alternate inference-interface source.
- [ ] Decide the first implementation slice for existing `PortOptionsProvider`,
      selection-input, and option-cache reuse versus a dedicated descriptor
      option renderer. The decision must preserve typed descriptor ownership
      and avoid duplicating backend semantics.

**Verification:**

- Descriptor serde fixture round trips for ports, defaults, options,
  availability, runtime conditions, and diagnostics.
- Contract-crate dependency-direction check proving DTO-only ownership and no
  imports of workflow-service, scheduler, runtime-host, Pumas lookup/runtime
  wiring, inference execution, or frontend policy.
- Contract-crate README/affected README or ADR review proving public boundary,
  structured producer, and API consumer contracts are documented.
- `cargo check` coverage for affected public crates in default,
  no-default-features, and all-features modes.
- Authored snapshot fixture round trips proving saved graph shape can be
  preserved without Pumas package facts, executable paths, runtime load
  targets, scheduler decisions, or large media payloads.
- Drift report fixture and unit tests for added/removed ports, type changes,
  requirement changes, default changes, option changes, availability changes,
  blocking drift, and non-blocking drift.
- Typed graph patch proposal tests proving backend proposals update authored
  snapshots, report affected edges/literals, require confirmation for
  destructive changes, and reject unsafe reconciliation.
- Live validation session tests proving stale session/revision events cannot
  update current graph validation state.
- Validation summary tests proving enqueue is disabled while validation is
  pending, stale, unavailable, unresolved, or blocked, and enabled only when
  backend says `executable = true`.
- Resolver unit tests for ready, unavailable, unsupported, unimplemented,
  missing selected artifact, stale fingerprint, and ambiguous capability
  states.
- Runtime/device constraint tests proving explicit invalid constraints block,
  alternatives are advisory only, omitted constraints leave scheduler as the
  final runtime/device decision owner, and descriptors do not expose full
  scheduler candidate lists in the common path.
- First vertical-slice acceptance test proving model ref input resolves a
  descriptor, projects authored ports, emits validation summary, and gates
  frontend submit plus backend admission from that summary.
- Effective-definition tests proving inference nodes derive visible ports from
  authored/current descriptors, not semantic `node.data.definition` fallbacks.
- Backend draft-validation tests proving graph editor-visible ports come from
  the descriptor and include typed disabled reasons.
- Live validation transport tests proving graph locks are not held while Pumas
  or inference fact resolution runs, and stale session/revision events are
  ignored.
- Workflow save-validation tests proving invalid ports, missing required
  inputs, wrong upstream output types, and unsupported options fail closed.
- Submit/admission tests proving frontend submit state and backend queue
  admission use the backend validation summary as authority.
- Materialization tests proving defaults are applied only from the descriptor,
  connected upstream task results are type-checked, and runtime-host inputs
  are path-free typed values or typed artifact/result references.
- Runtime-host contract tests proving execution requests carry typed
  materialized inputs with scheduler handoff and reject graph paths, package
  facts, and untyped payload bags.
- Pre-dispatch tests proving stale descriptor fingerprints and changed
  capability facts fail before scheduler dispatch.
- Search/deletion checks for retired dynamic inference port tables,
  `modelPath`/`model_path` support paths, planned-inference validation branches,
  raw package-fact graph payloads, and untyped inference metadata bags.
- Search/deletion checks for retired `inference_settings` JSON interface paths,
  `expand-settings` inference-interface ownership, static all-port
  `llm-inference` task/model ports, and inference-node `node.data.definition`
  semantic fallbacks.
- `cargo fmt`, focused package tests, affected crate `cargo check` in default,
  no-default-features, and all-features modes, plus frontend type/test coverage
  for touched draft-validation UI.

**Status:**

- [ ] Pending implementation. Added as a Milestone 5 family prerequisite after
      the runtime-host input payload replan exposed that production runtime
      dispatch needs a canonical model-specific inference interface before
      task inputs can be materialized safely.
- [ ] Design decisions recorded through live validation summary and
      runtime/device alternative diagnostics. Codebase investigation resolved
      the main integration direction: authored snapshots replace inference
      `node.data.definition` semantics, static all-port inference descriptors
      are retired, JSON `inference_settings`/`expand-settings` paths are
      removal or rewrite targets, validation summary gates submit/enqueue, and
      runtime-host input contracts must be aligned before production dispatch.
      Remaining decisions are the concrete option-renderer reuse path,
      resolver module/API shape, graph patch ownership for drafts versus saved
      workflows, and live validation transport details.
- [x] 2026-05-25 contract-crate slice completed:
  - Smallest useful vertical slice: added the DTO-only
    `pantograph-inference-interface-contracts` crate, workspace registration,
    crate/source/test README coverage, versioned typed descriptors, resolver
    and validator request contracts, authored snapshots, drift reports,
    validation summaries, bounded availability/reason/diagnostic enums, and
    serde fixtures.
  - No-fallback/no-legacy confirmation: the slice adds only the canonical
    contract owner. It does not route `node.data.definition`,
    `inference_settings`, `expand-settings`, static all-port descriptors,
    model paths, Pumas package facts, runtime load targets, scheduler
    decisions, or worker execution through the new contracts.
  - Dependency-direction result: the new crate depends only on
    `pantograph-dependency-planning`, `serde`, and `thiserror`; the Pumas model
    reference and runtime/device intent ids are reused from the existing
    path-free dependency-planning contract instead of creating parallel DTOs.
  - Verification passed: `cargo fmt -p
    pantograph-inference-interface-contracts`; `cargo test -p
    pantograph-inference-interface-contracts`; `cargo check -p
    pantograph-inference-interface-contracts`; `cargo check -p
    pantograph-inference-interface-contracts --no-default-features`; `cargo
    check -p pantograph-inference-interface-contracts --all-features`; and
    targeted source search for forbidden imports, model-path fields, retired
    inference settings, and untyped metadata bags in the new crate.
  - Remaining follow-up: implement the first cross-layer descriptor resolution
    and validation acceptance slice before broad frontend, scheduler,
    runtime-host, or legacy-deletion work.
