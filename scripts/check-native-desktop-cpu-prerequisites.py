#!/usr/bin/env python3
"""Read-only native desktop qualification admission; never installs packages."""
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys


def inspect():
    missing = []
    commands = {name: shutil.which(name) for name in
                ["pkg-config", "WebKitWebDriver", "Xvfb", "xvfb-run", "tauri-driver", "protoc"]}
    if os.environ.get("PROTOC"):
        commands["protoc"] = shutil.which(os.environ["PROTOC"])
    missing.extend(name for name, path in commands.items() if not path)
    libraries = {}
    for name in ["gtk+-3.0", "webkit2gtk-4.1", "javascriptcoregtk-4.1", "libsoup-3.0"]:
        result = subprocess.run([commands["pkg-config"], "--modversion", name],
                                capture_output=True, text=True) if commands["pkg-config"] else None
        libraries[name] = result.stdout.strip() if result and result.returncode == 0 else None
        if libraries[name] is None:
            missing.append(name)
    if platform.system() != "Linux":
        missing.append("existing Linux native-driver platform")
    return {"platform": platform.platform(), "commands": commands, "libraries": libraries,
            "missing": missing, "admitted": not missing,
            "policy": "authorized official runner dependency setup allowed; no security changes or access-denial workaround",
            "qualification": "not run; availability probe is not desktop/CPU graph acceptance"}


if __name__ == "__main__":
    record = inspect()
    output = Path(os.environ.get("PANTOGRAPH_NATIVE_CPU_EVIDENCE_DIR", ".native-cpu-evidence"))
    output.mkdir(parents=True, exist_ok=True)
    (output / "native-prerequisites.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2), flush=True)
    sys.exit(0 if record["admitted"] else 2)
