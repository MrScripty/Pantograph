# 2026-04-29 Worker Report: C# Session Request Repair

## Scope

Owned write set: `bindings/` and `scripts/`.

Repair C# smoke and quickstart request examples after the workflow-service
session-run contract started requiring `workflow_semantic_version`.

## Changes

- Added `workflow_semantic_version` to C# native smoke session-run requests.
- Added `workflow_semantic_version` to the direct-runtime quickstart
  `--run-session` request.
- Changed the direct-runtime quickstart workflow identity to a validated
  Pantograph workflow identity.
- Updated the C# smoke surface check to reject the removed direct generated
  `WorkflowRun` method.

## Verification

Passed:

- `./scripts/check-uniffi-csharp-smoke.sh`
- `PANTOGRAPH_PACKAGE_PROFILE=debug ./scripts/package-uniffi-csharp-artifacts.sh`
- `./scripts/check-packaged-csharp-quickstart.sh`

Notes:

- The generated C# binding formatting step emitted the known CSharpier
  availability warning and still completed successfully.
