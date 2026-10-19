"""Read-only M4 fixture admission. No host, native forward or reference regeneration."""
import argparse
import hashlib
import json
import math
from pathlib import Path

REFERENCE_MANIFEST_SHA256 = "ae1644b099d946e03e75a1b75c65593bd2fcee0aa03df31aa79fb5fb94e2bd39"
M3_HEAD = "b63aa3ebe5fab1531aef952a35ce74932bf76b30"


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare(output):
    repo = Path(__file__).resolve().parents[4]
    fixtures = repo / "crates/inference/tests/fixtures/candle_bert"
    manifest_path = fixtures / "reference-manifest.json"
    if sha256(manifest_path) != REFERENCE_MANIFEST_SHA256:
        raise ValueError("independent reference manifest changed; re-admission required")
    manifest = json.loads(manifest_path.read_text())
    checked = {}
    for name, expected in manifest["file_hashes"].items():
        actual = sha256(fixtures / name)
        if actual != expected:
            raise ValueError(f"fixture changed: {name}")
        checked[name] = actual
    models = {}
    for width in (8, 12):
        golden = json.loads((fixtures / f"bert-{width}/golden.json").read_text())
        if golden["hidden_size"] != width:
            raise ValueError("unexpected hidden size")
        vectors = golden["single_vectors"]
        if len(vectors) != len(golden["texts"]):
            raise ValueError("reference text/vector population differs")
        for vector in vectors:
            if len(vector) != width or not all(math.isfinite(x) for x in vector):
                raise ValueError("reference vector shape/finite check failed")
            if abs(sum(x * x for x in vector) - 1) > 1e-5:
                raise ValueError("reference is not the declared normalized recipe")
        text = golden["texts"][0]
        models[f"bert-{width}"] = {
            "hidden_size": width,
            "text": text,
            "text_sha256": hashlib.sha256(text.encode()).hexdigest(),
            "golden_vector": vectors[0],
            "golden_vector_position": 0,
            "atol": golden["atol"],
            "rtol": golden["rtol"],
        }
    # Hand-declared fixed acceptance oracle, independent of production selection.
    result = {
        "kind": "M4 fixture admission; no native/reference execution",
        "m3_preserved_head": M3_HEAD,
        "m3_acceptance": "pending parent confirmation",
        "native_invocations": 0,
        "reference_manifest_sha256": REFERENCE_MANIFEST_SHA256,
        "fixture_file_sha256": checked,
        "reference_packages": manifest["reference_packages"],
        "models": models,
        "fixed_work": ["bert-8", "bert-8", "bert-12", "bert-12", "bert-8"],
        "retained_protocol_expected": {
            "cold_loads": ["bert-8", "bert-12", "bert-8"],
            "same_owner_retains": ["bert-8", "bert-12"],
            "switch_stops": ["bert-8", "bert-12"],
            "final_idle_stop": "bert-8",
            "outputs": 5,
        },
        "stop_between_tasks_expected": {
            "cold_loads": ["bert-8", "bert-8", "bert-12", "bert-12", "bert-8"],
            "total_stops": 5,
            "outputs": 5,
        },
        "native_capacity_and_pressure": "must freeze authoritative consumed claims before producer admission; no invented small estimates",
        "phase_fields_required": ["load", "forward_and_drain", "stop", "reload"],
        "oracle_identity": "full run/task/attempt/request/lease plus native instance and generation, never local task name alone",
        "performance_claim": False,
        "semantic_model_quality_claim": False,
        "model_acquisition_or_regeneration": False,
    }
    if not output.is_absolute():
        raise ValueError("output must be an absolute new file")
    with output.open("x") as handle:
        json.dump(result, handle, indent=2)
        handle.write("\n")
    print(f"Verified {len(checked)} original fixture files; no models executed")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    prepare(parser.parse_args().output)
