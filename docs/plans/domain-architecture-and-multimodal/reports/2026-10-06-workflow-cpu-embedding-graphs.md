# Canonical CPU embedding graphs

The existing selected Candle CPU embedding owner could return a vector through a
single host request, but embedding descriptors exposed no ports, scheduler
envelopes rejected the task, and the registered vector sink had no scheduler
execution template. This M2 successor connects those existing owners without
introducing another inference executor.

## Source and acceptance

Normal Git fetch confirmed main
`c75fa2379a730833709ea8976075bf17a117040f`, tree
`6c8c4bb1cbed0527b58c1048ebf49fa0e243d56b`, before edits or builds.
The isolated branch is `feat/workflow-cpu-embedding-graphs`. Accepted stop-string
candidate `1911983715155fc3927d0ebdfd0b94b851a58536` remains separate; main,
PR60, seed/precision/repair and paused work are preserved.

Acceptance requires typed descriptor and connection compatibility, a persisted
`text-input → llm-inference(embedding) → vector-output` graph through the public
scheduler, actual committed Candle width-8 and width-12 CPU golden comparisons,
per-member execution identity and selected model metadata, optional usage,
cancellation and invalid-result refusal. Compatible envelope members may execute
sequentially; backend-native batching and throughput are outside this contract.

## Changed behavior

The additive descriptor category `structured` distinguishes `embedding` from
general `json`. Embedding exposes required string `text`, required `embedding`
and `metadata`, and optional `usage`, with no authored default or generation
controls. Rust effective definitions, live connection previews and the actual
Svelte descriptor overlay map these types consistently. Existing tensor/artifact
categories retain their meaning.

The batch host admits a shared Candle embedding context and runs members
sequentially through the existing selected single-request owner. All member shapes
are checked before resolution. Existing cancellation, rejection and identity
conversion apply to each member. No native batch capability is advertised.

The scheduler lowers `vector-output` to its own non-runtime template. A connected
input must be a JSON array of 1–4096 finite numbers within the existing 64 KiB
structured-output limit. It preserves the original JSON values, including integer
representation, rather than converting through the generic core sink's f64 path.
Source lookup checks workflow, run, node and task identity. Pending, missing,
failed and malformed sources produce distinct readiness outcomes. An unconnected
optional sink retains the existing null behavior.

## Verification

The public session test saves and reloads through `FileSystemWorkflowGraphStore`
and the public graph service, then executes the restored graph through
`WorkflowSessionExecutionRuntime`. Both committed untrained Candle fixtures
produce the expected vectors, exact sink passthrough, selected model identity,
width/index/token metadata, runtime/device identity and usage. The fixture host's
alternate execution and runtime-load hooks remain unused.

Two-member envelope tests compare actual Candle vectors and token goldens for
both widths and preserve distinct workflow/run/node/task/execution/assignment
identities. Panic sentinels establish that malformed inputs and precancellation
do not resolve targets or packages. Projector tests reject absent/multiple/wrong
result kinds, wrong/missing index, missing token count, empty/oversized and
nonfinite vectors; valid vectors preserve metadata and optional usage. Scheduler
tests cover byte/element boundaries, exact JSON, source scope, missing ports,
failed producers, malformed values and optional null. Rust/TS tests cover saved
descriptor types, fingerprints and vector connection compatibility.

The complete effective Cargo feature graph was inspected before the first
build. ORT uses `load-dynamic`/`disable-linking`; `download-binaries` is absent.
Every build sets `ORT_SKIP_DOWNLOAD=1` and uses locked offline dependencies.
The existing Candle revision `88ed7911` and current Pumas revision `26a84e32`
remain pinned; no manifests, lockfiles, authentication or binary-download behavior
change. The repository's ONNX policy script also passes its full supported
target/feature matrix.

Final locked offline qualification passes 2,465 Rust test cases and one doctest:
795 inference library plus 74 integrations, 530 embedded, 28 interface contracts,
52 runtime-host contracts, and 986 workflow-service tests. Four explicitly ignored
native integration cases and two ignored doctests remain unqualified. The full
run includes the new public graph and envelope tests and the existing actual
Candle load/forward cancellation regressions. Strict all-target Clippy passes for
all five affected packages. All 670 frontend tests, TypeScript typecheck, affected
TS ESLint, root lint, critical/a11y, scheduler-only execution, frontend build,
workspace formatting, staged traceability and diff checks pass.

The first combined run passed inference and rejected two new envelope fixtures
because their nested readiness/dispatch/reservation identities still named the
original source member. The test-only repair aligns all identities; the final
complete rerun passes. Both the failure and final results remain in the evidence.
No production validation was relaxed.

Reproduction uses the existing pinned toolchain and offline cache, with
`ORT_SKIP_DOWNLOAD=1`, `HF_HUB_OFFLINE=1`, `TRANSFORMERS_OFFLINE=1`, and the
installed Python library directory on `LD_LIBRARY_PATH`. The suite command is:

```sh
cargo test --locked --offline \
  -p inference -p pantograph-embedded-runtime \
  -p pantograph-inference-interface-contracts \
  -p pantograph-runtime-host-contracts -p pantograph-workflow-service \
  --features inference/backend-pytorch,inference/backend-candle,inference/std-process,pantograph-embedded-runtime/backend-pytorch
```

