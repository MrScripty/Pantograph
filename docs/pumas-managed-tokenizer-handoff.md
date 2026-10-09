# Pumas local provider ingestion and Pantograph interpreter binding

This document and its JSON are the original frozen wheel/source handoff and
Pumas26a consumer qualification receipt. Their dependency/status fields describe
that increment, not the current source pin. The published Pumas3d component
assembler and [selected-byte consumer adapter](pumas-python-startup-broker.md)
supersede the earlier assembly/selection proposal below. Pumas issues immutable
revision/manifest/depot identities, with no environment ID; Pantograph keeps its
configuration identity separate. The original JSON and artifact association
remain unchanged. Actual wheel transfer and positive runtime qualification
remain held.

This is a concrete consumer requirement and proposed owner boundary, pending
Pumas acknowledgment. It supplies reviewed local bytes; it grants no installation
or execution authority. The machine-readable input is
[pumas-managed-tokenizer-handoff.json](pumas-managed-tokenizer-handoff.json).
No new package store, network resolver, in-place repair or Python rebinding is
introduced by Pantograph.

Pumas subsequently published held-file local acquisition at `c9fb9ce7`.
Pantograph now prepares its compatible exact wheel manifest using the existing
Pumas26a constructors. Pumas published `retain_torch_runtime_bytes` byte custody
at `650b6cb3`; genuine initialization/import registration is still
required. See the [current consumer requirements and acceptance gates](pumas-interpreter-registration-requirements.md).
Real wheel transfer and alternate artifact publication are on hold pending
specific user approval; this update performs neither.

## Exact first input

| Item | Identity |
| --- | --- |
| Consumer source | Pantograph `f26721b8d6dcda8fc2d3f62c6d81d374e9ea9257`, tree `4ea7f55f50102f6cf5c5158072c5c5f5474d12b9` |
| Upstream source | Tokenizers0.21.4 `e892882fd4608b468dcf9dc33ea95283882b8e6d`, Apache-2.0 |
| Artifact | Python wheel `tokenizers-0.21.4+pantograph.snapshot1-cp39-abi3-linux_x86_64.whl`, 24,366,281 bytes |
| Wheel SHA256 | `678b155145bb06c271ad6d8eb2df95a8a8173155323fafd4b102e50349940b95` |
| Installed extension | `tokenizers/tokenizers.abi3.so`, 24,156,176 bytes |
| Extension SHA256 | `5a7eacfad202bcacf122716f5a45e9796d098c8e65f86a56536efddceb573423` |
| Embedded build association SHA256 | `b2a4fc2a43b6ae5631b2f5ec45deb5394cd2059e456f97213a87d1bc5d1f703a` |

The JSON preserves all nine prepared source hashes, consumer file hashes, exact
qualification interpreter inputs and ELF dependency/version references. The
handoff archive includes the wheel, frozen association, upstream source archive,
prepared overlay, license and independent reviews. Reproduction is optional;
materializing the supplied bytes requires no model, runtime or dependency-source
download. Another executor owns its local materialization and hash checks; paths
in old review associations describe that qualification run and are not portable
destinations. Preserve the original wheel/embedded association bytes unchanged.

The component qualification requires conventional-GIL CPython3.12.3 and the
source-associated Rust1.92.0 `ded5c06cf21d2b93bffd5d884aa6e96934ee4234` build for
`x86_64-unknown-linux-gnu`. The wheel's abi3 tag does not certify source or runtime
portability. Static ELF inspection shows GLIBC2.34, GLIBCXX3.4.29 and CXXABI1.3.8
version references, plus the libraries recorded in JSON. This is a local Linux
development build, without manylinux certification. A different interpreter
build, target or opaque settings profile needs separate qualification. No GPU,
native batching, total allocation or elapsed-time authority follows.

## Original proposed Pumas boundary (historical)

Add a local wheel component to the existing validated Torch runtime plan. Input
is owner-materialized local bytes plus the exact SHA256, size, distribution,
version, wheel tag, installed native-image hash and reviewed source association.
The consumer does not nominate arbitrary installation paths or URLs. Resolve
dependencies within the existing approved offline runtime lock; missing inputs
refuse instead of fetching or repairing.

The proposed request also names the selected base runtime's registered environment
ID, opaque revision and published manifest digest. No base runtime is selected by
this handoff. Pumas must recheck that identity under its mutation/publication lock
and derive a new registered revision incorporating the component and interpreter
identities. Keep the upstream Torch release tag as a separate fact; that tag alone
cannot distinguish this component-bearing bundle from the original release.
Runtime revision naming, storage location and durable lease representation remain
Pumas-owned and must be acknowledged before a positive consumer is implemented.

