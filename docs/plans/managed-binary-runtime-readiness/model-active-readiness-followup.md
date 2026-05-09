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

- [x] Add typed load phase DTOs and errors in the owning backend/runtime crate.
- [x] Project resolved managed-binary command facts into the phase contract.
- [x] Add tests for missing binary, partial install, and active mutating job.

Status: Complete. The `inference::runtime_load` module now owns pure
runtime-load phase DTOs, bounded command facts, active llama.cpp runtime
descriptor shape, and managed-runtime readiness errors without expanding
`server.rs` or managed-runtime operation files.

### 2. llama.cpp Active Model Proof

- [x] Extend `LlamaServer`/gateway state with a structured active-runtime
      descriptor for mode, port, model path, mmproj path, and device.
- [x] Verify reused runtimes against that descriptor before reporting ready.
- [x] Add wrong-model and wrong-mode reuse regressions.

Status: Complete for the server/gateway descriptor slice. `LlamaServer`
projects a ready managed llama.cpp sidecar into
`LlamaCppActiveRuntimeDescriptor`; the backend trait and gateway expose that
descriptor, and the runtime reuse matchers now compare against it.

### 3. Scheduler Diagnostics Ownership

- [x] Move lifecycle phase emission behind a single runtime-load owner.
- [x] Emit `load_completed` only after requested-model-active proof.
- [x] Convert spawn/HTTP/model mismatch failures into terminal run failures.

Status: Complete. Workflow-service runtime-load lifecycle event construction
now flows through `session_runtime_load_lifecycle.rs` instead of being hand-built
in the scheduler admission path. Runtime-load failures still record canonical
diagnostics, emit `load_failed`, release the scheduler reservation, and finish
the run as failed. `load_completed` is gated behind an additive host-boundary
`WorkflowSessionRuntimeLoadProof`; generic admission success still only records
dependency resolution. The embedded runtime returns proof for llama.cpp only
when the active descriptor confirms inference mode and the requested GGUF path.

### 4. Verification

- [x] Runtime phase contract slice:
      `cargo test -p inference runtime_load`
- [x] Runtime phase contract slice:
      `cargo check -p inference`
- [x] Active runtime descriptor slice:
      `cargo test -p inference active_runtime_descriptor`
- [x] Active runtime descriptor slice:
      `cargo test -p inference inference_runtime_matcher_requires_matching_port`
- [x] Scheduler lifecycle owner slice:
      `cargo test -p pantograph-workflow-service workflow_execution_session_run_records_snapshot_before_execution`
- [x] Scheduler terminal load failure slice:
      `cargo test -p pantograph-workflow-service workflow_execution_session_runtime_load_failure_records_canonical_error`
- [x] Scheduler lifecycle owner slice:
      `cargo check -p pantograph-workflow-service`
- [x] Runtime load proof slice:
      `cargo test -p pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`
- [x] Runtime load proof slice:
      `cargo check -p pantograph-embedded-runtime`
- [ ] Full llama.cpp/runtime verification:
      `cargo test -p inference llamacpp`
- [ ] `cargo test -p pantograph-workflow-service session_execution`
- [x] `cargo test -p pantograph-embedded-runtime session_runtime`
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] `bash launcher.sh --build-release`

## Execution Notes

- 2026-05-09: Runtime phase contract slice added
  `crates/inference/src/runtime_load.rs` with `RuntimeLoadPhase`,
  `ManagedRuntimeLoadFacts`, `RuntimeLoadCommandFacts`,
  `RuntimeLoadPhaseRecord`, `LlamaCppActiveRuntimeDescriptor`, and
  `RuntimeLoadReadinessError`. The slice projects backend `ResolvedCommand`
  facts into the phase contract and rejects missing runtimes, partial installs,
  and active mutating managed-runtime jobs before process readiness can be
  claimed. This is intentionally pure contract/projection code; process spawn,
  HTTP probing, scheduler emission, and runtime registry policy remain in
  their current owner modules for later milestones.
- 2026-05-09: Verification passed:
  `cargo test -p inference runtime_load`, `cargo check -p inference`, and
  `cargo fmt --all -- --check`.
- 2026-05-09: Active runtime descriptor slice added
  `LlamaServer::active_runtime_descriptor`, exposed it through the backend
  trait and `InferenceGateway`, and changed llama.cpp runtime reuse matching
  to compare against the structured descriptor. The descriptor includes mode,
  port, model path, optional mmproj path, device config, context size, and
  llama.cpp performance knobs so reuse decisions remain as strict as the
  existing direct matcher.
- 2026-05-09: Verification passed:
  `cargo test -p inference active_runtime_descriptor`,
  `cargo test -p inference inference_runtime_matcher_requires_matching_port`,
  `cargo check -p inference`, and `cargo fmt --all -- --check`.
- 2026-05-09: Scheduler lifecycle owner slice added
  `session_runtime_load_lifecycle.rs` so load-requested, dependency-resolved,
  and load-failed event construction has one workflow-service owner. The
  generic scheduler model lifecycle writer remains shared for unload events.
  Runtime-load failures continue to terminate the admitted run instead of
  leaving it running. The session execution snapshot test was reconciled with
  the minimal factual artifact model: it now expects the three retained factual
  observations instead of the removed derived `node_input` duplicate.
- 2026-05-09: Verification passed:
  `cargo test -p pantograph-workflow-service workflow_execution_session_run_records_snapshot_before_execution`,
  `cargo test -p pantograph-workflow-service workflow_execution_session_runtime_load_failure_records_canonical_error`,
  `cargo check -p pantograph-workflow-service`, and
  `cargo fmt --all -- --check`.
- 2026-05-09: Runtime load proof slice added
  `WorkflowSessionRuntimeLoadProof` and a default host method for returning
  proof after session runtime load. Workflow-service records `load_completed`
  only when that proof says the requested model is active. Embedded runtime maps
  the llama.cpp active descriptor into proof only for ready managed inference
  sidecars whose active model path matches the requested workflow GGUF.
- 2026-05-09: Verification passed:
  `cargo test -p pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`,
  `cargo test -p pantograph-workflow-service workflow_execution_session_run_records_snapshot_before_execution`,
  `cargo check -p pantograph-embedded-runtime`, and
  `cargo fmt --all -- --check`.
- 2026-05-09: Embedded session-runtime verification exposed a stale fixture:
  `test_session_runtime_load_blocks_when_runtime_preflight_reports_not_ready`
  was not declaring a required backend or failed selected runtime, so the
  preflight path could load successfully in the current runtime capability
  model. The fixture now declares `llama_cpp` and persists a failed selected
  runtime version before asserting the load is blocked.
- 2026-05-09: Verification passed:
  `cargo test -p pantograph-embedded-runtime test_session_runtime_load_blocks_when_runtime_preflight_reports_not_ready`
  and `cargo test -p pantograph-embedded-runtime session_runtime`.

## Completion Summary

### Completed

- Milestone 1 runtime phase contract.
- Milestone 2 llama.cpp active runtime descriptor and reuse checks.
- Milestone 3 scheduler lifecycle owner and terminal runtime-load failure
  behavior.

### Remaining

- Milestone 4: run full cross-crate and release verification after behavior is
  wired through.

## Re-Plan Triggers

- llama.cpp does not expose enough runtime identity to prove model activity
  without an active request probe.
- The current gateway abstraction cannot return structured active-runtime
  descriptors without breaking existing backend implementations.
- Process lifecycle diagnostics require changes to Tauri task ownership.
