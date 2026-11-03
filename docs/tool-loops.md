# Scheduler-managed pure tool loops

The Rust workflow service can execute the stable `tool-loop` node with a
host-admitted pure controller and exact grants to trusted string transforms.
This is the bounded CN-03 nonruntime slice. Inference-backed agent turns and
side-effectful tools remain unavailable. No model readiness, licensed weights,
isolation or complete effect mediation is asserted.

## Host composition and graph authority

Register the controller and tools using [CN-01](custom-nodes.md). Each must have
exactly one required string input and output. Their existing host attestations
require immutable code/configuration/environment, nonblocking execution, safe
future disposal, no external effects, no inference and no detached work.
The controller is a pure turn policy, not a way to call a model outside runtime
admission. This is trusted in-process Rust, not a sandbox.

Construct `ToolLoopRegistration { id, controller, tools, max_turns }` from the
returned exact `CustomNodeIdentity` values, where `tools` is a sorted map of
capability names to identities. Build
`CustomNodeCatalog::try_new_with_tool_loops(nodes, registrations)` and pass the
immutable catalog to `WorkflowService::with_trusted_custom_nodes`. Missing or
stale primitives, duplicate loop IDs, invalid names, incompatible port counts
and invalid limits fail admission. Capability names/loop IDs are 1..64 ASCII
letters, digits, hyphens or underscores; each primitive identity is at most
1024 serialized bytes. An ordinary CN-01 registration grants no tool authority.

Pin the catalog's complete `tool_loop_identity(id)` in
`node.data.tool_loop_identity`. The graph cannot install or modify a grant.
The identity contains controller, tools, turn limit and adapter version. The
scheduler template pins the same value; per-instance executable behavior hashes
it, so changing implementation, environment, grant or budget changes workflow
version attribution. Saved graphs preserve the pin across reopen, but hosts
must reinstall the exact trusted implementations. There is no cold code replay.

Connect exactly one string source to `prompt`. This slice rejects all optional
authoring ports (`system_prompt`, `context`, `tools`, `max_turns`) and equivalent
node data overrides, including dynamic `definition`. The canonical descriptor
remains stable; its generic `run` path remains unavailable. NodeLab/edit-session
authoring is owned by CN-02; this slice adds no UI or runtime publishing.

## Turn and capability protocol

The controller's single input string encodes `ToolLoopTurnInput`:

```json
{"prompt":"hello","turn":1,"observation":null}
```

Its single output string encodes one strictly decoded `ToolLoopAction`:

```json
{"action":"call_tool","tool":"uppercase","input":"hello"}
```

The named grant is a pure string-transform capability, resolved by exact
identity through the CN-01 invocation adapter. The next controller input carries
only the latest `observation: { "tool": "uppercase", "output": "HELLO" }`.
Completion uses `{"action":"complete","response":"HELLO"}`. Unknown fields,
ungranted names, malformed JSON and invalid payloads fail the whole attempt.

The host admits 1..16 controller turns and 0..8 grants, with at most one tool
call per turn. A final turn cannot start another tool: it must complete or fail
with turn-budget exhaustion. Every primitive input/output and the serialized
controller state is bounded by CN-01's 2048 UTF-8 byte limit. Only the latest
observation and at most 15 bounded call records are retained. Escaping can make
a valid tool string too large for the next turn; that is an explicit invalid
state outcome, not truncation. Controller state, output and tool arguments are
never implicitly coerced.

## Scheduler ownership and results

One existing nonruntime scheduler readiness/start admission owns the bounded
attempt. Controller and tools run serially through its single prepared CN-01
invocation owner; no alternate runtime, detached task, static graph expansion,
retry or direct executor entrypoint is added. This admission is valid only for
the declared bounded pure transforms. Actual inference must go through per-turn
runtime readiness, resource-fit/reservation and model/license attribution;
the loop cannot bypass those contracts by invoking a model callback.

The whole loop has one cooperative 30-second monotonic deadline, including all
turns/tools; a shorter workflow deadline still applies. Cancellation uses
`workflow_cancel_active_execution_session_task` and shares the session-store
commit gate with completion. Acknowledged cancellation prevents accepted
outputs even after callbacks return. In-flight cancellation/timeout/drop disposes
the currently owned future and invocation registration. This proves stopped
polling; it does not prove external-effect rollback for a misbehaving host.

Callbacks receive the scheduler-authored parent workflow/run/task/attempt
extension plus `ToolLoopPrimitiveInvocation` at
`TOOL_LOOP_PRIMITIVE_EXTENSION`, containing loop, turn, exact primitive identity
and controller/tool role. Tool call IDs are minted from attempt and turn.
Success publishes `response`, `turns` and `tool_calls` (bounded call provenance,
including tool identity). Only final success commits results. Failure, panic,
unauthorized calls, exhaustion, invalid state, cancellation and timeout publish
no partial loop outputs or downstream success. Callback error text is redacted
under CN-01; the host's process panic hook remains its logging responsibility.

Task graph schema version 9 adds `tool_loop`; adapter version 1 defines this
protocol. Coordinated Rust producers/consumers move together; older consumers
cannot execute the new template. No saved graph format replacement is needed.
There is no automatic retry or durable mid-loop resume: a new run starts at
turn one, which is permitted only because all admitted tools are pure.

See [ADR-019](adr/ADR-019-scheduler-managed-pure-tool-loops.md).
