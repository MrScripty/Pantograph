# Wave 05: API Bindings Frontend Settings

## Objective

Project ArtifactStore, format settings, media dependency capabilities, and
binary-safe payload access through API/bindings and the workbench Settings page.

## Dependencies

Waves `01` through `04` must provide stable backend contracts and service
commands before frontend or host-binding implementation starts.

## Workers

Parallel workers may be split across API transport, supported bindings, and
frontend only after the host records exact non-overlapping files in the
coordination ledger.

## Candidate Write Sets

- Tauri/frontend HTTP adapter command surfaces and TypeScript service types.
- UniFFI/C# DTO projection and host smoke tests.
- Workbench Settings page native persistent settings surface.
- Output-node format selectors driven by backend capability DTOs.
- I/O Inspector binary artifact browsing and consume acknowledgement.

## Forbidden Files

- Backend contract files unless host reopens contract freeze.
- ArtifactStore storage internals unless a blocking integration bug is
  recorded.
- `.pantograph/**`, `assets/**`, generated output, and unrelated manifests.

## Standards

Frontend, accessibility, language bindings, interop, security, release,
testing, and documentation standards.

## Verification

Defined by implementation owner before launch; must include frontend tests,
binding smoke/DTO parity tests, binary-safe retrieval tests, and Settings
ownership checks.

## Report Path

`reports/wave-05-worker-<name>.md`

## Escalation Rules

Escalate if frontend hardcodes media option lists, owns persistent global
settings outside the Settings page, or host bindings expose inline media JSON.

## Integration Order

Transport/API types before bindings and frontend; Settings page before
feature-local selectors.
