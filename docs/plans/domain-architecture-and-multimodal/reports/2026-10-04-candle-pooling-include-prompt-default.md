# Candle Pooling include_prompt default compatibility

Branch: `fix/candle-pooling-include-prompt-default`, based on frozen embedding
output `1f7ad59b93a1651064c92e2ab52df3ad9e435620`. This compatibility slice is
separate from accepted consumer `645889a969cc025f604001935da7d60770992b5f`,
embedding test repair `34bd2655c3c7f78b0d5b0d2a0ab318b7a5d4a914`, and their
planned composition. Pretrained evidence `2ef6359edca227a991245450df01480274fea3be`
remains unchanged and records the earlier frozen loader's rejection.

## Defined semantics and provenance

The pinned MiniLM [exporter metadata](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/raw/1110a243fdf4706b3f48f1d95db1a4f5529b4d41/config_sentence_transformers.json)
declares Sentence Transformers 2.0.0. At tag commit
`3ddd7a76257a2611d48a7f6ceba6e3906b520eb1`, its [exact Pooling class](https://raw.githubusercontent.com/UKPLab/sentence-transformers/3ddd7a76257a2611d48a7f6ceba6e3906b520eb1/sentence_transformers/models/Pooling.py)
has no prompt-exclusion parameter, omits it when saving configuration, and mean
pools all tokens enabled by the supplied attention mask. The absence is expected
for that exporter, not an unknown inferred recipe choice.

At 2.6.0 tag commit `a5f774998eaae056e0cbe42dc1d587a636115c3d`, the [same class](https://raw.githubusercontent.com/UKPLab/sentence-transformers/a5f774998eaae056e0cbe42dc1d587a636115c3d/sentence_transformers/models/Pooling.py)
has constructor default `include_prompt=True`, loads using `Pooling(**config)`,
and masks a prompt prefix only when exclusion and prompt length are provided.
Omission therefore retains inclusion. Prompt-prefix exclusion is a separate
concept from special-token exclusion. Tag identities were checked with read-only
`git ls-remote`; implementations were inspected at those immutable commits.

## Narrow change

The existing loader already recognizes only exact
`sentence_transformers.models.Transformer` → `Pooling` → `Normalize`, a CPU F32
absolute-position BERT encoder and declared masked mean pooling. Within that
profile, only a missing `include_prompt` field now receives the defined true
default. Explicit true continues to pass. Explicit false, null, strings, numbers
and objects continue to fail. Existing architecture, recipe, tokenizer, weight,
selected-identity, lifecycle and audit gates are unchanged; no prompt API or
additional architecture is implemented. Original pooling bytes remain in the
load snapshot; omission is not patched into a rewritten model package.

## Executed evidence

The new real-forward regression first failed on the unchanged loader at the
omitted field (exit 101). After the guard repair, the actual inference library
suite passed: **450 tests, zero failed or ignored**, including the new case and
existing native BERT reference, cancellation and recovery cases:

```sh
CARGO_BUILD_JOBS=1 cargo test --locked --offline -p inference --features backend-candle --lib -- --nocapture
CARGO_BUILD_JOBS=1 cargo clippy --locked --offline -p inference --features backend-candle --all-targets
```

For both untrained width-8 and width-12 fixtures, real locked Candle CPU forwards
with omission exactly matched explicit true; maximum absolute error against the
existing independent Transformers vectors was `5.9604645e-8`. The regression
checks dimensions, token counts, finite normalized vectors, and preservation of
the resident model after all rejected values. All 14 original checkpoint,
tokenizer, config and golden-file hashes still match the reference manifest;
no fixtures or reference vectors were regenerated. Clippy retains only the
already-recorded inference `SelectedTextLoad` unused-field warning.

Rust formatting, critical lint, whitespace and staged traceability are checked
before committing. Receipts are retained in the handoff archive.

## Limits

This establishes one supported configuration default, not complete MiniLM
package admission. Existing tokenizer/preprocessing and tensor-dtype concerns
remain unverified. No pretrained acquisition was retried; proxy-403 access limits
and network policy were unchanged. No pretrained Torch/Candle inference,
semantic retrieval, Pumas IPC, native host/workflow, GUI or GPU qualification ran
in this slice. Existing synthetic forwards do not establish pretrained quality.
