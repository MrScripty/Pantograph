# Workflow basic image generation controls

Status: implemented for coordinated review. Native execution qualification is
owner-deferred because the Codex cloud ONNX download/link limitation persists;
it does not block source delivery or continued feature work.

## Milestone and source

The backend-owned image descriptor previously advertised only `prompt`, although
the runtime host already accepts `negative_prompt`, `width`, `height`,
`num_inference_steps` and `seed`. The descriptor now exposes those five optional
controls. Users can connect string/integer sources through the existing graph
authoring and PR50 scheduler source materialization. Omission preserves backend
defaults. This is a small usable extension toward DA-03's dependent text-to-image
workflow, not evidence that required-real DA-03 acceptance has been completed.

- Branch: `feat/workflow-image-controls`.
- Base and single parent: `d8c636f9239b46d24ed0cafb584329841cc826fa`.
- Base tree: `0103e0f6ac0eef1b967eebcdc865e3c3f58fa745`.
- Pumas pin: `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`, unchanged.
- Standards inspected: `366c1d90a24bbfb50973f62b155a5f3396c0f107`.
- Scope admitted by the coordinator's next scheduler/inference feature task;
  write set: image descriptor provider, existing image projection tests/constants,
  shared descriptor fixture/tests, and this report.
- Full PR50 stacked ancestry is preserved; PR49's independent repairs remain
  separate. Pooling candidate `495efea606620153fe2f0363b104e56518436a06` is excluded.

Width, height and steps declare unsigned scalar values with positive `u32`
bounds and unit steps. The common positive-u32 descriptor builder also retains
PR50's unchanged text token-limit descriptor. Seed declares the existing `u64`
wire type with no floating-point range; a rounded `u64::MAX` range would be
misleading. JavaScript number-input precision is still limited by its existing
numeric representation; this change does not promise GUI entry of every u64.
Package-specific dimension compatibility remains the selected image runtime's
responsibility. Guidance scale, multiple-image counts, denoising scheduler
choices and scheduler ranking are outside this admitted feature.

## Acceptance and executed evidence

| Claim / command | Result and boundary |
| --- | --- |
| `cargo test --locked -p pantograph-inference-interface-contracts` | 18 integration tests passed, including two new checks for six advertised ports, optionality, omitted defaults, wire round-trip, positive-u32 bounds and exact seed type. |
| Descriptor producer equals the shared controls fixture | New native producer test added and compiler-checked; not executed locally. |
| Advertised IDs and representative values reach the canonical image plan without loss, including negative prompt whitespace and `u64::MAX` seed | New runtime-host projection/planner test added and compiler-checked; not executed locally. Controlled package/path fixtures, no model loading. |
| Omitted controls remain absent through host projection | New native test added and compiler-checked; not executed locally. |
| `cargo test --locked -p inference --features backend-pytorch --lib` | 682 passed, none ignored. Includes existing image planner and worker command compatibility tests; controlled Python fixtures/stubs do not establish model inference. |
| `npm run test:frontend` | 659 passed, none skipped. |
| `ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-embedded-runtime --features backend-pytorch --lib --tests` | Compiler check passed; no native linkage or execution claimed. |
| `cargo fmt --all -- --check`, `git diff --check` | Passed. |
| Critical/accessibility gates and traceability checker tests | Passed; 27 accessibility checker tests and 28 traceability checker tests. |
| `npm audit --omit=dev --audit-level=high` | Zero production vulnerabilities. |

Commands used the existing toolchain activation script, regular speed, one Cargo
build job and the already installed Python 3.12 library directory in
`LD_LIBRARY_PATH`. Logs: `/workspace/pantograph-cache/image-controls-*.log`.
Staged and committed-range traceability gates remain required for this upload.

The coordinator reported hosted PR50 tests green, including its eight changed
tests; that establishes prior source materialization coverage as reported by the
parent, not hosted acceptance of this new image descriptor/projection change.
Native consumer tests for this feature remain deferred for coordinator execution
in a suitable native/CI environment. No repeat ONNX download/link attempt,
network-policy change, alternate runtime substitution or dependency workaround
was made. No GUI, desktop IPC, real image/text-model inference, GPU, or mixed
workflow execution was performed. The real CPU BERT checks on the separate PR49
repair do not qualify image generation.

Retain the feature branch/worktree as a protected candidate. The parent owns
review, native qualification, integration and eventual retirement. This upload
does not advance reviewed PR49/50 refs, create a PR, request external review, or
merge source.
