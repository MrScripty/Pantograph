# Independent saved-data verdict

**Main-v2 failed, exit 1. M4 C4 remains Partial/Blocked.** No retry, continuation,
criteria relaxation, new native execution, source edit, or publication occurred
in this review. The process error is `M4 close/shutdown did not acknowledge every
actual owner`.

The frozen source was `c25fd49d19dd9268b5d63a794835c568eb32ff9b`, tree
`4ac114a25c45e90e5c64623f5053ed3ab6193024`. The invoked release binary SHA256 is
`ff1f35ac0044f987b25cf8d939df68a8e4e00440dce9b420da137f2524f6e539`.
This review rechecked 967 source files, 14 fixture files, eight pre-run manifests,
the executed binary, runtime provenance/environment and exact single-invocation
status. No model, Cargo command, workflow call or original projection refresh was
invoked. SQLite reads used temporary byte copies only.

## Positive completed-owner evidence

All six completed owners pass the unchanged strict per-owner checks from BOTH
preserved pre-outcome scripts. Retained AABBA completed five outputs with three
cold loads, two retains, two switch stop/ACKs and one final stop/ACK. The five
fresh comparison owners each completed one load/forward/stop. All ten real stored
outputs match their public results and frozen goldens; maximum absolute error is
`5.960464477539063e-08`.

Actual totals are eight loads, two retains, ten forward/drains and eight stops.
Eight distinct full native loaded-instance identities are present. Checks join
actual run/task/attempt/request/candidate/lease, full native profile/settings and
generation +2 ancestry, exact task-only release ACK, retained original guard
incarnation and full per-kind RAM/VRAM claims. A reload uses a fresh A instance.
Old stop/ACK precedes later cold publication, and every completed owner's stopped
registry snapshot precedes the next owner's first raw event. Each of those six
owners has acknowledged native shutdown and metadata closes.

The complete collector matches all owner logs in order: 61 records, 55,592
canonical record bytes, largest record 1,708 bytes, no overflow; the admitted
bounds are 128 records, 1 MiB total and 8 KiB per record. Load, publication,
forward-plus-drain, physical stop and registry ACK intervals are present as
distinct raw observations. The six owners' summed raw load/publication/stop/ACK
durations also match their saved diagnostic totals. These observations establish
the scoped three-versus-five physical-load result; they do not pass overall C4.

## Pressure failures retained

Pressure run `run_1962f0b9-b1d7-4dbf-bc3d-4ce8b757395f` has exactly four raw
events: accepted snapshot, library access, estimate and queue placement. It has
**zero run-terminal events** and zero attempt lifecycle/Start events. Its actual
preparation requires 620,781,187 RAM bytes against 83,886,080 available bytes; the
same preparation ID records a headroom refusal before guard/task acquisition.
Saved registry boundaries are empty and all native counters are zero. Those are
positive refusal/zero-effects observations only.

The required Failed terminal is absent. Execution-session close returned
`scheduler_busy` for session `259473a3-aa5e-4e10-b4e5-be5eb699c4bf`; native shutdown
itself acknowledged successfully. No completed pressure `native-oracle.json` was
saved. Therefore the full pressure and acknowledged-close criteria FAILED.
Producer `execution_ok:true` denotes its expected-error branch and does not
override these failures.

Frozen source supports a narrow causal handoff: residency preparation failure
returns `RuntimeBranchFailed` before candidate/Start in
`workflow/task_execution_worker.rs`; the session API directly maps the awaited
worker outcome to `Internal` on this path. Raw data independently establishes the
missing finalization and busy close. No reserved T4 finalization helper was read
or edited, and no remedy is qualified by this evidence.

## Preserved failures and limits

The exact script pinned by execution status is
`review_saved_main.initial.py`, SHA256
`e8267d1b1f741820c1e159784e8356b66984af7ca72d2bc5802cae41a5361985`.
Its unchanged execution and failure are in `original-prospective-report.json`
and `original-prospective-review-status.json`. The stricter pre-outcome script
`review_saved_main.py` and `report.json` also remain unchanged and failed at the
global exit gate. Both scripts expect status field `attempt`; actual metadata
uses `attempt_within_v2`. The real exit-1 gate fails first, before that lookup.
This schema limitation is disclosed; it does not explain or negate the failed
native process, absent terminal or busy close. The separate subset wrapper checks
the actual `attempt_within_v2==1` and preserves overall failure.

All 351 entries in the root's immediate post-execution hash manifest still match;
all 353 current input files were unchanged by this review. The original failed
v1 main's 54 recorded files, its frozen binary/archive, and all 16 recorded
cancellation-supplement files still match prior independent hashes. Original
main exit 1 and the supplement's limited scope remain intact. No supplement case
was rerun.

Private capability authority and phase-hook meaning are source-assisted; JSON
does not confer authority. Generic `candle-1`/lease labels are local, not global
native identity. Artifact SHA256/values/goldens and stored-BLAKE3 joins are
checked; this standard-library analysis does not recompute BLAKE3. There is no
compute-only/per-component wall-clock timing, speedup, allocator/RSS, GPU,
semantic-quality, fairness-equivalence, interrupted-compute or training claim.

`completed-subset-review.json` is reproducible through the separate saved-data
wrapper `review_completed_subset.py`. Its exit 0 means this decomposition of the
FAILED cohort reproduced; its `overall_main_accepted` and complete-pressure flags
remain false. Exact review files are frozen by `SHA256SUMS`.
