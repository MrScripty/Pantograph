# Workflow Contract Fixture Repair

Date: 2026-10-02. Status: `Verifying`.
Authority: [domain architecture plan](../plan.md), CI-01 baseline qualification.
Branch: `fix/workflow-contract-fixtures-2026-10-02`, based on prerequisite
`4cf2317b5d541e534cd953dddb9a3aa919fceb52` ([PR #6](https://github.com/MrScripty/Pantograph/pull/6)).
The workflow-service test owner implements this slice; the domain integrator owns
independent review and integration. Retain the branch until integrated, rejected
or superseded. Coding standards inspected at
`MrScripty/Coding-Standards@dcc56f26e884ade260770beceba2501d3746200d` include
Implementation, Verification, Commit, Documentation and Rust tooling guidance.

## Reproduction

The prerequisite branch installed the pinned C# generator successfully. Its
[headless run](https://github.com/MrScripty/Pantograph/actions/runs/37075520823)
then stopped at `cargo test -p pantograph-workflow-service`: 852 library tests
passed and 20 failed. All 20 failure identities match the earlier base-4938e40
library run. The earlier aggregate of 23 also included three integration-contract
failures reached with `--no-fail-fast`. Comparing the prerequisite head with base
shows no workflow-service, Cargo manifest or lockfile source changes.
The original assertion logs were retained during diagnosis.

PR #6 also passed the runtime-separation workflow and workspace checks with both
no default features and all features. Those receipts qualify provisioning and
native compilation, not the skipped generated-binding smoke or whole CI.

## Corrected Preconditions

The shared inference graph fixture omitted required `runtime_source_context`,
so publication rejected its request before lifecycle and dependency assertions.
Supply the same explicit operation, context shape and cancellation facts as the
existing publication tests. No production fallback or validation relaxation is
introduced. The stale-revision test also raced the new automatic validation
started by graph mutation. Use its existing sequenced facts provider to distinguish
both requests, then drain background validation before asserting the new revision
is Current. All original stale-response and current-summary assertions remain.

The classifier fixture demanded a built-in contract for retired `expand-settings`.
Assert that the registry omits that type and that a missing contract is rejected;
retain rejection checks for existing excluded and unknown node types.

The missing-candidate test supplied neither a runtime requirement nor a selected
model, making readiness irrelevant, and supplied a differently failing runtime.
Declare its required backend and omit candidate capabilities so the unchanged
incomplete-candidate warning and blocking assertions exercise their intended path.
Existing unavailable-runtime and irrelevant-runtime tests remain intact.

## Verification And Open Work

- Fresh Cargo compilation and complete library run: 867 passed, five failed,
  zero ignored. All 15 repaired failures passed; only the unchanged capacity
  tests remain. This is not a green aggregate claim.
- The five capacity fixtures execute a source/output graph without inputs and
  expect runtime admission; non-runtime execution intentionally does not load a
  runtime. Their admission/lifecycle setup requires a separate bounded repair.
- Three earlier integration-contract failures and other CI lint, audit and Rustler
  failures remain separate. No workflow check or test has been disabled.
- Focused library run excluding the unchanged nine-test capacity module: 863
  passed. The synchronized race passed ten isolated repetitions. Cargo format,
  staged whitespace and staged traceability passed.
- Independent integrator review accepted staged tree
  `0650a9620edeff2c943f649147e5e2be06b460f8` after inspecting the changes,
  surrounding source, report and whitespace. This report records that bounded
  acceptance afterward. Exact-head hosted qualification remains pending.
