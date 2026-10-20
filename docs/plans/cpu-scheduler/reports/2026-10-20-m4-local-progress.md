# M4 local implementation checkpoint

Status: **In progress; not qualified or published**. M3 exact-head acceptance is
complete at preserved b63aa3eb/tree defca3d3. This checkpoint implements the
independently admitted native and workflow lanes. It does not accept M4-C1–C5.

The native linear capability owns the original load-started mode guard through
load-only preparation, actual warm receipt transfer, matching task-release ACK
and sole-guard stop ACK. Separate load/publication/stop/ACK intervals are retained.
Independent source review found no blocker;26 scoped CPU tests pass (8new).

Workflow explicit residency requires fair service opportunities. Preparation
follows the actual grant/permit before candidate claims; Ready and prepared-owner
fences remain mandatory at final Start, including FIFO. Changed owners invalidate
history. Independent source review found no blocker;7new tests and complete
workflow suite1015library+65integration pass,0ignored. The initial missing-import
compile failure is preserved alongside the corrected successful run.

Exact source hashes and original logs are in [m4-local-v1](m4-local-v1/). The initial
embedded port6test result predates later review fixes; it does not qualify them.
Its original log retains a NUL hole from a transport failure/retry. Process audit
found the original build still running; only the duplicate waiting Cargo was
terminated. Original completed6tests. No fixed protocol started or retried.

Port review corrected two defects: actual per-kind intent maxima must feed
pre-load headroom, and DispatchStarted requires a no-release ACK. New regressions
and facade integration await affected tests. Facade also requires retained
shutdown supervision when callers drop waits. Source/producer/composed review,
source/input/environment freeze, fixed AABBA/native cancellation evidence and
final review remain gates. No fixed M4 native protocol has run.

The comparison prospectively uses5fresh acknowledged private facade lifetimes
versus1retained lifetime. Compare counts, outputs and ownership, with no startup,
latency, fairness-history or speedup claim. No GPU, native UI, batching or semantic
model quality qualification is made.

Publication remains blocked by the unanswered automatic-review rejection for
12-file07138639 payload to MrScripty/Pantograph. No retry, alternate upload, merge
or branch deletion is authorized. This checkpoint is local only.
