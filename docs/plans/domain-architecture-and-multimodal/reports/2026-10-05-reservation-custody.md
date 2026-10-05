# Preparation-to-binding reservation custody

`fix/scheduler-reservation-custody` repairs the coordinator-identified lease gap:
the resource-backed provider previously committed during preparation, before
task start and reservation-intent binding. Ordinary terminal cleanup requires
that intent, so a failed start, cancelled preparation, rejected selection or
failed bind could leave an unowned lease. Final candidate validation also ran
after commit, and an evaluated runtime instance could change before commit.

The exact base is `1f6dc007b6a9ec8202e02edfb990af64657abfed`, tree
`9a485aa9dc4f8d46b0e2d386d7b9943a03ac8ba5`. Its parents are accepted observations
`2d861ba7527764a8c796ac9775ef2b9ea9d69024` and graph composition
`defc8c5fcafbea07533847b40fbe0cdd253c9909`; their clean automatic source merge
tree is `a007754ea5945d9475012da1047562c1b0d0bb6f`. The distinct per-run image
artifact fixture remains present. This branch preserves merged main
`7555f193dcd342f18fbd65613a33d9deb073e2b6`, selected reservation
`b19961bac2c86e8511a2c809a4823bac04db8c14`, evaluation prerequisite
`1f6401b7282cb88404c5174115a5b2cb5f655f57`, and the full PR44 stack through
`ca9edde6850cd58ade0b7e534bb4f58e704c2c64`. Pumas remains exactly pinned to
`2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`; pooling
`495efea606620153fe2f0363b104e56518436a06` is excluded.

## Bounded ownership and failure contract

- Evaluation stays read-only. Both ordinary evaluation commit and provisional
  publication reject changed runtime status/instance under the registry mutex.
  Current capacity, canonical global owner conflicts and legacy stopping/failed
  rejection outcomes remain authoritative.
- The selected provider's synchronous callback builds and validates the complete
  evidence and candidate bundle against a prospective real lease before any
  reservation mutation. Validation errors leave a previous lease unchanged.
  Callbacks must not re-enter the registry or publish external side effects.
- Successful publication returns non-clonable, non-serializable rollback custody.
  Candidate collection, prepared selection and selected evidence carry custody
  until the task's in-memory cleanup intent is bound. Both runner entry paths
  transfer custody after that binding, outside the session-store lock and with
  no intervening await. Normal errors, dropped preparation futures and unwinding
  roll back untransferred custody.
- A provisional replacement retains componentwise maximum old/new capacity and
  the predecessor's pin/keep-alive protection. The held claim must fit the current
  configured budget. Competitors cannot consume capacity promised for rollback.
  Rollback restores the entire prior record, including ID, creation time, owner,
  workflow, model, usage profile, claims and retention metadata. Transfer installs
  the validated replacement and frees old excess capacity.
- Pending same-owner updates and retention mutations are rejected. Explicit
  registry release ends the lease, including its pending lineage; a later custody
  drop cannot revive it or remove a later same-owner lease. Owned preparation
  rejection uses custody rollback rather than ordinary unselected-lease release.
  Fully bound and legacy non-custody leases retain ordinary lifecycle cleanup.

This is in-memory ownership, not durable recovery after process abort. Runtime
status/instance comparison does not prove unchanged Pumas/catalog/configuration
epochs or physical host-wide capacity. Missing capacities remain unknown. There
is no new learned ranking, phase calibration, dependency policy or framework.
Candidate-set cloning/equality is removed because facts do not confer custody;
fixture-only providers explicitly copy data without duplicating a live token.

## Verification and limits

The new stale-instance fixture first **failed** against observations head `2d861`
(whose registry source matches the composition base): evaluation saw instance
001, the runtime changed to instance 002, and commit wrongly succeeded. Actual
baseline result: zero passed, one failed, exit 101. Evidence:
`/workspace/pantograph-cache/reservation-custody-baseline-instance.log`.

- `cargo test --locked -p pantograph-runtime-registry`: 95 passed (74 unit,
  12 custody, eight evaluation and one technical-fit fixture), no failures or
  ignored tests. `cargo clippy --locked -p pantograph-runtime-registry
  --all-targets -- -D warnings` passed.
  Public fixtures cover rejected publication for fresh/replacement leases,
  cancellation of a polled preparation future, fresh-lease drop, full predecessor
  restoration, protected shrinking capacity, transfer, current-budget rejection,
  stale instance, concurrent admission, global owner conflict, explicit release,
  release/reacquire isolation, and unwinding.
- Executed on this source branch before the final ownership-only correction:
  116 diagnostics-ledger, 49 runtime-host (42 unit plus seven compatibility) and
  133 scheduler tests passed. Those crates' source is unchanged by the correction.
- Final dual-feature workflow/embedded consumer Clippy passed without warnings:
  `ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-workflow-service
  -p pantograph-embedded-runtime --features backend-candle,backend-pytorch
  --lib --tests`. This compiles the native test fixtures; it does not execute them.
  Three new workflow fixtures include dropped preparation, invalid intent, start
  failure, cancellation before bind, rejected selection and successful binding.
  Their recording custody is a mock, not inference or a real registry execution.
- `ORT_SKIP_DOWNLOAD=1 cargo check --locked --workspace --all-targets` was attempted
  and failed in desktop dependency `glib-sys`: this cloud environment lacks the
  `glib-2.0 >= 2.70` development package metadata. No workspace-wide success is
  claimed; no system package or permission change was attempted.
- `ORT_SKIP_DOWNLOAD=1 cargo check --locked --workspace --all-targets
  --exclude pantograph` passed, including workflow/embedded consumers, UniFFI,
  Rustler and the HTTP adapter. One unused-field warning remains in unchanged
  `inference/src/selected_text_execution.rs` (`package`, `target`, `device`) under
  this default feature combination; repeated for its test target. This is compile
  evidence only, without native linking, binding execution or inference.
- Formatting/whitespace, critical anti-pattern, accessibility (27 tests),
  traceability checker (28 tests) and scheduler-only public-surface gates passed.
  Production npm audit previously reported zero vulnerabilities; dependency files
  are unchanged. Staged and committed-range traceability are checked before upload
  with 15 changed paths and one mapped ADR/runbook impact. ADR-002 and the operational
  runbook document the ownership contract. Semantic contract review remains required.

Portable logs live at `/workspace/pantograph-cache/reservation-custody-*.log`.
The base composition separately executed 725 inference, 261 node (one existing
ignored) and 659 frontend tests; those counts are **not reruns on this fix**.
Its controlled process IPC and frozen CPU BERT evidence remain base evidence.
No native workflow test, real desktop IPC, GUI, GPU or text/image-model execution
is claimed for this fix. ORT native execution remains owner-deferred after the
cloud proxy's CDN CONNECT 403; no download retry or link/substitution workaround.

Repository/source instructions and coding standards `366c1d90a24bbfb50973f62b155a5f3396c0f107`
were inspected. Work uses the existing regular-speed environment, Rust 1.92,
one Cargo build job and existing credentials/network policy. No manifest, lock,
workflow, audit-gate, permission or network changes. Only the separate source
branch is uploaded; the parent owns review, PR publication and integration.
