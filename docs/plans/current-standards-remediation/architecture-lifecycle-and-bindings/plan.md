# Plan: Architecture, Lifecycle, And Bindings Remediation

> Superseded on 2026-09-08 by the [domain architecture and multimodal plan](../../domain-architecture-and-multimodal/plan.md).
> The remaining body is historical scope/evidence, not implementation authority.
> Outstanding claims and findings transfer to the successor; none are accepted by supersession.

**Plan status:** `Superseded`

**Current phase:** Superseded by the domain architecture and multimodal plan.

**Next slice:** `none`

**Acceptance status:** `pending`

**Execution ledger:** [execution-ledger.md](execution-ledger.md)

**Issues:** [issues.md](issues.md)

**Reports:** `none`

**Related ADRs:** [ADR index](../../../adr/README.md)

**Source audit:** [Architecture, lifecycle, and bindings audit](../../../audits/2026-09-03-current-standards/02-architecture-lifecycle-and-bindings.md)

## Objective

Make the scheduler/runtime-host path the only owner of runtime-backed workflow execution, make each retained native binding a thin adapter over that authority, and give spawned processes and shutdown work explicit, observable lifecycles.

The intended ownership flow is:

```text
native host -> runtime API -> workflow service -> scheduler -> task worker
                                                       | runtime task
                                                       v
                                             RuntimeHostExecutionPort
                                                       |
                                                       v
                                                embedded runtime host

                                             non-runtime task
                                                       |
                                                       v
                                          node-engine graph semantics
```

This is a deletion-led cutover. It does not preserve an alternate direct-execution path as a fallback.

## Scope

- Remaining direct inference ownership in `node-engine` and embedded-runtime call sites.
- Rustler and UniFFI API surfaces, runtime ownership, boundary parsing, and generated-host contract tests.
- Standard and Tauri process adapters, their reader/monitor tasks, and event backpressure.
- Workflow cleanup, dependency-readiness, embedded-runtime, Tauri, and UniFFI shutdown propagation.
- Documentation and dependency edges directly changed by those removals.

## Non-goals

- PyTorch/Diffusers dynamic-code trust policy or GPU/image-generation acceptance. The security remediation owns those.
- A new inference backend, workflow feature, or binding transport.
- Whole-repository CI, hook, or pre-existing Clippy cleanup. The verification/tooling remediation owns the global baseline.
- Release versioning, platform-support policy, or packaged-artifact provenance.
- A universal shutdown result type. Each lifecycle owner needs an operation-specific typed outcome.
- Compatibility shims for an unverified consumer.

## Constraints And Assumptions

- The accepted architecture makes workflow sessions and the scheduler the only public runtime-work admission path; `RuntimeHostExecutionPort` is the runtime-task Seam.
- `node-engine` retains graph semantics and non-runtime task execution. It must not depend on inference backends after M3.
- Rustler and UniFFI are unpublished workspace crates. Current in-repository evidence shows the BEAM host using graph/registry operations and generated C# hosts using `FfiPantographRuntime`; external consumers remain `unavailable` until the milestone inventories record them.
- Rustler 0.34 does not establish a host-owned async runtime Interface for the current exported work. The plan therefore removes unproved async exports instead of creating another private runtime.
- UniFFI's generated-host contract is migrated atomically with Rust exports, generated declarations, examples, and representative C# evidence.
- The standard and Tauri process implementations are two real Adapters at the `ProcessSpawner` Seam. Process ownership must cover the child, IO readers, monitor, cancellation, and drain.
- Shared error vocabulary comes from workflow-service/runtime-host diagnostics. This plan does not create a competing cross-layer taxonomy.
- Implementation is serial. Active image-generation work must integrate or hand off shared workflow-service, embedded-runtime, node-engine, inference, and Tauri files before they are edited here.
- Global CI/Clippy failures outside affected files do not satisfy or waive this plan's gates; verification/tooling owns the repository-wide baseline.

