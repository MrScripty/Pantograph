# Runtime Operations

Pantograph keeps workflow policy in backend services, concrete runtime
execution in the runtime host, and desktop commands as transport/composition
adapters. See [ARCHITECTURE.md](../ARCHITECTURE.md) for ownership boundaries.

## Python Workers

Python-backed nodes execute in child processes; Pantograph does not embed one
global Python interpreter. The worker entry points are:

- `crates/inference/torch/worker.py`
- `crates/inference/audio/worker.py`

Interpreter selection uses, in order:

1. `PANTOGRAPH_PYTHON_ENV_MAP_JSON`, an `env_id` to executable JSON object;
2. `PANTOGRAPH_PYTHON_ENV_MAP_FILE`, containing the same mapping;
3. `PANTOGRAPH_PYTHON_EXECUTABLE`;
4. `PYO3_PYTHON`;
5. `python3` or `python` on `PATH`; and
6. the project `.venv` executable when available.

Example:

```bash
export PANTOGRAPH_PYTHON_ENV_MAP_JSON='{"venv:pytorch":"/opt/pantograph/pytorch/bin/python"}'
```

The selected interpreter and dependency environment are runtime inputs. A
missing mapping, dependency, model fact, or worker capability must remain an
explicit unavailable/unsupported outcome; do not fall back to a different
model or execution path.

The current process adapter does not yet provide complete task/reader/monitor
ownership and typed shutdown evidence. Operational automation must account for
that known limitation until the
[architecture remediation](plans/current-standards-remediation/architecture-lifecycle-and-bindings/plan.md)
is accepted.

## Runtime Registry Inspection

The desktop host exposes three diagnostic commands:

- `get_runtime_registry_snapshot` returns backend-owned runtime-registry state;
- `get_runtime_debug_snapshot` aggregates registry, health, recovery,
  scheduler, and workflow diagnostics; and
- `reclaim_runtime_registry_runtime` requests targeted backend-owned reclaim
  and returns the resulting state.

These commands are projections. Runtime identity, state transitions, reclaim
eligibility, and reconciliation remain owned by the runtime registry and
embedded runtime, not by Tauri or the frontend.

## Recovery And Reclaim

Recovery follows this ownership flow:

```text
desktop health/manual trigger
  -> recovery coordinator
  -> embedded-runtime restart plan
  -> producer stop/restore
  -> runtime-registry reconciliation
  -> projected diagnostics
```

Before forcing recovery:

1. capture the runtime and debug snapshots;
2. record the stable runtime identity and reported lifecycle state;
3. prefer targeted reclaim over broad restart;
4. verify the post-operation registry snapshot; and
5. preserve incomplete or failed shutdown/reclaim outcomes in the incident
   record.

Do not edit registry state directly, infer ownership from a process ID, or
treat log output as authoritative runtime state.

## Trust Warning

The current Diffusers worker has a critical remote-code trust bypass, and the
generated-component path can fail open. Do not use untrusted model packages or
generated UI source until the
[security remediation](plans/current-standards-remediation/security-and-dynamic-code/plan.md)
closes those paths.
