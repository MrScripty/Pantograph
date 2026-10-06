# Readiness action boundary repair

Preserved source before edits: 65e26adaec56457ba628c181a10743a9a14761fb,
tree 627e3b785ecca198d28a451efb3417dda83477eb. Its exact native failure is
preserved under post-resolution/native-attempt: Pumas resolution and requirements
seed storage succeeded; the consumer queried Resolve again, observed Resolved,
and repeatedly deferred the same inference task through state version 19.

The production repair changes only the dependency-readiness adapter action to
Check. Requirements seed retrieval remains Resolve. No proof validation, ID
production, saved-snapshot scope, scheduler policy, or registry guard changes.

A regression runs the actual graph proof producer, saved-snapshot JSON round trip,
scheduler request owner, inventory projection contract, snapshot provider and
readiness lifecycle. Separate Resolved Resolve and Ready Check observations must
admit the task. Resolve Ready alone, stale Check and another model's Check must
not admit it. Existing saved package/revision/artifact/runtime/device negatives
remain. These are controlled inventory inputs, not claims of production facts.
The real Pumas producer regression and actual native qualification are separate.

The meaningful red log records PausedDeferred despite a scoped Ready Check. The
initial test-setup error omitted the contract-required Ready environment reference;
that failure is retained. Existing session fixtures had published only Resolve
Ready; their failures and correction to separate Resolve/Check snapshots are
retained. The downstream recovery test withholds a stale requirements seed:
stale Check remains terminal under existing policy, as covered by the new negative.

Local workflow checks pass: 929 unit and 60 integration tests. Embedded tests
pass: 537, including real Pumas cold bootstrap and controlled CPU retained output.
Effective graphs precede every Rust build, retain current Pumas 26a84e32,
ORT load-dynamic and ort-sys disable-linking, and exclude download features.
ORT_SKIP_DOWNLOAD=1 is set. Strict selected-crate all-target Clippy passes for
workflow-service and embedded-runtime. Formatting, critical and staged traceability
checks pass (traceability reports no mapped impact; semantic review remains required).

Native acceptance remains: exact-source native Tauri build/startup, graph authoring,
save/reopen, visible wiring, current Executable proof and typed Resolve, one GUI
Submit, automatic scheduling, actual dispatch, retained finite 8-value CPU vector
within 1e-5 of the deterministic fixture oracle and candle.cpu/cpu selection
metadata. Preserve every failure before further repair; do not infer loader,
GPU, pretrained or broad production qualification from controlled tests.

Actual native run 37536948833 now confirms requirements seed storage and Ready
Check admission, followed by dispatch preparation and TerminalFailed with zero
outputs. Build and eight startup tests pass. See [the complete native failure](native-attempt/README.md).
The next selection/runtime cause is not captured; no further repair follows.
