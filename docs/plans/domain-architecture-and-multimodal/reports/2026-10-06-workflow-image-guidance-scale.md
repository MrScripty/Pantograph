# Graph-authored image guidance scale

Status: implemented and locally qualified for parent review on
`feat/workflow-image-guidance-scale`, separate from frozen integration `ab7a1b4e`.
This completes the guidance control explicitly deferred by the
[basic image-controls slice](2026-10-04-workflow-image-controls.md), advancing
DA-03 without claiming desktop or pretrained-model acceptance.

Source checkpoint: `9486610d892b5516fb220b585264ea590047e6c8`; tree
`bf967143a4fc2c5e3fa26914d59f5c118a564177`; single parent
`ab7a1b4e8ba6afe76e42c3d53c3c46417ded1c98`. The final documentation successor
and remote verification are recorded in the local qualification manifest.

The image descriptor now advertises optional numeric `guidance_scale`. The real
host validates and forwards it to the existing canonical image plan and worker.
Omission stays `None`, preserving the selected pipeline's default. Existing
checked numeric conversion accepts finite f32 values, including zero and negative
values, and rejects overflow, underflow and authored precision loss. The declared
range uses the full finite f32 bounds; no tuning limit, clamp or default is
invented. Other image controls and unsupported-port rejection remain intact.

The existing Diffusers contract enables classifier-free guidance above one;
smaller values use conditional noise directly. See the
[official Stable Diffusion API](https://huggingface.co/docs/diffusers/api/pipelines/stable_diffusion/text2img).
Host semantics preserve the existing planner's finite-f32 contract rather than
restricting it to a recommended model-specific tuning range.

## Executed qualification

All Rust commands used `--locked --offline`. Mixed inference/embedded commands
used `--no-default-features --features backend-llamacpp,backend-pytorch`.
Source checkpoint `9486610d` was tested with Rust 1.92, Python 3.12.3, actual
Torch 2.14.1+cpu and Diffusers 0.39.0. Hub/Transformers offline settings were
enabled. Logs are `/workspace/qualification-evidence/image-guidance-final-*.log`.

| Executed scope | Result and evidence boundary |
| --- | --- |
| Full inference package | 803 passed, four ignored: native CUDA, optional native guidance and two documentation examples. No failed tests. |
| Full embedded runtime package | 507 passed. Production public workflow sessions connect a selection-input node to guidance and verify zero, one and 7.5 reach the backend exactly once, retaining existing media, lifecycle and ownership assertions. Pumas and image backend fixtures are controlled. |
| Inference-interface contracts | 22 passed, including seven image ports, optional/default semantics and finite-f32 descriptor bounds. |
| Workflow service `--lib runtime_host_task_input_mapping` | Eight passed; numeric source materialization preserves values for guidance and existing ports. |
| Explicit native `diffusers_guidance_native -- --ignored --nocapture` | One passed, six cases: omission, -1, zero, one, 2.5 and 7.5. A real tiny randomly initialized CPU UNet, Diffusers pipeline and scheduler execute one denoising step. Recorded noise exactly matches the classifier-free guidance oracle. No pretrained model weights are used. |
| Strict Clippy | Four affected packages, all targets, mixed backend features, `-D warnings`: passed. |
| Formatting, critical/accessibility gates | Passed; accessibility checker 27 tests passed. |
| ONNX no-build-download contract | All nine Linux/Windows/macOS dependency graphs passed. These are dependency checks, not native cross-platform executions. |

Host batch tests additionally cover omission, negative/zero/unit/default-scale
values and rejection before backend dispatch or media writes. Controlled Python
worker-envelope tests verify exact propagation and omission; the separate native
executable avoids those stubs and qualifies actual CPU guidance math. The native
test remains explicitly ignored in ordinary suites because its real Python
dependencies are optional.

Staged and final committed-range traceability, checker regressions and whitespace
checks accompany publication. Cargo manifests/lock, CI and runtime-registry
source are byte-for-byte unchanged from `ab7a1b4e`; full peak admission, custody,
resident uncertainty, source/generation protection and known-zero semantics are
preserved. Pumas remains pinned to `26a84e323cae566a46a8f76bef48fa1010aed48b`.
No dependency or build-download policy changes were made.

This does not qualify pretrained-image quality, exact-model timing, physical GPU
execution/capacity, unresolved native RAM backing, GTK/WebKit desktop execution,
ONNX runtime execution or other operating systems. The external standards path
named by the plan is unavailable here; current repository instructions and the
approved plan were inspected, without claiming a fresh external Core/Router read.
Parent retains review, PR publication and merge. Existing PR54/55 and the frozen
integration ref are not modified.
