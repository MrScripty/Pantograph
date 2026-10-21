# CPU text dependency handoff verification

The dependency repair passes T1/T2. Trained-model text generation (T3) remains
blocked. This is CPU contract/integration evidence, not GPU, browser text,
native desktop, release or model-quality qualification.

## Change and scope

PR72's graph Resolve path delegated only to the native Candle embedding profile.
A text request therefore reached readiness without concrete requirements. The
new profile supports explicit `text_generation` / `pytorch` / `cpu` for the
current host platform, ordinary local Transformers artifacts and no custom code.
Pumas must supply valid current package facts and prove there are no additional
model dependency declarations. Declared pins are refused, never approximated by
presence checks. Request proof, identity, bindings and environment remain bound.

Resolve returns `Resolved` with seven distribution-presence requirements:
NumPy, SoundFile, Torch, Transformers, Accelerate, Safetensors and Tokenizers.
The list is owned by inference and shared by payload construction and observation.
It does not certify versions, imports, ABI compatibility or successful generation.
The existing graph service owns payload insertion and freshness; inventory Check
owns observations; scheduler admission remains with its existing owner.

The text inventory reads `importlib.metadata` through the same linked PyO3
interpreter as execution. Host executable overrides cannot supply another
Python environment's package evidence. Only `PackageNotFoundError` means absent;
other metadata/join failures remain unavailable. Altered owned-profile payloads
fail validation without falling through to the host-process probe. Default and
explicit Check results retain `python:pytorch-embedded` with manifest
`pytorch.embedded.cpu.text.v1`.

No scheduler source, serialized contract, Cargo manifest, lockfile or standalone
probe changed. The external M2 worker owns the probe and subsequent generation
qualification. Its cross-thread address was unavailable to local collaboration;
this report and the PR provide the adapter handoff.

## Evidence

Base: `07b23d47b2cdfae5669bbda78515c9c728081031`. Exact tested source SHA-256
values and outcomes are in [source-and-results.json](source-and-results.json).
All source remained unchanged after these runs.

| Check | Result | Evidence |
| --- | --- | --- |
| Focused text requirements tests | 7 passed | [log](evidence/focused-tests.log.gz) |
| Inference library suite | 853 passed | [log](evidence/inference-tests.log.gz) |
| Embedded-runtime library suite | 619 passed; 3 ignored | [log](evidence/embedded-tests.log.gz) |
| Clippy, both owners/lib/tests, warnings denied | exit 0 | [log](evidence/clippy.log.gz) |
| Affected Rust formatting and diff whitespace | exit 0 | source/result record |

Focused tests cover exact profile rejection, identity/proof preservation, actual
Pumas declared-constraint refusal, custom-code and missing-runtime refusal,
public graph Resolve registry insertion without any Ready snapshot, actual linked
metadata routing with a process runner that panics if called, and complete
inventory snapshot environment identity for default/explicit Check.
The two inference tests cover missing-versus-failed metadata lookup and the real
linked metadata boundary. Synthetic GPT-2 files are inspected as package facts;
these tests never load their weights or claim useful generated text.

Independent reviewer `review_sol`, explicitly GPT-6.1 Sol / High, admitted the
revised design and reviewed the final sources/tests. Its one material finding
(default Check incorrectly labelling embedded observations as host Python) was
fixed with a regression. No source findings remain.

## Reproduction environment

Saved toolchain: Rust 1.92.0, Python 3.12.3, CPU-only development defaults,
`CARGO_BUILD_JOBS=2`, incremental compilation disabled. Tests used writable,
dedicated XDG directories rather than the read-only home configuration.

```bash
source /workspace/pantograph-tools/activate.sh
export LD_LIBRARY_PATH=/workspace/pantograph-tools/python/cpython-3.12.3-linux-x86_64-gnu/lib
export PYTHONPATH=/workspace/Pantograph/.venv/lib/python3.12/site-packages
export XDG_CONFIG_HOME=/tmp/pantograph-text-qualification/config
export XDG_DATA_HOME=/tmp/pantograph-text-qualification/data
export XDG_CACHE_HOME=/tmp/pantograph-text-qualification/cache
mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_CACHE_HOME"
timeout 600 cargo test --locked -p pantograph-embedded-runtime --features backend-pytorch,host-dependency-inventory --lib inference_text_dependency_requirements
timeout 600 cargo test --locked -p inference --features backend-pytorch,backend-candle --lib
timeout 600 cargo test --locked -p pantograph-embedded-runtime --features backend-pytorch,host-dependency-inventory --lib
timeout 600 cargo clippy --locked -p inference -p pantograph-embedded-runtime --features pantograph-embedded-runtime/backend-pytorch,pantograph-embedded-runtime/host-dependency-inventory --lib --tests -- -D warnings
```

The no-default baseline test configuration failed because existing tests call
`InferenceGateway::new`, gated by `backend-llamacpp`. The supported default
configuration above passes. The initial test executable also required the saved
libpython loader path. These setup failures are not changed-behavior evidence.

## Remaining qualification blockers and handoff

The initial probe worker selected the trained
[DistilGPT2 candidate](https://huggingface.co/distilbert/distilgpt2/blob/2290a62682d06624634c1f46a6ad5be0f47f38aa/README.md),
revision `2290a62682d06624634c1f46a6ad5be0f47f38aa`, with Apache-2.0 metadata
and an approximately 353 MB safetensors file. The allowed acquisition budget was
600 MiB total, with an explicit file allowlist, local-only loading and no remote
model code. That worker reported proxy HTTP 403 for metadata access, including
one approved read-only escalation. No weights were acquired and no repeated
bypass was attempted after the denial. Candidate metadata is not evidence of
model acquisition, licensed incorporation or generation.

The saved development venv also lacks the SoundFile distribution. This repair
performs no package installation. The new inventory correctly retains actual
missing-package observations rather than injecting Ready. The ignored native
qualification tests remain ignored and do not satisfy T3.

To close T3, M2 needs sanctioned model access or an operator-supplied local model,
verified artifact revision/license/resource bounds, satisfied worker prerequisites,
and actual text from the public hosted Resolve/Check/scheduler/runtime path.
Use `prompt`, `max_new_tokens` and `temperature` controls and output `text`.
Retain real output, model identity, diagnostics and cleanup evidence. Keep this
plan blocked until that observation exists.
