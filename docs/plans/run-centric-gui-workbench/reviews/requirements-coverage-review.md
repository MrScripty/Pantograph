# Requirements Coverage Review

## Status

Planning coverage review. No source implementation.

Last updated: 2026-04-27.

## Purpose

Check the run-centric GUI workbench plans against
`../../../requirements/pantograph-gui-run-centric-workbench.md`.

This review records whether the current plan set satisfies the requirements,
where coverage lives, and which gaps or weak spots were corrected during this
pass.

## Verdict

The plan set satisfies the requirements at the planning level after the
corrections in this pass.

Most requirements were already covered by the numbered stages. The weak spots
were not missing architecture; they were details that were implied but not
explicit enough for implementation:

- Scheduler timelines must include relevant `run.*` and `node.*` lifecycle
  events without duplicating those facts as `scheduler.*` event truth.
- Diagnostics and projections must preserve comparison-readiness, not only
  current filtering.
- I/O no-active-run browsing and raw/unknown payload views needed explicit
  plan language.
- Retention settings needed the concrete controls listed by the requirements.
- Network future peer pairing/trust needed to be reserved in the page and API
  model even though Iroh is out of scope.
- API planning needed to explicitly include immutable run submission and
  client/admin queue actions.

## Coverage Matrix

| Requirement area | Coverage | Plan location | Notes |
| --- | --- | --- | --- |
| Scheduler default landing page | Covered | `05`, `06`, `README` | Scheduler-first shell and dense Scheduler page are explicit. |
| Distinct top-level pages | Covered | `05`, `06` | Scheduler, Diagnostics, Graph, I/O Inspector, Library, Network, Node Lab are planned. |
| Drawing-to-Svelte no longer default | Covered | `05` | Plan requires relocation or retirement. |
| Active run shared across pages | Covered | `05`, `06` | Active-run store is frontend transient state; pages consume backend projections. |
| No active-run persistence across restarts | Covered | `05` | Explicit tests required. |
| Useful no-active-run states | Covered after this pass | `05`, `06` | I/O general retained-artifact browsing was made explicit. |
| Dense Scheduler table | Covered | `06` | Columns, sorting/filter/search scaffolding, row selection, and actions are planned. |
| Future/queued/historic runs in one surface | Covered | `02`, `04`, `06` | Requires durable/global run projection, not current session queue snapshots. |
| Scheduler estimates before execution | Covered | `02`, `04`, `06` | Estimates are pre-run records with quality/confidence and API projections. |
| Estimates distinguish expected vs observed facts | Covered | `02`, `03` | Estimate/observation separation and projection comparison are planned. |
| Scheduler derives resources, workflows do not declare authoritative resources | Covered | `02` | Scheduler derives from node/library/runtime/diagnostics facts. |
| Scheduler authority for reservations/load/unload/placement/order/retry/delay | Covered | `02` | Authority and admin/client boundaries are explicit. |
| Scheduler events and model load/unload observability | Covered | `02`, `03`, `../diagnostic-event-ledger-architecture.md` | Events use shared typed ledger. Timeline projections may join `run.*` and `node.*`. |
| Client/session/bucket/admin controls | Covered | `02`, `04`, `06` | Scoped client actions and privileged GUI/admin actions are planned. |
| Stable validated workflow identity | Covered | `01` | Exact grammar remains an open implementation decision. |
| Invalid workflow identity explicit rejection | Covered | `01`, `04` | Error taxonomy and validation tests are planned. |
| Workflow versions from topology plus node versions | Covered | `01` | Node version facts are mandatory for executable workflow versions. |
| Semantic versions plus fingerprints with strict conflict rejection | Covered | `01` | Backend does not infer semantic version intent; fingerprints enforce correctness. |
| Presentation revision separate from diagnostics identity | Covered | `01`, `06` | Historic graph uses presentation revision when available. |
| Run immutability and audit snapshot | Covered | `01`, `02`, `03` | Run snapshot before queueing; cancel/resubmit for changes. |
| Run snapshot context fields | Covered | `01`, `03`, `04` | Includes workflow/node/model/runtime/policy/settings/input/session/bucket fields. |
| Version-aware diagnostics filters | Covered after this pass | `03`, `06` | Input profile/characteristics and comparison-readiness were made explicit. |
| Mixed-version visibility | Covered | `03`, `06` | Facets and warnings are planned. |
| Scheduler decision section in Diagnostics | Covered | `06` | Facts, estimates, choices, delay, load/unload, actions, and observed result. |
| Future comparison support | Covered after this pass | `03`, `04`, `06` | Plans now preserve comparison keys and facets without implementing full comparison workflows. |
| Graph historic run view | Covered | `01`, `04`, `06` | Graph run view uses workflow version/presentation revision, not current editable graph. |
| I/O Inspector workflow/node/intermediate/final data | Covered | `03`, `04`, `06` | Artifact metadata and gallery projections are planned. |
| I/O gallery object types and raw fallback | Covered after this pass | `06` | Text, image, audio, video, table, JSON, file, unknown/raw, metadata-only states. |
| Global retention policy | Covered | `03`, `06` | Global policy, retroactivity, metadata survival, cleanup events. |
| Retention setting controls | Covered after this pass | `03`, `06` | Concrete settings from requirements are now listed. |
| Payload expiration with audit metadata retained | Covered | `03`, `../diagnostic-event-ledger-architecture.md` | Payload retention and audit retention are separate. |
| Library combines Pumas and Pantograph-owned additions | Covered | `03`, `04`, `06` | Library asset categories and Pumas operations are planned. |
| Library highlights active-run assets | Covered | `06` | Active-run asset highlighting and usage summaries are planned. |
| Library facts feed scheduler/diagnostics | Covered | `02`, `03`, `06` | Model sizes/cache/runtime compatibility and historical performance are planned. |
| Pumas/Library audit | Covered | `03`, `04`, `06` | Search/download/delete/access/cache/network/failure attribution. |
| Network exists before P2P | Covered | `04`, `06` | Local instance is first network node. |
| Local Network stats | Covered | `04`, `06` | Local identity, CPU/memory/GPU/disk, runtimes, models, load, queue, cache, health. |
| Future Iroh peer model | Covered after this pass | `04`, `06` | Peer pairing/trust placeholders are reserved; implementation remains out of scope. |
| Node Lab future page | Covered | `05`, `06` | Reserved/disabled/future page state. |
| API support implied by GUI | Covered after this pass | `04` | Immutable run submission and scoped/admin queue actions are explicit. |
| Invariants preserved | Covered | `README`, `00`, `07` | Invariants appear in stage gates and cutover strategy. |
| Revisit triggers | Covered | `README`, numbered plans | Iroh, Node Lab, granular retention, roles, estimates, duplicate workflows, presentation revision history. |