## Preconditions and ordering

1. Integrate or explicitly hand off active work from `current-image-generation-graphs` on workflow-service, embedded-runtime, node-engine, and Tauri paths. Do not edit those shared paths concurrently.
2. Confirm, in the ledger, whether Rustler and the legacy UniFFI engine have any supported external consumers. The current evidence is `publish = false`, a BEAM smoke host that exercises only graph/registry operations, and C# hosts that use `FfiPantographRuntime`.
3. If a public compatibility promise or an external consumer is found, stop before deletion and re-plan an atomic migration with that consumer as required-real contract evidence.
4. Reuse the workflow-service/runtime-host error vocabulary. Do not introduce a second cross-layer error taxonomy.

Implementation is serial in milestone order. Later milestones rely on earlier authority removal and intentionally overlap some composition roots.

## Objective Acceptance

| ID | Acceptance criterion | Evidence kind | Environment | Mode | Status | Evidence |
|---|---|---|---|---|---|---|
| ALB-01 | Every supported runtime-backed workflow run is admitted through a workflow session and scheduler and reaches inference only through `RuntimeHostExecutionPort`; `node-engine` retains graph semantics and non-runtime task execution only. | `integration` | `simulated` | `automated` | `pending` | pending |
| ALB-02 | The declared Rustler surface neither creates/owns a Tokio runtime nor uses `block_on` or detached callback work; retained operations reject malformed/unknown input with typed boundary errors and work in the real BEAM host. | `contract` | `representative` | `automated` | `pending` | pending |
| ALB-03 | UniFFI exposes the application-lifetime `FfiPantographRuntime` surface only; checked conversions and typed errors cross generated C# bindings, and no unbounded/silent event buffer remains. | `contract` | `representative` | `automated` | `pending` | pending |
| ALB-04 | Both standard and Tauri process adapters return one owner for the child and all reader/monitor tasks, use non-blocking async process launch, define bounded event behavior, and report cancellation/drain failures. | `system` | `representative` | `automated` | `pending` | pending |
| ALB-05 | Cleanup, readiness, embedded-runtime, Tauri, and UniFFI shutdown close admission, signal owned work, observe completion, and distinguish complete, incomplete, and failed outcomes in the representative generated-C# path. | `integration` | `representative` | `automated` | `pending` | pending |
| ALB-06 | Retired direct-execution symbols, feature edges, binding exports, and adapters are absent, while retained non-runtime and host-binding behavior continues through its declared Interface. | `focused` | `not-applicable` | `automated` | `pending` | pending |

Evidence is valid only when its exact command, environment, result, and implementation revision are recorded in the execution ledger. A compile-only check supports, but cannot prove, behavioral claims.

## Binding Decisions

| Decision | Rule |
|---|---|
| Execution seam | `RuntimeHostExecutionPort` is the one runtime-task seam. `node-engine::execute_core_task_once` may remain only for non-runtime graph work. |
| Rustler | Retain only synchronous, host-safe graph/registry operations demonstrated by the BEAM host. Remove unproved executor, inference-gateway, async orchestration, callback, Pumas-runtime, extension-runtime, and frontend-HTTP runtime ownership. Do not replace them with another private Tokio runtime. |
| UniFFI | Retain `FfiPantographRuntime` and scheduler/session operations used by generated C#. Remove the legacy `FfiWorkflowEngine` and `BufferedEventSink` if the precondition inventory confirms no consumer. |
| Process execution | Deepen `ProcessSpawner`: the returned handle owns the child, stdout/stderr readers, monitor, cancellation, and drain outcome. Keep the standard and Tauri adapters because both are real composition points. |
| Shutdown | Preserve typed lifecycle results through every adapter. Logging an error is not a successful shutdown result. |
| Compatibility | Treat unpublished in-repository bindings as internally coordinated until contrary evidence is recorded. Make code, declarations, generated bindings, tests, and docs change atomically. |

