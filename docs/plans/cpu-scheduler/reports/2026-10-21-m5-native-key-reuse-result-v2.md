# Native CPU key-reuse v2: bounded comparison result

The single separately authorized v2 cohort passed its prospectively frozen
producer oracle. Both fresh arms completed. Independent raw-evidence review
and exact-head integration checks decide scoped acceptance separately below.
[V1](2026-10-21-m5-native-key-reuse-result.md) remains failed; its original
source, protocol, marker, evidence and archive are unchanged.

## Prospective correction and provenance

The reviewed correction requires every native candidate to identify
`runtime.candle` and match its own member's registry ACK. It preserves distinct
task, attempt, execution-request, reservation-lease and grant-ticket custody
identities within each arm, along with all other original criteria. Stable
graph task names and registry-local lease numbers may repeat across fresh runs;
their run/attempt/owner/registry context supplies the namespace. V2 does not
reuse v1's baseline. No acceptance criterion was selected after v2 execution.

- Frozen source `1f6af8a25440d6a4049a3e86c86785689b71cf70`, tree
  `0368ee2e945976ec84930f3a779995b4cc6f473a`, published normally on PR91 before
  execution, preserving v1 commits `fb7ba0e3` and `aea54f7f` as ancestors.
- Producer source SHA256
  `f088036bfed607633c3fa9e93d019444c105ba7f6d1380e080fadc9a4fc480c2`;
  copied executable SHA256
  `c41b0a573cc834595118545d70c723085ee33afdcd012971b5d6fc96cb88b8f0`.
- [Prospective v2 protocol](2026-10-21-m5-native-key-reuse-protocol-v2.md)
  SHA256 `780f42c20dcc69ff210a08bac8208df3b6b30492db65667960b79bbe63327c91`;
  supervisor SHA256
  `cdc2efb9e0b3a715ce47b71a7433c23c5776eb0bb0a62b238257fb6dfeaae51f`.
- Frozen manifest SHA256
  `331e55de0b7854484fa45527b57e95dd792baab5f8000a34c8bff310c0a51d53`.
  Independent source/protocol/supervisor and exact manifest/binary review
  preceded the single run. Discovery listed exactly one v2 held test without
  facade startup, metadata or native work.
- Evidence root:
  `/workspace/scratch/pantograph-recovery/native-key-reuse-cohort-v2`.
  The exclusive execution marker remains. V1 uses its separate retained root.
- Actual execution: 2026-10-21T07:11:49.339705Z to 07:11:55.326203Z;
  supervised elapsed 5,986,517,955 ns, exit 0, reaped, no timeout or supervisor
  error. The owned process is absent. No second v2 execution was performed.

The build is a debug/test executable on accessible CPU hardware using the same
committed untrained BERT-8 fixture and fixed two texts as v1. One configured CPU
thread, 8 GiB virtual-address-space ceiling, 180-second outer watchdog and all
per-arm/log/cleanup bounds were fixed before execution. Actual cgroup/affinity,
toolchain, fixture and binary hashes are in the frozen manifest. Resource
declarations and reservation claims are not physical allocation measurements.

## Observed mechanism, correctness and lifecycle

| Fixed criterion | Baseline | Enabled |
| --- | --- | --- |
| Explicit key reuse | false | true |
| Genuine Ready proposal | infer_a, infer_b | infer_a, infer_b |
| Encodings / hits / healthy retained entries | 6 / 0 / 0 | 4 / 4 / 0 |
| Loads / retains / native forwards / stops | 1 / 1 / 2 / 1 | 1 / 1 / 2 / 1 |
| Session/graph closes and public shutdown | acknowledged | acknowledged |
| Final reservations / idle owner | empty / false | empty / false |
| Body / collector / event / diagnostic-I/O errors | null | null |
| Native owner generations | 4→6→8, one thread | 4→6→8, one thread |
| Structured rows / ledger events | 12 / 19 | 16 / 19 |

