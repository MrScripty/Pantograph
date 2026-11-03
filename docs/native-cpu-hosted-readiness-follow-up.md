# Native CPU hosted readiness follow-up

Remote main was reverified at `038dacaaa98ebd007c32e13d4608726ca5ccf63a`.
The completed CPU dependency-planning candidate `601f1adb1c0718f1e658992075b65f3f6cda42ee`
was fetched from the public qualification branch and inspected before integration.
Its minimal production dependencies were integrated as `10ede28c` (validated
dependency controls), `fda651f3` (authored text Submit inputs), and `677dc614`
(producer-aligned planning identity). Inspector, gradient, qualification CI,
and audio/rerank adapter changes were not imported. Session fixtures now use the
same planning constructor in `9038c4e8`; their behavior assertions are unchanged.
Owned embedded image/text/chat/embedding and completion fixtures are aligned in
`9fc1b605`, preserving task IDs, constraints and assertions.

## Observed failure and source diagnosis

Public [run 37683966982](https://github.com/MrScripty/Pantograph/actions/runs/37683966982)
at qualification head `ccdbd2ea7125e0d312b48ea82a0eca7465366769` passes the previous
admission error. Its native graph Submit then waits for dependency readiness.
The logs show a `request_ready` resolve-action intent, followed by the same queued
run with `dependency_readiness_pending`, no selected runtime/device and no output.
An action intent is not an executed resolve operation or a Ready proof.

Standalone lifecycle composition supplies real gateway/runtime/device inventory
to the readiness producer. Hosted bundle composition supplied only the default
Python-package inventory. Non-Python dispatch and sources were compiled behind
`standalone`, while desktop enabled Candle/llama.cpp without that feature.
Consequently the hosted producer could return NotImplemented for an available
compiled Candle runtime feature. This is a concrete missing composition path;
the logs alone do not establish that it is the only remaining native failure.

The fix separates `host-dependency-inventory` from standalone process mode.
Standalone includes it transitively; desktop enables it explicitly. A new owned
startup option supplies the desktop's actual app-data directory. Bundle creation
injects read-only managed-runtime inventory and gateway-backed runtime-feature
and device-toolchain sources before spawning the actual readiness producer,
using the same gateway as dispatch. Existing callers default to no host context.
This performs no install or model/catalog download and makes no CPU/GPU capacity
or serial scheduling capability claim. Unsupported features remain unavailable.

## Validation and limits

The scheduler, runtime-host contracts and workflow-service suite passes 1,252
unit/integration/doc tests with zero failures and one existing ignored benchmark.
Workflow-service contributes 955 unit passes. All-target Clippy passes with
warnings denied except existing inference dead-code fields. Logs are retained
under `/workspace/pantograph-cache/serial-ready-integrated-*` in this executor.

The hosted-inventory feature compiles with default embedded backends without
standalone. Composition tests exercise an actual bundle, its shared registry and
work queue, the spawned producer and published snapshot: real compiled Candle
request lifecycle is Ready with explicit context; Candle external connection is
Unavailable with the runtime-feature source diagnostic; absent context remains
NotImplemented. These tests execute no model. Public startup option ownership is
checked separately; managed-runtime filesystem lookup is source-reviewed.

The broader default hosted library run exposed 21 additional legacy proof
fixtures. After aligning the four owned fixture files with the producer's
optional task type and actual platform, 566 pass, two are ignored, and one
reserved rerank fixture still fails at admission. The real local Candle CPU
embedding graph fixture now passes through the public scheduler with its vector,
metadata and usage assertions intact. Rerank/audio worker files remain untouched:
their planning helpers still overwrite `task_type` with `Some(task_id)`, producing
a different requirements identity from production's `None`. The rerank failure
is `saved_cpu_rerank_graph_reopens_and_matches_parent_outputs_and_selected_identity`
in `cpu_rerank_graph_tests.rs:73`. Three analogous audio fixtures need their
owner's semantic alignment before qualifying audio-feature suites. This is not a
reason to weaken production identity checking or skip existing assertions.

The complete eight-package library run with standalone and PyTorch features,
using `--no-fail-fast`, finishes with 2,778 passes, one reserved rerank fixture
failure and four existing ignores. No other package fails. Hosted inventory
all-target Clippy passes with `-D warnings -A dead_code` (the allowance is for
existing inference fields), and formatting passes. The integrated frontend suite
passes 673 tests and TypeScript checking. Final local logs are
`hosted-readiness-combined-all-tests.log`, `hosted-readiness-full-tests-aligned.log`,
`hosted-readiness-clippy.log`, `serial-ready-integrated-frontend-tests.log` and
`serial-ready-integrated-typecheck.log` in `/workspace/pantograph-cache`.

The native GUI rerun is blocked locally by missing GTK3/WebKit2GTK prerequisites.
No public run was triggered. The authorized artifact Library materialization
helper failed with `library file transfer failed: download failed`; therefore
the native artifact ZIP's local bytes and hash are unverified. No alternate
storage URL or connector was used. Public logs and source were inspected instead.

This readiness fix does not implement the native serial owner capability. Warm
reuse needs an owner-held calibration-generation transition, and cleanup needs
generation-fenced stop/reconciliation across direct gateway entry points. Those
architectural decisions remain prerequisites to native activation. The default
PriorityThenFifo policy, starvation boosts and one-position warm-reuse window are
unchanged. No full research scheduler or native end-to-end success is claimed.
