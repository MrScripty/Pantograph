# Pumas v2 Cutover Verification Log

Date: 2026-02-27

## Completed

- `npx tsc --noEmit` (pass)
- `cargo check -p node-engine --features inference-nodes` (pass)
- `cargo check -p pantograph` (pass)
- `cargo check -p pantograph --bin pumas_dependency_runtime_probe` (pass)
- `cargo test -p node-engine` (pass)
- `cargo test -p node-engine --features inference-nodes` (pass)
- `cargo test -p pantograph workflow::commands::tests::test_build_model_dependency_request` (pass)
- `cargo test -p pantograph workflow::model_dependencies::tests` (pass)
- `npm run build` (pass with existing Svelte warnings)
- `timeout 20s npm run dev -- --host 127.0.0.1 --port 4173` (dev server started successfully for runtime validation)
- `cargo run -p pantograph --bin pumas_dependency_runtime_probe -- --json --install` (pass)
  - Full log: `docs/logs/pumas-runtime-probe-2026-02-27.log`
  - Runtime flow executed for one audio and one non-audio model:
    - `resolve -> check -> install -> check`
    - both scenarios ended in `ready` with `blocked_after_check=0`
- `cargo run -p pantograph --bin pumas_dependency_runtime_probe -- --json --model-id llm/gen-verse/trado-8b-instruct --model-id llm/stabilityai/stable-audio-open-1_0` (pass, escalated runtime)
  - Full log: `docs/logs/pumas-runtime-probe-trado-stable-audio-2026-02-27.log`
  - Explicit model evidence:
    - TraDo (`llm/gen-verse/trado-8b-instruct`) dependency hints found from model files:
      - `has_modeling_py=true`
      - `has_auto_map=true`
      - `import_hints=[flash_attn, liger_kernel, torch, transformers]`
    - Stable Audio (`llm/stabilityai/stable-audio-open-1_0`) model directory has no explicit dependency sidecar files
    - Backend matrix for both models shows `binding_count=0` for all probed backends (`stable_audio`, `pytorch`, `transformers`, `llamacpp`, `ollama`, `unspecified`)
    - Resolver flow still returns `ready` with zero bindings, confirming dependency bindings are not currently attached in the authoritative Pumas dependency tables.
- `cargo run -p pantograph --bin pumas_dependency_runtime_probe -- --json --model-id llm/gen-verse/trado-8b-instruct --model-id llm/stabilityai/stable-audio-open-1_0` (pass, escalated runtime, pass 2)
  - Full log: `docs/logs/pumas-runtime-probe-trado-stable-audio-2026-02-27-pass2.log`
  - Confirmed same result after latest implementation pass: both models are `ready` and `binding_count=0`.
- `cargo run -p pantograph --bin pumas_dependency_runtime_probe -- --json --install --model-id llm/gen-verse/trado-8b-instruct --model-id llm/stabilityai/stable-audio-open-1_0` (pass, escalated runtime, pass 2)
  - Full log: `docs/logs/pumas-runtime-probe-trado-stable-audio-install-2026-02-27-pass2.log`
  - End-to-end flow (`resolve -> check -> install -> check`) executes successfully for both models; install is a no-op because bindings are empty.
- `cargo run -p pantograph --bin pumas_dependency_runtime_probe -- --json` (pass, escalated runtime, auto selection)
  - Full log: `docs/logs/pumas-runtime-probe-auto-2026-02-27-pass2.log`
  - Auto-selection logic still cannot find an audio/non-audio model with dependency bindings in the current local library.
- `cargo run -p pantograph --bin pumas_dependency_runtime_probe -- --json --scan-all --model-id llm/gen-verse/trado-8b-instruct --model-id llm/stabilityai/stable-audio-open-1_0` (pass, escalated runtime)
  - Full log: `docs/logs/pumas-runtime-probe-trado-stable-audio-scanall-2026-02-27.log`
  - New evidence from scan mode:
    - `scan_all_models=22`
    - `scan_binding_hits=0`
    - `scan_result=no_non_empty_bindings_found`
  - This confirms no model/backend pair in the current local Pumas library resolves to non-empty dependency bindings.
- Non-escalated probe attempt fails in sandbox with readonly DB error (expected in this environment):
  - `Database error: attempt to write a readonly database`
  - Runtime evidence commands must run with escalated permissions for authoritative local Pumas DB access.

## Not Yet Completed

- E2E acceptance:
  - Stable Audio missing -> install -> ready -> execute
  - One non-audio model path with same dependency gate behavior
  - Note: scan-all evidence shows `scan_binding_hits=0` across all 22 local models, so per-binding remediation paths cannot be exercised until dependency bindings are published in the authoritative Pumas dependency tables.
