# PR49 typed embedding model-name compatibility

Status: implemented for coordinated review; no merge or external review request.

## Source and acceptance

- Base and single parent: `61141f1fe469263d7aa47c79a0f84160780b25ba`.
- Branch: `fix/pr49-replacement-supervisor-panic`.
- Standards: `366c1d90a24bbfb50973f62b155a5f3396c0f107`.
- Finding: [PR49 final risk summary](https://github.com/MrScripty/Pantograph/pull/49#issuecomment-5979194388), coordinator-reported run `acbee1c0`.
- Write set: selected embedding validation, existing gateway replacement tests,
  and this report. Full stacked ancestry and dependency pins are preserved.
- Separate `feat/workflow-image-controls` work remains isolated and uncommitted
  while this repair is qualified. PR49's reviewed branch is not advanced here.

The supported typed gateway call validated the authoritative model references
but ignored its optional legacy `model_name`. A request naming the old 8-wide
model while selecting the 12-wide model replaced the resident and returned
12-wide embeddings. This conflicts with the existing legacy name guard.

Selected embedding validation now rejects a nonempty conflicting name before
replacement or load. `None`, the intentionally supported empty string, and exact
identities with or without the `pumas://models/` prefix remain accepted. Basename
aliases and whitespace are rejected. Revisions and artifact checks remain in
their existing authoritative references; no generic identity framework is added.

## Executed evidence

| Command / observation | Result and boundary |
| --- | --- |
| New typed mismatch regression on unchanged base, `cargo test --locked -p inference --features backend-candle --lib typed_model_name_mismatch_preserves_resident_before_replacement` | Red: returned real 12-wide model B embeddings despite caller name A. |
| `cargo test --locked -p inference --features backend-candle --lib` after repair | 463 passed, none ignored. |
| `cargo test --locked -p inference --lib` | 422 passed, none ignored. |
| `cargo clippy --locked -p inference --features backend-candle --lib --tests` | Passed; existing `SelectedTextLoad` dead-field warning, warnings not denied. |
| `cargo fmt --all -- --check`, `git diff --check` | Passed. |
| Critical/accessibility gates | Passed; 27 accessibility checker tests. |
| Traceability checker tests | 28 passed. |
| `npm audit --omit=dev --audit-level=high` | Zero vulnerabilities. |

The regression executes frozen synthetic CPU BERT forward inference. It verifies
all rejected names retain the old 8-wide resident, its config and instance ID,
and zero stop calls; accepted names return 12-wide embeddings with exactly one
replacement stop. Existing supervisor join/cancellation/caller-loss checks run
in the aggregate. No worker custody logic changes.

Logs: `/workspace/pantograph-cache/pr49-typed-model-name-*.log`, regular speed,
Rust 1.92.0, one Cargo build job. No GUI, desktop IPC, GPU, pretrained model or
image generation was executed for this repair. The cloud ONNX download limitation
remains owner-deferred and is unrelated to these executed Candle checks.

Retain this branch/worktree for parent-owned review and integration. No reviewed
PR49/50 ref is changed by this upload.
