# Ordered producer resident estimates

Separate successor `feat/runtime-producer-resident-estimates` starts from frozen
`254aef1ae67dd7dba9cb93becdfd79b8678879f3`, tree
`3e7724252d017cd248dc8ce34b08e1cb7e9ba030`. Parent owns PR/review/publication.
Previous branches, PR54/55 and sampler implementation remain unchanged.

## Selected owner and implemented scope

The selected producer is the active PyTorch text-model lifecycle already owned
by `InferenceGateway`. Startup settings provide explicit RAM/VRAM estimates keyed
to the exact observed model target. No package-size/device heuristic invents a
split. The registry still owns accounting/admission; no second lifecycle manager,
allocator, process watcher or backend controller was added.

Inspection found three boundaries needing explicit treatment:

- Backend readiness/model metadata is cleared before effectful worker loading.
  A load error can therefore leave uncertain allocations despite missing metadata.
- `PyTorchBackend::stop` previously skipped shutdown when both metadata and
  readiness were absent. It now retains effectful-load uncertainty and requires
  acknowledged worker shutdown before declaring this allocation released.
- Model/instance labels alone cannot order asynchronous publications. Snapshot
  generation is allocated under the existing backend lifecycle lock, before
  delivery. Stable source token plus increasing sequence rejects old stops,
  duplicate/older loads, reused identity labels and implicit owner handoffs.

`runtime_model_resident_estimates` is an optional flattened AppConfig startup
setting. Canonical runtime aliases and exact opaque model targets identify its
entries. Invalid/duplicate/empty/overflowing estimates fail composition.
Explicit zero means known zero; an omitted resource kind remains unknown.
Configuration implies neither a loaded model nor readiness. Cold persistence
round-trips retain estimates, while each fresh registry starts without residency.
Live edits require restart through the existing config guard.

Gateway snapshots distinguish Resident, Released and Unknown. Resident frames
publish the configured envelope for the current model/instance atomically under
the registry admission lock. Oversized declarations leave actual new residency
unknown instead of restoring fictitious old lifecycle state; a subsequent frame
retries after task capacity is released. Missing estimates fail shared admission.
Unknown owner evidence retains the previous known envelope, sets the observable
`resident_resources_uncertain` flag, and blocks other members of a shared pool.
Only acknowledged owner stop/switch publishes logical release. Existing backend
start, selected-text load, stop and embedding replacement remain the owners of
those operations; the new fields are observation evidence, not control authority.

Host sync, warmup, restore, stop-all and reclaim reconciliation publish these
frames through the existing controller. Core gateway wiring is compiler checked;
the desktop controller forwards to its existing inner gateway. Once a runtime is
owned by sequenced frames, unsequenced stopped observations/transitions, negative
inactivity and manual declarations cannot free or replace its allocation.
Matching negative health evidence can still restrict dispatch. Unconfigured
legacy lifecycle/admission behavior is preserved.

Complete task peak claims and provisional custody remain independently charged;
confirmed model unload does not remove a live task lease. Same-identity resident
declarations retain the componentwise maximum. No guessed weight subtraction,
allocation alias, measured transient split or physical GPU safety is claimed.

## Executed validation

Executed results: 274 portable registry/scheduler/AppConfig tests (129/133/12),
441 portable inference tests including 15 new lifecycle tests, and one controlled
Python worker owner test: 716 passed, zero failed or ignored. Four-package Clippy
with PyTorch enabled passes with warnings denied. Embedded library/test compiler
checking passes with native downloads disabled. Formatting, critical-pattern,
scheduler-only and whitespace gates pass. Decision traceability is checked over
the staged successor against the frozen base before commit.

The default-only inference test build emits an existing dead-code warning for
selected-text validation fields when PyTorch is disabled; the PyTorch-enabled
Clippy configuration passes with `-D warnings`.

Commands:

```bash
source /workspace/pantograph-tools/activate.sh
cargo test --locked --offline -p pantograph-runtime-registry -p pantograph-scheduler -p pantograph-app-config
cargo test --locked --offline -p inference --lib
cargo test --locked --offline -p inference --lib --features backend-pytorch production_text_iterator_drains_before_successful_load_and_unload
cargo clippy --locked --offline -p pantograph-runtime-registry -p pantograph-scheduler -p pantograph-app-config -p inference --all-targets --features inference/backend-pytorch -- -D warnings
ORT_SKIP_DOWNLOAD=1 cargo check --locked --offline -p pantograph-embedded-runtime --lib --tests --features backend-pytorch
cargo fmt --all -- --check
node scripts/check-critical-antipatterns.mjs
bash scripts/check-scheduler-only-workflow-execution.sh
git diff --check
```

The gateway fixtures use the actual sampling/publication API, without models,
Python or hardware. They cover start, retained idle, full peak lease charging,
replacement, delayed stop/load, reused labels, pre-effect rejection, effectful
failure, failed initial load, failed unload, readiness/worker loss, acknowledged
unload, missing/oversized estimates, retry after lease release, known-zero VRAM, cold backend switch, legacy behavior, matching negative health,
different source rejection, unsequenced stop rejection and live custody retention.
Actual AppConfig tests cover load/save/composition and invalid estimate rejection.
The controlled Python worker test exercises actual PyTorch backend shutdown after
load metadata disappears, including a rejected shutdown and a later acknowledgement;
it uses existing stub worker dependencies, not physical runtime execution.

## Limits and remaining qualification

Automatic estimate publication is bounded to the current active PyTorch text-model
owner. Other producers, independent diffusion caches, standalone backend unload
calls and allocator caches outside that owner are not projected or measured.
Readiness loss alone does not release capacity; the existing owner must reconcile
or acknowledge stop. Changing owner requires fresh composition. No native GPU,
real model allocation, GTK desktop IPC/process or cancellation/abort durability
qualification is inferred from portable fixtures or compiler success.

The existing native prerequisites remain blocked: GTK/WebKit/GLib are unavailable,
and standard pinned ORT download returns 403. One broad workflow-service test
attempt encountered that existing ORT failure; subsequent compiler checking used
`ORT_SKIP_DOWNLOAD=1`. No package installation, pin update, native substitution,
model download, frozen-ref rewrite or PR/review creation occurred. Cargo.lock
changes only to add inference's local registry dependency; all external package
versions/sources/checksums remain unchanged.
