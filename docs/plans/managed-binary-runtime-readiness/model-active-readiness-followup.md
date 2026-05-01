# Supplemental Plan: llama.cpp Model-Active Readiness Owner

## Objective

Introduce a single backend-owned readiness owner for llama.cpp startup phases so
`load_completed` is emitted only after the requested process, mode, port,
device, model path, and optional multimodal projection are proven active.

## Scope

- Add a typed runtime-load phase result owned by the inference/runtime layer.
- Emit process-spawning, process-spawned, HTTP-ready, requested-model-active,
  load-completed, and load-failed diagnostics from one owner path.
- Keep workflow-service scheduler code from inferring model residency from
  generic admission success.
- Add regressions for reused wrong-model processes, spawn/HTTP failure, and
  terminal run failure instead of stuck `running`.

## Standards Constraints

- Keep contracts additive and backend-owned.
- Do not expand over-threshold files such as `server.rs`,
  `runtime_capabilities.rs`, or `session_execution_api.rs`; extract focused
  modules before adding phase ownership.
- Keep blocking process/filesystem work out of async locks.
- Preserve `TauriProcessSpawner` task ownership for stdout, stderr, monitor,
  PID files, and cleanup.
- Use isolated temp roots and fake process/gateway implementations for durable
  state and lifecycle tests.

## Milestones

### 1. Runtime Phase Contract

- [ ] Add typed load phase DTOs and errors in the owning backend/runtime crate.
- [ ] Project resolved managed-binary command facts into the phase contract.
- [ ] Add tests for missing binary, partial install, and active mutating job.

### 2. llama.cpp Active Model Proof

- [ ] Extend `LlamaServer`/gateway state with a structured active-runtime
      descriptor for mode, port, model path, mmproj path, and device.
- [ ] Verify reused runtimes against that descriptor before reporting ready.
- [ ] Add wrong-model and wrong-mode reuse regressions.

### 3. Scheduler Diagnostics Ownership

- [ ] Move lifecycle phase emission behind a single runtime-load owner.
- [ ] Emit `load_completed` only after requested-model-active proof.
- [ ] Convert spawn/HTTP/model mismatch failures into terminal run failures.

### 4. Verification

- [ ] `cargo test -p inference llamacpp`
- [ ] `cargo test -p pantograph-workflow-service session_execution`
- [ ] `cargo test -p pantograph-embedded-runtime session_runtime`
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] `bash launcher.sh --build-release`

## Re-Plan Triggers

- llama.cpp does not expose enough runtime identity to prove model activity
  without an active request probe.
- The current gateway abstraction cannot return structured active-runtime
  descriptors without breaking existing backend implementations.
- Process lifecycle diagnostics require changes to Tauri task ownership.
