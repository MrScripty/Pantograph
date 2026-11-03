#!/usr/bin/env python3
"""Verify consumer pins and reject additive Cargo ONNX download capabilities."""
import re
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGETS = ("x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "aarch64-apple-darwin")


def check_features(tree: str, scope: str) -> None:
    for line in tree.splitlines():
        if not line.strip():
            continue
        identity, features = line.split("|", 1)
        name = identity.split()[0]
        if name not in ("ort", "ort-sys"):
            continue
        enabled = set(features.removesuffix(" (*)").split(","))
        forbidden = enabled & {"download-binaries", "fetch-models", "copy-dylibs", "tls-native"}
        if forbidden:
            raise RuntimeError(f"{scope}: {name} enables forbidden features {sorted(forbidden)}")
        required = "load-dynamic" if name == "ort" else "disable-linking"
        if required not in enabled:
            raise RuntimeError(f"{scope}: {name} requires {required}")


def check_pins() -> None:
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
    dependency = manifest["workspace"]["dependencies"]["pumas-library"]
    pin = dependency["rev"]
    if not re.fullmatch(r"[0-9a-f]{40}", pin):
        raise RuntimeError("Pumas requires an exact full commit pin")
    locked = tomllib.loads((ROOT / "Cargo.lock").read_text())
    sources = [p["source"] for p in locked["package"] if p["name"] == "pumas-library"]
    expected = f"git+{dependency['git']}?rev={pin}#{pin}"
    if sources != [expected]:
        raise RuntimeError("Pumas lockfile source does not match its manifest pin")
    workflow = (ROOT / ".github/workflows/quality-gates.yml").read_text()
    if re.findall(r"PUMAS_LIBRARY_REF: ([0-9a-f]+)", workflow) != [pin]:
        raise RuntimeError("Pumas CI checkout reference does not match its Cargo pin")


def main() -> None:
    check_pins()
    for target in TARGETS:
        for selection in (
            ["--workspace"],
            ["--workspace", "--all-features"],
            ["-p", "pantograph-embedded-runtime", "--no-default-features",
             "--features", "backend-llamacpp,backend-pytorch"],
        ):
            command = ["cargo", "tree", "--locked", "--target", target,
                       "--edges", "normal,build,dev", "--prefix", "none",
                       "--format", "{p}|{f}", *selection]
            tree = subprocess.check_output(command, cwd=ROOT, text=True)
            check_features(tree, f"{target} {selection}")
            print(f"{target} {selection}: ONNX no-build-download contract passed", flush=True)


if __name__ == "__main__":
    main()
