# Plan: Pantograph Phase 5 Follow-On Completion

## Status
Complete

Last updated: 2026-04-18

## Current Source-of-Truth Summary

This document is the dedicated source of truth for the follow-on audit that
closed roadmap Phase 5 after the main event-contract plan reached its
Milestone 5 close-out.

Use this file for the recorded close-out result covering:

- the concrete remaining backend-owned workflow-event gaps
- the standards-driven refactors required in the immediate touched code
- the remaining real transport and binding acceptance gaps that still belong to
  Phase 5
- the handoff that moves broader binding-platform expansion out of Phase 5 once
  the true event-contract work is closed

Use the earlier plan
`docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`
for completed milestone history and already-landed contract work. Use
`IMPLEMENTATION-PLAN-pantograph-binding-platform.md` for broader first-class
binding platform scope that is not required to close the remaining event
contract.

This follow-on audit is now complete. It did not find an additional real
backend-owned producer path or runtime-hosted transport surface that still
required Phase 5 workflow-event-contract work beyond the coverage already
landed across backend, adapter, and wrapper boundaries.

## Objective

Close the remaining real workflow-event-contract gaps without reopening the
completed Phase 5 implementation unnecessarily, while ensuring that:

- canonical lifecycle semantics stay backend-owned in Rust
- wrappers and transports remain thin validators/transports rather than policy
  owners
- any immediate code touched during implementation is first brought back toward
  standards compliance instead of deepening existing oversized modules
- any broader C#, Python, or BEAM binding-platform expansion is routed to the
  binding-platform plan unless it is truly required to finish the event
  contract

## Scope

### In Scope

- Remaining backend-owned producer paths that may still bypass canonical
  `WorkflowCancelled`, `WaitingForInput`, `GraphModified`, or
  `IncrementalExecutionStarted` semantics
- Remaining real non-streaming/headless transport or runtime-hosted binding
  surfaces that are still not directly pinned for the already-approved event
  contract
- Refactors required to keep the immediate touched backend, wrapper, adapter,
  and consumer code standards compliant during implementation
- Roadmap and README reconciliation needed to close Phase 5 accurately once the
  remaining event-contract work is complete
- Reclassification of broader binding-platform work out of Phase 5 when that
  work is not required for event-contract completion

### Out of Scope

- Expanding the supported client-facing binding surface beyond what is required
  to preserve the current event contract
- New C#, Python, or BEAM product-surface work that belongs to
  `IMPLEMENTATION-PLAN-pantograph-binding-platform.md`
- Scheduler V2, incremental graph execution, KV cache, or runtime-registry
  roadmap work except where they expose a concrete remaining Phase 5 gap
- Frontend-owned workflow semantics, optimistic event handling, or adapter-side
  synthetic lifecycle reconstruction

## Inputs

### Problem

The main Phase 5 plan is effectively complete, but the roadmap intentionally
keeps Phase 5 open because several follow-ons are still mixed together:

- a small set of possible remaining backend producer gaps
- a small set of possible remaining transport or binding acceptance gaps
- broader binding-platform work that is adjacent to Phase 5 but not the same as
  event-contract completion

Without a dedicated follow-on plan, implementation would likely drift in one of
two bad directions:

- adding more event and binding logic into already oversized files
- continuing to use Phase 5 as a catch-all bucket for binding-platform
  expansion, which would keep the roadmap stale and make it unclear when the
  event-contract target is actually done

### Constraints

- Backend Rust crates own canonical workflow semantics.
- `src-tauri` remains a composition and transport layer, not the owner of
  cancellation classification, graph-mutation meaning, or lifecycle policy.
- UniFFI and Rustler remain Layer 2 wrappers under
  `LANGUAGE-BINDINGS-STANDARDS.md`.
- Binding verification must satisfy both native-language and host-language
  expectations when a supported or experimental binding surface is touched.
- Existing public facades should stay facade-first unless an explicit break is
  approved.
- Touched directories under `src/` must keep README coverage current.

### Public Facade Preservation Note

This is a facade-first follow-on plan. Existing producer, wrapper, and
transport entry points should remain stable while implementation extracts
helpers and tightens remaining coverage behind the current boundary.

