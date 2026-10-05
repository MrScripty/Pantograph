# Real CPU sampler hosted qualification

## Scope and decision

Extend the existing `rust-tests` quality-gates job with an isolated CPU Python
environment, the project's existing `transformers>=4.52,<4.54` constraint, and
Torch from the [official CPU wheel index](https://download.pytorch.org/whl/cpu).
The [PyTorch installation guide](https://pytorch.org/get-started/locally/) and
[Transformers installation guide](https://huggingface.co/docs/transformers/installation)
describe these supported package sources. Production requirements and frozen
source milestones remain unchanged. This adds no workflow or test framework.

The aggregate Cargo tests use worker fixtures and do not execute the real
autoregressive sampler. The dedicated Python command runs the real tensor tests:

```sh
python -m unittest discover -s crates/inference/torch/tests -v
```

CI checks that at least four tests are discovered, rejects a CUDA Torch build,
disables model Hub access, preserves the command's failure status through the
pipeline, and uploads its log even when tests fail. Dependency provisioning
failures fail the job; there is no fallback to fixture-only Cargo evidence.

## Evidence and qualification limits

Existing local real CPU evidence: four Python tests passed with Torch
`2.14.1+cpu` and Transformers `4.53.3`; the same tests detect eight errors against
the old sampler. These were preserved without rerunning expensive checks in
Library packet `libfile_cdf0c1d747c0819183dfb308ea0875d0`, along with the
686 inference and 11 node-engine logs and source/log SHA-256 inventory.

The follow-up validates YAML structure and the exact discovery/preflight and
offline unittest commands using installed dependencies. It does not claim fresh
dependency provisioning or hosted execution. Parent coordinates publication
and exact-head hosted qualification. Native/ONNX qualification remains pending.

See [sampler source evidence](2026-10-05-top-k-vocabulary-sampling.md) and
[plan](../plan.md).
