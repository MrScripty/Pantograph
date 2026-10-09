# Bounded native tokenizer component

This Apache-2.0 provider patch targets Hugging Face Tokenizers 0.21.4, revision
`e892882fd4608b468dcf9dc33ea95283882b8e6d`. It is a source development integration,
not an automatic replacement of the installed runtime dependency. The upstream
license is retained in LICENSE; modified provider files carry change notices.

`Tokenizer._pantograph_wordlevel_snapshot_v1()` is consumed directly by
`service_timing_owner._tokenizer_state`, before native getters. It returns
`(accepted, payload_bytes, work_units, copied_buffer_bytes)`. Missing capability
retains the existing advisory path. Available capability plus refusal never
falls back. The binary identity is explicitly versioned; its hash is distinct
from legacy JSON identity. The provider's exact source/build association is a
qualification prerequisite, not something the API name alone can attest.

The operation has fixed compiled ceilings: 4,096 entries per collection,
8,192 retained capacity, 16,384 UTF-8 bytes per string, 65,536 aggregate borrowed
string bytes, and 4 MiB cumulatively copied buffer bytes. These derive from the
existing owner caps; duplicate stored strings count separately, so the bounded
profile is narrower. Work units charge capacity scan envelopes, entry checks,
worst-case string comparison bytes, and buffer writes. The fixed work ceiling
is `5 * 4 * 4096 * 14 * (16384 + 1) + 32 * 4 MiB` and is admitted internally
before inspection; counters include refused prefixes. No caller can increase
ceilings or replace them with reported elapsed time.

The capability is compiled as qualified only for Rust 1.92.0 commit
`ded5c06cf21d2b93bffd5d884aa6e96934ee4234`, target
`x86_64-unknown-linux-gnu`. Other compiler/target builds return refusal before
inspection. The capacity-to-bucket calculation uses this standard library's
hashbrown 0.15.5: for the provider's maps with no deletion tombstones, a full
storage scan is bounded by `2 * (capacity + 1) + 16`, including control groups.
Both preflight and reference collection are charged separately. This is a
sealed Python-provider construction invariant, not a generic claim about
arbitrary Rust HashMaps: WordLevel creation/deserialization/training builds
fresh maps; its vocabularies are private and have no deletion mutation. Added
maps/set likewise only insert/overwrite. Clones retain these invariants. A
provider or mutation-path change requires fresh qualification; opaque Rust
embeddings supplying arbitrary tombstoned builder maps are outside this profile.
See [standard-library dependency](https://github.com/rust-lang/rust/blob/1.92.0/library/std/Cargo.toml)
and [hashbrown capacity arithmetic](https://github.com/rust-lang/hashbrown/blob/v0.15.5/src/raw/mod.rs).

The exact work/copy domain is logical traversal, comparisons and native buffer
writes, including reference-vector slots/swaps, fallible PyBytes zero-initialization
and the PyBytes payload copy. Fixed stack metadata (including a 4,096-byte ID-presence array), binding
objects, allocation bookkeeping, native/interpreter instructions, scheduling,
and allocator/lock implementation overhead are outside these units. This is
not a wall-time, physical allocator capacity or total process-memory guarantee.
No unbounded input traversal/copy is licensed by those exclusions.

Preflight uses borrowed counts, capacities and byte lengths under a single
nonblocking model read guard and a nonblocking single pre-tokenizer read guard.
It holds the outer immutable Python tokenizer borrow and GIL throughout; no
callback, serializer, log formatting, blocking lock or GIL release runs. Normal
load/collector/drain custody remains unchanged. Missing, poisoned or contended
native guards refuse. Interrupted callers must still retain native ownership
until the existing worker drain completes.

Only WordLevel with Whitespace/WhitespaceSplit and absent normalizer, processor
and decoder is supported. Sparse/out-of-range IDs refuse. The payload includes
forward/reverse maps, added-token maps/flags, ordered classic/special histories,
special-token membership, matcher IDs/structural metadata, encode-special flag,
padding (including pad-token bytes), and truncation. Any normalized added token
in current or historical storage refuses: a current absent normalizer does not
prove what output a previous normalizer used to compile its matcher. Existing
general serializers are never invoked; occupied entries use metered heapsort.

This qualifies a tokenizer component only. Python settings, configuration,
tensor/build/CPU-domain collection and the aggregate owner still need their own
cost models. `NativeOwnerSnapshot` in the Rust inspection ledger remains
Unknown/refused. No real timing, capacity, GPU/native batching, default policy
or full research-scheduler rollout is established by this component.

## Build and local qualification

Obtain the pinned Apache-2.0 source through your normal dependency workflow.
`prepare.py` requires a clean checkout, applies only the reviewed patch, copies
the snapshot module and frozen build lock, and emits input hashes. It performs
no network access or installation:

```sh
python3 crates/inference/torch/provider/tokenizers-0.21.4/prepare.py /path/to/tokenizers
ORT_SKIP_DOWNLOAD=1 cargo build --locked --offline \
  --manifest-path /path/to/tokenizers/bindings/python/Cargo.toml
```

Use an isolated Python package directory containing the provider's `py_src`
package and the built `libtokenizers.so` copied as
`tokenizers/tokenizers.abi3.so`. Keep the normal installed wheel intact. Set
that directory first in PYTHONPATH and run
`crates/inference/torch/tests/test_bounded_tokenizer_snapshot_oracle.py`.
The synthetic native units create no model and run no inference. Record source,
lock and extension hashes and the receipt alongside test results. Missing build
dependencies fail qualification; the helper does not fetch them or suppress
build/test failures. Runtime distribution and the aggregate inspection budget
remain separate integration gates.
