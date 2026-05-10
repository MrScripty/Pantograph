# Milestone 2: Retired Node Producers Removed

**Goal:** Ensure Pantograph no longer generates, suggests, or validates current
graphs that use retired direct diffusion nodes.

**Tasks:**

- [x] Update `pumas_dependency_runtime_probe` so diffusion models map to
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
- [x] Tests prove diffusion model probes produce canonical image-generation graph
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

- `cargo test -p workflow-nodes --features model-library puma_lib` passed after
  projecting Pumas selector/probe diffusion task labels to graph-facing
  `image_generation` while leaving factual `pipeline_tag: text-to-image`,
  `recommended_backend: diffusers`, and runtime engine hints intact.
- `cargo test -p pantograph-workflow-service graph::` passed after deleting
  compatibility migration helpers and rewriting persistence/canonicalization
  tests around no-rewrite stale graph behavior.
- `node --experimental-strip-types --test src/services/workflow/templateService.test.ts`
  passed after adding tracked saved workflow graph-shape assertions.
- `rg -n "diffusion-inference" crates src packages .pantograph/workflows -g '!target'`
  now reports only `.pantograph/workflows/README.md`, not tracked saved workflow
  bodies, bundled template bodies, or executable producers.

**Discovered Issues:**

- `crates/workflow-nodes/src/input/puma_lib.rs` is 1885 lines after this slice,
  above the standards decomposition-review threshold. This slice kept the file
  intact because the planned vertical change was narrow and a module split would
  have broadened the write set. Defer extraction until a dedicated
  workflow-nodes/Pumas maintenance slice can preserve the public facade.

**Status:** Completed on 2026-05-10
