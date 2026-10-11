"""Execute the committed untrained BERT fixture locally, without changing it.

This is independent model-forward evidence, NOT Pantograph workflow/editor
qualification. It never downloads a model or reads a user's model catalog.
"""
import hashlib
import json
import platform
import time
from pathlib import Path

import safetensors
import tokenizers
import torch
import transformers
from tokenizers import Tokenizer
from transformers import BertModel


def main():
    fixture_root = Path(__file__).resolve().parents[1] / "crates/inference/tests/fixtures/candle_bert"
    fixture = fixture_root / "bert-8"
    manifest = json.loads((fixture_root / "reference-manifest.json").read_text())
    # Check every committed model/config/reference input before loading anything.
    for name, expected in manifest["file_hashes"].items():
        if name.startswith("bert-8/"):
            actual = hashlib.sha256((fixture_root / name).read_bytes()).hexdigest()
            if actual != expected:
                raise ValueError(f"Fixture checksum mismatch: {name}")
    torch.set_num_threads(1)
    torch.use_deterministic_algorithms(True)
    started = time.perf_counter()
    model = BertModel.from_pretrained(
        fixture, local_files_only=True, add_pooling_layer=False,
        attn_implementation="eager", use_safetensors=True,
    ).eval().to(device="cpu", dtype=torch.float32)
    tokenizer = Tokenizer.from_file(str(fixture / "tokenizer.json"))
    load_ms = (time.perf_counter() - started) * 1000
    prompt = "hello world"
    encoded = tokenizer.encode(prompt)
    ids = torch.tensor([encoded.ids])
    mask = torch.tensor([encoded.attention_mask])
    types = torch.tensor([encoded.type_ids])
    started = time.perf_counter()
    with torch.inference_mode():
        hidden = model(input_ids=ids, attention_mask=mask, token_type_ids=types).last_hidden_state
        pooled = (hidden * mask.unsqueeze(-1)).sum(1) / mask.sum(1).unsqueeze(-1)
        vector = torch.nn.functional.normalize(pooled, p=2, dim=1, eps=1e-12)[0]
    forward_ms = (time.perf_counter() - started) * 1000
    golden = json.loads((fixture / "golden.json").read_text())["single_vectors"][0]
    expected = torch.tensor(golden, dtype=torch.float32)
    torch.testing.assert_close(vector, expected, atol=1e-5, rtol=1e-4)
    print(json.dumps({
        "scope": "independent_model_forward_only",
        "editor_qualified": False, "scheduler_qualified": False,
        "backend": "transformers.BertModel / torch", "device": "cpu", "dtype": "float32",
        "python": platform.python_version(), "platform": platform.platform(),
        "packages": {"torch": torch.__version__, "transformers": transformers.__version__,
                     "tokenizers": tokenizers.__version__, "safetensors": safetensors.__version__},
        "model": "committed untrained synthetic BERT-8; no semantic-quality claim",
        "weights_sha256": manifest["file_hashes"]["bert-8/model.safetensors"],
        "weights_bytes": (fixture / "model.safetensors").stat().st_size,
        "model_acquisition_bytes": 0, "prompt": prompt,
        "load_wall_ms": load_ms, "forward_wall_ms": forward_ms,
        "output": vector.tolist(), "max_absolute_reference_error": (vector - expected).abs().max().item(),
    }, indent=2))


if __name__ == "__main__":
    main()
