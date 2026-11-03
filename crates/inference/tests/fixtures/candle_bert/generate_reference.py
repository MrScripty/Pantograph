"""Regenerate independent, untrained CPU F32 Transformers reference packages.

Requires the exact versions recorded in reference-manifest.json. No downloads,
sentence-transformers execution, or Candle implementation are involved.
"""
import hashlib
import json
from pathlib import Path

import safetensors
import tokenizers
import torch
import transformers
from tokenizers import Tokenizer, models, normalizers, pre_tokenizers, processors
from transformers import BertConfig, BertModel

ROOT = Path(__file__).resolve().parent
torch.set_num_threads(1)
torch.use_deterministic_algorithms(True)


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


for width, heads in [(8, 2), (12, 3)]:
    directory = ROOT / f"bert-{width}"
    directory.mkdir(exist_ok=True)
    torch.manual_seed(171 + width)
    config = BertConfig(
        vocab_size=8, hidden_size=width, num_hidden_layers=1,
        num_attention_heads=heads, intermediate_size=4 * width,
        max_position_embeddings=16, type_vocab_size=2, pad_token_id=0,
        hidden_dropout_prob=0.0, attention_probs_dropout_prob=0.0,
        use_cache=False,
    )
    config._attn_implementation = "eager"
    model = BertModel(config, add_pooling_layer=False).eval()
    model.save_pretrained(directory, safe_serialization=True)
    vocab = {word: index for index, word in enumerate(
        ["[PAD]", "[UNK]", "[CLS]", "[SEP]", "hello", "world", "different", "third"]
    )}
    tokenizer = Tokenizer(models.WordPiece(vocab=vocab, unk_token="[UNK]"))
    tokenizer.normalizer = normalizers.BertNormalizer(lowercase=True)
    tokenizer.pre_tokenizer = pre_tokenizers.BertPreTokenizer()
    tokenizer.post_processor = processors.BertProcessing(("[SEP]", 3), ("[CLS]", 2))
    tokenizer.enable_truncation(max_length=16)
    tokenizer.enable_padding(pad_id=0, pad_token="[PAD]")
    tokenizer.save(str(directory / "tokenizer.json"))
    write_json(directory / "modules.json", [
        {"idx": index, "name": str(index), "path": path,
         "type": f"sentence_transformers.models.{kind}"}
        for index, path, kind in [(0, "", "Transformer"), (1, "1_Pooling", "Pooling"), (2, "2_Normalize", "Normalize")]
    ])
    (directory / "sentence_bert_config.json").write_text(json.dumps({"max_seq_length": 16, "do_lower_case": True}) + "\n")
    (directory / "1_Pooling").mkdir(exist_ok=True)
    (directory / "2_Normalize").mkdir(exist_ok=True)
    write_json(directory / "1_Pooling/config.json", {
        "word_embedding_dimension": width, "pooling_mode_cls_token": False,
        "pooling_mode_mean_tokens": True, "pooling_mode_max_tokens": False,
        "pooling_mode_mean_sqrt_len_tokens": False, "pooling_mode_weightedmean_tokens": False,
        "pooling_mode_lasttoken": False, "include_prompt": True,
    })

    def forward(texts):
        encodings = tokenizer.encode_batch(texts)
        ids = torch.tensor([encoding.ids for encoding in encodings])
        mask = torch.tensor([encoding.attention_mask for encoding in encodings])
        types = torch.tensor([encoding.type_ids for encoding in encodings])
        with torch.inference_mode():
            hidden = model(input_ids=ids, attention_mask=mask, token_type_ids=types).last_hidden_state
            mean = (hidden * mask.unsqueeze(-1)).sum(1) / mask.sum(1).unsqueeze(-1)
            vectors = torch.nn.functional.normalize(mean, p=2, dim=1, eps=1e-12)
        return ids.tolist(), mask.tolist(), types.tolist(), vectors.tolist()

    texts = ["hello world", "hello", "different world"]
    ids, masks, types, vectors = forward(texts)
    singles = [forward([text])[3][0] for text in texts]
    long_texts = ["hello world " * 20, "hello"]
    long_ids, long_masks, long_types, long_vectors = forward(long_texts)
    write_json(directory / "golden.json", {
        "kind": "synthetic_untrained_reference; not pretrained-model acceptance",
        "recipe": "declared BERT encoder -> attention-mask mean including special tokens -> L2 normalize, epsilon 1e-12",
        "texts": texts, "input_ids": ids, "attention_mask": masks, "token_type_ids": types,
        "hidden_size": width, "batch_vectors": vectors, "single_vectors": singles,
        "single_batch_max_abs_difference": max(abs(a - b) for row, single in zip(vectors, singles) for a, b in zip(row, single)),
        "truncation_texts": long_texts, "truncation_input_ids": long_ids,
        "truncation_attention_mask": long_masks, "truncation_token_type_ids": long_types,
        "truncation_vectors": long_vectors,
        "seed": 171 + width, "atol": 1e-5, "rtol": 1e-4,
    })
    print(f"actual Transformers CPU F32 forward: width={width}, batch=3, singles=3, truncated_batch=2")

write_json(ROOT / "reference-manifest.json", {
    "kind": "generated untrained numerical reference; not pretrained semantic qualification",
    "reference_packages": {"torch": torch.__version__, "transformers": transformers.__version__,
                           "tokenizers": tokenizers.__version__, "safetensors": safetensors.__version__},
    "torch_git_version": torch.version.git_version,
    "candle_pin": "88ed7911de9e88196b1f55b199145d22647f415e",
    "actual_reference_bert_forward_executed": True, "model_downloads": False,
    "file_hashes": {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
                    for path in sorted(ROOT.glob("bert-*/**/*")) if path.is_file()},
})