## Systemic Finding Audit

Before changing a representative occurrence, search and classify the whole bounded population:

| Population | Required disposition |
|---|---|
| `CoreTaskExecutor`, `.with_gateway`, inference-node features, raw graph execution | Route through scheduler/runtime host, retain explicitly as non-runtime, or delete. No unclassified occurrence. |
| `Runtime::new`, runtime fields, `block_on`, `spawn`, Rustler callbacks/dirty schedulers | Remove from binding ownership or prove the host lifecycle and completion contract. No detached success acknowledgement. |
| `serde_json` defaults/fallbacks and integer casts at binding boundaries | Replace with checked decode/conversion and typed rejection. |
| `try_write`, ignored send results, unbounded event collections | Delete with the retired surface or define capacity, overflow behavior, and observable failure. |
| `ProcessSpawner`/`ProcessHandle` implementations and consumers | Migrate both real adapters and every compiler-identified consumer to the owned lifecycle. |
| `shutdown`, cleanup worker, readiness producer, auto-resume task | Classify admission close, signal, drain, timeout, repeated-call, and error propagation behavior. |

The executor, binding, and process abstractions are real seams because each has multiple current consumers or adapters. No new abstraction should be added unless it removes duplicated ownership and passes the deletion test: removing one adapter must not require changing domain policy.

## Simplicity And Ownership Review

**Applicability:** `applicable`

- Independent concepts and dimensions: workflow admission, graph semantics, runtime-task execution, native-host adaptation, process supervision, event delivery, and shutdown change for distinct reasons and retain distinct owners.
- State, identity, value, time, policy, and mechanism: workflow/session identity and scheduler state govern admission; execution values cross the runtime-host Seam; process identity covers the child and owned tasks; time matters to cancellation, drain, and shutdown; workflow/runtime policy remains outside Rustler, UniFFI, and process Adapters; Tokio, BEAM dirty scheduling, UniFFI futures, channels, and Tauri process APIs are mechanisms.
  - **Canonical authority scope and referenced authorities:** workflow service owns admission and scheduler state; runtime host owns concrete runtime execution; `node-engine` owns graph semantics/non-runtime work; each binding references those authorities; each lifecycle owner owns its completion result.
  - **Version roles and owned promises:** crate/package versions, generated UniFFI contract identity, BEAM NIF exports, workflow revision/session identity, and process protocol values are distinct. Release/documentation owns public version promises; this plan owns coordinated internal contract replacement only.
  - **Supported compatibility overlaps and consumer matrix:** BEAM graph/registry calls and generated C# `FfiPantographRuntime` are proven consumers; legacy Rustler async/direct resources and `FfiWorkflowEngine` have no proven external consumer. A discovered consumer changes the migration contract and triggers re-planning.
  - **Material identity-invalidation effects:** removing an export invalidates its generated/declaration consumer; workflow revision/session changes invalidate stale execution requests; process exit/cancellation ends its IO/monitor lifecycle; shutdown closes admission and invalidates further host use.
- Caller and composition-root knowledge: workflow callers know sessions and typed results, not gateway/runtime selection; bindings know transport conversion and host lifecycle, not workflow policy; embedded/Tauri composition roots construct the selected runtime and process Adapters and retain their owners.
- Representative change paths and forced owners: a new runtime-backed task changes runtime-host execution and its worker contract, not `node-engine` inference branches; a binding representation change updates one Adapter and its real host contract; a process lifecycle change updates the `ProcessSpawner` Interface, both Adapters, and direct consumers; a shutdown change propagates only through owners that expose its result.
- Stable Interfaces versus hidden knowledge: stable Interfaces are workflow sessions, `RuntimeHostExecutionPort`, retained binding methods, and the deepened process owner. Backend selection, Tokio handles, reader tasks, channel capacity, JSON layout, and generated-language mechanics remain hidden implementation knowledge.
- Independent evolution, testing, failure, and replacement: graph semantics test without inference; runtime execution tests through the host port; BEAM and C# Adapters have separate real-host evidence; standard and Tauri process Adapters share lifecycle contract tests and retain adapter-specific system evidence; each shutdown owner reports its own typed terminal state.
- Necessary complexity and containment: scheduler admission, cross-language conversion, process supervision, and coordinated shutdown are inherent. They are contained in deep Modules with small Interfaces rather than repeated in graph nodes or binding entry points.
- Deletion and cumulative machinery result: delete direct execution, private binding runtimes, obsolete UniFFI engine/event buffering, detached process tasks, and erased shutdown results. Retain one runtime-task Seam, two justified native Adapters, two justified process Adapters, and operation-specific lifecycle results; no compatibility layer or universal lifecycle framework is added.

