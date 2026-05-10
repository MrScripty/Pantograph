# Milestone 1: Current Juggernaut Graph Slice

**Goal:** Replace the broken saved Juggernaut graph with one current canonical
image-generation graph and remove the duplicate workflow.

**Tasks:**

- [ ] Identify the canonical Juggernaut workflow file to keep.
- [ ] Delete the duplicate saved workflow file.
- [ ] Update the retained graph to mirror the Tiny SD Turbo current pattern:
  `puma-lib` -> canonical `llm-inference` -> image output.
- [ ] Use the current Pumas model id
  `diffusion/rundiffusion/juggernaut-x-v10`.
- [ ] Remove embedded stale runtime definitions and output payloads from the
  saved workflow.
- [ ] Store stable model identity and graph intent only; do not retain raw
  Pumas paths, previous run outputs, or generated media bodies in the saved
  workflow file.
- [ ] Validate saved workflow JSON through backend parsers rather than relying
  on frontend-only shape assumptions.
- [ ] Keep graph labels and user-facing names clear without relying on stale
  node types.
- [ ] Update `.pantograph/workflows/README.md` so tracked workflow examples
  describe the current image-generation graph shape and retained Juggernaut
  file.

**Verification:**

- Unit or fixture test loads the saved Juggernaut graph and asserts no
  `diffusion-inference` node exists.
- Test asserts exactly one Juggernaut workflow appears in workflow listing.
- Test asserts the Puma-Lib node carries stable model identity and image
  generation receives `pumas_model_ref` / package facts.
- Test asserts stale or raw local model paths are not persisted in the repaired
  tracked workflow.
- Frontend graph smoke or snapshot test verifies the workflow materializes with
  visible ports and edges.
- Test or fixture notes confirm the graph shape is model-family agnostic and
  does not hardcode behavior beyond the Juggernaut model id selected for this
  saved workflow.

**Status:** Not started
