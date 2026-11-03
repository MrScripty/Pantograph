# Fixed native CPU comparison v1: retained failure

The single prospectively admitted cohort **failed**. Its enabled arm never
started. This report does not change the frozen criteria or accept M5-CPU.
No second physical execution, input tuning or post-outcome acceptance change
was performed.

## Exact provenance and terminal result

- Frozen source `fb7ba0e37645aaa6627ef6ae1b729c931387a772`, tree
  `203f7b4a3d99f637fb96e99cb8fbc1c55b3f8f82`, based on PR90's actual normal
  merge `ecfed47da7c51d03b968bed739e2abc06eb4a6de`.
- Producer source SHA256
  `cdd3ec4888a01dd1723616bfabd4099d5368013ba76714cc16b11bb527894b0e`;
  copied executable SHA256
  `d465fc543206ba8879550901b415a1f092d995b6bbf4410606da0f2eeb5ac832`.
- [Prospective protocol](2026-10-21-m5-native-key-reuse-protocol.md) SHA256
  `a6648d0785bac9979ea3fc1f951f4c9b31a59d277f55f02b375b3bb7ae9c6216`;
  supervisor SHA256
  `83f8598ee2d8f61e26711b64c18ae1e10fc1e596d1c1009b1b54e06d47b38d3c`.
- Independently approved frozen manifest SHA256
  `1aecf82f87d586f9eb28f8f5a4afdb03d721f72358f646804cc3fe58073b8e63`.
  Exact held-test discovery listed one case without facade startup or metadata.
- Evidence root retained at
  `/workspace/scratch/pantograph-recovery/native-key-reuse-cohort-v1`.
  All original binary, fixture hashes, source identity, environment allowlist,
  progress, raw rows and outcomes remain there. The exclusive execution marker
  remains; the supervisor provides no retry path.
- Actual execution: 2026-10-21T06:15:00.456242Z to 06:15:05.539324Z;
  supervised elapsed 5,083,100,620 ns, exit 101, reaped, no watchdog timeout,
  supervisor error null. Producer verdict false, error `native identity reused`.
- Actual-main PR90 CI passed all 20 checks/four workflows before launch. The
  integration receipt SHA256 is
  `9d7e20d34d5a1a55811134dd35058f3795f0f966a0fdf49b0c05b95b84a38daf`.
  Independent source/oracle/protocol/supervisor and exact-manifest/binary review
  preceded execution. No local compilation ran concurrently.

The environment was a debug/test binary, untrained committed BERT-8, one
configured CPU thread, CPU cgroup 400000 us/100000 us, memory cgroup 16 GiB, an 8 GiB
virtual-address-space ceiling and host-reported AMD EPYC 9V74. These facts do
not make the case representative of deployed models or hardware.

## Observed baseline facts and failed criterion

The baseline returned both requested 8-dimensional outputs, retained four
artifact readbacks, and recorded two native drains. Independent review of actual
artifact bytes confirmed agreement with returned values and the frozen golden;
maximum absolute golden error was 5.960464477539063e-8. Its scalar work observation
was six key encodings, zero hits and zero healthy retained cache entries.
Diagnostics recorded one load, one retain, two forward drains and one stop.
Public session close, graph closes and shutdown acknowledged success; shutdown
did not time out, final reservations were empty and no idle owner remained.
Body, collector, event and diagnostic-I/O errors were all null. The fsynced
journal contains 12 structured rows and the ledger 19 events.

The frozen oracle requires distinct `task_id`, `attempt_id`,
`execution_request_id`, `candidate_id` and `reservation_lease_id` across the two
native drains. Actual task IDs were `infer_a` and `infer_b`; attempts and requests
differed, and reservation leases were 3 and 5. Both candidate IDs were
`runtime.candle`. This equality triggered the fixed failure. It is not evidence
that the distinct grants or leases were reused. Both producer-author and
independent source/evidence review confirmed that production deliberately builds
candidate IDs as `runtime.<runtime_id>` in
[runtime_candidate_draft](../../../../crates/pantograph-embedded-runtime/src/runtime_dispatch_candidate_provider.rs).
Candidate uniqueness applies within one alternatives list, as enforced by
[dispatch selection](../../../../crates/pantograph-scheduler/src/dispatch_selection_policy.rs),
and not across member attempts. The frozen oracle and its hand-declared positive
fixture confused provider identity with custody identity. This is a qualification
software defect missed during pre-run review, rather than a discovered production
reuse-of-authority defect. This report does not retroactively exempt that field
or relabel the arm as passed.

