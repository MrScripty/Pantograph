# Production host-request observation prerequisite

Status: source candidate with portable persistence evidence and compiler-only
native producer evidence. No PR, main merge or external review request.

## Source and bounded admission

Branch `feat/runtime-host-observations` follows selected reservation
`b19961bac2c86e8511a2c809a4823bac04db8c14` and preserves merged main
`7555f193dcd342f18fbd65613a33d9deb073e2b6`. The clean ancestry merge is
`2ff2070dfde7f82be56d8d284e5a32feba51d84d`, tree
`2cf14dd47dd5c4754a2569ffdf7eb128e6b4a655`, with those two ordered parents.
It retains PR44 `ca9edde6850cd58ade0b7e534bb4f58e704c2c64`, PR49 repairs,
PR50–52 and the ordered text/JSON/image feature sources. It does not advance
the frozen graph composition `defc8c5fcafbea07533847b40fbe0cdd253c9909` or
incorporate that composition's distinct-image-ID fixture change. Parent owns
subsequent coherent composition and hosted qualification.

Pumas remains pinned to `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`. Pooling
`495efea606620153fe2f0363b104e56518436a06` remains excluded. No manifest,
lockfile, dependency, network, permission or credentials change.

The coordinator authorized production observation/reservation prerequisites
needed for ranking. This slice owns measurement and bounded durable storage of
single host-request elapsed time, not a ranking algorithm or hardware service
profile. Acceptance: real production producer wiring; exact declared request and
host-epoch matching; failed/canceled/nonterminal/dropped attempts cannot become
successful duration samples; freshness and retention bounds; additive migration,
idempotency/conflict, concurrency and reopening evidence.

## Canonical owners and behavior

The diagnostics ledger owns the new observation DTOs, validation, query/summary
and rolling SQLite journal. Workflow-service shares its existing ledger through
a recorder without retaining the service or host port, avoiding an ownership
cycle. Embedded-runtime owns the measuring wrapper, request fingerprint and
response correlation/outcome classification. Both resource-backed hosted
composition paths attach the wrapper when a diagnostics ledger is configured.
The desktop setup already supplies that ledger; no desktop setting was changed.
A service without a ledger logs observation unavailability and keeps its original
execution port. Recording errors are logged and do not replace execution results;
query errors remain explicit.

Each actual call uses a monotonic `Instant` and records once on completion. A
dropped in-flight future records `Abandoned`, which does not claim cancellation
of backend work. Completed, failed, rejected, acknowledged cancellation/shutdown
and nonterminal outcomes are distinct in stored records. Invalid response
correlation cannot become a successful sample. The original execution result is
preserved and dispatcher validation remains authoritative.

The BLAKE3 fingerprint hashes profile version/scope, declared selected model,
revision/artifact, task type, runtime/variant, device, environment, traits and
exact materialized inputs. Raw prompts, configuration values and artifact refs
are not stored. Attempt/run IDs and leases do not partition the fingerprint.
Each port gets a fresh host epoch; different epochs never match. Missing model
revision stays missing. Declared identity is not proof of immutable model contents,
actual backend mode, physical hardware identity or source-generation stability.

Measured host elapsed time includes resolution, gateway waiting, setup and
publication. Scheduler queue delay, model-load, backend-compute, transfer,
batch-member timing and concurrent external gateway work are unmeasured. They
remain unavailable; no zero values, synthetic inference or throughput claims
stand in for those phases. The summary names host elapsed time explicitly and
uses completed outcomes only. It is not an executable dispatch, reservation proof
or calibrated ranking estimate. Existing coarse run history is not relabeled.

## Persistence and evolution

Schema 27 adds one table and two indexes transactionally. Supported prior schema
states retain the existing forward migrations, then add this table; newer schema
versions remain rejected. The v26 fixture preserves existing data and proves
reopening after recording. No down-migration or automatic rollback to software
that only supports schema 26 is provided; its existing future-version rejection
preserves the database rather than deleting it.

Observation ID uniqueness and a transaction prevent overwrite. Repeating an
identical record is idempotent; changing that identity's payload fails. A competing
SQLite writer may receive a typed conflict/storage error; no stale write is retried.
The same transaction retains only the latest 5,000 observations globally, ordered
by recorded UTC time and ID. This is a rolling observation journal, not the complete
attempt audit. Queries require exact fingerprint/epoch, an explicit inclusive UTC
window and 1–500 samples. Unknown or stale profiles return no successful duration,
without fallback to a different profile. Stored payloads are validated on read.

## Actual verification and limits

- `cargo test --locked -p pantograph-diagnostics-ledger`: 109 existing unit and
  seven new public persistence tests passed. New tests exercise successful-only
  summaries, exact matching/freshness, bounded retention, identity conflicts,
  invalid boundaries, file-backed v26 migration/reopening and real concurrent
  SQLite writers. No ignored tests or failures.
- Ledger Clippy, all targets with `-D warnings`: passed without warnings.
- `ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-workflow-service
  -p pantograph-embedded-runtime --features backend-pytorch --lib --tests`:
  passed without warnings. Five native observer fixtures compile: unchanged
  response/single recording, correlation rejection, outcome classification,
  dropped-future recording and request/epoch/config fingerprint isolation.
  These fixtures were not linked or executed here and do not perform inference.
- Formatting/whitespace, critical/accessibility and scheduler-only surface gates,
  and traceability checker tests passed (27 accessibility; 28 traceability tests).
  Production dependency audit: zero vulnerabilities.
- Staged and committed-range traceability run before upload; semantic source
  review remains required. Frontend sources are unchanged and tests were not
  rerun for this slice.

Logs: `/workspace/pantograph-cache/host-observations-*.log`. Standards inspected at
`366c1d90a24bbfb50973f62b155a5f3396c0f107`, including persistence/evolution and
verification profiles. Existing Rust 1.92.0 toolchain, regular speed, one Cargo
build job. Source-upload approval authorizes only this new branch; no frozen
refs, PRs or reviewed branches are advanced.

No production observer/model run, GUI, desktop IPC, GPU, real text/image model,
backend phase calibration or mixed-model workflow was executed. ORT remains
owner-deferred with no further download/link attempt or workaround. Available
source qualification continues independently of that cloud limitation. Parent
owns native/hosted qualification and coordinated review/integration. Safe learned
completion ranking still needs actual phase/source/hardware/configuration evidence
and a matching ready-input context; this host-wall-time journal does not satisfy
those prerequisites by itself or authorize an arbitrary winner.
