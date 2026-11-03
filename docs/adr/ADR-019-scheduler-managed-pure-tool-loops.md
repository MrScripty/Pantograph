# ADR-019: Scheduler-managed pure tool loops

## Status

Accepted for the bounded host-admitted nonruntime controller slice.

## Context

CN-01 supplies exact trusted pure string primitives and scheduler invocation
ownership. CN-03 still needs explicit tool authority and bounded progress.
ADR-009 retires static inference/tool expansion; a controller must not call a
model outside scheduler resource admission. CN-02 separately owns authoring.

## Decision

Admit immutable loop registrations alongside their CN-01 primitives in the same
catalog. Require one string input/output per primitive. A loop identity pins a
pure controller, a named map of exact pure string-transform grants, an admitted
turn budget and adapter version. Graphs and typed scheduler templates pin the
complete identity. Per-instance behavior digests distinguish the selected grant
and budget. Graph data, controller output and descriptor metadata grant no code
loading, inference or external-effect authority.

The existing nonruntime scheduler attempt owns serial controller/tool calls,
bounded latest-observation state, generated call identities, one whole-loop
deadline, cancellation and final commitment. Reuse CN-01's prepared invocation
and guarded cancellation/completion arbitration. Publish only complete results;
never retry or resume a partially executed loop. Pure-only capabilities make a
new run safe under the host's existing attestations. Preserve primitive identity
and turn/call correlation without inventing inference ledger evidence.

The exact protocol, limits and unavailable shapes are owned by
[tool loops](../tool-loops.md). Task graph schema 9 replaces the coordinated
internal producer/consumer contract; saved graphs retain exact pins. The stable
descriptor and its unavailable generic execution path remain unchanged.

## Consequences

Hosts can run a saved/reopened `source → tool-loop → sink` graph using a pure
turn policy and explicitly authorized trusted transformations. There is no new
executor trait, registry service, worker runtime or package loader. Callback
purity remains an in-process host obligation, not sandbox enforcement.

Inference-backed turns require a later per-turn scheduler readiness/resource
admission, reservation, runtime dispatch and model/license attribution slice.
Side effects need their own typed authority, interruption/idempotency and
recovery contract. They remain unavailable here. This decision neither changes
PR92 trained-text readiness nor supplies licensed trained-model acceptance.