Both arms used distinct fresh run identities, private runtime registries and
loaded-instance identities. Every member kept its own grant, Start, attempt,
execution request, reservation and matching registry ACK; within each arm the
two lease numbers were 3 and 5. Candidate IDs correctly repeated
`runtime.candle`. Native current/previous owner identity, settings and generation
continuity were retained. The complete vectors were returned and read back
through actual retained artifacts, met the frozen golden tolerance (maximum
absolute error 5.960464477539063e-8), and were
exactly equal between arms. Numerical evidence covers these fixed untrained
fixture outputs only.

The enabled compatible prefix contains both members. Its forecast diagnostics
report zero eligible samples and `no_fresh_samples`; proposals report
`insufficient_fresh_samples` with sample counts [0,0]. Minimum three samples
remains in force. This is no trained-ranking or forecast-accuracy evidence.

| Enabled grouping phase | Observed calls | Inclusive elapsed ns |
| --- | --- | --- |
| probe | 2 | 49254 |
| capture | 1 | 598840 |
| fill | 1 | 127322 |
| lookup | 4 | 48874 |
| key_calculation | 4 | 154894 |

All five aggregates were Available. Baseline grouping observations were absent,
not measured zero. These are service-lifetime, monotonic, inclusive spans with
uncalibrated observer/logging overhead; they overlap and must not be summed.
No timing threshold, latency/throughput gain or representative speedup is
inferred from this one debug-build synthetic comparison.

Each native drain retained conservative member RAM and VRAM claims of
536895107 bytes, alongside the existing load/resident claims. Those claims
remained charged through drain until member cleanup. Final reservations were
empty after acknowledged shutdown. No reservation arithmetic or RSS was used
as incremental-allocation evidence. Scheduler cleanup matching and final
readiness/Start fence validation remain source-assisted through the existing
adapter; raw registry ACKs are not relabeled independent scheduler ACK proof.

## Evidence retention and finite disposition

Original v2 execution result SHA256:
`d09e727c3b1c6cb6315cd20b83e439a3d9064d087a0828c4dd398ee65c197a2c`.
Comparison SHA256:
`0930afc7e8f7c437b08549b764098fa70efc66914051587c8a22a90795266a64`.
Preparation receipt SHA256:
`19e5268c012970dcee68e7b1a649ef412641db15e78c2ad7bbb3a6c380bf81b1`.
It records ten model-free oracle passes, default embedded 617 passes/two
existing ignores, strict own-package Clippy/Rustdoc, formatting, traceability
and declared pre-push checks. The environment/model-capacity interruption was
reconciled before committing: preserved staged bytes and completed no-run build,
no active producer or v2 root/marker, and unchanged v1 hashes. No uncertain
native launch was retried. No credentials, permissions or security settings
changed.

Independent raw-evidence review confirmed all frozen criteria and found no
material issue. Review receipt SHA256:
`07e9636205e7bdd8bd6f0d41d0988a326b497e30b7f1b2657ce30b11daf0dc56`.
The [retained original archive](m5-native-key-reuse-v2/retained-cohort.tar.gz)
contains 39 files / 131938 bytes, SHA256
`d7cd6334bcec3b2a72c9f4eee026b0a507d62f7388db9739e611d4c179b98828`.
Its [per-file manifest](m5-native-key-reuse-v2/original-file-sha256.json) and
[archive identity](m5-native-key-reuse-v2/archive-sha256.json) preserve both raw
arms, the actual artifact bodies, journal, prospective protocol/manifest/
supervisor, source/preparation/review receipts and check logs. All entry bytes
were matched to originals and their hashes without executing them. Executable
binaries, databases and client credentials are excluded; full local originals
remain protected.

M5-CPU is supported by the narrow v2 evidence and independent review. M6-CLOSE
remains verifying until final source/report/archive review, exact-candidate CI
and actual integration checks pass. Acceptance, if earned, is limited to the finite ordinary-member
bounded pure-key-reuse envelope in the [checklist](2026-10-21-m5-m6-completion-checklist.md).
Broader C1 memory-aware dispatch, complete incremental allocation, native
batching, trained-model quality, forecast usefulness, deployed performance,
GPU/remote and representative hardware qualification remain separate. No
allocator expansion, admission-policy extension or default activation is
selected to close this finite task.
