# Typed image results on CPU

Status: Verifying. Acceptance: partial. Phase: native dialog completion.
Next slice: observe native close plus Svelte completion before fixture reopening,
retain every input/focus assertion, and require terminal final-head CI/browser
acceptance before integration and main verification in
[PR74](https://github.com/MrScripty/Pantograph/pull/74).

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

Claims (all automated): C1–C4 satisfied by [verification](reports/verification.md)
on implementation source 051b08ed. Independent review verified the actual hosted
browser receipt. All 21 checks attached to 051b08ed passed, including Quality
summary. Publication and integration still require the final committed-head gates.
Repeated qualification on d9e52e51 failed pointer reopening after native Escape;
C3 is unsatisfied for the current head until the fixture observes completed native
close-event delivery and Svelte updates and passes again. Prior 051b08ed evidence
remains an actual observation; it does not qualify a failing successor.

| ID | Criterion | Kind | Environment |
| --- | --- | --- | --- |
| C1 | Actual Rust media-ref contract decoded; malformed refs and incomplete/nonimage/mismatched bytes rejected | contract | not-applicable |
| C2 | Unsupported image-input cannot silently disappear at submit boundary | focused | not-applicable |
| C3 | Actual Svelte node decodes raster fixtures, expands, downloads matching format, shows failures, clears stale output, releases URLs | user-workflow | representative Chromium with controlled transport |
| C4 | Existing affected frontend tests, typecheck, lint and build pass; independent review has no unresolved blockers | focused | not-applicable |

Gates: focused Node tests, Chromium fixture, frontend suite, typecheck, full/critical
lint, production build, staged diff review. No tests claim diffusion execution.

## Constraints, blockers and replanning

The approved fixture repair observes each real native close event, waits for Svelte
completion, then permits the next CDP action. Three complete Escape/pointer-reopen
cycles must all pass with existing decode/focus/isolation assertions; they are not
retries. No production or workflow-schema change is admitted from this failure.

No AGENTS.md present in repo/workspace at inspection. Standards read from
Coding-Standards `188beda1fa477d21d576c233dd8c7f4c4c267d23`: Core/Router,
Implementation, Planning, Proportionality, Verification/GUI/Oracles, Commit,
Documentation, Contracts, Security, Concurrency, Accessibility, Frontend,
TypeScript and TypeScript Async.

Local Chromium setuid and user-namespace sandbox probes both failed. C3 passed
in the existing GitHub-hosted Ubuntu frontend job with preinstalled Chrome and
its ordinary sandbox; no override, security setting, permission or credential
change. The job checked out and verified source 051b08ed, preserved pipefail,
and uploaded the successful test receipt. Independent review verified all 18
outcomes and completed cleanup. Final committed-head gates remain required for merge.
Replan if shared schema changes, unsupported source execution, runtime host work,
or broader media families become necessary. Do not weaken sandbox silently.

[Ledger](execution-ledger.md) · [Issues](issues.md) · [Evidence](reports/verification.md)