### Assumptions

- Most remaining Phase 5 work is now verification and producer-gap closure, not
  a broad semantic rewrite.
- Some roadmap items currently listed under Phase 5 are better classified as
  binding-platform follow-ons rather than event-contract blockers.
- The main immediate standards debt for this follow-on sits in oversized
  backend/wrapper/adapter files, not in missing README coverage.

### Dependencies

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`
- `IMPLEMENTATION-PLAN-pantograph-binding-platform.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-rustler-nif-testability-and-beam-verification.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/ARCHITECTURE-PATTERNS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CODING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/INTEROP-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/LANGUAGE-BINDINGS-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TESTING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CONCURRENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DOCUMENTATION-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DEPENDENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/FRONTEND-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CROSS-PLATFORM-STANDARDS.md`

### Affected Structured Contracts

- Backend-owned workflow-event emission and terminal envelope semantics
- Tauri workflow event transport and diagnostics projection payload shaping
- UniFFI and Rustler workflow-event / workflow-error wrapper contracts
- Any remaining non-streaming cancellation or interactive mismatch envelopes
- Roadmap classification between true Phase 5 event work and broader
  binding-platform follow-ons

### Affected Persisted Artifacts

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- This follow-on plan
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`
- `IMPLEMENTATION-PLAN-pantograph-binding-platform.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-rustler-nif-testability-and-beam-verification.md`
- Touched backend, wrapper, adapter, or consumer `README.md` files

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate insertion points that Phase 5 follow-on work is most likely to
touch already exceed decomposition thresholds from `CODING-STANDARDS.md`:

- `crates/node-engine/src/orchestration/executor.rs` is approximately 915 lines
- `crates/pantograph-uniffi/src/lib.rs` is approximately 1671 lines
- `crates/pantograph-uniffi/src/runtime.rs` is approximately 1149 lines
- `crates/pantograph-rustler/src/lib.rs` is approximately 2692 lines
- `src-tauri/src/workflow/headless_workflow_commands.rs` is approximately 1372
  lines
- `crates/pantograph-frontend-http-adapter/src/lib.rs` is approximately 853
  lines
- `src-tauri/src/workflow/event_adapter/tests.rs` is approximately 838 lines
- `packages/svelte-graph/src/stores/createWorkflowStores.ts` is approximately
  707 lines
- `packages/svelte-graph/src/components/WorkflowToolbar.svelte` is
  approximately 300 lines

This plan therefore requires decomposition review and local extraction in the
exact touched areas before more semantic work lands there. The goal is not a
codebase-wide cleanup; it is to ensure that the implementation and the
immediate surrounding code end in a clean standards-compliant state.

### Concurrency / Race-Risk Review

- Remaining producer-gap work touches cancellation, waiting, restart, and
  resume paths. Any change must preserve one backend-owned lifecycle authority
  and avoid split state-machine ownership across backend and adapters.
- Binding and transport acceptance tests will mutate temp roots, environment
  variables, compiled artifacts, native library search paths, and host-runtime
  lifecycle state. These suites must isolate durable state and serialize only
  where isolation is impossible.
- If BEAM-hosted verification is touched, NIF load/unload ownership and test
  isolation must be explicit and documented.

### Ownership And Lifecycle Note

- Backend crates own event meaning and terminal outcome classification.
- Wrappers own conversion, validation, and host-runtime bridges only.
- GUI/frontend code remains a read-only consumer of backend-owned workflow
  events.
- Host-language harnesses own test bootstrapping and shutdown only; they do not
  define contract meaning.

## Standards Review Passes

### Draft Pass

Initial draft built from:

- the roadmap’s explicit `Still missing` list under Phase 5
- the main Phase 5 plan’s `Follow-Ups`
- direct inspection of the immediate oversized insertion points

### Pass 1: Plan And Documentation Standards

Reviewed against:

- `PLAN-STANDARDS.md`
- `DOCUMENTATION-STANDARDS.md`

Resulting plan requirements:

- Use a dedicated follow-on plan instead of silently extending the completed
  main Phase 5 plan
