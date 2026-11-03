# Workflow text generation token limit

Status: implemented and ready for coordinated review. The owner deferred native
qualification on 2026-10-04 because the Codex cloud environment denies the ONNX
Runtime download. This environment limitation does not block source delivery,
review, or continued feature work. Deferred checks remain unexecuted; compiler
and frontend results do not establish native or model inference acceptance.

## Milestone and source

Users can supply an integer number-input value to a text-generation node's
optional `max_new_tokens` port. Scheduler-owned source materialization retains
that value, and the runtime host projects it into typed
`GenerationOptions.length.max_new_tokens`. The existing gateway maps it to the
selected text worker's `max_tokens` command field. Omission preserves backend
defaults. Limits must be integers from 1 through `u32::MAX`; malformed, negative,
zero and out-of-range values receive typed errors before dependency resolution.
The existing 1024-byte text input/output bounds continue to apply; a token limit
does not guarantee that generated text fits the output byte bound.

- Branch: `feat/workflow-text-token-limit`.
- Feature implementation: `f6b6894f6c5b18807bf571b3597d188d47c48a63`.
- Implementation base and single parent: `24bcbd409df0f30f99164874bbb908656f370d56`.
- Base tree: `8589a0f826b79134d284f38fa0aa6920abed52e0`.
- Pumas pin: `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`, unchanged.
- Standards inspected: `366c1d90a24bbfb50973f62b155a5f3396c0f107`.
- The coordinator's independent PR49 repair branch is not included here.
- Pooling candidate `495efea606620153fe2f0363b104e56518436a06` remains excluded.

The inspected research was `pantograph_scheduler_thesis.md` (Library identity
`libfile_6c1d9aa6e1f08191b4a14e0de0846220`),
`pantograph_scheduler_v2_evaluation.md`
(`libfile_8b6d205bd260819187ef9fbac029a243`), and
`designing-compatible-inference-backends.md`
(`libfile_fe49f57b568481918a285086f54f138d`). These are research sources, not
current source acceptance. Their scheduler ranking/search and additional native
task proposals need contracts beyond this slice. This milestone supplies a
small usable task-specific control through the established descriptor, scheduler
and inference owners, without inventing scheduling evidence or adding a backend.

The concrete gaps were: text descriptors exposed only `prompt`; the runtime host
rejected all other inputs and always omitted generation options; scheduler-owned
runs did not materialize number-input sources. All three are addressed together.
The descriptor declares an optional unsigned integer, numeric range and unit
step, without a new default. Existing descriptor resolution/fingerprinting and
patch ownership apply when refreshing a text node's authored interface. The
source template adds the `integer` variant to the existing non-exhaustive enum;
existing text and boolean representations are unchanged. Scheduler-owned numeric
source inputs are integer-only in this milestone, with fractional/string inputs
rejected rather than coerced. Generic node-engine numeric execution is unchanged.

## Acceptance criteria and actual evidence

Native checks marked unexecuted below are deferred follow-ups, not delivery
blockers. Their execution owner is the coordinator in a suitable native or CI
environment.

| Claim | Evidence target | Current status |
| --- | --- | --- |
| Descriptor exposes the optional typed port, range, step and no default; image descriptor is unchanged | Focused descriptor contract check | Test added and compiler-checked; not executed locally. |
| Number-input graph lowers to an integer source and retains its value through runtime-host input mapping | In-process materialization integration check | Tests added and compiler-checked; not executed locally. |
| Scheduler completes the source task with `number-input` intent and retains the integer result | Scheduler state/store integration check | Test added and compiler-checked; not executed locally. |
| Runtime host accepts integer boundaries, rejects invalid values and leaves omitted options absent | Focused projection checks | Tests added and compiler-checked; not executed locally. |
| Supplied limit reaches the gateway's selected-text backend request as `max_tokens: 128` | Host/gateway integration with a recording backend | Test updated and compiler-checked; not executed locally; its generated text is a stub. |
| Existing text worker command mapping remains compatible | Existing PyTorch-enabled inference aggregate | 682 passed, none ignored; Python fixtures/stubs do not prove model inference. |
| Existing frontend and command contracts remain compatible | Frontend aggregate | 659 passed, none skipped. |
| Supporting static and repository gates | PyTorch-enabled consumer Clippy, formatting, whitespace, critical/accessibility/traceability checker tests and production dependency audit | Passed; 27 accessibility checker tests, 28 traceability checker tests, zero production vulnerabilities. Native execution is deferred. |

Commands and logs:

```bash
source /workspace/pantograph-tools/activate.sh
export CARGO_BUILD_JOBS=1
cargo test --locked -p pantograph-workflow-service --lib external_input_materialization
ORT_SKIP_DOWNLOAD=1 cargo check --locked -p pantograph-embedded-runtime -p pantograph-workflow-service --tests
ORT_SKIP_DOWNLOAD=1 cargo test --locked -p pantograph-workflow-service --lib external_input_materialization
ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-workflow-service -p pantograph-embedded-runtime --features backend-pytorch --lib --tests
cargo test --locked -p inference --features backend-pytorch --lib
npm run test:frontend
```

The ordinary workflow test attempt stopped before tests in `ort-sys`: the
1.24.2 download from `cdn.pyke.io` received proxy CONNECT 403. The no-download
attempt compiled the changed test sources, then failed native linkage with
`OrtGetApiBase` undefined and ort-sys's missing-library diagnostic. Zero workflow
tests executed in either attempt. The no-download consumer check passed, with
the existing `SelectedTextLoad` dead-field warning; it is compiler evidence only.
An installed ONNX Runtime 1.21.0 was observed but not substituted for the required
1.24.2. No network policy, permissions, credentials or dependency features were
changed to work around this environment limitation.

The inference aggregate initially built but its executable could not find
`libpython3.12.so.1.0` (exit 127, zero tests). The same built test executable then
ran with `LD_LIBRARY_PATH` pointing to the already installed toolchain's Python
library directory, yielding the 682 passing tests above. Local logs are
`/workspace/pantograph-cache/token-limit-*.log`; the executed binary was
`target/debug/deps/inference-14f4aaec63a8eb8a` in the shared target directory.
The PyTorch-enabled consumer Clippy check emitted no warnings; warnings were
not denied in its invocation. The native download and link failures are retained
in `token-limit-workflow-focused.log` and `token-limit-workflow-no-download.log`.

No GUI, desktop IPC, real text-model loading/inference, GPU, or text-to-image
workflow was executed for this feature. The real frozen CPU BERT checks on the
separate PR49 repair do not qualify token-limited text generation.

## Handoff and next action

Retain the feature branch/worktree as a protected candidate. The cloud lane owns
this feature; the coordinator owns independent review, native qualification,
integration and retirement. Do not advance PR49's reviewed head or merge this
candidate. Continue coordinated review and feature work. Deferred qualification:
run the changed workflow-service and embedded-runtime tests in an environment
that can link the required ONNX Runtime, then exercise the descriptor-to-worker
path with a real selected text model. Preserve those results as separate native
and user-workflow evidence when executed.
