# Candle failed-job lifecycle recovery

This narrow repair is a separate descendant of the frozen embedding candidate
`05ea1f2c7818a77b632696b77ec50b2a97c8b436`, on branch
`fix/candle-embedding-failed-job-recovery`. It is ready for the same independent
reviewer. The original candidate is unchanged.

## Problem and change

The original completion drain observed worker termination but retained a failed
completion indefinitely. Non-cancellation failures were then replayed by every
stop or reload, preventing recovery while the model still reported ready.

Lifecycle drain now waits for the retained completion, then retires that exact
completed job. It compares the job's shared stop identity before clearing the
slot, so a concurrent replacement job cannot be removed. The request keeps its
own shared completion and observes its original Config or Inference error;
lifecycle cleanup does not replay that request failure.

There is no early retirement or preemption of running native work. The existing
caller-loss test still proves that drain stays pending until the actual worker
exits. Only the backend-local job drain and its regression tests change; model
loading, numerical recipes, gateway ownership, dependencies, audit gates, and
the frozen fixture identities remain unchanged.

## Validation

- Focused Candle tests: 25 passed, including both Config and Inference failure
  followed by stop/reload or direct reload, repeated stop, late observation of
  the original request error, retained-worker caller loss, and cancellation.
- The Inference regression uses a controlled fault in the loaded real tokenizer:
  removing padding produces an inconsistent batch shape. It tests lifecycle
  recovery from the genuine execution error; it is not a model-quality claim.
- Actual CPU F32 Candle forwards still match both independent untrained
  Transformers references, with maximum absolute error `5.9604645e-8`.
- Candle-enabled inference library suite: 449 passed.
- Candle-enabled inference Clippy, all targets: completed with only the existing
  selected-text dead-code warning; no warning from the repair.
- Regression tests applied to the original `05ea1f2c` production drain both fail
  on failed-job stop, demonstrating the tests cover the reviewed defect.
- Formatting, critical anti-pattern, staged/range traceability, and diff checks
  passed. No gate or feature profile was weakened.

The already documented Pumas fixture integration failure was not reclassified
or repeated for this isolated job-lifecycle repair. GUI, GPU, pretrained semantic
qualification, and live Pumas embedding artifact handoff remain unexecuted.

## Preserved consumer checkpoint

The separate provisional consumer branch `feature/pumas-full-facts-consumer`
is checkpointed at `a02b652e6957b3d01834820d0b22558b52837ccf`, tree
`156bf7f1308be678c4993759d791b7dee332fa1c`, based on accepted owner/client
`1c0d86dc6275ac2be466c3f918d2fcd35be99cb6`. It pins producer
`5be6d967dbd5c0ff7449f342e77f05f9cd645a8e`, whose inspected tree is
`8e693fe410dac9142e6213430638e7180e5643c7`.

That checkpoint contains the six reserved consumer files and pin/lock updates,
but it is **unfinished and unqualified**. Its build is blocked by the existing
ONNX Runtime CDN `403` before the consumer code can be compiled. The producer
default feature profile remains intact, and no network/security workaround was
attempted. Independent producer review and remaining consumer tests/integration
are pending. Consumer changes are not part of this Candle repair candidate.

Neither branch was pushed, merged, or submitted for external review. The parent
owns publication, independent review, and coordinated integration.
