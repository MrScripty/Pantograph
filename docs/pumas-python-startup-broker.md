# Python startup broker and Pumas selected-byte adapter

This increment implements Rust startup exclusion and keepalive retention, using
controlled tests. It supplies no native initialization/import permission or
observed runtime witness. Real wheel transfer, installation, import, inference
and positive managed startup remain on hold. The
[registration requirements](pumas-interpreter-registration-requirements.md)
remain the acceptance boundary.

## Proportionate consumer design

One process-owned `PythonStartupBroker` uses a short `std::sync::Mutex`. The
production singleton is private and rooted in `OnceLock`; it has no reset or
model-stop retirement method. The mutex protects only bounded Rust state and
generation transitions. It is released before the GIL, owner destructors or
observation callbacks. Pumas capture/validation/acquisition and registered job
drainage must happen outside it.

| State | Meaning | Allowed next action |
| --- | --- | --- |
| Unclaimed | No claim recorded by these seams; actual Python history remains unknown | Legacy claim or exclusive declared preflight reservation |
| LegacyClaimed | A participating legacy path claimed before entering Python | Repeated legacy use; managed reservation refuses |
| ManagedReserved | One generation holds declared selection metadata and the exact supplied Arc | Pre-exposure caller drop rolls back that generation; legacy/other reservations refuse |
| ProcessPinned | Keepalive parked before potential native exposure | Keep custody until process-owner retirement; no reset or automatic rollback |

Every state refuses managed execution. `ManagedReserved` and `ProcessPinned`
are custody/order states, not verified registrations, loaded-image observations
or executable readiness. An arbitrary `Arc<()>` satisfies keepalive ownership
but supplies no owner issuance. `reserve_declared` preserves the actual supplied
Arc's control block, so a future genuine retained owner Arc can use the same
lifetime mechanism; the current interface does not authenticate its issuer.
Selection/configuration identities remain explicitly declared metadata.

A reservation is non-cloneable and non-serializable and is tied to its actual
broker and checked, non-reused generation. Pre-exposure Drop removes only that
generation and releases keepalive custody outside the mutex. `pin_for_process`
consumes the reservation, transfers custody into the broker and returns no
execution permit. After parking, cancellation, caller loss, failed startup or
model/worker stop cannot undo retention. Native closures must additionally own
their actual source Arc until registered completion. Controlled tests use fresh
broker owners and inert Drop-tracked objects; dropping such an owner models
retirement and does not qualify actual OS/native-image retirement.

The actual global legacy fence is composed at `InferenceBackend::start` before
drain/start effects, and at generic worker init and shutdown helpers before
`Python::with_gil`. Its sticky claim preserves repeated ordinary legacy startup.
Shutdown can initialize the inherited worker, so that helper is also fenced.
Generic loaded-model/KV helpers, public Transformers loading, unloading, text
producers and KV truncation now claim the same broker before their first Python
entry. Text producers claim on their registered effect thread after cancellation
checks; KV truncation claims before temporary-file effects. Refusal propagates
through each existing error contract. Claims remain sticky for ordinary legacy
work; they do not certify actual interpreter history.
Managed `start_managed_runtime` uses the same broker for a closed preflight and
reads only the existing Rust worker flag. No Python query or initialization is
performed. The metadata-only managed entry does not expose global reservations
or accept a dummy keepalive as a real owner selection.

All 19 known production GIL entry sites now use `PythonLegacyAdmission::with_gil`;
the token method is the sole direct production `Python::with_gil` call. CUDA
inventory, audio execution/shared-private loading/shutdown, image generation,
timing revalidation and isolated-worker Drop are included. Inventory returns
`python_startup_refused` when admission fails, image/audio operations propagate
the existing backend error, and optional version diagnostics retain their
`unknown` fallback. Separate inspector, gateway, selected/host audio/rerank
adapters and all Python workload files remain unchanged. The earlier broader
backend freeze was an internal worker precaution; this backend-only coordination
does not rescind the standing adapter ownership restriction.

The generic process owner retains its original admission in static `LEGACY_ENTRY`.
Shutdown/unload therefore do not reacquire admission and skip required cleanup
after later broker poison. Isolated workers inherit that actual token and retain
an independent `PythonLegacyExposure` record; timing owners retain it alongside
their native handles. Preparation occurs before GIL on production effect paths.
Worker initialization also defensively retains admission for internal callers
already holding Python; that post-entry retention is not first-entry evidence.
No claim or broker mutex spans Python callbacks or native owner destruction.