## Milestones

### M1 — Cut remaining embedded workflow routes over to scheduler ownership

**Goal:** Remove supported whole-workflow execution that bypasses session admission while preserving non-runtime task execution.

**Allowed write set:**

- `crates/pantograph-workflow-service/src/workflow/non_runtime_task_adapter.rs`
- `crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`
- `crates/pantograph-workflow-service/src/workflow/service_config.rs`
- `crates/pantograph-workflow-service/src/workflow/tests/`
- `crates/pantograph-workflow-service/src/scheduler/task_orchestrator.rs`
- `crates/pantograph-workflow-service/src/scheduler/task_orchestrator_tests.rs`
- `crates/pantograph-embedded-runtime/src/workflow_execution_session_execution.rs`
- `crates/pantograph-embedded-runtime/src/embedded_edit_session_execution.rs`
- `crates/pantograph-embedded-runtime/src/embedded_data_graph_execution.rs`
- `crates/pantograph-embedded-runtime/src/embedded_workflow_host.rs`
- `crates/pantograph-embedded-runtime/src/runtime_host_execution_port.rs`
- `crates/pantograph-embedded-runtime/src/workflow_service_composition.rs`
- `crates/pantograph-embedded-runtime/src/lib.rs`
- `crates/pantograph-embedded-runtime/src/lib_tests/`

**Work:**

- Inventory public embedded entry points and name each as scheduler-routed, non-runtime-only, or retired.
- Make every retained runtime-backed entry point create/use the canonical workflow session and scheduler path.
- Delete raw edit/data graph execution routes that cannot satisfy the session contract; do not emulate them behind a compatibility wrapper.
- Add tests that fail if a runtime-backed task reaches the non-runtime adapter or executes before scheduler admission.

**Gate:**

```bash
cargo test -p pantograph-workflow-service --lib
cargo test -p pantograph-embedded-runtime --lib
cargo check -p pantograph-embedded-runtime --no-default-features
cargo check -p pantograph-embedded-runtime
```

**Rollback/deletion:** Revert the coherent cutover if the session path regresses; do not restore two active authorities. No persisted-data migration is expected.

**Status:** `Planned`

### M2 — Retire Rustler-owned execution runtimes

**Goal:** Reduce Rustler to a thin, synchronous adapter surface with real BEAM evidence.

**Allowed write set:**

- `crates/pantograph-rustler/`
- `bindings/beam/`
- `scripts/check-rustler-beam-smoke.sh`
- `Cargo.lock` only for dependency-edge changes

**Work:**

- Complete and record the consumer/export inventory before deletion.
- Remove binding-owned Tokio runtimes, `block_on`, detached demand/callback acknowledgement, and direct inference/workflow executor resources.
- Remove the now-unreachable inference, Pumas, extension-runtime, and frontend-HTTP feature/dependency edges.
- Keep declared Elixir functions, Rust NIF exports, tests, and BEAM documentation in lockstep.
- Add malformed JSON, unknown variant, integer overflow, and declaration/export parity tests for retained inputs.

**Gate:**

```bash
cargo test -p pantograph_rustler
cargo check -p pantograph_rustler --all-features
./scripts/check-rustler-beam-smoke.sh
```

