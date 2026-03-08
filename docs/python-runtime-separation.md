# Python Runtime Separation

## Summary

Pantograph no longer embeds Python in-process. Python-backed nodes execute through a host-managed process adapter, and model environments are resolved externally per dependency binding.

This is a breaking change by design. No backward compatibility path is provided for in-process PyO3 execution.

## What Changed

- Default Pantograph build excludes Python backend features.
- `pantograph` binary no longer links `libpython`.
- `pytorch-inference` and `audio-generation` execute through `ProcessPythonRuntimeAdapter`.
- Dependency preflight is enforced before execution (plan resolution, readiness check, `model_ref` resolution/validation, and deterministic blocking for non-ready states including `unpinned_dependency`).

## Runtime Configuration

Configure Python executables using one of:

- `PANTOGRAPH_PYTHON_ENV_MAP_JSON`
  - JSON object mapping `env_id` to Python executable path.
  - Example:
```json
{
  "venv:profile.pytorch:2:abc123:linux-x86_64:pytorch": "/opt/pantograph/envs/pytorch-a/bin/python",
  "venv:stable-audio:1:def456:linux-x86_64:stable_audio": "/opt/pantograph/envs/audio-b/bin/python"
}
```

- `PANTOGRAPH_PYTHON_ENV_MAP_FILE`
  - Path to a JSON file with the same `env_id -> python_path` shape.

- `PANTOGRAPH_PYTHON_EXECUTABLE`
  - Fallback Python executable when an env-specific mapping is not provided.
  - Supports either absolute/relative paths or executable names on `PATH` (for example `python3`).

- `PYO3_PYTHON`
  - Secondary fallback used when `PANTOGRAPH_PYTHON_EXECUTABLE` is unset.
  - Also accepts paths or `PATH` executable names.

- `PATH` discovery
  - If no explicit variables are set, Pantograph attempts `python3` then `python` from `PATH`.
- Local project fallback
  - For direct binary runs, Pantograph also checks `./.venv/bin/python3`, `./.venv/bin/python`, then `./.venv/Scripts/python.exe` (repo root).

## Operational Notes

- The adapter launches Python with argument-safe process APIs (`tokio::process::Command`), not shell command strings.
- Worker scripts are loaded from repository worker paths:
  - `crates/inference/torch/worker.py`
  - `crates/inference/audio/worker.py`
- A bridge script is materialized at runtime in the system temp directory:
  - `pantograph_python_runtime_bridge.py`
- `launcher.sh` exports `PANTOGRAPH_PYTHON_EXECUTABLE` to the project `.venv` Python by default.
- `./launcher.sh --install` provisions both the base Python runtime
  requirements and `requirements-diffusion.txt`, so local text-to-image
  workflows can use the default `.venv` without a second manual install step.

## Migration Notes

If your deployment previously relied on embedded Python:

- Remove assumptions that Pantograph ships/links one global Python runtime.
- Provision model-specific virtual environments externally.
- Register each resolved `env_id` to its Python executable using `PANTOGRAPH_PYTHON_ENV_MAP_JSON` or `PANTOGRAPH_PYTHON_ENV_MAP_FILE`.
- Validate with:
```bash
npm run test:runtime-separation
```

For local diffusion worker verification against an imported model bundle, use:
```bash
./.venv/bin/python scripts/diffusion_cli_smoketest.py --model-path /path/to/tiny-sd-turbo
```

## Verification Guard

The separation guard is implemented in:

- `scripts/check-no-python-linkage.sh`

CI workflow:

- `.github/workflows/runtime-separation-check.yml`
