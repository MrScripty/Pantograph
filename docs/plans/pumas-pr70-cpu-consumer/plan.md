# Pumas PR70 CPU consumer compatibility

Status: `Active`, operation `start` under the parent's explicit continuation.
Acceptance: `pending`. Current phase: exact dependency qualification.
Next slice: resolve candidate pin and run actual Pantograph CPU composition.

Baseline Pantograph source: `34dfedd27dde9a30274226e1ea26ec1e1b2de8e3`.
Candidate Pumas: `df53db08e2ba224edfaf713522ddd410f7d4ac44`, PR70; its
receipt identifies production source equivalent to main `ef9eead9`. The current
Pantograph baseline pin `aa1d79ae2d53f0acb088bc85223ecc2ff46af12b` remains
untouched in PR78. Chrema stays `508c8b8cb724b49c655dab8bc4c8336190e516c7`;
Candle stays `88ed7911de9e88196b1f55b199145d22647f415e`.

## Objective and M1 write set

Establish whether the exact Pumas candidate satisfies this actual consumer edge:
locked Rust compilation, Pumas model/descriptor/facts/load target, public
Pantograph CPU scheduler execution and fresh-process retained-state inspection,
composed through public Chrema. Untrained fixture results do not qualify PR75's
pretrained text T3. No release tags or upstream source writes.

M1 (`Active`) may write only root `Cargo.toml`, `Cargo.lock`,
`.github/workflows/quality-gates.yml`,
`examples/pumas-chrema-cpu-demo/compatibility-lock.json`, and this plan directory.
Manifest, resolved source, CI checkout and demo lock move together only on this
candidate branch. API breaks trigger a bounded upstream/consumer proposal to
parent before any source repair outside this write set. No transitive updates
unless resolution demonstrates they are required by the new Pumas declaration.

Acceptance: exact resolution/no-build-download contract; actual native
descriptor/package/load-target API; 13 baseline integration contracts plus any
review repairs; fresh separate-process read; relevant owner tests. Record failed
API/build/runtime attempts as failures. A successful receipt identifies all
consumed commits, binary/source hashes and observed gates, not release approval.

## Composition and isolation

Composed-design applicability: `not-applicable` for new architecture; this slice
changes the existing dependency edge only. It inherits the baseline composition
with identical owners/contract boundaries and reruns real consumer observations.
Replan if candidate changes those ownership or persisted/runtime semantics.

Task branch `qualification/pumas-pr70-cpu-consumer-2026-10-14`, worktree
`/workspace/Pantograph-pumas-pr70`, integration owner parent; never merge or
retarget PR78 implicitly. Retain candidate ref for review/handoff. Remove the
task-created clean worktree after exact committed head is reachable from that
retained ref and evidence records the disposition. No other worktree cleanup.

No blocker yet. Parent/Pumas owner owns candidate promotion and APIs. Read
baseline repository guidance and current public Coding-Standards; available
AGENTS.md/.agents skills remain absent. [Ledger](execution-ledger.md) and
[issues](issues.md) own dated findings.
