# Graph-authored Stable Diffusion scheduler choices

Status: implemented and qualified for parent review on the separate
`feat/workflow-image-scheduler` branch.
Source checkpoint `69e7a3b95342f19866f6baec36296f86e153dcdb`, tree
`62a2633c1c1a88bcaed2007f2bcbda2308ffd095`, parent
`3fe884725329f0020baff844943a505baf6c10cf`. This completes the denoising-scheduler
control deferred by the basic image-controls slice, after guidance and image count.

The descriptor exposes optional `denoising_scheduler` with closed choices `ddim`
and `euler`, without a host default. Omission preserves the loaded bundle's
scheduler. Stable Diffusion planning and compatibility diagnostics accept those
choices; unsupported scheduler IDs, class names and paths fail before model or
media effects. Compatible batches require one shared choice; differing choices
remain incompatible. Both solo and grouped worker execution use the same helper.

The helper copies the resident pipeline for a selected scheduler and shares its
loaded components. It constructs only the corresponding fixed built-in Diffusers
scheduler from the resident scheduler configuration. Request-local scheduler and
pipeline call state leave the resident scheduler/configuration unchanged, including
when execution fails. No package metadata or request selects arbitrary code.
Restricted loading, dependency pins, full peak admission and custody are unchanged.

## Interface reconciliation

Parent froze correctness branch `13d24e898abd4175eb799ed310d1d275537880e9` for
independent review and integration. Its generation, task-pool accounting, config
save and external-connect repairs do not change this feature's public request or
descriptor signatures. Runtime generation IDs are opaque to the scheduler control.
No interface migration is necessary and no correctness commits were mixed into
feature publication. Parent coordinates ordinary integration and PR/review actions.

## Qualification and cache provenance

A combined run initially reused shared-target correctness-branch binaries: it
included those branch-specific tests while omitting the new scheduler regressions.
That run and its shared-target service/Clippy evidence are excluded from qualification.
The isolated target `/workspace/pantograph-cache/image-scheduler-target` reused only
external dependency artifacts after cleaning every workspace package from its
copied cache. It rebuilt the actual feature sources. The resulting logs include
the new worker-envelope and public-session scheduler test names and exclude the
correctness-only generation regressions.

| Scope | Actual result |
| --- | --- |
| Full isolated inference package | 808 passed; six optional native/doctest cases ignored; no failures. |
| Full isolated embedded package | 511 passed; no failures. Both choices execute through the actual public session, Pumas host adapter and controlled backend, retaining three image outputs. |
| Interface/runtime-host contracts | 75 passed; no failures. Optional enum schema and host mapping are exercised. |
| Full isolated workflow-service library | 916 passed; no failures. |
| Explicit native CPU checks | Three passed on installed Diffusers 0.37.0 and Torch 2.14.1+cpu in the isolated target; no pretrained models. |
| Strict isolated Clippy | Five affected packages/all targets/mixed backends, warnings denied: passed. |
| Gates | Formatting, whitespace, critical, accessibility (27 tests), source/documentation traceability and all nine ONNX no-build-download feature/target graphs passed. |

Native scheduler evidence executes the actual worker helper with a small random
UNet and real DDIM/Euler schedulers, two prompts and two outputs per prompt.
The intended scheduler takes actual denoising steps; results equal explicit
native pipeline oracles and the two choices produce different latent tensors.
Resident scheduler/configuration identity survives both choices, omitted choice,
unsupported input and execution failure. Complementary guidance/count checks
exercise actual CPU arithmetic and advancing seed streams. No pretrained weights
are downloaded, no image-quality acceptance is implied, and production model
loading, native GPU, GTK/WebKit and cross-platform execution remain unqualified.

Evidence is under `/workspace/qualification-evidence/image-scheduler-*`.
`image-scheduler-qualification.json` records final source/tip identities, verified
remote refs and executed results. The historical resumption manifest retains the
invalid shared-cache evidence and private-target preparation. No service-limit
failure occurred and no running test was interrupted. The final checkpoint changes
documentation only; qualified source remains exactly `69e7a3b`.
