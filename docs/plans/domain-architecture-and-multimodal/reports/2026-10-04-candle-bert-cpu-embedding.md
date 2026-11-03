# Candle CPU F32 BERT embedding candidate

The bounded Candle/backend/gateway slice now executes actual native BERT
forwards. It is prepared for the parent's independent source review; it has
not been merged, published as a branch, or submitted for external review.

## Source and scope

- Branch: `feature/candle-bert-cpu-embeddings`.
- Verified base: `11b046fa23da5715d69e33e98c41cb9aad792f9e` (accepted PR48 merge).
- Original PR44 head `ca9edde6850cd58ade0b7e534bb4f58e704c2c64` remains an ancestor.
- Locked maintained Candle: `88ed7911de9e88196b1f55b199145d22647f415e`.
- The accepted owner/client + CI branch remains unchanged at
  `1c0d86dc6275ac2be466c3f918d2fcd35be99cb6`.
- Pumas consumer/pin, host graph routing, artifact sinks, audit workflows,
  traceability policy, and external review partitions are unchanged.

The executable profile requires CPU F32 `BertModel`, absolute positions,
explicit Transformer -> masked mean Pooling -> Normalize modules, matching
tokenizer lowercasing, right longest-batch padding, and matching declared
right truncation. Pooling includes all unmasked tokens, including special
tokens, followed by L2 normalization with epsilon `1e-12`.

Unsupported architectures, decoder/cross-attention configurations, dtypes,
recipes, vocabulary/dimension mismatches, missing files, and escaping component
paths fail clearly before publishing a newly ready model. Full BERT loading
and forward use the existing locked `candle-transformers` implementation.
The previously unreachable Candle HTTP embedding adapter is superseded by
native inference; other backends and the staged resource probe are preserved.
Legacy unselected Candle startup still rejects: the admitted entry point is
`execute_selected_embedding_with_cancellation` with resolved package facts,
the separately approved target, and an explicit scheduler selection.

Request/package/target/scheduler model, revision, artifact identity, task,
contract, runtime, and CPU device must agree. The loaded instance retains the
selected target, projected source plan, parsed config/tokenizer/recipe bytes,
and actual model tensor snapshot. This does not independently verify an
optional producer content fingerprint or establish a new package authority.

The existing gateway exclusively owns backend residency. Blocking CPU jobs
retain their completion in that backend; cleanup, stop, and replacement await
actual worker termination. A running forward is not preemptible. Cancellation
is checked at native stage boundaries, and caller loss cannot discard the
backend's retained join. The controlled worker lifecycle test exercises this
ownership separately from the actual model-forward tests.

## Validation

Commands used the environment's existing tooling/cache, locked dependencies,
offline Cargo, and regular execution. No credentials, permissions, or network
settings were changed.

| Check | Observed result |
| --- | --- |
| Independent reference generator | Actual Transformers CPU F32 forwards for widths 8 and 12, three-item batch, three singles, and two-item truncated batch; no model downloads |
| Focused native Candle tests | 22 passed; max absolute error against original golden vectors `5.9604645e-8` for both widths; `atol=1e-5`, `rtol=1e-4` |
| Candle-enabled inference unit suite | 446 passed, including real gateway execution/replacement and eight invalid identity/device handoffs |
| Candle-enabled inference aggregate, `--no-fail-fast` | 516 passed, one pre-existing Pumas fixture decode failure, two ignored doc examples |
| Untouched base reproduction | Same Pumas fixture test fails at exact base `11b046fa…` |
| Default inference unit suite | 421 passed |
| Default workflow-nodes library suite | 168 passed |
| Candle-enabled inference Clippy, all targets | Completed; only existing selected-text dead-code warning, no warning from the new embedding code |
| Formatting, critical anti-pattern, staged/range traceability | Passed; gates preserved |

The existing failing aggregate test is
`pumas_artifact_load_target_decodes_existing_pumas_wire_shape`; its fixture
contains `model_ref_contract_version`, rejected by the current `PumasModelRef`
decoder. This reproduces in a separate detached worktree on the untouched base.
It remains for coordinated handling in the reserved consumer lane and is not
reported as a green aggregate run.

Fixture versions, exact file SHA-256 values, generator, and recipe provenance
are committed under `crates/inference/tests/fixtures/candle_bert`. Config,
tokenizer, weights, and original numerical vectors preserve their independent
preparation identities; additional truncation vectors use those same weights.

## Qualification limits and handoff

Actual native/model-dependent numerical inference **was executed** with the
two untrained local encoders. Pretrained semantic/retrieval qualification,
live Pumas-issued embedding artifact handoff, GPU inference, GUI execution,
and an end-to-end host embedding graph **were not executed**. Existing cloud
CDN denial and GUI prerequisites are unchanged; no access workaround was tried.

The parent owns review/integration, the reserved producer/consumer dependency
work, and pretrained/native host qualification. This slice introduces no
additional inference framework, scheduler ledger, model resolver, or semantic
quality claim. Lean proof work remains separately scoped and was not executed.
