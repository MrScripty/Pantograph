# Trusted custom-node execution

Pantograph's Rust workflow service supports one bounded custom-node adapter.
A host registers a trusted `node_engine::NodeExecutor` with a canonical
`NodeTypeContract` using `TrustedCustomNode::admit`, builds an immutable
`CustomNodeCatalog::try_new`, and constructs a fresh
`WorkflowService::with_trusted_custom_nodes`. The public types live in
`pantograph_workflow_service::custom_nodes`.

Only Processing/Batch nodes with 1..8 required, single-cardinality string ports
in each direction are admitted. Capabilities, inference contracts, port
constraints/options, duplicate port IDs and collisions with any built-in node
name are rejected. Each string is at most 2048 UTF-8 bytes; each input/output
map totals at most 8192 value bytes. Unknown, missing, oversized and non-string
values fail before invocation or result commitment. There is no implicit
coercion, retry or direct workflow-run entrypoint.

## Trust and identity

This is trusted in-process execution. Registration is a host composition action;
workflow JSON, palette metadata and node names cannot install executable code.
The host attests that callback code, captured configuration and environment are
immutable for the catalog lifetime. Callbacks must be nonblocking, safe to drop,
free of external effects and must not start detached work. Rust callbacks can
violate these obligations; this adapter is neither a sandbox nor an enforcement
boundary for malicious code. Blocking work and third-party packages require a
separately admitted isolated adapter.

The host supplies implementation and environment identities as `blake3:` plus
64 lowercase hexadecimal digits. These are host attestations, not verified
measurements of a closure or environment. Admission computes the canonical
contract digest itself, ignoring any supplied contract digest, and derives an
execution behavior digest from the contract, attestations and adapter version.
Changing either attestation or the contract changes execution identity.

Every graph node pins the complete returned `CustomNodeIdentity` in
`node.data.custom_node_identity`. The scheduler template pins the same identity;
execution resolves exact equality against the same catalog used at derivation.
Dynamic `node.data.definition` overrides are rejected. A missing registration,
missing/stale pin or mismatch fails closed. Catalog entries cannot be replaced
or merged. A changed implementation requires a new service/catalog and an
explicit graph pin update. Saved graphs retain pins across reopen; a digest
identifies a registration but does not provide its executable implementation.
No cold replay or code reconstruction is promised.

## Invocation and lifecycle

The adapter invokes the admitted executor with a fresh `graph_flow::Context`
and `ExecutorExtensions`. Extensions contain a scheduler-authored
`CustomNodeInvocation` at `CUSTOM_NODE_INVOCATION_EXTENSION`, with workflow,
run, task and attempt IDs. Caller `_data`, task-ID patterns and persisted graph
fields cannot supply those execution authorities. The callback implements domain
logic; existing scheduler task/attempt transitions own baseline lifecycle events.
This correlation extension does not replace embedded runtime observation contexts
or assert complete effect mediation under ADR-007.

Callback construction and polling are covered by panic capture. Callback failure,
panic, invalid output, cancellation and timeout return distinct internal typed
outcomes and no accepted outputs. Callback error text is omitted from returned
errors; the process panic hook still follows the host's logging policy.
The adapter's cooperative deadline is 30 seconds, using Tokio's monotonic timer;
a shorter workflow timeout still applies. A blocking callback can prevent polling
of cancellation/deadline, which is why such callbacks are outside this contract.

The existing `workflow_cancel_active_execution_session_task` operation also
accepts an active custom task. It validates the active run/task/attempt before
requesting cancellation. Pre-cancellation avoids invoking the callback;
in-flight cancellation drops its future. Cancellation requests and completion
share the scheduler state gate: an acknowledged cancellation cannot commit a
result. A request arriving after completion is refused. Prepared invocation
registrations are removed on completion, failure or outer workflow timeout/drop.
Cancellation proves that the adapter stopped polling and disposed of the future;
it does not prove that external effects ceased or rolled back.

## Supported path and follow-on work

The supported slice is a host-registered, saved/reopened source → custom string
transformation → sink graph executed through scheduler sessions. The service's
saved-graph inspection and run/version inspection use its admitted contracts.
Default service construction remains built-in-only. Task graph schema version 8
adds the `custom_node` non-runtime template; identity adapter version 1 owns this
bounded contract. Coordinated Rust producers/consumers are updated together;
older implementations cannot execute that template.

## NodeLab graph authoring

NodeLab authors workflows from implementations already admitted by its host. It
reads `custom_node_authoring_catalog` (wire version 1), a narrow projection of
canonical labels, required string port IDs and complete immutable identities.
The projection's `trusted_in_process` profile carries the trust obligations above;
it is not an installation or execution permission. Default desktop composition
has an empty catalog and says so. Host maintainers register implementations in
Rust before service construction; browser clients cannot register or load code.
The legacy generic palette remains built-in-only.

`create_custom_node_draft` requires the complete selected identity. The service
resolves exact equality before creating a session, then builds one text source
per required input, the pinned custom node, and one text sink per output. Custom
nodes never receive `node.data.definition`. NodeLab owns a separate edit session,
allows text source edits, displays backend contract diagnostics, and saves/reopens
through existing graph persistence. It provides no execution button; saved graphs
use scheduler sessions. Input text is authored configuration, not tool authority.

The same admitted registry reaches edit-session canonicalization, diagnostics,
connection candidates and commits, including provider-builder construction. New
custom additions require exact pins. General custom data patches cannot change
identity or definitions. Imported/reopened missing or stale pins remain unchanged
and produce blocking diagnostics; explicit new authoring is the recovery path.
Name-only insert-and-connect/insert-on-edge APIs cannot express a selected complete
identity, so custom insertion there is explicitly unsupported and omitted from
creation suggestions. Adding a pinned node and connecting existing endpoints is
supported.

Failed create/reopen preserves the current draft. Replacement and page teardown
close owned sessions, including late creation completion. Transport failures during
close retain exact session IDs in the app-scoped NodeLab service. Pending cleanup
is visible on the next NodeLab mount, blocks new creation/reopening and has an
explicit retry action. No automatic graph-session reap is assumed. A process or
host restart ends these ephemeral sessions; saved files retain their exact pins.

The desktop commands are `get_custom_node_authoring_catalog`,
`create_custom_node_draft` and `get_node_lab_draft`; mutations/persistence reuse
existing commands. Local Agent and runtime publishing remain unavailable or
unsupported. Arbitrary packages still need independently admitted artifact
verification, isolation and lifecycle contracts.

A scheduler-managed tool loop must consume this exact admission and invocation
identity, plus its own typed tool authority, bounded iterations, cancellation,
result and side-effect/idempotency contracts. These string transforms grant no
tool capability. The deliberately retired static loop stays retired. External
workers, Wasm, Python, package discovery, automatic updates, side-effectful tools,
streaming and inference ports require follow-on design admission; none are
inferred from a palette entry.

See [ADR-018](adr/ADR-018-trusted-custom-node-adapter.md) and the
[admitted implementation slice](plans/domain-architecture-and-multimodal/plan.md#admitted-custom-node-adapter-slice-2026-10-21).
