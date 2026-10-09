# Python startup broker and proposed owner selection contract

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
Managed `start_managed_runtime` uses the same broker for a closed preflight and
reads only the existing Rust worker flag. No Python query or initialization is
performed. The metadata-only managed entry does not expose global reservations
or accept a dummy keepalive as a real owner selection.

This is incomplete PyO3-entry coverage. Separate audio/rerank/inspection paths,
CUDA inventory, other `with_gil` calls and external interpreters may have prior
unregistered history; their working bodies are untouched. A successful legacy
claim or empty broker is not evidence that CPython was uninitialized. Positive
startup remains refused until all relevant first-entry paths and actual linked
image/configuration/import provenance are coordinated and qualified.

## Exact proposed Pumas owner contract — not implemented

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

Proposed names and signatures for owner reconciliation:

```rust
// Proposed owner types; no production stub or positive implementation exists.
pub struct RetainedTorchStartupSelection { /* private owner-issued fields */ }
pub struct TorchStartupSelectionIdentity {
    pub environment_id: String,
    pub runtime_revision: String,
    pub published_manifest_sha256: String,
    pub interpreter_depot_manifest_sha256: String,
}

impl VersionManager {
    pub async fn retain_torch_startup_selection(
        &self,
        tag: &str,
        expected: &TorchStartupSelectionIdentity,
    ) -> Result<RetainedTorchStartupSelection>;
}
impl RetainedTorchStartupSelection {
    pub fn identity(&self) -> &TorchStartupSelectionIdentity;
    pub fn bytes(&self) -> &Arc<RetainedRuntimeReadSource>;
}
```

`expected` is a consumer declaration. Pumas must match it against the actual
published immutable component manifest and managed depot while acquiring the
genuine revision/depot source through existing publication/deletion custody.
The returned wrapper has no public constructor or serde reconstruction. Its
identity must be a frozen fact associated with those actual retained bytes,
not a later active tag, caller-provided JSON or mutable registry lookup.
Its immutable manifest must identify the exact wheel/RECORD/native provider,
build-source association and actual retained base interpreter/depot. An approved
executable/shared-library build, runnable interpreter entry, startup profile and
offline dependency closure are separate startup requirements; absent or
unqualified fields refuse rather than being inferred from base retention.
The owner's unqualified disposition and launch restrictions must remain bound
to the same immutable manifest and must not be removed by this retention API.
No successful wrapper return or empty caller-supplied restriction list may
constitute executable readiness. The depot digest names the
owner's closed selected depot manifest; its format and canonical digest must
be specified by the owner assembler. All field shapes and expected values
must be validated; omission/mismatch refuses before native effects. This is
still a byte/selection association, not loaded-image or execution proof.

The method must preserve650b behavior: no lifetime global versions lock;
same-revision physical mutation exclusion and unrelated revision publication;
capture effects retain their real leases through caller loss/drainage. Selection
must not be fabricated from a publicly captured source with a dummy Arc. A
component-bearing revision must already be atomically published/finalized by
VersionInstaller; this method neither repairs in place nor installs anything.
The wrapper and its returned actual source Arc remain held through the consumer
process/native owner, including possibly partial import failure. No model-stop
event releases that custody.

Pantograph still links Pumas26a. The new wrapper names are a concrete proposal,
not claims about current Pumas methods/types or a speculative dependency bump.
The generic broker is ready for the real Arc lifetime without importing an
unpublished type; the metadata-only managed entry remains closed. Linking the
published byte API and owner-issued wrapper requires a separately reviewed
dependency/consumer adapter increment after agreement and assembly publication.

## Joint startup protocol after that owner contract

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
