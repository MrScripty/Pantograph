#!/usr/bin/env python3

"""Retired process bridge for python-backed Pantograph workflow nodes.

The old bridge must not launch runtime work from graph paths. Runtime execution
is scheduler-owned and must flow through runtime-host contracts.
"""

from __future__ import annotations

import json
import traceback
from typing import Any, Dict


def _retired_runtime_error(node_type: str) -> RuntimeError:
    return RuntimeError(
        "retired_python_runtime_bridge: "
        f"Python-backed node '{node_type}' must run through scheduler task "
        "state/results and runtime-host execution, not the old process bridge."
    )


def _run_audio(inputs: Dict[str, Any]) -> Dict[str, Any]:
    _ = inputs
    raise _retired_runtime_error("audio-generation")


def _run_onnx(inputs: Dict[str, Any]) -> Dict[str, Any]:
    _ = inputs
    raise _retired_runtime_error("onnx-inference")


def _main() -> int:
    raw = input_stream = ""
    try:
        import sys

        raw = sys.stdin.read()
        payload = json.loads(raw if raw else "{}")

        node_type = payload.get("node_type")
        if not isinstance(node_type, str) or not node_type.strip():
            raise RuntimeError("Missing node_type in python runtime bridge payload")
        node_type = node_type.strip()

        inputs = payload.get("inputs")
        if not isinstance(inputs, dict):
            inputs = {}

        if node_type == "audio-generation":
            outputs = _run_audio(inputs)
        elif node_type == "onnx-inference":
            outputs = _run_onnx(inputs)
        else:
            raise RuntimeError(f"Unsupported python runtime node_type '{node_type}'")

        print(json.dumps({"ok": True, "outputs": outputs}, separators=(",", ":")))
        return 0
    except Exception as exc:
        trace = traceback.format_exc()
        print(
            json.dumps(
                {"ok": False, "error": str(exc), "traceback": trace},
                separators=(",", ":"),
            )
        )
        return 1


if __name__ == "__main__":
    raise SystemExit(_main())
