# PR56 correctness review checkpoint

This isolated successor starts at the published import-only integration
`b3c587560c6a28d6ecb15fd2949cf604dadbeb7d`. Production repair source was saved at `5da09171`. Its tested source successor is
`a92f67ff40b36fb445294be18d4e26b9299b5364`, tree
`9456e8e7bfce285c36c48c88e692d8f93030dd26`, parent
`5da091713ad1e4e68f980a8f0e151f1c6780edb9`. It adjusts the existing uncertain-pool
inspection helper to use a positive charge and fixes one new test initializer lint.
The separate image scheduler feature remains paused at local `69e7a3b9`;
image count `3fe88472` and guidance `14dfd4a7` remain separate. No PR54/55,
review replies, resolutions, PR publication or merge actions are included.

## Review access and provisional dispositions

GitHub denied the authorized API reads of all five comment bodies with Forbidden.
No credentials, permissions or alternate access route were changed. The full
bodies have been requested from the parent. These are source-validated provisional
assessments of the supplied summaries, not claims to have read or disposed the
exact comments. Integration publication is held for that missing evidence.

| Comment | Assessment and bounded action |
| --- | --- |
| 4191535304 | Genuine generation collision. Ordinary allocation incremented before formatting, while selected text/embedding formatted the old counter. One allocator now serves all allocation paths; proven reuse retains its prior identity. An ordinary start followed by actual selected-text gateway execution reproduced `pytorch-1` twice before the fix and produces distinct IDs after it. |
| 4191535312 | Genuine destructive-save failure. Serialize, write and sync a temporary file in the same directory, then atomically persist it over the destination. Preserve the existing file mode. A real per-child file-size limit reproduced truncation of saved capacity settings; the repaired save returns an error and leaves the prior complete bytes readable through startup composition. Malformed present settings still fail startup rather than discard capacity constraints. Atomic replacement is qualified; power-loss durability and cross-platform native execution are not. |
| 4191535320 | Genuine rejection of an explicitly zero charge by unknown peer residency. Only when every resource kind bound to the request in an uncertain pool is explicitly `Some(0)` may admission omit that unavailable observation. It never fabricates free pool capacity or resident totals. Positive and missing claims remain unavailable; known pool observations remain unchanged. Actual registry evaluation and acquisition reproduce the defect and pass after repair. |
| 4191535341 | Genuine startup race. The actual external connect handler and panel require successful initial config loading. A load error keeps connect disabled. Tests execute the component's actual mount/connect functions with delayed or failed config services: neither connects nor saves defaults early, and successful loading preserves persisted backing domains. These are controlled handler tests, not browser/GTK execution. |
| 4191535345 | Supplied hypothesis contradicted by pinned Tokio 1.49.0: `Child::start_kill` returns `Ok(())` for `FusedChild::Done`. Tauri uses this method and keeps its monitor alive; the server awaits the generation-specific termination event before releasing custody. Two actual Unix child regressions show reaped-child repeat kill succeeds with cached exit retained, and live-child kill still requires an observed exit. No shutdown production change is justified by the summary. Full desktop execution remains unavailable. |

The full task peaks, resident uncertainty, source/instance fencing and authoritative
known-zero distinction remain conservative. The configuration parser's fail-closed
startup policy is intentional; reverting to defaults would erase explicit capacity
constraints. The ignored config-save subprocess helper is actually executed by
its parent regression; it is not an unexecuted qualification assertion.

## Qualification

The affected mixed-backend Rust command completes with 805 inference passes
(including a passing doctest), three ignored native/doctest cases, 503 embedded
passes, 141 registry passes, and 14 config passes. The config subprocess helper
is reported ignored in the outer suite and is explicitly executed successfully
by its parent test. The real host resident/custody regressions remain green.

The initial combined Rust run had 724 inference passes and six failures: an
existing inspection helper used a zero request and indexed an unavailable pool
observation. Changing that probe to one positive byte retains its unknown-resident
assertions under the corrected zero-charge policy. The passing successor run
executes all affected packages. Initial strict Clippy identified a new test's
field reassignment after default construction; the successor initializes that
field directly, without any warning suppression.

All 661 frontend assertions and TypeScript checking pass. Formatting, critical,
Svelte accessibility (27 tests), committed source-range traceability and all nine
ONNX no-build-download feature/target graphs pass. Warning-deny all-target mixed-backend Clippy passes for inference, embedded,
registry and config. The post-initializer config rerun also passes all 14 checks.
All 28 traceability-gate tests pass. Source is unchanged after final qualification;
the final successor adds this report and ledger/plan records only.

Raw evidence and preserved patches are under `/workspace/qualification-evidence/`
with prefix `pr56-review-`. No pretrained-model, GPU, GTK/WebKit, native ONNX model
execution or cross-platform child/config qualification is claimed. The actual
desktop Clippy attempt on the published import successor failed at missing
`glib-2.0.pc`; isolated import compilation did not qualify that desktop target.
