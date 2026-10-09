# Limited resident CPU calibration checkpoint

Remote main verified at `038dacaaa98ebd007c32e13d4608726ca5ccf63a` on 2026-10-07. This local slice builds on `c5fa298d`; PR65's five files remain read-only. Its CPU capability enrollment is separate from calibration.

Opt in at gateway construction to a private Candle CPU thread pool (1–4 threads). The actual F32 BERT loader binds the bytes passed to tensor deserialization, retained configuration/tokenizer/pooling bytes, selected model reference, implementation and owner configuration. Observations stay in that process/owner; they are not portable CPU capacity measurements. The default constructor and scheduler stay unchanged.

Collect only successful selected embedding calls after actual worker drain and cancellation checks. Keep at most 64 exact single-text keys and eight samples per key, with a monotonic age limit and at least three warm samples. Explicit reload, failed publication and stop invalidate evidence; repeated verified reuse retains observations with their original generation. Controlled clocks are synthetic and excluded from public production lookup.

The implemented comparison API chooses between two exact single-text workloads on the currently resident CPU owner. It compares the maximum observed execution-plus-drain cost, preserving baseline order for ties or incomplete evidence. This is an empirical estimate, not a deadline bound. It does not admit or dispatch workflow tasks.

Independent design/source review found an architectural blocker to the proposed first-two Ready-task adapter: TaskWorker always invokes the batch broker and runs concurrent branches. A singleton serialized dispatch contract is needed before that adapter is safe. Future integration must snapshot the authoritative first-two population and revalidate both Ready proofs, inputs, actual selection and generation before start, then revalidate actual owner residency at execution. This slice therefore leaves workflow files untouched.

Repeated typed selected-embedding service calls on a retained gateway populate the store. Existing Ephemeral workflow release clears calibration; it cannot bootstrap warm samples by pretending residency persists. There is no transfer/release timing, cold-runtime ranking, GPU claim, native batching, remote-node planning or default-policy change. Those remain prerequisites for broader completion plans.

Acceptance: controlled clocks through the real synthetic-BERT service path, warm population and lower observed completion comparison, exact-input misses, age/clock faults, cancellation/failure/drop exclusions, reload/stop invalidation, bounded eviction and atomic stale-generation refusal. Workflow first-two cohort/race tests and realistic host dispatch overhead checks remain prerequisites for the blocked adapter. Build/test with `ORT_SKIP_DOWNLOAD=1`, cached dependencies only, no real model run or download. Independent source review precedes completion.