**Rollback/deletion:** Revert the entire binding surface change if the representative host fails. A discovered external async consumer is a re-plan trigger, not a reason to restore private runtimes.

**Status:** `Planned`

### M3 — Remove node-engine inference authority

**Goal:** Make `node-engine` independent of inference backends while retaining graph semantics and the non-runtime execution adapter.

**Allowed write set:**

- `crates/node-engine/`
- `crates/pantograph-embedded-runtime/Cargo.toml`
- `src-tauri/Cargo.toml`
- `ARCHITECTURE.md`
- `Cargo.lock` only for dependency-edge changes

**Work:**

- Delete `CoreTaskExecutor` runtime/inference branches, gateway injection, fallback inference, unload ownership, and obsolete tests.
- Remove `inference-nodes`/`pytorch-nodes` features and the `inference` dependency from `node-engine`; remove downstream feature requests.
- Preserve and test non-runtime task behavior without importing inference types.
- Update the architecture description and crate metadata that still describe direct execution.

**Gate:**

```bash
cargo test -p node-engine --lib
cargo check -p node-engine --no-default-features
cargo tree -p node-engine -e normal
cargo check -p pantograph-embedded-runtime
cargo check --manifest-path src-tauri/Cargo.toml
```

The recorded `cargo tree` must contain no `inference` edge beneath `node-engine`; a source scan must show no retired gateway/features in the allowed write set.

**Rollback/deletion:** Revert as one dependency/API slice. Do not leave dormant direct-execution code behind feature flags.

**Status:** `Planned`

### M4 — Narrow and harden the UniFFI contract

**Goal:** Expose one application-lifetime runtime to generated hosts and remove the obsolete local workflow engine/event bridge.

**Allowed write set:**

- `crates/pantograph-uniffi/`
- `bindings/csharp/`
- `scripts/check-uniffi-embedded-runtime-surface.sh`
- `scripts/check-uniffi-csharp-smoke.sh`
- `scripts/package-uniffi-csharp-artifacts.sh`
- `Cargo.lock` only for dependency-edge changes

**Work:**

- Record the generated/API consumer inventory; then delete `FfiWorkflowEngine`, `BufferedEventSink`, and their direct execution dependencies if the precondition holds.
- Replace JSON/default fallbacks and unchecked numeric conversions on the retained surface with typed boundary errors.
- Keep generated C# declarations, native metadata assertions, examples, package docs, and Rust tests synchronized.
- Add tests for malformed envelopes, unknown discriminants, over-range values, and repeated runtime use.

**Gate:**

```bash
cargo test -p pantograph-uniffi
./scripts/check-uniffi-embedded-runtime-surface.sh
./scripts/check-uniffi-csharp-smoke.sh
```

The GPU/diffusion smoke is explicitly not an ALB claim; it remains owned by the security/image plans.

**Rollback/deletion:** Revert native API, generated declarations, host sample, and docs together. No dual UniFFI runtime authority may remain after rollback or completion.

**Status:** `Planned`

### M5 — Make spawned-process lifecycle one owned module

**Goal:** Have one process handle own launch, IO forwarding, monitoring, cancellation, and drain for both standard and Tauri adapters.

**Allowed write set:**

- `crates/inference/src/process.rs`
- `crates/inference/src/backend/`
- `crates/inference/src/embedding_runtime.rs`
- `crates/inference/src/gateway.rs`
- `crates/inference/src/server.rs`
- `crates/inference/src/llamacpp_sidecar_events.rs`
- `crates/inference/src/*tests*`
- `src-tauri/src/llm/process_tauri.rs`
- `src-tauri/src/llm/backend/`
- `src-tauri/src/llm/commands/`
- `src-tauri/src/llm/gateway.rs`
- `src-tauri/src/llm/runtime_registry.rs`
- `crates/pantograph-embedded-runtime/src/runtime_host_execution_port.rs`
- affected `Cargo.toml` files and `Cargo.lock` only if the lifecycle implementation changes dependencies/features

