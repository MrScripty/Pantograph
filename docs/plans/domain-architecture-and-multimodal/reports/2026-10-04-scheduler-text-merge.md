# Scheduler-owned text fan-in

Status: implemented for coordinated review. Native workflow execution is
owner-deferred under the Codex cloud ONNX limitation; this does not block source
delivery or continued work. No merge or external review request.

## Gap, acceptance and source

The existing `merge` node and core node-engine implementation join strings, but
the scheduler classified that node as unsupported. Submitted graphs therefore
could not combine independently completed text branches before a downstream
text output or inference prompt. Merge now has a typed non-runtime scheduler
template and executes through the existing single-task node-engine adapter.

The workflow owner preserves all dependency bindings and existing canonical
ordering (source task ID, then source/target port IDs). Arrival order does not
choose string order. The core node-engine retains its newline separator,
whitespace-only filtering, preserved content and count semantics. The adapter
retains `merged` as a string and `count` as an unsigned integer. Missing or
non-string upstream values reject instead of publishing partial text. An empty
optional input set produces empty text and count zero. Existing task-result
string size bounds remain enforced; oversize joins fail rather than truncate.

- Branch: `feat/scheduler-text-merge`.
- Base/single parent: `762e2b7e4afca0f14781412015a0a6e946bc5ca0`.
- Base tree: `f81698d88586b144bb7b2c3ccf2a776a24c1f86b`.
- Pumas pin: `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`, unchanged.
- Standards: `366c1d90a24bbfb50973f62b155a5f3396c0f107`.
- Write set: task classification and template/graph lowering, orchestrator input
  readiness, existing
  non-runtime adapter, focused graph/orchestrator/public-session tests,
  node-engine single-task tests, and this report.
- PR49–52 candidate identities remain preserved. This branch descends from the
  frozen system-prompt candidate; no earlier branch is advanced. The independent
  PR49 repair and excluded pooling candidate remain separate.

The current domain plan keeps graph readiness with workflow-service and node
mechanics with node-engine. ADR-011 requires session submission and forbids a
new direct execution surface. This slice follows both owners. The scheduler
research was reread from `pantograph_scheduler_thesis.md` (Library identity
`libfile_6c1d9aa6e1f08191b4a14e0de0846220`, version 1) and
`pantograph_scheduler_v2_evaluation.md`
(`libfile_8b6d205bd260819187ef9fbac029a243`). Their joins and completion-oriented
goals motivate useful graph functionality; modeled results do not establish
production acceptance. Their learned ranking/hybrid/residency proposals still
need production capabilities, observation and reservation contracts. This
feature does not import a simulator, invent placement scores, or alter resource
admission, runtime custody, priority, batching or graph correctness.

## Executed evidence and limits

| Command / claim | Result and boundary |
| --- | --- |
| `cargo test --locked -p node-engine --lib` | 259 passed, one existing benchmark-like multi-demand harness ignored. New single-task test executes the real core merge with ordering, whitespace-only filtering, exact content, count and empty-input checks. |
| `cargo test --locked -p pantograph-scheduler` | 130 passed across unit/integration targets, no failed tests. Existing queue/dispatch/readiness/resource contracts remain covered. |
| Graph lowering, wire round-trip, adapter retained outputs, all-upstream readiness, public session fan-in→text-output without runtime load or legacy host execution | Focused tests added/updated and compiler-checked; not executed locally. The public-session test uses a stored graph and host methods that panic if runtime loading or legacy execution is attempted. |
| `ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-workflow-service -p pantograph-embedded-runtime --features backend-pytorch --lib --tests` | Passed compiler check; no native linkage/execution claimed. |
| Formatting and whitespace | Passed. |
| Critical/accessibility gates and traceability checker tests | Passed; 27 accessibility checker tests and 28 traceability checker tests. Staged and committed-range gates are applied for upload. |
| Scheduler-only public surface gate | Passed. |
| `npm audit --omit=dev --audit-level=high` | Zero production vulnerabilities. |

An initial Clippy invocation selected only workflow-service with
`--features backend-pytorch`; Cargo rejected that command because the feature is
owned by embedded-runtime/inference. Zero checks ran in that invocation. The
corrected combined-package command above checks the intended feature composition.
Its first pass found the exhaustive orchestrator readiness match needed the new
template; the explicit merge case now waits for every upstream string and
preserves blocked/unavailable/invalid outcomes. Final compiler qualification
uses the complete readiness and execution slice.

Logs: `/workspace/pantograph-cache/scheduler-merge-*.log`, Rust 1.92.0, regular
speed, one Cargo build job. No GUI, desktop IPC, model inference, GPU, or actual
workflow-session execution ran locally. No network/permission setting, ONNX
download retry, runtime substitution or dependency workaround was made. Parent
owns hosted execution of the changed workflow tests and coordinated review.

Freeze this candidate and retain its worktree for parent-owned integration.
Continue a separate admitted scheduler slice while independent useful work
remains; do not wait for this candidate's review before progressing.
