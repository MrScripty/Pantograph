# Managed PyO3 interpreter/import registration requirements

These are consumer requirements for positive interpreter/import qualification.
The owner-issued selected-byte contract is published and linked; it supplies
no positive runtime proof type.
Pantograph's explicit managed-start entry continues to refuse. Real wheel
transfer, alternate artifact publication, installation, native import and live
execution remain outside this source preparation increment.

## Published acquisition and byte retention

Pumas published local acquisition at
[`c9fb9ce7e502bb04682a1ca640f8d0a446a4ee50`](https://github.com/MrScripty/Pumas-Library/commit/c9fb9ce7e502bb04682a1ca640f8d0a446a4ee50),
branch `integration/v08-local-component-ingestion-20261009`.
`AcquisitionLocalSource::new(File, Arc<dyn Send + Sync>)` requires an actual
held regular file and cooperating input-owner keepalive. Linux is supported;
the keepalive supplies neither immutable-byte nor no-symlink-origin proof.
`AcquisitionLocalRequest` requires one source per manifest member in order,
exact size and SHA256, the existing demand/workspace, and positive finite retry
limits. `AcquisitionConsumer::acquire_local` retains registered read effects
and the existing store/verified-use/consumer-publication lifecycle. A refusal
can arrive before an admitted read drains; retain the consumer/service and
await shutdown before owner teardown. Its deadline gates new reads and retries,
without promising a bound on a blocked OS read or shutdown.

The original preparation increment compiled against Pumas26a. The existing
`ArtifactManifest` constructors are unchanged at published c9, so
`managed_python_acquisition::reviewed_tokenizer_acquisition_manifest` prepares
the exact one-member selection at that original pin. It provides
the 24,366,281-byte wheel and SHA256 recorded in the
[handoff](pumas-managed-tokenizer-handoff.json); revision evidence identifies
the frozen build-source association, including all nine prepared source files.
It opens no file, acquires no input keepalive, creates no workspace/store/task,
submits no request and grants no execution readiness. It belongs in the existing
embedded host's Pumas adapter. The current source advances both Pumas dependencies
to3d and adds an optional typed owner-selection adapter in inference; real
acquisition submission remains held.

Pumas published the async `VersionManager` method
`pub async fn retain_torch_runtime_bytes(&self, tag: &str) -> Result<Arc<RetainedRuntimeReadSource>>`
at [`650b6cb38eb432b2dd39a58cc6ea14855d2f62ae`](https://github.com/MrScripty/Pumas-Library/blob/650b6cb38eb432b2dd39a58cc6ea14855d2f62ae/docs/plans/artifact-acquisition/reports/torch-revision-custody-2026-10-09.md),
branch `integration/v08-torch-revision-custody-20261009`, tree
`8855c3941dfaec9101e5cf2a1bf9a22dc4ccd43f`. Its Linux byte custody retains the
selected revision and interpreter depot, releasing the global lock before
blocking capture. Same-revision physical mutations refuse while retained;
unrelated revision publication is covered by owner synthetic tests. Acquire the
actual Arc before initialization/import and retain it through process retirement.
The current3d dependency includes this source and the genuine component selection
wrapper. The consumer calls its blocking validation and retains the actual Arc,
with no parallel lease or package store.

This source retains bytes, not registry metadata: registration/selection metadata
may change while held. It uses cooperative physical-mutation locks, not protection
against hostile same-user writes. `RetainedRuntimeReadSource::validate()` checks
selected namespaces, identities, sizes and hashes; `manifest()` exposes role/member
facts and `clone_root`/`clone_member` supply held selections. A cloned File alone
does not keep the parent source's revision/depot leases alive. These blocking
validation/read effects need registered drainage and must retain the actual Arc.
Interpreter, Dependencies and Sidecar roles do not attest omitted namespaces,
system ELF libraries, loader search paths or every model-specific executable read.

That retained source meets the published byte-retention seam. It does not
prove which interpreter or native image has already been initialized/imported.
Acquisition receipts, retained paths and serializable metadata cannot open the
managed-start gate.

Pumas owns the registered revision/depot selection, immutable byte identity and
retention/publication/deletion authority. Pantograph owns its in-process PyO3
startup and provider import, and must establish the corresponding registration
while holding that real owner source. Pumas need not initialize Python inside
`retain_torch_runtime_bytes`. Both owners still need to agree the association and
startup protocol; byte-source metadata alone cannot stand in for the consumer's
actual initialization/import evidence.
The [startup broker and published owner-selection adapter](pumas-python-startup-broker.md)
implement controlled exclusion and actual selected-byte Arc lifetime mechanics.
Pumas3d issues revision/manifest/depot facts, with no environment ID; Pantograph's
configuration identity remains declared. Every managed phase still refuses;
selected byte custody supplies no observed native proof or positive startup.

## Exact registration requirements still outstanding

1. **Registered selection.** Resolve the opaque revision and immutable published
   manifest under owner mutation/publication authority. Keep Pantograph's declared
   environment/configuration ID separate; Pumas issues no environment ID.
   A component-bearing revision must also bind the reviewed wheel,
   installed RECORD, native extension and build-source association. Bind its
   actual interpreter depot/build and approved offline dependency closure.
   The handoff selects no base, and a Torch release tag alone is insufficient.
2. **Initialization ownership.** Establish an owner-associated registration
   before the first interpreter initialization or provider import in the actual
   Pantograph process. Synchronize it with legacy shared-worker initialization
   and every native startup path. Competing legacy start, unknown initialization
   or a prior unregistered import must refuse managed adoption. Serialize the
   decision without holding a blocking registry lock while waiting for the GIL.
   This is a joint startup contract, not a consumer-declared boolean.
3. **Actual interpreter identity.** Associate registration with the actual
   process/interpreter and a non-reused owner generation. Prove the linked
   CPython library/build and approved initialization configuration, prefix and
   package resolution match the retained selected runtime and qualified cohort.
   PyO3's linked library is not rebound by installing a venv. A matching current
   executable name, `sys.prefix`, version string or on-disk hash cannot establish
   prior initialization custody. The current consumer performs no interpreter
   query or initialization to obtain this evidence.
4. **Native import provenance.** Under that same retained registration, control
   and record the provider's initial import against the selected wheel/RECORD,
   exact native image and source association. Bind the actual imported image,
   including the qualified native dependency/interpreter cohort, rather than
   trusting a later `module.__file__` or hashing a replaceable current path.
   Duplicate or preexisting unregistered images refuse. Unknown provenance
   remains Unknown/refused; no filename-based retroactive adoption is accepted.
5. **Live association.** Supply an owner-issued, non-serializable association
   retaining the actual byte source and registration. The consumer must be able
   to establish its selected revision, process/interpreter generation, provider
   identity and current readiness as one consistent association. A manifest,
   acquisition receipt, input keepalive or separately submitted IDs cannot
   manufacture it. A caller-created `RuntimeReadRoot`/`capture` with a dummy
   keepalive is not a VersionManager-issued revision/depot lease. The broker must
   preserve the real owner-issued association rather than accepting any captured
   source merely because it has the same public Rust type.
   Representation and API naming remain owner-owned; no fake
   `Arc<()>`, proof constructor or `custody_verified` switch is supplied here.
6. **Lifetime and failure.** Keep byte custody and registration through every
   native closure's actual completion, including cancellation, stream drop and
   caller loss. Model stop drains producers but does not retire imported native
   images. Partial initialization/import failure must close executable readiness
   while keeping pins for any possibly mapped images until process retirement.
   Before import, existing stage/child drainage governs rollback; publication
   failure preserves the prior active/default revision. No global Python edits,
   `Py_Finalize`, module eviction or in-place repair supplies safe retirement.

## Joint acceptance gates before any positive consumer

The proposed startup sequence is: assembled immutable selection -> retained and
validated role/member bytes -> owner-associated first-initializer registration ->
controlled initialization/import -> ready registration -> drain -> process-held
retirement custody. These are protocol phases, not a new executable API or enum.
Pumas's existing VersionInstaller owns staged validation, atomic publication and
finalization; its component-bearing immutable byte assembler is published at3d.
Its output remains `assembled_unqualified`, with no runnable interpreter entry.
The broker must retain the same actual source throughout the later phases and
close readiness on every failure without releasing possibly mapped-image pins.

The present PyO3 dependency enables `auto-initialize`, and shared-worker paths
enter `Python::with_gil`. A managed broker must therefore establish exclusive
first-start ownership before any such path, with a qualified linked CPython build
and approved startup configuration. A later prefix check cannot repair that
ordering. PyO3 .23 supplies no safe no-initialization query used by this consumer,
and workspace Rust forbids unsafe code. The actual pre-initialization broker,
build/load evidence collection and configuration mechanism remain architectural
decisions requiring joint review; this slice supplies no raw FFI, speculative
interpreter query, global Python edit or positive proof constructor.

Owner tests must establish selected revision/depot retention against deletion
and replacement, while an unrelated revision can install. The registration
tests must cover concurrent legacy/managed first startup; wrong revision,
interpreter, prefix or provider image; reused process identity; missing/stale
registration; replaced paths after import; and initial/partial import failure.
Native task cancellation/caller loss must retain the same association until
actual drain. Model stop must leave imported-image pins held. A positive witness
requires the actual approved component-bearing runtime, interpreter and imported
image, with joint source/artifact binding. Synthetic metadata tests do not supply
that witness, and no such live run is authorized by this handoff.

The interpreter cohort stays conventional-GIL CPython3.12.3 and the exact
source-associated Linux x86_64 build in the handoff. The abi3 tag does not expand
qualification. Opaque tokenizer settings, other inspection components and the
aggregate NativeOwnerSnapshot remain Unknown/refused. Comparable timing/resource
coverage and dispatch acceptance are separate scheduler gates; the existing
PriorityThenFifo policy, starvation boosts and warm-reuse window stay unchanged.