Admission and native exposure are distinct. A prepared but never-entered worker
does not enter Python during Drop. Exposure is marked inside the admitted GIL
body before imports/module creation, so partial initialization failure still
requires cleanup through the original token. Blocking jobs retain their actual
worker Arc through completion, including caller loss; Drop runs after the final
Arc release. Poison cannot revoke the earlier claim or suppress its cleanup.
Global worker/native images and deferred `Py<T>` decrefs remain under sticky
process admission until retirement; model stop does not reset that ownership.
Controlled Rust tests exercise this actual record without entering Python.

This completes coordination for known repo-owned production entry/cleanup seams,
not an authenticated first-initializer protocol. External/third-party interpreter
entry and earlier unregistered initialization remain unknown. A successful claim
or empty broker cannot prove that CPython was uninitialized or identify its loaded
image. Positive startup remains refused until real linked-image/configuration/
interpreter/import provenance is qualified; no such witness is fabricated here.

## Published Pumas owner contract and consumer adapter

The published Pumas650b API supplies physical revision/depot byte custody:
`pub async fn retain_torch_runtime_bytes(&self, tag: &str) -> Result<Arc<RetainedRuntimeReadSource>>`.
Its public `RuntimeReadRoot`/`capture` can accept arbitrary keepalives, so its
Rust type alone cannot identify VersionManager-issued custody. Registry metadata
can change while the bytes remain retained. The next needed owner association
should be issued from the existing immutable component assembler/publication
authority, without another store or consumer lock.

The owner's component assembler currently retains the actual selected base Arc,
copies fresh outputs and removes inherited probe evidence. Its new component
bundles are explicitly unqualified and have no runnable venv interpreter;
identity/probe/dependency/core launches refuse pending genuine qualification.
Retaining such a bundle must preserve those restrictions. A retained base
interpreter/depot is not evidence of a runnable interpreter entry in the newly
assembled component bundle. The consumer broker therefore continues to refuse
even when byte custody and all declared digest fields match.

