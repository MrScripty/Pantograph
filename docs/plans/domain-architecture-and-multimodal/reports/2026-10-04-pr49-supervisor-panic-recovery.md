# PR49 replacement supervisor panic recovery

Status: implemented; local inference acceptance satisfied, independent review
and integration owned by the coordinator. No merge or review request made.

## Source and scope

- Base and single parent: `24bcbd409df0f30f99164874bbb908656f370d56`.
- Base tree: `8589a0f826b79134d284f38fa0aa6920abed52e0`.
- Branch: `fix/pr49-replacement-supervisor-panic`.
- Standards inspected at `366c1d90a24bbfb50973f62b155a5f3396c0f107`.
- Finding: [PR49 supervisor panic custody](https://github.com/MrScripty/Pantograph/pull/49#discussion_r4179391944).
- Write set: `crates/inference/src/gateway_embedding_replacement.rs`, its
  existing `gateway_embedding_replacement_tests.rs`, and this report.
- Full PR49 stacked ancestry and dependency pins remain intact. The independent
  `feat/workflow-text-token-limit` worktree is unchanged by this repair.

A resident backend panic in `stop()` terminates the replacement supervisor.
The base reports that JoinError to the requesting caller but permanently retains
the shared failed completion. Every later lifecycle mutation drains the same
cached error and fails before it can recover.

The repair awaits the actual supervisor join, then retires only that completed
shared future. An identity check prevents a delayed observer from removing a
newer entry; a completion check prevents retiring live custody. The originating
caller still receives `SwitchFailed` with the supervisor panic. A later drain
establishes termination and logs an observed supervisor error instead of
attributing an earlier request's failure to the new operation. Cancellation and
caller loss retain supervision until termination. No join is detached or replaced
with a timeout, task identifier, or inferred completion.

## Executed evidence

| Claim / command | Result and boundary |
| --- | --- |
| `cargo test --locked -p inference --features backend-candle --lib supervisor_panic_reports_to_caller` with the new regression and unchanged base implementation | Red: the requesting caller received the expected panic error; the next replacement failed with the identical cached JoinError. |
| `cargo test --locked -p inference --features backend-candle --lib` after repair | 462 passed, none ignored. |
| `cargo test --locked -p inference --lib` | 422 passed, none ignored. |
| `cargo clippy --locked -p inference --features backend-candle --lib --tests` | Passed; existing `SelectedTextLoad` dead-field warning remains. Warnings were not denied. |
| `cargo fmt --all -- --check` and `git diff --check` | Passed. |
| Traceability gate tests | 28 passed. |
| Critical and accessibility gates | Passed; 27 accessibility checker tests passed. |
| `npm audit --omit=dev --audit-level=high` | Zero vulnerabilities. |

The real embedding checks load frozen synthetic CPU BERT weights of dimensions
8 and 12. They execute forward inference and verify that the surviving resident
and later replacement return the correct dimensions. The retirement panic is a
controlled backend fault. The caller-loss case injects a panic through the
existing publication test hook, proves that stop waits while the supervisor is
live, and observes successful stop and selected execution after its actual join.
The custody check also exercises a completed older observer against a newer live
completion. Existing cancellation, load-join and late-publication checks remain
in the aggregate.

Local logs use `/workspace/pantograph-cache/pr49-supervisor-panic-*.log`.
The environment is the published Codex Linux checkout, Rust 1.92.0, regular
speed, one Cargo build job, with the existing toolchain activation script.

No GUI, desktop IPC, GPU, pretrained model, or text-to-image acceptance was
executed for this narrow repair. PyTorch mocks are not evidence of model
inference. The coordinator separately reported all three hosted workflows green
on the exact base; this report does not claim hosted qualification of the repair.

## Handoff lifecycle

Retain this branch and worktree as a protected review candidate. The cloud lane
owns the source repair; the parent owns review, integration and eventual branch
retirement. Do not advance PR49's reviewed branch as part of this handoff.
