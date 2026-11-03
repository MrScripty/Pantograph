# Runtime-owned host RAM ceilings

## Thesis reconciliation and bounded acceptance

The Library's `pantograph_scheduler_thesis.md`, version 1
(`libfile_6c1d9aa6e1f08191b4a14e0de0846220`), separates physical feasibility from
performance policy (§2, §4, §6, §15). Current implementation already has shared
resource domains, authoritative selected-candidate custody, PyTorch/llama resident
owners, and owner-advertised CPU candidates. Existing technical-fit history
ranking is also implemented; a new warm-first heuristic would duplicate policy
without comparable completion evidence.

The useful next priority is a fresh necessary RAM ceiling on explicitly declared
physical pools. Acceptance: with configured capacity 100 and owner ceiling 60,
two contending 40-byte claims admit only one. Shrink below live usage or missing
owner facts retains every charge and blocks further positive claims. Admission
rereads the source under the existing registry lock; advisory fit is not custody.
The source cannot replace another attached owner or raise the configured budget.
Full peak claims, resident charges, safety margins, and generation fencing remain.

After this slice, priorities are exact-model comparable load/execution timing,
actual per-device GPU ownership/backing facts, then completion-oriented ranking
with frozen matched cohorts. Concurrent execution, sharding, static batching and
resident/transient discounts require explicit backend support and complete hard
envelopes; they are not enabled by this change.

## Implementation

An immutable host RAM capacity source is attached to the registry. It applies to
declared pools containing RAM, including later declarations; a VRAM-only pool
keeps its existing owner. Unified pools still charge every RAM and VRAM claim.
Effective capacity is the minimum of configured capacity and the fresh ceiling,
followed by the configured margin and all existing task/resident charges.
Unavailable facts give zero admission budget without pretending the physical
capacity was measured as zero. Source attachment, known zero and unavailable
ceiling remain distinct in the read-only observation.

Capacity changes do not call the configured-budget setter, which rejects limits
below existing usage. They do not release, shrink or deduplicate leases. Resident
publication checks the same ceiling; if publication is initially blocked, the
existing missing-resident state blocks admission until an ordered owner retry.
Stale or wrong lifecycle sources still cannot replace that publication.

Both hosted composition paths attach the native source before constructing the
resource-backed provider. A previously attached owner remains authoritative.
The native source reads total physical memory and, on Linux, the current cgroup
v2 group's hard limit and every visible ancestor at a verified conventional mount.
Missing/malformed facts, v1 or unresolved mount roots are unavailable. It reads
no free-memory sample and retains no stale fallback. No new dependency, manifest,
lockfile, ONNX download feature, device binding or scheduling rank is introduced.

These are necessary ceilings, not complete available capacity. Namespaces can
hide ancestors; external consumers and complete peak envelopes still belong in
the configured budget. On other platforms the implementation supplies only total
physical RAM; native execution there has not been qualified.

## Qualification and preserved failures

The successor starts at frozen CPU `394d4748be333fade6653aa5665a13129d8f4495`.
PR54/55 and all frozen qualification refs remain separate. The descriptor repair
`9ec3c2e036400ce5e245d3ade820d0b4c869ccb0` is an ancestor of PR55 `ff84841`,
but is absent from CPU `394d4748`; it is not duplicated here.

Qualification uses Rust 1.92, locked/offline dependencies, the existing Python
3.12 library, unset `ORT_*`, and a dedicated target directory. Executed checks:

- All 138 registry tests pass, including eight ceiling regressions for contention,
  shrink/unavailability, immutable ownership, resident charges, VRAM-only pools,
  unified memory/margins, ordered resident retry, and zero/unknown distinction.
- All 711 inference library tests pass with `--test-threads=1`. Two controlled
  Linux hierarchy tests cover positive limits and unavailable layouts. The native
  source also runs against this host and drives actual registry admission.
- The controlled selected-text regression goes through actual gateway CPU facts,
  provider admission, scheduler selection and the embedded host port. Insufficient
  capacity prevents a backend load; sufficient capacity executes; later shrink
  retains the complete peak lease. The focused host run passes 64 tests.
- The full embedded suite executes 500 tests: 498 pass, two frozen-parent failures
  remain (descriptor four-versus-six inputs; warmup timeout receiving success).
  Neither assertion nor timeout is changed.
- A broader inference command passes its 711 library tests and integration groups
  of 7, 10 and 12, then fails one of 40 model-contract tests. The stale fixture
  passes `model_ref_contract_version` to a DTO that rejects it; the same failure
  reproduces on clean exact CPU parent `394d4748` after cleaning local core crates
  in a separate copied target. It is outside this bounded slice.
- The initial parallel inference library run passes 710 and fails an unchanged
  PyTorch fixture importing `AutoModelForCausalLM`. Clean exact parent `394d4748`
  reproduces the same failure (707 passed, one failed). Serial library
  qualification passes; the parallel failure is retained as a baseline failure.
  An earlier targeted invocation could not find the Python shared library and
  executed no tests; subsequent runs provide its actual library path.
- Warning-deny Clippy covers all targets of registry, inference and embedded
  runtime with the supported mixed backend features. Formatting, repository
  critical/accessibility/traceability gates, 28 traceability regressions and all
  nine ONNX no-build-download feature graphs pass.

This host's cgroup mount root is `/..`, outside the verified mapping. The actual
native observation is unavailable and admission fails closed. Positive hierarchy
and host-execution evidence uses controlled owners; this is not positive native
capacity qualification for this container. GTK/WebKit desktop execution, real
model quality, GPU discovery, cross-platform native execution, and hidden-limit
or external-consumer accounting remain unqualified.
