# CPU image result verification — 2026-10-13

Implementation is complete and independently reviewed. C3 passed on exact source
051b08ed774c7b22f22c6242bacc8fb16d4238e2 through the existing sandboxed Actions
frontend job. All 21 checks attached to that head passed, including Quality summary.
Implementation acceptance is complete; publication/integration requires the final
committed-head gates, recorded in [PR74](https://github.com/MrScripty/Pantograph/pull/74).
This qualifies controlled CPU browser presentation, not generated-image capability.

## Passed evidence

Environment: saved Codex Cloud CPU workspace; Node v24.12.0, npm 11.6.2, Chromium
151.0.7922.173 (Debian 13). Commands use `/workspace/pantograph-tools/activate.sh`.

| Claim | Command / observation | Result |
| --- | --- | --- |
| C1 | `node --experimental-strip-types --test src/components/nodes/workflow/imageOutputArtifact.test.ts src/components/workflowToolbarEvents.test.ts` | 33 passing tests; actual committed Rust host reference fixture traverses toolbar output projection; strict ref/descriptor/read/byte rejection |
| C2 | Same focused suite | Unsupported image-input rejection preserves authored data and existing capability_violation / unsupported details through normalization and visible error formatter |
| C3 | [Hosted frontend job 115599420788](https://github.com/MrScripty/Pantograph/actions/runs/38485919458/job/115599420788) | Exact-source Chrome 154.0.8037.97, sandbox enabled, all 18 browser outcomes and cleanup passed; actual downloaded PNG/JPEG bytes match retained artifacts |
| C4 | `npm run test:frontend` | 731 passed, zero failures/skips |
| C4 | `npm run typecheck` | Passed |
| C4 | `npm run lint:full` and `npm run lint:critical` | Passed |
| C4 | `npm run lint:a11y` | 27 checker tests and Svelte accessibility gate passed |
| C4 | `npm run build` | Passed; actual Svelte production compilation |
| C4 | Browser runner syntax and strict fixture TypeScript | Passed |
| C4 | Staged decision traceability | Passed; 16 changed paths, zero mapped impacts |
| C4 | Independent source review and `git diff --check` | Passed; no unresolved code findings |
| C4 | [Quality Gates](https://github.com/MrScripty/Pantograph/actions/runs/38485919458) and checks attached to source 051b08ed | All 21 completed successfully, including Quality summary, CodeQL, runtime separation, contract/parity and existing native text controls; other jobs retain their established checkout semantics |

Focused evidence deliberately does not claim the short unit byte fixture decodes
as an image. Real PNG/JPEG canvas encoding, browser decoding, downloaded bytes,
keyboard/modal focus, supersession and Blob URL cleanup are assertions in
[the browser fixture](../../../../tests/e2e/image-output-artifact/README.md).
That fixture uses the actual image component and decoder with a controlled graph
shell, store and artifact transport. It does not qualify the full workflow editor,
Tauri, runtime inference, diffusion generation, native desktop or GPU behavior.

## Hosted browser acceptance — 2026-10-14

The frontend job in [Quality Gates run 38485919458](https://github.com/MrScripty/Pantograph/actions/runs/38485919458)
checked out and verified source 051b08ed774c7b22f22c6242bacc8fb16d4238e2.
The actual success receipt records `sandboxMode: enabled`, headless CPU rendering,
GPU disabled, fresh temporary profile/config/cache, loopback-only fixture, and
`cleanup: completed`. [Evidence artifact 11860640474](https://github.com/MrScripty/Pantograph/actions/runs/38485919458/artifacts/11860640474)
uploaded successfully. An independent GPT-6.1 Sol High reviewer parsed the actual
job log and verified its source, environment, all 18 outcomes and cleanup.

- PNG: native decode 48×32, 226 exact bytes downloaded as output.png (image/png).
- JPEG: native decode 64×40, 1168 exact bytes downloaded as output.jpg (image/jpeg).
- Native Tab/Enter expansion, modal focus, Escape, restored Expand focus and pointer
  Expand/Close passed; image control events did not escape into the parent shell.
- Replacement/clearing closes the modal and releases the old preview URL.
- Malformed refs, partial/nonimage/unreadable data, descriptor/read identity mismatch,
  MIME mismatch, read failure and native raster decode failure show errors.
- Superseded read success/failure and descriptor success cannot publish; unmount
  releases every live URL and pending read completion cannot publish afterward.

The fixture uses the actual Svelte image component and decoder with controlled
transport/store/shell and real canvas-encoded raster bytes. It does not qualify
the full editor, native desktop, diffusion execution, model usefulness or GPU.

## Superseded startup and failure observations

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
Enter now carries CR text/unmodifiedText; Tab/Escape use rawKeyDown. Superseded
[d45b0e89 job 115580714200](https://github.com/MrScripty/Pantograph/actions/runs/38480225647/job/115580714200)
preserved the precise modal-open timeout without a masking cleanup error.
All native modal/focus assertions and cleanup then passed on 051b08ed.

## Independent review

The initial plan review approved the frontend-only direction with no blockers.
Fresh worker `image_review_sol`, explicitly configured gpt-6.1-sol / high, reviewed
completed production code, tests and browser fixture. It independently ran all 33
focused tests, additional UTF-8/control/surrogate probes and diff whitespace checks.

One finding was fixed: a plain local unsupported Error was being normalized to
transport_error by the toolbar. `UnsupportedWorkflowInputError` now uses the
existing WorkflowServiceError capability_violation category with unsupported
status details. The reviewer verified the actual normalization and display path.
Final review: no unresolved production code findings. Follow-up review verified
the complete actual hosted C3 success receipt; no browser acceptance blocker remains.
