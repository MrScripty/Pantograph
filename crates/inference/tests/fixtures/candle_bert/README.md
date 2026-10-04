# CPU F32 BERT numerical fixtures

These two small checkpoints are locally generated **untrained** BERT encoders,
not pretrained embedding models and not artifacts resolved by a live Pumas owner.
The package-facts and scheduler identities supplied by tests are synthetic.

`generate_reference.py` uses maintained Transformers `BertModel` with eager
attention, evaluation mode, CPU F32, deterministic operations, one CPU thread,
and seeds 179 / 183. It executes actual forwards independently of Candle.
Its explicit recipe is BERT -> attention-mask mean including special tokens ->
L2 normalization (`eps=1e-12`). It records token IDs, attention masks, type IDs,
batch/single vectors, and a truncated mixed-length batch. The two widths are 8
and 12; each encoder has one layer and a vocabulary of eight tokens.

`reference-manifest.json` records exact reference package versions and SHA-256
identities for the input and golden files. Regenerate only in an environment
matching those versions:

```sh
python3 crates/inference/tests/fixtures/candle_bert/generate_reference.py
```

Rust tests construct a temporary package and test actual locked Candle BERT
forwards at `atol=1e-5`, `rtol=1e-4`. They preserve exact order, dimensions,
nonpadding token counts, single/batch parity, and normalized finite vectors.
The config, tokenizer, weights, and original vectors retain the earlier
independent preparation's exact identities. Additional truncated vectors were
generated with the same encoder weights.

The standard Normalize module writes no serialized payload, so its declared
module directory may be absent from a Git checkout. Tests exercise both an
absent directory and an empty one. This follows the maintained
[Normalize implementation](https://raw.githubusercontent.com/UKPLab/sentence-transformers/v5.1.0/sentence_transformers/models/Normalize.py).

These tests qualify numerical execution and the supported declared recipe.
They do not qualify semantic retrieval quality, pretrained model admission,
live Pumas artifact handoff, GPU execution, or a GUI/model workflow.

The exact `sentence_transformers.models.Pooling` class has a defined inclusion
default. The pinned MiniLM [exporter metadata](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/raw/1110a243fdf4706b3f48f1d95db1a4f5529b4d41/config_sentence_transformers.json)
declares Sentence Transformers 2.0.0. Its [original Pooling implementation](https://raw.githubusercontent.com/UKPLab/sentence-transformers/3ddd7a76257a2611d48a7f6ceba6e3906b520eb1/sentence_transformers/models/Pooling.py)
has no prompt-exclusion parameter, saves no `include_prompt` field and pools
every token enabled by the supplied attention mask. The later [2.6.0 class](https://raw.githubusercontent.com/UKPLab/sentence-transformers/a5f774998eaae056e0cbe42dc1d587a636115c3d/sentence_transformers/models/Pooling.py)
loads configuration with `Pooling(**config)` and defaults `include_prompt` to
true; explicit false can mask a prompt prefix when prompt length is supplied.
Prompt-prefix exclusion is distinct from excluding BERT special tokens.

Only omission receives that class default in the existing admitted BERT recipe.
Tests load unchanged explicit-true fixtures, remove the field in temporary
copies and compare real Candle forwards exactly for both widths, against the
existing independent reference vectors. Explicit false, null and malformed
values are rejected without replacing the resident model. Other recipe,
tokenizer, dtype, identity and architecture gates remain unchanged. These
sources establish this field's semantics, not complete MiniLM admission.
