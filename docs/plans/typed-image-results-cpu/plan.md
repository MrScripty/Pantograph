# Typed image results on CPU

Status: Verifying. Acceptance: partial. Phase: browser acceptance.
Next slice: qualify sandboxed Chrome in the existing Actions frontend job and inspect final-head checks.

## Objective and scope

Make Graph Image Output consume the scheduler's path-free media artifact reference,
read its retained bytes, and show/download the actual image with correct MIME type.
Reject unsupported image-input submission explicitly. No image generation, weights,
GPU, native desktop, remote workers, or scheduler capability expansion is included.
User authorization selects this plan and the start operation after independent review.

## Decisions and ownership

- Frontend owner: this branch, `feat/typed-image-results-cpu-2026-10-13`, based on
  `07b23d47`, targeting main by PR; retain until integration/rejection. Parent owns
  integration and all shared workflow schema changes. No shared schema change needed.
- Preserve Rust `WorkflowSchedulerTaskMediaArtifactRef` authority: artifact_id and
  optional media_type only. Decode at the frontend presentation boundary; fetch by ID
  through existing artifact APIs, never interpret a reference as base64 or a path.
- Use existing complete binary artifact read contract. Only supported raster MIME
  types can become image previews; reject incomplete/mismatched bodies. Blob URL
  lifecycle belongs to the component and ends on replacement/unmount. Superseded
  reads cannot publish. Decode failures show compact errors and disable download.
- Existing base64-only legacy image output is unsupported by this scheduler path;
  show that state explicitly. Do not invent PNG metadata or silently ignore values.
- Backend remains the authority for image-input unsupported status. Frontend submission
  should expose that known limitation before silently dropping input bytes.
- One coherent production slice; independent read-only reviewer works in parallel.
  Local unsupported input maps to existing capability_violation with details.status
  unsupported, preserving the current toolbar error protocol without schema changes.
  Browser evidence may use controlled artifact transport with real raster bytes,
  explicitly excluding runtime/inference qualification.
- Composed-design review: not-applicable; existing component/service ownership and
  public contracts remain intact; a small presentation decoder replaces an invalid
  string interpretation. No new service, persistence, or orchestration mechanism.

## Milestone M1 — typed result presentation (Verifying)

Write set: `src/components/nodes/workflow/ImageOutputNode.svelte`, adjacent
`imageOutput*.ts`, `src/components/workflowToolbarEvents.ts` and its tests,
`tests/e2e/image-output-artifact/`, and this plan directory. No shared types,
Rust, lockfiles or runtime worker changes. Adjacent CI write set: the existing
`frontend-tests` job in `.github/workflows/quality-gates.yml` and fixture evidence
reporting; user requested hosted sandbox acceptance after local startup was blocked.

Claims (all automated): C1, C2 and C4 satisfied by [verification](reports/verification.md);
C3 pending hosted sandboxed browser acceptance. Source implementation and independent review are complete.

| ID | Criterion | Kind | Environment |
| --- | --- | --- | --- |
| C1 | Actual Rust media-ref contract decoded; malformed refs and incomplete/nonimage/mismatched bytes rejected | contract | not-applicable |
| C2 | Unsupported image-input cannot silently disappear at submit boundary | focused | not-applicable |
| C3 | Actual Svelte node decodes raster fixtures, expands, downloads matching format, shows failures, clears stale output, releases URLs | user-workflow | representative Chromium with controlled transport |
| C4 | Existing affected frontend tests, typecheck, lint and build pass; independent review has no unresolved blockers | focused | not-applicable |

Gates: focused Node tests, Chromium fixture, frontend suite, typecheck, full/critical
lint, production build, staged diff review. No tests claim diffusion execution.

## Constraints, blockers and replanning

No AGENTS.md present in repo/workspace at inspection. Standards read from
Coding-Standards `188beda1fa477d21d576c233dd8c7f4c4c267d23`: Core/Router,
Implementation, Planning, Proportionality, Verification/GUI/Oracles, Commit,
Documentation, Contracts, Security, Concurrency, Accessibility, Frontend,
TypeScript and TypeScript Async.

Blockers: Chromium setuid and user-namespace sandbox probes both failed. C3
will use the existing GitHub-hosted Ubuntu frontend job with preinstalled Chrome
and its ordinary sandbox; no override, security setting, permission or credential
change. The job checks out and verifies the requested source head, preserves
pipefail, and uploads the test log through the existing artifact action convention.
Hosted failure leaves C3 blocked. Independent reviewer approved this bounded route.
Replan if shared schema changes, unsupported source execution, runtime host work,
or broader media families become necessary. Do not weaken sandbox silently.

[Ledger](execution-ledger.md) · [Issues](issues.md) · [Evidence](reports/verification.md)