**Work:**

- Specify typed spawn, exit, cancellation, IO, and drain errors and an operation-specific shutdown outcome.
- Use async process launch on async paths. Return a handle that retains and observes child, stdout/stderr readers, and monitor tasks.
- Specify channel capacity and overflow/disconnect behavior; never silently discard lifecycle-significant events.
- Update both adapters and every compiler-identified consumer atomically.
- Test normal exit, startup failure, event saturation/disconnect, child kill failure, reader failure, cancellation, and repeated shutdown with a short local child process.

**Gate:**

```bash
cargo test -p inference --features std-process process_owner_contract
cargo test --manifest-path src-tauri/Cargo.toml process_tauri_contract
cargo test -p pantograph-embedded-runtime --lib
cargo check --manifest-path src-tauri/Cargo.toml
```

**Rollback/deletion:** Quiesce/terminate test children before rollback, then revert the trait, both adapters, and consumers as one slice. Do not ship mixed old/new handle semantics.

**Status:** `Planned`

### M6 — Propagate typed shutdown to every lifecycle owner

**Goal:** Make successful shutdown mean that owned work was observed, or return an explicit incomplete/failed outcome.

**Allowed write set:**

- `crates/pantograph-workflow-service/src/scheduler/contracts.rs`
- `crates/pantograph-workflow-service/src/workflow/session_lifecycle_api.rs`
- `crates/pantograph-workflow-service/src/workflow/tests/session_stale_cleanup.rs`
- `crates/pantograph-embedded-runtime/src/dependency_readiness_auto_resume.rs`
- `crates/pantograph-embedded-runtime/src/dependency_readiness_lifecycle.rs`
- `crates/pantograph-embedded-runtime/src/embedded_runtime_lifecycle.rs`
- `crates/pantograph-embedded-runtime/src/workflow_service_composition.rs`
- `crates/pantograph-embedded-runtime/src/lib.rs`
- `crates/pantograph-embedded-runtime/src/lib_tests/`
- `src-tauri/src/app_lifecycle.rs`
- `src-tauri/src/app_setup.rs`
- `src-tauri/src/app_tasks.rs`
- `src-tauri/src/workflow/`
- `crates/pantograph-uniffi/src/runtime.rs`
- `crates/pantograph-uniffi/src/tests.rs`
- `bindings/csharp/`
- `scripts/check-uniffi-embedded-runtime-surface.sh`
- `scripts/check-uniffi-csharp-smoke.sh`

**Work:**

- For each owner, define admission closure, cancellation authority, drain/timeout, repeated-call, and child-error aggregation semantics.
- Retain join handles for cleanup/readiness/auto-resume tasks and observe them during shutdown.
- Propagate typed complete/incomplete/failed results through `EmbeddedRuntime`, Tauri teardown, UniFFI, and generated C# rather than logging and returning success.
- Add active-work, cancellation, panic/error, timeout, and idempotent repeated-shutdown tests.

**Gate:**

```bash
cargo test -p pantograph-workflow-service shutdown_contract
cargo test -p pantograph-embedded-runtime shutdown_contract
cargo test --manifest-path src-tauri/Cargo.toml shutdown_contract
cargo test -p pantograph-uniffi shutdown_contract
./scripts/check-uniffi-csharp-smoke.sh
```

**Rollback/deletion:** Revert the entire propagated result contract together. Before process restart, terminate or drain work from the attempted implementation; do not reinterpret an incomplete result as success.

**Status:** `Planned`

### M7 — Objective verification and deletion review

**Goal:** Prove ALB-01 through ALB-06 and close every issue disposition without adding implementation scope.

**Allowed write set:** this plan directory only.

**Work:**

