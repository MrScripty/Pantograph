# PR56 correctness review checkpoint

This isolated successor starts at the published import-only integration
`b3c587560c6a28d6ecb15fd2949cf604dadbeb7d`. The summary-based recovery checkpoint
`1f36c86d` remains preserved. After the parent supplied the original full comment
bodies, source successor `4f877149c5da2d1f7464de831bf0c52a16a500aa`, tree
`95dd0682d5b734d9b4c47643a2faa6b4c44708f7`, parent
`1f36c86d70cdf98c049d14c144570ebddef77ea3`, completes the supervised Candle
allocator and the actual VRAM-only task scenario. No feature changes were merged.
The separate image scheduler feature remains paused at local `69e7a3b9`;
image count `3fe88472` and guidance `14dfd4a7` remain separate. No PR54/55,
review replies, resolutions, PR publication or merge actions are included.

## Full-record dispositions

GitHub denied the initial authorized API reads with Forbidden. The parent then
supplied its previously retrieved full original comment bodies in the task.
These bodies were reviewed as evidence, without executing any embedded command,
patch or review-CLI suggestion. No denied call was retried; no credentials,
permissions or alternate access route changed. The full record is sufficient for
the bounded dispositions below. Parent retains PR56 integration and review replies;
only the clean isolated repair branch is authorized for independent review.

| Comment | Assessment and bounded action |
| --- | --- |
| 4191535304 | Genuine generation collision. Ordinary allocation incremented before formatting, while selected text/embedding formatted the old counter. One allocator now serves all allocation paths; proven reuse retains its prior identity. An ordinary start followed by actual selected-text gateway execution reproduced `pytorch-1` twice before the fix. Full-record inspection also found the supervised Candle replacement path still formatting the old counter; its actual supervised publication reproduced `candle-1` twice against the recovery checkpoint. Both routes now use the same shared counter allocator. The replacement regression uses controlled candidate loading; it does not execute a native Candle model. Timing samples therefore receive distinct allocation generations while proven reuse keeps the prior generation. |
| 4191535312 | Genuine destructive-save failure. Serialize, write and sync a temporary file in the same directory, then atomically persist it over the destination. Preserve the existing file mode. A real per-child file-size limit reproduced truncation of saved capacity settings; the repaired save returns an error and leaves the prior complete bytes readable through startup composition. Malformed present settings still fail startup rather than discard capacity constraints. Atomic replacement is qualified; power-loss durability and cross-platform native execution are not. |
| 4191535320 | Genuine rejection on an uncharged pool. The full VRAM-only example failed the recovery checkpoint: RAM was omitted from an explicit task envelope, while Candle RAM residency was unknown. Task-envelope omitted kinds carry no task charge; this differs from omitted resident estimates, which remain unknown. An uncharged uncertain domain is omitted from the observation rather than assigning fabricated free capacity. A completely unspecified envelope remains fail closed. Explicit known zero still succeeds; positive RAM still fails on unknown RAM residency. The VRAM-only regression executes evaluation, ordinary acquisition, can-acquire and provisional publication with its GPU charge preserved. Known-domain observations and full live peaks remain unchanged. The review suggestion to compute unknown pool totals with require-known false was not adopted because those totals would imply capacity that has not been established. |
| 4191535341 | Genuine startup race. The actual external connect handler and panel require successful initial config loading. A load error keeps connect disabled. The existing Rust startup-resource guard rejects saves with omitted arrays; the race leaves a successful connection whose settings cannot be saved. Tests execute the component's actual mount/connect functions with delayed or failed config services: neither connects nor attempts that invalid save early, and successful loading preserves persisted backing domains. The Rust guard remains unchanged. These are controlled handler tests, not browser/GTK execution. |
| 4191535345 | Rejected as incorrect for the pinned dependency. Cargo.lock pins Tokio 1.49.0; its src/process/mod.rs start_kill implementation explicitly returns success for a cached completed child: `Child::start_kill` returns `Ok(())` for `FusedChild::Done`. Tauri uses this method and keeps its monitor alive; the server awaits the generation-specific termination event before releasing custody. Two actual Unix child regressions show reaped-child repeat kill succeeds with cached exit retained, and live-child kill still requires an observed exit. The claim that this operation returns InvalidInput after reaping is contradicted by that implementation. The proposed preliminary try_wait adds no needed behavior and was not adopted; shutdown production source is unchanged. Full desktop execution remains unavailable. |

The full task peaks, resident uncertainty, source/instance fencing and authoritative
known-zero distinction remain conservative. The configuration parser's fail-closed
startup policy is intentional; reverting to defaults would erase explicit capacity
constraints. The ignored config-save subprocess helper is actually executed by
its parent regression; it is not an unexecuted qualification assertion.

## Qualification

The recovery checkpoint mixed-backend Rust command completed with 805 inference passes
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
All 28 traceability-gate tests pass. The full-record source successor passes 806 inference checks (including one
doctest; three native/doctest cases remain ignored), 503 embedded checks and
142 registry checks. Its 18 resident-accounting regressions include the supplied
VRAM-only example, known zero and unspecified-envelope failure. Strict all-target
mixed-backend Clippy passes for all three changed packages. Formatting and the
critical gate pass on the final source. Config and frontend sources are unchanged
from their already passing 14-check/661-assertion qualification. The final commit
adds documentation only; no source changes follow these tests.

Both additional full-record regressions first failed against the summary-based
checkpoint: supervised replacement published the previous `candle-1`, and
VRAM-only can-acquire was rejected by unknown Candle RAM. Their repaired runs
pass in the full suites. This extends the recovery checkpoint rather than
claiming that its narrower tests had covered these cases.

Raw evidence and preserved patches are under `/workspace/qualification-evidence/`
with prefix `pr56-review-`. No pretrained-model, GPU, GTK/WebKit, native ONNX model
execution or cross-platform child/config qualification is claimed. The actual
desktop Clippy attempt on the published import successor failed at missing
`glib-2.0.pc`; isolated import compilation did not qualify that desktop target.