Independent raw review also confirmed exact joins across two grants, Starts,
Completed events and registry ACKs, with one-thread owner continuity 4→6→8 and
all 12 persisted rows matching the saved arm. Actual records contain conservative
VRAM reservation claims of 536895107 bytes per drain despite the CPU dispatch.
The frozen protocol's CPU/VRAM wording does not establish zero VRAM reservation
claims or physical GPU use. Neither reservation estimates nor declarations are
physical allocation evidence.

The baseline failure stopped the loop after acknowledged teardown. There is no
enabled-arm directory, native output, key-reuse count, captured-prefix or phase
measurement from this cohort. Only two native forwards occurred, within the
four-forward maximum. No baseline-versus-enabled result exists. Scheduler
cleanup matching remains source-assisted through the existing adapter; the raw
registry ACK is not independent scheduler-ACK proof.

## Retained evidence identities

The [original retained cohort archive](m5-native-key-reuse-v1/retained-failed-cohort.tar.gz)
contains 40 files, 131098 bytes, SHA256
`e6edef6c58518ab809be8fc5b31d1c9e18abd700ddafdb5037c82aa70fbed340`.
Its [per-file hashes](m5-native-key-reuse-v1/original-file-sha256.json) and
[archive identity/scope](m5-native-key-reuse-v1/archive-sha256.json) permit exact
verification without execution. It includes the actual artifact bodies and
review/preparation receipts, excluding executable binaries, databases and client
credentials. Original full local evidence remains preserved as well.

| File under evidence root | Bytes | SHA256 |
| --- | --- | --- |
| execution-started.json | 145 | edc88952ee485a00fc4328c12c0c6ce8d873ae4ec5ec8375fb9594641565e6bf |
| execution-result.json | 598 | 77a24b38ccf56afccba928988e52d8c4fa99d2968b5e620689a19159b6fc1a91 |
| comparison.json | 150464 | 27cb921f9f4cf86bbd5cb23c224ce2084029c40bc1b2b05276f7e7592a15a0df |
| baseline/arm.json | 133686 | be7f39bc407b869955d282a40bcc22569e28d5952470ea35c225457dd4f2684f |
| native-rows.jsonl | 11369 | 3bfe2b7316a2d3754382e4a815755d560e2d94391827708278805789782d2f57 |
| progress.jsonl | 920 | 8870f3b249d6fc1ddef531cab2d29413ee465fb4a69dad0ef7e7004309b3bf9b |
| producer.stdout.log | 400 | 61139d5816979f711d6f99a11837422df7941fb5c3cd63949b188f01e6fad32c |

Preparation receipt `native-key-reuse-local-validation.json` SHA256
`3adf218aacf4b9cdec2bfe0fcdfeeb21a66b5a5f05a8b9c79af9a656939d71b1`
retains successful affected checks and initial failures. A wrong full source SHA
was refused by freeze preflight before creating the evidence root; the unchanged
actual Git HEAD then froze successfully. A PR90 receipt collector initially
referenced its not-yet-created output instead of the prior PR89 receipt; that
read-only collection error was corrected before the successful receipt and run.
Neither preparation error executed the held test.

## Finite disposition

PR89 integration, M5 software and the PR90 owning consumer are implemented and
integrated within their existing evidence limits. M5-CPU is failed for v1;
M6-CLOSE remains unaccepted. Stop at this terminal outcome. Preserve the frozen
source/protocol and this failure; any evidenced oracle repair and fresh cohort
must be separately re-planned prospectively. Missing representative model or
hardware measurements did not cause this failure and are not a blocker to
diagnosing the software assumption. No allocator expansion is selected.

Production admission/defaults are unchanged. This cohort establishes no
key-reuse speedup, complete incremental-memory guarantee, native batching,
representative model quality or hardware qualification. RSS and reservation
estimates were not substituted for allocation measurements. Historical denied
log routes remain unretried, and baseline strict inference lint/Rustdoc
limitations remain unsuppressed.
