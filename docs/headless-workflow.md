# Headless Workflow Integration

Pantograph's canonical application boundary is the Rust
`pantograph-workflow-service` API. Tauri, UniFFI, Rustler, and the optional HTTP
adapter are projections of that backend-owned contract; they do not own graph,
scheduler, runtime, or diagnostic semantics.

## Composition

```text
host application
  -> native Rust API or language/transport adapter
  -> pantograph-embedded-runtime
  -> pantograph-workflow-service
  -> scheduler and runtime host
```

The target requires every public runtime-backed run to enter through a
scheduler execution session. Some direct and binding-owned execution paths
still exist and are scheduled for removal by the
[architecture remediation plan](plans/current-standards-remediation/architecture-lifecycle-and-bindings/plan.md).

## Recommended Run Flow

1. Construct the application-lifetime runtime at the host composition root.
2. Discover node definitions and workflow I/O from the backend.
3. Run preflight and preserve typed invalid, unsupported, and unavailable
   outcomes;
4. create a workflow execution session;
5. submit work to that session;
6. inspect status, queue, diagnostics, and artifacts as needed;
7. close the session; and
8. shut down the application runtime and observe its terminal result.

Do not create a private async runtime in a language binding, execute the graph
directly through node-engine, or reconstruct backend validation in a client.

## Graph Authoring

Headless consumers can save/load workflow documents, open edit sessions,
perform backend-owned graph mutations, query connection candidates, inspect
undo/redo state, and close the edit session. Treat every returned graph
revision and connection intent as authoritative for only the identity/revision
it names.

## Contract Sources

Exact request, response, error, and lifecycle schemas live with their code:

- `crates/pantograph-workflow-service/src/workflow/contracts.rs`
- `crates/pantograph-workflow-service/src/graph/session_types.rs`
- `crates/pantograph-workflow-service/src/graph/persistence.rs`
- `crates/pantograph-workflow-service/src/scheduler/contracts.rs`
- `crates/pantograph-uniffi/src/runtime.rs`

Consumer examples and packaging notes live beside the binding:

- [C# bindings](../bindings/csharp/README.md)
- [C# direct-runtime quickstart](../bindings/csharp/Pantograph.DirectRuntimeQuickstart/README.md)
- [BEAM/Rustler binding](../bindings/beam/README.md)
- `crates/pantograph-workflow-service/examples/rust_host_workflow_run.rs`

The Rust types are authoritative. Copied JSON examples, generated bindings,
and host wrappers must be regenerated or updated with the producer contract and
must decode runtime input rather than relying on static-language assertions.

## Support Status

- Native Rust is the canonical integration surface.
- C# uses generated UniFFI bindings and an application-lifetime
  `FfiPantographRuntime`.
- Elixir/BEAM uses Rustler projections.
- The optional frontend HTTP adapter is not required for native embedding.
- Python-backed workflow nodes are child-process runtime consumers, not the
  Python host-language binding.

Packaging, platform support, and binding compatibility are not yet established
as release-grade claims. See [Release](release.md) and the current audit.

### Session capacity and keep-alive diagnostics

Explicit keep-alive creation and enablement own session-count capacity eviction.
The host may choose an idle victim; the default policy preserves workflow,
model and backend affinity. Disabling keep-alive or closing the session performs
explicit cleanup. Ordinary non-runtime runs do not load a session runtime, and
runtime-task resource reservations remain a separate admission contract.

Capacity eviction events carry `workflow_execution_session_id` for the target
and `unloaded_workflow_execution_session_id` for the victim, plus the existing
model/runtime facts and shared timing-attempt ID. Session-only operations leave
run attribution absent and do not create run projections. Historical run-scoped
payloads retain their existing decoding and required run identity.

A failed session-runtime admission records the `session_runtime_admission` error
phase under `session_runtime` scope. The returned error retains the operational
code and message; its diagnostic link identifies the recorded error or explains
why diagnostics were unavailable. No workflow-run ID is synthesized. Failed
keep-alive enablement rolls back the flag; failed keep-alive creation removes
the new session. A failed victim unload does not mark that victim unloaded.

Once the host confirms an eviction, session residency is updated before terminal
telemetry is recorded. If that recording fails, admission reports the diagnostic
failure and the caller rolls back its target request, but the evicted session
remains correctly unloaded and can be reloaded on retry. An unload failure leaves
the victim loaded. Missing lifecycle telemetry is reported even when recording
the separate canonical error succeeds; lifecycle error text is sanitized without
changing the original operational error returned to the caller.
