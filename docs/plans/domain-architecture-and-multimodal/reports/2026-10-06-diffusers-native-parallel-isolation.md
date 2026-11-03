# Native Diffusers parallel test isolation

Frozen composition `9e8cd64cc64fa7ccb8dfd6cd85d37bff2282dfb6` is unchanged. Its exact
qualification JSON and all referenced logs are published by evidence-only
successor `fdb2bf2b0bbeb28fdde281b566e89b40f6e623e8` in the
[verbatim archive](../evidence/text-controls-main-9e8cd64/README.md). Original
parallel failure and serial pass remain distinct; neither was rewritten.

The defect is shared test state. The scheduler test globally replaced
`DDIMScheduler.step`, and its counter observed calls from the other native tests
while Torch released the GIL. Parallel scheduling determines whether contamination
occurs, but the failure mechanism is deterministic once a foreign call overlaps
the hook. The production helper does not replace scheduler class methods.
No production change is needed for this failure; this diagnosis is not a general
claim that all possible production races or model implementations are qualified.

The [reproduction script](../evidence/diffusers-parallel-isolation/diagnose-diffusers-foreign-step.py)
loads the exact frozen native test/helper source from Git, retaining the actual
helper, native forwards, global hook and exact-two-calls assertion. It inserts
one real foreign-thread DDIM step while the hook is installed. The
[complete diagnostic traceback](../evidence/diffusers-parallel-isolation/foreign-step-reproduction.log)
shows three observed calls instead of two and the same assertion failure. The
original log has PyO3's traceback-object rendering; this diagnostic provides the
Python frames and observed count rather than inventing missing original frames.
Run the script from a repository checkout with the recorded CPU dependencies;
exit status one is the expected reproduced failure.

This separate successor changes only the native test harness. Thread-local
`sys.setprofile` observes the actual scheduler step code and instance during the
owned helper call, then restores the prior profiler in `finally`. No scheduler
class method is changed. During the first owned step, two real same-class steps
complete in another thread. Assertions require the foreign thread to differ,
both foreign steps to execute, exactly two owned steps on the same private
scheduler, unchanged residency/config and exact agreement with the native oracle.
Python failures now print their full traceback. No test mutex, serial harness,
tolerance increase, timeout increase or changed test count/ignore status is used.

The original parallel command now passes all three tests:

```sh
cargo test --locked --offline -p inference --no-default-features --features backend-llamacpp,backend-pytorch --test diffusers_guidance_native -- --ignored
```

`RUST_TEST_THREADS` is unset. Python 3.12.3, Torch 2.14.1+cpu, Transformers 4.53.3
and Diffusers 0.39.0 match the archived environment; no dependency was installed.
The full inference package passes 821 checks plus one doctest, with six existing
optional native/doctest cases ignored by that ordinary invocation. Strict
all-target inference Clippy, Rust format, critical gate, staged/range traceability
and nine ONNX no-build-download graphs pass. Native parallel success logs and
this deterministic reproduction are preserved with hashes. Other packages and
frontend are unchanged and their frozen composition results are not counted as
reruns. GPU, pretrained/custom models, full worker dependency import, desktop and
ONNX execution remain unqualified. Parent owns PR/review/merge; the separate
planned text seed may resume after this bounded parallel publication gate.
