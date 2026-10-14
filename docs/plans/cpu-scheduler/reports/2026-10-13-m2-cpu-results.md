# M2 actual retained CPU completion advice

Status: M2-C3 actual CPU cohort passed; independent whole-range/source and raw-evidence review
passed; ordinary final-head CI is pending. M2 remains Verifying. The core
[report](2026-10-13-m2-core.md) owns the controlled kernel/policy regression
results; this report owns actual CPU observations. No speedup or policy
activation is claimed.

## Source, environment and fixed protocol

One invocation ran on 2026-10-13 at 23:25:17 UTC from clean, committed and pushed
source `7a7165d493858de016f96196fb50ca998ebddcb5`, tree
`27d881b0da7042f0341eed0739f347348025ce11`. Release binary SHA-256:
`dd6c01eb30bb11c8b71c3d528532a10d109155dbe840ccfefdad292292d25fc2`.
The [source manifest](m2-cohort-v1/source-manifest.json) binds the binary,
policy/probe sources, Cargo.lock, fixture bytes and environment. The prospective
correction was published at `cccf299c` before its implementation; independent
GPT-6.1 Sol / High `m2_qualification_review` admitted the final producer before
execution. No model cohort ran under the superseded target-only construction.

The environment exposes AMD EPYC 9V74, four CPU equivalents and 16 GiB memory;
no accessible GPU paths were found. Actual execution used calibrated Candle CPU
F32 with one private CPU thread, the committed untrained BERT-8 numerical
fixture, authoritative public Pumas facts, an actual retained native owner and a
separate live retaining session. RAM 32 MiB / VRAM zero is an operator-declared
resident envelope, not allocator measurement. Existing fixture-license
qualifications remain in the [CPU guide](../../../../examples/cpu-embedding-demo/README.md).
No model was acquired. Numerical reference authority is the existing separately
executed Transformers 4.53.3 reference and its committed hash manifest.

Exactly 13 public singleton sessions ran AABBAABBAABBA, followed by exactly one
original two-Ready pair. The singleton graphs preserve only their original pair
branch, including node IDs/data/bindings. Actual workload digests establish
singleton/pair equality; no hash field, readiness, timing, history or output was
injected or normalized. All attempts, wall times and numerical outputs were
saved before interpreting the result. The command exited zero after 0.995 s
for the complete invocation. This is one invocation duration, not a baseline or
performance comparison. No retry was performed or admitted for a winner.

## Raw observations and independent arithmetic

The pair's predecessor is A, native owner generation 30. Every conditioned row
contains three fresh owner-comparable observations. The raw task/sample counts
and actual Starts match each declared singleton/pair population. The M1
unconditioned rows have seven A and six B observations.

| Conditioned service row | Samples | Maximum observed ns |
| --- | ---: | ---: |
| A after A | 3 | 15,739,340 |
| B after A | 3 | 19,735,385 |
| A after B | 3 | 16,538,568 |
| B after B | 3 | 14,174,652 |

With predecessor A, the two complete service-order forecasts are:

| Order | First completion ns | Terminal completion ns | Completion sum ns |
| --- | ---: | ---: | ---: |
| A → B | 15,739,340 | 35,474,725 | 51,214,065 |
| B → A | 19,735,385 | 36,273,953 | 56,009,338 |

A → B wins on terminal cumulative service. Advice evaluates exactly two plans
and four completion events. Actual M1 dispatch also selected A → B from its
own unconditioned maxima; advisory mode did not change its actual Starts.
The controlled hand oracle in the core report separately demonstrates a case
where immediate and lookahead recommendations differ. The actual cohort does
not establish such a difference or a speedup.

All 15 eight-component cohort vectors agree with the committed independent
truncation reference at absolute tolerance 1e-5. Bootstrap output is retained
separately and does not train the serial history. Execution resumes preserve
run identity. Cleanup reports no errors, reservations are empty and the native
serial owner is absent after explicit shutdown. The registry conservatively
retains its existing stopping/residency accounting; this does not prove physical
memory reclamation (CPU-14).

