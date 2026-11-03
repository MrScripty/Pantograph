# Private node validation lifecycle context

Fresh PR #30 aggregate Clippy cleared inference and stopped at node-engine's private record_task_validation_failure_lifecycle helper. Group its existing eight values into TaskValidationFailureContext, keep ExecutorExtensions separate, and migrate the single caller. Both the new private type and helper retain the inference-nodes feature gate. The sink lookup and complete event loop are byte-identical after destructuring.

The two existing contract-only depth/video tests already assert complete request/task/backend/runtime/model attribution, Started/Failed/Cleanup ordering, and failure-only option diagnostics and artifact references. Hosted commands now explicitly enable inference-nodes, require nonzero exact-module discovery, and execute these tests. Public API, task support policy, generated request IDs, diagnostics and sink failure handling are unchanged.

Formatting, whitespace and unchanged event-body comparison pass locally. No local Rust execution is claimed. Root source review accepted frozen tree f20d0e683c0057dfa7337abe8b0e56f6e2a6effa after inspecting the exact three-file diff, unchanged eight-value grouping, feature gates, single caller and nonzero test discovery. This is source review, not Rust execution or independent-agent review. Fresh hosted tests/Clippy qualification and the full combined external review remain pending; no lint suppression is added.

## Feature-enabled fixture compilation correction

The initial exact-head focused job 111207010012 at 92ca205 failed before lifecycle test discovery: MockKvBackend::stop resolved Result through super::* to node-engine's one-parameter NodeEngineError alias (E0107/E0053). This fixture file is unchanged from main 4938e405. Qualify std::result::Result explicitly, matching InferenceBackend::stop and neighboring mock methods, without changing the successful mock response. The two lifecycle tests have not yet executed; the feature/nonzero gate remains in place. Root source review accepted correction tree 39a1d45aaf80fe05264f5f26357e3cb2d00a9875. Corrected hosted execution remains pending.
