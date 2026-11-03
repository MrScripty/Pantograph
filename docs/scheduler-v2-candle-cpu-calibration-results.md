# Resident CPU calibration slice

Local branch: `scheduler/candle-cpu-calibration`. Base: `c5fa298d19f1b2a47c3cbad29845eaf0c0a1906f`. Design checkpoint: `c628c04d`. Remote main was verified as `038dacaaa98ebd007c32e13d4608726ca5ccf63a`, rather than assuming the older snapshot. The preserved `scheduler/owned-dependency-audio-qualification` branch remains at `fd224c366645c19f868c36ed331264a82b79db68`.

## Delivered path

`InferenceGateway::new_calibrated_candle_cpu(config)` constructs the actual CPU F32 BERT owner; `with_candle_cpu_calibration(config)` optionally installs it before a built-in gateway's first runtime. Injected backends cannot enable it. Configuration owns a private Rayon pool of one to four workers, a three-to-eight sample minimum and an age limit in `(0, 1 hour]`. No global environment or network setting changes are made.

The actual loader hashes the same weight buffer passed to tensor deserialization and the retained config, tokenizer and pooling recipe bytes, width, implementation source and locked dependency identities. Actual loaded instance identity prevents another backend created through the read-only registry from being confused with the resident gateway owner. CPU/F32 is the executed device/dtype; there is no invented hardware serial, pinned-core identity or portable capacity claim. Effective GEMM partition settings and tokenizer parallelism, including its atomic override, are checked at load, actual pool execution, collection and query.

Repeated successful typed `execute_selected_embedding_with_cancellation` calls while this owner remains resident populate a private ring. Only warm, exact single-text calls enter it after actual execution, native worker drain, output/usage validation and cancellation checks. There is no external timing ingestion. At most 64 text digests and eight observations per digest are retained. Observations retain their original generations across verified reuse; reload, failed publication, stop, clock faults, changed effective configuration and abandoned/incomplete service invalidate evidence. Retained load/execution custody remains busy until actual drain retires it.

`compare_resident_cpu_embeddings(model_ref, [first, second])` performs a bounded local comparison: both exact texts must have enough fresh warm observations on that same loaded instance. It returns their maximum observed execution-plus-drain costs and the lower-cost position, preserving the first position on ties. Missing, synthetic, stale, busy or mismatched evidence returns `None`; callers preserve existing order. `is_current()` is a generation-only atomic check, not a dispatch/residency lease. The measurements are empirical estimates, not worst-case guarantees.

Query work hashes at most 128 KiB, scans at most 128 stored keys and 16 samples, and uses nonblocking custody/store access. It does no model loading, tensor execution, tokenization or artifact-file reads. Clock/effective-settings probes read the live process configuration. Default construction does no calibration clocks, hashes, pool creation or recording.

## Implemented and missing production evidence

| Area | Present | Still required for broader scheduling |
| --- | --- | --- |
| CPU | Actual CPU F32 owner, private worker budget, effective worker conditions | Available/pinned CPU capacity, shared host contention and capacity admission evidence |
| GPU | Existing capability contracts preserved | Measured GPU capacity/device ownership and calibrated costs; no GPU support advertised here |
| Priorities and queues | Existing priority/FIFO, starvation boosts and one-position warm window | Explicit serialized dispatch/cohort contract for activating this comparison |
| Batching | Existing embedding and broker paths preserved | Broker-aware calibrated plan semantics; this API qualifies only one exact text |
| Residency | Existing runtime/session ownership and real cleanup preserved | Explicit retention/lease contract for any workflow adapter; Ephemeral cleanup invalidates evidence |
| Timing | Successful warm execution plus actual worker drain | Admission/custody wait, preparation, cold/reuse load, transfer, release/reconcile and realistic host dispatch overhead |
| Remote nodes | Existing contracts unchanged | Authoritative remote availability, transfer/network and timing evidence |

