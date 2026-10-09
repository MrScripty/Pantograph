#!/usr/bin/env python3
"""Apply the reviewed snapshot API to an existing clean, pinned source checkout.

No downloads, build, environment installation, credentials or runtime changes.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

REVISION = "e892882fd4608b468dcf9dc33ea95283882b8e6d"


def prepare(destination):
    bundle = Path(__file__).resolve().parent
    destination = destination.resolve(strict=True)

    def git(*args):
        return subprocess.check_output(["git", "-C", str(destination), *args], text=True).strip()

    if git("rev-parse", "HEAD") != REVISION or git("status", "--porcelain"):
        raise SystemExit("Requires a clean Tokenizers source checkout at " + REVISION)
    target = destination / "bindings/python/src/pantograph_snapshot.rs"
    if target.exists():
        raise SystemExit("Snapshot module already exists")
    patch = bundle / "provider.patch"
    git("apply", "--check", str(patch))
    git("apply", str(patch))
    shutil.copyfile(bundle / "pantograph_snapshot.rs", target)
    shutil.copyfile(bundle / "snapshot_units.rs", target.with_name("snapshot_units.rs"))
    shutil.copyfile(bundle / "build.rs", destination / "bindings/python/build.rs")
    shutil.copyfile(bundle / "Cargo.lock", destination / "bindings/python/Cargo.lock")
    paths = ["tokenizers/src/models/wordlevel/mod.rs",
             "tokenizers/src/tokenizer/added_vocabulary.rs",
             "bindings/python/src/lib.rs", "bindings/python/src/tokenizer.rs",
             "bindings/python/src/pantograph_snapshot.rs", "bindings/python/src/snapshot_units.rs",
             "bindings/python/build.rs",
             "bindings/python/Cargo.lock"]
    manifest = {"provider_revision": REVISION,
                "profile": "pantograph-tokenizers-0.21.4-wordlevel-snapshot.v1",
                "source_sha256": {name: hashlib.sha256((destination/name).read_bytes()).hexdigest()
                                  for name in paths}}
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("checkout", type=Path)
    prepare(parser.parse_args().checkout)
