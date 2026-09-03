# Pantograph Architecture

## Purpose

This document is the durable entry point for understanding Pantograph's
current architecture, target execution model, module ownership, and active
transition. It summarizes stable decisions and links to the detailed ADRs and
implementation plans that own their exact contracts and status.

This is not a milestone checklist. For changing implementation status, use the
status sources in [Where To Find Current Status](#where-to-find-current-status).

## Product Shape

Pantograph is a local-first, Rust-native execution platform for node-based AI
workflows. Its headless modules provide the canonical execution interface. The
Svelte/Tauri desktop application is an optional host and visualization adapter,
and UniFFI, Rustler, and HTTP adapters project the same backend-owned contracts
to other consumers.

The repository is a modular monolith. Most policy and execution code is linked
into one native process, while selected model runtimes and Python-backed model
execution run out of process.

## Current Module Flow

```text
Svelte workbench / @pantograph/svelte-graph
                    |
             typed Tauri commands
                    v
       Tauri composition and adapters
                    |
                    v
       pantograph-workflow-service
      graph sessions, run orchestration,
      artifacts, diagnostics, projections
                    |
          +---------+---------+
          |                   |
          v                   v
 node-engine/workflow-nodes   scheduler/runtime registry
          |                   |
          +---------+---------+
                    v
       pantograph-embedded-runtime
                    |
          inference adapters / Pumas
          managed and external runtimes
```

Headless and binding consumers enter through the Rust workflow and embedded
runtime interfaces rather than through Tauri. Tauri is the desktop composition
root and transport adapter, not the owner of workflow or runtime policy.

## Target Execution Model

Pantograph is replacing its remaining whole-workflow execution assumptions
with durable, scheduler-owned task execution. The canonical target flow is:

```text
workflow run submitted
  -> canonical graph and topology validation
  -> path-free scheduler task graph
  -> durable per-task scheduler state
  -> ready task admission
  -> dependency, resource, batching, runtime, and device policy
  -> dispatch-selected SchedulerRuntimeHandoff
  -> runtime-host or non-runtime task execution
  -> persisted task result, artifacts, and diagnostics
  -> dependent task unblocking
```

The scheduler sees ready workflow tasks, not only whole runs. It may pause one
workflow while resources serve another, batch compatible tasks across runs,
reuse proven runtime/model residency, and resume dependent tasks when their
materialized inputs become available.

Every public workflow run enters through a workflow execution session. Direct
run interfaces are not a supported compatibility path.

## Module Ownership

| Module | Owns | Must not own |
| --- | --- | --- |
| Svelte frontend and `@pantograph/svelte-graph` | Interaction and presentation state, typed user intent, active-run selection | Graph validity, scheduler decisions, runtime readiness, or reconstructed backend state |
| Tauri desktop adapter | Process composition, command/event transport, windows, desktop lifecycle | Workflow, scheduler, model, device, retention, or artifact policy |
| `pantograph-workflow-service` | Workflow use cases, graph sessions, task-graph derivation, run orchestration, artifacts, diagnostics, and user-facing projections | Runtime-specific execution or host transport policy |
| `pantograph-node-contracts` | Canonical node types, ports, effective contracts, compatibility, and rejection diagnostics | Concrete runtime execution |
| `node-engine` and `workflow-nodes` | Graph semantics, built-in descriptors, and materialized non-runtime node execution | Runtime selection, dependency resolution, Pumas load-target lookup, or model-runtime launch |
| `pantograph-scheduler` | Queue, readiness, resource admission, batching, placement, dispatch, retry, and lifecycle policy | Graph authoring, model storage, host process launch, or frontend behavior |
| `pantograph-runtime-registry` | Live runtime state, residency, reservations, retention, and reclaim eligibility | Workflow input or run-local state |
| `pantograph-runtime-host-contracts` | Validated scheduler-to-runtime-host requests, responses, ports, and reservation lifecycle contracts | Scheduler policy or concrete runtime implementation |
| `pantograph-embedded-runtime` | Runtime-host adapters, execution composition, managed capabilities, runtime lifecycle, and Pumas load-target resolution at the host seam | Public workflow policy or frontend behavior |
| `inference` | Backend-neutral inference contracts and concrete backend adapters | Workflow scheduling or graph-authoring semantics |
| Pumas Library | Canonical model, package, artifact, and license facts | Pantograph runtime/device selection or scheduling policy |
| Diagnostics, attribution, and artifact modules | Durable identities, event history, projections, retention metadata, and retained bodies | UI presentation decisions |
| UniFFI, Rustler, and HTTP adapters | Host-safe projections over backend-owned contracts | Independent node catalogs, validation rules, or execution semantics |

## Architectural Invariants

- Backend Rust modules own canonical execution contracts and policy.
- Public workflow execution uses scheduler-backed execution sessions.
- The scheduler owns task admission, runtime/device selection, batching,
  resource decisions, and dispatch.
- Scheduler and workflow contracts carry stable model/artifact identity, not
  local executable or model paths.
- The runtime host resolves Pumas-approved local load targets only after a
  scheduler-selected handoff.
- One backend-owned inference descriptor governs editor ports, validation,
  task materialization, and pre-dispatch checks.
- Missing or stale authority produces typed diagnostics and fails closed; it
  does not select a silent fallback.
- Runtime residency may be reused, but workflow inputs, outputs, cancellation
  state, and other run-local values are never inherited by another run.
- Large media has one retained artifact body. Other layers exchange bounded
  descriptors, references, and attribution metadata.
- Frontend pages consume typed backend projections rather than replaying raw
  diagnostic events or reconstructing backend truth.
- Bindings are adapters over the canonical Rust interfaces and advertise
  support according to verified host-language artifacts and smoke tests.

## The Active Transition

The foundational execution-platform work includes durable attribution,
canonical node contracts, runtime-owned observability, model/license
diagnostics, composed-node contracts, artifact storage, and the run-centric
workbench.

The active transition is completing the scheduler-to-runtime-host path:

- reconcile stale plan status and restore the active test baseline;
- repair backend-owned inference validation for real saved workflows;
- dispatch any valid non-empty group of `1..N` compatible assignments through
  one scheduler-owned batch path;
- carry dependency-readiness proof and resource reservations through durable
  task state into runtime-host execution;
- remove remaining successful `model_path`, `ModelRefV2`, reduced-plan, and
  direct-runtime execution paths;
- prove real PyTorch/Diffusers image generation from the workflow editor to one
  retained artifact displayed in the I/O Inspector.

This transition deliberately replaces stale execution authority rather than
preserving parallel compatibility routes.

## Run-Centric Workbench

The desktop application uses one Scheduler-first workbench. Selecting a
workflow run establishes shared context for Scheduler, Diagnostics, Graph, I/O
Inspector, Library, Network, and Node Editor views.

Those pages read durable materialized projections with projection versions and
event cursors. Historic workflow graphs are immutable run snapshots and remain
separate from the current editable graph.

## Repository Map

| Path | Responsibility |
| --- | --- |
| `src/` | Svelte desktop frontend, workbench pages, typed adapters, and stores |
| `packages/svelte-graph/` | Reusable graph editor modules and interaction policy |
| `src-tauri/src/` | Desktop composition, lifecycle, and Tauri transport adapters |
| `crates/pantograph-workflow-service/` | Host-agnostic workflow application module |
| `crates/pantograph-scheduler/` | Scheduler contracts and pure policy |
| `crates/pantograph-runtime-registry/` | Runtime lifecycle, residency, and reservations |
| `crates/pantograph-runtime-host-contracts/` | Shared runtime-host execution seam |
| `crates/pantograph-embedded-runtime/` | Concrete embedded runtime composition and adapters |
| `crates/node-engine/` | Graph validation and execution primitives |
| `crates/workflow-nodes/` | Built-in node descriptors and implementations |
| `crates/inference/` | Inference contracts, gateways, and backend adapters |
| `crates/pantograph-diagnostics-ledger/` | Durable diagnostic events and read projections |
| `crates/pantograph-runtime-attribution/` | Durable client, session, workflow, version, and run identities |
| `crates/pantograph-uniffi/` | Non-BEAM native binding adapter |
| `crates/pantograph-rustler/` | Elixir/BEAM binding adapter |
| `docs/adr/` | Accepted architectural decisions |
| `docs/plans/` | Active implementation plans and their execution records |

## Where To Find Current Status

Use these sources according to the question being asked:

1. **Architecture overview:** this document.
2. **Accepted decisions:** [`docs/adr/README.md`](docs/adr/README.md), especially
   ADR-006 and ADR-009 through ADR-016.
3. **Current standards baseline:**
   [`docs/audits/2026-09-03-current-standards/README.md`](docs/audits/2026-09-03-current-standards/README.md).
4. **Cross-codebase remediation order:**
   [`docs/plans/current-standards-remediation/plan.md`](docs/plans/current-standards-remediation/plan.md).
5. **Image-generation workflow status:**
   [`docs/plans/current-image-generation-graphs/plan.md`](docs/plans/current-image-generation-graphs/plan.md).
6. **Development and release capability:**
   [`docs/development.md`](docs/development.md) and
   [`docs/release.md`](docs/release.md).

Accepted ADRs are the durable decision record. Active plans own implementation
sequence and changing status. Historical plans and detailed execution narration
are available through Git history, not as competing current authorities.

## Maintenance

Update this document when module ownership, the target execution flow, or a
stable architectural invariant changes. Update the relevant ADR for an accepted
decision and the active plan for milestone-level status. Avoid copying detailed
checklists or transient failure counts here.
