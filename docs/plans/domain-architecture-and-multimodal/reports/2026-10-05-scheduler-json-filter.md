# Scheduler-owned JSON selection and extraction

Status: implemented for parent-coordinated review and hosted native tests. The
owner-deferred cloud ONNX linkage limitation does not block source delivery.

## Gap and implemented behavior

Registered `selection-input` and `json-filter` nodes were unsupported by the
scheduler, despite existing core JSON extraction. A session can now materialize
a request-supplied JSON value, extract a configured field or array element, and
pass a string result to a downstream text output or inference prompt.

Selection input preserves JSON-compatible values without stringification or
floating-point integer conversion. Strings, booleans and exact signed/unsigned
integers use existing typed results; objects, arrays, fractions and null retain
the existing JSON result variant. Existing string and 64 KiB serialized JSON
bounds still apply. No new result schema or media/model coercion is introduced.

Graph lowering captures only the filter's path into an owned task template;
later editor-data changes cannot mutate that path. Missing path means the
existing root selection, while an explicitly non-string path is diagnosed.
Readiness waits for its upstream JSON-compatible result and preserves invalid,
blocked and unavailable outcomes. The existing node-engine single-task adapter
performs extraction; `value` and boolean `found` are retained separately. Missing
paths produce null with `found: false`, rather than an invented empty string.

Acceptance includes exact whitespace and `u64::MAX` preservation, root and
missing-path semantics, graph/template wire round-trip and snapshot ownership,
bounded external materialization, typed adapter outputs and a public session
selection→filter→text-output fixture. No ranking, resource, runtime lifecycle,
reservation, persistence-owner or direct execution API change is included.

- Branch: `feat/scheduler-json-filter`.
- Base/single parent: `0b8209c43517314f55b2a22c0f9c121cada6e6c3`.
- Base tree: `3b643f3fd4e94344937161479a63e05f0cf8bf7b`.
- Preserves the frozen text-merge, PR50, PR51 and PR52 stack. PR49 repair and
  `integration/workflow-controls-pr50-52` remain separate preserved refs.
- Pumas pin: `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`, unchanged.
- Standards inspected at `366c1d90a24bbfb50973f62b155a5f3396c0f107`.
- Write set: workflow classification/template/lowering, external materialization,
  result conversion, non-runtime adapter/readiness, focused tests and this report.

This bounded feature follows the coordinator's instruction to continue useful
existing scheduler/inference-plan functionality beyond workflow controls. It
keeps graph readiness with workflow-service and node mechanics with node-engine,
as required by the existing domain plan and ADR-011. The scheduler thesis and
evaluation identities recorded in the predecessor report motivate actual graph
functionality; simulated ranking results are not production acceptance. DA-03
dependent real-model workflow acceptance remains open.

## Executed qualification and limits

| Check | Actual evidence |
| --- | --- |
| `cargo test --locked -p node-engine --lib` | 260 passed, none failed; one existing benchmark-like harness ignored. New test executes actual core JSON extraction through single-task execution for nested strings, exact unsigned integers, missing paths and root selection. |
| Native external materialization, graph snapshot/wire tests, adapter outputs and stored-graph public session | Added/updated and compiler-checked; not linked or executed locally. The public-session fixture uses a test host; it is not model inference. |
| `ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-workflow-service -p pantograph-embedded-runtime --features backend-pytorch --lib --tests` | Final compiler check passed without warnings. First pass found one introduced redundant-guard warning; replaced with a direct typed match before final qualification. |
| Critical/accessibility gates; traceability checker tests | Passed; accessibility checker 27 tests and traceability checker 28 tests. |
| Scheduler-only public execution surface gate | Passed. |
| `npm audit --omit=dev --audit-level=high` | Zero production vulnerabilities. |
| Formatting, whitespace and staged/committed-range traceability | Applied before publication. |

Logs: `/workspace/pantograph-cache/scheduler-json-filter-*.log`. Existing toolchain,
regular speed, Rust 1.92.0, one Cargo build job and installed Python 3.12 library.
No frontend code changed; the composition's 659 frontend tests are separate
evidence, not a repeated test run on this candidate.

No GUI, desktop IPC, GPU, model inference or native workflow session ran locally.
No ONNX download/link retry, runtime substitution, dependency workaround,
credentials, permissions or network setting change was attempted. The parent
owns hosted execution, review and integration; no PR or external review request
was created. Preserve this candidate and prior reviewed heads independently.