## Architectural stop

Independent source review found that TaskWorker always invokes a batch broker (up to eight assignments) and uses a `JoinSet` for concurrent branches. It is not an established singleton serial dispatch seam. The proposed first-two Ready-task adapter therefore was not added. The next architectural decision is an explicit singleton serialized dispatch mode or an owner-held queue boundary that preserves admission, batching and session ownership.

That adapter must bound borrowed graph/population checks before cloning, snapshot both first Ready tasks and complete population order, revalidate both materialized inputs/Ready proofs and actual selection, and revalidate residency when execution acquires the owner. It must recompute after actual cleanup and refuse unsupported routes. Realistic host dispatch overhead and simulator differential tests remain acceptance requirements before default activation.

The production population path is retained typed gateway service calls. Existing Ephemeral workflow executions cannot bootstrap three warm samples by pretending their owner survives cleanup. The unavailable POC archive did not block this fresh Rust implementation; no C3 source or simulator-equivalence claim is made. This is a real collection/comparison path, not the completed research scheduler.

## Validation

The combined-feature library suite across eight packages passed **2,741 tests, zero failures, four existing ignored tests**. After the final worker-condition refinement and public API smoke addition, the complete inference library passed **830 tests, zero failures**. The independent reviewer approved the narrow slice with no remaining findings and separately executed **10 calibration tests, all passing**.

Fixtures exercise actual Candle forward on the existing untrained tiny BERT models. Controlled-clock cases prove phase costs, exact input/model misses, freshness, bounded ring eviction, loaded-buffer changes, failed publication, clock faults with populated history, caller abort while the actual worker remains blocked, and post-execution cancellation. An isolated process changes tokenizer parallelism during actual forward and restores it before caller-side probes, proving worker conditions are checked. Controlled clocks remain synthetic and public production lookup refuses them. The real-clock smoke verifies both public constructors return a comparison after three warm calls per text and refuse after stop; it asserts neither winner nor wall-time performance.

All-target Clippy passed for the eight combined-feature packages with `--locked --offline -- -D warnings -A dead_code`; the allowance is for existing inference fields. Final inference all-target Clippy passed again after the public smoke addition. Candle-only and default-feature inference library checks passed. `cargo fmt --all --check` and `git diff --check` passed. No pretrained/production model, runtime or ONNX download was used.

Logs are in `/workspace/pantograph-cache/`: `candle-cpu-calibration-tests.log`, `candle-cpu-calibration-inference-final.log`, `candle-cpu-calibration-clippy.log`, `candle-cpu-calibration-clippy-inference-final.log`, and `candle-cpu-calibration-feature-checks.log`. The first aggregate launch after environment restart lacked the cached libpython search path and exited before executing tests; the corrected environment passed.

Reproduction uses the cached tools and Python libraries:

```sh
source /workspace/pantograph-tools/activate.sh
export ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1
export PYO3_PYTHON=/workspace/Pantograph/.venv/bin/python
export LD_LIBRARY_PATH=/workspace/pantograph-tools/python/cpython-3.12.3-linux-x86_64-gnu/lib
export PYTHONPATH=/workspace/Pantograph/.venv/lib/python3.12/site-packages
export XDG_CONFIG_HOME=/workspace/pantograph-cache/test-config
cargo test -p pantograph-timing-contracts -p inference -p node-engine \
  -p pantograph-scheduler -p pantograph-workflow-service \
  -p pantograph-embedded-runtime -p pantograph-runtime-host-contracts \
  -p pantograph-inference-interface-contracts --lib --no-default-features \
  --features backend-llamacpp,backend-candle,backend-pytorch,backend-audio,standalone \
  --locked --offline
```

Only inference code/manifests and these repository reports change. PR65's five files, workflow selection, inspector normalization and existing audio/rerank adapters remain untouched. No public write, push, merge, credential change or default-policy change is authorized or performed.
