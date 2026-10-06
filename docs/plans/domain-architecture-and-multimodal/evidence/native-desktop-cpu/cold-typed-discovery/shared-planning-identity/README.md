# Shared graph-to-scheduler planning identity repair

Parent source is 3b9e731ac678b00463d4f1b48ceb78322a096c04, tree
25c78817472a64a5f5a66c48dca7b9aaca4c0c2c. The predecessor native failure
and all its members remain preserved in typed-proof-successor/readiness-admission.

Graph Resolve previously omitted task_type and included host platform; scheduler
readiness included task_type and omitted platform. Both fields are hashed. One
workflow-service constructor now supplies explicit task and host platform to both
owners. Generic hash/equality validation is unchanged.

The regression-before-repair.patch.gz can be applied to the parent to replay the
actual graph producer -> saved snapshot round-trip -> scheduler readiness test.
The positive test failed with the nested saved-proof mismatch. That initial log
also retains a negative-test assertion failure because the wrapper Display hides
its nested invalid-request message; the final regression inspects the error variant.
The final positive and changed model/revision/artifact/device/runtime controls pass,
and another graph revision cannot publish the proof. No requirements ID or ready
receipt is manufactured. These use controlled descriptors; native GUI/Pumas/actual
CPU output qualification is a separate hosted check.

Final checks: 928 workflow-service unit tests and 60 integration tests; strict
selected-crate all-target Clippy; 537 default-feature embedded tests, including
real-Pumas cold Candle descriptor/typed-proof tests; formatting, critical and
accessibility gates. All build feature graphs were inspected, with pinned Pumas
26a84e323cae566a46a8f76bef48fa1010aed48b, ORT load-dynamic / ort-sys
disable-linking, no forbidden download features, and ORT_SKIP_DOWNLOAD=1.

The Candle-only embedded test attempt failed to compile because existing test
constructors call InferenceGateway::new, gated by backend-llamacpp. Its complete
log and graph are retained; the established default-feature suite passes. The
initial traceability invocation lacked an explicit staged/range mode and failed;
the final staged check is recorded separately. Existing dependency warnings are
preserved. Logs/feature graphs/patch are losslessly gzipped with deterministic mtime.
