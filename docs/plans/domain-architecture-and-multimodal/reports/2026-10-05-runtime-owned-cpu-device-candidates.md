# Runtime-owned CPU device candidates

## Scope and acceptance

This is the bounded CPU discovery slice after frozen
`1be6243bb1f5270491994cfe20400ba445702f09`. An unconstrained request can select
the sole eligible registered runtime's owner-advertised CPU device and carry
its canonical runtime variant into selected execution. Explicit runtime and
device constraints remain binding. Missing owner capabilities, unavailable CPU
variants, stale or mismatched source snapshots, and ambiguous alternatives
cannot allocate a speculative lease.

The gateway reads available backend factories' existing capability declarations.
It emits `cpu` only for an available CPU variant whose namespace matches its
canonical backend owner. These are support facts, not loaded-state observations,
dependency readiness proofs, or device reservations. GPU class declarations do
not identify physical devices and produce no automatic device candidates.

The embedded capability source joins these facts to registered backend keys,
including canonical aliases such as `torch`. Model package hints such as
`transformers` remain library labels rather than ownership aliases. Both hosted
composition paths provide their existing gateway to the capability source.
The existing model-scoped source refresh and freshness checks cover the new
facts together with package and load-target facts.

The provider evaluates bound alternatives without acquiring custody. The existing
sole-eligible-candidate policy selects one; the runtime registry then rechecks its
admission observation and acquires provisional custody. Full peak RAM and VRAM
claims remain unchanged. Resident estimates are not subtracted from task claims.
Duplicate bindings fail closed, including duplicate CPU bindings under an explicit
CPU constraint. Other explicitly requested devices retain their existing evidence
and host-verification path; CPU discovery never substitutes CPU for them.

## Verification boundaries

Deterministic regressions exercise owner availability and namespace checks,
projection through registered aliases, stale/model-mismatched snapshots,
unconstrained CPU selection, hard constraints, duplicate and ambiguous choices,
budget rejection, full peak claims, and rollback of untransferred custody.

The selected-text regression reads the actual gateway capability producer,
projects the registered runtime, obtains a provider-backed lease, uses scheduler
selection, and executes through the real embedded host port with a controlled
backend. It checks `pytorch.cpu`, CPU loading, generated output, and preservation
of the complete peak lease until test custody is dropped. Package facts and model
loading are controlled fixtures; this does not claim real model inference.

Native tests use a dedicated target directory to prevent registry artifacts from
the separate PR55 worktree from being reused. Supported mixed-backend embedded
and inference suites and warning-deny Clippy are the qualification entry points.
Final focused runs pass 708 inference tests, 56 dispatch tests, 63 host tests,
and 12 reservation-lifecycle tests. Mixed-backend warning-deny Clippy covers both
affected crates and all targets; format, repository gates and all nine ONNX
no-build-download feature graphs pass.
The full embedded run executes 499 tests: 497 pass and two baseline failures
remain. Both reproduce independently on clean frozen `1be6243`: the text
descriptor regression expects four inputs while the current descriptor has six,
and the session warmup-timeout regression receives success instead of a timeout.
They remain for the parent's integration review. The CPU acceptance regressions
pass without changing either assertion.
GTK/WebKit desktop packaging, GPU discovery, image-specific runtime aliases,
cross-platform native execution, and real model quality are not qualified by this
slice. PR55 source and qualification refs remain separate.