## Corrections Applied

- Updated Stage `02` so scheduler timeline projections include relevant
  `run.*` and `node.*` lifecycle events while preserving event-family
  ownership.
- Updated Stage `03` with input profile/characteristic filters,
  comparison-ready facets, and concrete global retention settings.
- Updated Stage `04` to include immutable run submission, scoped client queue
  actions, privileged admin actions, retained-artifact browsing, and future
  Network pairing/trust projection placeholders.
- Updated Stage `06` to make I/O no-active-run browsing, raw/unknown artifact
  rendering, active-run Network scheduler-decision highlighting, future peer
  trust/pairing placeholders, and comparison-readiness explicit.
- Linked this review from the plan README.

## Remaining Open Decisions

These are acceptable implementation decisions, not requirement gaps:

- Exact workflow identity grammar.
- Exact workflow version registry storage owner.
- Typed diagnostic event ledger crate/database ownership.
- DTO parity mechanism: generated bindings versus paired Rust/TypeScript
  contract tests.
- Initial artifact payload storage location and preview policy.
- First-pass local system metrics source.
- Whether retention pinning is included in the first implementation or remains
  future extensibility.

## Risk Notes

The highest risk to requirement satisfaction remains mixed architecture, not
missing scope. The implementation must avoid:

- using current `graph_fingerprint` as workflow execution identity
- building Scheduler UI from session queue snapshots instead of global durable
  projections
- persisting Tauri workflow events directly as diagnostic ledger events
- letting frontend pages parse raw event payloads as page truth
- storing I/O payloads before retention/privacy policy exists
- creating a separate scheduler event store instead of the shared event ledger
