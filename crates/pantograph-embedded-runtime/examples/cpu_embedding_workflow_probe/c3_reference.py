"""Verify the frozen c3-768b-v1 inputs with cached, unchanged reference weights.

Run once before the separately bounded hosted cohort. No model generation,
downloads, timing calibration, input tuning, or writes to the original fixture.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path

import safetensors
import tokenizers
import torch
import transformers
from tokenizers import Tokenizer
from transformers import BertModel


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if not args.output.is_absolute() or args.output.exists():
        raise ValueError("pass a new absolute reference output file")

    fixture_root = Path(__file__).resolve().parents[3] / "inference/tests/fixtures/candle_bert"
    fixture = fixture_root / "bert-8"
    manifest = json.loads((fixture_root / "reference-manifest.json").read_text())
    versions = {
        "torch": torch.__version__, "transformers": transformers.__version__,
        "tokenizers": tokenizers.__version__, "safetensors": safetensors.__version__,
    }
    if versions != manifest["reference_packages"]:
        raise ValueError(f"reference package versions changed: {versions}")
    hashes = {}
    for relative, expected in manifest["file_hashes"].items():
        if relative.startswith("bert-8/"):
            actual = digest((fixture_root / relative).read_bytes())
            if actual != expected:
                raise ValueError(f"reference fixture changed: {relative}")
            hashes[relative] = actual

    texts = ["hello world " * 64, "hello"]
    expected_hashes = [
        "37abb1bb44478b85ca20fe74213169f68d833875a43317f10d76733465ef82b1",
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    ]
    if [digest(text.encode()) for text in texts] != expected_hashes:
        raise ValueError("frozen workload identity changed")
    golden = json.loads((fixture / "golden.json").read_text())
    tokenizer = Tokenizer.from_file(str(fixture / "tokenizer.json"))
    encodings = tokenizer.encode_batch(texts)
    arrays = {
        "input_ids": [item.ids for item in encodings],
        "attention_mask": [item.attention_mask for item in encodings],
        "token_type_ids": [item.type_ids for item in encodings],
    }
    for field, actual in arrays.items():
        if actual != golden[f"truncation_{field}"]:
            raise ValueError(f"exact input reference differs: {field}")

    torch.set_num_threads(1)
    torch.use_deterministic_algorithms(True)
    model = BertModel.from_pretrained(
        fixture, local_files_only=True, add_pooling_layer=False,
        attn_implementation="eager",
    ).eval()
    tensors = {name: torch.tensor(value) for name, value in arrays.items()}
    with torch.inference_mode():
        hidden = model(**tensors).last_hidden_state
        mask = tensors["attention_mask"]
        mean = (hidden * mask.unsqueeze(-1)).sum(1) / mask.sum(1).unsqueeze(-1)
        vectors = torch.nn.functional.normalize(mean, p=2, dim=1, eps=1e-12)
    actual = vectors.tolist()
    error = max(abs(a - b) for row, ref in zip(actual, golden["truncation_vectors"])
                for a, b in zip(row, ref))
    oracle_passed = bool(torch.isfinite(vectors).all()) and vectors.shape == (2, 8) and error <= 1e-5
    result = {
        "protocol_id": "c3-768b-v1", "reference_packages": versions,
        "device": str(next(model.parameters()).device), "cpu_threads": torch.get_num_threads(),
        "texts": texts, "text_sha256": expected_hashes,
        "utf8_bytes": [len(text.encode()) for text in texts],
        **arrays, "vectors": [[x if math.isfinite(x) else str(x) for x in row] for row in actual],
        "max_absolute_reference_error": error if math.isfinite(error) else str(error),
        "reference_oracle_passed": oracle_passed,
        "actual_reference_cpu_forward_executed": True,
        "fixture_file_sha256": hashes,
        "reference_manifest_sha256": digest((fixture_root / "reference-manifest.json").read_bytes()),
        "model_downloads": False, "fixture_modified": False,
    }
    with args.output.open("x") as out:
        json.dump(result, out, indent=2, allow_nan=False)
        out.write("\n")
    if not oracle_passed:
        raise ValueError(f"CPU reference output mismatch; actual result preserved: {error}")
    print(json.dumps({"output": str(args.output), "utf8_bytes": result["utf8_bytes"],
                      "max_absolute_reference_error": error, "device": result["device"]}))


if __name__ == "__main__":
    main()