Independent GPT-6.1 Sol / High `m2_final_review` recomputed all row counts,
maxima and scores directly from the raw attempt exports without consulting the
producer summary/oracle or calling the ranking kernel. It independently checked
all singleton/pair Starts and unchanged samples, exact keys, generation and
freshness, fixture/source/binary hashes, shutdown, and all 15 vectors against
`git show 07b23d47`'s original golden bytes. Maximum absolute vector error was
8.940696716308594e-08. Pair's oldest raw sample age was 506,850,171 ns, below
the 60-second bound. Whole source range review found no material defect.

These are first-order, predecessor-conditioned dispatch-through-cleanup service
observations. Cold preparation, transfer, scheduling gaps, other apps and full
path history are outside this forecast. No missing costs are treated as zero.
Later M3–M6 contracts, GPU, native desktop, remote work, action promotion and
default activation remain separately reviewed scope.

## Evidence and reproduction

The [raw archive](m2-cohort-v1/raw-evidence.tar.gz) contains all 14 attempt
exports, actual ledger events, advisory/immediate projections, protocol,
bootstrap, shutdown and execution/oracle status, plus the pre-execution
invocation manifest and stdout/stderr/exit status. It excludes model binaries
and databases. [Raw hashes](m2-cohort-v1/raw-file-sha256.json) cover every archived
member; `SHA256SUMS` covers the delivered files. Digest identity is required for
provenance and exact comparable workloads, not a semantic oracle by itself.
[Summary](m2-cohort-v1/summary.json), [advice](m2-cohort-v1/advice.json),
[immediate selection](m2-cohort-v1/immediate.json) and
[shutdown](m2-cohort-v1/shutdown.json) are convenient unchanged projections.

The single admitted invocation is complete. The following is the reproducible
procedure for a separately admitted replication, not authorization to tune or
repeat this cohort until its winner changes:

```sh
source /workspace/pantograph-tools/activate.sh
export ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1
cargo build --locked --offline -p pantograph-embedded-runtime \
  --no-default-features --features backend-candle,host-dependency-inventory \
  --example cpu_embedding_workflow_probe --release
cargo clippy --locked --offline --no-deps -p pantograph-embedded-runtime \
  --no-default-features --features backend-candle,host-dependency-inventory \
  --example cpu_embedding_workflow_probe -- -D warnings
# Use a fresh absolute path: the producer refuses an existing directory.
export PUMAS_REGISTRY_DB_PATH=/workspace/new-m2-replication/registry.db
OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 \
  /workspace/pantograph-cache/target/release/examples/cpu_embedding_workflow_probe \
  /workspace/new-m2-replication --serial-lookahead
```

## Supporting gates

Scheduler205 tests pass (one existing ignored cost probe); workflow-service988
pass (zero ignored). Scheduler all-target warning-deny Clippy and workflow
all-target `--no-deps` warning-deny Clippy pass. The existing three actual native
worker lifecycle regressions pass: successful linear cleanup/retaining session,
post-drain cancellation without training, and lost-receipt fencing. These
controlled lifecycle cases are separate from the one fixed calibration cohort.
Declared core pre-push tests pass (node-engine261 /one existing ignore,
workflow-nodes168); the later producer adds no changes to those owners.
Formatting and whitespace pass.
[Supporting commands/statuses](m2-cohort-v1/supporting-gates.json) and
[logs](m2-cohort-v1/supporting-logs.tar.gz) preserve successes and the failed
broader invocation. The fixture [reference manifest](m2-cohort-v1/reference-manifest.json)
records independent reference authority and bytes. These gates observe their
named local scopes, not the ordinary PR CI or graphical/GPU behavior.

The affected example's scoped warning-deny Clippy gate passes. The first
command without `--no-deps` stopped in unchanged inference optional-consumer
items before reaching the example; preserve that failed invocation as CPU-17,
not a repaired Candle-only dependency configuration. No warning suppression,
PR75-owned source edit, manifest or lockfile change was made. Current normal CI
remains required and pending; local CPU fixtures do not replace it.
