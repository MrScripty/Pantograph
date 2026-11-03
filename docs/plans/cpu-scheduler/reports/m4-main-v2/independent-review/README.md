# Prospective saved-data review

Current result: **main-v2 FAILED; M4 C4 Partial/Blocked**. See
`CURRENT-VERDICT.md` and `completed-subset-review.json`. The completed six-owner
native subset supports three retained loads versus five fresh loads. Pressure
has no Failed terminal and its execution-session close returned `scheduler_busy`.
No criteria were relaxed and no new native invocation was made by this review.

Both pre-outcome scripts and their failed reports are retained unchanged. The
execution status pins `review_saved_main.initial.py` (SHA256 `e8267d1b...`).
`review_completed_subset.py` is a separate saved-data analysis of the failed run;
its successful exit means the partial-failure analysis reproduced, not acceptance.
Outputs use exclusive creation; preserve the existing reports when reproducing.

`review_saved_main.py` was prepared before any main-v2 native outcome. It reads
only saved evidence, Git objects and frozen binaries; SQLite runs on temporary
copies. It invokes no model, workflow, Cargo command, or original projection
refresh. It creates a new report exclusively outside the evidence root.

After the one authorized invocation has ended:

```sh
python3 /workspace/m4-main-v2-review/review_saved_main.py
```

The prospective checks require retained AABBA: five golden outputs, three fresh
cold incarnations, two exact retains, two switch ACKs plus final stop; five fresh
comparison owners: one load/forward/stop each; pressure: one headroom refusal,
Failed terminal, no Start/native/guard acquisition. Totals are eight loads/stops,
ten forwards, two retains, and 61 collected records within the 128/8KiB bounds.

Full run/task/attempt/request/lease joins, native instance/profile/settings/+2
lineage, exact task-only ACK removal, full additive charges and ordered sole-guard
stop are required. Raw Start may omit selected fields; Completed must carry the
matching selected lease/runtime/device. The script independently compares raw
events with copied SQLite, real stored vector bodies with public results and
frozen goldens, and projection cursors with relevant raw-event prefixes.

All required phase durations must be present. The combined forward/drain interval
is not called compute-only time. Saved original failure and cancellation evidence
are hashed against their prior independent reviews. A failure remains
Partial/Blocked and authorizes no retry or continuation.

The binary build/hash gate is still pending at script preparation. This file's
existence is neither an execution nor acceptance claim. Runtime instance labels
such as `candle-1` are local to an owner, never global native lineage.
