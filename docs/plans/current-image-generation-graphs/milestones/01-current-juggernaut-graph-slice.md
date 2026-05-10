# Milestone 1: Current Juggernaut Graph Slice

**Goal:** Replace the broken saved Juggernaut graph with one current canonical
image-generation graph and remove the duplicate workflow.

**Tasks:**

- [x] Identify the canonical Juggernaut workflow file to keep.
- [x] Delete the duplicate saved workflow file.
- [x] Update the retained graph to mirror the Tiny SD Turbo current pattern:
  `puma-lib` -> canonical `llm-inference` -> image output.
- [x] Use the current Pumas model id
  `diffusion/rundiffusion/juggernaut-x-v10`.
- [x] Remove embedded stale runtime definitions and output payloads from the
  saved workflow.
- [x] Store stable model identity and graph intent only; do not retain raw
  Pumas paths, previous run outputs, or generated media bodies in the saved
  workflow file.
- [x] Validate saved workflow JSON through backend parsers rather than relying
  on frontend-only shape assumptions.
- [x] Keep graph labels and user-facing names clear without relying on stale
  node types.
- [x] Update `.pantograph/workflows/README.md` so tracked workflow examples
  describe the current image-generation graph shape and retained Juggernaut
  file.

**Verification:**

- [x] Unit or fixture test loads the saved Juggernaut graph and asserts no
  `diffusion-inference` node exists.
- [x] Test asserts exactly one Juggernaut workflow appears in workflow listing.
- [x] Test asserts the Puma-Lib node carries stable model identity and image
  generation receives `pumas_model_ref` / package facts.
- [x] Test asserts stale or raw local model paths are not persisted in the repaired
  tracked workflow.
- [x] Frontend graph smoke or snapshot test verifies the workflow materializes with
  visible ports and edges.
- [x] Test or fixture notes confirm the graph shape is model-family agnostic and
  does not hardcode behavior beyond the Juggernaut model id selected for this
  saved workflow.

**Verification Results:**

- `node --experimental-strip-types --test src/services/workflow/templateService.test.ts`
  passed and now covers tracked image-generation saved workflows.
- `cargo test -p pantograph-workflow-service graph::` passed, including backend
  parser/canonicalization coverage for current graph loading and persistence.

**Status:** Completed on 2026-05-10
