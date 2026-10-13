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
The runner retains sandboxing by default. The user selected hosted ordinary
sandboxing; no override has been executed or is needed for that route.

Hosted Google Chrome 154.0.8037.97 started with sandboxing enabled in superseded
[f9d46511 job 115573112824](https://github.com/MrScripty/Pantograph/actions/runs/38477919111/job/115573112824).
[d0b92652 job 115574792903](https://github.com/MrScripty/Pantograph/actions/runs/38478429385/job/115574792903)
advanced past real raster/download assertions into keyboard checks, then failed;
old cleanup masked the precise primary assertion with ENOTEMPTY. Neither run
establishes C3 or final-source acceptance. The runner now owns browser-group
shutdown through profile release and preserves primary failures.

Independent review also established a keyboard-driver defect from Chromium's
[keypress dispatch](https://github.com/chromium/chromium/blob/main/third_party/blink/renderer/core/input/keyboard_event_manager.cc)
and [Puppeteer's CDP driver](https://github.com/puppeteer/puppeteer/blob/main/packages/puppeteer-core/src/cdp/Input.ts).
Enter now carries CR text/unmodifiedText; Tab/Escape use rawKeyDown. All native
modal/focus assertions remain. The next exact-source hosted run must pass them
and cleanup before C3 can be accepted.

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
