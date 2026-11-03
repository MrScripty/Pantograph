"""Independent Transformers CPU reference for the richer graph; no downloads."""
import hashlib
import json
from pathlib import Path

import torch
import transformers
from tokenizers import Tokenizer
from transformers import BertModel

ROOT = Path(__file__).resolve().parents[2]
torch.set_num_threads(1)
results = {}
for width, text in [(8, "different world\nhello"), (12, "different world")]:
    directory = ROOT / f"crates/inference/tests/fixtures/candle_bert/bert-{width}"
    encoding = Tokenizer.from_file(str(directory / "tokenizer.json")).encode(text)
    inputs = {name: torch.tensor([value]) for name, value in {
        "input_ids": encoding.ids, "attention_mask": encoding.attention_mask,
        "token_type_ids": encoding.type_ids,
    }.items()}
    model = BertModel.from_pretrained(directory, local_files_only=True,
                                      add_pooling_layer=False, attn_implementation="eager").eval()
    with torch.inference_mode():
        hidden = model(**inputs).last_hidden_state
        mask = inputs["attention_mask"].unsqueeze(-1)
        mean = (hidden * mask).sum(1) / mask.sum(1)
        vector = torch.nn.functional.normalize(mean, p=2, dim=1, eps=1e-12)[0].tolist()
    weights = (directory / "model.safetensors").read_bytes()
    results[str(width)] = {"text": text, "vector": vector,
                           "weights_bytes": len(weights),
                           "weights_sha256": hashlib.sha256(weights).hexdigest()}
print(json.dumps({"torch": torch.__version__, "transformers": transformers.__version__,
                  "device": "cpu", "dtype": "float32", "models": results}, indent=2))
