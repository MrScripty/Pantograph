# Milestone 2: Retired Node Producers Removed

**Goal:** Ensure Pantograph no longer generates, suggests, or validates current
graphs that use retired direct diffusion nodes.

**Tasks:**

- [ ] Update `pumas_dependency_runtime_probe` so diffusion models map to
  canonical `llm-inference` with image-generation task semantics.
- [ ] Audit built-in templates, fixtures, and tests for direct
  `diffusion-inference` usage.
- [ ] Treat saved workflow cleanup and producer cleanup as one slice: fixed
  workflow files are not complete while any tracked producer can still emit the
  retired graph shape.
- [ ] Keep tests that prove retired nodes are rejected and delete or rewrite
  tests that expect current load/save paths to silently normalize retired
  diffusion as a valid graph shape.
- [ ] Split canonicalization so current app paths run current graph
  normalization only. Retired-node migration must be removed from those paths
  and deleted unless a read-only stale-diagnostic fixture requires isolated
  historical sample data.
- [ ] Preserve useful current graph repairs, such as current inference setting
  and edge normalization, without treating retired node types as valid current
  graphs.
- [ ] Audit persistence, graph session state, graph edit-session mutation, and
  workflow listing tests because the canonicalization split changes more than
  file save/load behavior.
- [ ] Keep current normalization as a sync domain function that accepts
  validated graph types. Stale-diagnostic paths may classify retired shapes,
  but no current path returns or applies migration behavior.
- [ ] Update workflow-node and template documentation where stale wording still
  implies migration compatibility or old-shape support.

**Verification:**

- `rg "diffusion-inference"` only finds retired-node guardrails, stale
  diagnostic fixtures, or historical docs.
- Tests prove diffusion model probes produce canonical image-generation graph
  recommendations.
- Tests prove the repaired tracked workflows and the probe/template producers
  agree on the same canonical `puma-lib -> llm-inference -> image-output`
  graph shape.
- Tests prove retired direct diffusion remains non-executable and unregistered.
- Persistence/session tests prove current load/save/session paths do not
  rewrite retired node types, while current graph normalization still handles
  valid current graph repairs.
- Test review confirms graph session canonicalization does not reintroduce
  retired-node migration through edit-session state refresh.
- Code search confirms compatibility-only migration helpers are deleted or
  isolated as stale-diagnostic fixtures, not reachable app code.
- Tests prove current normalization does not perform filesystem, Pumas, worker,
  or artifact I/O.

**Status:** Not started
