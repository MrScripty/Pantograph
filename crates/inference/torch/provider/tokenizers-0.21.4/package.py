#!/usr/bin/env python3
"""Package verified local build bytes for review; never install or register them.

The owner must separately accept a local component through Pumas runtime bundle
custody and bind the live interpreter. This creates neither ownership nor trust.
"""
import argparse
import base64
import csv
import hashlib
import io
import json
from pathlib import Path
import subprocess
import zipfile

VERSION = "0.21.4+pantograph.snapshot1"
TAG = "cp39-abi3-linux_x86_64"
REVISION = "e892882fd4608b468dcf9dc33ea95283882b8e6d"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def package(provider, binding_path, output):
    provider = provider.resolve(strict=True)
    binding = json.loads(binding_path.read_text())
    revision = subprocess.check_output(
        ["git", "-C", str(provider), "rev-parse", "HEAD"], text=True).strip()
    if revision != REVISION or binding["provider_revision"] != REVISION:
        raise ValueError("wrong upstream revision")
    required = {"tokenizers/src/models/wordlevel/mod.rs",
                "tokenizers/src/tokenizer/added_vocabulary.rs",
                "bindings/python/src/lib.rs", "bindings/python/src/tokenizer.rs",
                "bindings/python/src/pantograph_snapshot.rs",
                "bindings/python/src/pantograph_snapshot/settings_snapshot.rs",
                "bindings/python/src/snapshot_units.rs", "bindings/python/build.rs",
                "bindings/python/Cargo.lock"}
    if set(binding["source_sha256"]) != required:
        raise ValueError("incomplete source association")
    changed = set()
    for command in (["diff", "--name-only", "HEAD", "-z"],
                    ["ls-files", "--others", "--exclude-standard", "-z"]):
        names = subprocess.check_output(["git", "-C", str(provider), *command]).decode().split("\0")
        changed.update(name for name in names if name)
    if not changed <= required:
        raise ValueError("unassociated source changes")
    for name, expected in binding["source_sha256"].items():
        if digest((provider / name).read_bytes()) != expected:
            raise ValueError("source bytes changed: " + name)
    artifact = Path(binding["artifact"]["path"]).resolve(strict=True)
    native = artifact.read_bytes()
    if len(native) != binding["artifact"]["bytes"] or digest(native) != binding["artifact"]["sha256"]:
        raise ValueError("artifact bytes changed")
    source = provider / "bindings/python/py_src"
    tracked = set(subprocess.check_output(
        ["git", "-C", str(provider), "ls-files", "-z", "--", "bindings/python/py_src"]
    ).decode().split("\0"))
    files = {}
    for path in sorted(source.rglob("*")):
        if path.is_symlink():
            raise ValueError("package symlink refused")
        if path.is_file() and path.suffix in (".py", ".pyi"):
            if path.relative_to(provider).as_posix() not in tracked:
                raise ValueError("unassociated Python package input")
            files[path.relative_to(source).as_posix()] = path.read_bytes()
    files["tokenizers/tokenizers.abi3.so"] = native
    info = f"tokenizers-{VERSION}.dist-info"
    files[f"{info}/METADATA"] = (
        f"Metadata-Version: 2.1\nName: tokenizers\nVersion: {VERSION}\n"
        "Summary: Pantograph source-qualified local snapshot development provider\n"
        "License: Apache-2.0\nRequires-Python: >=3.9\n"
        "Requires-Dist: huggingface-hub>=0.16.4,<1.0\n\n").encode()
    files[f"{info}/WHEEL"] = (
        f"Wheel-Version: 1.0\nGenerator: pantograph-local-provider\n"
        f"Root-Is-Purelib: false\nTag: {TAG}\n").encode()
    files[f"{info}/LICENSE"] = (provider / "LICENSE").read_bytes()
    # Embed source/artifact association, preserving its exact original bytes.
    files[f"{info}/pantograph-build-source-binding.json"] = binding_path.read_bytes()
    records = io.StringIO(newline="")
    writer = csv.writer(records, lineterminator="\n")
    for name, data in sorted(files.items()):
        encoded = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=").decode()
        writer.writerow((name, "sha256=" + encoded, len(data)))
    writer.writerow((f"{info}/RECORD", "", ""))
    files[f"{info}/RECORD"] = records.getvalue().encode()
    output.mkdir(parents=True, exist_ok=True)
    wheel = output / f"tokenizers-{VERSION}-{TAG}.whl"
    if wheel.exists():
        raise ValueError("existing artifact refused")
    with zipfile.ZipFile(wheel, "x", compression=zipfile.ZIP_STORED) as archive:
        for name, data in sorted(files.items()):
            entry = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            entry.external_attr = 0o100644 << 16
            archive.writestr(entry, data)
    receipt = {"wheel": str(wheel), "bytes": wheel.stat().st_size,
               "sha256": digest(wheel.read_bytes()), "build_binding_sha256": digest(binding_path.read_bytes()),
               "custody": "unregistered local development artifact; Pumas local ingestion and live interpreter binding required",
               "qualification": "component-only; settings require exact CPython3.12.3 build qualification; aggregate Unknown"}
    wheel.with_suffix(".receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("provider", type=Path)
    parser.add_argument("binding", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    print(json.dumps(package(args.provider, args.binding, args.output), indent=2))
