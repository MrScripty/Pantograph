# PR49 embedding identity and replacement custody

This bounded repair descends directly from PR49 head
`6a2e393e66571c25004031c779c4388a4d15c8b5`, tree
`9c50812a2ed082d0b63a79b9fb6cfb86b57d83ee`, on the separate branch
`fix/pr49-embedding-identity-custody`. It preserves the complete stacked ancestry,
including PR44 `ca9edde6850cd58ade0b7e534bb4f58e704c2c64`, and the strict consumer
pin to Pumas `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`. The separate pooling
change `495efea606620153fe2f0363b104e56518436a06` remains excluded.

## Demonstrated findings and bounded repair

The [PR49 review](https://github.com/MrScripty/Pantograph/pull/49#pullrequestreview-5407199459)
identified a reachable wrong-model operation: legacy Candle embedding calls
ignored a nonempty requested model name and ran the resident model. The backend
now compares that name with the exact retained selected model ID. One recognized
`pumas://models/` prefix is accepted; a basename, unrelated ID, or whitespace-only
name is refused. Exactly empty names retain the existing resident-model
convention. Typed artifact, revision and producer-contract checks are unchanged.
The independent numerical-reference test now uses the resident ID for batched
calls and the established empty-name convention for single calls.

The cross-backend selected embedding path also stopped the resident before
loading its replacement. That contradicts the intended candidate-retention
behavior for this path. The ordinary explicit `switch_backend` API still has its
separate documented destructive-switch contract.

A private replacement supervisor now loads and validates the real Candle
candidate while retaining the prior backend and all its residency metadata.
The gateway retains the supervisor's actual join, and the supervisor owns the
candidate's existing actual blocking load join. Dropping the requesting future
signals cancellation without detaching either owner. Start, stop, explicit
switch and selected execution await that custody before mutating residency.
The returned exclusive backend guard remains held into selected forward
execution; publication does not introduce an unlocked execution gap.

Replacement admission occurs after candidate loading, acquisition of all
publication metadata guards and a final cancellation check, immediately before
retiring the old backend. Failures or cancellation before admission preserve the
resident. Retirement and publication after admission complete under the retained
supervisor even if the caller disappears or a late host cancellation arrives.
Late cancellation refuses the request while leaving a consistently published
replacement. This is not a promise to roll back arbitrary old-backend shutdown
once retirement has begun. An old-backend stop failure remains an error under
that backend's existing stop contract.

## Verification

Two new regression tests were transplanted into an isolated worktree with the
exact base production source. Both failed: nonempty B was accepted against
resident A, and a failed B candidate had already stopped resident A. That red
worktree is excluded from the candidate ancestry.

| Check | Result and scope |
| --- | --- |
| Native Candle aggregate, `cargo test --locked --offline -p inference --features backend-candle --lib` | 459 passed, zero failed or ignored |
| Default inference aggregate, `cargo test --locked --offline -p inference --lib` | 421 passed, zero failed or ignored |
| Candle all-target Clippy | Completed; only the existing selected-text unused-fields warning |
| Consumer compiler check, default dependency features, `ORT_SKIP_DOWNLOAD=1` | Embedded runtime, workflow service and model-library node test targets passed; compiler evidence only |
| Decision traceability gate tests | 28 passed |
| Critical lint and accessibility gates, staged `npm run lint:no-new` | Passed; audit and traceability gates unchanged |
| Production dependency audit | Zero vulnerabilities |
| Rust formatting and whitespace checks | Passed |

An initial timing-test fixture accidentally registered both `Candle` and
`candle`, allowing the case-normalizing registry to choose the ungated factory.
The test now replaces the exact existing `Candle` registration. Production
registry behavior was not changed. Final aggregates use the corrected fixture.
The four new test functions cover backend and gateway A/B refusal, established
empty-name and URI behavior, unchanged valid A forward, replacement with real B,
failed cross-backend candidates, host cancellation before and immediately after
publication, dropped callers with actual stop and replacement waiters, and
publication custody through stop. The native regressions exercise actual frozen
CPU F32 BERT weights at widths
8 and 12. The prior backend in cross-backend tests deliberately has a distinct
registry identity and delegates actual forward execution to Candle; this proves
the gateway replacement path, not llama.cpp or GPU execution. The worker timing
hook runs after real safetensor deserialization and BERT construction. A separate
publication hook runs after the actual backend and metadata swap. No mock vectors
are presented as inference.

The existing exact-reuse, optional-revision, canonical-root, failed-worker,
selected-load and numerical-reference coverage remains part of the native suite.
No frozen fixture, manifest, lockfile, audit gate or traceability map is changed.

## Qualification limits and coordinated handoff

The authorized pinned ONNX Runtime 1.24.2 URL was probed again in this environment;
the proxy rejected CONNECT with HTTP 403. Compiler checks using
`ORT_SKIP_DOWNLOAD=1` keep the unchanged default dependency features, but cannot
establish linked native IPC or full workflow acceptance. No permission, network,
credential, library substitution or dependency-feature change was made.
GUI, GPU, pretrained semantic inference and linked full workflow execution are
unexecuted in this cloud lane.

Parent receipts for Quality `37217218521` and Runtime Separation `37217218442`
belong to the preceding PR49 head. They do not qualify this successor. The parent
coordinates independent review and exact-head hosted/native acceptance before
advancing PR49. This lane publishes a separate candidate only; it does not merge,
advance PR49, resolve review findings or request external review.
