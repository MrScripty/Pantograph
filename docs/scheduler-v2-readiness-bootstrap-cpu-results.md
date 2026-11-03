# Native CPU readiness bootstrap qualification

This continues `69440572383d08aedf604031b3e8bf58d25ac376` on local branch
`scheduler/readiness-bootstrap-cpu-2026-10-08`. Public Pantograph main was
verified at `038dacaaa98ebd007c32e13d4608726ca5ccf63a`. The public cold Candle
candidate `6338b71a55371317c5ace6adbeacdec469c96950` was inspected and cherry-picked
locally as `45411c40`. No public writes or merge were made.

## Diagnosis and repair

An empty readiness snapshot could not seed its own authoritative requirements:
the producer needed a registry payload, while the runner looked for a resolved
snapshot to obtain that payload. The graph Resolve action also discarded its
result rather than handing it to the hosted requirements registry. Separately,
owned producer work used Check while the readiness consumer looked for Resolve.

The graph facts owner can now resolve requirements independently of readiness.
The service preserves the full requested identity and requirements proof ID,
accepts owner-selected default bindings only when authored selections are empty,
and publishes a validated payload under a final fence that covers graph-session
membership, revision/fingerprint and the executable validation generation.
Graph edits, validation replacement, closure and foreign identities refuse the
handoff. Explicit authored binding choices must remain exact; every selected
binding must have a corresponding payload binding.

The runner uses a fresh matching registry payload before trying its legacy
Resolve seed, queues an actual Check, and consumes the resulting Check evidence.
A legacy Resolve Ready snapshot is accepted only when the Check snapshot is
absent, with the provider's exact absence diagnostic. Factual Missing, stale or
failed Check results cannot be replaced by that compatibility fallback.

The native profile supports only explicitly selected Candle CPU embedding on
the current OS/architecture, bounded identity fields, no path hint, migration
diagnostic, override or trait intent, and either empty authored choices or its
two exact bindings. It requires actual gateway embedding capability, an actual
CPU candidate, fresh valid Pumas embedding facts without custom code, and an
owner query confirming no declared package dependencies. Read-only/local RPC
sources cannot supply that last proof and retain the existing fallback.
Resolve returns Resolved/Valid/NotRequested. Only the actual hosted inventory
can subsequently publish Ready; this adds no model residency, timing, GPU or
native batching claim.

The readiness-resume path now settles downstream non-runtime tasks after
successful runtime completion and cleanup, before final output projection. It
does not dispatch newly unblocked runtime tasks or implement general chained
runtime continuation.

## Pumas owner boundary

Real CPU dispatch exposed a separate owner inconsistency: the published Pumas
pin `26a84` identifies an HF directory artifact but supplies its primary weight
file as a directory load target. The native loader correctly refuses that
contradiction. The same defect was verified on public Pumas main
`5e114f6d8e4559e0a4d67e56000b423120a0fde0`.

An isolated local Pumas commit
`70e45d8c60e5b2b93da9fa0463049913e67e0193`, based on the published pin, uses the
owner's same
inspection manifest and artifact-kind precedence to return the actual owned
root for HF directories. Diffusers and external-reference handling stay on
their existing paths; GGUF, ONNX and bare safetensors remain file targets.
This inspection adds directory/sidecar I/O and propagates manifest inspection
errors for owned file descriptors too; it is not a cost-free path substitution.
Pantograph's Cargo.toml and committed Cargo.lock retain the published git pin.

All five focused `execution_descriptor` owner tests passed: the new HF/GGUF/ONNX/
bare-safetensors matrix, existing external and owned Diffusers roots, stale owned
entry-path handling, and batch descriptor behavior. The HF case first resolves
actual owner package facts, then verifies the cache-backed OwnerFresh load
target. No guessed target or native inference is used in these descriptor tests.

## Qualification boundary

The actual hosted fixture uses committed, untrained eight-dimensional BERT
weights and the normal graph validation, authored interface snapshot, Resolve,
executable publication, session Submit, owned Check producer and owned resume.
The observer delegates the actual resume port; it injects no Ready snapshot,
requirements payload, vector, timing or package load target.

All seven fixture files were verified against the committed reference manifest.
The 6,464-byte safetensors file has SHA256
`549cf713305780cd5af92cf397e6be34ada0e3650f967bb9772fa08ba9cedd44`.
Successful completion/reuse qualifications require the local Pumas owner fix
and are explicitly ignored on the published dependency pin. They do not establish
success for that published pin. Model/runtime downloads were disabled with
`ORT_SKIP_DOWNLOAD=1`, `HF_HUB_OFFLINE=1` and `TRANSFORMERS_OFFLINE=1`.

The initial corrected-composition run passed both real CPU tests (four workflow runs).
Each produced the committed eight-dimensional golden vector within `1e-4`.
The default cold/Ephemeral case had Stopped Candle, no model and no reservation
after each run. The explicit Candle KeepAlive case retained only its one session
lease, reused the same `candle-1` instance and reported actual
`runtime_reused=true` on the second run. Close and shutdown left no lease,
instance or model in both cases.

The harness completed in 0.80 seconds. Reported initial resumed-response wall
times were 74 and 87 milliseconds; subsequent response counters rounded to
0 milliseconds. These are test observations under concurrent test execution,
not serialized timing profiles or a zero-cost claim. No completion-ranking
evidence was fabricated or activated from these values.

Reproduction in the isolated checkout uses the ordinary test build with a
command-line dependency override, leaving the published pin intact:

```sh
ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 \
cargo test --offline \
  --config 'patch."https://github.com/MrScripty/Pumas-Library.git".pumas-library.path="/workspace/Pumas-cpu-owner-fix/rust/crates/pumas-core"' \
  -p pantograph-embedded-runtime --features host-dependency-inventory \
  empty_hosted_readiness_bootstrap -- --ignored --nocapture
```

The path is this executor's isolated source checkout; another executor must
bring in the reviewed Pumas commit locally and supply its own path. The
command-line override can rewrite the local lock entry; restore the committed
Cargo.lock before checking the published dependency pin.

## Verification

The workflow service, dependency environment service, scheduler and embedded
runtime suites with `host-dependency-inventory` passed 1,791 tests with five
ignored, explicitly excluding the existing
`saved_cpu_rerank_graph_reopens_and_matches_parent_outputs_and_selected_identity`
failure. The unfiltered embedded suite had 573 passed, that one failure and four
ignored; the same rerank failure appears in the baseline logs before this slice.
The worker-owned rerank file was left intact. Two of the ignored tests are the
real CPU successes requiring the unavailable published Pumas correction.

Strict Clippy passed for these four packages and all targets with `--no-deps --
-D warnings`. Formatting, the separately included CPU fixture's rustfmt check,
and `git diff --check` passed. An existing dead-code warning in inference's
selected text dependency is outside the changed packages. Independent source
review approved the bootstrap, final negative-snapshot guard and Pumas owner
seam, and reviewed the CPU output/reuse/cleanup evidence. It did not separately
execute binaries.

## Remaining production gates

The upstream owner correction must become available and its exact published
dependency revision must receive integration qualification before this CPU
workflow can be declared unblocked. Default scheduling remains
PriorityThenFifo with starvation boosts and the one-position warm reuse window.
This is readiness/ownership groundwork for the migration in
`scheduler-v2-production-migration.md`, not the completed research scheduler.

Comparable timing and resource profiles, actual cleanup-generation evidence for
the researched completion policy, simulator archive hash verification and
differential acceptance, and exact-head CI/review remain rollout gates. No C#
bridge, inspector normalization, rerank/audio adapter or public application
surface was changed by this slice.
