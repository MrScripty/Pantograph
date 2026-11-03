# Read-only reservation evaluation and authoritative commit

Status: implemented as the first production ranking prerequisite, for coordinated
review. Continue candidate discovery/selection on a separate branch after freezing
this contract. No PR, main merge or external review request.

## Admission and contract

The coordinator explicitly resumed production observation/reservation prerequisites
on 2026-10-05. The concrete gap is that `can_acquire_reservation` exposes only a
yes/no result, while embedded candidate discovery reserves each alternative before
ranking. This slice adds useful read-only observations to the current registry
owner; the next slice changes candidate discovery to consume them before selection.

`RuntimeRegistry::evaluate_reservation` validates the immutable request against
the same authoritative admission owner without changing leases, IDs, residency or
retention. The returned evaluation borrows its registry and exposes only shared
references to its request and observation. Dropping it is safe. Its consuming
`commit` revalidates and acquires/updates the selected request under the existing
registry lock. The earlier observation never authorizes spending stale capacity.

The observation reports runtime/instance identity, capture time, requested bytes,
reserved bytes, configured capacity, safety margin and available bytes for each
requested resource kind. Same-owner replacement excludes the old lease and names
that exclusion. Missing capacity and available bytes remain `None`, rather than
pretending `u64::MAX` is measured hardware. Claims are aggregated with the registry's
existing overflow and margin rules. Commit can reject after capacity, runtime state
or owner binding changes; no failed replacement destroys the original claim.

These are advisory in-process observations over the registry's existing per-runtime
budgets. They do not certify global physical domains, immutable model capability,
calibrated service duration or output-progress closure. No transport/persistence
schema, runtime feature or dependency is changed. The existing embedded resource
reservation consumer now uses evaluate→commit, preserving its typed outcomes.

Acceptance: alternatives leave state and lease IDs untouched, exactly one concurrent
80/100-byte commit succeeds, stop/replacement races reject safely, unknown capacity
stays unknown, and exact aggregated claims/margins and owner/overflow errors retain
their current admission semantics. The seven public-API tests execute real registry
state and thread contention rather than mocks or a scheduler simulator.

- Branch: `feat/runtime-reservation-evaluation`.
- Base/parent: `3fd46ff4a21d881b618fe84a64961036d1476d9f`.
- Base tree: `cdc7b513043f3dbee8d6a8154c883fe04821951c`.
- Frozen PR49–52, graph feature refs and PR50–52 composition remain unchanged.
- Pumas pin: `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`, unchanged.
- Standards: `366c1d90a24bbfb50973f62b155a5f3396c0f107`, including Contracts,
  Verification, Implementation and the Rust API profile.
- Write set: registry export/new evaluation owner, public-API tests, existing
  embedded resource consumer and this report.
- Research: `pantograph_scheduler_thesis.md`, Library identity
  `libfile_6c1d9aa6e1f08191b4a14e0de0846220`, version 1, especially observations,
  non-mutating candidate planning and authoritative commit; evaluation identity
  `libfile_8b6d205bd260819187ef9fbac029a243`. Research is not hardware evidence.

## Actual checks

| Check | Result |
| --- | --- |
| `cargo test --locked -p pantograph-runtime-registry` | 74 unit, 7 new public evaluation and 1 technical-fit contract test passed; no failures or ignored tests. |
| `cargo clippy --locked -p pantograph-runtime-registry --all-targets -- -D warnings` | Passed. |
| Embedded/workflow Clippy with `backend-pytorch`, `--lib --tests`, `ORT_SKIP_DOWNLOAD=1` | Passed compiler check; native test code was not linked or executed locally. |
| Critical/accessibility gates and traceability checker tests | Passed; 27 accessibility and 28 traceability checker tests. |
| Scheduler-only public surface gate; production audit | Passed; zero production audit vulnerabilities. |
| Formatting, whitespace and staged/committed-range traceability | Applied before upload. |

Logs: `/workspace/pantograph-cache/reservation-evaluation-*.log`. Regular speed,
existing Rust 1.92.0 toolchain, one Cargo build job. The owner-deferred ORT limitation
was not retried. No GUI, model inference, GPU, calibration or native workflow session
was executed. No network, credentials or permissions change was made.

This contract does not yet remove speculative candidate leases; that is the next
continuing implementation milestone, not a reason to stop for review. Parent owns
hosted/native checks, coordinated reviews, PRs and integration.