- Run every milestone gate on the final revision and record exact results/environment in the ledger.
- Run bounded source/dependency scans for every systemic population and record the classified survivors.
- Run affected-package formatting and Clippy gates; do not weaken lints to obtain green output.
- Review each retained adapter against the target ownership flow and apply the deletion test.
- Mark a claim accepted only when its behavioral oracle passes. Link residual unrelated global failures to the verification/tooling plan.

**Final supporting gates:**

```bash
cargo fmt --all -- --check
cargo clippy -p node-engine -p inference -p pantograph-workflow-service -p pantograph-embedded-runtime -p pantograph_rustler -p pantograph-uniffi --all-targets --all-features -- -D warnings
cargo check --workspace --no-default-features
./scripts/check-rustler-beam-smoke.sh
./scripts/check-uniffi-embedded-runtime-surface.sh
./scripts/check-uniffi-csharp-smoke.sh
```

**Status:** `Planned`

## Risks

| Risk/trigger | Response |
|---|---|
| A supported external binding consumer or public compatibility promise is discovered. | Stop before deletion; add the consumer and migration window as required-real acceptance evidence. |
| Active image-generation work still owns a shared file. | Serialize or obtain an explicit handoff; never merge competing authority changes opportunistically. |
| A retained Rustler capability genuinely requires async host work. | Re-plan around a demonstrated Rustler/BEAM lifecycle contract; do not create a private runtime or acknowledge detached work as complete. |
| Process cancellation can interrupt externally visible mutation. | Define cooperative cancellation and safe completion boundaries before implementation; return incomplete when safety cannot be proven. |
| Generated C# cannot represent a proposed typed result. | Adjust the boundary representation while preserving semantics and prove it in the real generated host. |
| Global Clippy/CI remains red for unrelated reasons. | Record the exact unrelated failure and link its owner; do not waive this plan's affected-package gates. |
| Required changes exceed a milestone's write set or introduce a new authority. | Stop and update this plan before editing. |

## Blockers

- `none` for the read-only shared-path and consumer checks at the start of M1.
- Product writes in M1 wait for a recorded handoff or idle state from `current-image-generation-graphs` on shared files.
- Rustler deletion in M2 and legacy UniFFI deletion in M4 wait for their recorded consumer inventories. These are milestone prerequisites, not permission to keep alternate authorities.

## Re-Plan Triggers

- A supported external binding consumer or public compatibility promise is discovered.
- Active image-generation work cannot hand off a shared file without changing this plan's sequence or ownership.
- A retained Rustler capability requires async host work that the demonstrated BEAM lifecycle cannot own.
- Process cancellation can cross an externally visible mutation boundary without a safe completion or compensation contract.
- Generated C# cannot represent the selected typed result without changing public compatibility.
- An affected-package failure reveals a new semantic owner or a file outside the allowed write set.
- A replacement design or cumulative machinery materially exceeds the composed-design admission.

## Cross-plan dependencies

- `current-image-generation-graphs`: shared scheduler/runtime-host implementation must be integrated or handed off before M1.
- `security-and-dynamic-code`: owns Diffusers `trust_remote_code`, model trust decisions, and required-real image/GPU evidence. This plan must not edit that policy.
- `workflow-error-diagnostics-spine`: owns the public diagnostic vocabulary reused at these boundaries.
- `verification-and-tooling`: owns repository-wide command discovery, CI/hook alignment, and the pre-existing global Clippy baseline.
- `dependencies-release-documentation`: owns public version/platform/release promises; a compatibility fact discovered there can trigger this plan's re-plan rule.

## Final Acceptance

- Acceptance status: `pending`
- Deferred follow-ups: `none` within this plan; named out-of-scope standards findings remain with their cross-plan owners.
- Final status: `Superseded`

Set the plan to `Accepted` only when ALB-01 through ALB-06 are satisfied, all in-scope issue dispositions are closed or explicitly superseded, exact evidence is recorded for one final revision, and no direct-execution fallback or silently detached lifecycle owner remains. If an acceptance claim changes, update this plan before further implementation.
