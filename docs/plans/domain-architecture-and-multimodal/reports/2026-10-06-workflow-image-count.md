# Graph-authored multiple-image generation

Status: implemented for parent review on `feat/workflow-image-count`, descended
from frozen guidance milestone `14dfd4a7`. This completes another control
explicitly deferred by the [basic image-controls slice](2026-10-04-workflow-image-controls.md).
Guidance and the PR56 integration branch remain frozen.

Source: `994463e144f3e30e8be332aaf122d06abbb087d9`; tree
`c512fc1508ed5cf9117ebe53a1cc823215fae63b`; single parent
`14dfd4a7e1d48a42dab9f08bae54224fea835a28`.

The image descriptor exposes optional `num_images_per_prompt`, an integer from
one through the existing runtime-host limit of 64 output values per member.
The host enforces that existing limit before backend effects. Omission preserves
one image. Compatible PyTorch batches now forward a shared positive count,
validate the total returned image count and partition images into their original
members in prompt order. Different member counts remain incompatible. Each seeded
member uses one advancing generator stream, matching solo generation, rather
than restarting its seed for each image.

The existing workflow response requires one binding per requested output target.
Output projection now preserves all values produced at that target: singleton
`image` remains an artifact-reference object; multiple images produce an ordered
array of path-free artifact references. The actual public session regression
reads every retained image and proves unique IDs for counts one, three and 64,
while preserving load-proof, lifecycle and freshness assertions. This qualifies
terminal workflow outputs; downstream multi-image consumers and desktop array
presentation are not newly qualified.

## Executed evidence

Rust checks used `--locked --offline`; mixed inference/embedded checks used
`--no-default-features --features backend-llamacpp,backend-pytorch`.

| Scope | Result |
| --- | --- |
| Full inference package | 805 passed, five optional tests ignored, zero failures. |
| Full embedded runtime package | 509 passed, zero failures. Controlled Pumas/backend public sessions and invalid-count host rejection run here. |
| Full workflow-service library | 916 passed, zero failures. |
| Interface and runtime-host contracts | 74 passed, zero failures. |
| Explicit native CPU executable | Two passed on actual Torch 2.14.1+cpu with both Diffusers 0.39.0 and production-loader-admitted 0.37.0. Guidance runs six cases; count runs one and three images across two prompts. |
| Strict Clippy | Five affected packages, all targets, mixed backends, `-D warnings`: passed. |
| Repository gates | Formatting, whitespace, critical/accessibility, staged/range traceability and nine ONNX no-build-download dependency graphs passed. |

The native count test executes the actual worker's batch-kwargs functions with
real Torch generators and a real small random UNet/Diffusers pipeline. Grouped
outputs match separately seeded solo runs; images within each prompt differ.
Controlled worker-envelope tests separately prove ordered response partitioning
using marked images and generator order. These complementary checks use no
pretrained weights and do not qualify the production model loader or image
quality. Diffusers 0.37.0 and its compatible official Hub dependency were installed
in a separate qualification environment; repository dependency pins are unchanged.

Logs: `/workspace/qualification-evidence/image-count-*.log`; exact final
commit/tree/parent and verified remote refs are in `image-count-qualification.json`.
The documentation-only final checkpoint has identical source to the tested
commit. Registry accounting, full peak admission, custody, unknown-fact semantics,
Pumas pin, CI and no-build-download policy are unchanged. No PR/review/merge
actions. Physical GPU, pretrained-model performance/quality, GTK/WebKit and native
cross-platform execution remain unqualified. The external standards path remains
unavailable as recorded by the preceding guidance milestone.
