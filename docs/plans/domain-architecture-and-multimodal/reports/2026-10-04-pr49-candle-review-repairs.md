# PR49 Candle review repairs

This candidate is a separate descendant of passing PR49 head
`01ee5b0cd3ffd3579fd1cb2aa1efcbc1dd833a42`, tree
`e23e71094c9f2e71d2dae1692c3ce03f015b1897`, on branch
`fix/pr49-candle-review-repairs`. It retains the full stacked ancestry, including
PR44 `ca9edde6850cd58ade0b7e534bb4f58e704c2c64`. The separate pooling change
`495efea606620153fe2f0363b104e56518436a06` is excluded.

## Findings and bounded changes

- [Repeated blocking reload](https://github.com/MrScripty/Pantograph/pull/49#discussion_r4177861531):
  compare the complete executable target and load plan, exact small input bytes,
  and checkpoint file identity before reusing the resident model. File identity
  includes canonical path, length and modification time, plus device, inode and
  change time on Unix. A changed revision, fingerprint, configuration or physical
  checkpoint triggers candidate loading. This is local snapshot validation, not
  a replacement for producer validation or a cryptographic checkpoint scan.
  Reuse preserves the gateway runtime instance ID and reports the backend's
  actual reuse outcome.
- All synchronous plan validation, file reads, tensor deserialization and BERT
  construction run on a blocking worker. The backend retains the worker's actual
  join through the existing completion mechanism. Dropping the caller signals
  cancellation without retiring that join; replacement and stop drain it. Host
  cancellation is checked at load stage boundaries and before publication. A
  failed or cancelled candidate leaves the previous model resident. Native work
  is cooperative, not preemptible; stop waits if a native stage does not return.
  The original backend loading method remains available, with an additive
  cancellation-aware method used by the gateway.
- [Optional revision direction](https://github.com/MrScripty/Pantograph/pull/49#discussion_r4177861535):
  omitted caller/scheduler revisions permit additional producer target evidence.
  Every explicitly requested revision must match both package and target. A
  known package revision must be preserved by the target. Model, artifact,
  contract, task, device and architecture admission checks remain intact.
- [Historical producer pin](https://github.com/MrScripty/Pantograph/pull/49#discussion_r4177861543):
  the configured-owner report now labels `f87c3da8` as that historical slice's
  pin and identifies the integrated current pin
  `5be6d967dbd5c0ff7449f342e77f05f9cd645a8e`. Historical receipts are retained;
  dependency manifests and locks are unchanged.

## Executed regression evidence

Two new regression tests were transplanted into an isolated worktree containing
the exact base production code. Both failed: a second identical model load
reported `runtime_reused = false`, and target-only revision evidence was rejected
when request, scheduler and package revisions were omitted. The isolated red
worktree is not part of the candidate ancestry.

| Check | Result and scope |
| --- | --- |
| `cargo test --locked --offline -p inference --features backend-candle --lib` | 454 passed, zero failed or ignored |
| `cargo test --locked --offline -p inference --lib` | 421 passed, zero failed or ignored |
| `cargo clippy --locked --offline -p inference --features backend-candle --all-targets` | Completed; only the existing selected-text unused-fields warning |
| `node --test scripts/check-decision-traceability.test.mjs` | 28 passed |
| Critical anti-pattern and accessibility gates | Passed; 27 accessibility checker tests passed |
| `TRACEABILITY_MODE=staged npm run lint:no-new` | Passed; no traceability map or gate modification |
| `ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo check --locked --offline -p pantograph-embedded-runtime -p pantograph-workflow-service -p workflow-nodes --tests --features workflow-nodes/model-library` | Passed with unchanged dependency features; compiler evidence only |
| Rust formatting and whitespace checks | Passed |

The five added tests cover exact reuse and changed input/target snapshots,
dropped-caller worker custody through replacement and stop, host cancellation
after actual BERT construction, fourteen revision evidence combinations, and
gateway reuse with omitted request revisions and stable runtime identity.
The cancellation tests use a timing hook after real checkpoint deserialization
and real BERT construction. They do not substitute mocked models or vectors.
Existing actual CPU F32 forwards, independent Torch goldens at widths 8 and 12,
invalid recipe/identity rejection and failed-worker recovery tests still pass.
The frozen untrained fixture bytes are unchanged. These fixtures establish
numerical execution and lifecycle behavior, not pretrained semantic quality.

## Integration limits and coordinator handoff

The parent's passing hosted receipts at the base are Quality `37204567391`,
Headless `37204567380`, and Runtime Separation `37204567384`, with 209
model-library and 462 embedded-runtime tests, including real IPC and actual
Candle vectors/rejections. Those receipts qualify the base, not this descendant.

Cloud native-host execution still needs the pinned ONNX Runtime 1.24.2 artifact;
the authorized CDN has returned HTTP 403 here. Compiler checks with
`ORT_SKIP_DOWNLOAD=1` retain the unchanged default dependency features but do not
link or execute ONNX Runtime, native IPC or full workflow acceptance. No older
system runtime substitution, feature reduction or network workaround is used.
GUI, GPU and pretrained inference are unexecuted.

The coordinator must rerun the three hosted workflows and unchanged-feature
model-library/embedded-runtime acceptance on the exact candidate head after
independent review. This lane only publishes a separate source branch and
handoff; it does not advance PR49, merge, resolve review threads, or request
external/bot review. Audit and traceability gates remain intact.
