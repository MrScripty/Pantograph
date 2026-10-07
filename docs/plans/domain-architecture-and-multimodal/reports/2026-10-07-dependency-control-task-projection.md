# Dependency environment control task projection

The preserved native run at `0e3e47a5` accepted and enqueued a saved CPU
embedding graph, then stopped with `runtime scheduler task 'deps' cannot
continue from Invalid`. Its current validation was executable for owner revision
`d0f7a8eb92be581d`; the typed dependency action resolved for that same graph,
revision and validation session. The evidence does not indicate stale validation.

The dependency-environment descriptor defines a manual control association whose
actions belong to workflow-service. Task projection already excluded its sidecar
edge from dataflow inputs, but still emitted the control node as an unsupported
scheduler task. Initialization therefore assigned Invalid at state version 1,
before any dependency readiness transition. The runtime rejection was correct
for the incorrectly projected task.

Task projection now omits only a control node with a validated, singular,
canonical association to an existing inference node and no other incident
edges. It reuses the existing typed action-subject resolver. Authored nodes,
edges, topology and semantic fingerprints remain intact. Malformed associations,
ordinary dataflow-bound controls and other unsupported tasks remain fail-closed.
No Invalid transition, runtime admission policy or readiness proof is bypassed.

Ten focused binding tests and all 931 workflow-service unit tests pass. New
regressions exercise the real initial-state constructor, preserve inference
dependency-readiness sources and AwaitingInputs, cover eight malformed or
dataflow-bound associations, and retain missing-inference-facts failures. A
separate baseline worktree holds the prior implementation plus the new tests;
its positive control projection regression fails with an extra dep-env task.
Formatting, critical checks and scheduler-only public-surface checks pass.
Strict Clippy stops on pre-existing unused SelectedTextLoad fields in the
inference dependency; its failure is retained without unrelated source changes.

Complete effective all-target normal/build/dev Cargo graphs are inspected before
compilation: dynamic ORT with disabled linking, no download-binaries or model
download features, and Pumas `26a84e32`. ORT_SKIP_DOWNLOAD=1 is set defensively.
This is controlled CPU contract evidence. Actual native runtime admission, CPU
output, artifact Read and inspector display require the separately bounded
continuation. GPU, pretrained models, real-user discovery and full production
loader qualification remain outside these tests.