- Keep milestones ordered by dependency and close-out clarity
- Update roadmap, main Phase 5 plan, and touched READMEs in the same slices
  that change truth or ownership

### Pass 2: Architecture And Coding Standards

Reviewed against:

- `ARCHITECTURE-PATTERNS.md`
- `CODING-STANDARDS.md`

Resulting plan requirements:

- Backend-owned event semantics must not move into Tauri, Svelte, UniFFI, or
  Rustler
- Decompose oversized touched files before adding more behavior
- Preserve public facades while extracting helpers under clearer ownership
- Keep single-owner state-machine logic for cancellation/wait/restart flows

### Pass 3: Interop And Language-Binding Standards

Reviewed against:

- `INTEROP-STANDARDS.md`
- `LANGUAGE-BINDINGS-STANDARDS.md`

Resulting plan requirements:

- Validate and shape data at boundaries, but keep canonical semantics in
  backend-owned or binding-neutral helpers
- Do not widen the public binding surface casually while finishing Phase 5
- Any wrapper-local logic shared by more than one binding must be extracted
  behind a framework-neutral owner
- Use the BEAM NIF plan only for the Rustler-specific verification gap that
  truly remains

### Pass 4: Testing And Concurrency Standards

Reviewed against:

- `TESTING-STANDARDS.md`
- `CONCURRENCY-STANDARDS.md`

Resulting plan requirements:

- Add real remaining acceptance only where a real unpinned surface still
  exists; do not add wrapper-only tests as a substitute
- For touched bindings, maintain native-language coverage and host-language
  coverage expectations
- Isolate durable state, temp roots, env vars, host-runtime lifecycles, and
  compiled artifacts
- Keep async flows non-blocking and cleanup deterministic

### Pass 5: Dependency, Frontend, And Cross-Platform Applicability

Reviewed against:

- `DEPENDENCY-STANDARDS.md`
- `FRONTEND-STANDARDS.md`
- `CROSS-PLATFORM-STANDARDS.md`

Resulting plan requirements:

- Prefer extraction and small local helpers over adding new dependencies for
  this follow-on
- If GUI surfaces are touched again, keep event consumption declarative and
  event-driven; no polling-based synchronization is permitted
- If host-language verification expands beyond current lanes, keep platform-
  specific behavior isolated to harness/build layers rather than business logic

## Milestones

### Milestone 1: Freeze Remaining Phase 5 Scope And Repair Immediate Insertion Points

**Goal:** Convert the remaining Phase 5 bullets into a concrete scoped worklist
and prevent more logic from landing in already non-compliant files.

**Current classification result:**

- `remaining_event_contract_gap`
  The currently documented event-contract remainder is now limited to
  validating whether any real backend-owned producer or transport-hosted
  surface is still unpinned beyond the already covered human-input pause,
  orchestration wait/cancel, graph-modification, incremental rerun,
  embedded-runtime non-streaming, frontend-HTTP, UniFFI, Rustler host-path,
  and Tauri adapter coverage.
- `binding_platform_follow_on`
  Curated client-facing binding surface policy, C# lane hardening, Python
  client binding work, and BEAM-hosted opaque-NIF verification now belong to
  the binding-platform / Rustler-specific plans rather than blocking Phase 5
  event-contract close-out by default.
- `defer_until_phase_6`
  Incremental graph execution and any future graph-surface expansion remain
  Phase 6 work unless a concrete failing event-contract case proves otherwise.

**Tasks:**
- [x] Audit every remaining roadmap Phase 5 item and classify it as one of:
      `remaining_event_contract_gap`, `binding_platform_follow_on`, or
      `defer_until_phase_6`.
- [ ] If backend producer work touches
      `crates/node-engine/src/orchestration/executor.rs`, extract focused
      helpers for terminal-event emission, cancellation/wait mapping, and test
      harness utilities before adding more behavior there.
- [ ] If UniFFI work touches `crates/pantograph-uniffi/src/lib.rs` or
      `crates/pantograph-uniffi/src/runtime.rs`, split event shaping, error
      envelope mapping, runtime-host APIs, and tests into focused modules
      before growing those files further.
