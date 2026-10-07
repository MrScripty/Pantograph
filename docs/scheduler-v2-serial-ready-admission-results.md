# Serial admission contract results

Branch: `scheduler/serial-ready-admission`, based on `0bfa4df713552d8baf7900d8245eef3685803b62`. Design checkpoint: `0805d506`. Remote main reverified as `038dacaaa98ebd007c32e13d4608726ca5ccf63a`.

Fresh Rust adds a shared, nonblocking serial admission contract in `pantograph-scheduler`. Preparation refuses Busy before claims. Consuming promotion retains exclusion; acknowledged drained cleanup releases it. Abandoned dispatch and worker panic poison every clone permanently. Six targeted tests cover these transitions and concurrent single-owner admission. No scheduler algorithm or POC source is copied.

Validation with `ORT_SKIP_DOWNLOAD=1`, locked/offline cached dependencies:

- Complete `pantograph-scheduler` suite: 168 passed, zero failed, one existing ignored.
- New contract cases: six passed; independent reviewer reran the cached binary and confirmed six passed, zero failed, zero ignored.
- Clippy for the scheduler crate and all targets with `-D warnings`: passed.
- Workspace format check and `git diff --check`: passed.

Independent review approved the pure ownership contract without a correctness blocker. Its cleanup acknowledgement deliberately remains an unchecked assertion by the trusted dispatch adapter; it does not prove actual host drain, match an attempt/lease, or recover runtime ownership. Those obligations are explicit acceptance gates in the design.

Production wiring is unfinished. The CPU projection worker `01a11075` reserved-file scope was requested but not supplied in this executor. No shared workflow seam, protected PR65 file, inspector/audio/rerank adapter, or other worker branch was edited. There is no available cross-thread messaging tool in this executor. The next integration must coordinate `workflow.rs`, `workflow/service_config.rs`, `workflow/task_execution_worker.rs`, `workflow/session_scheduler_runner.rs`, `workflow/runtime_dispatch_assignment.rs`, and a bounded store-cohort module. It also needs execution-time loaded-owner validation and deterministic host end-to-end tests for singleton membership, actual cleanup and first-two Ready selection.

Defaults and production dispatch order are unchanged. This is a local serial-admission prerequisite, not the completed research scheduler. No publication/merge, model/runtime downloads, generated evidence commits, credentials or network-setting changes occurred. Test logs remain outside Git in `/workspace/pantograph-cache/serial-ready-scheduler-{tests,clippy}.log`.