Strict Clippy uses the same package/feature selection, `--all-targets`, and
`-- -D warnings`. Evidence logs, the full effective feature graph, source-only
patch and preservation record are retained in the qualification archive described
below. Frontend dependencies reuse installed third-party modules; the workspace
package points at this candidate's source.

Qualification archive:
`/workspace/qualification-evidence/workflow-cpu-embedding-graphs-evidence.tar.gz`
(22 files), SHA-256
`e95e2b79e438139e2a6361a125b70551ac959f5994980b3a25e3d86930cd4c24`.
The archive's source-only patch SHA-256 is
`95966596715675253301a1cdfd6770196931c6132a0a8aa090fed5a6bb14c20f`;
it covers all product/test changes including newly added fixtures and tests.
Its manifest hashes individual logs and the preservation record.

Independent read-only review found no remaining substantive source issue. It
corrected a test-only usage-key assertion from `input_tokens` to the owner's
existing `prompt_tokens`; the public CPU graph subsequently passed.
It also reviewed the complete test-only nested identity correction and found no
remaining issue. Parent review/publication and fresh hosted CI remain separate.

## Qualification limits

Numerical execution is actual Candle CPU inference over the existing committed
synthetic, untrained BERT fixtures. Descriptor fixtures, Pumas package/target
resolution, dependency readiness, dispatch candidate facts and session host
metadata are controlled. This does not qualify live Pumas discovery/import,
production pretrained or custom models, semantic embedding quality, GPU,
desktop IPC/GUI execution, native batching or throughput. The full plan's DA-03
and DA-07 qualification remains open. No stop-string/seed, sampling, ranking or
resident-accounting work is reopened.

External standards at the plan's recorded `/media/jeremy/.../Coding-Standards`
path are unavailable here; the existing repository authority and preserved plan
instructions govern this bounded change. Existing disjoint implementation and
independent-review agents were reused. Session billing/rate telemetry is
unavailable, so complete API cost remains unknown rather than inferred.

## Explicit requested revision successor

Independent read-only review found that a structurally valid handoff could carry
an explicit requested revision different from the selected/package/target
revision. The old host projected both the typed request and selected decision
from `selected_model_ref`, erasing the original request's revision before the
gateway's existing revision checks. This inherited projection gap blocks the
new public embedding graph's requested-identity acceptance.

The user-authorized narrow successor is isolated on
`fix/workflow-embedding-requested-revision` from frozen source
`2961b1480c103ec242c6359a45ceee6bb5d999f1`, tree
`fff2f10bb15d396c698e88ce01de283f6ad4b3b6`. Normal Git fetch confirms that
source and main `c75fa2379a730833709ea8976075bf17a117040f`, tree
`6c8c4bb1cbed0527b58c1048ebf49fa0e243d56b`. PR60's separate successor and
the embedding evidence publication are not merged into this source. Frozen
implementation, original evidence archive, seed/stop and resident histories stay
preserved; this change does not add a generic scheduler revision policy.

Acceptance requires reproducing the reviewer's exact four requested-identity
JSON changes with the actual width-8 Candle host fixture while leaving selected
model/package/target at `untrained-seed-179`. A mismatch or missing selected
revision must reject before package/target resolution, loading or inference on
both single and envelope routes. Matching explicit revisions remain valid;
omitted requested revisions permit selected-owner refinement and actual CPU
golden output. The typed request must preserve the original requested revision,
including omission, while the selected decision retains its own revision.

The embedding-specific host validator enforces explicit requested/selected
revision agreement before its resolvers. Projection preserves the requested
revision in the typed request while retaining the already-validated selected
model/artifact identity and selected decision. Existing gateway checks separately
constrain package/target revisions. Model-ID and authored artifact constraints
retain the existing handoff validation; unrelated text/image/scheduler paths are
unchanged. Envelope members reuse that same validation and selected owner.

The exact original case reproduces as `Completed` with actual width-8 CPU vector
and `untrained-seed-179` metadata where `Rejected` is required; the failing log
is retained. The final five-package suite passes 2,469 non-doctest tests and one
doctest, including 534 embedded-runtime tests (four new revision methods), the
public saved/reopened CPU graph and retained matching-revision/member cases.
Six existing optional cases remain ignored. Strict all-target Clippy, Rust
format, critical anti-patterns and scheduler-boundary checks pass. An initial
enclosing command reported exit 1 despite all test groups passing; the deciding
rerun records the child Cargo exit status explicitly as 0. Frontend interfaces
are unchanged; original frontend qualification is not rerun or promoted to new
desktop evidence.

The [revision successor evidence](../evidence/workflow-cpu-embedding-revision/README.md)
binds the failing native reproduction and final source hashes separately from
the frozen embedding archive. All CPU fixtures remain synthetic/untrained;
Pumas/descriptor/readiness/dispatch facts are controlled. Production loader,
live Pumas, pretrained semantic quality, GPU, throughput and desktop execution
remain unqualified. Parent independent verification and new hosted qualification
remain separate. No external weights, ONNX/ORT binaries, privilege or
authentication changes, main mutation, PR creation or merge are performed.