- [x] If Rustler work touches `crates/pantograph-rustler/src/lib.rs`, split
      serializer/envelope helpers, workflow-host contract helpers, and
      Rustler-specific entrypoints into focused modules before more behavior
      lands.
- [ ] If remaining acceptance work touches
      `src-tauri/src/workflow/headless_workflow_commands.rs` or
      `crates/pantograph-frontend-http-adapter/src/lib.rs`, decompose those
      files first so transport acceptance does not deepen standards debt.
- [ ] If GUI acceptance requires touching
      `packages/svelte-graph/src/stores/createWorkflowStores.ts` or
      `packages/svelte-graph/src/components/WorkflowToolbar.svelte`, perform a
      decomposition review and extract focused reducers/helpers before adding
      more event semantics there.

**Verification:**
- File-size and responsibility review against `CODING-STANDARDS.md`
- `cargo check -p node-engine`
- `cargo check -p pantograph-uniffi`
- `cargo check -p pantograph-rustler`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check -p pantograph-frontend-http-adapter`
- `npm run typecheck` if any `packages/svelte-graph` files are touched

**Status:** Complete

### Milestone 2: Complete Backend-Owned Producer Parity

**Goal:** Ensure the remaining real backend producer paths emit or preserve the
canonical event contract consistently.

**Tasks:**
- [ ] Inventory every remaining backend-owned interactive and cancellable
      producer path still reachable after the current Phase 5 work and pin
      whether it should emit `WorkflowCancelled`, `WaitingForInput`,
      `GraphModified`, `IncrementalExecutionStarted`, or an explicit
      non-streaming envelope instead.
- [ ] Add canonical backend-owned emission or preserve explicit backend-owned
      non-streaming envelope behavior for any remaining path identified by the
      inventory.
- [ ] Remove any remaining transport-local cancellation or interactive
      classification that still exists only because a backend-owned producer
      path was previously incomplete.
- [ ] Keep any shared event/error shaping needed by multiple wrappers in a
      backend-owned or binding-neutral helper rather than duplicating wrapper
      policy.

**Verification:**
- `cargo test -p node-engine`
- `cargo test -p pantograph-embedded-runtime`
- `cargo test -p pantograph-workflow-service`
- Focused tests for any newly pinned cancellation, waiting, restart, or
  replayable producer path

**Audit result:** The remaining backend-owned producer inventory did not reveal
another uncovered interactive or cancellable producer path beyond the already
covered human-input pause, orchestration wait/cancel, graph-modification, and
incremental rerun flows.

**Status:** Complete

### Milestone 3: Close Remaining Real Transport And Binding Acceptance Gaps

**Goal:** Add only the real remaining acceptance paths still needed to trust
the current event contract end to end.

**Tasks:**
- [ ] Add cross-layer acceptance only for transport or runtime-hosted surfaces
      that remain explicitly unpinned after Milestone 2.
- [ ] For any touched UniFFI or Rustler contract path, keep native-language
      tests and host-language tests aligned with the current support tier
      instead of relying on Rust-only wrapper tests.
- [ ] If Rustler’s remaining gap is truly NIF-bound rather than pure contract
      shaping, apply the narrower Option 1 and Option 2 sequencing from
      `IMPLEMENTATION-PLAN-pantograph-phase-5-rustler-nif-testability-and-beam-verification.md`:
      pure-Rust extraction first, BEAM-hosted harness only for the remaining
      wrapper-specific behavior.
- [ ] Keep the accepted contract limited to the current backend-owned event and
      envelope surface; do not widen the public binding API opportunistically
      while adding acceptance.

**Verification:**
- Targeted Rust tests for touched backend and wrapper crates
- Real host-language smoke or acceptance runs for any touched supported or
  experimental binding lane
- Cross-layer acceptance path per touched surface as required by
  `TESTING-STANDARDS.md`

**Audit result:** The remaining transport/binding acceptance inventory did not
find another real event-contract surface still unpinned after the existing
embedded-runtime, frontend-HTTP, UniFFI, Rustler host-path, and Tauri adapter
coverage landed. The only still-distinct BEAM-specific work is opaque-NIF
verification, which is now tracked outside Phase 5 in the Rustler/binding
plans.

**Status:** Complete

### Milestone 4: Reclassify Non-Event Follow-Ons And Close Phase 5 Cleanly

**Goal:** Leave Phase 5 complete when the true event-contract work is done,
without losing visibility into broader binding-platform follow-ons.

**Tasks:**
- [ ] Update the roadmap’s remaining Phase 5 items so true event-contract gaps
      stay in Phase 5 and broader binding-platform product work points to
      `IMPLEMENTATION-PLAN-pantograph-binding-platform.md`.
- [ ] Update
      `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`
      to mark this follow-on plan as the source of truth for the remaining
      closure work.
- [ ] Update
      `IMPLEMENTATION-PLAN-pantograph-phase-5-rustler-nif-testability-and-beam-verification.md`
      if any Rustler-specific gap was closed or reclassified.
- [ ] Reconcile touched README files so the resulting backend/wrapper/adapter
      boundaries document the new clean state accurately.
- [ ] Close Phase 5 only when the remaining event-contract work is truly done;
      leave only binding-platform scope in the separate binding-platform plan.

**Verification:**
- Documentation review against `DOCUMENTATION-STANDARDS.md`
- Plan-to-roadmap consistency review
- Focused verification summary referencing the milestone checks already run

**Status:** Complete

## Risks And Mitigations

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Phase 5 keeps absorbing broader binding-platform work | High | Freeze item classification in Milestone 1 and route platform-surface expansion to the binding-platform plan |
| Remaining behavior lands in oversized files and creates new standards debt | High | Make decomposition review and local extraction mandatory before new behavior in the immediate touched areas |
| Wrappers regain canonical lifecycle logic | High | Extract shared logic to backend-owned or binding-neutral helpers and keep wrapper modules thin |
| Acceptance becomes wrapper-only and misses real host/runtime paths | High | Require real remaining cross-layer acceptance only where a real unpinned surface still exists |
| BEAM verification scope expands into full platform work prematurely | Medium | Keep Rustler follow-on scoped to pure-Rust extraction and minimal BEAM-hosted verification per the dedicated NIF plan |

## Recommendations

- Recommendation 1: treat Milestone 1 classification as blocking work. Without
  it, implementation will keep mixing event-contract closure with binding
  platform expansion.
- Recommendation 2: extract wrapper-local helpers before adding more tests or
  semantics to `pantograph-uniffi` and `pantograph-rustler`; those files are
  already far past safe review size.
- Recommendation 3: only keep Phase 5 open for real event-contract gaps. Once
  the remaining producer and transport gaps are closed, move broader binding
  platform work to the binding-platform plan and mark Phase 5 complete.

## Re-Plan Triggers

- The Milestone 1 inventory finds a remaining Phase 5 gap in a producer surface
  not currently represented in the roadmap
- Closing a remaining event-contract gap requires a new public binding surface
  rather than preserving the current one
- A touched wrapper or transport file needs more decomposition than this plan
  currently sequences
- BEAM-hosted verification turns out to require a broader ownership boundary
  than the current Rustler NIF plan assumes
- Incremental graph execution or another later phase lands first and creates a
  new backend-owned event producer path

## Completion Criteria

- Every remaining roadmap Phase 5 bullet is either implemented here or
  explicitly reclassified to the binding-platform plan or a later roadmap phase
- Any touched immediate insertion point ends in a standards-compliant state for
  file size, responsibility split, and ownership
- Remaining backend producer gaps for the current event contract are closed
  without moving semantics into wrappers, Tauri, or GUI code
- Remaining real transport and binding acceptance gaps for the current contract
  are closed with the required native-language and host-language verification
- Roadmap, plans, and touched READMEs agree on whether Phase 5 is complete and
  what work remains elsewhere

## Execution Notes

Update during implementation:
- 2026-04-18: Drafted dedicated follow-on plan after Phase 2 close-out and
  roadmap reconciliation showed that the remaining Phase 5 work no longer has a
  milestone-level active plan even though the roadmap still marks the target in
  progress.
