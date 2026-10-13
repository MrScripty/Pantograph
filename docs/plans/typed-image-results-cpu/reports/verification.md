# CPU image result verification — 2026-10-13

Implementation is complete and independently reviewed. C3 is pending sandboxed
hosted acceptance through the existing Actions frontend job after local Chromium
sandbox startup failed. No browser assertion or generated-image capability is
claimed until that exact-source run completes successfully.

## Passed evidence

Environment: saved Codex Cloud CPU workspace; Node v24.12.0, npm 11.6.2, Chromium
151.0.7922.173 (Debian 13). Commands use `/workspace/pantograph-tools/activate.sh`.

| Claim | Command / observation | Result |
| --- | --- | --- |
| C1 | `node --experimental-strip-types --test src/components/nodes/workflow/imageOutputArtifact.test.ts src/components/workflowToolbarEvents.test.ts` | 33 passing tests; actual committed Rust host reference fixture traverses toolbar output projection; strict ref/descriptor/read/byte rejection |
| C2 | Same focused suite | Unsupported image-input rejection preserves authored data and existing capability_violation / unsupported details through normalization and visible error formatter |
| C4 | `npm run test:frontend` | 731 passed, zero failures/skips |
| C4 | `npm run typecheck` | Passed |
| C4 | `npm run lint:full` and `npm run lint:critical` | Passed |
| C4 | `npm run lint:a11y` | 27 checker tests and Svelte accessibility gate passed |
| C4 | `npm run build` | Passed; actual Svelte production compilation |
| C4 | Browser runner syntax and strict fixture TypeScript | Passed |
| C4 | Staged decision traceability | Passed; 16 changed paths, zero mapped impacts |
| C4 | Independent source review and `git diff --check` | Passed; no unresolved code findings |

Focused evidence deliberately does not claim the short unit byte fixture decodes
as an image. Real PNG/JPEG canvas encoding, browser decoding, downloaded bytes,
keyboard/modal focus, supersession and Blob URL cleanup are assertions in
[the browser fixture](../../../../tests/e2e/image-output-artifact/README.md).
That fixture uses the actual image component and decoder with a controlled graph
shell, store and artifact transport. It does not qualify the full workflow editor,
Tauri, runtime inference, diffusion generation, native desktop or GPU behavior.

## Browser blocker

`env -u PANTOGRAPH_IMAGE_BROWSER_NO_SANDBOX node tests/e2e/image-output-artifact/run.mjs`
built the actual component successfully, then exited 1 before browser assertions:

> The SUID sandbox helper binary was found, but is not configured correctly.
> /usr/lib/chromium/chrome-sandbox must be owned by root and have mode 4755.

The worker also tested user-namespace sandboxing with `--disable-setuid-sandbox`;
Chromium reported no usable sandbox. No sandbox override has been executed.
The runner retains sandboxing by default and exposes an explicit approved-test-only
opt-in. A pending approval request covers a fresh credential-free profile,
loopback-only fixture, CPU rendering and full owned process cleanup for this run.

## Independent review

The initial plan review approved the frontend-only direction with no blockers.
Fresh worker `image_review_sol`, explicitly configured gpt-6.1-sol / high, reviewed
completed production code, tests and browser fixture. It independently ran all 33
focused tests, additional UTF-8/control/surrogate probes and diff whitespace checks.

One finding was fixed: a plain local unsupported Error was being normalized to
transport_error by the toolbar. `UnsupportedWorkflowInputError` now uses the
existing WorkflowServiceError capability_violation category with unsupported
status details. The reviewer verified the actual normalization and display path.
Final review: no unresolved production code findings; C3 remains unverified.
