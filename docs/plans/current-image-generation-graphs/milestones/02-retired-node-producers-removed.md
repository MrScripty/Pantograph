# Milestone 2: Retired Node Producers Removed

**Goal:** Ensure Pantograph no longer generates, suggests, or validates current
graphs that use retired direct diffusion nodes.

**Tasks:**

- [ ] Update `pumas_dependency_runtime_probe` so diffusion models map to
  canonical `llm-inference` with image-generation task semantics.
- [x] Audit built-in templates, fixtures, and tests for direct
  `diffusion-inference` usage.
- [x] Treat saved workflow cleanup and producer cleanup as one slice: fixed
  workflow files are not complete while any tracked producer can still emit the
  retired graph shape.
- [x] Keep tests that prove retired nodes are rejected and delete or rewrite
  tests that expect current load/save paths to silently normalize retired
  diffusion as a valid graph shape.
- [x] Split canonicalization so current app paths run current graph
  normalization only. Retired-node migration must be removed from those paths
  and deleted unless a read-only stale-diagnostic fixture requires isolated
  historical sample data.
- [x] Preserve useful current graph repairs, such as current inference setting
  and edge normalization, without treating retired node types as valid current
  graphs.
- [x] Audit persistence, graph session state, graph edit-session mutation, and
  workflow listing tests because the canonicalization split changes more than
  file save/load behavior.
- [x] Keep current normalization as a sync domain function that accepts
  validated graph types. Stale-diagnostic paths may classify retired shapes,
  but no current path returns or applies migration behavior.
- [x] Update workflow-node and template documentation where stale wording still
  implies migration compatibility or old-shape support.

**Verification:**

- [x] `rg "diffusion-inference"` only finds retired-node guardrails, stale
  diagnostic fixtures, or historical docs.
- [ ] Tests prove diffusion model probes produce canonical image-generation graph
  recommendations.
- [x] Tests prove the repaired tracked workflows and the probe/template producers
  agree on the same canonical `puma-lib -> llm-inference -> image-output`
  graph shape.
- [x] Tests prove retired direct diffusion remains non-executable and unregistered.
- [x] Persistence/session tests prove current load/save/session paths do not
  rewrite retired node types, while current graph normalization still handles
  valid current graph repairs.
- [x] Test review confirms graph session canonicalization does not reintroduce
  retired-node migration through edit-session state refresh.
- [x] Code search confirms compatibility-only migration helpers are deleted or
  isolated as stale-diagnostic fixtures, not reachable app code.
- [x] Tests prove current normalization does not perform filesystem, Pumas, worker,
  or artifact I/O.

**Verification Results:**

- `cargo test -p pantograph-workflow-service graph::` passed after deleting
  compatibility migration helpers and rewriting persistence/canonicalization
  tests around no-rewrite stale graph behavior.
- `node --experimental-strip-types --test src/services/workflow/templateService.test.ts`
  passed after adding tracked saved workflow graph-shape assertions.
- `rg -n "diffusion-inference" crates src packages .pantograph/workflows -g '!target'`
  now reports guardrail/test/doc references only, not tracked saved workflow
  bodies or bundled template bodies.

**Remaining Follow-Up:**

- The Pumas dependency/runtime probe still needs its own focused slice to prove
  diffusion models produce canonical image-generation graph recommendations
  without relying on legacy direct-diffusion routing.

**Status:** Partially completed on 2026-05-10
