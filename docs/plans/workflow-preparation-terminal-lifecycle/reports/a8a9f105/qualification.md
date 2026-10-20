# Registered preparation terminal lifecycle qualification

Source checkpoint: `a8a9f1055910be2999f0b3ab4c138be39c15087d`.
Base: public M4 contract `2da63ae9908572905a98c0c3b5a14e6952e5d3e3`.
Captured 2026-10-20 UTC in the isolated preparation-lifecycle worktree.

The bounded L1–L3 repair passes local qualification. A registered worker that
refuses before Start, retains no dispatch/batch custody, and successfully settles
its event now closes the matching workflow once through the canonical terminal
projection. Original failure text and typed cancellation survive. Deferred work
stays active; stale/foreign/incoherent or started/uncertain-cleanup owners cannot
use this path. Check and finish hold one session-store lock.

## Results and provenance

| Gate | Result | Retained record |
| --- | --- | --- |
| Initial public-production regression | Expected red: zero workflow terminals versus one required | public-baseline-red.log.gz |
| Exact clean checkpoint focused regressions | 4 passed, 0 failed | preparation-focused.log.gz |
| Exact clean checkpoint parent invalid-dispatch regression | 1 passed, 0 failed | public-invalid-dispatch.log.gz |
| Full workflow-service library at identical qualified source hashes | 1012 passed, 0 failed | workflow-library.log.gz |
| Inference/service lib/tests Clippy with inference/backend-pytorch, warnings denied | passed | clippy-pytorch.log.gz |
| Default backend Clippy | existing inference SelectedTextLoad dead-code warning fails warnings-denied gate | clippy-default-backend.log.gz |
| Affected Rust formatting, whitespace and decision traceability | passed before checkpoint | execution ledger |
| Independent source/regression/checkpoint review | admitted, no material findings | review below |

The initial red run used unchanged public production source plus an initial
regression include/test. Its test was subsequently extended; it is defect evidence,
not an exact-current-test baseline. Full-suite and Clippy logs were captured before
the commit with the same three source hashes. Focused and parent regressions were
then rerun at the clean committed checkpoint. Commands use `--locked`; tests use
`cargo test -p pantograph-workflow-service --lib`, with `preparation_` and the full
parent invalid-dispatch test name as the two clean-checkpoint filters. Clippy uses
`cargo clippy --locked -p inference -p pantograph-workflow-service --features
inference/backend-pytorch --lib --tests -- -D warnings`.

[Source manifest](source-manifest.txt) records exact file hashes. The
[evidence manifest](evidence-manifest.json) records compressed and uncompressed log
SHA256 values and provenance. Gzip headers use a fixed zero timestamp. Logs retain
the original command output; intermediate fixture-setup compile failures are not
acceptance results.

## Scope and composed-source boundary

L1 injects a controlled candidate-preparation refusal through the registered
worker driver and proves one Failed workflow terminal, preserved cause, empty
active run/queue, public close success, no Start, and zero native/reservation-port
calls. Duplicate/replay does not gain finalization authority. The other focused
tests cover real readiness deferral and typed cancellation, foreign/incoherent
active contexts, and canonical post-Start assignment custody. Attempt removal
without a native release ACK does not grant this finalizer authority.

After the parent approved/publicized PR83, its public exact head
`5b4fc6dda17342e6b27f90ebb852ffd2b7ea47eb` was fetched. A temporary-index
`git apply --cached --check` passes for this checkpoint's three-file production/
test patch against that head. Patch SHA256:
`a259c5226d666f1070bbcbc746b9e7e1633116f9acf7bd8677a5700bef893daf`.
V2 source `c25fd49d` is an ancestor reachable through that approved public ref;
local `git diff --exit-code` confirms the two affected production files equal
those at the public head. This proves structural patch compatibility, not integrated-source compile
or native M4 acceptance. PR83's prepare_residency can perform native work before
Start; the zero-native observation applies only to the controlled regression.

No actual M4 cohort was run. The parent and M4 owner must first compose the
repair and freeze a prospective protocol, then qualify the composed source.
API pre-enqueue admitted failures and worker supervision exits remain separate
existing gaps. Assigned/post-Start cleanup stays with its existing owner. No new
native cleanup ACK, GPU/model/performance, trained text or text-to-image claim.

## Independent review

The separate text_review agent admitted the atomic finalization design, source
and regression evidence with no material findings. It verified all three source
hashes, the intended baseline failure, focused 4/4 and full 1012/1012 results,
existing cancellation/shutdown and panic custody tests, clean checkpoint
`a8a9f105`, supported-feature Clippy completion and the exact-checkpoint 4+1
regressions. It changed no files and ran no duplicate tests or M4 cohort.
Archive verification is recorded in the execution ledger after packaging.
