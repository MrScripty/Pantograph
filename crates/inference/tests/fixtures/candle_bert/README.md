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
