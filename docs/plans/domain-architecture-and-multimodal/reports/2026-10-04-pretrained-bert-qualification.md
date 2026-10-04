# Pretrained BERT qualification: blocked before acquisition

Qualification branch: `qualification/pretrained-bert-evidence`, based on frozen
embedding-output candidate `1f7ad59b93a1651064c92e2ab52df3ad9e435620`.
Consumer repair `129eb57ec9ca94e350899b53c9bede298148fd05` remains separate and
unchanged. This branch contains evidence and an acceptance plan only.

## Candidate, identity and bounded acquisition

Candidate: `sentence-transformers/all-MiniLM-L6-v2`, published by the Sentence
Transformers organization. The pinned [model card](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/blob/1110a243fdf4706b3f48f1d95db1a4f5529b4d41/README.md)
declares Apache-2.0. The [immutable repository revision](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/commit/1110a243fdf4706b3f48f1d95db1a4f5529b4d41)
is `1110a243fdf4706b3f48f1d95db1a4f5529b4d41`.

The [config](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/blob/1110a243fdf4706b3f48f1d95db1a4f5529b4d41/config.json)
declares a six-layer absolute-position `BertModel`, hidden width 384, 12 attention
heads, intermediate width 1536, vocabulary 30522 and position capacity 512.
The [modules](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/blob/1110a243fdf4706b3f48f1d95db1a4f5529b4d41/modules.json)
declare Transformer → Pooling → Normalize. The pooling configuration selects
masked mean; the sentence recipe limits input to 256 tokens.

The [pinned safetensors Git LFS pointer](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/raw/1110a243fdf4706b3f48f1d95db1a4f5529b4d41/model.safetensors)
records **90,868,376 bytes** and SHA-256
`53aa51172d142c89d9012cce15ae4d6cc0ca6895895114379cacb4fab128d9db`.
The bounded selection below is approximately **91.6 MB / 87.3 MiB**, using the
publisher's rounded tokenizer/vocabulary/card sizes. The exact weight size and
hash are pointer metadata, not a locally verified download. This estimate was
reported before any attempted weight acquisition.

Select only these pinned paths: `model.safetensors`, `config.json`,
`modules.json`, `1_Pooling/config.json`, `sentence_bert_config.json`,
`config_sentence_transformers.json`, `tokenizer.json`, `tokenizer_config.json`,
`special_tokens_map.json`, `vocab.txt`, `README.md`. Do not acquire the roughly
977 MB multi-format repository, pickle weights, training scripts, ONNX or
OpenVINO exports. No remote model code is required by this BERT recipe.

## Current blockers

1. Workspace requests to `https://huggingface.co/api/models/...` fail at the
   proxy tunnel with `403 Forbidden`. No pretrained weights were downloaded;
   accessible workspace caches contain only existing synthetic fixtures. Public
   metadata was inspected using the web tool. Network settings, credentials,
   proxy routes and permissions were unchanged; denied workspace requests were
   not retried through another acquisition route.
2. The raw published package does **not** satisfy the frozen loader's strict
   serialized recipe profile. Its [pinned pooling config](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/blob/1110a243fdf4706b3f48f1d95db1a4f5529b4d41/1_Pooling/config.json)
   omits `include_prompt`. In `crates/inference/src/backend/candle_embedding.rs`,
   the loader requires that JSON field to equal `true`; an absent field is null
   and rejects the package. The [Sentence Transformers v5.1.0 reference](https://raw.githubusercontent.com/UKPLab/sentence-transformers/v5.1.0/sentence_transformers/models/Pooling.py)
   defaults this option to true. Thus the declared recipe matches semantically,
   but unchanged raw-package compatibility is not established.

The [sentence configuration](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/blob/1110a243fdf4706b3f48f1d95db1a4f5529b4d41/sentence_bert_config.json)
has `do_lower_case=false`; the pinned tokenizer configuration has
`do_lower_case=true`. The reference [Transformer implementation](https://raw.githubusercontent.com/UKPLab/sentence-transformers/v5.1.0/sentence_transformers/models/Transformer.py)
treats its sentence flag as optional preprocessing, separate from tokenizer
normalization, and supplies padding/truncation at invocation. The frozen loader
instead checks sentence lowercasing against `tokenizer.json` and requires saved
padding/truncation settings. The actual tokenizer JSON could not be inspected
through the available web tool, so these are additional compatibility risks to
verify, not claimed executed failures. Advertised I64/F32 checkpoint metadata
also requires inspecting the actual safetensors header against the loader's
all-F32-tensors guard before acceptance.

No model files were rewritten, no defaults guessed into a fabricated package,
and no frozen backend or output code was modified to make this candidate pass.

## Bounded CPU acceptance plan

The existing reference environment is available: Torch `2.14.1+cpu`,
Transformers `4.53.3`, tokenizers `0.21.4`, safetensors `0.8.0`. Record these exact
versions, the Candle pin `88ed7911de9e88196b1f55b199145d22647f415e`, original file
hashes and any explicitly reviewed recipe adaptation separately. Fetch only at
the immutable revision in an already authorized environment, then verify the
weight's exact byte count and SHA-256 plus each configuration/file's provenance
before loading. Inspect the safetensors header and tokenizer invocation policy.

Use CPU F32, one Torch thread, deterministic algorithms, eval/inference mode,
local-only trusted built-in `BertModel` and safetensors. Execute the same exact
texts and token IDs in Torch and the existing selected-Candle gateway. Apply
attention-mask mean pooling including special tokens, then L2 normalization
with epsilon `1e-12`; right batch-longest padding and longest-first truncation
must be resolved from verified recipe/reference evidence, not silently patched.

For a small batch, require 384 finite values per text, unit L2 norms within
`1e-4`, preserved ordering and token counts. Compare vectors against the pinned
Torch reference using initially planned `atol=1e-5`, `rtol=1e-4`, and report
actual maximum absolute error and padding/single-batch differences. Record
failure honestly if tolerances do not hold; no pass is currently asserted.

Illustrative semantic smoke texts are in the adjacent JSON plan: two paraphrase
pairs and one unrelated sentence. Report cosine values and whether each pair
ranks above its unrelated comparison in both implementations. These are a few
English examples, not a benchmark, quality guarantee or clinical validation.

If host/workflow execution remains blocked by ORT or pending composition,
separately report genuine Candle/gateway execution and leave host, Pumas IPC,
scheduler admission, GUI and GPU evidence unexecuted. Do not reduce dependency
features or bypass network access to claim integration qualification.

## Executed status

Model-card/license declaration, immutable repository metadata, weight pointer,
small configuration metadata and frozen source gates were inspected. JSON-plan
parsing, Rust format, critical lint, whitespace and decision traceability passed.
**No pretrained Torch forward, Candle/gateway forward, dimensions/norms,
semantic similarities, host/workflow, real IPC, GUI or GPU acceptance ran.**
The reusable plan records all runtime gates as unexecuted. Existing synthetic
inference evidence from the previous slice is not pretrained qualification.
