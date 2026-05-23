# Milestone 5b: Runtime Host Handoff And Legacy Execution Removal

**Goal:** Replace successful `model_path`/`ModelRefV2` runtime execution with
runtime-host execution that consumes scheduler-owned handoff facts and resolves
Pumas-approved load targets only at the host boundary.

This milestone is split out of Milestone 5a because replacing node-engine
dependency preflight alone would preserve the legacy successful execution path.
Milestone 5a owns scheduler contracts and dynamic dispatch. Milestone 5b owns
runtime-host handoff wiring and deletion of the old resolver/path contracts.
Milestone 5b is a hard gate before real image-generation execution slices can
depend on runtime loading: the canonical runtime-host request/response and
host-owned Pumas load-target resolution must exist before old successful
`ModelRefV2`/`model_path` paths are deleted.

**Tasks:**

- [x] Define the runtime-host execution request/response contract first. It must
  consume `SchedulerRuntimeHandoff`, scheduler dispatch decision, dependency
  environment ref, and Pumas model/artifact identity without exposing
  `ModelRefV2`, `model_path`, executable load targets, reservations, batching
  groups, or worker launch internals to graph/node-engine contracts.
- [x] Add the host-owned Pumas load-target resolution service. It must resolve
  executable load targets only from scheduler-selected Pumas refs/artifact
  identity at runtime dispatch, and return typed unavailable/stale/invalid
  diagnostics instead of falling back to paths.
- [ ] Replace PyTorch execution so successful model loading consumes
  host-owned executable facts and no longer reads graph `model_path` or emits
  `ModelRefV2`.
- [ ] Replace llama.cpp execution so successful model loading consumes
  host-owned executable facts and no longer reads graph `model_path` or emits
  `ModelRefV2`.
- [ ] Replace audio execution so successful model loading consumes host-owned
  executable facts and no longer reads graph `model_path` or emits
  `ModelRefV2`.
- [ ] Replace node-engine dependency preflight output with typed readiness or
  scheduler handoff facts after the host execution request exists. Missing
  scheduler handoff must fail closed with typed diagnostics, not repair old
  inputs.
- [ ] Remove embedded-runtime `ModelDependencyResolver`/`ModelRefV2` resolution
  paths after runtime host load-target resolution is wired.
- [ ] Remove retired node-engine contracts and helpers:
  `ModelDependencyRequest`, `ModelDependencyResolver`, `ModelRefV2`,
  `build_model_ref_v2`, path repair helpers, and successful `model_path` test
  fixtures only after the runtime-host request/response and load-target
  resolution path are wired.
- [ ] Remove frontend/Tauri dependency actions keyed by `modelPath` or
  `model_path` after backend capability and task diagnostics cover the
  replacement user-visible state.
- [ ] Update README/crate documentation for every new host-facing contract,
  Pumas load-target boundary, runtime migration, deleted legacy path, and
  fixture replacement.

**Verification:**

- Contract tests and JSON fixtures for runtime-host execution input/output and
  Pumas load-target diagnostics.
- Boundary tests proving graph, node-engine, saved-workflow, scheduler hint,
  and scheduler handoff payloads reject executable path fields.
- Runtime-host tests proving Pumas load targets are resolved only at the host
  boundary and unavailable states produce typed diagnostics.
- PyTorch, llama.cpp, and audio tests proving successful execution no longer
  reads graph `model_path` or emits `ModelRefV2`.
- Node-engine tests proving affected runtime nodes fail closed when scheduler
  handoff is missing and do not call `ModelDependencyResolver`.
- Deletion/search checks proving successful production paths no longer contain
  `ModelDependencyResolver`, `ModelDependencyRequest`, `ModelRefV2`,
  `build_model_ref_v2`, frontend `modelPath` dependency actions, or path-shaped
  success fixtures.
- Focused crate checks for every touched Rust crate, including default,
  all-features, and no-default-features checks when public feature contracts
  change.

**No-Fallback Requirements:**

- Do not adapt scheduler readiness or handoff facts back into `ModelRefV2`.
- Do not preserve `model_path`/`modelPath` as successful runtime execution
  identity.
- Do not leave old resolver calls as alternate successful execution branches.
- Do not let node-engine, runtime adapters, frontend actions, or Tauri commands
  choose scheduler runtime/device/dependency policy.
- Do not expose executable Pumas load targets outside the runtime host
  boundary.

**Status:**

- [ ] Not started.
- 2026-05-22: Created from the Milestone 5a node-engine legacy boundary
  re-plan. Decision: use Option 3 planning structure with Option 1
  implementation direction. Milestone 5b owns runtime-host handoff and legacy
  execution removal; Milestone 5a continues scheduler-owned dynamic dispatch.
- 2026-05-22: Milestone 5a closeout decision recorded. Option 2 selected:
  close Milestone 5a as scheduler-contract complete and keep actual legacy
  deletion in this milestone as the hard gate. Implementation must begin with
  the runtime-host execution request/response contract, then host-owned Pumas
  load-target resolution, then runtime migrations and deletion. Do not create
  a scheduler-handoff-to-`ModelRefV2` adapter or a path-repair bridge while
  crossing this boundary.
- 2026-05-22 runtime-host request/response contract slice completed. Smallest
  useful vertical slice: add the embedded-runtime host-facing execution
  request/response DTOs, validated wrappers, typed diagnostics, and JSON
  fixtures without resolving Pumas load targets or launching runtimes. Allowed
  write set: `crates/pantograph-embedded-runtime/`, this milestone file, and
  execution notes. No-fallback confirmation: the request consumes a
  dispatch-selected `SchedulerRuntimeHandoff` and rejects readiness-only
  handoff; request/response contracts expose no executable load target, local
  path, `ModelDependencyRequest`, `ModelRefV2`, graph `model_path`, frontend
  `modelPath`, path repair, reservation/batching internals, or worker launch
  details. Verification passed: `cargo fmt -p pantograph-embedded-runtime`,
  `cargo test -p pantograph-embedded-runtime runtime_host_execution`,
  `cargo check -p pantograph-embedded-runtime`,
  `cargo check -p pantograph-embedded-runtime --all-features`,
  `cargo check -p pantograph-embedded-runtime --no-default-features`,
  `cargo fmt -p pantograph-embedded-runtime -- --check`, `git diff --check`,
  README coverage review, source/test fixture directory coverage review, and
  file-size standards check for new runtime-host source/test files. Remaining
  follow-up: add host-owned Pumas load-target resolution service.
- 2026-05-22 host-owned Pumas load-target resolution slice completed.
  Smallest useful vertical slice: add the embedded-runtime host-only load
  target resolver module that builds Pumas requests from validated
  runtime-host execution requests and maps ready/unavailable Pumas responses
  into host-owned results without wiring runtime execution. Allowed write set:
  `crates/pantograph-embedded-runtime/`, this milestone file, and execution
  notes. No-fallback confirmation: the resolver uses scheduler-selected Pumas
  model/artifact identity and Pumas typed resolver states only; it does not
  accept graph `model_path`, frontend `modelPath`, `ModelDependencyRequest`,
  `ModelRefV2`, path repair, package-fact scraping, or alternate successful
  resolver branches. Verification passed: `cargo fmt -p
  pantograph-embedded-runtime`, `cargo test -p pantograph-embedded-runtime
  runtime_host_load_target`, `cargo test -p pantograph-embedded-runtime
  runtime_host_execution`, crate check matrix, fmt check, diff checks, README
  coverage review, and file-size standards check. Remaining follow-up: wire
  PyTorch execution to consume host-owned executable facts instead of graph
  `model_path`.