Pantograph now pins Pumas commit `3d977f11be98eb9bce5be199e05b5ed1846043ca`,
tree `f906976a94f87e086698cf0e4f177e8cd6d87366`. The exact published
[component assembly/selection contract](https://github.com/MrScripty/Pumas-Library/blob/3d977f11be98eb9bce5be199e05b5ed1846043ca/docs/plans/artifact-acquisition/reports/torch-component-assembly-2026-10-09.md)
replaces the earlier proposed wrapper. Exported names are:

```rust
pub struct TorchComponentSelection { /* private owner-issued fields */ }
impl VersionManager {
    pub async fn select_torch_component_revision(
        &self, revision_tag: &str,
    ) -> Result<TorchComponentSelection>;
    pub async fn select_torch_component_revision_matching(
        &self, revision_tag: &str, expected_manifest_sha256: &str,
        expected_interpreter_depot_manifest_sha256: &str,
    ) -> Result<TorchComponentSelection>;
}
impl TorchComponentSelection {
    pub fn revision_tag(&self) -> &str;
    pub fn manifest_sha256(&self) -> &str;
    pub fn interpreter_depot_manifest_sha256(&self) -> &str;
    pub fn qualification(&self) -> &'static str; // assembled_unqualified
    pub fn retained_source(&self) -> &Arc<RetainedRuntimeReadSource>;
    pub fn validate(&self) -> Result<()>; // blocking
}
```

The private, cloneable selection is genuinely owner-issued and binds immutable
revision/manifest/depot identities to the actual retained source. The depot
digest uses Pumas's domain-separated sorted Interpreter byte manifest. There is
no Pumas environment identity: Pantograph's `environment_id` remains its own
configuration declaration. It is not synthesized from an owner tag or digest.
Owner issuance alone does not authenticate the consumer's declared provider
wheel, extension or build-source constants against component contents.
No runnable interpreter or initialized/imported provider is inferred from them.

The method must preserve650b behavior: no lifetime global versions lock;
same-revision physical mutation exclusion and unrelated revision publication;
capture effects retain their real leases through caller loss/drainage. Selection
must not be fabricated from a publicly captured source with a dummy Arc. A
component-bearing revision must already be atomically published/finalized by
VersionInstaller; this method neither repairs in place nor installs anything.
The wrapper and its returned actual source Arc remain held through the consumer
process/native owner, including possibly partial import failure. No model-stop
event releases that custody.

The optional `pumas-component-selection` inference feature links the published
app-manager API; `backend-pytorch` enables it. The typed
`reserve_component_selection` validates declaration/digest shapes and exact
revision/manifest/depot matches before blocking `selection.validate()`, then
reserves the broker with a clone of the exact `retained_source()` Arc. All byte
validation happens outside the broker mutex while this synchronous stack owns
the selection. A legacy winner during validation makes final admission refuse.
The returned transaction retains the genuine selection, rolls back before
exposure, or parks the actual source Arc in the existing process pin mechanism.
Pinning returns no execution permission.

`start_managed_component_runtime_blocking` is a synchronous typed PyTorch
preflight. It reserves validated selected-byte custody, checks retained provider
association outside the mutex, refuses the fixed
`assembled_unqualified` disposition, and releases its pre-exposure reservation.
Even a future different disposition refuses missing startup evidence. It creates
no async/detached job, enters no GIL and changes no worker/configuration state.
An async caller must own and drain its blocking validation effect; this adapter
does not install a new supervisor or provide bounded OS-read latency. The
metadata-only managed entry remains closed and existing legacy behavior remains.

The static provider check reads only the genuine retained
`Sidecar/component-manifest.json`, capped at the owner's 64 MiB manifest limit.
Strict published component/acquisition schemas reject unknown or duplicate wire
fields. Wheel identity, size/digest declarations, immutable build-binding source
revision, Python/platform/tag and provider names must match the reviewed inputs.
Archive and final dependency paths are exact, case-fold duplicates refuse, and
every archive member must survive unchanged in the actual retained final closure.
Base dependencies outside the component remain allowed. The check reopens the
required extension and standalone build-binding through retained descriptors,
verifying their exact reviewed digests (and native size) without importing them.
Alternative tokenizers distributions/native members refuse.

This establishes required native/build-member byte association plus consistent
declared closure. The retained wheel itself is absent: its SHA is input metadata,
and other wrapper/RECORD/METADATA digests are owner-manifest declarations rather
than an independently frozen reviewed wheel-member manifest. Full reviewed wheel
association, build-history authenticity and positive acceptance remain held
input gates. The blocking check revalidates bytes, holds the actual selection,
creates no detached job and returns no startup permit. Generic byte reservations
remain useful independently; only the managed execution preflight requires this
additional check before its unconditional refusal.

Controlled selection tests seed isolated inert assembly/depot metadata and
sidecar bytes, then invoke the actual public VersionManager selector. They do
not execute the assembler/install pipeline or any Python/native bytes. Set
`PUMAS_COMPONENT_TEST_SOURCE_ROOT` to the verified Pumas3d repository source root
for those fixtures; a missing fixture input fails explicitly.
The fixture copies upstream embedded sidecar source as bytes only. Those tests
and fresh broker retirement qualify byte selection/refusal/lifetime/races, not
real OS process retirement, a transferred wheel or positive runtime startup.

## Joint positive startup protocol still outstanding

1. Obtain and validate the actual owner-issued immutable selection outside the
   broker mutex. A legacy startup can win during capture; after capture, reserve
   the broker and refuse if it already claimed. Reject unqualified assembly,
   missing runnable interpreter or launch restrictions before proceeding.
   Nothing native occurs yet.
2. While holding that exclusive reservation and actual source, establish the
   approved linked CPython build and startup configuration evidence for this
   process before any auto-initializing PyO3 entry. Synchronize every relevant
   first-entry path. Unknown prior initialization/import refuses; reading
   current sys/module labels or mutable paths does not establish history.
3. Park actual custody in the process owner before any potentially native effect.
   Separately record genuine initialization and initial provider-image provenance
   under the agreed broker; successful configuration checks alone are not that
   observation. No such witness or positive registration constructor exists here.
4. Publish executable readiness only after genuine evidence matches the retained
   revision, interpreter generation and provider image. On failure close readiness
   and drain jobs while retaining all possibly mapped-image pins until process
   retirement. Opaque/remaining accounting stays Unknown/refused.

Pumas selected roles omit namespaces, system ELF libraries, loader search paths
and complete model-specific read sets. The broker cannot invent attestations for
them. Actual build/load evidence collection and approved initialization
configuration remain joint architectural decisions under Rust's unsafe-code
denial and PyO3 .23 auto-initialization. This source slice adds no raw FFI, sys.path
mutation, interpreter rebinding/finalization, provider loading or native campaign.