Use `TorchPendingStage`, `TorchVersionsLock`, the staged-file manifest, registered
child cleanup custody, and VersionManager's atomic publication lifecycle. Create
a distinct immutable runtime revision. Do not alter an active/shared environment.
The pinned Pumas26a exposes release installs and forbids in-place Torch dependency
repair. Published c9 adds local acquisition; integration with Torch staging,
component-bearing publication and consumer dependency migration still remain.
Pantograph cannot implement them by copying files into an active runtime.

Return the registered environment ID, opaque revision, published manifest digest,
interpreter build identity and installed provider image identity. The published
retained runtime read source supplies revision/depot byte custody, separately
from mutable registration metadata. Retain it through process retirement and
acquire it before native
initialization/import under the existing owner publication/deletion authority.
It remains separate from genuine interpreter/import registration; retained bytes
alone grant no execution authority. A serializable manifest, readiness probe,
active-version name, file path or consumer-side parallel lock/store is insufficient.

An owner-associated interpreter/import registration must bind that lease to the
actual process/interpreter and the approved imported native image. Mutable
`sys.executable`, `sys.prefix`, `module.__file__`, module names and capability
descriptors are useful negative checks but cannot prove prior load custody.
Hashing a current file cannot retroactively identify an image loaded before its
path was replaced. An unknown already-initialized interpreter refuses managed
adoption. Owner startup/import control or genuine loaded-image evidence must be
agreed before a positive Pantograph adapter is enabled.

## Custody, refusal and rollback

1. Validate complete source/hash/size/profile/ABI/RECORD facts before staging.
   Refuse symlinks, path escapes, duplicate files, unqualified image collisions
   and mutable bundle destinations. An input manifest is a request, not a lease.
2. Stage under existing ownership. Caller loss/cancellation retains the stage,
   version lock and child cleanup lease until actual child completion. Partial
   stages never become executable; quarantine only owner-owned partial bytes.
3. Verify every installed file against wheel RECORD, the runtime lock and staged
   manifest. Publish/register atomically. A failure leaves the previous active
   and default runtime unchanged and cleans pending bytes only after drain.
4. Acquire the immutable execution lease before interpreter/provider import.
   Bind the registered interpreter/import proof to the exact revision. Missing,
   revoked, wrong-environment or unqualified proof refuses before backend effects.
5. Serialize future positive registration with shared-worker initialization,
   without holding a blocking registry lock while waiting for the GIL. Every
   native closure must retain custody through actual completion, including stream
   drop, cancellation and backend caller loss.
6. Model/worker stop drains existing producers but leaves native libraries and
   Python modules mapped. Retain imported-image custody until process
   retirement; do not release the bundle lease merely on model unload/stop. No
   Py_Finalize, sys.path edits or module unloading substitutes for owner retirement.
   Post-import failure clears executable readiness after draining while preserving
   that pin. CPython documents that extension modules are not unloaded even by
   [interpreter finalization](https://docs.python.org/3.12/c-api/init.html#c.Py_FinalizeEx).
   A future safe native-image retirement mechanism requires a separate contract.

## Pantograph increment

`ManagedPythonRuntimeStartRequest` carries bounded selected-revision and reviewed
provider metadata. `PyTorchBackend::start_managed_runtime` is an explicit closed
counterpart to the existing startup lifecycle. It validates the request before
reading the existing Rust shared-worker flag, then refuses a legacy initialized
shared worker or unavailable registered owner/interpreter custody. PyO3 .23 has
no safe no-initialization interpreter query and the repository denies unsafe
Rust; this entry performs no FFI query or speculative interpreter inspection.
It invokes no with_gil, imports, initialization, drains, model loads or
Python-global changes. There is deliberately no positive proof constructor or
caller-declared lease switch. Ordinary startup and worker/drain behavior remain
unchanged; the new entry is not composed as a default route.

Integrating published retained byte custody and implementing real interpreter/import
registration are the next prerequisites. Separate configuration, tensor, CPU-domain and atomic owner
inspection remain unqualified; NativeOwnerSnapshot stays Unknown/refused. The
qualified native tokenizer/settings component profiles continue to refuse
opaque values. Research scheduling defaults and timing acceptance are unchanged.
