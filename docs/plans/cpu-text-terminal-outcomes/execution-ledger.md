# Execution ledger

- Parent explicitly confirmed separation: M3 leaves finalization files untouched;
  its Start changes are session_scheduler_runner.rs/task_execution_worker.rs at
  e9d89eac. Created clean separate branch/worktree from frozen PR80 040969de.
  No source edit preceded this coordination. Parent authorizes the T4 fix/PR.
- Independent text_review (GPT-6.1 Sol / High) admitted the bounded projection
  design: coherent staged Failed result + TerminalFailed record + first Error
  diagnostic, typed cancellation by exact code, all other errors preserved as
  Internal. Generic incomplete behavior remains for missing/incoherent facts.
- PR80 CI monitor: CPU text and all existing workflow jobs remain queued without
  logs or conclusions; no CI failure is attributed or repaired yet.
- Source review admitted without material findings. Reviewer independently
  inspected causal precedence, full result identity, producer Error ordering,
  exact cancellation codes, fail-closed behavior and strict probe admission.
  Reviewed owner SHA256 be8dce090f87b4cc09ed5055cbda0ce77d17ea21ee990fceb28b4e0ae6098393;
  probe d6139d555d2a0eb724a7da60af9a24cbb8deb75de94cb01ed831d5a3990ce53b.
- Fresh fetch confirmed origin/main remains 4aec0d5c4927a6af2832d40e224be5c2136fc9ef.
  Original /workspace/Pantograph and frozen /workspace/pantograph-text worktrees
  both remain clean; no README or old evidence changed.
- F1 focused owner regressions passed: 8/8 (including four new cases), command
  `cargo test --locked -p pantograph-workflow-service --lib runtime_branch_run_finalization`.
  Full default workflow-service library passed 992/992; retained raw log at
  /workspace/text-t4-evidence/workflow-library.log. The default backend-disabled
  inference dependency emits its existing unused SelectedTextLoad-fields warning;
  the selected CPU-feature warnings-denied Clippy gate is still pending here.
- Parent relayed a 23:33 UTC environment-disconnect callback. Direct shell,
  repository, source hashes and the existing Cargo PID 17602/session 43053 were
  verified live. The owner/probe hashes still match independent review. The
  already-running embedded library test was retained without restarting or
  duplicating workers. Last committed baseline remains frozen PR80; current
  T4 edits are isolated in the declared worktree and still unqualified pending
  embedded/static and actual public-path evidence.
- Full embedded CPU-feature library passed 619 tests with 3 ignored; command
  `cargo test --locked -p pantograph-embedded-runtime --features backend-pytorch,host-dependency-inventory --lib`.
  Raw log retained at /workspace/text-t4-evidence/embedded-library.log.
- Staged traceability initially found a premature link to the not-yet-created
  reports directory. Removed that link until exact-checkpoint reports exist;
  no implementation or acceptance criterion changed.
- F3 supporting gates passed: workflow-service and embedded owner Clippy,
  lib/tests/example with selected PyTorch+inventory features and warnings denied;
  affected rustfmt, Bash syntax, staged traceability and whitespace checks.
  Raw Clippy log retained at /workspace/text-t4-evidence/clippy.log. Eighty-one
  frozen baseline files (README, M3 callers, all prior plan/reports/evidence)
  remain byte-identical. F2 is pending clean-checkpoint execution; T3 stays blocked.
- Before source checkpoint, the staged diff contains exactly six declared text
  files, no credentials/configuration/weights/binary artifacts, and no unrelated
  or M3 edits. Independent source review admitted this exact owner/probe pair.
