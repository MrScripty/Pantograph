# Wave 05 Host Diffusion Preview Producer Audit

## Scope

Read-only audit of the remaining producer-specific streamed preview and
diffusion child/revision artifact tasks.

## Findings

- No producer-specific diffusion preview event path exists yet.
- `crates/pantograph-embedded-runtime/src/python_runtime_bridge.py` routes
  `diffusion-inference` through a synchronous `_run_diffusion` call that returns
  final image outputs.
- `crates/inference/torch/worker.py` runs diffusion generation synchronously and
  returns final `image_base64` output without a callback, generator, preview
  payload, or per-step event emission.
- Existing generic stream artifactization can handle `image_base64` stream
  chunks after a producer emits them, but it does not create first-class
  child/revision artifact relationships.
- `ArtifactDescriptor` currently has no relationship fields such as
  `parent_artifact_id`, `revision_of`, `revision_index`, or `artifact_role`.
  Child/revision artifact semantics therefore require a separate
  workflow-service contract/store slice.

## Recommendation

Leave the diffusion child/revision task open. Implement producer preview
emission first in the Python diffusion bridge and worker, then add first-class
artifact relationship fields and store/query behavior in a separate contract
slice.
