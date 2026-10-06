# Chat integration with PR60/61 main

The tested merge source is `bd99b620d479a96c6c6d0bc093e7e56b8bdd8eac`, tree
`3779e55bf32613871944cc7f14b9b7bbcfee4c74`. Its ordinary merge parents are current
main `11ef074ad6fe42a4269a3b78edc931ae5d27533c` (after PR61) and independently
reviewed frozen chat source `0d7d573f36d8f25f3ae5c8adb25ab86c42e1dd8c`, tree
`4f8b991baa72b537d6cb1a96c7450c019aec1577`; that frozen source contains tested
implementation `2189046a6a0a418b54444308028b1af27323bf5a`.

Application code, fixtures and the original chat evidence remain byte-identical
to frozen chat. Relative to frozen chat, integration changes only the placement
of an identical stop contract test (keep main's placement) and retains both plan
records. The original 29-member evidence packet and both fixture manifests verify.
The subsequent candidate commit adds only this integration evidence directory.
`source.json`, `integration-review.json`, and `integration.patch.gz` bind this
packet to the combined source. Frozen refs and earlier histories are preserved;
no rebase, force push or unpublished native desktop repairs are used.

## Deciding results

- Affected five-package Rust suite: 2,486 tests plus one doctest pass; five native
  tests and two doctests are ignored by the default run.
- The explicit ignored saved/reopened CPU chat graph test passes separately:
  28 public graph runs and eight scoped sequential envelope members.
- Expanded eight-package run with `--no-fail-fast`: 3,170 passes including the
  doctest, one failure and 23 ignored. Scheduler and workflow-node suites pass.
- The one failure is
  `core_executor::tests::inference_tests::test_execute_llm_inference_non_streaming_uses_typed_gateway_boundary`
  at `crates/node-engine/src/core_executor/inference_tests.rs:488`: its mock seed
  diagnostic still expects `unsupported`, while the gateway reports
  `requires_backend_support`. The fixture and diagnostic implementation are
  identical to main. An isolated `git archive` of exact main reproduces the same
  failure with `node-engine/inference-nodes`; logs and main feature graph are kept.
  This aggregate gate remains failed; no unrelated fixture or product repair is made.
- 670 frontend tests, 78 Python worker tests, 55 tooling tests and 32 actual full
  worker CPU requests pass. Typecheck, production frontend build, affected strict
  Clippy, formatting, lint/accessibility/critical/explicit-range traceability,
  dependency checks and the scheduler-only source guardrail pass.

The all-target eight-package Cargo graph was inspected before builds: current
lockfile-pinned Pumas `26a84e32`, ORT `load-dynamic` and `disable-linking`, no
`download-binaries`. Cargo builds/tests/Clippy use locked offline resolution and
`ORT_SKIP_DOWNLOAD=1`; no ONNX/ORT artifact download is attempted. Missing
lockfile-pinned official Cargo sources were fetched for graph resolution.
SoundFile 0.14.0, CFFI 2.1.1 and pycparser 3.0 came from official PyPI; existing
Torch/Transformers and manifests/lockfiles are unchanged.

Initial failures remain as diagnostics: missing cached Pumas prevented the first
offline feature inspection; a scheduler command selected an unselected package
feature; read-only platform registry startup caused 31 initial runtime failures;
an in-flight wrapper edit interrupted initial exit-status bookkeeping. The fixed,
immutable runner directs XDG configuration/cache to workspace-only test locations.
Deciding reruns pass all affected tests and preserve the inherited seed failure.
Frozen historical logs/source.patch produce whitespace-check findings; product
source whitespace passes, and historical packet bytes are preserved.

## Scope and review

This is text-only single-prompt chat plus optional system prompt, retaining text
controls and exact canonical task/revision constraints. Two untrained 5,392-parameter
GPT-2 models and explicit standard tokenizer templates exercise actual CPU forwards.
Package/target/readiness/dispatch/graph discovery are controlled. Cancellation
coverage is precancellation; envelopes remain sequential. These checks establish
no production-model quality, GPU, usage-accuracy, post-start cancellation, desktop,
live Pumas discovery or arbitrary architecture/tokenizer/template-fallback claims.

The frozen source's independent review is the parent's supplied review status.
Fresh independent/external review remains parent-owned. The separate worker's
native desktop gate remains unqualified here; no failing desktop gate is reported
as passed. This draft candidate is not fully gate-clean due to the inherited seed
fixture. No merge or CodeRabbit review request is performed by this task.

`qualification.json` records exact check statuses/counts, `.command` files record
invocations, compressed logs preserve both deciding and initial results, and
`manifest.sha256.json` verifies every packet member. Commands use the saved official
toolchain and local CPU Python environment described in `environment.json`.
