# Candle embedding workflow output boundary

Feature branch: `feature/candle-workflow-embedding-output`, directly descended
from accepted Candle repair `8ed84956b72b383f78ae5a0121cff94f589db915`.
The frozen consumer `a5ed074d5e44db13f47ff6f7cd5059e234c183ff` and its separate
diagnostic repair are not composed into this branch. Full PR44 ancestry remains.

## Existing behavior and missing feature

The accepted gateway already executes selected CPU F32 BERT weights with their
declared tokenizer, pooling and normalization recipe and returns typed vectors,
indices, token counts and usage. The legacy node convention is `text` input and
`embedding` output. Canonical inference ports remain model/task descriptors;
the static bootstrap node is unchanged.

The runtime-host port only had explicit text execution and otherwise attempted
image projection. Its output DTO and the scheduler task-result DTO could not
carry vectors or structured usage. These were the missing output boundaries.

## Bounded change

- Route the `embedding` task through the existing selected-embedding gateway,
  with one nonblank `text` input of at most 1024 bytes and an explicit scheduler
  selection of `candle` / `candle.cpu` / `cpu`. Graph trait overrides are rejected.
- Preserve the selected model reference and complete package facts. Physical
  paths still come from the separately resolved Pumas load target; the gateway
  repeats the identity and artifact checks and enforces the declared recipe.
- Add the additive `json` output value to host and scheduler result contracts,
  bounded by the same exported 64 KiB limit. Scalar text limits are unchanged.
- Return `embedding` as a finite array (1–4096 values), `metadata` with the exact
  selected model reference, vector width, index, token count and CPU selection,
  plus structured `usage`. Validate the completed host response before returning
  it; invalid or oversized output produces failure with no vectors.
- Preserve structured values through host-to-task mapping, task persistence and
  requested workflow output projection. A caller can request the inference
  node's `embedding` output. Structured values are not new runtime input types;
  downstream vector-processing adapters and batching are outside this slice.

No new backend architecture, model lookup authority, admission ledger, dependency
feature reduction, audit gate change or GUI flow was added.

## Executed evidence

| Check | Evidence and limit |
|---|---|
| `cargo test --locked --offline -p pantograph-runtime-host-contracts --lib` | 42 passed, including structured vector round trip and byte-limit rejection |
| `cargo test --locked --offline -p inference --features backend-candle --lib selected_embedding` | 2 passed with actual CPU forwards, model replacement and invalid-handoff/cancellation rejection |
| Independent production boundary harness | 7 passed; actual widths 8 and 12 survive host validation, task mapping, serialization and raw workflow output conversion; maximum absolute Torch-reference error `5.960464477539063e-8` for each width |
| `ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo check --locked --offline -p pantograph-embedded-runtime --tests` | Passed with the unchanged default/full dependency profile; compiler evidence only |
| `ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo clippy --locked --offline -p pantograph-embedded-runtime --all-targets` | Passed; compiler-only evidence, with the existing selected-text unused-fields warning |
| Rust format, critical lint, whitespace and decision traceability | Passed |

The independent harness imports the production embedding projector, output
adapter, runtime-host contracts, scheduler result contract and host mapping.
It extracts the exact model-reference and raw output conversion functions from
production source, and reuses the native fixture generator with fixture-location
paths replaced. It checks identity/recipe failures, blank input, GPU selection,
empty/nonfinite/oversized vectors, plus existing host mapping tests. It uses
locked Candle commit `88ed7911de9e88196b1f55b199145d22647f415e` and the committed
synthetic untrained BERT fixtures, with no downloads or mocked inference.

The harness does **not** execute the full `EmbeddedRuntimeHostExecutionPort`,
scheduler admission, complete `workflow_run`, Pumas IPC or ORT. The committed
native host tests use fixture package/target resolvers and actual Candle
inference; their source is compiler checked here, but they are not linked or
executed here. GUI, GPU, pretrained semantics and real Pumas-to-Candle workflow
acceptance remain unexecuted. No repeated CDN attempt was made.

## Native acceptance commands

Use the existing authorized native runtime with the unchanged feature profile:

```sh
cargo test --locked -p pantograph-embedded-runtime --lib runtime_host_embedding_execution
cargo test --locked -p pantograph-workflow-service --lib task_result_output_projection
cargo test --locked -p pantograph-embedded-runtime --lib
```

The first gate requires actual host-returned Candle vectors against the Torch
goldens at widths 8 and 12, exact identity and usage, no media-sink calls, and
zero outputs after identity/recipe/cancellation failures. The second requires
host structured output to reach a requested workflow output and rejects an
oversized scheduler result. The final gate covers existing host behavior.
Full workflow and real IPC qualification remain separate from these fixtures.
