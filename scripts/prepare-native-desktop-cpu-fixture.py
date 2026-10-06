#!/usr/bin/env python3
"""Provision controlled discovery metadata for committed synthetic CPU weights."""
import hashlib
import json
from pathlib import Path
import shutil
import sys

ROOT = Path(__file__).resolve().parents[1]
MODEL_ID = "embedding/qualification/synthetic-bert-8"


def prepare(destination):
    destination = Path(destination).resolve()
    destination.mkdir(parents=True, exist_ok=False)
    project = destination / "project"
    library = destination / "synthetic-library"
    for path in [project / "src-tauri", project / ".pantograph/workflows",
                 library / "launcher-data/metadata", library / "launcher-data/cache",
                 library / "launcher-data/logs", library / "shared-resources/models"]:
        path.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / "Cargo.toml", project / "Cargo.toml")
    shutil.copyfile(ROOT / "src-tauri/Cargo.toml", project / "src-tauri/Cargo.toml")
    source = ROOT / "crates/inference/tests/fixtures/candle_bert/bert-8"
    reference = json.loads((source.parent / "reference-manifest.json").read_text())
    for name, expected in reference["file_hashes"].items():
        if name.startswith("bert-8/"):
            actual = hashlib.sha256((source.parent / name).read_bytes()).hexdigest()
            if actual != expected:
                raise ValueError(f"Committed CPU fixture hash mismatch: {name}")
    model = library / "shared-resources/models" / MODEL_ID
    shutil.copytree(source, model)
    metadata = {"schema_version": 2, "model_id": MODEL_ID, "family": "qualification",
                "model_type": "embedding", "official_name": "Synthetic-BERT-8",
                "cleaned_name": "synthetic-bert-8", "source_path": str(model),
                "entry_path": str(model), "storage_kind": "library_owned",
                "selected_artifact_id": "main", "selected_artifact_files": ["model.safetensors"],
                "import_state": "ready", "validation_state": "valid",
                "pipeline_tag": "feature-extraction", "task_type_primary": "embedding",
                "input_modalities": ["text"], "output_modalities": ["embedding"],
                "task_classification_source": "synthetic-native-qualification",
                "task_classification_confidence": 1.0,
                "recommended_backend": "candle", "runtime_engine_hints": ["candle"]}
    (model / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    descriptor = json.loads((ROOT / "crates/pantograph-inference-interface-contracts/tests/fixtures/descriptor_embedding_ready.json").read_text())
    snapshot = {key: descriptor[key] for key in ["contract_version", "descriptor_fingerprint", "task_kind"]}
    for direction in ["inputs", "outputs"]:
        snapshot[direction] = [{key: value for key, value in port.items() if key != "options"}
                               for port in descriptor[direction]]
    graph = {"nodes": [
        {"id": "prompt", "node_type": "text-input", "position": {"x": 0, "y": 100},
         "data": {"text": "hello world", "label": "Synthetic input: hello world"}},
        {"id": "infer", "node_type": "llm-inference", "position": {"x": 300, "y": 100},
         "data": {"label": "Synthetic CPU embedding", "task_kind": "embedding",
                  "runtime": "candle", "device": "cpu",
                  "runtime_source_context": {"operation_type": "embedding.text",
                                             "context_shape_key": "embedding.one-text",
                                             "cancellation_mode": "run_scoped"},
                  "pumas_model_ref": {"model_id": MODEL_ID, "selected_artifact_id": "main"},
                  "inference_interface_snapshot": snapshot}},
        {"id": "vectors", "node_type": "vector-output", "position": {"x": 600, "y": 100},
         "data": {"label": "Synthetic vector output"}},
        {"id": "deps", "node_type": "dependency-environment", "position": {"x": 300, "y": 450},
         "data": {"label": "Typed dependency control", "mode": "manual"}}],
        "edges": [
            {"id": "text-to-embedding", "source": "prompt", "source_handle": "text", "target": "infer", "target_handle": "text"},
            {"id": "embedding-to-vector", "source": "infer", "source_handle": "embedding", "target": "vectors", "target_handle": "vector"},
            {"id": "deps-to-infer", "source": "deps", "source_handle": "dependency_environment_sidecar",
             "target": "infer", "target_handle": "dependency_environment_sidecar"}]}
    (destination / "graph.json").write_text(json.dumps(graph, indent=2) + "\n")
    golden = json.loads((source / "golden.json").read_text())
    record = {"synthetic_untrained": True, "discovery": "controlled metadata seeded into isolated Pumas library; not real-user discovery acceptance",
              "model_id": MODEL_ID, "project_root": str(project), "library_root": str(library),
              "expected_vector": golden["single_vectors"][0],
              "fixture_sha256": {str(path.relative_to(source)): hashlib.sha256(path.read_bytes()).hexdigest()
                                 for path in sorted(source.rglob("*")) if path.is_file()}}
    (destination / "fixture.json").write_text(json.dumps(record, indent=2) + "\n")
    return record


if __name__ == "__main__":
    print(json.dumps(prepare(sys.argv[1]), indent=2))
