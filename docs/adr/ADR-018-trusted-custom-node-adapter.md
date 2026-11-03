# ADR-018: Trusted custom-node adapter

## Status

Accepted for the bounded host-registered string-transform adapter.

## Context

Node-engine callback registration can populate metadata without supplying the
canonical scheduler's node contracts or immutable implementation identity.
Canonical graph/task derivation used built-ins only. NodeLab is unavailable;
its placeholder UI is not authority to load code. The dynamic-extension research
is a proposal, not an execution contract.

## Decision

The workflow service admits trusted in-process `NodeExecutor` registrations
through one immutable catalog shared by canonical graph/task derivation and
scheduler invocation. Restrict the first adapter to Processing/Batch nodes with
bounded required single string inputs and outputs, no external effects and no
capability/inference declarations. The host explicitly owns callback trust,
nonblocking/cancellation-safe behavior and immutable captured configuration.
There is no arbitrary code loader or sandbox claim.

Bind each registration to canonical contract content, host-attested implementation
and environment digests and adapter version. Pin the complete identity in saved
graphs and scheduler templates. Never authorize execution by palette metadata,
mutable registration replacement, graph-supplied node definitions or task-ID
inference. Versioned topology behavior digests include implementation/environment
identity so version attribution distinguishes those changes.

The managed adapter validates the complete input/output contract, supplies fresh
context/extensions and scheduler-authored invocation correlation, captures
callback failure/panic, and observes cancellation/deadline. Existing scheduler
owners retain attempt state, terminal diagnostics and result commitment. Shared
scheduler state gating arbitrates cancellation against completion. Disposal
attests stopped future polling, not external-effect cessation. ADR-007's embedded
observation/capability guarantee ownership remains unchanged.

## Alternatives and consequences

A full package/plugin ecosystem, Python/native workers and a Wasm runtime would
require independent artifact, isolation and lifecycle obligations. They are
unnecessary for the trusted first slice and are deferred. A new interpreted
language would change the authoring objective without resolving trusted Rust
registration. A low-level callback-only change would leave canonical execution
unusable; the admitted slice includes saved graph → scheduler → validated result.

Hosts can add a node through Rust composition and execute it through scheduler
sessions. New executable code still requires host composition/build; no dynamic
installation or cold reconstruction is promised. NodeLab, edit-session exposure,
tool-loop effects and other value types remain separate admission work.

Affected boundaries: `custom_nodes`, canonical graph registry/behavior versions,
workflow-service session task derivation, non-runtime scheduler invocation,
active-task cancellation and saved/run graph inspection. The stable consumer
contract and limits are in [custom nodes](../custom-nodes.md).
